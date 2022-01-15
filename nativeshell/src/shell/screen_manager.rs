use std::{cell::RefCell, collections::HashSet, rc::Weak};

use crate::{
    codec::{
        value::{from_value, to_value},
        MethodCallResult, Value,
    },
    util::{Late, OkLog},
    Context, Error, Result,
};

use super::{
    api_constants::{channel, method},
    platform::screen_manager::PlatformScreenManager,
    EngineHandle, MethodCallHandler, MethodInvokerProvider, RegisteredMethodCallHandler,
};

pub trait ScreenManagerDelegate {
    fn screen_configuration_changed(&self);
}

pub struct ScreenManager {
    platform_manager: Late<PlatformScreenManager>,
    invoker_provider: Late<MethodInvokerProvider>,
    engines: HashSet<EngineHandle>,
}

impl ScreenManager {
    pub(super) fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        Self {
            platform_manager: Late::new(),
            invoker_provider: Late::new(),
            engines: HashSet::new(),
        }
        .register(context, channel::SCREEN_MANAGER)
    }

    fn map_result<T>(result: Result<T>) -> MethodCallResult<Value>
    where
        T: serde::Serialize,
    {
        result.map(|v| to_value(v).unwrap()).map_err(|e| e.into())
    }
}

impl MethodCallHandler for ScreenManager {
    fn on_method_call(
        &mut self,
        call: crate::codec::MethodCall<Value>,
        reply: crate::codec::MethodCallReply<Value>,
        engine: super::EngineHandle,
    ) {
        self.engines.insert(engine);
        match call.method.as_str() {
            method::screen_manager::GET_SCREENS => {
                let screens = self.platform_manager.get_screens();
                reply.send(Self::map_result(screens.map_err(Error::from)));
            }
            method::screen_manager::GET_MAIN_SCREEN => {
                let id = self.platform_manager.get_main_screen();
                reply.send(Self::map_result(id.map_err(Error::from)));
            }
            method::screen_manager::LOGICAL_TO_SYSTEM => {
                let offset = self
                    .platform_manager
                    .logical_to_system(from_value(&call.args).unwrap());
                reply.send(Self::map_result(offset.map_err(Error::from)));
            }
            method::screen_manager::SYSTEM_TO_LOGICAL => {
                let offset = self
                    .platform_manager
                    .system_to_logical(from_value(&call.args).unwrap());
                reply.send(Self::map_result(offset.map_err(Error::from)));
            }
            _ => {}
        }
    }

    fn on_engine_destroyed(&mut self, engine: EngineHandle) {
        self.engines.remove(&engine);
    }

    fn assign_weak_self(&mut self, weak_self: Weak<RefCell<Self>>) {
        self.platform_manager
            .set(PlatformScreenManager::new(weak_self));
    }

    fn assign_invoker_provider(&mut self, provider: MethodInvokerProvider) {
        self.invoker_provider.set(provider);
    }
}

impl ScreenManagerDelegate for ScreenManager {
    fn screen_configuration_changed(&self) {
        for engine in &self.engines {
            let invoker = self.invoker_provider.get_method_invoker_for_engine(*engine);
            invoker
                .call_method(method::screen_manager::SCREENS_CHANGED, Value::Null, |_| {})
                .ok_log();
        }
    }
}
