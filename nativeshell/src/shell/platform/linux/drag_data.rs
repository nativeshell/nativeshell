use std::{collections::HashMap, mem::take};

use gdk::Atom;
use gtk::SelectionData;
use lazy_static::__Deref;
use log::warn;
use percent_encoding::percent_decode_str;
use url::Url;

use crate::{
    codec::{MessageCodec, StandardMethodCodec, Value},
    shell::{api_constants::drag_data, ContextOptions},
};

pub trait DragDataSetter {
    fn set(&self, selection_data: &SelectionData);
    fn data_formats(&self) -> Vec<Atom>;
}

pub trait DragDataAdapter {
    fn retrieve_drag_data(&self, data: &SelectionData, data_out: &mut HashMap<String, Value>);
    fn data_formats(&self) -> Vec<Atom>;

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
    ) -> Vec<Box<dyn DragDataSetter>>;
}

pub(super) struct UriListDataAdapter {}

impl UriListDataAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

const FRAGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');

impl DragDataAdapter for UriListDataAdapter {
    fn retrieve_drag_data(&self, data: &SelectionData, data_out: &mut HashMap<String, Value>) {
        let mut uris = Vec::<String>::new();
        let data_uris = data.uris();
        for uri in data_uris {
            let uri = uri.trim().to_string();
            if !uris.contains(&uri) {
                uris.push(uri);
            }
        }
        if let Some(string) = data.text() {
            let parts = string.split('\n');
            for part in parts {
                let part = part.trim().to_string();
                if !part.is_empty() && !uris.contains(&part) {
                    uris.push(part);
                }
            }
        }

        let mut res_uris = Vec::<String>::new();
        let mut res_paths = Vec::<String>::new();

        for uri in uris {
            if let Ok(parsed) = Url::parse(&uri) {
                if parsed.scheme() == "file"
                    && parsed.query().is_none()
                    && parsed.fragment().is_none()
                {
                    res_paths.push(percent_decode_str(parsed.path()).decode_utf8_lossy().into());
                    continue;
                }
            }

            res_uris.push(uri);
        }

        let res_uris: Vec<Value> = res_uris.iter().map(|s| Value::String(s.into())).collect();
        let res_paths: Vec<Value> = res_paths.iter().map(|s| Value::String(s.into())).collect();

        if !res_uris.is_empty() {
            data_out.insert(drag_data::key::URLS.into(), Value::List(res_uris));
        }
        if !res_paths.is_empty() {
            data_out.insert(drag_data::key::FILES.into(), Value::List(res_paths));
        }
    }

    fn data_formats(&self) -> Vec<Atom> {
        vec![
            Atom::intern("text/uri-list"),
            Atom::intern("UTF8_STRING"),
            Atom::intern("COMPOUND_TEXT"),
            Atom::intern("TEXT"),
            Atom::intern("STRING"),
            Atom::intern("text/plain;charset=utf-8"),
            Atom::intern("text/plain"),
        ]
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
    ) -> Vec<Box<dyn DragDataSetter>> {
        let mut uris = Vec::<String>::new();

        let urls = data_in.remove(drag_data::key::URLS);
        if let Some(mut urls) = extract_string_list(urls) {
            uris.append(&mut urls);
        }

        let files = data_in.remove(drag_data::key::FILES);
        if let Some(files) = extract_string_list(files) {
            for file in files {
                let uri = format!(
                    "file://{}",
                    percent_encoding::utf8_percent_encode(&file, FRAGMENT).to_string()
                );
                if !uris.contains(&uri) {
                    uris.push(uri);
                }
            }
        }

        vec![
            Box::new(UriDragData {
                uris: uris.clone(),
                set_as_uris: true,
                formats: vec![Atom::intern("text/uri-list")],
            }),
            Box::new(UriDragData {
                uris,
                set_as_uris: false,
                formats: vec![
                    Atom::intern("UTF8_STRING"),
                    Atom::intern("COMPOUND_TEXT"),
                    Atom::intern("TEXT"),
                    Atom::intern("STRING"),
                    Atom::intern("text/plain;charset=utf-8"),
                    Atom::intern("text/plain"),
                ],
            }),
        ]
    }
}

fn extract_string_list(value: Option<Value>) -> Option<Vec<String>> {
    match value {
        Some(value) => {
            if let Value::List(list) = value {
                let mut res = Vec::new();
                for value in list {
                    if let Value::String(value) = value {
                        res.push(value)
                    } else {
                        panic!("Invalid value found in list: ${:?}", value);
                    }
                }
                return Some(res);
            }
            panic!("Invalid value: {:?}, expected list of strings", value)
        }
        None => None,
    }
}

struct UriDragData {
    uris: Vec<String>,
    set_as_uris: bool,
    formats: Vec<Atom>,
}

impl DragDataSetter for UriDragData {
    fn set(&self, selection_data: &SelectionData) {
        if self.set_as_uris {
            let uris: Vec<&str> = self.uris.iter().map(|s| s.deref()).collect();
            selection_data.set_uris(&uris);
        } else {
            let mut str = String::new();
            for uri in &self.uris {
                str.push_str(uri);
                str.push('\n');
            }
            selection_data.set_text(&str);
        }
    }

    fn data_formats(&self) -> Vec<Atom> {
        self.formats.clone()
    }
}

//
//
//

pub struct FallThroughDragDataAdapter {
    format: Atom,
}

impl FallThroughDragDataAdapter {
    pub fn new(context_option: &ContextOptions) -> Self {
        Self {
            format: Atom::intern(&format!(
                "FLUTTER_INTERNAL/{}",
                context_option.app_namespace
            )),
        }
    }
}

impl DragDataAdapter for FallThroughDragDataAdapter {
    fn retrieve_drag_data(&self, data: &SelectionData, data_out: &mut HashMap<String, Value>) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        let data = data.data();
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

    fn data_formats(&self) -> Vec<Atom> {
        vec![self.format]
    }

    fn prepare_drag_data(
        &self,
        data_in: &mut HashMap<String, Value>,
    ) -> Vec<Box<dyn DragDataSetter>> {
        vec![Box::new(FallthroughDragDataSetter {
            values: take(data_in),
            format: self.format,
        })]
    }
}

struct FallthroughDragDataSetter {
    values: HashMap<String, Value>,
    format: Atom,
}

impl DragDataSetter for FallthroughDragDataSetter {
    fn set(&self, selection_data: &SelectionData) {
        let codec: &'static dyn MessageCodec<Value> = &StandardMethodCodec;
        let mut map = HashMap::new();
        for e in &self.values {
            map.insert(Value::String(e.0.into()), e.1.clone());
        }
        let data = codec.encode_message(&Value::Map(map));
        selection_data.set(&self.format, 0, &data);
    }

    fn data_formats(&self) -> Vec<Atom> {
        vec![self.format]
    }
}
