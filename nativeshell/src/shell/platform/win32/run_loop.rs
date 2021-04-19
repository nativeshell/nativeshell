use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use super::all_bindings::*;

use super::window_adapter::WindowAdapter;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    state: Box<State>,
}

struct Timer {
    scheduled: Instant,
    callback: Box<dyn FnOnce() -> ()>,
}

type SenderCallback = Box<dyn FnOnce() -> () + Send>;

struct State {
    next_handle: Cell<HandleType>,
    hwnd: Cell<HWND>,
    timers: RefCell<HashMap<HandleType, Timer>>,

    // Callbacks sent from other threads
    sender_callbacks: Arc<Mutex<Vec<SenderCallback>>>,
}

impl State {
    fn new() -> Self {
        Self {
            next_handle: Cell::new(INVALID_HANDLE + 1),
            hwnd: Cell::new(HWND(0)),
            timers: RefCell::new(HashMap::new()),
            sender_callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn initialize(&self) {
        self.hwnd.set(self.create_window_custom(
            "nativeshell RunLoop Window",
            WINDOW_STYLE(0),
            WINDOW_EX_STYLE(0),
        ));
    }

    fn wake_up_at(&self, time: Instant) {
        let wait_time = time.saturating_duration_since(Instant::now());
        unsafe {
            SetTimer(self.hwnd.get(), 0, wait_time.as_millis() as u32, None);
        }
    }

    fn on_timer(&self) {
        let next_time = self.process_timers();
        self.wake_up_at(next_time);
    }

    fn next_timer(&self) -> Instant {
        let min = self.timers.borrow().values().map(|x| x.scheduled).min();
        min.unwrap_or_else(|| Instant::now() + Duration::from_secs(60 * 60))
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        return r;
    }

    pub fn schedule<F>(&self, callback: F, in_time: Duration) -> HandleType
    where
        F: FnOnce() -> () + 'static,
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
    }

    fn process_timers(&self) -> Instant {
        loop {
            let now = Instant::now();
            let pending: Vec<HandleType> = self
                .timers
                .borrow()
                .iter()
                .filter(|v| v.1.scheduled <= now)
                .map(|v| v.0.clone())
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

        self.next_timer()
    }

    fn process_callbacks(&self) {
        let callbacks: Vec<SenderCallback> = {
            let mut callbacks = self.sender_callbacks.lock().unwrap();
            callbacks.drain(0..).collect()
        };
        for c in callbacks {
            c()
        }
    }

    fn new_sender(&self) -> PlatformRunLoopSender {
        PlatformRunLoopSender {
            hwnd: self.hwnd.get(),
            callbacks: self.sender_callbacks.clone(),
        }
    }

    fn run(&self) {
        unsafe {
            let mut message = MSG::default();
            while GetMessageW(&mut message, HWND(0), 0, 0) == TRUE {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn stop(&self) {
        unsafe { PostMessageW(HWND(0), WM_QUIT as u32, WPARAM(0), LPARAM(0)) };
    }
}

impl WindowAdapter for State {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        match msg {
            WM_TIMER => {
                self.on_timer();
            }
            WM_USER => {
                self.process_callbacks();
            }
            _ => {}
        }
        self.default_wnd_proc(h_wnd, msg, w_param, l_param)
    }
}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        let res = Self {
            state: Box::new(State::new()),
        };
        res.state.initialize();
        res
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.state.unschedule(handle);
    }

    #[must_use]
    pub fn schedule<F>(&self, callback: F, in_time: Duration) -> HandleType
    where
        F: FnOnce() -> () + 'static,
    {
        self.state.schedule(callback, in_time)
    }

    pub fn run(&self) {
        self.state.run();
    }

    pub fn stop(&self) {
        self.state.stop();
    }

    pub fn new_sender(&self) -> PlatformRunLoopSender {
        self.state.new_sender()
    }
}

pub struct PlatformRunLoopSender {
    hwnd: HWND,
    callbacks: Arc<Mutex<Vec<SenderCallback>>>,
}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() -> () + 'static + Send,
    {
        {
            let mut callbacks = self.callbacks.lock().unwrap();
            callbacks.push(Box::new(callback));
        }
        unsafe {
            PostMessageW(self.hwnd, WM_USER as u32, WPARAM(0), LPARAM(0));
        }
    }
}
