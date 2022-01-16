use core::slice;
use std::{ffi::CStr, mem::size_of, ptr::null_mut, u32};

use widestring::WideCStr;
use windows::{
    core::GUID,
    Win32::{
        Foundation::POINT,
        System::{
            Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED, TYMED_HGLOBAL},
            Memory::{GlobalLock, GlobalSize, GlobalUnlock},
            Ole::{
                ReleaseStgMedium, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
                DROPEFFECT_NONE,
            },
        },
        UI::Shell::DROPFILES,
    },
};

use crate::shell::api_model::DragEffect;

use super::util::as_u8_slice;

use byte_slice_cast::*;

#[allow(non_upper_case_globals)]
pub const CLSID_DragDropHelper: GUID = GUID::from_values(
    0x4657278a,
    0x411b,
    0x11d2,
    [0x83, 0x9a, 0x0, 0xc0, 0x4f, 0xd9, 0x18, 0xd0],
);

pub fn convert_drop_effect_mask(mask: u32) -> Vec<DragEffect> {
    let mut res = Vec::new();

    if mask & DROPEFFECT_COPY == DROPEFFECT_COPY {
        res.push(DragEffect::Copy);
    }
    if mask & DROPEFFECT_MOVE == DROPEFFECT_MOVE {
        res.push(DragEffect::Move);
    }
    if mask & DROPEFFECT_LINK == DROPEFFECT_LINK {
        res.push(DragEffect::Link);
    }
    res
}

pub fn convert_drag_effect(effect: &DragEffect) -> u32 {
    match effect {
        DragEffect::None => DROPEFFECT_NONE,
        DragEffect::Copy => DROPEFFECT_COPY,
        DragEffect::Link => DROPEFFECT_LINK,
        DragEffect::Move => DROPEFFECT_MOVE,
    }
}

pub fn convert_drag_effects(effects: &[DragEffect]) -> u32 {
    let mut res: u32 = 0;
    for e in effects {
        res |= convert_drag_effect(e);
    }
    res
}

pub struct DataUtil {}

impl DataUtil {
    pub fn get_data(object: IDataObject, format: u32) -> windows::core::Result<Vec<u8>> {
        let mut format = Self::get_format(format);

        unsafe {
            let mut medium = object.GetData(&mut format as *mut _)?;

            let size = GlobalSize(medium.Anonymous.hGlobal);
            let data = GlobalLock(medium.Anonymous.hGlobal);

            let v = slice::from_raw_parts(data as *const u8, size);
            let res: Vec<u8> = v.into();

            GlobalUnlock(medium.Anonymous.hGlobal);

            ReleaseStgMedium(&mut medium as *mut STGMEDIUM);

            Ok(res)
        }
    }

    pub fn has_data(object: IDataObject, format: u32) -> bool {
        let mut format = Self::get_format(format);
        unsafe { object.QueryGetData(&mut format as *mut _).is_ok() }
    }

    pub fn extract_files(buffer: Vec<u8>) -> Vec<String> {
        let files: &DROPFILES = unsafe { &*(buffer.as_ptr() as *const DROPFILES) };

        let mut res = Vec::new();
        if { files.fWide }.as_bool() {
            let data = buffer.as_slice()[files.pFiles as usize..]
                .as_slice_of::<u16>()
                .unwrap();
            let mut offset = 0;
            loop {
                let str = WideCStr::from_slice_truncate(&data[offset..]).unwrap();
                if str.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy());
                offset += str.len() + 1;
            }
        } else {
            let data = &buffer.as_slice()[files.pFiles as usize..];
            let mut offset = 0;
            loop {
                let str = CStr::from_bytes_with_nul(&data[offset..]).unwrap();
                let bytes = str.to_bytes();
                if bytes.is_empty() {
                    break;
                }
                res.push(str.to_string_lossy().into());
                offset += bytes.len();
            }
        }
        res
    }

    pub fn extract_url_w(buffer: &[u8]) -> String {
        let data = buffer.as_slice_of::<u16>().unwrap();
        let str = WideCStr::from_slice_truncate(data).unwrap();
        str.to_string_lossy()
    }

    pub fn extract_url(buffer: &[u8]) -> String {
        let str = CStr::from_bytes_with_nul(buffer).unwrap();
        str.to_string_lossy().into()
    }

    pub fn bundle_files(files: &[String]) -> Vec<u8> {
        let mut res = Vec::new();

        let drop_files = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };

        let drop_files = unsafe { as_u8_slice(&drop_files) };
        res.extend_from_slice(drop_files);

        for f in files {
            let mut wide: Vec<u16> = f.encode_utf16().collect();
            wide.push(0);
            res.extend_from_slice(wide.as_byte_slice());
        }
        res.extend_from_slice(&[0, 0]);

        res
    }

    pub fn get_format(format: u32) -> FORMATETC {
        Self::get_format_with_tymed(format, TYMED_HGLOBAL)
    }

    pub fn get_format_with_tymed(format: u32, tymed: TYMED) -> FORMATETC {
        FORMATETC {
            cfFormat: format as u16,
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT.0 as u32,
            lindex: -1,
            tymed: tymed.0 as u32,
        }
    }
}
