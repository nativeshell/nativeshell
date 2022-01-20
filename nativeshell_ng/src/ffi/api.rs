pub mod api {
    use std::{
        cell::RefCell,
        ffi::CStr,
        mem,
        os::raw::{c_char, c_int, c_void},
    };

    use once_cell::sync::OnceCell;

    use crate::ffi::dart::{raw::CObject, raw::Port};

    pub type PortHandler = unsafe extern "C" fn(port: Port, message: *const CObject);

    #[derive(Clone, Debug, PartialEq)]
    pub struct Functions {
        pub post_cobject: unsafe extern "C" fn(Port, *mut CObject) -> bool,
        pub post_integer: unsafe extern "C" fn(Port, i64) -> bool,
        pub new_native_port: unsafe extern "C" fn(*const c_char, PortHandler, bool) -> Port,
        pub close_native_port: unsafe extern "C" fn(Port) -> bool,
    }

    impl Functions {
        /// Returns resolved FFI functions. Will panic if FFI has not been initialized yet.
        ///
        /// See [`nativeshell_init_ffi`].
        pub fn get() -> Self {
            FUNCTIONS_TL.with(|m| {
                m.borrow_mut()
                    .get_or_insert_with(|| {
                        FUNCTIONS
                            .get()
                            .expect("NativeShell FFI not initialized.")
                            .clone()
                    })
                    .clone()
            })
        }
    }

    // Implementation

    static FUNCTIONS: OnceCell<Functions> = OnceCell::new();

    thread_local! {
        static FUNCTIONS_TL: RefCell<Option<Functions>> = RefCell::new(None);
    }

    #[repr(C)]
    struct ApiEntry {
        name: *const c_char,
        function: *const c_void,
    }

    #[repr(C)]
    struct Api {
        major: c_int,
        minor: c_int,
        functions: *const ApiEntry,
    }

    impl Api {
        fn lookup_fn(&self, name: &str) -> *const c_void {
            for i in 0..usize::MAX {
                let entry = unsafe { self.functions.add(i) };
                let entry = unsafe { &*entry };
                if entry.name == std::ptr::null_mut() {
                    break;
                }
                let fn_name = unsafe { CStr::from_ptr(entry.name) };
                if name == fn_name.to_string_lossy() {
                    return entry.function;
                }
            }
            panic!("FFI function ${} not found", name);
        }
    }

    pub(super) fn init(ptr: *mut c_void) {
        let functions = unsafe {
            let api = ptr as *const Api;
            let api = &*api;
            if api.major != 2 {
                panic!("Unsupported Dart API version {}.{}", api.major, api.minor);
            }
            Functions {
                post_cobject: mem::transmute(api.lookup_fn("Dart_PostCObject")),
                post_integer: mem::transmute(api.lookup_fn("Dart_PostInteger")),
                new_native_port: mem::transmute(api.lookup_fn("Dart_NewNativePort")),
                close_native_port: mem::transmute(api.lookup_fn("Dart_CloseNativePort")),
            }
        };
        if let Some(prev_functions) = FUNCTIONS.get() {
            if prev_functions != &functions {
                panic!("NativeShell FFI already initialized but with different function pointers");
            }
        }
        FUNCTIONS.set(functions).unwrap();
    }
}

/// Initializes FFI. Needs to be called before any other Dart FFI function. Can be called
/// multiple times, but the function pointers must remain same between calls.
///
/// # Arguments
///
/// * `ptr` - Pointer to the Dart API obtained through ffi.NativeApi.initializeApiDLData
///
#[no_mangle]
pub extern "C" fn nativeshell_init_ffi(ptr: *mut std::os::raw::c_void) -> () {
    api::init(ptr);
}
