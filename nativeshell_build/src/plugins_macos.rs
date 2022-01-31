use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    artifacts_emitter::ArtifactsEmitter,
    plugins::Plugin,
    util::{get_artifacts_dir, mkdir, run_command, symlink},
    BuildError, BuildResult, FileOperation, Flutter, IOResultExt,
};

pub(super) struct PluginsImpl<'a> {
    build: &'a Flutter<'a>,
}

impl<'a> PluginsImpl<'a> {
    pub fn new(build: &'a Flutter, _artifacts_emitter: &'a ArtifactsEmitter<'a>) -> Self {
        Self { build }
    }

    pub fn process(&self, plugins: &[Plugin], _skip_build: bool) -> BuildResult<()> {
        // Nothing to do here
        if plugins.is_empty() && self.build.options.macos_extra_pods.is_empty() {
            self.write_plugin_registrar(&HashMap::new())?;
            return Ok(());
        }

        let xcode = mkdir(&self.build.out_dir, Some("xcode"))?;
        let symlinks_dir = self.create_plugin_symlinks(&xcode, plugins)?;
        let framework_dir = mkdir(&xcode, Some("FlutterMacOS"))?;
        let podfile = xcode.join("PodFile");
        let build_ok = xcode.join("build_ok");
        let mut skip_build = self
            .write_podfile(&podfile, plugins, &symlinks_dir, &framework_dir)
            .wrap_error(FileOperation::Write, || podfile.clone())?;
        skip_build &= build_ok.exists();
        if !skip_build {
            if build_ok.exists() {
                fs::remove_file(&build_ok).wrap_error(FileOperation::Remove, || build_ok.clone())?
            }
            self.create_flutter_framework_podspec(&framework_dir)?;
            self.write_dummy_xcode_project(&xcode)?;
            self.install_cocoa_pods(&xcode)?;
        }
        let (frameworks_path, products_path) = self.build_pods(&xcode, skip_build)?;
        self.link_and_emit_frameworks(&frameworks_path)?;
        let classes = self.get_plugin_classes(plugins, &products_path)?;
        self.write_plugin_registrar(&classes)?;
        if !skip_build {
            fs::write(&build_ok, "").wrap_error(FileOperation::Write, || build_ok.clone())?;
        }
        Ok(())
    }

    fn create_plugin_symlinks(&self, path: &Path, plugins: &[Plugin]) -> BuildResult<PathBuf> {
        let symlinks_dir = mkdir(path, Some("symlinks"))?;
        for plugin in plugins {
            let dst = symlinks_dir.join(&plugin.name);
            if dst.exists() {
                fs::remove_file(&dst).wrap_error(FileOperation::Remove, || dst.clone())?;
            }
            symlink(&plugin.path, &dst)?
        }

        Ok(symlinks_dir)
    }

    fn create_flutter_framework_podspec(&self, folder: &Path) -> BuildResult<()> {
        let content = "Pod::Spec.new do |s|\n\
  s.name             = 'FlutterMacOS'\n\
  s.version          = '1.0.0'\n\
  s.summary          = 'High-performance, high-fidelity mobile apps.'\n\
  s.homepage         = 'https://flutter.io'\n\
  s.license          = { :type => 'MIT' }\n\
  s.author           = { 'Flutter Dev Team' => 'flutter-dev@googlegroups.com' }\n\
  s.source           = { :git => 'https://github.com/flutter/engine', :tag => s.version.to_s }\n\
  s.osx.deployment_target = '10.11'\n\
  s.vendored_frameworks = 'FlutterMacOS.framework'\n\
end\n";
        let podspec_file = folder.join("FlutterMacOS.podspec");
        fs::write(&podspec_file, content).wrap_error(FileOperation::Write, || podspec_file)?;
        let dst = folder.join("FlutterMacOS.framework");
        if dst.exists() {
            fs::remove_file(&dst).wrap_error(FileOperation::Remove, || dst.clone())?;
        }
        let src = folder.join("../../flutter/FlutterMacOS.framework");
        let src = src
            .canonicalize()
            .wrap_error(FileOperation::Canonicalize, || src.clone())?;
        symlink(&src, &dst)?;
        Ok(())
    }

    // return true if build can be skipped
    fn write_podfile(
        &self,
        file: &Path,
        plugins: &[Plugin],
        symlinks_dir: &Path,
        framework_dir: &Path,
    ) -> io::Result<bool> {
        let mut contents = String::new();
        use std::fmt::Write;
        write!(
            contents,
            "ENV['COCOAPODS_DISABLE_STATS'] = 'true'\n\
            platform :osx, '{}'\n\
            abstract_target 'NativeShellTarget' do\n  use_frameworks! :linkage=>:dynamic\n",
            Flutter::macosx_deployment_target()
        )
        .unwrap();

        writeln!(
            contents,
            "  pod 'FlutterMacOS', :path => '{}'",
            framework_dir.to_string_lossy()
        )
        .unwrap();

        for plugin in plugins {
            let plugin_path = symlinks_dir.join(&plugin.name);
            writeln!(
                contents,
                "  pod '{}', :path => '{}'",
                plugin.name,
                plugin_path.join(&plugin.platform_name).to_string_lossy()
            )
            .unwrap();
        }

        for pod in self.build.options.macos_extra_pods {
            writeln!(contents, "  {}", pod).unwrap();
        }

        writeln!(contents, "  target 'DummyProject' do").unwrap();
        writeln!(contents, "  end").unwrap();
        writeln!(contents, "  target 'NativeShellPods' do").unwrap();
        writeln!(contents, "  end").unwrap();
        writeln!(contents, "end").unwrap();

        if file.exists() && fs::read_to_string(&file)? == contents {
            return Ok(true);
        }

        fs::write(file, contents)?;

        Ok(false)
    }

    fn install_cocoa_pods(&self, path: &Path) -> BuildResult<()> {
        let mut command = Command::new("pod");
        command.arg("install").current_dir(path);
        let res = run_command(command, "pod");

        if let Err(BuildError::FileOperationError {
            operation: FileOperation::Command,
            path: _,
            source_path: _,
            source,
        }) = &res
        {
            if source.raw_os_error() == Some(2) {
                return Err(BuildError::OtherError(
                    "CocoaPods is not installed. \
                    Please install cocoa pods: https://cocoapods.org"
                        .into(),
                ));
            }
        }

        res
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
        use std::io::Write;
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
        let project = include_bytes!("res/macos/DummyProject.tar");
        use tar::Archive;
        let mut archive = Archive::new(project as &[u8]);
        archive
            .unpack(&path)
            .wrap_error(FileOperation::Unarchive, || path.into())?;
        Ok(())
    }
}
