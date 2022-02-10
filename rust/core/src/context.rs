use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
};

use crate::{message_channel::nativeshell_get_ffi_context, util::black_box};

use super::RunLoop;

pub struct Context {
    internal: Rc<ContextInternal>,
    outermost: bool,
}

pub struct ContextInternal {
    run_loop: RunLoop,
    attachments: RefCell<HashMap<TypeId, Box<dyn Any>>>,
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
                attachments: RefCell::new(HashMap::new()),
            }),
            outermost: true,
        };
        ffi_methods();
        let prev_context = CURRENT_CONTEXT.with(|c| c.replace(Some(res.clone())));
        if prev_context.is_some() {
            panic!("another context is already associated with current thread.");
        }
        res
    }

    pub fn run_loop(&self) -> &RunLoop {
        &self.internal.run_loop
    }

    pub fn get_attachment<T: Any, F: FnOnce() -> T>(&self, on_init: F) -> Ref<T> {
        let id = TypeId::of::<T>();
        // Do a separate check here, make sure attachments is not borrowed while
        // creating the attachment
        if !self.internal.attachments.borrow().contains_key(&id) {
            let attachment = Box::new(on_init());
            self.internal
                .attachments
                .borrow_mut()
                .insert(id, attachment);
        }
        let map = self.internal.attachments.borrow();
        Ref::map(map, |r| {
            let any = r.get(&id).unwrap();
            any.downcast_ref::<T>().unwrap()
        })
    }

    /// Returns context associated with current thread. Can only be called
    /// on main thread and only while the original (outer-most) context is
    /// still in scope. Otherwise the function will panic.
    pub fn get() -> Self {
        Self::current().expect("no context is associated with current thread.")
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
        nativeshell_get_ffi_context(std::ptr::null_mut());
    }
}
