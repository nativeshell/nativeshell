use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use glib::{source_remove, timeout_add_local, Continue, MainContext, SourceId};

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    next_handle: Cell<HandleType>,
    timers: Rc<RefCell<HashMap<HandleType, SourceId>>>,
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
            source_remove(source);
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
            Continue(false)
        });
        self.timers.borrow_mut().insert(handle, source_id);
        handle
    }

    pub fn run(&self) {
        gtk::main();
    }

    pub fn stop(&self) {
        gtk::main_quit();
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
        let context = MainContext::default();
        context.invoke(callback);
    }
}
