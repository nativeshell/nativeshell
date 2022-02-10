use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use async_trait::async_trait;

use crate::{
    unpack_method_call, unpack_result, util::FutureCompleter, Context, GetMessageChannel,
    IsolateId, MessageChannelDelegate, MethodCall, MethodCallError, MethodCallReply, PlatformError,
    TryFromError, Value,
};

pub type PlatformResult = Result<Value, PlatformError>;

pub trait IntoPlatformResult {
    fn into_platform_result(self) -> PlatformResult;
}

impl<T: Into<Value>, E: Into<PlatformError>> IntoPlatformResult for Result<T, E> {
    fn into_platform_result(self) -> Result<Value, PlatformError> {
        self.map(|v| v.into()).map_err(|e| e.into())
    }
}

#[async_trait(?Send)]
pub trait AsyncMethodHandler: Sized + 'static {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult;

    // Implementation can store weak reference if it needs to pass it around.
    // Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    // Keep the method invoker provider if you want to call methods on engines.
    fn assign_invoker(&mut self, _invoker: AsyncMethodInvoker) {}

    // Called when engine is about to be destroyed.
    fn on_isolate_destroyed(&self, _engine: IsolateId) {}

    // Registers itself for handling platform channel methods.
    fn register(self, channel: &str) -> RegisteredAsyncMethodHandler<Self> {
        RegisteredAsyncMethodHandler::new(channel, self)
    }
}

#[derive(Clone)]
pub struct AsyncMethodInvoker {
    channel_name: String,
}

impl AsyncMethodInvoker {
    /// Convenience call method that will attempt to convert the result to specified type.
    pub async fn call_method_cv<
        V: Into<Value>,
        R: TryFrom<Value, Error = E>,
        E: Into<TryFromError>,
    >(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
    ) -> Result<R, MethodCallError> {
        let res = self.call_method(target_isolate, method, args).await;
        match res {
            Ok(value) => value
                .try_into()
                .map_err(|e: E| MethodCallError::ConversionError(e.into())),
            Err(err) => Err(err),
        }
    }

    pub async fn call_method<V: Into<Value>>(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
    ) -> Result<Value, MethodCallError> {
        let (
            future, //
            completer,
        ) = FutureCompleter::<Result<Value, MethodCallError>>::new();

        let args: Value = args.into();
        let call: Value = vec![Value::String(method.into()), args].into();
        Context::get().message_channel().send_message(
            target_isolate,
            &self.channel_name,
            call,
            move |res| match res {
                Ok(value) => {
                    let result = unpack_result(value).expect("Malformed message");
                    completer.complete(result);
                }
                Err(err) => completer.complete(Err(MethodCallError::SendError(err))),
            },
        );

        future.await
    }
}

pub struct RegisteredAsyncMethodHandler<T: AsyncMethodHandler> {
    inner: Rc<RegisteredAsyncMethodHandlerInner<T>>,
}

impl<T: AsyncMethodHandler> RegisteredAsyncMethodHandler<T> {
    fn new(channel: &str, handler: T) -> Self {
        Self::new_ref(channel, Rc::new(RefCell::new(handler)))
    }

    fn new_ref(channel: &str, handler: Rc<RefCell<T>>) -> Self {
        let res = Self {
            inner: Rc::new(RegisteredAsyncMethodHandlerInner {
                channel: channel.into(),
                handler,
            }),
        };
        res.inner.init();
        Context::get()
            .message_channel()
            .register_delegate(&res.inner.channel, res.inner.clone());
        res
    }
}

impl<T: AsyncMethodHandler> Drop for RegisteredAsyncMethodHandler<T> {
    fn drop(&mut self) {
        Context::get()
            .message_channel()
            .unregister_delegate(&self.inner.channel);
    }
}

struct RegisteredAsyncMethodHandlerInner<T: AsyncMethodHandler> {
    channel: String,
    handler: Rc<RefCell<T>>,
}

impl<T: AsyncMethodHandler> RegisteredAsyncMethodHandlerInner<T> {
    fn init(&self) {
        let weak = Rc::downgrade(&self.handler);
        self.handler.borrow_mut().assign_weak_self(weak);
        self.handler
            .borrow_mut()
            .assign_invoker(AsyncMethodInvoker {
                channel_name: self.channel.clone(),
            });
    }
}

impl<T: AsyncMethodHandler> MessageChannelDelegate for RegisteredAsyncMethodHandlerInner<T> {
    fn on_isolate_joined(&self, _isolate: IsolateId) {}

    fn on_message(
        &self,
        isolate: IsolateId,
        message: Value,
        reply: Box<dyn FnOnce(Value) -> bool>,
    ) {
        if let Some(call) = unpack_method_call(message, isolate) {
            let handler = self.handler.clone();
            Context::get().run_loop().spawn(async move {
                let handler = handler.borrow();
                let result = handler.on_method_call(call).await;
                MethodCallReply { reply }.send(result);
            });
        } else {
            panic!("malformed method call message");
        }
    }

    fn on_isolate_exited(&self, isolate: IsolateId) {
        self.handler.borrow().on_isolate_destroyed(isolate);
    }
}
