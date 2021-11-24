use std::collections::HashMap;

use byte_slice_cast::AsByteSlice;
use log::warn;
use widestring::WideCString;
use windows::Win32::System::{
    Com::IDataObject, DataExchange::RegisterClipboardFormatW, SystemServices::CF_HDROP,
};

use crate::{
    codec::{MessageCodec, StandardMethodCodec, Value},
    shell::{api_constants::*, ContextOptions},
};

use super::drag_util::DataUtil;

pub trait DragDataAdapter {
    // Retrieve drag data from data object; This is called when receiving drop;
    // Key must match DragDataKey on dart side
    fn retrieve_drag_data(&self, data: IDataObject, data_out: &mut HashMap<String, Value>);

    // Remove recognized data from data_in and transform it to appropriate cliboard format
    // and put it to data_out; Key in data_out is windows cliboard format
    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        data_out: &mut HashMap<u32, Vec<u8>>,
    );
}

//
// Default implementations
//

pub(super) struct FilesDragDataAdapter {}

impl FilesDragDataAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl DragDataAdapter for FilesDragDataAdapter {
    fn retrieve_drag_data(&self, data: IDataObject, data_out: &mut HashMap<String, Value>) {
        let files = DataUtil::get_data(data, CF_HDROP.0)
            .map(DataUtil::extract_files)
            .ok();

        if let Some(files) = files {
            data_out.insert(
                drag_data::key::FILES.into(),
                Value::List(files.iter().map(|f| f.clone().into()).collect()),
            );
        }
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        data_out: &mut HashMap<u32, Vec<u8>>,
    ) {
        let files = data_in.remove(drag_data::key::FILES);
        if let Some(files) = files {
            data_out.insert(
                CF_HDROP.0,
                DataUtil::bundle_files(&extract_string_list(files)),
            );
        }
    }
}

pub(super) struct UrlsDragDataAdapter {
    format_inet_url: u32,
    format_inet_url_w: u32,
}

impl UrlsDragDataAdapter {
    pub fn new() -> Self {
        Self {
            format_inet_url: register_format("UniformResourceLocator"),
            format_inet_url_w: register_format("UniformResourceLocatorW"),
        }
    }

    fn get_url(&self, data_object: IDataObject) -> windows::core::Result<String> {
        let data = DataUtil::get_data(data_object.clone(), self.format_inet_url_w);
        if data.is_ok() {
            return data.map(|d| DataUtil::extract_url_w(&d));
        }
        let data = DataUtil::get_data(data_object, self.format_inet_url);
        data.map(|d| DataUtil::extract_url(&d))
    }
}

impl DragDataAdapter for UrlsDragDataAdapter {
    fn retrieve_drag_data(&self, data: IDataObject, data_out: &mut HashMap<String, Value>) {
        let url = self.get_url(data);
        if let Ok(url) = url {
            data_out.insert(drag_data::key::URLS.into(), Value::List(vec![url.into()]));
        }
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        data_out: &mut HashMap<u32, Vec<u8>>,
    ) {
        let urls = data_in.remove(drag_data::key::URLS);
        if let Some(urls) = urls {
            let strings = extract_string_list(urls);
            if strings.len() > 1 {
                warn!("Only one URL is supported in drag data on Windows");
            }
            if let Some(url) = strings.first() {
                let url = WideCString::from_str(&url).unwrap();
                let bytes = url.as_slice().as_byte_slice();
                let mut data = Vec::from(bytes);
                data.extend_from_slice(&[0, 0]);
                data_out.insert(self.format_inet_url_w, data);
            }
        }
    }
}

pub(super) struct FallThroughDragDataAdapter {
    format: u32,
}

impl FallThroughDragDataAdapter {
    pub fn new(context_options: &ContextOptions) -> Self {
        Self {
            format: register_format(&format!(
                "FlutterInternal:{}",
                context_options.app_namespace
            )),
        }
    }
}

impl DragDataAdapter for FallThroughDragDataAdapter {
    fn retrieve_drag_data(&self, data: IDataObject, data_out: &mut HashMap<String, Value>) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;

        let data = DataUtil::get_data(data, self.format);
        if let Ok(data) = data {
            let value = codec.decode_message(&data).unwrap();
            if let Value::Map(value) = value {
                for entry in value {
                    if let Value::String(key) = entry.0 {
                        data_out.insert(key, entry.1);
                    } else {
                        warn!("Unexpected key type {:?}", entry.0);
                    }
                }
            } else {
                warn!("Unexpected value in clipboard {:?}", value);
            }
        }
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        data_out: &mut HashMap<u32, Vec<u8>>,
    ) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        let mut map = HashMap::new();
        for e in data_in.drain() {
            map.insert(e.0.into(), e.1);
        }
        let data = codec.encode_message(&Value::Map(map));
        data_out.insert(self.format, data);
    }
}

fn register_format(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(name) }
}

fn extract_string_list(value: Value) -> Vec<String> {
    if let Value::List(list) = value {
        let mut res = Vec::new();
        for value in list {
            if let Value::String(value) = value {
                res.push(value)
            } else {
                panic!("Invalid value found in list: ${:?}", value);
            }
        }
        return res;
    }
    panic!("Invalid value: {:?}, expected list of strings", value)
}
