use std::{
    collections::HashMap,
    ffi::CString,
    sync::{Arc, Mutex},
};

use super::dart;
use dart::IntoDart;
use once_cell::sync::OnceCell;

/// Wraps dart port and provides method to send messages.
pub struct Port {
    port: dart::raw::Port,
}

impl Port {
    pub fn new(port: dart::raw::Port) -> Port {
        Port { port }
    }

    /// Sends a message to the port.
    ///
    /// Returns true if message was successfully posted;
    pub fn send<T: IntoDart>(&self, value: T) -> bool {
        let mut value = value.into_dart();

        let functions = dart::api::Functions::get();
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
    port: dart::raw::Port,
}

impl NativePort {
    pub fn new<F>(name: &str, handler: F) -> Self
    where
        F: Fn(dart::Value) + Send + Sync + 'static,
    {
        let name = CString::new(name).unwrap();
        let functions = dart::api::Functions::get();
        let port =
            unsafe { (functions.new_native_port)(name.as_ptr(), Self::handle_message, false) };
        let global_data = Self::global_data();
        let mut global_data = global_data.lock().unwrap();
        let handler = Arc::new(handler);
        global_data.insert(port, handler);
        Self { port }
    }

    pub fn as_send_port(&self) -> dart::raw::CObjectSendPort {
        dart::raw::CObjectSendPort {
            id: self.port,
            origin_id: -1,
        }
    }

    unsafe extern "C" fn handle_message(port: dart::raw::Port, message: *const dart::raw::CObject) {
        let handler = {
            let global_data = Self::global_data();
            let global_data = global_data.lock().unwrap();
            global_data.get(&port).cloned()
        };
        if let Some(handler) = handler {
            let value = dart::Value::from_dart(message);
            handler(value);
        }
    }

    fn global_data(
    ) -> &'static Mutex<HashMap<dart::raw::Port, Arc<dyn Fn(dart::Value) + Sync + Send>>> {
        static INSTANCE: OnceCell<
            Mutex<HashMap<dart::raw::Port, Arc<dyn Fn(dart::Value) + Sync + Send>>>,
        > = OnceCell::new();
        INSTANCE.get_or_init(|| {
            let m = HashMap::new();
            Mutex::new(m)
        })
    }
}

impl Drop for NativePort {
    fn drop(&mut self) {
        let global_data = Self::global_data();
        let mut global_data = global_data.lock().unwrap();
        global_data.remove(&self.port);
        let functions = dart::api::Functions::get();
        unsafe { (functions.close_native_port)(self.port) };
    }
}
