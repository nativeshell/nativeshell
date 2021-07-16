use std::sync::{Arc, Mutex};

use io_surface::IOSurface;

use super::{platform::texture::PlatformTexture, Context, EngineHandle};

pub struct Texture {
    context: Context,
    engine: EngineHandle,
    id: i64,
    texture: Arc<Mutex<PlatformTexture>>,
}

impl Texture {
    pub fn new(context: Context, engine: EngineHandle) -> Option<Self> {
        if let Some(context) = context.get() {
            let manager = context.engine_manager.borrow();
            let engine_ref = manager.get_engine(engine);
            if let Some(engine_ref) = engine_ref {
                let registry = engine_ref.platform_engine.texture_registry();
                let texture = PlatformTexture::new();
                let id = registry.register_texture(texture.clone());
                return Some(Texture {
                    context: context.weak(),
                    engine,
                    id,
                    texture,
                });
            }
        }
        None
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn update(&self, surface: IOSurface) {
        let mut texture = self.texture.lock().unwrap();
        texture.set_surface(surface);
        if let Some(context) = self.context.get() {
            let manager = context.engine_manager.borrow();
            let engine_ref = manager.get_engine(self.engine);
            if let Some(engine_ref) = engine_ref {
                let registry = engine_ref.platform_engine.texture_registry();
                registry.texture_frame_available(self.id);
            }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if let Some(context) = self.context.get() {
            let manager = context.engine_manager.borrow();
            let engine_ref = manager.get_engine(self.engine);
            if let Some(engine_ref) = engine_ref {
                let registry = engine_ref.platform_engine.texture_registry();
                registry.unregister_texture(self.id);
            }
        }
    }
}
