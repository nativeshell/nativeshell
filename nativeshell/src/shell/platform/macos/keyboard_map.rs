use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    mem::ManuallyDrop,
    os::raw::c_ulong,
    rc::Weak,
};

use core_foundation::{
    base::CFRelease,
    data::{CFDataGetBytePtr, CFDataRef},
    dictionary::CFDictionaryRef,
    string::CFStringRef,
};

use crate::{
    shell::{
        api_model::{Key, KeyboardMap},
        keyboard_map_manager::KeyboardMapDelegate,
        platform::platform_impl::keyboard_map_sys::{
            altKey, kTISPropertyUnicodeKeyLayoutData, shiftKey, CFObject, TISGetInputSourceProperty,
        },
        Context,
    },
    util::LateRefCell,
};

use super::keyboard_map_sys::{
    cmdKey, kTISNotifySelectedKeyboardInputSourceChanged, kUCKeyActionDisplay,
    kUCKeyTranslateNoDeadKeysMask, CFNotificationCenterAddObserver,
    CFNotificationCenterGetDistributedCenter, CFNotificationCenterRef,
    CFNotificationCenterRemoveObserver, CFNotificationSuspensionBehaviorCoalesce, LMGetKbdType,
    TISCopyCurrentASCIICapableKeyboardLayoutInputSource, UCKeyTranslate,
};

pub struct PlatformKeyboardMap {
    weak_self: LateRefCell<Weak<PlatformKeyboardMap>>,
    observer: Cell<*const PlatformKeyboardMap>,
    current_layout: RefCell<Option<KeyboardMap>>,
    delegate: Weak<RefCell<dyn KeyboardMapDelegate>>,
}

include!(std::concat!(
    std::env!("OUT_DIR"),
    "/generated_keyboard_map.rs"
));

impl PlatformKeyboardMap {
    pub fn new(_context: Context, delegate: Weak<RefCell<dyn KeyboardMapDelegate>>) -> Self {
        Self {
            weak_self: LateRefCell::new(),
            observer: Cell::new(std::ptr::null_mut()),
            current_layout: RefCell::new(None),
            delegate,
        }
    }

    pub fn get_current_map(&self) -> KeyboardMap {
        self.current_layout
            .borrow_mut()
            .get_or_insert_with(|| self.create_keyboard_layout())
            .clone()
    }

    fn create_keyboard_layout(&self) -> KeyboardMap {
        let key_map = get_key_map();
        unsafe {
            let input_source = TISCopyCurrentASCIICapableKeyboardLayoutInputSource();
            let layout_data: CFObject =
                TISGetInputSourceProperty(input_source, kTISPropertyUnicodeKeyLayoutData);

            let keys: Vec<Key> = key_map
                .iter()
                .map(|a| self.key_from_entry(a, layout_data))
                .collect();

            CFRelease(input_source);

            KeyboardMap { keys }
        }
    }

    unsafe fn key_from_entry(&self, entry: &KeyMapEntry, layout_data: CFObject) -> Key {
        match entry.logical {
            Some(logical) => Key {
                platform: entry.platform,
                physical: entry.physical,
                logical: Some(logical),
                logical_shift: None,
                logical_alt: None,
                logical_alt_shift: None,
                logical_meta: None,
            },
            None => {
                let mut logical_key = None::<i64>;
                let mut logical_key_shift = None::<i64>;
                let mut logical_key_alt = None::<i64>;
                let mut logical_key_alt_shift = None::<i64>;
                let mut logical_key_cmd = None::<i64>;

                let mut dead_key_state: u32 = 0;
                let mut unichar: u16 = 0;
                let mut unichar_count: c_ulong = 0;

                let layout = CFDataGetBytePtr(layout_data as CFDataRef);

                UCKeyTranslate(
                    layout as *mut _,
                    entry.platform as u16,
                    kUCKeyActionDisplay,
                    0,
                    LMGetKbdType(),
                    kUCKeyTranslateNoDeadKeysMask,
                    &mut dead_key_state as *mut _,
                    1,
                    &mut unichar_count as *mut _,
                    &mut unichar as *mut _,
                );

                if unichar_count > 0 {
                    logical_key.replace(unichar as i64);
                }

                UCKeyTranslate(
                    layout as *mut _,
                    entry.platform as u16,
                    kUCKeyActionDisplay,
                    (shiftKey >> 8) & 0xFF,
                    LMGetKbdType(),
                    kUCKeyTranslateNoDeadKeysMask,
                    &mut dead_key_state as *mut _,
                    1,
                    &mut unichar_count as *mut _,
                    &mut unichar as *mut _,
                );

                if unichar_count > 0 {
                    logical_key_shift.replace(unichar as i64);
                }

                UCKeyTranslate(
                    layout as *mut _,
                    entry.platform as u16,
                    kUCKeyActionDisplay,
                    (altKey >> 8) & 0xFF,
                    LMGetKbdType(),
                    kUCKeyTranslateNoDeadKeysMask,
                    &mut dead_key_state as *mut _,
                    1,
                    &mut unichar_count as *mut _,
                    &mut unichar as *mut _,
                );

                if unichar_count > 0 {
                    logical_key_alt.replace(unichar as i64);
                }

                UCKeyTranslate(
                    layout as *mut _,
                    entry.platform as u16,
                    kUCKeyActionDisplay,
                    (shiftKey >> 8) & 0xFF | (altKey >> 8) & 0xFF,
                    LMGetKbdType(),
                    kUCKeyTranslateNoDeadKeysMask,
                    &mut dead_key_state as *mut _,
                    1,
                    &mut unichar_count as *mut _,
                    &mut unichar as *mut _,
                );

                if unichar_count > 0 {
                    logical_key_alt_shift.replace(unichar as i64);
                }

                // On some keyboard (SVK), using CMD modifier keys when specifying keyboard
                // shortcut results in results in US layout key matched. So we need to know
                // the value with CMD modifier as well.
                // Example: ] key on SVK keyboard is ä, but when specifying NSMenuItem key equivalent
                // CMD + ä with SVK keybaord is never matched. The equivalent needs to be CMD + ].
                // On the other hand ' key on French AZERTY is ù, and CMD + ù key equivalent
                // is matched. That's possibly because UCKeyTranslate CMD + ] on SVK keyboard returns ],
                // whereas on French AZERTY UCKeyTranslate CMD + ' returns ù.
                UCKeyTranslate(
                    layout as *mut _,
                    entry.platform as u16,
                    kUCKeyActionDisplay,
                    (cmdKey >> 8) & 0xFF,
                    LMGetKbdType(),
                    kUCKeyTranslateNoDeadKeysMask,
                    &mut dead_key_state as *mut _,
                    1,
                    &mut unichar_count as *mut _,
                    &mut unichar as *mut _,
                );

                if unichar_count > 0 {
                    logical_key_cmd.replace(unichar as i64);
                }

                // println!(
                //     "KEY: {:?}, {:?} {:?} {:?} {:?}",
                //     entry.platform,
                //     logical_key,
                //     logical_key_shift,
                //     logical_key_alt,
                //     logical_key_alt_shift,
                // );

                Key {
                    platform: entry.platform,
                    physical: entry.physical,
                    logical: logical_key,
                    logical_shift: logical_key_shift,
                    logical_alt: logical_key_alt,
                    logical_alt_shift: logical_key_alt_shift,
                    logical_meta: logical_key_cmd,
                }
            }
        }
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformKeyboardMap>) {
        self.weak_self.set(weak.clone());

        let ptr = weak.into_raw();

        unsafe {
            let center = CFNotificationCenterGetDistributedCenter();
            CFNotificationCenterAddObserver(
                center,
                ptr as *const _,
                Some(observer),
                kTISNotifySelectedKeyboardInputSourceChanged,
                std::ptr::null_mut(),
                CFNotificationSuspensionBehaviorCoalesce,
            );
            self.observer.set(ptr);
        }
    }

    fn on_layout_changed(&self) {
        self.current_layout.borrow_mut().take();
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.borrow().keyboard_map_did_change();
        }
    }
}

impl Drop for PlatformKeyboardMap {
    fn drop(&mut self) {
        let observer = self.observer.replace(std::ptr::null_mut());
        if !observer.is_null() {
            unsafe {
                let center = CFNotificationCenterGetDistributedCenter();
                CFNotificationCenterRemoveObserver(
                    center,
                    observer as *const _,
                    kTISNotifySelectedKeyboardInputSourceChanged,
                    std::ptr::null_mut(),
                );
                Weak::from_raw(observer);
            }
        }
    }
}

extern "C" fn observer(
    _center: CFNotificationCenterRef,
    observer: *mut c_void,
    _name: CFStringRef,
    _object: *const c_void,
    _user_info: CFDictionaryRef,
) {
    let layout =
        ManuallyDrop::new(unsafe { Weak::from_raw(observer as *const PlatformKeyboardMap) });

    if let Some(layout) = layout.upgrade() {
        layout.on_layout_changed();
    }
}
