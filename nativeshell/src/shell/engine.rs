use super::{platform::engine::PlatformEngine, BinaryMessenger};
use crate::Result;

pub struct FlutterEngine {
    pub(super) platform_engine: PlatformEngine,
    binary_messenger: Option<BinaryMessenger>,
}

impl FlutterEngine {
    pub fn create() -> Self {
        let platform_engine = PlatformEngine::new();

        let messenger = BinaryMessenger::new(platform_engine.new_binary_messenger());
        FlutterEngine {
            platform_engine,
            binary_messenger: Some(messenger),
        }
    }

    pub fn binary_messenger(&self) -> &BinaryMessenger {
        self.binary_messenger.as_ref().unwrap()
    }

    pub fn launch(&mut self) -> Result<()> {
        self.platform_engine.launch().map_err(|e| e.into())
    }

    pub fn shut_down(&mut self) -> Result<()> {
        self.binary_messenger.take();
        self.platform_engine.shut_down().map_err(|e| e.into())
    }
}
