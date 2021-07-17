use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use super::{
    platform::texture::{PlatformTexture, TexturePayload, PIXEL_BUFFER_FORMAT},
    Context, EngineHandle,
};

pub struct Texture<Payload> {
    context: Context,
    engine: EngineHandle,
    id: i64,
    texture: Arc<Mutex<PlatformTexture>>,
    _phantom: PhantomData<Payload>,
}

pub enum PixelBufferFormat {
    BGRA,
}

pub struct PixelBuffer {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

impl PixelBuffer {
    pub fn format() -> PixelBufferFormat {
        PIXEL_BUFFER_FORMAT
    }
}

impl<Payload: TexturePayload> Texture<Payload> {
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
                    _phantom: PhantomData::<Payload> {},
                });
            }
        }
        None
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn update(&self, payload: Payload) {
        let mut texture = self.texture.lock().unwrap();
        texture.set_payload(payload);
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

impl<T> Drop for Texture<T> {
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
