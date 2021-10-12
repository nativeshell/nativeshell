use std::os::raw::{c_char, c_void};

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

#[cfg(test)]
extern "C" {
    pub fn fl_dart_project_new() -> *mut GObject;
}

// Only link flutter_linux_gtk when not building for tests
#[cfg(not(test))]
#[link(name = "flutter_linux_gtk")]
extern "C" {
    pub fn fl_dart_project_new() -> *mut GObject;
}

extern "C" {
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
}
