use std::rc::Rc;

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodCallResult, Value,
    },
    util::OkLog,
    Error, Result,
};

use super::{
    api_constants::{channel, method},
    api_model::{HotKeyCreateRequest, HotKeyDestroyRequest, HotKeyPressed},
    platform::hot_key::PlatformHotKeyManager,
    Context, ContextRef, EngineHandle,
};

pub struct HotKeyManager {
    context: Context,
    platform_manager: Rc<PlatformHotKeyManager>,
    next_handle: HotKeyHandle,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HotKeyHandle(pub(crate) i64);

impl HotKeyManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        let platform_manager = Rc::new(PlatformHotKeyManager::new(context.weak()));
        platform_manager.assign_weak_self(Rc::downgrade(&platform_manager));

        let context_weak = context.weak();
        context
            .message_manager
            .borrow_mut()
            .register_method_handler(channel::HOT_KEY_MANAGER, move |value, reply, engine| {
                if let Some(context) = context_weak.get() {
                    context
                        .hot_key_manager
                        .borrow_mut()
                        .on_method_call(value, reply, engine);
                }
            });

        Self {
            context: context.weak(),
            platform_manager,
            next_handle: HotKeyHandle(1),
        }
    }

    fn on_create(
        &mut self,
        request: HotKeyCreateRequest,
        engine: EngineHandle,
    ) -> Result<HotKeyHandle> {
        let handle = self.next_handle;
        self.next_handle.0 += 1;

        self.platform_manager
            .create_hot_key(request.accelerator, handle.clone(), engine)
            .map_err(Error::from)?;

        Ok(handle)
    }

    fn map_result<T>(result: Result<T>) -> MethodCallResult<Value>
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }

    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::hot_key::CREATE => {
                let request: HotKeyCreateRequest = from_value(&call.args).unwrap();
                let res = self.on_create(request, engine);
                reply.send(Self::map_result(res));
            }
            method::hot_key::DESTROY => {
                let request: HotKeyDestroyRequest = from_value(&call.args).unwrap();
                let res = self
                    .platform_manager
                    .destroy_hot_key(request.handle)
                    .map_err(Error::from);
                reply.send(Self::map_result(res));
            }
            _ => {}
        }
    }

    pub(crate) fn on_hot_key_pressed(&self, handle: HotKeyHandle, engine: EngineHandle) {
        if let Some(invoker) = self.context.get().and_then(|context| {
            context
                .message_manager
                .borrow()
                .get_method_invoker(engine, &channel::HOT_KEY_MANAGER)
        }) {
            invoker
                .call_method(
                    method::hot_key::ON_PRESSED.into(),
                    to_value(&HotKeyPressed { handle: handle }).unwrap(),
                    |_| {},
                )
                .ok_log();
        }
    }
}
