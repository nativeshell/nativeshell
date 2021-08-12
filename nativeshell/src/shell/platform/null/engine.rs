use super::{
    binary_messenger::PlatformBinaryMessenger, error::PlatformResult,
    texture::PlatformTextureRegistry,
};

pub struct PlatformEngine {
    texture_registry: PlatformTextureRegistry,
}

pub type PlatformPlugin = isize;

impl PlatformEngine {
    pub fn new(_plugins: &[PlatformPlugin]) -> Self {
        PlatformEngine {
            texture_registry: PlatformTextureRegistry::new(),
        }
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

    pub(crate) fn texture_registry(&self) -> &PlatformTextureRegistry {
        &self.texture_registry
    }
}
