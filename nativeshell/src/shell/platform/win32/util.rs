use std::mem::size_of;

use libc::c_void;
use windows::{
    core::{Interface, RawPtr, GUID, HRESULT},
    Win32::{
        Foundation::{GetLastError, BOOL, HANDLE, HWND, LPARAM, PWSTR},
        Graphics::Gdi::{
            CreateDIBSection, GetDC, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS, HBITMAP,
        },
        System::{
            Com::{CoCreateInstance, CLSCTX_ALL},
            DataExchange::GetClipboardFormatNameW,
            Diagnostics::Debug::FACILITY_WIN32,
        },
    },
};

use crate::shell::api_model::ImageData;

use super::error::{PlatformError, PlatformResult};

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
    #[repr(transparent)]
    struct Extractor(usize);
    unsafe {
        let s = &*(t as *const _ as *const Extractor);
        s.0
    }
}

/// # Safety
///
/// ptr must point to a valid COM object instance
pub unsafe fn com_object_from_ptr<T: Clone>(ptr: RawPtr) -> Option<T> {
    if ptr.is_null() {
        None
    } else {
        #[repr(transparent)]
        struct ComObject(RawPtr);
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

pub fn create_instance<T: Interface>(clsid: &GUID) -> windows::core::Result<T> {
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

pub fn image_data_to_hbitmap(image: &ImageData) -> HBITMAP {
    let bitmap = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width,
            biHeight: image.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB as u32,
            biSizeImage: (image.width * image.height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: Default::default(),
    };

    unsafe {
        let dc = GetDC(HWND(0));

        let mut ptr = std::ptr::null_mut();

        let bitmap = CreateDIBSection(
            dc,
            &bitmap as *const _,
            DIB_RGB_COLORS,
            &mut ptr as *mut *mut _ as *mut *mut c_void,
            HANDLE(0),
            0,
        );

        // Bitmap needs to be flipped and unpremultiplied

        let dst_stride = (image.width * 4) as isize;
        let ptr = ptr as *mut u8;
        for y in 0..image.height as isize {
            let src_line = image
                .data
                .as_ptr()
                .offset((image.height as isize - y - 1) * image.bytes_per_row as isize);

            let dst_line = ptr.offset(y * dst_stride);

            for x in (0..dst_stride).step_by(4) {
                let (r, g, b, a) = (
                    *src_line.offset(x) as i32,
                    *src_line.offset(x + 1) as i32,
                    *src_line.offset(x + 2) as i32,
                    *src_line.offset(x + 3) as i32,
                );

                let (r, g, b) = if a == 0 {
                    (0, 0, 0)
                } else {
                    (r * 255 / a, g * 255 / a, b * 255 / a)
                };
                *dst_line.offset(x) = b as u8;
                *dst_line.offset(x + 1) = g as u8;
                *dst_line.offset(x + 2) = r as u8;
                *dst_line.offset(x + 3) = a as u8;
            }
        }

        ReleaseDC(HWND(0), dc);

        bitmap
    }
}
