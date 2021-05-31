use std::{ffi::c_void, os::raw::c_ulong};

use core_foundation::{
    base::CFRelease,
    data::{CFDataGetBytePtr, CFDataRef},
};

type CFObject = *mut c_void;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: CFObject;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> CFObject;
    fn TISGetInputSourceProperty(input_source: CFObject, property_key: CFObject) -> *mut c_void;

    fn LMGetKbdType() -> u32;

    fn UCKeyTranslate(
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

fn unmodified_character_for_virtual_key(scancode: i64) -> (u16, u16) {
    unsafe {
        let input_source = TISCopyCurrentKeyboardLayoutInputSource();
        let layout_data: CFObject =
            TISGetInputSourceProperty(input_source, kTISPropertyUnicodeKeyLayoutData);

        let layout = CFDataGetBytePtr(layout_data as CFDataRef);

        let mut dead_key_state: u32 = 0;
        let mut unichar: u16 = 0;
        let mut unichar_shift: u16 = 0;
        let mut unichar_count: c_ulong = 0;

        #[allow(non_upper_case_globals)]
        const kUCKeyActionDisplay: u16 = 3;
        #[allow(non_upper_case_globals)]
        const kUCKeyTranslateNoDeadKeysBit: u32 = 0;
        #[allow(non_upper_case_globals)]
        const kUCKeyTranslateNoDeadKeysMask: u32 = 1 << kUCKeyTranslateNoDeadKeysBit;
        #[allow(non_upper_case_globals)]
        const shiftKey: u32 = 512;

        UCKeyTranslate(
            layout as *mut _,
            scancode as u16,
            kUCKeyActionDisplay,
            0,
            LMGetKbdType(),
            kUCKeyTranslateNoDeadKeysMask,
            &mut dead_key_state as *mut _,
            1,
            &mut unichar_count as *mut _,
            &mut unichar as *mut _,
        );

        UCKeyTranslate(
            layout as *mut _,
            scancode as u16,
            kUCKeyActionDisplay,
            (shiftKey >> 8) & 0xFF,
            LMGetKbdType(),
            kUCKeyTranslateNoDeadKeysMask,
            &mut dead_key_state as *mut _,
            1,
            &mut unichar_count as *mut _,
            &mut unichar_shift as *mut _,
        );

        CFRelease(input_source);
        (unichar, unichar_shift)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeyEvent {
    characters_ignoring_modifiers: Option<String>,
    characters_ignoring_modifiers_ex: Option<String>,
    characters_ignoring_modifiers_except_shift_ex: Option<String>,
    key_code: isize,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

pub fn process_key_event(data: Vec<u8>) -> Vec<u8> {
    let mut event: KeyEvent = serde_json::from_slice(&data).unwrap();
    //
    // [NSEvent charactersIgnoringModifiers] which is used as source for
    // characters_ignoring_modifiers doesn't ignore the SHIFT modifier
    //
    if event.characters_ignoring_modifiers.is_some() {
        let char = unmodified_character_for_virtual_key(event.key_code as i64);
        event.characters_ignoring_modifiers_ex =
            Some(String::from_utf16_lossy(std::slice::from_ref(&char.0)));
        event.characters_ignoring_modifiers_except_shift_ex =
            Some(String::from_utf16_lossy(std::slice::from_ref(&char.1)));
    }
    serde_json::to_vec(&event).unwrap()
}
