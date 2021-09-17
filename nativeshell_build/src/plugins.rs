use std::{
    collections::HashMap,
    fs::{self},
    path::{Path, PathBuf},
};

use yaml_rust::YamlLoader;

use crate::{
    artifacts_emitter::ArtifactsEmitter, BuildError, BuildResult, FileOperation, Flutter,
    IOResultExt, TargetOS,
};

#[cfg(target_os = "macos")]
#[path = "plugins_macos.rs"]
mod plugins_impl;

#[cfg(target_os = "windows")]
#[path = "plugins_windows.rs"]
mod plugins_impl;

#[cfg(target_os = "linux")]
#[path = "plugins_linux.rs"]
mod plugins_impl;

#[derive(Debug)]
pub(crate) struct PluginPlatformInfo {
    pub plugin_class: String,
}

#[derive(Debug)]
#[allow(dead_code)] // on some platforms
pub(crate) struct Plugin {
    pub name: String,
    pub path: PathBuf,
    pub platform_path: PathBuf,
    pub platform_name: String,
    pub platform_info: PluginPlatformInfo,
}

pub(super) struct Plugins<'a> {
    build: &'a Flutter<'a>,
    artifacts_emitter: &'a ArtifactsEmitter<'a>, // need to get artifacts location
}

impl<'a> Plugins<'a> {
    pub fn new(build: &'a Flutter, artifacts_emitter: &'a ArtifactsEmitter) -> Self {
        Self {
            build,
            artifacts_emitter,
        }
    }

    pub fn process(&self) -> BuildResult<()> {
        let plugins_path = self.build.root_dir.join(".flutter-plugins");
        let platform = plugins_impl::PluginsImpl::new(self.build, self.artifacts_emitter);
        let (plugins, plugins_file_content) = if plugins_path.exists() {
            let plugins_file_content = fs::read_to_string(&plugins_path)
                .wrap_error(crate::FileOperation::Read, || plugins_path.clone())?;
            (
                self.load_plugins(&plugins_file_content)?,
                plugins_file_content,
            )
        } else {
            (Vec::new(), String::new())
        };

        let skip_build =
            plugins.is_empty() || self.plugins_already_processed(&plugins_file_content)?;

        platform.process(&plugins, skip_build)?;
        self.mark_last_plugins(&plugins_file_content)?;

        Ok(())
    }

    fn plugins_already_processed(&self, plugins: &str) -> BuildResult<bool> {
        let last_plugins_path = self.build.out_dir.join(".flutter-plugins.last");
        if last_plugins_path.exists() {
            let content = fs::read_to_string(&last_plugins_path)
                .wrap_error(FileOperation::Read, || last_plugins_path.clone())?;
            return Ok(content == plugins);
        }
        Ok(false)
    }

    fn mark_last_plugins(&self, plugins: &str) -> BuildResult<()> {
        let last_plugins_path = self.build.out_dir.join(".flutter-plugins.last");
        fs::write(&last_plugins_path, plugins)
            .wrap_error(FileOperation::Write, || last_plugins_path)
    }

    fn load_plugin_info<P: AsRef<Path>>(
        &self,
        plugin_path: P,
    ) -> BuildResult<HashMap<String, PluginPlatformInfo>> {
        let mut res = HashMap::new();

        let path = plugin_path.as_ref().join("pubspec.yaml");
        let pub_spec = fs::read_to_string(&path).wrap_error(FileOperation::Read, || path)?;
        let pub_spec = YamlLoader::load_from_str(&pub_spec)
            .map_err(|err| BuildError::YamlError { source: err })?;
        let pub_spec = &pub_spec[0];
        let pub_spec = &pub_spec["flutter"];
        let pub_spec = &pub_spec["plugin"];
        let platforms = &pub_spec["platforms"];

        if let Some(platforms) = platforms.as_hash() {
            for platform in platforms {
                let plugin_class: Option<String> =
                    platform.1["pluginClass"].as_str().map(|s| s.into());
                if let Some(plugin_class) = plugin_class {
                    res.insert(
                        platform.0.as_str().unwrap().into(),
                        PluginPlatformInfo { plugin_class },
                    );
                }
            }
        }
        Ok(res)
    }

    fn load_plugins(&self, file: &str) -> BuildResult<Vec<Plugin>> {
        let lines = file.split('\n');
        let lines: Vec<(String, String)> = lines
            .filter_map(|line| {
                line.find('=')
                    .map(|sep| (line[..sep].into(), line[sep + 1..].into()))
            })
            .collect();
        let mut res = Vec::<Plugin>::new();
        for item in lines {
            let mut platform_info = self.load_plugin_info(&item.1)?;
            let platform_name = match self.build.target_os {
                TargetOS::Mac => "macos",
                TargetOS::Windows => "windows",
                TargetOS::Linux => "linux",
            };
            if let Some(platform_info) = platform_info.remove(platform_name) {
                let path: PathBuf = item.1.into();
                let platform_path = path.join(platform_name);

                // some plugins are FFI only, no need to build them
                if platform_path.exists() {
                    res.push(Plugin {
                        name: item.0,
                        path,
                        platform_path,
                        platform_name: platform_name.into(),
                        platform_info,
                    });
                }
            }
        }
        Ok(res)
    }
}
