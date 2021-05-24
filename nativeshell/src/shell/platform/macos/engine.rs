use std::ffi::c_void;

use cocoa::base::{id, nil, BOOL, NO};
use objc::rc::{autoreleasepool, StrongPtr};

use crate::shell::platform::key_interceptor::override_key_event;

use super::{
    binary_messenger::PlatformBinaryMessenger,
    error::{PlatformError, PlatformResult},
};

pub struct PlatformEngine {
    handle: StrongPtr,
    pub(super) view_controller: StrongPtr,
}

impl PlatformEngine {
    pub fn new() -> Self {
        autoreleasepool(|| unsafe {
            let class = class!(FlutterViewController);
            let view_controller: id = msg_send![class, alloc];
            let view_controller = StrongPtr::new(msg_send![view_controller, initWithProject: nil]);
            let () = msg_send![*view_controller, setMouseTrackingMode: 3]; // always track mouse
            let engine: id = msg_send![*view_controller, engine];
            let embedder_api: *mut c_void = msg_send![engine, embedderAPI];
            override_key_event(embedder_api);
            Self {
                handle: StrongPtr::retain(engine),
                view_controller,
            }
        })
    }

    pub fn new_binary_messenger(&self) -> PlatformBinaryMessenger {
        autoreleasepool(|| unsafe {
            let messenger: id = msg_send![*self.handle, binaryMessenger];
            PlatformBinaryMessenger::from_handle(StrongPtr::retain(messenger))
        })
    }

    pub fn launch(&mut self) -> PlatformResult<()> {
        let res: BOOL =
            autoreleasepool(|| unsafe { msg_send![*self.view_controller, launchEngine] });
        if res == NO {
            Err(PlatformError::LaunchEngineFailure)
        } else {
            Ok(())
        }
    }

    pub fn shut_down(&mut self) -> PlatformResult<()> {
        autoreleasepool(|| unsafe {
            let () = msg_send![*self.handle, shutDownEngine];
        });
        Ok(())
    }
}
