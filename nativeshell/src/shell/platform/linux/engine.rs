use gtk::prelude::WidgetExt;

use super::{
    binary_messenger::PlatformBinaryMessenger,
    error::PlatformResult,
    flutter::{self, EngineExt, ViewExt},
};

pub struct PlatformEngine {
    pub(super) view: flutter::View,
}

#[repr(C)]
struct _FlEngine {
    _parent_instance: gobject_sys::GObject,
    _thread: isize,
    _project: isize,
    _renderer: isize,
    _binary_messenger: isize,
    _settings_plugin: isize,
    _task_runner: isize,
    _aot_data: isize,
    _engine: isize,
}

pub struct PlatformPlugin {
    pub name: String,
    pub register_func: Option<unsafe extern "C" fn(registrar: *mut std::os::raw::c_void)>,
}

impl PlatformEngine {
    pub fn new(plugins: &[PlatformPlugin]) -> Self {
        let project = flutter::DartProject::new();
        let view = flutter::View::new(&project);
        for plugin in plugins {
            let registrar = view.get_registrar_for_plugin(&plugin.name);
            if let Some(func) = plugin.register_func {
                unsafe {
                    func(registrar);
                }
            }
        }
        PlatformEngine { view }
    }

    pub fn new_binary_messenger(&self) -> PlatformBinaryMessenger {
        PlatformBinaryMessenger::new(self.view.get_engine().get_binary_messenger())
    }

    pub fn launch(&mut self) -> PlatformResult<()> {
        // This assumes the view has already been added to GtkWindow
        self.view.realize();
        Ok(())
    }

    pub fn shut_down(&mut self) -> PlatformResult<()> {
        Ok(())
    }
}
