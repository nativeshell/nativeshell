#![allow(clippy::forget_copy)] // windows-rs !implement macro

use super::{all_bindings::*, bindings::*, util::create_instance};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Weak,
};

use crate::{
    shell::{
        api_model::{Key, KeyboardMap},
        Context, KeyboardMapDelegate,
    },
    util::{LateRefCell, OkLog},
};

pub struct PlatformKeyboardMap {
    weak_self: LateRefCell<Weak<PlatformKeyboardMap>>,
    source: RefCell<Option<ITfSource>>,
    cookie: Cell<u32>,
    cached_layout: RefCell<HashMap<isize, KeyboardMap>>,
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
            source: RefCell::new(None),
            cookie: Cell::new(TF_INVALID_COOKIE),
            cached_layout: RefCell::new(HashMap::new()),
            delegate,
        }
    }

    pub fn get_current_map(&self) -> KeyboardMap {
        let current = unsafe { GetKeyboardLayout(0) };
        self.cached_layout
            .borrow_mut()
            .entry(current.0)
            .or_insert_with(|| self.create_keyboard_layout())
            .clone()
    }

    fn create_keyboard_layout(&self) -> KeyboardMap {
        let key_map = get_key_map();

        let layout = unsafe { self.get_keyboard_layout() };
        let keys: Vec<Key> = unsafe {
            key_map
                .iter()
                .map(|a| self.key_from_entry(a, layout))
                .collect()
        };

        KeyboardMap { keys }
    }

    unsafe fn get_keyboard_layout(&self) -> HKL {
        let current = GetKeyboardLayout(0);

        // If current layout is ascii capable but with numbers having diacritics, accept that
        if self.is_ascii_capable(current, false) {
            return current;
        }

        let cnt = GetKeyboardLayoutList(0, std::ptr::null_mut());
        let mut vec: Vec<HKL> = vec![HKL(0); cnt as usize];
        GetKeyboardLayoutList(cnt, vec.as_mut_ptr());

        // if choosing from list, prefer layout that has actual numbers
        for l in &vec {
            if self.is_ascii_capable(*l, true) {
                return *l;
            }
        }
        for l in &vec {
            if self.is_ascii_capable(*l, false) {
                return *l;
            }
        }

        current
    }

    unsafe fn is_ascii_capable(&self, hkl: HKL, including_numbers: bool) -> bool {
        // A .. Z
        for vc in 0x41..0x5A {
            let sc = MapVirtualKeyW(vc, MAPVK_VK_TO_VSC);
            let char = Self::get_character(vc, sc, false, false, hkl);
            match char {
                Some(char) => {
                    if char < 'a' as u16 || char > 'z' as u16 {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if including_numbers {
            // 0 .. 9
            for vc in 0x30..0x39 {
                let sc = MapVirtualKeyW(vc, MAPVK_VK_TO_VSC);
                let char = Self::get_character(vc, sc, false, false, hkl);
                match char {
                    Some(char) => {
                        if char < '0' as u16 || char > '9' as u16 {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
        }
        true
    }

    unsafe fn get_character(vc: u32, sc: u32, shift: bool, alt: bool, hkl: HKL) -> Option<u16> {
        let key_state = &mut [0u8; 256];
        let buf = &mut [0u16, 10];

        if shift {
            key_state[VK_SHIFT.0 as usize] = 128;
        }

        if alt {
            key_state[VK_CONTROL.0 as usize] = 128;
            key_state[VK_MENU.0 as usize] = 128;
        }

        // According to documentation, since Windows 10 version 1607 if bit 2 is
        // set the call will not change keyboard state.
        let flags = 0x04;

        let res = ToUnicodeEx(
            vc,
            sc,
            key_state.as_ptr(),
            PWSTR(buf.as_mut_ptr()),
            buf.len() as i32,
            flags,
            hkl,
        );

        // Clear keyboard state
        loop {
            let key_state = &mut [0u8; 256];
            let buf = &mut [0u16, 10];
            let res = ToUnicodeEx(
                VK_SPACE.0 as u32,
                MapVirtualKeyW(VK_SPACE.0 as u32, MAPVK_VK_TO_VSC),
                key_state.as_ptr(),
                PWSTR(buf.as_mut_ptr()),
                buf.len() as i32,
                flags,
                hkl,
            );
            if res >= 0 {
                break;
            }
        }

        if res > 0 && buf[0] >= 0x20 {
            Some(buf[0])
        } else {
            None
        }
    }

    unsafe fn key_from_entry(&self, entry: &KeyMapEntry, hkl: HKL) -> Key {
        let mut key = Key {
            platform: entry.platform,
            physical: entry.physical,
            logical: entry.logical,
            logical_shift: None,
            logical_alt: None,
            logical_alt_shift: None,
            logical_meta: None,
        };

        let virtual_code = MapVirtualKeyW(entry.platform as u32, MAPVK_VSC_TO_VK);

        let character = Self::get_character(virtual_code, entry.platform as u32, false, false, hkl);

        // This is a printable character
        if let Some(character) = character {
            key.logical = Some(character as i64);

            key.logical_shift =
                Self::get_character(virtual_code, entry.platform as u32, true, false, hkl)
                    .map(|i| i as i64);

            key.logical_alt =
                Self::get_character(virtual_code, entry.platform as u32, false, true, hkl)
                    .map(|i| i as i64);

            key.logical_alt_shift =
                Self::get_character(virtual_code, entry.platform as u32, true, true, hkl)
                    .map(|i| i as i64);

            // println!(
            //     "{:?} - {:?} {:?} {:?} {:?}",
            //     virtual_code,
            //     key.logical
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_shift
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_alt
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into()),
            //     key.logical_alt_shift
            //         .map(|c| String::from_utf16_lossy(&[c as u16]))
            //         .unwrap_or("--".into())
            // );
        }

        key
    }

    pub fn assign_weak_self(&self, weak: Weak<PlatformKeyboardMap>) {
        self.weak_self.set(weak);
        let profiles: ITfInputProcessorProfiles =
            create_instance(&CLSID_TF_InputProcessorProfiles).unwrap();
        let source = profiles.cast::<ITfSource>().unwrap();
        let sink: ITfLanguageProfileNotifySink =
            LanguageProfileNotifySink::new(self.weak_self.borrow().clone()).into();

        unsafe {
            let cookie = source
                .AdviseSink(
                    &ITfLanguageProfileNotifySink::IID,
                    Some(sink.cast::<IUnknown>().unwrap()),
                )
                .ok_log()
                .unwrap_or(0);
            self.cookie.set(cookie);
        }

        self.source.borrow_mut().replace(source);
    }

    fn keyboard_layout_changed(&self) {
        if let Some(delegate) = self.delegate.upgrade() {
            delegate.borrow().keyboard_map_did_change();
        }
    }
}

impl Drop for PlatformKeyboardMap {
    fn drop(&mut self) {
        if let Some(source) = self.source.borrow().clone() {
            if self.cookie.get() != TF_INVALID_COOKIE {
                unsafe {
                    source.UnadviseSink(self.cookie.get()).ok_log();
                }
            }
        }
    }
}

//
// Implementation of ITfLanguageProfileNotifySink
//

#[implement(Windows::Win32::UI::TextServices::ITfLanguageProfileNotifySink)]
struct LanguageProfileNotifySink {
    target: Weak<PlatformKeyboardMap>,
}

#[allow(non_snake_case)]
impl LanguageProfileNotifySink {
    fn new(target: Weak<PlatformKeyboardMap>) -> Self {
        Self { target }
    }

    fn OnLanguageChange(&self, _langid: u16) -> windows::Result<BOOL> {
        Ok(true.into())
    }

    fn OnLanguageChanged(&self) -> ::windows::Result<()> {
        if let Some(target) = self.target.upgrade() {
            target.keyboard_layout_changed();
        }
        Ok(())
    }
}
