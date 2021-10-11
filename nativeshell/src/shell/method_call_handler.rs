use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

use crate::codec::{MethodCall, MethodCallReply, MethodInvoker, StandardMethodCodec, Value};

use super::{Context, EngineHandle, Handle};

#[derive(Clone)]
pub struct MethodInvokerProvider {
    context: Context,
    channel: String,
}

impl MethodInvokerProvider {
    pub fn get_method_invoker_for_engine(&self, handle: EngineHandle) -> MethodInvoker<Value> {
        MethodInvoker::new(
            self.context.clone(),
            handle,
            self.channel.clone(),
            &StandardMethodCodec,
        )
    }
}

pub trait MethodCallHandler: Sized + 'static {
    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    );

    // Implementation can store weak reference if it needs to pass it around.
    // Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    // Keep the method invoker provider if you want to call methods on engines.
    fn assign_invoker_provider(&mut self, _provider: MethodInvokerProvider) {}

    // Called when engine is about to be destroyed.
    fn on_engine_destroyed(&mut self, _engine: EngineHandle) {}

    // Registers itself for handling platform channel methods.
    fn register(self, context: Context, channel: &str) -> RegisteredMethodCallHandler<Self> {
        RegisteredMethodCallHandler::new(context, channel, self)
    }
}

pub struct RegisteredMethodCallHandler<T: MethodCallHandler> {
    context: Context,
    channel: String,
    _destroy_engine_handle: Handle,
    handler: Rc<RefCell<T>>,
}

// Active method call handler
impl<T: MethodCallHandler> RegisteredMethodCallHandler<T> {
    fn new(context: Context, channel: &str, handler: T) -> Self {
        Self::new_ref(context, channel, Rc::new(RefCell::new(handler)))
    }

    fn new_ref(context: Context, channel: &str, handler: Rc<RefCell<T>>) -> Self {
        let context_ref = context.get().unwrap();

        handler
            .borrow_mut()
            .assign_weak_self(Rc::downgrade(&handler));

        let handler_clone = handler.clone();
        let destroy_engine_handle = context_ref
            .engine_manager
            .borrow_mut()
            .register_destroy_engine_notification(move |handle| {
                handler_clone.borrow_mut().on_engine_destroyed(handle);
            });

        handler
            .borrow_mut()
            .assign_invoker_provider(MethodInvokerProvider {
                context: context.clone(),
                channel: channel.into(),
            });

        let handler_clone = handler.clone();
        context_ref
            .message_manager
            .borrow_mut()
            .register_method_handler(channel, move |call, reply, engine| {
                handler_clone
                    .borrow_mut()
                    .on_method_call(call, reply, engine);
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

impl<T: MethodCallHandler> Drop for RegisteredMethodCallHandler<T> {
    fn drop(&mut self) {
        if let Some(context) = self.context.get() {
            context
                .message_manager
                .borrow_mut()
                .unregister_method_handler(&self.channel);
        }
    }
}

trait Holder {}

impl<T: MethodCallHandler> Holder for RegisteredMethodCallHandler<T> {}

// Convenience interface for registering custom method call handlers
pub struct MethodChannel {
    _registration: Box<dyn Holder>,
}

impl MethodChannel {
    pub fn new<H>(context: Context, channel: &str, handler: H) -> Self
    where
        H: MethodCallHandler,
    {
        Self {
            _registration: Box::new(handler.register(context, channel)),
        }
    }
}
