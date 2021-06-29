use std::{ffi::CString, mem};

use glib::{
    translate::{FromGlibPtrFull, ToGlibPtr},
    Cast,
};
use gtk::prelude::CheckMenuItemExt;

// GtkCheckMenuItem has a 'feature' where it unconditionally changes check status on activation.
// We want to be more explicit about it - activation should not change check status
//
// Here the activate class method is replaced by one from GtkMenuItemClass
unsafe extern "C" fn class_init(class: glib_sys::gpointer, _class_data: glib_sys::gpointer) {
    let our_class = class as *mut gtk_sys::GtkMenuItemClass;
    let our_class = &mut *our_class;

    let name = CString::new("GtkMenuItem").unwrap();
    let menu_item_type = gobject_sys::g_type_from_name(name.as_ptr());

    let menu_item_class =
        gobject_sys::g_type_class_peek(menu_item_type) as *mut gtk_sys::GtkMenuItemClass;
    let menu_item_class = &*menu_item_class;

    our_class.activate = menu_item_class.activate;
}

unsafe extern "C" fn instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

// CheckMenuItem is not subclassable in Gtk-rs, need to do it manually
fn check_menu_item_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = CString::new("NativeShellCheckMenuItem").unwrap();
        TYPE = gobject_sys::g_type_register_static_simple(
            gtk_sys::gtk_check_menu_item_get_type(),
            name.as_ptr(),
            mem::size_of::<gtk_sys::GtkCheckMenuItemClass>() as u32,
            Some(class_init),
            mem::size_of::<gtk_sys::GtkCheckMenuItem>() as u32,
            Some(instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

fn radio_menu_item_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = CString::new("NativeShellRadioMenuItem").unwrap();
        TYPE = gobject_sys::g_type_register_static_simple(
            gtk_sys::gtk_radio_menu_item_get_type(),
            name.as_ptr(),
            mem::size_of::<gtk_sys::GtkRadioMenuItemClass>() as u32,
            Some(class_init),
            mem::size_of::<gtk_sys::GtkRadioMenuItem>() as u32,
            Some(instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

pub(super) fn create_check_menu_item() -> gtk::CheckMenuItem {
    unsafe {
        let instance = gobject_sys::g_object_new(check_menu_item_get_type(), std::ptr::null_mut());
        gobject_sys::g_object_ref_sink(instance);
        gtk::CheckMenuItem::from_glib_full(instance as *mut _)
    }
}

pub(super) fn create_radio_menu_item() -> gtk::RadioMenuItem {
    unsafe {
        let instance = gobject_sys::g_object_new(radio_menu_item_get_type(), std::ptr::null_mut());
        gobject_sys::g_object_ref_sink(instance);
        gtk::RadioMenuItem::from_glib_full(instance as *mut _)
    }
}

// Sets or clears checked status on menu item; This requires calling the original
// "activate" class method on menu item
pub(super) fn check_menu_item_set_checked(item: &gtk::CheckMenuItem, checked: bool) {
    if item.is_active() == checked {
        return;
    }
    activate_menu_item(item.upcast_ref::<gtk::MenuItem>());
}

pub(super) fn radio_menu_item_set_checked(item: &gtk::RadioMenuItem, checked: bool) {
    if item.is_active() == checked {
        return;
    }
    activate_menu_item(item.upcast_ref::<gtk::MenuItem>());
}

fn activate_menu_item(item: &gtk::MenuItem) {
    unsafe {
        let instance: *mut gtk_sys::GtkMenuItem = item.to_glib_none().0;

        let item_class = gobject_sys::g_type_class_peek(gtk_sys::gtk_check_menu_item_get_type())
            as *mut gtk_sys::GtkMenuItemClass;
        let item_class = &*item_class;

        if let Some(activate) = item_class.activate {
            activate(instance as *mut _);
        }
    }
}
