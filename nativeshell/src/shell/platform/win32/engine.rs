use std::{ffi::CString, ptr};

use super::{
    binary_messenger::PlatformBinaryMessenger,
    error::PlatformResult,
    flutter_sys::{
        FlutterDesktopEngineCreate, FlutterDesktopEngineDestroy, FlutterDesktopEngineGetMessenger,
        FlutterDesktopEngineGetPluginRegistrar, FlutterDesktopEngineProperties,
        FlutterDesktopEngineRef,
    },
    util::to_utf16,
};

pub type PlatformEngineType = FlutterDesktopEngineRef;

pub struct PlatformEngine {
    pub(crate) handle: PlatformEngineType,
}

pub struct PlatformPlugin {
    pub name: String,
    pub register_func: Option<unsafe extern "C" fn(registrar: *mut std::os::raw::c_void)>,
}

impl PlatformEngine {
    pub fn new(plugins: &[PlatformPlugin]) -> Self {
        let assets = to_utf16("data\\flutter_assets");
        let icu = to_utf16("data\\icudtl.dat");
        let aot = to_utf16("data\\app.so");
        let properties = FlutterDesktopEngineProperties {
            assets_path: assets.as_ptr(),
            icu_data_path: icu.as_ptr(),
            aot_library_path: aot.as_ptr(),
            dart_entrypoint_argc: 0,
            dart_entrypoint_argv: ptr::null_mut(),
        };

        let engine = unsafe { FlutterDesktopEngineCreate(&properties) };

        // register plugins
        for plugin in plugins {
            let name = CString::new(plugin.name.as_str()).unwrap();
            let registrar =
                unsafe { FlutterDesktopEngineGetPluginRegistrar(engine, name.as_ptr()) };
            if let Some(register_func) = plugin.register_func {
                unsafe {
                    register_func(registrar as *mut _);
                }
            }
        }
        Self { handle: engine }
    }

    pub fn new_binary_messenger(&self) -> PlatformBinaryMessenger {
        let messenger = unsafe { FlutterDesktopEngineGetMessenger(self.handle) };
        PlatformBinaryMessenger::from_handle(messenger)
    }

    pub fn launch(&mut self) -> PlatformResult<()> {
        // This is a bit inconsistent; On windows engine is unconditionally launched from controller
        // unsafe { FlutterDesktopEngineRun(self.handle, ptr::null()); }
        Ok(())
    }

    pub fn shut_down(&mut self) -> PlatformResult<()> {
        unsafe {
            FlutterDesktopEngineDestroy(self.handle);
        }
        Ok(())
    }
}
