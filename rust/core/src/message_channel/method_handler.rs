use core::panic;
use std::{
    cell::RefCell,
    convert::{TryFrom, TryInto},
    fmt::Display,
    rc::{Rc, Weak},
};

use crate::{value::Value, Context, ContextMessageChannel, TryFromError};

use super::{IsolateId, MessageChannelDelegate, SendMessageError};

#[derive(Debug)]
pub enum MethodCallError {
    SendError(SendMessageError),
    PlatformError(PlatformError),
    ConversionError(TryFromError),
}

impl Display for MethodCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodCallError::SendError(e) => write!(f, "error sending message: {}", e),
            MethodCallError::PlatformError(e) => write!(f, "platform error: {}", e),
            MethodCallError::ConversionError(e) => write!(f, "conversion error: {}", e),
        }
    }
}

impl std::error::Error for MethodCallError {}

#[derive(Debug)]
pub struct PlatformError {
    pub code: String,
    pub message: Option<String>,
    pub detail: Value,
}

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "platform error (code: {}, message: {:?}, detail: {:?}",
            self.code, self.message, self.detail
        )
    }
}

impl std::error::Error for PlatformError {}

#[derive(Debug)]
pub struct MethodCall {
    pub method: String,
    pub args: Value,
    pub isolate: IsolateId,
}

pub trait MethodHandler: Sized + 'static {
    fn on_method_call(&mut self, call: MethodCall, reply: MethodCallReply);

    // Implementation can store weak reference if it needs to pass it around.
    // Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    // Keep the method invoker if you want to call methods on engines.
    fn assign_invoker(&mut self, _invoker: MethodInvoker) {}

    // Called when engine is about to be destroyed.
    fn on_isolate_destroyed(&mut self, _isolate: IsolateId) {}

    // Registers itself for handling platform channel methods.
    fn register(self, channel: &str) -> RegisteredMethodHandler<Self> {
        RegisteredMethodHandler::new(channel, self)
    }
}

pub struct MethodInvoker {
    channel_name: String,
}

impl MethodInvoker {
    /// Convenience call method that will attempt to convert the result to specified type.
    pub fn call_method_cv<
        V: Into<Value>,
        F, //
        T: TryFrom<Value, Error = E>,
        E: Into<TryFromError>,
    >(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
        reply: F,
    ) where
        F: FnOnce(Result<T, MethodCallError>) + 'static,
    {
        self.call_method(target_isolate, method, args, |r| {
            let res = match r {
                Ok(value) => value
                    .try_into()
                    .map_err(|e: E| MethodCallError::ConversionError(e.into())),
                Err(err) => Err(err),
            };
            reply(res);
        });
    }

    pub fn call_method<V: Into<Value>, F>(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
        reply: F,
    ) where
        F: FnOnce(Result<Value, MethodCallError>) + 'static,
    {
        let args: Value = args.into();
        let call: Value = vec![Value::String(method.into()), args].into();
        Context::get().message_channel().send_message(
            target_isolate,
            &self.channel_name,
            call,
            move |res| match res {
                Ok(value) => {
                    let result = unpack_result(value).expect("Malformed message");
                    reply(result);
                }
                Err(err) => reply(Err(MethodCallError::SendError(err))),
            },
        );
    }
}

pub struct MethodCallReply {
    pub(crate) reply: Box<dyn FnOnce(Value) -> bool>,
}

impl MethodCallReply {
    pub fn send_ok(self, value: Value) {
        (self.reply)(Value::List(vec!["ok".into(), value]));
    }

    pub fn send_error(self, code: String, message: Option<String>, detail: Value) {
        (self.reply)(Value::List(vec![
            "err".into(),
            code.into(),
            message.map(|s| s.into()).unwrap_or(Value::Null),
            detail,
        ]));
    }

    pub fn send<V: Into<Value>, E: Into<PlatformError>>(self, result: Result<V, E>) {
        match result {
            Ok(value) => self.send_ok(value.into()),
            Err(err) => {
                let err: PlatformError = err.into();
                self.send_error(err.code, err.message, err.detail)
            }
        }
    }
}

pub struct RegisteredMethodHandler<T: MethodHandler> {
    inner: Rc<RegisteredMethodHandlerInner<T>>,
}

// Active method call handler
impl<T: MethodHandler> RegisteredMethodHandler<T> {
    fn new(channel: &str, handler: T) -> Self {
        Self::new_ref(channel, Rc::new(RefCell::new(handler)))
    }

    fn new_ref(channel: &str, handler: Rc<RefCell<T>>) -> Self {
        let res = Self {
            inner: Rc::new(RegisteredMethodHandlerInner {
                channel: channel.into(),
                handler,
            }),
        };
        Context::get()
            .message_channel()
            .register_delegate(&res.inner.channel, res.inner.clone());
        res.inner.init();
        res
    }
}

impl<T: MethodHandler> Drop for RegisteredMethodHandler<T> {
    fn drop(&mut self) {
        Context::get()
            .message_channel()
            .unregister_delegate(&self.inner.channel);
    }
}

struct RegisteredMethodHandlerInner<T: MethodHandler> {
    channel: String,
    handler: Rc<RefCell<T>>,
}

impl<T: MethodHandler> RegisteredMethodHandlerInner<T> {
    fn init(&self) {
        let weak = Rc::downgrade(&self.handler);
        self.handler.borrow_mut().assign_weak_self(weak);
        self.handler.borrow_mut().assign_invoker(MethodInvoker {
            channel_name: self.channel.clone(),
        });
    }
}

impl<T: MethodHandler> MessageChannelDelegate for RegisteredMethodHandlerInner<T> {
    fn on_isolate_joined(&self, _isolate: IsolateId) {}

    fn on_message(
        &self,
        isolate: IsolateId,
        message: Value,
        reply: Box<dyn FnOnce(Value) -> bool>,
    ) {
        if let Some(call) = unpack_method_call(message, isolate) {
            let reply = MethodCallReply { reply };
            self.handler.borrow_mut().on_method_call(call, reply);
        } else {
            panic!("malformed method call message");
        }
    }

    fn on_isolate_exited(&self, isolate: IsolateId) {
        self.handler.borrow_mut().on_isolate_destroyed(isolate);
    }
}

pub(crate) fn unpack_result(value: Value) -> Option<Result<Value, MethodCallError>> {
    let vec: Vec<Value> = value.try_into().ok()?;
    let mut iter = vec.into_iter();
    let ty: String = iter.next()?.try_into().ok()?;
    match ty.as_str() {
        "ok" => Some(Ok(iter.next()?)),
        "err" => {
            let code = iter.next()?.try_into().ok()?;
            let message = match iter.next()? {
                Value::String(s) => Some(s),
                _ => None,
            };
            let detail = iter.next()?;
            Some(Err(MethodCallError::PlatformError(PlatformError {
                code,
                message,
                detail,
            })))
        }
        _ => None,
    }
}

pub(crate) fn unpack_method_call(value: Value, isolate: IsolateId) -> Option<MethodCall> {
    let vec: Vec<Value> = value.try_into().ok()?;
    let mut iter = vec.into_iter();
    Some(MethodCall {
        method: iter.next()?.try_into().ok()?,
        args: iter.next()?,
        isolate,
    })
}
