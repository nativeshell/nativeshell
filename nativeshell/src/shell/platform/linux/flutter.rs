#![allow(clippy::from_over_into)]

use std::{
    mem::ManuallyDrop,
    os::raw::{c_char, c_void},
};

use super::flutter_sys;
use glib::{translate::*, Bytes, Object};
use gtk::{Container, Widget};

use glib::{
    object::{Cast, IsA},
    GString,
};

glib::wrapper! {
    pub struct DartProject(Object<flutter_sys::FlDartProject,
        flutter_sys::FlDartProjectClass>);

    match fn {
        type_ => || gobject_sys::g_object_get_type(),
    }
}

impl DartProject {
    pub fn new() -> DartProject {
        unsafe { Object::from_glib_none(flutter_sys::fl_dart_project_new()).unsafe_cast() }
    }
}

glib::wrapper! {
    pub struct View(Object<flutter_sys::FlView,
        flutter_sys::FlViewClass>) @extends Container, Widget;

    match fn {
        type_ => || gtk_sys::gtk_container_get_type(),
    }
}

impl View {
    pub fn new<P: IsA<DartProject>>(project: &P) -> View {
        unsafe {
            Widget::from_glib_none(flutter_sys::fl_view_new(project.as_ref().to_glib_none().0))
                .unsafe_cast()
        }
    }
}

pub trait ViewExt: 'static {
    fn get_engine(&self) -> Engine;
    fn get_registrar_for_plugin(&self, plugin: &str) -> *mut c_void;
}

impl<O: IsA<View>> ViewExt for O {
    fn get_engine(&self) -> Engine {
        unsafe {
            Object::from_glib_none(flutter_sys::fl_view_get_engine(
                self.as_ref().to_glib_none().0,
            ))
            .unsafe_cast()
        }
    }

    fn get_registrar_for_plugin(&self, plugin: &str) -> *mut c_void {
        unsafe {
            flutter_sys::fl_plugin_registry_get_registrar_for_plugin(
                self.as_ref().to_glib_none().0,
                plugin.to_glib_none().0,
            )
        }
    }
}

glib::wrapper! {
    pub struct Engine(Object<flutter_sys::FlEngine,
        flutter_sys::FlEngineClass>);

    match fn {
        type_ => || gobject_sys::g_object_get_type(),
    }
}

pub trait EngineExt: 'static {
    fn get_binary_messenger(&self) -> BinaryMessenger;
}

impl<O: IsA<Engine>> EngineExt for O {
    fn get_binary_messenger(&self) -> BinaryMessenger {
        unsafe {
            Object::from_glib_none(flutter_sys::fl_engine_get_binary_messenger(
                self.as_ref().to_glib_none().0,
            ))
            .unsafe_cast()
        }
    }
}

glib::wrapper! {
    pub struct BinaryMessenger(Object<flutter_sys::FlBinaryMessenger,
        flutter_sys::FlBinaryMessengerClass>);

    match fn {
        type_ => || gobject_sys::g_object_get_type(),
    }
}

glib::wrapper! {
    pub struct BinaryMessengerResponseHandle(Object<flutter_sys::FlBinaryMessengerResponseHandle,
        flutter_sys::FlBinaryMessengerResponseHandleClass>);

    match fn {
        type_ => || gobject_sys::g_object_get_type(),
    }
}

pub trait BinaryMessengerExt: 'static {
    fn set_message_handler_on_channel<
        F: Fn(Bytes, &str, BinaryMessenger, BinaryMessengerResponseHandle) + 'static,
    >(
        &self,
        channel: &str,
        callback: F,
    );

    fn remove_message_handler_on_channel(&self, channel: &str);

    fn send_response<ResponseHandle: IsA<BinaryMessengerResponseHandle>>(
        &self,
        response_handle: ResponseHandle,
        response: Bytes,
    );

    fn send_message<F: FnOnce(Bytes) + 'static>(&self, channel: &str, message: Bytes, callback: F);

    fn post_message(&self, channel: &str, message: Bytes);
}

unsafe extern "C" fn message_handler(
    messenger: *mut flutter_sys::FlBinaryMessenger,
    channel: *const c_char,
    bytes: *mut glib_sys::GBytes,
    response_handle: *mut flutter_sys::FlBinaryMessengerResponseHandle,
    user_data: glib_sys::gpointer,
) {
    let b: Box<Box<dyn Fn(Bytes, &str, BinaryMessenger, BinaryMessengerResponseHandle)>> =
        Box::from_raw(user_data as *mut _);
    let b = ManuallyDrop::new(b);
    let messenger: BinaryMessenger =
        Object::from_glib_none(messenger as *mut gobject_sys::GObject).unsafe_cast();
    let response_handle: BinaryMessengerResponseHandle =
        Object::from_glib_none(response_handle as *mut gobject_sys::GObject).unsafe_cast();
    let channel: Option<GString> = from_glib_none(channel);
    let bytes: Bytes = from_glib_none(bytes);
    b(bytes, &channel.unwrap(), messenger, response_handle);
}

extern "C" fn dispose_callback(callback: glib_sys::gpointer) {
    let _: Box<Box<dyn Fn(Bytes, BinaryMessenger, BinaryMessengerResponseHandle)>> =
        unsafe { Box::from_raw(callback as *mut _) };
}

unsafe extern "C" fn message_callback(
    source_object: *mut gobject_sys::GObject,
    result: *mut gio_sys::GAsyncResult,
    user_data: glib_sys::gpointer,
) {
    let messenger: BinaryMessenger = Object::from_glib_none(source_object).unsafe_cast();
    let data: Bytes = from_glib_full(flutter_sys::fl_binary_messenger_send_on_channel_finish(
        messenger.to_glib_none().0,
        result,
        std::ptr::null_mut(),
    ));
    let b: Box<Box<dyn FnOnce(Bytes)>> = Box::from_raw(user_data as *mut _);
    b(data);
}

impl<O: IsA<BinaryMessenger>> BinaryMessengerExt for O {
    fn set_message_handler_on_channel<
        F: Fn(Bytes, &str, BinaryMessenger, BinaryMessengerResponseHandle) + 'static,
    >(
        &self,
        channel: &str,
        callback: F,
    ) {
        let b: Box<dyn Fn(Bytes, &str, BinaryMessenger, BinaryMessengerResponseHandle)> =
            Box::new(callback);
        let b = Box::new(b);
        unsafe {
            flutter_sys::fl_binary_messenger_set_message_handler_on_channel(
                self.as_ref().to_glib_none().0,
                channel.to_glib_none().0,
                Some(message_handler),
                Box::into_raw(b) as glib_sys::gpointer,
                Some(dispose_callback),
            );
        }
    }

    fn remove_message_handler_on_channel(&self, channel: &str) {
        unsafe {
            flutter_sys::fl_binary_messenger_set_message_handler_on_channel(
                self.as_ref().to_glib_none().0,
                channel.to_glib_none().0,
                None,
                std::ptr::null_mut(),
                None,
            );
        }
    }

    fn send_response<ResponseHandle: IsA<BinaryMessengerResponseHandle>>(
        &self,
        response_handle: ResponseHandle,
        response: Bytes,
    ) {
        unsafe {
            flutter_sys::fl_binary_messenger_send_response(
                self.as_ref().to_glib_none().0,
                response_handle.as_ref().to_glib_none().0,
                response.to_glib_none().0,
                std::ptr::null_mut(),
            );
        }
    }

    fn send_message<F: FnOnce(Bytes) + 'static>(&self, channel: &str, message: Bytes, callback: F) {
        let b: Box<dyn FnOnce(Bytes)> = Box::new(callback);
        let b = Box::new(b);
        unsafe {
            flutter_sys::fl_binary_messenger_send_on_channel(
                self.as_ref().to_glib_none().0,
                channel.to_glib_none().0,
                message.to_glib_none().0,
                std::ptr::null_mut(),
                Some(message_callback),
                Box::into_raw(b) as glib_sys::gpointer,
            );
        }
    }

    fn post_message(&self, channel: &str, message: Bytes) {
        unsafe {
            flutter_sys::fl_binary_messenger_send_on_channel(
                self.as_ref().to_glib_none().0,
                channel.to_glib_none().0,
                message.to_glib_none().0,
                std::ptr::null_mut(),
                None,
                std::ptr::null_mut(),
            );
        }
    }
}
