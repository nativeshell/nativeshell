use core::panic;
use std::{
    cell::RefCell,
    convert::TryInto,
    rc::{Rc, Weak},
};

use thiserror::Error;

use crate::{value::Value, Context, ContextMessageChannel};

use super::{IsolateId, MessageChannelDelegate, SendMessageError};

#[derive(Error, Debug)]
pub enum MethodCallError {
    #[error("error sending message")]
    SendError(#[from] SendMessageError),

    #[error("platform error")]
    PlatformError(#[from] PlatformError),
}

#[derive(Error, Debug)]
#[error("platform error (code: {code:?}, message: {message:?}, detail: {detail:?})")]
pub struct PlatformError {
    pub code: String,
    pub message: Option<String>,
    pub detail: Value,
}

#[derive(Debug)]
pub struct MethodCall {
    pub method: String,
    pub args: Value,
}

pub trait MethodHandler: Sized + 'static {
    fn on_method_call(&mut self, call: MethodCall, reply: MethodCallReply, isolate: IsolateId);

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
    pub fn call_method<F>(&self, target_isolate: IsolateId, method: &str, args: Value, reply: F)
    where
        F: FnOnce(Result<Value, MethodCallError>) + 'static,
    {
        let call: Value = vec![Value::String(method.into()), args].into();
        Context::get().message_channel().send_message(
            target_isolate,
            &self.channel_name,
            call,
            move |res| match res {
                Ok(value) => {
                    if let Some(result) = Self::unpack_result(value) {
                        reply(result)
                    } else {
                        panic!("Malformed message");
                    }
                }
                Err(err) => reply(Err(MethodCallError::SendError(err))),
            },
        );
    }

    fn unpack_result(value: Value) -> Option<Result<Value, MethodCallError>> {
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
}

pub struct MethodCallReply {
    reply: Box<dyn FnOnce(Value) -> bool>,
}

impl MethodCallReply {
    pub fn send_ok(self, value: Value) {
        (self.reply)(Value::List(vec!["ok".into(), value]));
    }

    pub fn send_error(self, code: String, message: Option<String>, detail: Value) {
        (self.reply)(Value::List(vec![
            "err".into(),
            code.into(),
            message
                .map(|s| Value::String(s.into()))
                .unwrap_or(Value::Null),
            detail,
        ]));
    }

    pub fn send(self, result: Result<Value, PlatformError>) {
        match result {
            Ok(value) => self.send_ok(value),
            Err(err) => self.send_error(err.code, err.message, err.detail),
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
    fn unpack_method_call(value: Value) -> Option<MethodCall> {
        let vec: Vec<Value> = value.try_into().ok()?;
        let mut iter = vec.into_iter();
        Some(MethodCall {
            method: iter.next()?.try_into().ok()?,
            args: iter.next()?,
        })
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
        if let Some(call) = Self::unpack_method_call(message) {
            let reply = MethodCallReply { reply };
            self.handler
                .borrow_mut()
                .on_method_call(call, reply, isolate);
        } else {
            panic!("Malformed method call message");
        }
    }

    fn on_isolate_exited(&self, isolate: IsolateId) {
        self.handler.borrow_mut().on_isolate_destroyed(isolate);
    }
}
