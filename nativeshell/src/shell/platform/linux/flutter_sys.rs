extern crate gobject_sys;
extern crate gtk_sys;

use std::os::raw::{c_char, c_void};

use glib_sys::GType;
use gobject_sys::{GObject, GObjectClass};
use gtk_sys::{GtkContainer, GtkContainerClass, GtkWidget};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlDartProject {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlDartProjectClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlEngine {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlEngineClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlView {
    parent_instance: GtkContainer,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlViewClass {
    parent_class: GtkContainerClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlBinaryMessenger {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlBinaryMessengerClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlBinaryMessengerResponseHandle {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlBinaryMessengerResponseHandleClass {
    parent_class: GObjectClass,
}

pub type FlBinaryMessengerMessageHandler = Option<
    unsafe extern "C" fn(
        messenger: *mut FlBinaryMessenger,
        channel: *const c_char,
        bytes: *mut glib_sys::GBytes,
        response_handle: *mut FlBinaryMessengerResponseHandle,
        user_data: glib_sys::gpointer,
    ),
>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureRegistrar {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureRegistrarClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTexture {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlPixelBufferTexture {
    parent_instance: FlTexture,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlPixelBufferTextureClass {
    parent_class: FlTextureClass,
    pub copy_pixels: Option<
        unsafe extern "C" fn(
            texture: *mut FlPixelBufferTexture,
            buffer: *mut *const u8,
            format: *mut u32,
            width: *mut u32,
            height: *mut u32,
            error: *mut *mut glib_sys::GError,
        ) -> glib_sys::gboolean,
    >,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureGL {
    parent_instance: FlTexture,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureGLClass {
    parent_class: FlTextureClass,
    pub populate: Option<
        unsafe extern "C" fn(
            texture: *mut FlTextureGL,
            target: *mut u32,
            name: *mut u32,
            width: *mut u32,
            height: *mut u32,
            error: *mut *mut glib_sys::GError,
        ) -> glib_sys::gboolean,
    >,
}

extern "C" {
    pub fn fl_dart_project_new() -> *mut GObject;

    pub fn fl_view_new(project: *mut FlDartProject) -> *mut GtkWidget;
    pub fn fl_view_get_engine(view: *mut FlView) -> *mut GObject;

    pub fn fl_engine_get_binary_messenger(engine: *mut FlEngine) -> *mut GObject;

    pub fn fl_binary_messenger_set_message_handler_on_channel(
        messenger: *mut FlBinaryMessenger,
        channel: *const c_char,
        handler: FlBinaryMessengerMessageHandler,
        user_data: glib_sys::gpointer,
        destroy_notify: glib_sys::GDestroyNotify,
    );

    pub fn fl_binary_messenger_send_response(
        messenger: *mut FlBinaryMessenger,
        response_handle: *mut FlBinaryMessengerResponseHandle,
        response: *mut glib_sys::GBytes,
        error: *mut *mut glib_sys::GError,
    ) -> glib_sys::gboolean;

    pub fn fl_binary_messenger_send_on_channel(
        messenger: *mut FlBinaryMessenger,
        channel: *const c_char,
        message: *mut glib_sys::GBytes,
        cancellable: *mut gio_sys::GCancellable,
        callback: gio_sys::GAsyncReadyCallback,
        user_data: glib_sys::gpointer,
    );

    pub fn fl_binary_messenger_send_on_channel_finish(
        messenger: *mut FlBinaryMessenger,
        result: *mut gio_sys::GAsyncResult,
        error: *mut *mut glib_sys::GError,
    ) -> *mut glib_sys::GBytes;

    pub fn fl_plugin_registry_get_registrar_for_plugin(
        registry: *mut FlView,
        name: *const c_char,
    ) -> *mut c_void;

    //

    pub fn fl_texture_get_type() -> GType;
    pub fn fl_pixel_buffer_texture_get_type() -> GType;
    pub fn fl_texture_gl_get_type() -> GType;

    pub fn fl_engine_get_texture_registrar(engine: *mut FlEngine) -> *mut GObject;

    pub fn fl_texture_registrar_register_texture(
        registrar: *mut FlTextureRegistrar,
        texture: *mut FlTexture,
    ) -> i64;

    pub fn fl_texture_registrar_mark_texture_frame_available(
        registrar: *mut FlTextureRegistrar,
        texture_id: i64,
    );

    pub fn fl_texture_registrar_unregister_texture(
        registrar: *mut FlTextureRegistrar,
        texture_id: i64,
    );
}
