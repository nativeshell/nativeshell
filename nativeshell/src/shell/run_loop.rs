use std::{rc::Rc, time::Duration};

use super::{
    platform::run_loop::{PlatformRunLoop, PlatformRunLoopSender},
    Handle,
};

pub struct RunLoop {
    platform_run_loop: Rc<PlatformRunLoop>,
}

impl RunLoop {
    pub fn new() -> Self {
        Self {
            platform_run_loop: Rc::new(PlatformRunLoop::new()),
        }
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> Handle
    where
        F: FnOnce() + 'static,
    {
        let run_loop = self.platform_run_loop.clone();
        let handle = run_loop.schedule(in_time, callback);
        Handle::new(move || {
            run_loop.unschedule(handle);
        })
    }

    // Convenience method to schedule callback on next run loop turn
    #[must_use]
    pub fn schedule_now<F>(&self, callback: F) -> Handle
    where
        F: FnOnce() + 'static,
    {
        self.schedule(Duration::from_secs(0), callback)
    }

    pub fn run(&self) {
        self.platform_run_loop.run()
    }

    pub fn stop(&self) {
        self.platform_run_loop.stop()
    }

    pub fn new_sender(&self) -> RunLoopSender {
        RunLoopSender {
            platform_sender: self.platform_run_loop.new_sender(),
        }
    }
}

// Can be used to send callbacks from other threads to be executed on run loop thread
#[derive(Clone)]
pub struct RunLoopSender {
    platform_sender: PlatformRunLoopSender,
}

impl RunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        self.platform_sender.send(callback)
    }
}
