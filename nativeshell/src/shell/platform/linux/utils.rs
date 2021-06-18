use std::{cell::RefCell, collections::HashMap, ffi::CStr};

use dylib::DynamicLibrary;
use epoxy::types::GLenum;
use gdk::{Event, EventType, Window, WindowExt};
use glib::translate::{FromGlibPtrFull, ToGlibPtr, ToGlibPtrMut};

#[derive(PartialEq)]
pub(super) enum SessionType {
    X11,
    Wayland,
}

pub(super) fn get_session_type() -> SessionType {
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    match session_type.as_deref() {
        Some("wayland") => SessionType::Wayland,
        _ => SessionType::X11,
    }
}

pub(super) fn synthetize_button_up(event: &Event) -> Event {
    if event.get_event_type() != EventType::ButtonPress {
        panic!("Invalid event type");
    }
    let mut event = event.clone();
    let e: *mut gdk_sys::GdkEvent = event.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    e.type_ = gdk_sys::GDK_BUTTON_RELEASE;
    event
}

pub(super) fn synthetize_leave_event_from_motion(event: &Event) -> Event {
    if event.get_event_type() != EventType::MotionNotify {
        panic!("Invalid event type");
    }
    let mut res = Event::new(EventType::LeaveNotify);
    let e: *mut gdk_sys::GdkEvent = res.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    e.crossing.window = event.get_window().unwrap().to_glib_full();
    e.crossing.subwindow = event.get_window().unwrap().to_glib_full();
    e.crossing.send_event = 1;
    e.crossing.x = event.get_coords().unwrap().0;
    e.crossing.y = event.get_coords().unwrap().1;
    e.crossing.x_root = event.get_root_coords().unwrap().0;
    e.crossing.y_root = event.get_root_coords().unwrap().1;

    unsafe {
        gdk_sys::gdk_event_set_device(e, event.get_device().unwrap().to_glib_none().0);
    }
    res
}

pub(super) fn translate_event_to_window(event: &Event, win: &Window) -> Event {
    let mut event = event.clone();
    let e: *mut gdk_sys::GdkEvent = event.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    if event.get_event_type() == EventType::MotionNotify {
        unsafe { Window::from_glib_full(e.motion.window) };
        e.motion.window = win.to_glib_full();
        let (_, win_x, win_y) = win.get_origin();
        e.motion.x = unsafe { e.motion.x_root } - win_x as f64;
        e.motion.y = unsafe { e.motion.y_root } - win_y as f64;
    }
    if event.get_event_type() == EventType::EnterNotify
        || event.get_event_type() == EventType::LeaveNotify
    {
        unsafe { Window::from_glib_full(e.crossing.window) };
        e.crossing.window = win.to_glib_full();
        let (_, win_x, win_y) = win.get_origin();
        e.crossing.x = unsafe { e.crossing.x_root } - win_x as f64;
        e.crossing.y = unsafe { e.crossing.y_root } - win_y as f64;
    }
    event
}

struct GlStringCache {
    values: RefCell<HashMap<GLenum, Option<String>>>,
}

impl GlStringCache {
    fn gl_get_string(&self, e: GLenum, win: &gdk::Window) -> Option<String> {
        self.values
            .borrow_mut()
            .entry(e)
            .or_insert_with(|| {
                let c = win.create_gl_context().unwrap();
                c.make_current();
                let string = unsafe { epoxy::GetString(e) };
                if string.is_null() {
                    None
                } else {
                    let s = unsafe { CStr::from_ptr(string as *const _) };
                    let s: String = s.to_string_lossy().into();
                    Some(s)
                }
            })
            .clone()
    }
}

lazy_static! {
    static ref GL_STRING_CACHE: GlStringCache = {
        epoxy::load_with(|s| unsafe {
            match DynamicLibrary::open(None).unwrap().symbol(s) {
                Ok(v) => v,
                Err(_) => std::ptr::null(),
            }
        });
        GlStringCache {
            values: RefCell::new(HashMap::new()),
        }
    };
}

unsafe impl Sync for GlStringCache {}

pub(crate) fn get_gl_vendor(win: &gdk::Window) -> Option<String> {
    GL_STRING_CACHE.gl_get_string(epoxy::VENDOR, win).clone()
}
