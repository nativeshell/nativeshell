use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use core_foundation::base::OSStatus;
use log::warn;

use crate::{shell::Context, util::LateRefCell};

use super::hot_key_sys::{
    kEventClassKeyboard, kEventHotKeyPressed, EventHandlerCallRef, EventHandlerRef, EventParamName,
    EventRef, EventTypeSpec, GetEventDispatcherTarget, InstallEventHandler, RemoveEventHandler,
};

const tag: u32 = 1314080844; // NSHL

pub(crate) struct PlatformHotKeyManager {
    weak_self: LateRefCell<Weak<PlatformHotKeyManager>>,
    event_handler_ref: Cell<EventHandlerRef>,
}

impl PlatformHotKeyManager {
    pub fn new(context: Context) -> Self {
        Self {
            weak_self: LateRefCell::new(),
            event_handler_ref: Cell::new(std::ptr::null_mut()),
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformHotKeyManager>) {
        self.weak_self.set(weak.clone());

        let spec = EventTypeSpec {
            eventClass: kEventClassKeyboard,
            eventKind: kEventHotKeyPressed,
        };

        let ptr = Box::into_raw(Box::new(weak));
        let mut event_handler_ref: EventHandlerRef = std::ptr::null_mut();
        let status = unsafe {
            InstallEventHandler(
                GetEventDispatcherTarget(),
                Some(event_handler),
                1,
                &spec as *const _,
                ptr as *mut _,
                &mut event_handler_ref as *mut _,
            )
        };
        if status != 0 {
            warn!("Couldn't install event handler: {}", status);
        }
    }
}

impl Drop for PlatformHotKeyManager {
    fn drop(&mut self) {
        println!("Dropping");
        if !self.event_handler_ref.get().is_null() {
            unsafe { RemoveEventHandler(self.event_handler_ref.get()) };
        }
    }
}

unsafe extern "C" fn event_handler(
    inHandlerCallRef: EventHandlerCallRef,
    inEvent: EventRef,
    inUserData: *mut ::std::os::raw::c_void,
) -> OSStatus {
    0
}
