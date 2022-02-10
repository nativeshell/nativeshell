use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem::ManuallyDrop,
    os::raw::c_int,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

use super::sys::{libc::*, ndk_sys::*};

pub struct PlatformRunLoop {
    looper: *mut ALooper,
    pipes: [c_int; 2],
    state: Rc<State>,
    state_ptr: *const State,
}

struct Timer {
    scheduled: Instant,
    callback: Box<dyn FnOnce()>,
}

struct State {
    timer_fd: c_int,
    callbacks: Arc<Mutex<Callbacks>>,
    next_handle: Cell<HandleType>,
    timers: RefCell<HashMap<HandleType, Timer>>,
}

type SenderCallback = Box<dyn FnOnce() + Send>;

struct Callbacks {
    fd: c_int,
    callbacks: Vec<SenderCallback>,
}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        let looper = unsafe {
            let looper = ALooper_forThread();
            ALooper_acquire(looper);
            looper
        };
        let mut pipes: [c_int; 2] = [0, 2];
        unsafe { pipe(pipes.as_mut_ptr()) };

        let timer_fd = unsafe { timerfd_create(CLOCK_MONOTONIC, TFD_NONBLOCK) };

        let state = Rc::new(State {
            timer_fd,
            callbacks: Arc::new(Mutex::new(Callbacks {
                fd: pipes[1],
                callbacks: Vec::new(),
            })),
            next_handle: Cell::new(INVALID_HANDLE + 1),
            timers: RefCell::new(HashMap::new()),
        });

        let state_ptr = Weak::into_raw(Rc::downgrade(&state));

        unsafe {
            ALooper_addFd(
                looper,
                pipes[0],
                0,
                ALOOPER_EVENT_INPUT as c_int,
                Some(Self::looper_cb),
                state_ptr as *mut _,
            );
            ALooper_addFd(
                looper,
                timer_fd,
                0,
                ALOOPER_EVENT_INPUT as c_int,
                Some(Self::looper_timer_cb),
                state_ptr as *mut _,
            );
        }

        Self {
            looper,
            pipes,
            state,
            state_ptr,
        }
    }

    unsafe extern "C" fn looper_cb(
        fd: ::std::os::raw::c_int,
        events: ::std::os::raw::c_int,
        data: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int {
        let mut buf = [0u8; 8];
        read(fd, buf.as_mut_ptr() as *mut _, buf.len());

        let state = data as *const State;
        let state = ManuallyDrop::new(Weak::from_raw(state));
        if let Some(state) = state.upgrade() {
            state.process_callbacks();
        }
        1
    }

    unsafe extern "C" fn looper_timer_cb(
        fd: ::std::os::raw::c_int,
        events: ::std::os::raw::c_int,
        data: *mut ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int {
        let mut buf = [0u8; 8];
        read(fd, buf.as_mut_ptr() as *mut _, buf.len());

        let state = data as *const State;
        let state = ManuallyDrop::new(Weak::from_raw(state));
        if let Some(state) = state.upgrade() {
            state.process_timers();
        }
        1
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.state.unschedule(handle);
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        self.state.schedule(in_time, callback)
    }

    pub fn new_sender(&self) -> PlatformRunLoopSender {
        PlatformRunLoopSender {
            callbacks: self.state.callbacks.clone(),
        }
    }
}

impl State {
    fn process_callbacks(&self) {
        let callbacks: Vec<SenderCallback> = {
            let mut callbacks = self.callbacks.lock().unwrap();
            callbacks.callbacks.drain(0..).collect()
        };
        for c in callbacks {
            c()
        }
    }

    fn process_timers(&self) {
        loop {
            let now = Instant::now();
            let pending: Vec<HandleType> = self
                .timers
                .borrow()
                .iter()
                .filter(|v| v.1.scheduled <= now)
                .map(|v| *v.0)
                .collect();
            if pending.is_empty() {
                break;
            }
            for handle in pending {
                let timer = self.timers.borrow_mut().remove(&handle);
                if let Some(timer) = timer {
                    (timer.callback)();
                }
            }
        }
        self.wake_up_at(self.next_timer());
    }

    fn wake_up_at(&self, time: Instant) {
        let wait_time = time.saturating_duration_since(Instant::now());
        let spec = itimerspec {
            it_interval: timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: timespec {
                tv_sec: wait_time.as_secs().try_into().unwrap(),
                tv_nsec: wait_time.subsec_nanos().try_into().unwrap(),
            },
        };
        unsafe {
            timerfd_settime(self.timer_fd, 0, &spec as *const _, std::ptr::null_mut());
        }
    }

    fn next_timer(&self) -> Instant {
        let min = self.timers.borrow().values().map(|x| x.scheduled).min();
        min.unwrap_or_else(|| Instant::now() + Duration::from_secs(60 * 60))
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        r
    }

    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        let handle = self.next_handle();

        self.timers.borrow_mut().insert(
            handle,
            Timer {
                scheduled: Instant::now() + in_time,
                callback: Box::new(callback),
            },
        );

        self.wake_up_at(self.next_timer());

        handle
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.timers.borrow_mut().remove(&handle);
        self.wake_up_at(self.next_timer());
    }
}

impl Drop for PlatformRunLoop {
    fn drop(&mut self) {
        unsafe {
            ALooper_removeFd(self.looper, self.pipes[0]);
            ALooper_removeFd(self.looper, self.state.timer_fd);
            ALooper_release(self.looper);
            Weak::from_raw(self.state_ptr);
            close(self.pipes[0]);
            close(self.pipes[1]);
        }
    }
}

#[derive(Clone)]
pub struct PlatformRunLoopSender {
    callbacks: Arc<Mutex<Callbacks>>,
}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.callbacks.push(Box::new(callback));
        let buf = [0u8; 8];
        unsafe {
            write(callbacks.fd, buf.as_ptr() as *const _, buf.len());
        }
    }
}
