use std::panic::Location;

use log::{Level, Record};
use widestring::WideCString;

use super::{
    all_bindings::*,
    error::{PlatformError, PlatformResult},
};

pub(super) fn to_utf16(string: &str) -> Vec<u16> {
    let mut res: Vec<u16> = string.encode_utf16().collect();
    res.push(0);
    res
}

pub unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub fn get_raw_ptr<T>(t: &T) -> usize {
    struct Extractor(usize);
    unsafe {
        let s: &Extractor = std::mem::transmute(t);
        s.0
    }
}

pub unsafe fn com_object_from_ptr<T: Clone>(ptr: ::windows::RawPtr) -> Option<T> {
    if ptr == std::ptr::null_mut() {
        None
    } else {
        struct ComObject(windows::RawPtr);
        let e = ComObject(ptr);
        let t: &T = std::mem::transmute(&e);
        Some(t.clone())
    }
}

pub trait HRESULTExt {
    fn ok_log(&self) -> bool;
    fn as_platform_result(&self) -> PlatformResult<()>;
}

impl HRESULTExt for HRESULT {
    #[track_caller]
    fn ok_log(&self) -> bool {
        if self.is_err() {
            let location = Location::caller();
            log::logger().log(
                &Record::builder()
                    .args(format_args!(
                        "Unexpected windows error 0x{:X} ({}) at {}",
                        self.0,
                        hresult_description(self.0).unwrap_or("Unknown".into()),
                        location
                    ))
                    .file(Some(location.file()))
                    .line(Some(location.line()))
                    .level(Level::Error)
                    .build(),
            );
            false
        } else {
            true
        }
    }

    #[must_use]
    fn as_platform_result(&self) -> PlatformResult<()> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(PlatformError::HResult(self.0))
        }
    }
}

pub fn clipboard_format_to_string(format: u32) -> String {
    let mut buf: [u16; 4096] = [0; 4096];
    unsafe {
        let len =
            GetClipboardFormatNameW(format, PWSTR(buf.as_mut_ptr() as *mut _), buf.len() as i32);

        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub trait BoolResultExt {
    fn as_platform_result(&self) -> PlatformResult<()>;
}

#[allow(non_snake_case)]
fn HRESULT_FROM_WIN32(x: u32) -> u32 {
    if x as i32 <= 0 {
        x as u32
    } else {
        ((x & 0x0000FFFF) | (FACILITY_CODE::FACILITY_WIN32.0 << 16) | 0x80000000) as u32
    }
}

impl BoolResultExt for BOOL {
    #[must_use]
    fn as_platform_result(&self) -> PlatformResult<()> {
        if self.as_bool() {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            let err = HRESULT_FROM_WIN32(err.0);
            Err(PlatformError::HResult(err))
        }
    }
}

pub(super) fn hresult_description(hr: u32) -> Option<String> {
    const FORMAT_MESSAGE_MAX_WIDTH_MASK: u32 = 0x000000FF;
    unsafe {
        let message_buffer: *mut u16 = std::ptr::null_mut();
        let format_result = FormatMessageW(
            FORMAT_MESSAGE_OPTIONS::FORMAT_MESSAGE_FROM_SYSTEM
                | FORMAT_MESSAGE_OPTIONS::FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_OPTIONS::FORMAT_MESSAGE_IGNORE_INSERTS
                | FORMAT_MESSAGE_OPTIONS(FORMAT_MESSAGE_MAX_WIDTH_MASK),
            std::ptr::null_mut(),
            hr,
            0,
            PWSTR(message_buffer),
            0,
            std::ptr::null_mut(),
        );
        if format_result == 0 || message_buffer.is_null() {
            return None;
        }

        let result = WideCString::from_raw(message_buffer);
        LocalFree(message_buffer as isize);
        result.to_string().ok()
    }
}

pub(super) fn direct_composition_supported() -> bool {
    // for now dsiable direct composition until flutter composition problems
    // are resolved
    false
    // unsafe {
    //     let handle = GetModuleHandleW("dcomp.dll");
    //     if handle != 0 {
    //         GetProcAddress(handle, "DCompositionCreateDevice").is_some()
    //     } else {
    //         false
    //     }
    // }
}

#[inline]
#[allow(non_snake_case)]
pub fn MAKELONG(lo: u16, hi: u16) -> u32 {
    (lo as u32) | ((hi as u32) << 16)
}

#[inline]
#[allow(non_snake_case)]
pub fn LOWORD(l: u32) -> u16 {
    (l & 0xffff) as u16
}

#[inline]
#[allow(non_snake_case)]
pub fn HIWORD(l: u32) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

#[inline]
#[allow(non_snake_case)]
pub fn GET_X_LPARAM(lp: LPARAM) -> i32 {
    LOWORD(lp.0 as u32) as i32
}

#[inline]
#[allow(non_snake_case)]
pub fn GET_Y_LPARAM(lp: LPARAM) -> i32 {
    HIWORD(lp.0 as u32) as i32
}

#[inline]
pub fn clamp<T: PartialOrd>(input: T, min: T, max: T) -> T {
    debug_assert!(min <= max, "min must be less than or equal to max");
    if input < min {
        min
    } else if input > max {
        max
    } else {
        input
    }
}
