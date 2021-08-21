use super::{
    all_bindings::*,
    error::{PlatformError, PlatformResult},
};

pub(super) fn to_utf16(string: &str) -> Vec<u16> {
    let mut res: Vec<u16> = string.encode_utf16().collect();
    res.push(0);
    res
}

/// # Safety
///
/// Data must be properly aligned (see slice::from_raw_parts)
pub unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub fn get_raw_ptr<T>(t: &T) -> usize {
    struct Extractor(usize);
    unsafe {
        let s = &*(t as *const _ as *const Extractor);
        s.0
    }
}

/// # Safety
///
/// ptr must point to a valid COM object instance
pub unsafe fn com_object_from_ptr<T: Clone>(ptr: ::windows::RawPtr) -> Option<T> {
    if ptr.is_null() {
        None
    } else {
        struct ComObject(windows::RawPtr);
        let e = ComObject(ptr);
        let t = &*(&e as *const _ as *const T);
        Some(t.clone())
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
        ((x & 0x0000FFFF) | (FACILITY_WIN32.0 << 16) | 0x80000000) as u32
    }
}

impl BoolResultExt for BOOL {
    fn as_platform_result(&self) -> PlatformResult<()> {
        if self.as_bool() {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            let err = HRESULT_FROM_WIN32(err.0);
            let err = HRESULT(err);
            Err(PlatformError::WindowsError(err.into()))
        }
    }
}

pub fn create_instance<T: Interface>(clsid: &Guid) -> Result<T> {
    unsafe { CoCreateInstance(clsid, None, CLSCTX_ALL) }
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
