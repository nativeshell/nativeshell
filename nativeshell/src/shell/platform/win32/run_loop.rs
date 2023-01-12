use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Weak,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostMessageW, SetTimer, TranslateMessage, MSG,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_HOTKEY, WM_QUIT, WM_TIMER, WM_USER,
    },
};

use super::window_adapter::WindowAdapter;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    state: Box<State>,
}

struct Timer {
    scheduled: Instant,
    callback: Box<dyn FnOnce()>,
}

type SenderCallback = Box<dyn FnOnce() + Send>;

pub(crate) trait PlatformRunLoopHotKeyDelegate {
    fn on_hot_key(&self, hot_key: i32);
}

pub(crate) trait PlatformRunLoopStatusItemDelegate {
    fn on_status_item_message(&self, w_param: WPARAM, l_param: LPARAM);
}

struct State {
    next_handle: Cell<HandleType>,
    hwnd: Cell<HWND>,
    timers: RefCell<HashMap<HandleType, Timer>>,

    // Callbacks sent from other threads
    sender_callbacks: Arc<Mutex<Vec<SenderCallback>>>,

    hot_key_delegate: RefCell<Option<Weak<dyn PlatformRunLoopHotKeyDelegate>>>,
    status_item_delegate: RefCell<Option<Weak<dyn PlatformRunLoopStatusItemDelegate>>>,
}

impl State {
    fn new() -> Self {
        Self {
            next_handle: Cell::new(INVALID_HANDLE + 1),
            hwnd: Cell::new(HWND(0)),
            timers: RefCell::new(HashMap::new()),
            sender_callbacks: Arc::new(Mutex::new(Vec::new())),
            hot_key_delegate: RefCell::new(None),
            status_item_delegate: RefCell::new(None),
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
            SetTimer(self.hwnd.get(), 1, wait_time.as_millis() as u32, None);
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

    fn process_timers(&self) -> Instant {
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
            while GetMessageW(&mut message, HWND(0), 0, 0).as_bool() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn stop(&self) {
        unsafe { PostMessageW(HWND(0), WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
}

pub const WM_STATUS_ITEM: u32 = WM_USER + 1; // WPARAM contains menu HWND

impl WindowAdapter for State {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        match msg {
            WM_TIMER => {
                self.on_timer();
            }
            WM_USER => {
                self.process_callbacks();
            }
            WM_HOTKEY => {
                if let Some(delegate) = self
                    .hot_key_delegate
                    .borrow()
                    .as_ref()
                    .and_then(|d| d.upgrade())
                {
                    delegate.on_hot_key(w_param.0 as i32);
                }
            }
            WM_STATUS_ITEM => {
                if let Some(delegate) = self
                    .status_item_delegate
                    .borrow()
                    .as_ref()
                    .and_then(|d| d.upgrade())
                {
                    delegate.on_status_item_message(w_param, l_param);
                }
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

    pub(crate) fn hwnd(&self) -> HWND {
        self.state.hwnd.get()
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        self.state.schedule(in_time, callback)
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

    pub(crate) fn set_hot_key_delegate(&self, delegate: Weak<dyn PlatformRunLoopHotKeyDelegate>) {
        self.state.hot_key_delegate.borrow_mut().replace(delegate);
    }

    pub(crate) fn set_status_item_delegate(
        &self,
        delegate: Weak<dyn PlatformRunLoopStatusItemDelegate>,
    ) {
        self.state
            .status_item_delegate
            .borrow_mut()
            .replace(delegate);
    }
}

#[derive(Clone)]
pub struct PlatformRunLoopSender {
    hwnd: HWND,
    callbacks: Arc<Mutex<Vec<SenderCallback>>>,
}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        {
            let mut callbacks = self.callbacks.lock().unwrap();
            callbacks.push(Box::new(callback));
        }
        unsafe {
            PostMessageW(self.hwnd, WM_USER, WPARAM(0), LPARAM(0));
        }
    }
}
