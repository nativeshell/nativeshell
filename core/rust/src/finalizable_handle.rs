use std::{
    cell::Cell,
    collections::HashMap,
    ffi::c_void,
    sync::{Mutex, MutexGuard},
};

use once_cell::sync::OnceCell;

use crate::{
    api::{DartFunctions, DartHandle, DartWeakPersistentHandle},
    RUN_LOOP_SENDER,
};

#[derive(Debug, PartialEq, PartialOrd, Hash)]
pub struct FinalizableHandle {
    pub(super) id: isize,
}

impl FinalizableHandle {
    pub fn new<F: FnOnce() + 'static>(external_size: isize, finalizer: F) -> Self {
        let id = NEXT_HANDLE.with(|c| {
            let res = c.get();
            c.replace(res + 1);
            res
        });
        let mut state = State::get();
        state.objects.insert(
            id,
            FinalizableObjectState {
                handle: None,
                external_size,
                finalizer: Some(Box::new(finalizer)),
            },
        );
        Self { id }
    }

    /// Whether this handle is attached to a Dart object.
    pub fn is_attached(&self) -> bool {
        let state = State::get();
        state
            .objects
            .get(&self.id)
            .map(|s| s.handle.is_some())
            .unwrap_or(false)
    }

    /// Whether the Dart object was already finalized.
    pub fn is_finalized(&self) -> bool {
        let state = State::get();
        state.objects.contains_key(&self.id)
    }

    /// Updates the external size. This is a hint to Dart garbage collector.
    pub fn update_size(size: isize) {
        let mut state = State::get();
        let object = state.objects.get_mut(&size);
        if let Some(object) = object {
            object.external_size = size;
            if let Some(handle) = object.handle {
                unsafe { (DartFunctions::get().update_External_size)(handle, size) };
            }
        }
    }
}

//
//
//

impl Drop for FinalizableHandle {
    fn drop(&mut self) {
        let mut state = State::get();
        state.objects.remove(&self.id);
    }
}

struct State {
    objects: HashMap<isize, FinalizableObjectState>,
}

unsafe impl Send for State {}

static FUNCTIONS: OnceCell<Mutex<State>> = OnceCell::new();

impl State {
    fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }

    fn get() -> MutexGuard<'static, Self> {
        let state = FUNCTIONS.get_or_init(|| Mutex::new(State::new()));
        state.lock().unwrap()
    }
}

struct FinalizableObjectState {
    handle: Option<DartWeakPersistentHandle>,
    external_size: isize,
    finalizer: Option<Box<dyn FnOnce()>>,
}

impl Drop for FinalizableObjectState {
    fn drop(&mut self) {
        if let Some(handle) = self.handle {
            unsafe { (DartFunctions::get().delete_weak_persistent_handle)(handle) };
        }
    }
}

fn finalize_handle(handle: isize) {
    let object_state = {
        let mut state = State::get();
        state.objects.remove(&handle)
    };
    if let Some(mut object_state) = object_state {
        object_state.handle.take(); // weak handle gets deleted when object is collected
        let finalizer = object_state
            .finalizer
            .take()
            .expect("Finalizer executed more than once");
        finalizer();
    }
}

unsafe extern "C" fn finalizer(_isolate_callback_data: *mut c_void, peer: *mut c_void) {
    let sender = RUN_LOOP_SENDER
        .get()
        .expect("MessageChannel was not initialized!");
    let handle = peer as isize;
    sender.send(move || {
        finalize_handle(handle);
    });
}

pub(crate) unsafe extern "C" fn attach_weak_persistent_handle(handle: DartHandle, id: isize) {
    let mut state = State::get();
    let object = state.objects.get_mut(&id);
    if let Some(object) = object {
        let handle = (DartFunctions::get().new_weak_persistent_handle)(
            handle,
            id as *mut c_void,
            object.external_size,
            finalizer,
        );
        object.handle = Some(handle);
    }
}

thread_local! {
    static NEXT_HANDLE: Cell<isize> = Cell::new(1);
}
