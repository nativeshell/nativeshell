use std::{collections::HashMap, ffi::CStr};

use super::utils::{from_nsstring, to_nsdata, to_nsstring};
use crate::{
    codec::{MessageCodec, StandardMethodCodec, Value},
    shell::{api_constants::drag_data, ContextOptions},
};
use cocoa::{
    base::{id, nil},
    foundation::NSArray,
};
use log::warn;
use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

pub trait DragDataAdapter {
    // Retrieve data from given pasteboard
    fn retrieve_drag_data(&self, pasteboard: id, data_out: &mut HashMap<String, Value>);

    // Remove recognized data from data_in and store it in pasteboard item
    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        pasteboard_items: &mut PasteboardItems,
    );

    fn register_types(&self, types: &mut Vec<StrongPtr>);
}

pub struct PasteboardItems {
    items: Vec<StrongPtr>,
    current_index: usize,
}

impl PasteboardItems {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_index: 0,
        }
    }

    pub fn reset_index(&mut self) {
        self.current_index = 0;
    }

    pub fn next_item(&mut self) -> StrongPtr {
        if self.current_index == self.items.len() {
            let item = unsafe {
                let item: id = msg_send![class!(NSPasteboardItem), alloc];
                StrongPtr::new(msg_send![item, init])
            };
            self.items.push(item);
        }
        let res = self.items.get(self.current_index).unwrap().clone();
        self.current_index += 1;
        res
    }

    pub fn get_items(&mut self) -> Vec<StrongPtr> {
        self.items.clone()
    }
}

//
// Default implementations
//

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub static NSPasteboardTypeFileURL: id;
    pub static NSPasteboardTypeURL: id;
}

pub(super) struct FilesDragDataAdapter {}

impl FilesDragDataAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl DragDataAdapter for FilesDragDataAdapter {
    fn retrieve_drag_data(&self, pasteboard: id, data_out: &mut HashMap<String, Value>) {
        unsafe {
            let mut res = Vec::<String>::new();

            let file_url = to_nsstring("public.file-url"); // NSPasteboardTypeFileURL
            let filenames = to_nsstring("NSFilenamesPboardType");

            let items: id = msg_send![pasteboard, pasteboardItems];
            for i in 0..NSArray::count(items) {
                let item = NSArray::objectAtIndex(items, i);
                let string: id = msg_send![item, stringForType: *file_url];
                if string != nil {
                    let url: id = msg_send![class!(NSURL), URLWithString: string];
                    let path: *const i8 = msg_send![url, fileSystemRepresentation];
                    res.push(CStr::from_ptr(path).to_string_lossy().into());
                }
            }

            let files: id = msg_send![pasteboard, propertyListForType: *filenames];

            for i in 0..NSArray::count(files) {
                let path = NSArray::objectAtIndex(files, i);
                let path = from_nsstring(path);
                if !res.contains(&path) {
                    res.push(path);
                }
            }

            if !res.is_empty() {
                let res: Vec<Value> = res.iter().map(|s| Value::String(s.clone())).collect();
                data_out.insert(drag_data::key::FILES.into(), Value::List(res));
            }
        }
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        pasteboard_items: &mut PasteboardItems,
    ) {
        unsafe {
            let files = data_in.remove(drag_data::key::FILES);

            let file_url = to_nsstring("public.file-url"); // NSPasteboardTypeFileURL

            if let Some(files) = files {
                let files = extract_string_list(files);
                for file in &files {
                    let item = pasteboard_items.next_item();
                    let url: id = msg_send![class!(NSURL), fileURLWithPath: *to_nsstring(file)];
                    let string: id = msg_send![url, absoluteString];
                    let () = msg_send![*item, setString:string forType:*file_url];
                }
            }
        }
    }

    fn register_types(&self, types: &mut Vec<StrongPtr>) {
        types.push(to_nsstring("public.file-url"));
        types.push(to_nsstring("NSFilenamesPboardType"));
    }
}

pub(super) struct UrlsDragDataAdapter {}

impl UrlsDragDataAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl DragDataAdapter for UrlsDragDataAdapter {
    fn retrieve_drag_data(&self, pasteboard: id, data_out: &mut HashMap<String, Value>) {
        unsafe {
            let mut res = Vec::<String>::new();

            let url_type_1 = to_nsstring("public.url"); // NSPasteboardTypeFileURL
            let url_type_2 = to_nsstring("Apple URL pasteboard type"); // NSURLPboardType

            let items: id = msg_send![pasteboard, pasteboardItems];
            for i in 0..NSArray::count(items) {
                let item = NSArray::objectAtIndex(items, i);
                let string: id = msg_send![item, stringForType: *url_type_1];
                if string != nil {
                    res.push(from_nsstring(string));
                } else {
                    let string: id = msg_send![item, stringForType: *url_type_2];
                    if string != nil {
                        res.push(from_nsstring(string));
                    }
                }
            }

            if !res.is_empty() {
                let res: Vec<Value> = res.iter().map(|s| Value::String(s.clone())).collect();
                data_out.insert(drag_data::key::URLS.into(), Value::List(res));
            }
        }
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        pasteboard_items: &mut PasteboardItems,
    ) {
        unsafe {
            let urls = data_in.remove(drag_data::key::URLS);

            let url_type = to_nsstring("public.url"); // NSPasteboardTypeFileURL

            if let Some(urls) = urls {
                let urls = extract_string_list(urls);
                for url in &urls {
                    let item = pasteboard_items.next_item();
                    let string = to_nsstring(url);
                    let () = msg_send![*item, setString:*string forType:*url_type];
                }
            }
        }
    }

    fn register_types(&self, types: &mut Vec<StrongPtr>) {
        types.push(to_nsstring("public.url"));
        types.push(to_nsstring("Apple URL pasteboard type"));
    }
}

pub(super) struct FallThroughDragDataAdapter {
    format: StrongPtr,
}

impl FallThroughDragDataAdapter {
    pub fn new(context_options: &ContextOptions) -> Self {
        Self {
            format: to_nsstring(&format!(
                "private.FlutterInternal.{}",
                context_options.app_namespace
            )),
        }
    }
}

impl DragDataAdapter for FallThroughDragDataAdapter {
    fn retrieve_drag_data(&self, pasteboard: id, data_out: &mut HashMap<String, Value>) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        unsafe {
            let data: id = msg_send![pasteboard, dataForType:*self.format];
            if data != nil {
                let bytes: *const u8 = msg_send![data, bytes];
                let length: usize = msg_send![data, length];
                let data: &[u8] = std::slice::from_raw_parts(bytes, length);
                let value = codec.decode_message(data).unwrap();
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
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
        pasteboard_items: &mut PasteboardItems,
    ) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        let mut map = HashMap::new();
        for e in data_in.drain() {
            map.insert(e.0.into(), e.1);
        }
        let data = codec.encode_message(&Value::Map(map));
        let data = to_nsdata(&data);
        let item = pasteboard_items.next_item();
        unsafe {
            let () = msg_send![*item, setData:*data forType:*self.format];
        }
    }

    fn register_types(&self, types: &mut Vec<StrongPtr>) {
        types.push(self.format.clone());
    }
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
