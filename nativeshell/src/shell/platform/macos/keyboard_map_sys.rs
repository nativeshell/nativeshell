use std::{ffi::c_void, os::raw::c_ulong};

use core_foundation::{array::CFIndex, dictionary::CFDictionaryRef, string::CFStringRef};

pub type CFObject = *mut c_void;
pub type CFNotificationCenterRef = CFObject;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub static kTISPropertyUnicodeKeyLayoutData: CFObject;
    pub static kTISNotifySelectedKeyboardInputSourceChanged: CFStringRef;
    pub fn TISCopyCurrentKeyboardLayoutInputSource() -> CFObject;
    pub fn TISCopyCurrentASCIICapableKeyboardLayoutInputSource() -> CFObject;
    pub fn TISGetInputSourceProperty(input_source: CFObject, property_key: CFObject)
        -> *mut c_void;

    pub fn LMGetKbdType() -> u32;

    pub fn UCKeyTranslate(
        layout_ptr: *mut c_void,
        virtual_key_code: u16,
        key_action: u16,
        modifier_key_state: u32,
        keyboard_type: u32,
        key_translate_options: u32,
        dead_code_state: *mut u32,
        max_string_length: c_ulong,
        actual_string_length: *mut c_ulong,
        unicode_string: *mut u16,
    );
}

pub type CFNotificationCallback = Option<
    extern "C" fn(
        center: CFNotificationCenterRef,
        observer: *mut c_void,
        name: CFStringRef,
        object: *const c_void,
        userInfo: CFDictionaryRef,
    ),
>;

pub type CFNotificationSuspensionBehavior = CFIndex;
pub const CFNotificationSuspensionBehaviorCoalesce: CFIndex = 2;

extern "C" {
    pub fn CFNotificationCenterGetDistributedCenter() -> CFNotificationCenterRef;
    pub fn CFNotificationCenterAddObserver(
        center: CFNotificationCenterRef,
        observer: *const c_void,
        callBack: CFNotificationCallback,
        name: CFStringRef,
        object: *const c_void,
        suspensionBehavior: CFNotificationSuspensionBehavior,
    );
    pub fn CFNotificationCenterRemoveObserver(
        center: CFNotificationCenterRef,
        observer: *const c_void,
        name: CFStringRef,
        object: *const c_void,
    );
}

#[allow(non_upper_case_globals)]
pub const kUCKeyActionDisplay: u16 = 3;
#[allow(non_upper_case_globals)]
pub const kUCKeyTranslateNoDeadKeysBit: u32 = 0;
#[allow(non_upper_case_globals)]
pub const kUCKeyTranslateNoDeadKeysMask: u32 = 1 << kUCKeyTranslateNoDeadKeysBit;
#[allow(non_upper_case_globals)]
pub const cmdKey: u32 = 256;
#[allow(non_upper_case_globals)]
pub const shiftKey: u32 = 512;
#[allow(non_upper_case_globals)]
pub const altKey: u32 = 2048;
