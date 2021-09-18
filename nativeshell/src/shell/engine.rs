use super::{
    platform::engine::{PlatformEngine, PlatformEngineType, PlatformPlugin},
    BinaryMessenger, EngineHandle,
};
use crate::Result;

pub struct FlutterEngine {
    pub(super) platform_engine: PlatformEngine,
    pub(super) parent_engine: Option<EngineHandle>,
    binary_messenger: Option<BinaryMessenger>,
}

impl FlutterEngine {
    pub fn new(plugins: &[PlatformPlugin], parent_engine: Option<EngineHandle>) -> Self {
        let platform_engine = PlatformEngine::new(plugins);

        let messenger = BinaryMessenger::new(platform_engine.new_binary_messenger());
        FlutterEngine {
            platform_engine,
            parent_engine,
            binary_messenger: Some(messenger),
        }
    }

    pub fn binary_messenger(&self) -> &BinaryMessenger {
        self.binary_messenger.as_ref().unwrap()
    }

    pub fn platform_engine(&self) -> PlatformEngineType {
        #[allow(clippy::clone_on_copy)]
        self.platform_engine.handle.clone()
    }

    pub fn launch(&mut self) -> Result<()> {
        self.platform_engine.launch().map_err(|e| e.into())
    }

    pub fn shut_down(&mut self) -> Result<()> {
        self.binary_messenger.take();
        self.platform_engine.shut_down().map_err(|e| e.into())
    }
}
