use std::{
    cell::RefCell,
    collections::HashSet,
    rc::{Rc, Weak},
};

use crate::{
    codec::value::to_value,
    util::{Late, OkLog},
};

use super::{
    api_constants::{channel, method},
    platform::keyboard_map::PlatformKeyboardMap,
    Context, EngineHandle, MethodCallHandler, MethodInvokerProvider, RegisteredMethodCallHandler,
};

pub struct KeyboardMapManager {
    context: Context,
    pub(crate) platform_map: Late<Rc<PlatformKeyboardMap>>,
    engines: HashSet<EngineHandle>,
    provider: Late<MethodInvokerProvider>,
}

pub trait KeyboardMapDelegate {
    fn keyboard_map_did_change(&self);
}

impl KeyboardMapManager {
    pub fn new(context: Context) -> RegisteredMethodCallHandler<Self> {
        Self {
            context: context.clone(),
            platform_map: Late::new(),
            engines: HashSet::new(),
            provider: Late::new(),
        }
        .register(context, channel::KEYBOARD_MAP_MANAGER)
    }
}

impl MethodCallHandler for KeyboardMapManager {
    fn on_method_call(
        &mut self,
        call: crate::codec::MethodCall<crate::codec::Value>,
        reply: crate::codec::MethodCallReply<crate::codec::Value>,
        engine: EngineHandle,
    ) {
        #[allow(clippy::single_match)]
        match call.method.as_str() {
            method::keyboard_map::GET => {
                self.engines.insert(engine);
                let layout = self.platform_map.get_current_map();
                reply.send_ok(to_value(layout).unwrap());
            }
            _ => {}
        }
    }

    fn assign_weak_self(&mut self, weak_self: Weak<RefCell<Self>>) {
        let delegate: Weak<RefCell<dyn KeyboardMapDelegate>> = weak_self;
        self.platform_map.set(Rc::new(PlatformKeyboardMap::new(
            self.context.clone(),
            delegate,
        )));
        self.platform_map
            .assign_weak_self(Rc::downgrade(&self.platform_map));
    }

    fn assign_invoker_provider(&mut self, provider: MethodInvokerProvider) {
        self.provider.set(provider);
    }

    // called when engine is about to be destroyed
    fn on_engine_destroyed(&mut self, engine: EngineHandle) {
        self.engines.remove(&engine);
    }
}

impl KeyboardMapDelegate for KeyboardMapManager {
    fn keyboard_map_did_change(&self) {
        let layout = self.platform_map.get_current_map();
        let layout = to_value(layout).unwrap();
        for engine in &self.engines {
            let invoker = self.provider.get_method_invoker_for_engine(*engine);
            invoker
                .call_method(method::keyboard_map::ON_CHANGED, layout.clone(), |_| {})
                .ok_log();
        }
    }
}
