use std::{cmp::max, ffi::CString, mem, ptr};

use glib::{
    translate::{FromGlibPtrFull, FromGlibPtrNone},
    ObjectExt,
};
use gtk::{prelude::WidgetExt, Widget};

unsafe extern "C" fn class_init(class: glib_sys::gpointer, _class_data: glib_sys::gpointer) {
    let widget_class = class as *mut gtk_sys::GtkWidgetClass;
    let widget_class = &mut *widget_class;
    widget_class.get_preferred_width = Some(get_preferred_width);
    widget_class.get_preferred_height = Some(get_preferred_height);
}

unsafe extern "C" fn instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

fn size_widget_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = CString::new("NativeShellSizeWidget").unwrap();
        TYPE = gobject_sys::g_type_register_static_simple(
            gtk_sys::gtk_bin_get_type(),
            name.as_ptr(),
            mem::size_of::<gtk_sys::GtkBinClass>() as u32,
            Some(class_init),
            mem::size_of::<gtk_sys::GtkBin>() as u32,
            Some(instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

unsafe extern "C" fn get_preferred_width(
    widget: *mut gtk_sys::GtkWidget,
    minimum: *mut i32,
    natural: *mut i32,
) {
    let widget = Widget::from_glib_none(widget);
    let width: Option<ptr::NonNull<i32>> = widget.data("nativeshell_minimum_width");
    if let Some(width) = width.map(|w| *w.as_ref()) {
        *minimum = max(width, 1);
        *natural = max(width, 1);
    } else {
        *minimum = 1;
        *natural = 1;
    }
}

unsafe extern "C" fn get_preferred_height(
    widget: *mut gtk_sys::GtkWidget,
    minimum: *mut i32,
    natural: *mut i32,
) {
    let widget = Widget::from_glib_none(widget);
    let height: Option<ptr::NonNull<i32>> = widget.data("nativeshell_minimum_height");
    if let Some(height) = height.map(|h| *h.as_ref()) {
        *minimum = max(height, 1);
        *natural = max(height, 1);
    } else {
        *minimum = 1;
        *natural = 1;
    }
}

pub(super) fn create_size_widget() -> gtk::Widget {
    unsafe {
        let instance = gobject_sys::g_object_new(size_widget_get_type(), std::ptr::null_mut());
        gobject_sys::g_object_ref_sink(instance);
        gtk::Widget::from_glib_full(instance as *mut _)
    }
}

pub(super) fn size_widget_set_min_size(widget: &gtk::Widget, width: i32, height: i32) {
    unsafe {
        widget.set_data("nativeshell_minimum_width", width);
        widget.set_data("nativeshell_minimum_height", height);
    }
    widget.queue_resize();
}
