use gdk::{Event, EventType, Window};
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
    if event.event_type() != EventType::ButtonPress {
        panic!("Invalid event type");
    }
    let mut event = event.clone();
    let e: *mut gdk_sys::GdkEvent = event.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    e.type_ = gdk_sys::GDK_BUTTON_RELEASE;
    event
}

pub(super) fn synthetize_leave_event_from_motion(event: &Event) -> Event {
    if event.event_type() != EventType::MotionNotify {
        panic!("Invalid event type");
    }
    let mut res = Event::new(EventType::LeaveNotify);
    let e: *mut gdk_sys::GdkEvent = res.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    e.crossing.window = event.window().unwrap().to_glib_full();
    e.crossing.subwindow = event.window().unwrap().to_glib_full();
    e.crossing.send_event = 1;
    e.crossing.x = event.coords().unwrap().0;
    e.crossing.y = event.coords().unwrap().1;
    e.crossing.x_root = event.root_coords().unwrap().0;
    e.crossing.y_root = event.root_coords().unwrap().1;

    res.set_device(Some(&event.device().unwrap()));
    res
}

pub(super) fn translate_event_to_window(event: &Event, win: &Window) -> Event {
    let mut event = event.clone();
    let e: *mut gdk_sys::GdkEvent = event.to_glib_none_mut().0;
    let e = unsafe { &mut *e };
    if event.event_type() == EventType::MotionNotify {
        unsafe { Window::from_glib_full(e.motion.window) };
        e.motion.window = win.to_glib_full();
        let (_, win_x, win_y) = win.origin();
        e.motion.x = unsafe { e.motion.x_root } - win_x as f64;
        e.motion.y = unsafe { e.motion.y_root } - win_y as f64;
    }
    if event.event_type() == EventType::EnterNotify || event.event_type() == EventType::LeaveNotify
    {
        unsafe { Window::from_glib_full(e.crossing.window) };
        e.crossing.window = win.to_glib_full();
        let (_, win_x, win_y) = win.origin();
        e.crossing.x = unsafe { e.crossing.x_root } - win_x as f64;
        e.crossing.y = unsafe { e.crossing.y_root } - win_y as f64;
    }
    event
}
