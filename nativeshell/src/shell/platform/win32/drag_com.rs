// in generated code
#![allow(clippy::forget_copy)]

use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    mem::forget,
    rc::Weak,
    slice,
};

use windows::HRESULT;

use super::{
    all_bindings::*,
    bindings::*,
    drag_util::{CLSID_DragDropHelper, DataUtil},
    util::{com_object_from_ptr, create_instance, get_raw_ptr},
};

use crate::util::OkLog;

pub trait DropTargetDelegate {
    fn drag_enter(&self, object: IDataObject, pt: &POINTL, effect_mask: u32) -> u32;
    fn drag_over(&self, pt: &POINTL, effect_mask: u32) -> u32;
    fn drag_leave(&self);
    fn perform_drop(&self, object: IDataObject, pt: &POINTL, effect_mask: u32) -> u32;
}

//
// DropTarget
//

#[implement(Windows::Win32::System::Com::IDropTarget)]
pub(super) struct DropTarget {
    drop_target_helper: IDropTargetHelper,
    hwnd: HWND,
    delegate: Weak<dyn DropTargetDelegate>,
}

#[allow(non_snake_case)]
impl DropTarget {
    pub fn new(hwnd: HWND, delegate: Weak<dyn DropTargetDelegate>) -> Self {
        let helper: IDropTargetHelper = create_instance(&CLSID_DragDropHelper).unwrap();
        Self {
            drop_target_helper: helper,
            hwnd,
            delegate,
        }
    }

    fn DragEnter(
        &self,
        pdataobj: &Option<IDataObject>,
        _grfkeystate: u32,
        pt: POINTL,
        pdweffect: *mut u32,
    ) -> ::windows::Result<()> {
        unsafe {
            if let (Some(delegate), Some(p_data_obj)) = //
                (self.delegate.upgrade(), pdataobj)
            {
                *pdweffect = delegate.drag_enter(p_data_obj.clone(), &pt, *pdweffect);
            }

            let mut point = POINT { x: pt.x, y: pt.y };
            self.drop_target_helper
                .DragEnter(self.hwnd, pdataobj, &mut point as *mut _, *pdweffect)
                .ok_log();
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: u32,
        pt: POINTL,
        pdweffect: *mut u32,
    ) -> ::windows::Result<()> {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                *pdweffect = delegate.drag_over(&pt, *pdweffect);
            }

            let mut point = POINT { x: pt.x, y: pt.y };
            self.drop_target_helper
                .DragOver(&mut point as *mut _, *pdweffect)
                .ok_log();
        }
        Ok(())
    }

    fn DragLeave(&self) -> ::windows::Result<()> {
        unsafe {
            if let Some(delegate) = self.delegate.upgrade() {
                delegate.drag_leave();
            }

            self.drop_target_helper.DragLeave().ok_log();
        }
        Ok(())
    }

    fn Drop(
        &self,
        pdataobj: &Option<IDataObject>,
        _grfkeystate: u32,
        pt: POINTL,
        pdweffect: *mut u32,
    ) -> ::windows::Result<()> {
        unsafe {
            if let (Some(delegate), Some(pdataobj)) = //
                (self.delegate.upgrade(), pdataobj)
            {
                *pdweffect = delegate.perform_drop(pdataobj.clone(), &pt, *pdweffect);
            }

            let mut point = POINT { x: pt.x, y: pt.y };
            self.drop_target_helper
                .Drop(pdataobj, &mut point as *mut _, *pdweffect)
                .ok_log();
        }
        Ok(())
    }
}

//
// EnumFormatETC
//

#[implement(Windows::Win32::System::Com::IEnumFORMATETC)]
struct EnumFORMATETC {
    formats: Vec<FORMATETC>,
    index: usize,
}

#[allow(non_snake_case)]
impl EnumFORMATETC {
    fn new_(formats: Vec<FORMATETC>, index: usize) -> EnumFORMATETC {
        Self { formats, index }
    }

    pub fn new(formats: Vec<FORMATETC>) -> EnumFORMATETC {
        Self::new_(formats, 0)
    }

    fn remaining(&self) -> usize {
        self.formats.len() - self.index
    }

    fn Next(
        &mut self,
        mut celt: u32,
        rgelt: *mut FORMATETC,
        pcelt_fetched: *mut u32,
    ) -> ::windows::HRESULT {
        let mut offset = 0;
        let dest: &mut [FORMATETC] = unsafe { slice::from_raw_parts_mut(rgelt, celt as usize) };
        while celt > 0 && self.remaining() > 0 {
            dest[offset] = *self.formats.get(self.index).unwrap();

            celt -= 1;
            self.index += 1;
            offset += 1;
        }
        if !pcelt_fetched.is_null() {
            unsafe {
                *pcelt_fetched = offset as u32;
            }
        }
        if celt > 0 {
            S_FALSE
        } else {
            S_OK
        }
    }

    fn Skip(&mut self, mut celt: u32) -> ::windows::HRESULT {
        while celt > 0 && self.remaining() > 0 {
            celt -= 1;
            self.index += 1;
        }
        if celt > 0 {
            S_FALSE
        } else {
            S_OK
        }
    }

    fn Reset(&mut self) -> ::windows::HRESULT {
        self.index = 0;
        S_OK
    }

    fn Clone(&self) -> ::windows::Result<IEnumFORMATETC> {
        let clone = EnumFORMATETC::new_(self.formats.clone(), self.index);
        Ok(clone.into())
    }
}

//
// DataObject
//

const DATA_E_FORMATETC: HRESULT = HRESULT((-2147221404 + 1) as u32);

#[implement(Windows::Win32::System::Com::IDataObject)]
pub struct DataObject {
    data: Weak<RefCell<HashMap<u32, Vec<u8>>>>,
}

#[allow(non_snake_case)]
impl DataObject {
    pub fn new(data: Weak<RefCell<HashMap<u32, Vec<u8>>>>) -> Self {
        Self { data }
    }

    fn with_data_or<F, R>(&self, callback: F, or: R) -> R
    where
        F: FnOnce(RefMut<HashMap<u32, Vec<u8>>>) -> R,
    {
        if let Some(data) = self.data.upgrade() {
            callback(data.as_ref().borrow_mut())
        } else {
            or
        }
    }

    fn GetData(&self, pformatetc_in: *const FORMATETC) -> ::windows::Result<STGMEDIUM> {
        let format = unsafe { &*pformatetc_in };

        // println!(
        //     "GET: {}, {}",
        //     clipboard_format_to_string(format.cfFormat as u32),
        //     format.tymed
        // );

        self.with_data_or(
            |data| {
                if format.tymed == TYMED_HGLOBAL.0 as u32 {
                    let data = data.get(&(format.cfFormat as u32));
                    if let Some(data) = data {
                        let global = unsafe {
                            let global = GlobalAlloc(0.into(), data.len());
                            let global_data = GlobalLock(global);
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                global_data as *mut u8,
                                data.len(),
                            );
                            GlobalUnlock(global);
                            global
                        };

                        Ok(STGMEDIUM {
                            tymed: TYMED_HGLOBAL.0 as u32,
                            Anonymous: STGMEDIUM_0 { hGlobal: global },
                            pUnkForRelease: None,
                        })
                    } else {
                        Err(Error::fast_error(DATA_E_FORMATETC))
                    }
                } else if format.tymed == TYMED_ISTREAM.0 as u32 {
                    unsafe {
                        let data = data.get(&(format.cfFormat as u32));

                        if let Some(data) = data {
                            let stream = SHCreateMemStream(data.as_ptr(), data.len() as u32);
                            stream.clone().unwrap().Seek(0, STREAM_SEEK_END).ok_log();
                            let res = Ok(STGMEDIUM {
                                tymed: TYMED_ISTREAM.0 as u32,
                                Anonymous: STGMEDIUM_0 {
                                    pstm: get_raw_ptr(&stream) as windows::RawPtr,
                                },
                                pUnkForRelease: None,
                            });
                            forget(stream); // will be released through sgtmedium
                            res
                        } else {
                            Err(Error::fast_error(DATA_E_FORMATETC))
                        }
                    }
                } else {
                    Err(Error::fast_error(DATA_E_FORMATETC))
                }
            },
            Err(Error::fast_error(DATA_E_FORMATETC)),
        )
    }

    fn GetDataHere(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> ::windows::Result<()> {
        Err(Error::fast_error(DATA_E_FORMATETC))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> ::windows::Result<()> {
        self.with_data_or(
            |data| {
                let format = unsafe { &*pformatetc };
                if (format.tymed == TYMED_HGLOBAL.0 as u32
                    || format.tymed == TYMED_ISTREAM.0 as u32)
                    && data.contains_key(&(format.cfFormat as u32))
                {
                    Ok(())
                } else {
                    Err(Error::fast_error(S_FALSE))
                }
            },
            Err(Error::fast_error(S_FALSE)),
        )
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
    ) -> ::windows::Result<FORMATETC> {
        Err(Error::fast_error(E_NOTIMPL))
    }

    fn SetData(
        &self,
        pformatetc: *const FORMATETC,
        pmedium: *const STGMEDIUM,
        frelease: BOOL,
    ) -> ::windows::Result<()> {
        let format = unsafe { &*pformatetc };

        self.with_data_or(
            |mut data| {
                if format.tymed == TYMED_HGLOBAL.0 as u32 {
                    unsafe {
                        let medium = &*pmedium;
                        let size = GlobalSize(medium.Anonymous.hGlobal);
                        let global_data = GlobalLock(medium.Anonymous.hGlobal);

                        let v = slice::from_raw_parts(global_data as *const u8, size);
                        let global_data: Vec<u8> = v.into();

                        GlobalUnlock(medium.Anonymous.hGlobal);
                        data.insert(format.cfFormat as u32, global_data);

                        if frelease.as_bool() {
                            ReleaseStgMedium(pmedium as *mut _);
                        }
                    }

                    Ok(())
                } else if format.tymed == TYMED_ISTREAM.0 as u32 {
                    unsafe {
                        let medium = &*pmedium;

                        let stream: Option<IStream> = com_object_from_ptr(medium.Anonymous.pstm);

                        let mut stream_data = Vec::<u8>::new();
                        let mut buf: [u8; 4096] = [0; 4096];
                        if let Some(stream) = stream {
                            loop {
                                let mut num_read: u32 = 0;
                                if stream
                                    .Read(
                                        buf.as_mut_ptr() as *mut _,
                                        buf.len() as u32,
                                        &mut num_read as *mut _,
                                    )
                                    .is_err()
                                {
                                    break;
                                }

                                if num_read == 0 {
                                    break;
                                }
                                stream_data.extend_from_slice(&buf[..num_read as usize]);
                            }
                        }

                        data.insert(format.cfFormat as u32, stream_data);

                        if frelease.as_bool() {
                            ReleaseStgMedium(pmedium as *mut _);
                        }
                    }

                    Ok(())
                } else {
                    Err(Error::fast_error(DATA_E_FORMATETC))
                }
            },
            Err(Error::fast_error(DATA_E_FORMATETC)),
        )
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> ::windows::Result<IEnumFORMATETC> {
        let mut formats = Vec::<FORMATETC>::new();

        self.with_data_or(
            |data| {
                if dwdirection == DATADIR_GET.0 as u32 {
                    for f in data.keys() {
                        formats.push(DataUtil::get_format_with_tymed(*f, TYMED_HGLOBAL));
                        formats.push(DataUtil::get_format_with_tymed(*f, TYMED_ISTREAM));
                    }
                }
                let enum_format = EnumFORMATETC::new(formats).into();
                Ok(enum_format)
            },
            Err(Error::fast_error(S_FALSE)),
        )
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: &Option<IAdviseSink>,
    ) -> ::windows::Result<u32> {
        Err(Error::fast_error(DATA_E_FORMATETC))
    }

    fn DUnadvise(&self, _dwconnection: u32) -> ::windows::Result<()> {
        Err(Error::fast_error(DATA_E_FORMATETC))
    }

    fn EnumDAdvise(&self) -> ::windows::Result<IEnumSTATDATA> {
        Err(Error::fast_error(DATA_E_FORMATETC))
    }
}

//
// DropSource
//

#[implement(Windows::Win32::System::Com::IDropSource)]
pub struct DropSource {}

#[allow(non_snake_case)]
impl DropSource {
    pub fn new() -> DropSource {
        Self {}
    }

    fn QueryContinueDrag(&self, f_escape_pressed: BOOL, grf_key_state: u32) -> ::windows::HRESULT {
        if f_escape_pressed.as_bool() {
            DRAGDROP_S_CANCEL
        } else if grf_key_state & MK_LBUTTON as u32 == 0 {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dw_effect: u32) -> ::windows::HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}
