use cocoa::base::{id, nil, BOOL, NO};
use log::warn;
use objc::rc::{autoreleasepool, StrongPtr};

use crate::shell::platform::platform_impl::utils::{class_from_string, to_nsstring};

use super::{
    binary_messenger::PlatformBinaryMessenger,
    error::{PlatformError, PlatformResult},
    texture::PlatformTextureRegistry,
};

pub struct PlatformEngine {
    pub(super) handle: StrongPtr,
    pub(super) view_controller: StrongPtr,
    texture_registry: PlatformTextureRegistry,
}

pub struct PlatformPlugin {
    pub name: String,
    pub class: String,
}

impl PlatformEngine {
    pub fn new(plugins: &[PlatformPlugin]) -> Self {
        autoreleasepool(|| unsafe {
            let class = class!(FlutterViewController);
            let view_controller: id = msg_send![class, alloc];
            let view_controller = StrongPtr::new(msg_send![view_controller, initWithProject: nil]);
            let engine: id = msg_send![*view_controller, engine];

            // register plugins with this engine
            for plugin in plugins {
                let class = class_from_string(&plugin.class);
                if class.is_null() {
                    warn!(
                        "Plugin {} for plugin {} not found",
                        plugin.name, plugin.class
                    );
                } else {
                    let registrar: id =
                        msg_send![engine, registrarForPlugin: *to_nsstring(&plugin.name)];
                    let () = msg_send![class, registerWithRegistrar: registrar];
                }
            }

            let engine = StrongPtr::retain(engine);
            Self {
                handle: engine.clone(),
                view_controller,
                texture_registry: PlatformTextureRegistry::new(engine),
            }
        })
    }

    pub(crate) fn texture_registry(&self) -> &PlatformTextureRegistry {
        &self.texture_registry
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
