use std::{cell::RefCell, collections::HashSet, rc::Weak};

use crate::{
    codec::{value::to_value, Value},
    util::{Late, OkLog},
    Context,
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
}

impl MethodCallHandler for ScreenManager {
    fn on_method_call(
        &mut self,
        call: crate::codec::MethodCall<crate::codec::Value>,
        reply: crate::codec::MethodCallReply<crate::codec::Value>,
        engine: super::EngineHandle,
    ) {
        self.engines.insert(engine.clone());
        match call.method.as_str() {
            method::screen_manager::GET_SCREENS => {
                let screens = self.platform_manager.get_screens();
                reply.send_ok(to_value(screens).unwrap());
            }
            method::screen_manager::GET_MAIN_SCREEN => {
                let id = self.platform_manager.get_main_screen();
                reply.send_ok(to_value(id).unwrap());
            }
            // macOS does the mapping
            method::screen_manager::LOGICAL_TO_SYSTEM => {
                reply.send_ok(call.args);
            }
            method::screen_manager::SYSTEM_TO_LOGICAL => {
                reply.send_ok(call.args);
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
