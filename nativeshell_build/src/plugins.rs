use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

use yaml_rust::YamlLoader;

use crate::{util::mkdir, BuildError, BuildResult, FileOperation, Flutter, IOResultExt, TargetOS};

#[derive(Debug)]
struct PluginPlatformInfo {
    plugin_class: Option<String>,
}

impl PluginPlatformInfo {
    fn is_valid(&self) -> bool {
        self.plugin_class.is_some()
    }
}

#[derive(Debug)]
struct Plugin {
    name: String,
    path: PathBuf,
    platform_info: PluginPlatformInfo,
}

pub(super) struct Plugins<'a> {
    build: &'a Flutter,
}

impl<'a> Plugins<'a> {
    pub fn new(build: &'a Flutter) -> Self {
        Self { build }
    }

    pub fn process(&self) -> BuildResult<()> {
        let plugins_path = self.build.root_dir.join(".flutter-plugins");
        if plugins_path.exists() {
            println!("{:?}", plugins_path);
            println!("P: ${:?}", self.build);
            let plugins = self.load_plugins(plugins_path);
            println!("Plugins ${:?}", plugins);

            let xcode = mkdir(&self.build.out_dir, Some("xcode"))?;
            self.write_dummy_xcode_project(&xcode)?;

            panic!("Done");
        }
        Ok(())
    }

    fn write_podfile<P: AsRef<Path>>(&self, path: P, plugins: &Plugins) -> BuildResult<()> {
        let mut file = File::create(path.as_ref().join("PodFile"))
            .wrap_error(FileOperation::Create, path.as_ref().into());
        Ok(())
    }

    fn write_dummy_xcode_project<P: AsRef<Path>>(&self, path: P) -> BuildResult<()> {
        let project = include_bytes!("DummyProject.tar");
        use tar::Archive;
        let mut archive = Archive::new(project as &[u8]);
        archive
            .unpack(&path)
            .wrap_error(FileOperation::Unarchive, path.as_ref().into())?;
        Ok(())
    }

    fn load_plugin_info<P: AsRef<Path>>(
        &self,
        plugin_path: P,
    ) -> BuildResult<HashMap<String, PluginPlatformInfo>> {
        let mut res = HashMap::new();

        let path = plugin_path.as_ref().join("pubspec.yaml");
        let pub_spec = fs::read_to_string(&path).wrap_error(FileOperation::Read, path)?;
        let pub_spec = YamlLoader::load_from_str(&pub_spec)
            .map_err(|err| BuildError::YamlError { source: err })?;
        let pub_spec = &pub_spec[0];
        let pub_spec = &pub_spec["flutter"];
        let pub_spec = &pub_spec["plugin"];
        let platforms = &pub_spec["platforms"];

        for platform in platforms.as_hash() {
            for p in platform {
                let info = PluginPlatformInfo {
                    plugin_class: p.1["pluginClass"].as_str().map(|s| s.into()),
                };
                if info.is_valid() {
                    res.insert(p.0.as_str().unwrap().into(), info);
                }
            }
        }
        Ok(res)
    }

    fn load_plugins<P: AsRef<Path>>(&self, path: P) -> BuildResult<Vec<Plugin>> {
        let file = fs::read_to_string(path.as_ref())
            .wrap_error(crate::FileOperation::Read, path.as_ref().into())?;
        let lines = file.split('\n');
        let lines: Vec<(String, String)> = lines
            .filter_map(|line| {
                if let Some(sep) = line.find('=') {
                    Some((line[..sep].into(), line[sep + 1..].into()))
                } else {
                    None
                }
            })
            .collect();
        let mut res = Vec::<Plugin>::new();
        for item in lines {
            let mut platform_info = self.load_plugin_info(&item.1)?;
            let key = match self.build.target_os {
                TargetOS::Mac => "macos",
                TargetOS::Windows => "windows",
                TargetOS::Linux => "linux",
            };
            if let Some(platform_info) = platform_info.remove(key) {
                if platform_info.is_valid() {
                    res.push(Plugin {
                        name: item.0,
                        path: item.1.into(),
                        platform_info: platform_info,
                    });
                }
            }
        }
        Ok(res)
    }
}
