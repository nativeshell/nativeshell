use std::{
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
    task::Poll,
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
        let _handle = self.context.get().unwrap().set_as_current();
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
    pub fn spawn<T: 'static>(&self, future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
        let future = future.boxed_local();
        let task = Arc::new(Task {
            sender: self.new_sender(),
            future: UnsafeCell::new(future),
            value: RefCell::new(None),
            waker: RefCell::new(None),
        });
        ArcWake::wake_by_ref(&task);
        JoinHandle {
            task,
            _data: PhantomData {},
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

//
//
//

struct Task<T> {
    sender: RunLoopSender,
    future: UnsafeCell<LocalBoxFuture<'static, T>>,
    value: RefCell<Option<T>>,
    waker: RefCell<Option<std::task::Waker>>,
}

// Tasks can only be spawned on run loop thread and will only be executed
// on run loop thread. ArcWake however doesn't know this.
unsafe impl<T> Send for Task<T> {}
unsafe impl<T> Sync for Task<T> {}

impl<T: 'static> Task<T> {
    fn poll(self: &std::sync::Arc<Self>) -> Poll<T> {
        let waker = waker_ref(self).clone();
        let context = &mut core::task::Context::from_waker(&waker);
        unsafe {
            let future = &mut *self.future.get();
            future.as_mut().poll(context)
        }
    }
}

impl<T: 'static> ArcWake for Task<T> {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        let arc_self = arc_self.clone();
        let sender = arc_self.sender.clone();
        sender.send(move || {
            if arc_self.value.borrow().is_none() {
                if let Poll::Ready(value) = arc_self.poll() {
                    *arc_self.value.borrow_mut() = Some(value);
                }
            }
            if arc_self.value.borrow().is_some() {
                if let Some(waker) = arc_self.waker.borrow_mut().take() {
                    waker.wake();
                }
            }
        });
    }
}

pub struct JoinHandle<T> {
    task: Arc<Task<T>>,
    // Task has unsafe `Send` and `Sync`, but that is only because we know
    // it will not be polled from another thread. This is to ensure that
    // JoinHandle is neither Send nor Sync.
    _data: PhantomData<*const ()>,
}

impl<T: 'static> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let value = self.task.value.borrow_mut().take();
        match value {
            Some(value) => Poll::Ready(value),
            None => {
                self.task
                    .waker
                    .borrow_mut()
                    .get_or_insert_with(|| cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
