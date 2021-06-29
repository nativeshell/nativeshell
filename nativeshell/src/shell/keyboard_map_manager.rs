use std::{collections::HashSet, rc::Rc};

use crate::{
    codec::{value::to_value, MethodCall, MethodCallReply, Value},
    util::OkLog,
};

use super::{
    api_constants::{channel, method},
    platform::keyboard_map::PlatformKeyboardMap,
    Context, ContextRef, EngineHandle, Handle,
};

pub struct KeyboardMapManager {
    context: Context,
    pub(crate) platform_map: Rc<PlatformKeyboardMap>,
    _engine_destroy_notification_handle: Handle,
    engines: HashSet<EngineHandle>,
}

impl KeyboardMapManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        let platform_layout = Rc::new(PlatformKeyboardMap::new(context.weak()));
        platform_layout.assign_weak_self(Rc::downgrade(&platform_layout));

        let context_weak = context.weak();
        context
            .message_manager
            .borrow_mut()
            .register_method_handler(
                channel::KEYBOARD_MAP_MANAGER,
                move |value, reply, engine| {
                    if let Some(context) = context_weak.get() {
                        context
                            .keyboard_map_manager
                            .borrow_mut()
                            .on_method_call(value, reply, engine);
                    }
                },
            );

        let context_weak = context.weak();
        let handle = context
            .engine_manager
            .borrow_mut()
            .register_destroy_engine_notification(move |engine| {
                if let Some(context) = context_weak.get() {
                    context
                        .keyboard_map_manager
                        .borrow_mut()
                        .on_engine_destroyed(engine);
                }
            });

        Self {
            context: context.weak(),
            platform_map: platform_layout,
            _engine_destroy_notification_handle: handle,
            engines: HashSet::new(),
        }
    }

    fn on_method_call(
        &mut self,
        call: MethodCall<Value>,
        reply: MethodCallReply<Value>,
        engine: EngineHandle,
    ) {
        match call.method.as_str() {
            method::keyboard_map::GET => {
                self.engines.insert(engine);
                let layout = self.platform_map.get_current_map();
                reply.send_ok(to_value(layout).unwrap());
            }
            _ => {}
        }
    }

    pub(crate) fn keyboard_layout_changed(&self) {
        if let Some(context) = self.context.get() {
            let layout = self.platform_map.get_current_map();
            let layout = to_value(layout).unwrap();
            for engine in &self.engines {
                let sender = context
                    .message_manager
                    .borrow()
                    .get_method_invoker(engine.clone(), channel::KEYBOARD_MAP_MANAGER);
                if let Some(sender) = sender {
                    sender
                        .call_method(
                            method::keyboard_map::ON_CHANGED.into(),
                            layout.clone(),
                            |_| {},
                        )
                        .ok_log();
                }
            }
        }
    }

    fn on_engine_destroyed(&mut self, engine: EngineHandle) {
        self.engines.remove(&engine);
    }
}
