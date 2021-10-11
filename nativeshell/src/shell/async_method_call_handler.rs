use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        MethodCall, MethodCallError, MethodCallResult, MethodInvoker, StandardMethodCodec, Value,
    },
    util::FutureCompleter,
    Error,
};

use async_trait::async_trait;

use super::{Context, EngineHandle, Handle};

#[derive(Clone)]
pub struct AsyncMethodInvoker {
    context: Context,
    channel: String,
}

#[derive(Debug)]
pub enum AsyncMethodCallError<V> {
    // Error originating from NativeShell, such as invalid engine handle
    ShellError(Error),
    // Error originating from Flutter code
    MethodCallError(MethodCallError<V>),
}

impl std::fmt::Display for AsyncMethodCallError<Value> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AsyncMethodCallError::ShellError(error) => write!(f, "NativeShell Error: {}", error),
            AsyncMethodCallError::MethodCallError(error) => write!(f, "MethodCallError: {}", error),
        }
    }
}

impl std::error::Error for AsyncMethodCallError<Value> {}

pub type AsyncMethodCallResult<V> = std::result::Result<V, AsyncMethodCallError<V>>;

impl AsyncMethodInvoker {
    pub async fn call_method(
        &self,
        engine: EngineHandle,
        method: &str,
        args: Value,
    ) -> AsyncMethodCallResult<Value> {
        let invoker = MethodInvoker::new(
            self.context.clone(),
            engine,
            self.channel.clone(),
            &StandardMethodCodec,
        );
        let (
            future, //
            completer,
        ) = FutureCompleter::<AsyncMethodCallResult<Value>>::new();

        let completer = Rc::new(RefCell::new(Some(completer)));
        let completer_clone = completer.clone();
        let res = invoker.call_method(method, args, move |reply| {
            let completer = completer_clone.borrow_mut().take().expect(
                "Reply block was invoked more than once or after call_method failed with error.",
            );
            match reply {
                Ok(value) => completer.complete(Ok(value)),
                Err(error) => completer.complete(Err(AsyncMethodCallError::MethodCallError(error))),
            }
        });
        if let Err(error) = res {
            let completer = completer
                .take()
                .expect("call_method failed with error but reply block was already invoked.");
            completer.complete(Err(AsyncMethodCallError::ShellError(error)));
        };
        future.await
    }
}

#[async_trait(?Send)]
pub trait AsyncMethodCallHandler: Sized + 'static {
    async fn on_method_call(
        &self,
        call: MethodCall<Value>,
        engine: EngineHandle,
    ) -> MethodCallResult<Value>;

    // Implementation can store weak reference if it needs to pass it around.
    // Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    // Keep the method invoker provider if you want to call methods on engines.
    fn assign_invoker(&mut self, _invoker: AsyncMethodInvoker) {}

    // Called when engine is about to be destroyed.
    fn on_engine_destroyed(&self, _engine: EngineHandle) {}

    // Registers itself for handling platform channel methods.
    fn register(self, context: Context, channel: &str) -> RegisteredAsyncMethodCallHandler<Self> {
        RegisteredAsyncMethodCallHandler::new(context, channel, self)
    }
}

pub struct RegisteredAsyncMethodCallHandler<T: AsyncMethodCallHandler> {
    context: Context,
    channel: String,
    _destroy_engine_handle: Handle,
    handler: Rc<RefCell<T>>,
}

// Active method call handler
impl<T: AsyncMethodCallHandler> RegisteredAsyncMethodCallHandler<T> {
    pub fn new(context: Context, channel: &str, handler: T) -> Self {
        Self::new_ref(context, channel, Rc::new(RefCell::new(handler)))
    }

    pub fn new_ref(context: Context, channel: &str, handler: Rc<RefCell<T>>) -> Self {
        let context_ref = context.get().unwrap();

        handler
            .borrow_mut()
            .assign_weak_self(Rc::downgrade(&handler));

        let handler_clone = handler.clone();
        let destroy_engine_handle = context_ref
            .engine_manager
            .borrow_mut()
            .register_destroy_engine_notification(move |handle| {
                handler_clone.borrow().on_engine_destroyed(handle);
            });

        handler.borrow_mut().assign_invoker(AsyncMethodInvoker {
            context: context.clone(),
            channel: channel.into(),
        });

        let handler_clone = handler.clone();
        let context_clone = context.clone();
        context_ref
            .message_manager
            .borrow_mut()
            .register_method_handler(channel, move |call, reply, engine| {
                if let Some(context) = context_clone.get() {
                    let handler_clone = handler_clone.clone();
                    context.run_loop.borrow().spawn(async move {
                        let result = handler_clone.borrow().on_method_call(call, engine).await;
                        reply.send(result);
                    });
                }
            });

        Self {
            context,
            channel: channel.into(),
            _destroy_engine_handle: destroy_engine_handle,
            handler,
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        self.handler.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.handler.borrow_mut()
    }
}

impl<T: AsyncMethodCallHandler> Drop for RegisteredAsyncMethodCallHandler<T> {
    fn drop(&mut self) {
        if let Some(context) = self.context.get() {
            context
                .message_manager
                .borrow_mut()
                .unregister_method_handler(&self.channel);
        }
    }
}
