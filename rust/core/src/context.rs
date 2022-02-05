use std::{cell::RefCell, rc::Rc};

use crate::{util::black_box, api::nativeshell_init_ffi};

use super::RunLoop;

pub struct Context {
    internal: Rc<ContextInternal>,
    outermost: bool,
}

pub struct ContextInternal {
    run_loop: RunLoop,
    // message_channel: MessageChannel,
}

impl Context {
    /// Creates a new context. The will be be associated with current and can be retrieved
    /// at any point while the instance is in scope by [`Context::get()`].
    ///
    /// Any NativeShell application must have exactly one context active.
    pub fn new() -> Self {
        let res = Self {
            internal: Rc::new(ContextInternal {
                run_loop: RunLoop::new(),
                // message_channel: MessageChannel::new(),
            }),
            outermost: true,
        };
        ffi_methods();
        let prev_context = CURRENT_CONTEXT.with(|c| c.replace(Some(res.clone())));
        if prev_context.is_some() {
            panic!("Another context was already associated with current thread.");
        }
        // res.message_channel.init();
        res
    }

    pub fn run_loop(&self) -> &RunLoop {
        &self.internal.run_loop
    }

    // pub fn message_channel(&self) -> &MessageChannel {
    //     &self.message_channel
    // }

    /// Returns context associated with current thread. Can only be called
    /// on main thread and only while the original (outer-most) context is
    /// still in scope. Otherwise the function will panic.
    pub fn get() -> Self {
        Self::current().expect("No Context is associated with current thread.")
    }

    /// Returns context associated with current thread.
    pub fn current() -> Option<Self> {
        CURRENT_CONTEXT.with(|c| c.borrow().as_ref().map(|c| c.clone()))
    }
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.outermost {
            CURRENT_CONTEXT.with(|c| c.take());
        }
    }
}

impl Context {
    // Intentionally private
    fn clone(&self) -> Context {
        Context {
            internal: self.internal.clone(),
            outermost: false,
        }
    }
}

fn ffi_methods() {
    // this ensures that all FFI methods are referenced and not removed by linker
    if black_box(false) {
        nativeshell_init_ffi(std::ptr::null_mut());
        // nativeshell_vec_methods();
    }
}
