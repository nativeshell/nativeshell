use std::{cell::RefCell, cmp::Ordering, hash::Hash, rc::Rc};

use crate::{
    raw::{self, DartCObjectExternalTypedData},
    Context, DartObject, DartValue, RunLoopSender,
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct NativePointer {
    pub pointer: isize,
    pub size: isize,
    finalizer: Rc<RefCell<Option<NativePointerFinalizer>>>,
}

impl NativePointer {
    ///
    ///  Creates a new NativePointer instance.
    ///
    /// # Arguments
    ///
    /// * `pointer` - value will be accessible in dart.
    /// * `size` - is a hint to garbage collector
    /// * `finalizer` - callback invoked when dart object gets garbage collected.
    ///
    pub fn new<F: FnOnce() + 'static>(pointer: isize, size: isize, finalizer: F) -> Self {
        Self {
            pointer,
            size,
            finalizer: Rc::new(RefCell::new(Some(NativePointerFinalizer(Box::new(
                finalizer,
            ))))),
        }
    }
}

//
//
//

impl From<NativePointer> for DartObject {
    fn from(object: NativePointer) -> Self {
        Self::NativePointer(object)
    }
}

impl Hash for NativePointer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let finalizer = self.finalizer.borrow();
        if let Some(finalizer) = finalizer.as_ref() {
            finalizer.hash(state)
        }
    }
}

impl NativePointer {
    fn call_finalizer(&self) {
        let mut finalizer = self.finalizer.borrow_mut();
        let finalizer = finalizer.take().expect("Finalizer was already executed!");
        finalizer.0();
    }
}

struct NativePointerFinalizer(Box<dyn FnOnce()>);

impl std::fmt::Debug for NativePointerFinalizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CustomObjectFinalizer").finish()
    }
}

impl PartialEq for NativePointerFinalizer {
    fn eq(&self, other: &Self) -> bool {
        let left: *const _ = self.0.as_ref();
        let right: *const _ = other.0.as_ref();
        left == right
    }

    fn ne(&self, other: &Self) -> bool {
        !(self.eq(other))
    }
}

impl PartialOrd for NativePointerFinalizer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let left: *const _ = self.0.as_ref();
        let right: *const _ = other.0.as_ref();
        left.partial_cmp(&right)
    }
}

impl Hash for NativePointerFinalizer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr: *const _ = self.0.as_ref();
        ptr.hash(state);
    }
}

unsafe extern "C" fn external_data_hack_finalizer(
    _isolate_callback_data: *mut std::ffi::c_void,
    peer: *mut std::ffi::c_void,
) {
    let peer = peer as *mut TrampolineData;
    let mut peer = Box::from_raw(peer);
    let sender = peer.sender.take().expect("Missing RunLoop Sender");
    let peer = TrampolineDataPointer(Box::into_raw(peer));
    sender.send(move || {
        let peer = peer;
        let peer = Box::from_raw(peer.0);
        peer.object.call_finalizer();
    })
}

struct TrampolineData {
    sender: Option<RunLoopSender>,
    object: NativePointer,
}

struct TrampolineDataPointer(*mut TrampolineData);

unsafe impl Send for TrampolineDataPointer {}

impl From<DartObject> for DartValue {
    fn from(object: DartObject) -> Self {
        match object {
            DartObject::SendPort(port) => port.into(),
            DartObject::Capability(capability) => capability.into(),
            // This is currently a hack; We should use NativePointer for this, but there is a bug
            // where the finalizer is never executed; Instead we use typed data with bogus content.
            // Deserialize must make sure that the data can not be accessed on flutter side.
            DartObject::NativePointer(object) => {
                let size = object.size;
                let data = Box::new(TrampolineData {
                    sender: Some(Context::get().run_loop().new_sender()),
                    object,
                });
                DartCObjectExternalTypedData {
                    ty: raw::DartTypedDataType::Uint8,
                    length: size,
                    data: std::ptr::null_mut(),
                    peer: Box::into_raw(data) as *mut _,
                    callback: external_data_hack_finalizer,
                }
                .into()
            }
        }
    }
}
