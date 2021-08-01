use std::{cell::RefCell, rc::Weak};

use crate::shell::{
    api_model::Accelerator, Context, EngineHandle, HotKeyHandle, HotKeyManagerDelegate,
};

use super::error::PlatformResult;

pub(crate) struct PlatformHotKeyManager {}

impl PlatformHotKeyManager {
    pub fn new(context: Context, delegate: Weak<RefCell<dyn HotKeyManagerDelegate>>) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformHotKeyManager>) {}

    pub fn create_hot_key(
        &self,
        accelerator: Accelerator,
        virtual_key: i64,
        handle: HotKeyHandle,
        engine: EngineHandle,
    ) -> PlatformResult<()> {
        Ok(())
    }

    pub fn destroy_hot_key(&self, handle: HotKeyHandle) -> PlatformResult<()> {
        Ok(())
    }

    pub fn engine_destroyed(&self, engine: EngineHandle) -> PlatformResult<()> {
        Ok(())
    }
}
