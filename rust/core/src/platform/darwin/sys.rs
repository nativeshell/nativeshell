#[allow(non_camel_case_types)]
pub mod dispatch {
    use std::os::raw::c_void;

    #[repr(C)]
    pub struct dispatch_object_s {
        _private: [u8; 0],
    }
    pub type dispatch_queue_t = *mut dispatch_object_s;
    pub type dispatch_function_t = extern "C" fn(*mut c_void);

    #[cfg_attr(
        any(target_os = "macos", target_os = "ios"),
        link(name = "System", kind = "dylib")
    )]
    #[cfg_attr(
        not(any(target_os = "macos", target_os = "ios")),
        link(name = "dispatch", kind = "dylib")
    )]
    extern "C" {
        pub fn dispatch_async_f(
            queue: dispatch_queue_t,
            context: *mut c_void,
            work: dispatch_function_t,
        );
        static _dispatch_main_q: dispatch_object_s;
    }

    pub fn dispatch_get_main_queue() -> dispatch_queue_t {
        unsafe { &_dispatch_main_q as *const _ as dispatch_queue_t }
    }
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod cocoa {
    use objc::{class, msg_send, runtime, sel, sel_impl};

    pub use objc::runtime::{BOOL, NO, YES};

    pub type id = *mut runtime::Object;
    pub const nil: id = 0 as id;

    #[cfg(target_pointer_width = "64")]
    pub type CGFloat = std::os::raw::c_double;
    #[cfg(not(target_pointer_width = "64"))]
    pub type CGFloat = std::os::raw::c_float;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct NSPoint {
        pub x: CGFloat,
        pub y: CGFloat,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    #[repr(u64)] // NSUInteger
    pub enum NSEventType {
        NSApplicationDefined = 15,
    }

    impl NSPoint {
        #[inline]
        pub fn new(x: CGFloat, y: CGFloat) -> NSPoint {
            NSPoint { x, y }
        }
    }

    pub trait NSApplication: Sized {
        unsafe fn sharedApplication(_: Self) -> id {
            msg_send![class!(NSApplication), sharedApplication]
        }
        unsafe fn activateIgnoringOtherApps_(self, ignore: BOOL);
        unsafe fn run(self);
        unsafe fn stop_(self, sender: id);
    }

    impl NSApplication for id {
        unsafe fn activateIgnoringOtherApps_(self, ignore: BOOL) {
            msg_send![self, activateIgnoringOtherApps: ignore]
        }

        unsafe fn run(self) {
            msg_send![self, run]
        }

        unsafe fn stop_(self, sender: id) {
            msg_send![self, stop: sender]
        }
    }
}
