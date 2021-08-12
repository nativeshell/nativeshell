use std::sync::{Arc, Mutex};

use crate::shell::PixelBufferFormat;

pub struct PlatformTexture {}

pub trait TexturePayload {}

pub(crate) const PIXEL_BUFFER_FORMAT: PixelBufferFormat = PixelBufferFormat::BGRA;

impl PlatformTexture {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {}))
    }

    pub fn set_payload<T: TexturePayload>(&mut self, payload: T) {}
}

pub(crate) struct PlatformTextureRegistry {}

impl PlatformTextureRegistry {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_texture<T: TexturePayload>(&self, texture: Arc<Mutex<PlatformTexture>>) -> i64 {
        0
    }

    pub fn texture_frame_available(&self, texture: i64) {}

    pub fn unregister_texture(&self, texture: i64) {}
}
