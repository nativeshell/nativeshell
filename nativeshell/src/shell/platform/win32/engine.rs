use std::{mem::size_of, ptr};

use crate::shell::platform::key_interceptor::override_key_event;

use super::{
    binary_messenger::PlatformBinaryMessenger,
    error::PlatformResult,
    flutter_api::{
        FlutterDesktopEngineCreate, FlutterDesktopEngineDestroy, FlutterDesktopEngineGetMessenger,
        FlutterDesktopEngineProperties, FlutterDesktopEngineRef,
    },
    util::to_utf16,
};

pub struct PlatformEngine {
    pub(super) handle: FlutterDesktopEngineRef,
}

impl PlatformEngine {
    pub fn new() -> Self {
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

        unsafe {
            // TODO: This makes assumption about internal engine layout and will possibly
            // break in future;
            override_key_event((engine as *mut u8).add(2 * size_of::<isize>()) as *mut _);
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
