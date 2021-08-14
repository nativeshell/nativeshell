use super::{binary_messenger::PlatformBinaryMessenger, error::PlatformResult};

pub type PlatformEngineType = isize;

pub struct PlatformEngine {
    pub(crate) handle: PlatformEngineType,
}

pub type PlatformPlugin = isize;

impl PlatformEngine {
    pub fn new(_plugins: &[PlatformPlugin]) -> Self {
        PlatformEngine { handle: 0 }
    }

    pub fn new_binary_messenger(&self) -> PlatformBinaryMessenger {
        PlatformBinaryMessenger {}
    }

    pub fn launch(&mut self) -> PlatformResult<()> {
        Err(super::error::PlatformError::NotImplemented)
    }

    pub fn shut_down(&mut self) -> PlatformResult<()> {
        Err(super::error::PlatformError::NotImplemented)
    }
}
