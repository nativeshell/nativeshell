use std::{cell::RefCell, rc::Weak};

use crate::shell::{
    api_model::Accelerator, Context, EngineHandle, HotKeyHandle, HotKeyManagerDelegate,
};

use super::error::{PlatformError, PlatformResult};

pub(crate) struct PlatformHotKeyManager {}

impl PlatformHotKeyManager {
    pub fn new(_context: Context, _delegate: Weak<RefCell<dyn HotKeyManagerDelegate>>) -> Self {
        Self {}
    }

    pub fn assign_weak_self(&self, _weak: Weak<PlatformHotKeyManager>) {}

    pub fn create_hot_key(
        &self,
        _accelerator: Accelerator,
        _virtual_key: i64,
        _handle: HotKeyHandle,
        _engine: EngineHandle,
    ) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn destroy_hot_key(&self, _handle: HotKeyHandle) -> PlatformResult<()> {
        Err(PlatformError::NotAvailable)
    }

    pub fn engine_destroyed(&self, _engine: EngineHandle) -> PlatformResult<()> {
        Ok(())
    }
}
