use std::{
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use cocoa::base::id;
use core_foundation::{
    base::{CFAllocatorRef, CFType, TCFType},
    dictionary::CFDictionaryRef,
};
use io_surface::{
    kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow, kIOSurfaceHeight, kIOSurfacePixelFormat,
    kIOSurfaceWidth, IOSurface, IOSurfaceRef,
};
use libc::c_void;
use log::warn;

use objc::{
    declare::ClassDecl,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
};

use crate::shell::{PixelBuffer, PixelBufferFormat};

pub struct PlatformTexture {
    pub texture: StrongPtr,
    pub surface: Option<IOSurface>,
}

pub trait TexturePayload {
    fn into_iosurface(self) -> IOSurface;
}

impl TexturePayload for IOSurface {
    fn into_iosurface(self) -> IOSurface {
        self
    }
}

pub(crate) const PIXEL_BUFFER_FORMAT: PixelBufferFormat = PixelBufferFormat::BGRA;

impl TexturePayload for PixelBuffer {
    fn into_iosurface(self) -> IOSurface {
        let surface = init_surface(self.width, self.height);
        surface.upload(&self.data);
        surface
    }
}

impl PlatformTexture {
    pub fn new() -> Arc<Mutex<Self>> {
        let texture = unsafe {
            let texture: id = msg_send![TEXTURE_CLASS.0, alloc];
            let texture: id = msg_send![texture, init];
            StrongPtr::new(texture)
        };
        let res = Arc::new(Mutex::new(Self {
            texture: texture.clone(),
            surface: None,
        }));
        let ptr = Arc::into_raw(res.clone());
        unsafe {
            (**texture).set_ivar("imState", ptr as *mut c_void);
        }
        res
    }

    pub fn set_payload<T: TexturePayload>(&mut self, payload: T) {
        self.surface.replace(payload.into_iosurface());
    }

    fn copy_pixel_buffer(&self) -> CVPixelBufferRef {
        let mut buffer: CVPixelBufferRef = std::ptr::null_mut();
        if let Some(surface) = &self.surface {
            unsafe {
                CVPixelBufferCreateWithIOSurface(
                    std::ptr::null_mut(),
                    surface.as_CFTypeRef() as *const _,
                    std::ptr::null_mut(),
                    &mut buffer as *mut _,
                );
            }
        }
        buffer
    }
}

type CVPixelBufferRef = *mut c_void;

extern "C" {
    fn CVPixelBufferCreateWithIOSurface(
        allocator: CFAllocatorRef,
        surface: IOSurfaceRef,
        pixelBufferAttributes: CFDictionaryRef,
        pixelBufferOut: *mut CVPixelBufferRef,
    ) -> i32;
}

pub(crate) struct PlatformTextureRegistry {
    registry: StrongPtr,
}

impl PlatformTextureRegistry {
    pub fn new(registry: StrongPtr) -> Self {
        Self { registry }
    }

    pub fn register_texture(&self, texture: Arc<Mutex<PlatformTexture>>) -> i64 {
        let texture = {
            let texture = texture.lock().unwrap();
            texture.texture.clone()
        };
        unsafe {
            let res: i64 = msg_send![*self.registry, registerTexture:*texture];
            res
        }
    }

    pub fn texture_frame_available(&self, texture: i64) {
        unsafe {
            let () = msg_send![*self.registry, textureFrameAvailable: texture];
        }
    }

    pub fn unregister_texture(&self, texture: i64) {
        unsafe {
            let () = msg_send![*self.registry, unregisterTexture: texture];
        }
    }
}

struct TextureClass(*const Class);
unsafe impl Sync for TextureClass {}

lazy_static! {
    static ref TEXTURE_CLASS: TextureClass = unsafe {
        let mut decl = ClassDecl::new("IMTexture", class!(NSObject)).unwrap();
        decl.add_method(
            sel!(copyPixelBuffer),
            copy_pixel_buffer as extern "C" fn(&Object, Sel) -> CVPixelBufferRef,
        );
        decl.add_method(
            sel!(onTextureUnregistered:),
            on_texture_unregistered as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
        decl.add_ivar::<*mut c_void>("imState");
        TextureClass(decl.register())
    };
}

extern "C" fn copy_pixel_buffer(this: &Object, _: Sel) -> CVPixelBufferRef {
    let state = unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        let ptr = ptr as *const Mutex<PlatformTexture>;
        ManuallyDrop::new(Arc::from_raw(ptr))
    };
    let texture = state.lock().unwrap();
    texture.copy_pixel_buffer() as *mut c_void
}

extern "C" fn on_texture_unregistered(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        this.set_ivar("imState", std::ptr::null_mut() as *mut c_void);
        let ptr = ptr as *const Mutex<PlatformTexture>;
        Arc::from_raw(ptr);
    }
}

extern "C" fn dealloc(this: &Object, _: Sel) {
    unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        if !ptr.is_null() {
            warn!("onTextureUnregistered was not called on texture object");
            let ptr = ptr as *const Mutex<PlatformTexture>;
            Arc::from_raw(ptr);
        }
    }
}

const fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24)
        + ((array[1] as u32) << 16)
        + ((array[2] as u32) << 8)
        + ((array[3] as u32) << 0)
}

fn init_surface(width: i32, height: i32) -> IOSurface {
    use core_foundation::{dictionary::CFDictionary, number::CFNumber, string::CFString};

    let k_width: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceWidth) };
    let v_width: CFNumber = width.into();

    let k_height: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceHeight) };
    let v_height: CFNumber = height.into();

    let k_bytes_per_row: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceBytesPerRow) };
    let v_bytes_per_row: CFNumber = (width * 4).into();

    let k_pixel_format: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfacePixelFormat) };
    let v_pixel_format: CFNumber = (as_u32_be(b"BGRA") as i32).into();

    let k_bytes_per_elem: CFString =
        unsafe { TCFType::wrap_under_get_rule(kIOSurfaceBytesPerElement) };
    let v_bytes_per_elem: CFNumber = 4.into();

    let pairs: Vec<(CFString, CFType)> = vec![
        (k_width, v_width.as_CFType()),
        (k_height, v_height.as_CFType()),
        (k_bytes_per_row, v_bytes_per_row.as_CFType()),
        (k_bytes_per_elem, v_bytes_per_elem.as_CFType()),
        (k_pixel_format, v_pixel_format.as_CFType()),
    ];

    io_surface::new(&CFDictionary::from_CFType_pairs(pairs.as_slice()))
}
