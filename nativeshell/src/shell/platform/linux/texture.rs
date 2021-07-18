use std::{
    ffi::CString,
    mem,
    sync::{Arc, Mutex},
};

use glib::translate::FromGlibPtrFull;

use super::{
    flutter::{Texture, TextureRegistrar, TextureRegistrarExt},
    flutter_sys,
};
use crate::shell::{GLTexture, PixelBuffer, PixelBufferFormat};

pub struct PlatformTexture {
    pixel_buffer: Option<PixelBuffer>,
    pending_pixel_buffer: Option<PixelBuffer>,
    gl_texture: Option<GLTexture>,
}

pub(crate) const PIXEL_BUFFER_FORMAT: PixelBufferFormat = PixelBufferFormat::RGBA;

impl PlatformTexture {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pixel_buffer: None,
            pending_pixel_buffer: None,
            gl_texture: None,
        }))
    }

    pub fn set_payload<T: TexturePayload>(&mut self, payload: T) {
        T::set_payload(self, payload);
    }
}

pub trait TexturePayload {
    fn set_payload(texture: &mut PlatformTexture, payload: Self);
    fn create_texture(texture: Arc<Mutex<PlatformTexture>>) -> Texture;
}

impl TexturePayload for PixelBuffer {
    fn set_payload(texture: &mut PlatformTexture, payload: PixelBuffer) {
        texture.pending_pixel_buffer.replace(payload);
    }

    fn create_texture(texture: Arc<Mutex<PlatformTexture>>) -> Texture {
        new_pixel_buffer_texture(move || {
            let mut texture = texture.lock().unwrap();
            if let Some(buffer) = texture.pending_pixel_buffer.take() {
                texture.pixel_buffer.replace(buffer);
            }
            if let Some(buffer) = texture.pixel_buffer.as_ref() {
                (
                    buffer.data.as_ptr(),
                    6408, // GL_RGBA,
                    buffer.width as u32,
                    buffer.height as u32,
                )
            } else {
                (std::ptr::null_mut(), 0, 0, 0)
            }
        })
    }
}

impl TexturePayload for GLTexture {
    fn set_payload(texture: &mut PlatformTexture, payload: Self) {
        texture.gl_texture.replace(payload);
    }

    fn create_texture(texture: Arc<Mutex<PlatformTexture>>) -> Texture {
        new_texture_gl(move || {
            let texture = texture.lock().unwrap();
            if let Some(texture) = texture.gl_texture.as_ref() {
                (
                    texture.target,
                    texture.name,
                    texture.width as u32,
                    texture.height as u32,
                )
            } else {
                (0, 0, 0, 0)
            }
        })
    }
}
pub(crate) struct PlatformTextureRegistry {
    registrar: TextureRegistrar,
}

impl PlatformTextureRegistry {
    pub fn new(registrar: TextureRegistrar) -> Self {
        Self { registrar }
    }

    pub fn register_texture<T: TexturePayload>(&self, texture: Arc<Mutex<PlatformTexture>>) -> i64 {
        self.registrar.register_texture(T::create_texture(texture))
    }

    pub fn texture_frame_available(&self, texture: i64) {
        self.registrar.mark_texture_frame_available(texture)
    }

    pub fn unregister_texture(&self, texture: i64) {
        self.registrar.unregister_texture(texture)
    }
}

//
//
//

fn new_pixel_buffer_texture<F>(callback: F) -> Texture
where
    F: Fn() -> (*const u8, u32, u32, u32) + 'static,
{
    unsafe {
        let instance =
            gobject_sys::g_object_new(pixel_buffer_texture_get_type(), std::ptr::null_mut());
        gobject_sys::g_object_ref_sink(instance);

        let texture = instance as *mut PixelBufferTextureImpl;
        let texture = &mut *texture;
        texture.callback = Some(Box::new(callback));

        Texture::from_glib_full(instance as *mut _)
    }
}

fn new_texture_gl<F>(callback: F) -> Texture
where
    F: Fn() -> (u32, u32, u32, u32) + 'static,
{
    unsafe {
        let instance = gobject_sys::g_object_new(texture_gl_get_type(), std::ptr::null_mut());
        gobject_sys::g_object_ref_sink(instance);

        let texture = instance as *mut TextureGLImpl;
        let texture = &mut *texture;
        texture.callback = Some(Box::new(callback));

        Texture::from_glib_full(instance as *mut _)
    }
}

//
// PixelBufferTextureImpl
//

#[repr(C)]
struct PixelBufferTextureImpl {
    parent_instance: flutter_sys::FlPixelBufferTexture,
    callback: Option<Box<dyn Fn() -> (*const u8, u32, u32, u32)>>,
}

#[repr(C)]
struct PixelBufferTextureImplClass {
    parent_class: flutter_sys::FlPixelBufferTextureClass,
}

unsafe extern "C" fn pixel_buffer_texture_impl_copy_pixels(
    texture: *mut flutter_sys::FlPixelBufferTexture,
    buffer: *mut *const u8,
    format: *mut u32,
    width: *mut u32,
    height: *mut u32,
    error: *mut *mut glib_sys::GError,
) -> glib_sys::gboolean {
    let s = texture as *mut PixelBufferTextureImpl;
    let s = &*s;
    let data = (s.callback.as_ref().unwrap())();
    *buffer = data.0;
    *format = data.1;
    *width = data.2;
    *height = data.3;
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    true.into()
}

unsafe extern "C" fn pixel_buffer_texture_dispose(instance: *mut gobject_sys::GObject) {
    let s = instance as *mut PixelBufferTextureImpl;
    let s = &mut *s;
    s.callback.take();

    let super_class = gobject_sys::g_type_class_peek(flutter_sys::fl_pixel_buffer_texture_get_type())
        as *mut gobject_sys::GObjectClass;
    let super_class = &*super_class;
    super_class.dispose.unwrap()(instance);
}

unsafe extern "C" fn pixel_buffer_texture_impl_class_init(
    class: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let texture_class = class as *mut flutter_sys::FlPixelBufferTextureClass;
    let texture_class = &mut *texture_class;
    texture_class.copy_pixels = Some(pixel_buffer_texture_impl_copy_pixels);

    let object_class = class as *mut gobject_sys::GObjectClass;
    let object_class = &mut *object_class;
    object_class.dispose = Some(pixel_buffer_texture_dispose);
}

unsafe extern "C" fn pixel_buffer_texture_impl_instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

fn pixel_buffer_texture_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = CString::new("PixelBufferTextureImpl").unwrap();
        TYPE = gobject_sys::g_type_register_static_simple(
            flutter_sys::fl_pixel_buffer_texture_get_type(),
            name.as_ptr(),
            mem::size_of::<PixelBufferTextureImplClass>() as u32,
            Some(pixel_buffer_texture_impl_class_init),
            mem::size_of::<PixelBufferTextureImpl>() as u32,
            Some(pixel_buffer_texture_impl_instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

//
// TextureGLImpl
//

#[repr(C)]
struct TextureGLImpl {
    parent_instance: flutter_sys::FlTextureGL,
    callback: Option<Box<dyn Fn() -> (u32, u32, u32, u32)>>,
}

#[repr(C)]
struct TextureGLImplClass {
    parent_class: flutter_sys::FlTextureGLClass,
}

unsafe extern "C" fn texture_gl_populate(
    texture: *mut flutter_sys::FlTextureGL,
    target: *mut u32,
    name: *mut u32,
    width: *mut u32,
    height: *mut u32,
    error: *mut *mut glib_sys::GError,
) -> glib_sys::gboolean {
    let s = texture as *mut TextureGLImpl;
    let s = &*s;
    let data = (s.callback.as_ref().unwrap())();
    *target = data.0;
    *name = data.1;
    *width = data.2;
    *height = data.3;
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    true.into()
}

unsafe extern "C" fn texture_gl_dispose(instance: *mut gobject_sys::GObject) {
    let s = instance as *mut TextureGLImpl;
    let s = &mut *s;
    s.callback.take();

    let super_class = gobject_sys::g_type_class_peek(flutter_sys::fl_texture_gl_get_type())
        as *mut gobject_sys::GObjectClass;
    let super_class = &*super_class;
    super_class.dispose.unwrap()(instance);
}

unsafe extern "C" fn texture_gl_impl_class_init(
    class: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let texture_class = class as *mut flutter_sys::FlTextureGLClass;
    let texture_class = &mut *texture_class;
    texture_class.populate = Some(texture_gl_populate);

    let object_class = class as *mut gobject_sys::GObjectClass;
    let object_class = &mut *object_class;
    object_class.dispose = Some(texture_gl_dispose);
}

unsafe extern "C" fn texture_gl_impl_instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

fn texture_gl_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = CString::new("TextureGLImpl").unwrap();
        TYPE = gobject_sys::g_type_register_static_simple(
            flutter_sys::fl_texture_gl_get_type(),
            name.as_ptr(),
            mem::size_of::<TextureGLImplClass>() as u32,
            Some(texture_gl_impl_class_init),
            mem::size_of::<TextureGLImpl>() as u32,
            Some(texture_gl_impl_instance_init),
            0,
        );
    });

    unsafe { TYPE }
}
