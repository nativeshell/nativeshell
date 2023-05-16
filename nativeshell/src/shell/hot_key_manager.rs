use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCall, MethodCallReply, MethodCallResult, Value,
    },
    util::{Late, OkLog},
    Error, Result,
};

use super::{
    api_constants::{channel, method},
    api_model::{HotKeyCreateRequest, HotKeyDestroyRequest, HotKeyPressed},
    platform::hot_key::PlatformHotKeyManager,
    Context, EngineHandle, MethodCallHandler, MethodInvokerProvider, RegisteredMethodCallHandler,
};

pub struct HotKeyManager {
    context: Context,
    platform_manager: Late<Rc<PlatformHotKeyManager>>,
    next_handle: HotKeyHandle,
    invoker_provider: Late<MethodInvokerProvider>,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HotKeyHandle(pub(crate) i64);

pub trait HotKeyManagerDelegate {
    fn on_hot_key_pressed(&self, handle: HotKeyHandle, engine: EngineHandle);
}

impl HotKeyManager {
    pub(super) fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        Self {
            context: context.clone(),
            platform_manager: Late::new(),
            next_handle: HotKeyHandle(1),
            invoker_provider: Late::new(),
        }
        .register(context, channel::HOT_KEY_MANAGER)
    }

    fn on_create(
        &mut self,
        request: HotKeyCreateRequest,
        engine: EngineHandle,
    ) -> Result<HotKeyHandle> {
        let handle = self.next_handle;
        self.next_handle.0 += 1;

        self.platform_manager
            .create_hot_key(request.accelerator, request.platform_key, handle, engine)
            .map_err(Error::from)?;

        Ok(handle)
    }

    fn map_result<T>(result: Result<T>) -> MethodCallResult<Value>
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }
}

impl HotKeyManagerDelegate for HotKeyManager {
    fn on_hot_key_pressed(&self, handle: HotKeyHandle, engine: EngineHandle) {
        let invoker = self.invoker_provider.get_method_invoker_for_engine(engine);
        invoker
            .call_method(
                method::hot_key::ON_PRESSED,
                to_value(HotKeyPressed { handle }).unwrap(),
                |_| {},
            )
            .ok_log();
    }
}

impl MethodCallHandler for HotKeyManager {
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

    fn assign_weak_self(&mut self, weak_self: std::rc::Weak<std::cell::RefCell<Self>>) {
        let delegate: Weak<RefCell<dyn HotKeyManagerDelegate>> = weak_self;
        self.platform_manager
            .set(Rc::new(PlatformHotKeyManager::new(
                self.context.clone(),
                delegate,
            )));
        self.platform_manager
            .assign_weak_self(Rc::downgrade(&self.platform_manager));
    }

    fn assign_invoker_provider(&mut self, provider: MethodInvokerProvider) {
        self.invoker_provider.set(provider);
    }

    fn on_engine_destroyed(&mut self, engine: EngineHandle) {
        self.platform_manager.engine_destroyed(engine).ok_log();
    }
}
