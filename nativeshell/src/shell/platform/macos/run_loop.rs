use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use block::ConcreteBlock;
use cocoa::{
    appkit::{
        NSApplication, NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        NSEventType::NSApplicationDefined,
    },
    base::{id, nil, YES},
    foundation::{NSPoint, NSRunLoop},
};

use dispatch::ffi::{
    dispatch_after_f, dispatch_async_f, dispatch_get_main_queue, dispatch_time, DISPATCH_TIME_NOW,
};

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

type Callback = Box<dyn FnOnce()>;

pub struct PlatformRunLoop {
    next_handle: Cell<HandleType>,
    callbacks: Rc<RefCell<HashMap<usize, Callback>>>,
}

struct CallbackData {
    handle: HandleType,
    callbacks: Rc<RefCell<HashMap<usize, Callback>>>,
}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        Self {
            next_handle: Cell::new(INVALID_HANDLE + 1),
            callbacks: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        r
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.callbacks.borrow_mut().remove(&handle);
    }

    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        let handle = self.next_handle();
        self.callbacks
            .borrow_mut()
            .insert(handle, Box::new(callback));

        let data = Box::new(CallbackData {
            handle,
            callbacks: self.callbacks.clone(),
        });

        let delta = in_time.as_nanos() as i64;
        unsafe {
            if delta > 0 {
                dispatch_after_f(
                    dispatch_time(DISPATCH_TIME_NOW, delta),
                    dispatch_get_main_queue(),
                    Box::into_raw(data) as *mut _,
                    Self::on_callback,
                );
            } else {
                // as a special case, with in_time == 0, schedule the callback
                // to be run on directly on NSRunLoop instead of dispatch queue; This
                // is necessary for tasks that run inner run loop (i.e. popup menu,
                // modal dialogs, etc) to not block the dispatch queue
                let data = Box::into_raw(data) as *mut _;
                let cb = move || {
                    Self::on_callback(data);
                };
                let runloop: id = NSRunLoop::currentRunLoop();
                let block = ConcreteBlock::new(cb).copy();
                let () = msg_send![runloop, performBlock:&*block];
            }
        }

        handle
    }

    extern "C" fn on_callback(user_data: *mut ::std::os::raw::c_void) {
        let data: Box<CallbackData> = unsafe { Box::from_raw(user_data as *mut _) };
        let entry = data.callbacks.borrow_mut().remove(&data.handle);
        if let Some(entry) = entry {
            entry();
        }
    }

    pub fn run(&self) {
        unsafe {
            let app = NSApplication::sharedApplication(nil);
            NSApplication::setActivationPolicy_(app, NSApplicationActivationPolicyRegular);
            NSApplication::activateIgnoringOtherApps_(app, YES);
            NSApplication::run(app);
        }
    }

    pub fn stop(&self) {
        unsafe {
            let app = NSApplication::sharedApplication(nil);
            app.stop_(nil);

            let dummy_event: id = msg_send![class!(NSEvent),
                otherEventWithType: NSApplicationDefined
                location: NSPoint::new(0.0, 0.0)
                modifierFlags: 0
                timestamp: 0
                    windowNumber: 0
                context: nil
                subtype: 0
                data1: 0
                data2: 0
            ];

            // // To stop event loop immediately, we need to post event.
            let () = msg_send![app, postEvent: dummy_event atStart: YES];
        }
    }

    pub fn new_sender(&self) -> PlatformRunLoopSender {
        PlatformRunLoopSender {}
    }
}

pub struct PlatformRunLoopSender {}

struct SenderCallbackData {
    callback: Callback,
}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let data = Box::new(SenderCallbackData {
            callback: Box::new(callback),
        });

        unsafe {
            dispatch_async_f(
                dispatch_get_main_queue(),
                Box::into_raw(data) as *mut _,
                Self::on_callback,
            );
        }
    }

    extern "C" fn on_callback(user_data: *mut ::std::os::raw::c_void) {
        let data: Box<SenderCallbackData> = unsafe { Box::from_raw(user_data as *mut _) };
        (data.callback)();
    }
}
