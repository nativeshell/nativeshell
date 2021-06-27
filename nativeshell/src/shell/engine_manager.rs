use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
};

use super::{Context, ContextRef, FlutterEngine, Handle};
use crate::{Error, Result};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct EngineHandle(pub i64);

pub struct EngineManager {
    context: Context,
    engines: HashMap<EngineHandle, Box<RefCell<FlutterEngine>>>,
    next_handle: EngineHandle,
    next_destroy_notification: i64,
    destroy_notifications: HashMap<i64, Box<dyn Fn(EngineHandle)>>,
}

impl EngineManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        Self {
            context: context.weak(),
            engines: HashMap::new(),
            next_handle: EngineHandle(1),
            next_destroy_notification: 1,
            destroy_notifications: HashMap::new(),
        }
    }

    pub fn create_engine(&mut self) -> Result<EngineHandle> {
        if let Some(context) = self.context.get() {
            let engine = FlutterEngine::create(&context.options.flutter_plugins);
            let handle = self.next_handle;
            self.next_handle.0 += 1;
            self.engines.insert(handle, Box::new(RefCell::new(engine)));
            context
                .message_manager
                .borrow_mut()
                .engine_created(self, handle);
            Ok(handle)
        } else {
            Err(Error::InvalidContext)
        }
    }

    pub fn launch_engine(&mut self, handle: EngineHandle) -> Result<()> {
        self.engines
            .get(&handle)
            .map(|engine| engine.borrow_mut().launch())
            .transpose()?
            .ok_or(Error::InvalidEngineHandle)
    }

    pub fn get_engine(&self, handle: EngineHandle) -> Option<Ref<FlutterEngine>> {
        self.engines.get(&handle).map(|a| a.borrow())
    }

    #[must_use]
    pub fn register_destroy_engine_notification<F>(&mut self, notification: F) -> Handle
    where
        F: Fn(EngineHandle) + 'static,
    {
        let handle = self.next_destroy_notification;
        self.next_destroy_notification += 1;

        self.destroy_notifications
            .insert(handle, Box::new(notification));

        let context = self.context.clone();
        Handle::new(move || {
            if let Some(context) = context.get() {
                context
                    .engine_manager
                    .borrow_mut()
                    .destroy_notifications
                    .remove(&handle);
            }
        })
    }

    pub fn remove_engine(&mut self, handle: EngineHandle) -> Result<()> {
        for n in self.destroy_notifications.values() {
            n(handle);
        }

        let entry = self.engines.remove(&handle);
        if let Some(entry) = entry {
            let mut engine = entry.borrow_mut();
            engine.shut_down()?;
        }
        if self.engines.is_empty() {
            if let Some(context) = self.context.get() {
                (context.options.on_last_engine_removed)(&context);
            }
        }
        Ok(())
    }

    pub fn get_all_engines(&self) -> Vec<EngineHandle> {
        self.engines.keys().cloned().collect()
    }

    // Posts message on all engines
    pub fn broadcast_message(&self, channel: &str, message: &[u8]) -> Result<()> {
        for engine in self.engines.values() {
            engine
                .borrow()
                .binary_messenger()
                .post_message(channel, message)?;
        }
        Ok(())
    }
}
