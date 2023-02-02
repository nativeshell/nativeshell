use std::{
    fmt::Write as _,
    fs::{self, File},
    io::Write,
    path::Path,
};

use cmake::Config;

use crate::{
    artifacts_emitter::ArtifactsEmitter,
    util::{copy, copy_to, get_artifacts_dir, mkdir},
    BuildResult, FileOperation, Flutter, IOResultExt,
};

use super::Plugin;

pub(super) struct PluginsImpl<'a> {
    build: &'a Flutter<'a>,
    artifacts_emitter: &'a ArtifactsEmitter<'a>,
}

impl<'a> PluginsImpl<'a> {
    pub fn new(build: &'a Flutter, artifacts_emitter: &'a ArtifactsEmitter<'a>) -> Self {
        Self {
            build,
            artifacts_emitter,
        }
    }

    pub fn process(&self, plugins: &[Plugin], skip_build: bool) -> BuildResult<()> {
        let flutter_artifacts = self
            .artifacts_emitter
            .find_artifacts_location(&self.build.build_mode)?;

        let plugins_dir = mkdir(&self.build.out_dir, Some("plugins"))?;
        let flutter_files = [
            "flutter_export.h",
            "flutter_messenger.h",
            "flutter_plugin_registrar.h",
            "flutter_texture_registrar.h",
            "flutter_windows.h",
            "flutter_windows.dll",
            "flutter_windows.dll.lib",
        ];
        for file in &flutter_files {
            copy_to(flutter_artifacts.join(file), &plugins_dir, true)?;
        }

        let cpp_client_wrapper = self
            .artifacts_emitter
            .find_artifacts_location("debug")?
            .join("cpp_client_wrapper");
        copy_to(cpp_client_wrapper, &plugins_dir, true)?;

        let flutter = mkdir(&plugins_dir, Some("flutter"))?;
        for plugin in plugins {
            copy(&plugin.platform_path, flutter.join(&plugin.name), true)?;
        }

        let mut cmakelist: String = include_str!("res/windows/CMakeLists.txt").into();
        for plugin in plugins {
            writeln!(cmakelist, "add_subdirectory(\"flutter/{}\")", plugin.name).ok();
        }

        let cmakelist_path = plugins_dir.join("CMakeLists.txt");
        std::fs::write(&cmakelist_path, &cmakelist)
            .wrap_error(FileOperation::Write, || cmakelist_path)?;

        let mut cmake = Config::new(plugins_dir);
        if !skip_build {
            cmake.no_build_target(true).build();
        }

        let configuration = cmake.get_profile();

        let artifacts_dir = get_artifacts_dir()?;

        for plugin in plugins {
            let plugin_artifacts_path = self
                .build
                .out_dir
                .join("build")
                .join("flutter")
                .join(&plugin.name)
                .join(configuration);

            for entry in fs::read_dir(&plugin_artifacts_path)
                .wrap_error(FileOperation::ReadDir, || plugin_artifacts_path.clone())?
            {
                let entry =
                    entry.wrap_error(FileOperation::ReadDir, || plugin_artifacts_path.clone())?;

                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(".dll") || name.ends_with(".lib") {
                    // false positive :-/
                    #[allow(clippy::needless_borrow)]
                    copy_to(entry.path(), &artifacts_dir, true)?;
                }
                if let Some(name) = name.strip_suffix(".dll") {
                    cargo_emit::rustc_link_lib! {
                        name
                    };
                }
            }
        }

        self.write_plugin_registrar(plugins)?;

        Ok(())
    }

    fn write_plugin_registrar(&self, plugins: &[Plugin]) -> BuildResult<()> {
        let path = self.build.out_dir.join("generated_plugins_registrar.rs");
        self._write_plugin_registrar(&path, plugins)
            .wrap_error(FileOperation::Write, || path)
    }

    fn _write_plugin_registrar(&self, path: &Path, plugins: &[Plugin]) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        writeln!(
            file,
            "fn flutter_get_plugins() -> Vec<nativeshell::shell::platform::engine::PlatformPlugin> {{"
        )?;

        for plugin in plugins {
            writeln!(file, "  extern \"C\" {{ pub fn {}RegisterWithRegistrar(registrar: *mut std::os::raw::c_void); }}", plugin.platform_info.plugin_class)?;
        }

        writeln!(file, "  vec![")?;
        for plugin in plugins {
            let class = &plugin.platform_info.plugin_class;
            writeln!(
                file,
                "    nativeshell::shell::platform::engine::PlatformPlugin {{ \
                name: \"{class}\".into(), register_func: Some({class}RegisterWithRegistrar) }},",
            )?;
        }

        writeln!(file, "  ]")?;
        writeln!(file, "}}")?;

        Ok(())
    }
}
