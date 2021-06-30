use super::all_bindings::*;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::rc::Weak;

use crate::shell::api_model::Key;
use crate::shell::KeyboardMapDelegate;
use crate::{
    shell::{api_model::KeyboardMap, Context},
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
            key_state[VK_SHIFT as usize] = 128;
        }

        if alt {
            key_state[VK_CONTROL as usize] = 128;
            key_state[VK_MENU as usize] = 128;
        }

        let res = ToUnicodeEx(
            vc,
            sc,
            key_state.as_ptr(),
            PWSTR(buf.as_mut_ptr()),
            buf.len() as i32,
            0,
            hkl,
        );

        // Clear keyboard buffer
        loop {
            let key_state = &mut [0u8; 256];
            let buf = &mut [0u16, 10];
            let res = ToUnicodeEx(
                vc,
                sc,
                key_state.as_ptr(),
                PWSTR(buf.as_mut_ptr()),
                buf.len() as i32,
                0,
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
        let sink = LanguageProfileNotifySink::new(self.weak_self.borrow().clone());

        unsafe {
            source
                .AdviseSink(
                    &ITfLanguageProfileNotifySink::IID,
                    Some(sink.cast::<IUnknown>().unwrap()),
                    self.cookie.as_ptr(),
                )
                .ok_log();
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

#[repr(C)]
struct LanguageProfileNotifySink {
    _abi: Box<ITfLanguageProfileNotifySink_abi>,
    ref_cnt: u32,
    target: Weak<PlatformKeyboardMap>,
}

#[allow(dead_code)]
impl LanguageProfileNotifySink {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(target: Weak<PlatformKeyboardMap>) -> ITfLanguageProfileNotifySink {
        let sink = Box::new(Self {
            _abi: Box::new(ITfLanguageProfileNotifySink_abi(
                Self::_query_interface,
                Self::_add_ref,
                Self::_release,
                Self::_on_language_change,
                Self::_on_language_changed,
            )),
            ref_cnt: 1,
            target,
        });

        unsafe {
            let ptr = Box::into_raw(sink);
            mem::transmute(ptr)
        }
    }

    fn query_interface(
        &mut self,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> HRESULT {
        if iid == &ITfLanguageProfileNotifySink::IID || iid == &IUnknown::IID {
            unsafe {
                *interface = self as *mut Self as *mut _;
            }
            self.add_ref();
            S_OK
        } else {
            E_NOINTERFACE
        }
    }

    fn add_ref(&mut self) -> u32 {
        self.ref_cnt += 1;
        self.ref_cnt
    }

    fn release(&mut self) -> u32 {
        self.ref_cnt -= 1;
        let res = self.ref_cnt;

        if res == 0 {
            unsafe {
                Box::from_raw(self as *mut Self);
            }
        }

        res
    }

    fn language_changed(&self) -> HRESULT {
        if let Some(target) = self.target.upgrade() {
            target.keyboard_layout_changed();
        }
        S_OK
    }

    unsafe extern "system" fn _query_interface(
        this: ::windows::RawPtr,
        iid: &::windows::Guid,
        interface: *mut ::windows::RawPtr,
    ) -> windows::HRESULT {
        (*(this as *mut Self)).query_interface(iid, interface)
    }

    unsafe extern "system" fn _add_ref(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).add_ref()
    }

    unsafe extern "system" fn _release(this: ::windows::RawPtr) -> u32 {
        (*(this as *mut Self)).release()
    }

    unsafe extern "system" fn _on_language_change(
        _this: ::windows::RawPtr,
        _langid: u16,
        pfaccept: *mut BOOL,
    ) -> HRESULT {
        *pfaccept = true.into();
        S_OK
    }
    unsafe extern "system" fn _on_language_changed(this: ::windows::RawPtr) -> HRESULT {
        (*(this as *mut Self)).language_changed()
    }
}
