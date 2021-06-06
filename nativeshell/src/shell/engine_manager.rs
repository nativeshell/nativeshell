use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

use super::{Context, FlutterEngine};
use crate::{Error, Result};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct EngineHandle(pub i64);

pub struct EngineManager {
    context: Rc<Context>,
    engines: HashMap<EngineHandle, Box<RefCell<FlutterEngine>>>,
    next_handle: EngineHandle,
}

impl EngineManager {
    pub(super) fn new(context: Rc<Context>) -> Self {
        Self {
            context,
            engines: HashMap::new(),
            next_handle: EngineHandle(1),
        }
    }

    pub fn create_engine(&mut self) -> EngineHandle {
        let engine = FlutterEngine::create(&self.context.options.flutter_plugins);
        let handle = self.next_handle;
        self.next_handle.0 += 1;
        self.engines.insert(handle, Box::new(RefCell::new(engine)));
        self.context
            .message_manager
            .borrow_mut()
            .engine_created(self, handle);
        handle
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

    pub fn remove_engine(&mut self, handle: EngineHandle) -> Result<()> {
        let entry = self.engines.remove(&handle);
        if let Some(entry) = entry {
            let mut engine = entry.borrow_mut();
            engine.shut_down()?;
        }
        if self.engines.is_empty() {
            (self.context.options.on_last_engine_removed)(self.context.clone());
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
