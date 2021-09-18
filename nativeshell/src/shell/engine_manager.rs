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
    next_notification: i64,
    create_notifications: HashMap<i64, Box<dyn Fn(EngineHandle, &FlutterEngine)>>,
    destroy_notifications: HashMap<i64, Box<dyn Fn(EngineHandle)>>,
}

impl EngineManager {
    pub(super) fn new(context: &ContextRef) -> Self {
        Self {
            context: context.weak(),
            engines: HashMap::new(),
            next_handle: EngineHandle(1),
            next_notification: 1,
            create_notifications: HashMap::new(),
            destroy_notifications: HashMap::new(),
        }
    }

    pub fn create_engine(&mut self, parent_engine: Option<EngineHandle>) -> Result<EngineHandle> {
        if let Some(context) = self.context.get() {
            let engine = FlutterEngine::new(&context.options.flutter_plugins, parent_engine);
            let handle = self.next_handle;

            for n in self.create_notifications.values() {
                n(handle, &engine);
            }

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

    // Returns the handle of engine responsible for creating provided engine. It is valid
    // for this method to return handle to engine that is no longer active.
    pub fn get_parent_engine(&self, handle: EngineHandle) -> Option<EngineHandle> {
        self.engines
            .get(&handle)
            .and_then(|e| e.borrow().parent_engine)
    }

    #[must_use]
    pub fn register_create_engine_notification<F>(&mut self, notification: F) -> Handle
    where
        F: Fn(EngineHandle, &FlutterEngine) + 'static,
    {
        let handle = self.next_notification;
        self.next_notification += 1;

        self.create_notifications
            .insert(handle, Box::new(notification));

        let context = self.context.clone();
        Handle::new(move || {
            if let Some(context) = context.get() {
                context
                    .engine_manager
                    .borrow_mut()
                    .create_notifications
                    .remove(&handle);
            }
        })
    }

    #[must_use]
    pub fn register_destroy_engine_notification<F>(&mut self, notification: F) -> Handle
    where
        F: Fn(EngineHandle) + 'static,
    {
        let handle = self.next_notification;
        self.next_notification += 1;

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

    pub fn shut_down(&mut self) -> Result<()> {
        let engines = self.get_all_engines();
        for engine in engines {
            self.remove_engine(engine)?;
        }
        Ok(())
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
