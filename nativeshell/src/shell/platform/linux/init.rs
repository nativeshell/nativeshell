use std::{
    cell::Cell,
    ffi::CString,
    rc::{Rc, Weak},
};

use gdk::{Event, Window, WindowExt};
use glib::{translate::FromGlibPtrNone, ObjectExt};

use crate::shell::{platform::window::PlatformWindow, Context, IRect};

use super::error::{PlatformError, PlatformResult};
struct Global {
    pub prev_check_resize: Cell<Option<unsafe extern "C" fn(*mut gtk_sys::GtkContainer)>>,
    pub prev_move_resize: Cell<
        Option<
            unsafe extern "C" fn(*mut gdk_sys::GdkWindow, glib_sys::gboolean, i32, i32, i32, i32),
        >,
    >,
}

unsafe impl Sync for Global {}

lazy_static! {
    static ref GLOBAL: Global = Global {
        prev_check_resize: Cell::new(None),
        prev_move_resize: Cell::new(None),
    };
}

pub fn init_platform(_context: Rc<Context>) -> PlatformResult<()> {
    gtk::init().map_err(|e| PlatformError::GLibError {
        message: e.message.into(),
    })?;

    Event::set_handler(Some(|e: &mut Event| {
        let win = e.get_window().map(|w| w.get_toplevel());

        if let Some(win) = win {
            let platform_window: Option<&Weak<PlatformWindow>> =
                unsafe { win.get_data("nativeshell_platform_window") };
            if let Some(platform_window) = platform_window.and_then(|w| w.upgrade()) {
                platform_window.on_event(e);
            }
        }

        gtk::main_do_event(e);
    }));

    // Gtk on X11 needs tight window hints for window to be non resizable. However in order
    // to get smooth animated resizing, these hints need to be set just before gdk_window_resize
    // is called. To do that we intercept move_resize on GdkWindowImplX11 and call it there.
    // Sadly this relies on knowing the layout of _GdkWindowImplClass, but the relevant part has
    // not changed in last 10 years so fingers crossed...
    unsafe {
        let name = CString::new("GdkWindowImplX11").unwrap();
        let t = gobject_sys::g_type_from_name(name.as_ptr());
        if t != 0 {
            let c = gobject_sys::g_type_class_peek(t) as *mut _GdkWindowImplClass;
            let c = &mut *c;
            GLOBAL.prev_move_resize.replace(c.move_resize);
            c.move_resize = Some(move_resize);
        }
    }

    Ok(())
}

unsafe extern "C" fn move_resize(
    win: *mut gdk_sys::GdkWindow,
    with_move: glib_sys::gboolean,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    {
        let win = Window::from_glib_none(win);
        let platform_window: Option<&Weak<PlatformWindow>> =
            win.get_data("nativeshell_platform_window");
        if let Some(platform_window) = platform_window.and_then(|w| w.upgrade()) {
            platform_window.on_move_resize(IRect::xywh(x, y, w, h));
        }
    }

    GLOBAL.prev_move_resize.get().unwrap()(win, with_move, x, y, w, h);
}

struct _GdkWindowImplClass {
    _parent_class: gobject_sys::GObjectClass,
    _ref_cairo_surface: Option<unsafe extern "C" fn()>,
    _create_similar_image_surface: Option<unsafe extern "C" fn()>,
    _show: Option<unsafe extern "C" fn()>,
    _hide: Option<unsafe extern "C" fn()>,
    _withdraw: Option<unsafe extern "C" fn()>,
    _raise: Option<unsafe extern "C" fn()>,
    _lower: Option<unsafe extern "C" fn()>,
    _restack_under: Option<unsafe extern "C" fn()>,
    _restack_toplevel: Option<unsafe extern "C" fn()>,
    move_resize: Option<
        unsafe extern "C" fn(*mut gdk_sys::GdkWindow, glib_sys::gboolean, i32, i32, i32, i32),
    >,
}
