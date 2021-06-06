use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    artifacts_emitter::ArtifactsEmitter,
    plugins::Plugin,
    util::{get_artifacts_dir, mkdir, run_command, symlink},
    BuildResult, FileOperation, Flutter, IOResultExt,
};

pub(super) struct PluginsImpl<'a> {
    build: &'a Flutter,
}

impl<'a> PluginsImpl<'a> {
    pub fn new(build: &'a Flutter, _artifacts_emitter: &'a ArtifactsEmitter<'a>) -> Self {
        Self { build }
    }

    pub fn process(&self, plugins: &[Plugin], skip_build: bool) -> BuildResult<()> {
        let xcode = mkdir(&self.build.out_dir, Some("xcode"))?;
        if !skip_build {
            self.write_dummy_xcode_project(&xcode)?;
            self.write_podfile(&xcode, plugins)?;
            self.install_cocoa_pods(&xcode)?;
        }
        let (frameworks_path, products_path) = self.build_pods(&xcode, skip_build)?;
        self.link_and_emit_frameworks(&frameworks_path)?;
        let classes = self.get_plugin_classes(plugins, &products_path)?;
        self.write_plugin_registrar(&classes)?;
        Ok(())
    }

    pub fn write_empty_registrar(&self) -> BuildResult<()> {
        self.write_plugin_registrar(&HashMap::new())
    }

    fn write_podfile(&self, path: &Path, plugins: &[Plugin]) -> BuildResult<()> {
        let mut file =
            File::create(path.join("PodFile")).wrap_error(FileOperation::Create, || path.into())?;

        write!(
            file,
            "ENV['COCOAPODS_DISABLE_STATS'] = 'true'\n\
            target 'DummyProject' do\n  use_frameworks!\n"
        )
        .wrap_error(FileOperation::Write, || path.into())?;

        for plugin in plugins {
            write!(
                file,
                "  pod '{}', :path => '{}', :binary => true\n",
                plugin.name,
                plugin.platform_path.to_string_lossy()
            )
            .wrap_error(FileOperation::Write, || path.into())?;
        }

        write!(file, "end\n").wrap_error(FileOperation::Write, || path.into())?;

        Ok(())
    }

    fn install_cocoa_pods(&self, path: &Path) -> BuildResult<()> {
        let mut command = Command::new("pod");
        command.arg("install").current_dir(path);
        run_command(command, "pod")
    }

    // Returns (path to frameworks, path to products)
    fn build_pods(&self, path: &Path, skip_build: bool) -> BuildResult<(PathBuf, PathBuf)> {
        let configuration = if self.build.build_mode == "debug" {
            "Debug"
        } else {
            "Release"
        };

        if !skip_build {
            let mut command = Command::new("xcrun");
            command
                .arg("xcodebuild")
                .arg("-verbose")
                .arg("-workspace")
                .arg("DummyProject.xcworkspace")
                .arg("-scheme")
                .arg("DummyProject")
                .arg("-destination")
                .arg("platform=macOS")
                .arg("CODE_SIGN_IDENTITY=")
                .arg("CODE_SIGNING_REQUIRED=NO")
                .arg("ONLY_ACTIVE_ARCH=NO")
                .arg(format!(
                    "ARCHS={}",
                    self.build.darwin_arch.as_ref().unwrap()
                ))
                .arg(format!(
                    "MACOSX_DEPLOYMENT_TARGET={}",
                    Flutter::macosx_deployment_target()
                ))
                .arg("-configuration")
                .arg(configuration)
                .arg(format!("SYMROOT={}", path.join("build").to_string_lossy()))
                .current_dir(path);
            run_command(command, "xcodebuild")?;
        }
        let products = path.join("build").join(configuration);
        let path = products
            .join("DummyProject.app")
            .join("Contents")
            .join("Frameworks");
        Ok((path, products))
    }

    fn link_and_emit_frameworks(&self, frameworks_path: &Path) -> BuildResult<()> {
        let artifacts_dir = get_artifacts_dir()?;
        cargo_emit::rustc_link_search! {
            artifacts_dir.to_string_lossy() => "framework",
        };
        for entry in fs::read_dir(frameworks_path)
            .wrap_error(FileOperation::ReadDir, || frameworks_path.into())?
        {
            let entry = entry.wrap_error(FileOperation::ReadDir, || frameworks_path.into())?;

            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if file_name == "FlutterMacOS.framework" {
                continue;
            }
            let dst = artifacts_dir.join(entry.file_name());
            if dst.exists() {
                fs::remove_file(&dst).wrap_error(FileOperation::Remove, || (&dst).into())?;
            }
            symlink(entry.path(), &dst)?;

            if let Some(framework_name) = file_name.strip_suffix(".framework") {
                cargo_emit::rustc_link_lib! {
                    framework_name => "framework"
                };
            }
        }
        Ok(())
    }

    fn get_plugin_classes(
        &self,
        plugins: &[Plugin],
        product_path: &Path,
    ) -> BuildResult<HashMap<String, String>> {
        let mut res = HashMap::new();

        // Swift class names are mangled. Try to extract mangled name from <plugin_name>-Swift.h
        // objc compatibility header
        for plugin in plugins {
            let swift_header_path = product_path
                .join(&plugin.name)
                .join(format!("{}.framework", plugin.name))
                .join("Headers")
                .join(format!("{}-Swift.h", plugin.name));

            if swift_header_path.exists() {
                let file = File::open(&swift_header_path)
                    .wrap_error(FileOperation::Open, || swift_header_path.clone())?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let line =
                        line.wrap_error(FileOperation::Read, || swift_header_path.clone())?;
                    if let Some(line) = line.strip_prefix("SWIFT_CLASS(\"") {
                        if let Some(line) = line.strip_suffix("\")") {
                            // the of suffix of mangled name should match plugin class
                            if line.ends_with(&plugin.platform_info.plugin_class) {
                                res.insert(plugin.platform_info.plugin_class.clone(), line.into());
                                break;
                            }
                        }
                    }
                }
            } else {
                // possibly not a swift plugin
                res.insert(
                    plugin.platform_info.plugin_class.clone(),
                    plugin.platform_info.plugin_class.clone(),
                );
            }
        }

        Ok(res)
    }

    fn write_plugin_registrar(&self, classes: &HashMap<String, String>) -> BuildResult<()> {
        let path = self.build.out_dir.join("generated_plugins_registrar.rs");
        self._write_plugin_registrar(&path, classes)
            .wrap_error(FileOperation::Write, || path)
    }

    fn _write_plugin_registrar(
        &self,
        path: &Path,
        classes: &HashMap<String, String>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(
            file,
            "fn flutter_get_plugins() -> Vec<nativeshell::shell::platform::engine::PlatformPlugin> {{"
        )?;
        writeln!(file, "  vec![")?;
        for class in classes {
            writeln!(
                file,
                "    nativeshell::shell::platform::engine::PlatformPlugin {{ \
                name: \"{}\".into(), class:\"{}\".into() }},",
                class.0, class.1
            )?;
        }
        writeln!(file, "  ]")?;
        writeln!(file, "}}")?;
        Ok(())
    }

    fn write_dummy_xcode_project(&self, path: &Path) -> BuildResult<()> {
        let project = include_bytes!("DummyProject.tar");
        use tar::Archive;
        let mut archive = Archive::new(project as &[u8]);
        archive
            .unpack(&path)
            .wrap_error(FileOperation::Unarchive, || path.into())?;
        Ok(())
    }
}
