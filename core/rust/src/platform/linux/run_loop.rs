use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_uint,
    rc::Rc,
    time::Duration,
};

use super::sys::glib::*;

type SourceId = c_uint;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    next_handle: Cell<HandleType>,
    timers: Rc<RefCell<HashMap<HandleType, SourceId>>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Continue(pub bool);
unsafe extern "C" fn trampoline<F: FnMut() -> gboolean + 'static>(func: gpointer) -> gboolean {
    let func: &RefCell<F> = &*(func as *const RefCell<F>);
    (&mut *func.borrow_mut())()
}

fn into_raw<F: FnMut() -> gboolean + 'static>(func: F) -> gpointer {
    let func: Box<RefCell<F>> = Box::new(RefCell::new(func));
    Box::into_raw(func) as gpointer
}

unsafe extern "C" fn destroy_closure<F: FnMut() -> gboolean + 'static>(ptr: gpointer) {
    Box::<RefCell<F>>::from_raw(ptr as *mut _);
}

pub fn timeout_add_local<F>(interval: Duration, func: F) -> SourceId
where
    F: FnMut() -> gboolean + 'static,
{
    unsafe {
        g_timeout_add_full(
            G_PRIORITY_DEFAULT,
            interval.as_millis() as _,
            Some(trampoline::<F>),
            into_raw(func),
            Some(destroy_closure::<F>),
        )
    }
}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        Self {
            next_handle: Cell::new(INVALID_HANDLE + 1),
            timers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn unschedule(&self, handle: HandleType) {
        let source = self.timers.borrow_mut().remove(&handle);
        if let Some(source) = source {
            unsafe { g_source_remove(source) };
        }
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        r
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        let callback = Rc::new(RefCell::new(Some(callback)));
        let handle = self.next_handle();

        let timers = self.timers.clone();
        let source_id = timeout_add_local(in_time, move || {
            timers.borrow_mut().remove(&handle);
            let f = callback
                .borrow_mut()
                .take()
                .expect("Timer callback was called multiple times");
            f();
            G_SOURCE_REMOVE
        });
        self.timers.borrow_mut().insert(handle, source_id);
        handle
    }

    pub fn run(&self) {
        unsafe { gtk_main() };
    }

    pub fn stop(&self) {
        unsafe { gtk_main_quit() };
    }

    pub fn new_sender(&self) -> PlatformRunLoopSender {
        PlatformRunLoopSender {}
    }
}

#[derive(Clone)]
pub struct PlatformRunLoopSender {}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        unsafe extern "C" fn trampoline<F: FnOnce() + 'static>(func: gpointer) -> gboolean {
            let func: &mut Option<F> = &mut *(func as *mut Option<F>);
            let func = func
                .take()
                .expect("MainContext::invoke() closure called multiple times");
            func();
            G_SOURCE_REMOVE
        }
        unsafe extern "C" fn destroy_closure<F: FnOnce() + 'static>(ptr: gpointer) {
            Box::<Option<F>>::from_raw(ptr as *mut _);
        }
        let callback = Box::into_raw(Box::new(Some(callback)));
        unsafe {
            g_main_context_invoke_full(
                g_main_context_default(),
                0,
                Some(trampoline::<F>),
                callback as gpointer,
                Some(destroy_closure::<F>),
            )
        }
    }
}
