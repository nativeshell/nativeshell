use std::{
    cell::UnsafeCell,
    future::Future,
    rc::Rc,
    sync::Arc,
    thread::{self, ThreadId},
    time::Duration,
};

use futures::{
    future::LocalBoxFuture,
    task::{waker_ref, ArcWake},
    FutureExt,
};

use super::{
    platform::run_loop::{PlatformRunLoop, PlatformRunLoopSender},
    Context, ContextRef, Handle,
};

pub struct RunLoop {
    pub(super) platform_run_loop: Rc<PlatformRunLoop>,
    context: Context,
}

impl RunLoop {
    pub fn new(context: &ContextRef) -> Self {
        Self {
            platform_run_loop: Rc::new(PlatformRunLoop::new()),
            context: context.weak(),
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
        // set context as current
        let _handle = self.context.get().unwrap().set_current();
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

    // Spawn the future with current run loop being the executor;
    // Generally the future will be executed synchronously until first
    // await.
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static) {
        let future = future.boxed_local();
        let task = Arc::new(Task {
            sender: self.new_sender(),
            thread_id: thread::current().id(),
            future: UnsafeCell::new(future),
        });
        ArcWake::wake_by_ref(&task);
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

//
//
//

struct Task {
    sender: RunLoopSender,
    thread_id: ThreadId,
    future: UnsafeCell<LocalBoxFuture<'static, ()>>,
}

//
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    fn poll(self: &std::sync::Arc<Self>) {
        let waker = waker_ref(self).clone();
        let context = &mut core::task::Context::from_waker(&waker);
        unsafe {
            let future = &mut *self.future.get();
            let _ = future.as_mut().poll(context);
        }
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        let arc_self = arc_self.clone();
        if arc_self.thread_id == thread::current().id() {
            arc_self.poll();
        } else {
            let sender = arc_self.sender.clone();
            sender.send(move || {
                arc_self.poll();
            });
        }
    }
}
