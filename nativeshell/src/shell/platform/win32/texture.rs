use std::sync::{Arc, Mutex};

use crate::shell::{PixelBuffer, PixelBufferFormat};

use super::flutter_sys::{
    kFlutterDesktopPixelBufferTexture, size_t, FlutterDesktopPixelBuffer,
    FlutterDesktopPixelBufferTextureConfig, FlutterDesktopTextureInfo,
    FlutterDesktopTextureInfo__bindgen_ty_1,
    FlutterDesktopTextureRegistrarMarkExternalTextureFrameAvailable,
    FlutterDesktopTextureRegistrarRef, FlutterDesktopTextureRegistrarRegisterExternalTexture,
    FlutterDesktopTextureRegistrarUnregisterExternalTexture, FlutterDesktopTextureType,
};

pub struct PlatformTexture {
    // PixelBuffer Texture
    // this is where data is kept until next callback
    pixel_buffer: Option<PixelBuffer>,
    // used to keep alive FlutterDesktopPixelBuffer struct
    flutter_pixel_buffer: Option<FlutterDesktopPixelBuffer>,
    // pending data stored here until next callback
    pending_pixel_buffer: Option<PixelBuffer>,
}

pub trait TexturePayload {
    const TEXTURE_TYPE: FlutterDesktopTextureType;

    fn set_payload(texture: &mut PlatformTexture, payload: Self);
    fn registration_info(texture: Arc<Mutex<PlatformTexture>>) -> FlutterDesktopTextureInfo;
}

impl TexturePayload for PixelBuffer {
    const TEXTURE_TYPE: FlutterDesktopTextureType = kFlutterDesktopPixelBufferTexture;

    fn set_payload(texture: &mut PlatformTexture, payload: PixelBuffer) {
        texture.pending_pixel_buffer.replace(payload);
    }

    fn registration_info(texture: Arc<Mutex<PlatformTexture>>) -> FlutterDesktopTextureInfo {
        FlutterDesktopTextureInfo {
            type_: PixelBuffer::TEXTURE_TYPE,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                pixel_buffer_config: FlutterDesktopPixelBufferTextureConfig {
                    callback: Some(pixel_buffer_callback),
                    user_data: texture.as_ref() as *const _ as *mut _,
                },
            },
        }
    }
}

pub(crate) const PIXEL_BUFFER_FORMAT: PixelBufferFormat = PixelBufferFormat::RGBA;

impl PlatformTexture {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pixel_buffer: None,
            pending_pixel_buffer: None,
            flutter_pixel_buffer: None,
        }))
    }

    pub fn set_payload<T: TexturePayload>(&mut self, payload: T) {
        T::set_payload(self, payload);
    }
}

pub(crate) struct PlatformTextureRegistry {
    registrar: FlutterDesktopTextureRegistrarRef,
}

extern "C" fn pixel_buffer_callback(
    _width: size_t,
    _height: size_t,
    user_data: *mut ::std::os::raw::c_void,
) -> *const FlutterDesktopPixelBuffer {
    let mutex = user_data as *const Mutex<PlatformTexture>;
    let mutex = unsafe { &*mutex };
    let mut texture = mutex.lock().unwrap();
    if let Some(buffer) = texture.pending_pixel_buffer.take() {
        texture.pixel_buffer.replace(buffer);
    }
    if let Some(pixel_buffer) = &texture.pixel_buffer {
        texture.flutter_pixel_buffer = Some(FlutterDesktopPixelBuffer {
            buffer: pixel_buffer.data.as_ptr(),
            width: pixel_buffer.width as usize,
            height: pixel_buffer.height as usize,
        });
        let buffer = texture.flutter_pixel_buffer.as_ref().unwrap();
        buffer as *const _
    } else {
        std::ptr::null_mut()
    }
}

impl PlatformTextureRegistry {
    pub fn new(registrar: FlutterDesktopTextureRegistrarRef) -> Self {
        Self { registrar }
    }

    pub fn register_texture<T: TexturePayload>(&self, texture: Arc<Mutex<PlatformTexture>>) -> i64 {
        let info = T::registration_info(texture);

        unsafe {
            FlutterDesktopTextureRegistrarRegisterExternalTexture(self.registrar, &info as *const _)
        }
    }

    pub fn texture_frame_available(&self, texture: i64) {
        unsafe {
            FlutterDesktopTextureRegistrarMarkExternalTextureFrameAvailable(
                self.registrar,
                texture,
            );
        }
    }

    pub fn unregister_texture(&self, texture: i64) {
        unsafe {
            FlutterDesktopTextureRegistrarUnregisterExternalTexture(self.registrar, texture);
        }
    }
}
