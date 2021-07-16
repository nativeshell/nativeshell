use std::{
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use cocoa::base::id;
use core_foundation::{
    base::{CFAllocatorRef, TCFType},
    dictionary::CFDictionaryRef,
};
use io_surface::{IOSurface, IOSurfaceRef};
use libc::c_void;
use log::warn;

use objc::{
    declare::ClassDecl,
    rc::StrongPtr,
    runtime::{Class, Object, Sel},
};

pub struct PlatformTexture {
    pub texture: StrongPtr,
    pub surface: Option<IOSurface>,
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

    pub fn set_surface(&mut self, surface: IOSurface) {
        self.surface.replace(surface);
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
