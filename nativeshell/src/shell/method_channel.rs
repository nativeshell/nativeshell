use std::{cell::RefCell, rc::Rc};

use crate::codec::{MethodCall, MethodCallReply, MethodInvoker, Value};

use super::{Context, EngineHandle, Handle};

#[derive(Clone)]
pub struct MethodInvokerProvider {
    context: Rc<Context>,
    channel: String,
}

impl MethodInvokerProvider {
    pub fn get_method_invoker_for_engine(
        &self,
        handle: EngineHandle,
    ) -> Option<MethodInvoker<Value>> {
        return self
            .context
            .message_manager
            .borrow()
            .get_method_invoker(handle, &self.channel);
    }
}

pub trait MethodCallHandler {
    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    );

    // keep the method invoker provider if you want to call methods on engines
    fn set_method_invoker_provider(&mut self, _provider: MethodInvokerProvider) {}

    // called when engine is about to be destroyed
    fn on_engine_destroyed(&mut self, _engine: EngineHandle) {}
}

// Convenience interface for registering custom method call handlers
pub struct MethodChannel {
    context: Rc<Context>,
    channel: String,
    _destroy_engine_handle: Handle,
}

impl MethodChannel {
    pub fn new<H>(context: Rc<Context>, channel: &str, handler: H) -> Self
    where
        H: MethodCallHandler + 'static,
    {
        let handler = Rc::new(RefCell::new(Box::new(handler)));

        let handler_clone = handler.clone();
        let destroy_engine_handle = context
            .engine_manager
            .borrow_mut()
            .register_destroy_engine_notification(move |handle| {
                handler_clone.borrow_mut().on_engine_destroyed(handle);
            });

        let res = Self {
            context: context.clone(),
            channel: channel.into(),
            _destroy_engine_handle: destroy_engine_handle,
        };
        handler
            .as_ref()
            .borrow_mut()
            .set_method_invoker_provider(MethodInvokerProvider {
                context: context.clone(),
                channel: channel.into(),
            });
        context
            .message_manager
            .borrow_mut()
            .register_method_handler(channel, move |call, reply, engine| {
                handler
                    .as_ref()
                    .borrow_mut()
                    .on_method_call(call, reply, engine);
            });
        res
    }
}

impl Drop for MethodChannel {
    fn drop(&mut self) {
        self.context
            .message_manager
            .borrow_mut()
            .unregister_method_handler(&self.channel);
    }
}
