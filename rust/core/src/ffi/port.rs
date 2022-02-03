use std::{
    collections::HashMap,
    ffi::CString,
    sync::{Arc, Mutex},
};

use super::{api::api, raw, DartValue, IntoDart};
use once_cell::sync::OnceCell;

/// Wraps dart port and provides a method to send messages.
#[derive(Clone, Debug)]
pub struct DartPort {
    port: raw::DartPort,
}

impl DartPort {
    pub fn new(port: raw::DartPort) -> DartPort {
        DartPort { port }
    }

    /// Sends a message to the port.
    ///
    /// Returns true if message was successfully posted;
    pub fn send<T: IntoDart>(&self, value: T) -> bool {
        let mut value = value.into_dart();

        let functions = api::DartFunctions::get();
        let res = unsafe { (functions.post_cobject)(self.port, &mut value as *mut _) };
        if !res {
            // If Dart_PostCObject returns false we need to perform the cleanup ourself.
            value.cleanup();
        }
        res
    }
}

/// NativePort can be used to receive messages from Dart.
pub struct NativePort {
    port: raw::DartPort,
}

impl NativePort {
    pub fn new<F>(name: &str, handler: F) -> Self
    where
        F: Fn(raw::DartPort, DartValue) + Send + Sync + 'static,
    {
        let name = CString::new(name).unwrap();
        let functions = api::DartFunctions::get();
        let port =
            unsafe { (functions.new_native_port)(name.as_ptr(), Self::handle_message, false) };
        let global_data = Self::global_data();
        let mut global_data = global_data.lock().unwrap();
        let handler = Arc::new(handler);
        global_data.insert(port, handler);
        Self { port }
    }

    pub fn as_send_port(&self) -> raw::DartCObjectSendPort {
        raw::DartCObjectSendPort {
            id: self.port,
            origin_id: -1,
        }
    }

    pub fn raw_port(&self) -> raw::DartPort {
        self.port
    }

    unsafe extern "C" fn handle_message(port: raw::DartPort, message: *const raw::DartCObject) {
        let handler = {
            let global_data = Self::global_data();
            let global_data = global_data.lock().unwrap();
            global_data.get(&port).cloned()
        };
        if let Some(handler) = handler {
            let value = DartValue::from_dart(message);
            handler(port, value);
        }
    }

    fn global_data() -> &'static Mutex<CallbackMapType> {
        static INSTANCE: OnceCell<Mutex<CallbackMapType>> = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let m = HashMap::new();
            Mutex::new(m)
        })
    }
}

type CallbackMapType = HashMap<raw::DartPort, Arc<dyn Fn(raw::DartPort, DartValue) + Sync + Send>>;

impl Drop for NativePort {
    fn drop(&mut self) {
        let global_data = Self::global_data();
        let mut global_data = global_data.lock().unwrap();
        global_data.remove(&self.port);
        let functions = api::DartFunctions::get();
        unsafe { (functions.close_native_port)(self.port) };
    }
}
