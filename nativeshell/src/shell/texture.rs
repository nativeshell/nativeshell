use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{Error, Result};

use super::{
    platform::texture::{PlatformTexture, TexturePayload, PIXEL_BUFFER_FORMAT},
    Context, EngineHandle,
};

#[allow(clippy::upper_case_acronyms)]
pub enum PixelBufferFormat {
    BGRA,
    RGBA,
}

// PixelBuffer texture payload
//
// Supported on all platforms, although the pixel format may vary.
// See PixelBuffer::FORMAT.
pub struct PixelBuffer {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

impl PixelBuffer {
    pub const FORMAT: PixelBufferFormat = PIXEL_BUFFER_FORMAT;
}

// OpenGL texture payload
//
// Supported on Linux
pub struct GLTexture {
    pub target: u32, // texture target (i.e. GL_TEXTURE_2D or GL_TEXTURE_RECTANGLE)
    pub name: u32,   // OpenGL texture name
    pub width: i32,
    pub height: i32,
}

// Texture
//
// Supported payload types:
// - PixelBuffer (all platforms)
// - IOSurface (macOS)
// - GLTexture (Linux)
//
// Usage:
//   let texture = Texture::<PixelBuffer>::new(context, engine).unwrap();
//   texture.update(PixelBuffer{...});
pub struct Texture<Payload> {
    context: Context,
    engine: EngineHandle,
    id: i64,
    texture: Arc<Mutex<PlatformTexture>>,
    _phantom: PhantomData<Payload>,
}

impl<Payload: TexturePayload> Texture<Payload> {
    pub fn new(context: Context, engine: EngineHandle) -> Result<Self> {
        if let Some(context) = context.get() {
            let manager = context.engine_manager.borrow();
            let engine_ref = manager.get_engine(engine);
            if let Some(engine_ref) = engine_ref {
                let registry = engine_ref.platform_engine.texture_registry();
                let texture = PlatformTexture::new();
                let id = registry.register_texture::<Payload>(texture.clone());
                Ok(Texture {
                    context: context.weak(),
                    engine,
                    id,
                    texture,
                    _phantom: PhantomData::<Payload> {},
                })
            } else {
                Err(Error::InvalidEngineHandle)
            }
        } else {
            Err(Error::InvalidContext)
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn update(&self, payload: Payload) {
        {
            let mut texture = self.texture.lock().unwrap();
            texture.set_payload(payload);
        }
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
