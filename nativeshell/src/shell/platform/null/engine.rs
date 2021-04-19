use super::{binary_messenger::PlatformBinaryMessenger, error::PlatformResult};

pub struct PlatformEngine {}

impl PlatformEngine {
    pub fn new() -> Self {
        PlatformEngine {}
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
