use std::time::Duration;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        Self {}
    }

    pub fn unschedule(&self, handle: HandleType) {}

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        INVALID_HANDLE
    }

    pub fn run(&self) {}

    pub fn stop(&self) {}

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
    }
}
