use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use dunce::simplified;
use path_slash::PathExt;
use yaml_rust::{Yaml, YamlLoader};

use crate::{
    artifacts_emitter::ArtifactsEmitter,
    error::BuildError,
    plugins::Plugins,
    util::{copy_to, find_executable, get_artifacts_dir, run_command},
    BuildResult, FileOperation, IOResultExt,
};

// User configurable options during flutter build
#[derive(Debug)]
pub struct FlutterOptions<'a> {
    // Project root relative to current cargo manifest file
    pub project_root: Option<&'a Path>,

    // lib/main.dart by default (relative to project root)
    pub target_file: &'a Path,

    // Custom Flutter location. If not specified, NativeShell build will try to find
    // flutter executable in PATH and derive the location from there.
    pub flutter_path: Option<&'a Path>,

    // Name of local engine
    pub local_engine: Option<&'a str>,

    // Source path of local engine. If not specified, NativeShell will try to locate
    // it relative to flutter path.
    pub local_engine_src_path: Option<&'a Path>,

    // Additional key-value pairs that will be available as constants from the
    // String.fromEnvironment, bool.fromEnvironment, int.fromEnvironment, and
    // double.fromEnvironment constructors.
    pub dart_defines: &'a [&'a str],

    // macOS: Allow specifying extra pods to be built in addition to pods from
    // Flutter plugins. For example: macos_extra_pods: &["pod 'Sparkle'"],
    pub macos_extra_pods: &'a [&'a str],
}

impl Default for FlutterOptions<'_> {
    fn default() -> Self {
        Self {
            project_root: None,
            target_file: "lib/main.dart".as_path(),
            flutter_path: None,
            local_engine: None,
            local_engine_src_path: None,
            dart_defines: &[],
            macos_extra_pods: &[],
        }
    }
}

pub trait AsPath {
    fn as_path(&self) -> &Path;
}

impl AsPath for str {
    fn as_path(&self) -> &Path {
        Path::new(self)
    }
}

impl FlutterOptions<'_> {
    pub(super) fn find_flutter_executable(&self) -> BuildResult<PathBuf> {
        let executable = if cfg!(target_os = "windows") {
            "flutter.bat"
        } else {
            "flutter"
        };
        match &self.flutter_path {
            Some(path) => {
                let out_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();
                let path = out_dir.join(path);
                let executable = path.join("bin").join(executable);
                if executable.exists() {
                    Ok(executable)
                } else {
                    Err(BuildError::FlutterPathInvalidError { path })
                }
            }
            None => {
                // Try FLUTER_ROOT if available
                let flutter_root = std::env::var("FLUTTER_ROOT").ok();
                if let Some(flutter_root) = flutter_root {
                    let executable = Path::new(&flutter_root).join("bin").join(executable);
                    if executable.exists() {
                        return Ok(executable);
                    }
                }
                let executable =
                    find_executable(executable).ok_or(BuildError::FlutterNotFoundError)?;
                let executable = executable
                    .canonicalize()
                    .wrap_error(FileOperation::Canonicalize, || executable)?;
                let executable = simplified(&executable).into();
                Ok(executable)
            }
        }
    }

    pub(super) fn find_flutter_bin(&self) -> BuildResult<PathBuf> {
        let executable = self.find_flutter_executable()?;
        Ok(executable.parent().unwrap().into())
    }

    pub(super) fn local_engine_src_path(&self) -> BuildResult<PathBuf> {
        match &self.local_engine_src_path {
            Some(path) => Ok(path.into()),
            None => self.find_local_engine_src_path(),
        }
    }

    fn find_local_engine_src_path(&self) -> BuildResult<PathBuf> {
        let bin = self.find_flutter_bin()?;
        let path = bin
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("engine").join("src"));
        if let Some(path) = path {
            if path.exists() {
                return Ok(path);
            }
        }
        Err(BuildError::FlutterLocalEngineNotFound)
    }
}

#[derive(Debug)]
pub enum TargetOS {
    Mac,
    Windows,
    Linux,
}

#[derive(Debug)]
pub struct Flutter<'a> {
    pub(super) root_dir: PathBuf,
    pub(super) out_dir: PathBuf,
    pub(super) options: FlutterOptions<'a>,
    pub(super) build_mode: String,
    pub(super) target_os: TargetOS,
    pub(super) target_platform: String,
    pub(super) darwin_arch: Option<String>,
}

impl Flutter<'_> {
    pub fn build(options: FlutterOptions) -> BuildResult<()> {
        let build = Flutter::new(options);
        build.do_build()
    }

    fn new(options: FlutterOptions) -> Flutter {
        Flutter {
            root_dir: std::env::var("CARGO_MANIFEST_DIR")
                .unwrap()
                .as_path()
                .join(&options.project_root.unwrap_or_else(|| "".as_path())),
            out_dir: std::env::var("OUT_DIR").unwrap().into(),
            options,
            build_mode: Flutter::build_mode(),
            target_os: Flutter::target_os(),
            target_platform: Flutter::target_platform(),
            darwin_arch: Flutter::darwin_arch(),
        }
    }

    fn do_flutter_pub_get(&self) -> BuildResult<()> {
        let mut command = self.create_flutter_command()?;
        command.arg("pub").arg("get");
        self.run_flutter_command(command)
    }

    fn do_build(&self) -> BuildResult<()> {
        let flutter_out_root = self.out_dir.join("flutter");
        let flutter_out_dart_tool = flutter_out_root.join(".dart_tool");
        fs::create_dir_all(&flutter_out_dart_tool)
            .wrap_error(FileOperation::CreateDir, || flutter_out_dart_tool.clone())?;

        let package_config = self.root_dir.join(".dart_tool").join("package_config.json");
        let package_config_out = flutter_out_dart_tool.join("package_config.json");

        if !Path::exists(&package_config) {
            self.do_flutter_pub_get()?;
        }

        Self::copy(&package_config, &package_config_out)?;

        let mut local_roots = HashSet::<PathBuf>::new();

        self.update_package_config_paths(package_config, package_config_out, &mut local_roots)?;

        Self::copy(
            self.root_dir
                .join(".dart_tool")
                .join("package_config_subset"),
            flutter_out_root
                .join(".dart_tool")
                .join("package_config_subset"),
        )?;

        let assets = self.copy_pubspec_yaml(
            &self.root_dir.join("pubspec.yaml"),
            &flutter_out_root.join("pubspec.yaml"),
        )?;

        self.precache()?;
        self.run_flutter_assemble(&flutter_out_root)?;
        self.emit_flutter_artifacts(&flutter_out_root)?;
        self.emit_flutter_checks(&local_roots, &assets).unwrap();

        if Self::build_mode() == "profile" {
            cargo_emit::rustc_cfg!("flutter_profile");
        }

        Ok(())
    }

    fn link_asset(
        &self,
        from_dir: &Path,
        to_dir: &Path,
        asset: &str,
    ) -> BuildResult<Option<PathBuf>> {
        let mut segments = asset.split('/');
        if let Some(first) = segments.next() {
            if first != "packages" {
                let asset = from_dir.join(first);
                copy_to(&asset, to_dir, true)?;
                return Ok(Some(asset));
            }
        }
        Ok(None)
    }

    fn extract_assets(pub_spec: &str) -> BuildResult<Vec<String>> {
        let pub_spec = YamlLoader::load_from_str(pub_spec)
            .map_err(|err| BuildError::YamlError { source: err })?;

        let mut res = Vec::new();

        let flutter = &pub_spec[0];
        if let Yaml::Hash(hash) = flutter {
            let flutter = hash.get(&Yaml::String("flutter".into()));
            if let Some(Yaml::Hash(flutter)) = flutter {
                let assets = flutter.get(&Yaml::String("assets".into()));
                if let Some(Yaml::Array(assets)) = assets {
                    for asset in assets {
                        if let Yaml::String(str) = asset {
                            res.push(str.clone());
                        }
                    }
                }
                let fonts = flutter.get(&Yaml::String("fonts".into()));
                if let Some(Yaml::Array(fonts)) = fonts {
                    for font in fonts {
                        if let Yaml::Hash(font) = font {
                            let fonts = font.get(&Yaml::String("fonts".into()));
                            if let Some(Yaml::Array(fonts)) = fonts {
                                for font in fonts {
                                    if let Yaml::Hash(font) = font {
                                        let asset = font.get(&Yaml::String("asset".into()));
                                        if let Some(Yaml::String(str)) = asset {
                                            res.push(str.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(res)
    }

    // copy pub_spec.yaml, linking assets in the process; asset directories
    // need to be linked relative to pubspec.yaml
    // Returns asset directories
    fn copy_pubspec_yaml(&self, from: &Path, to: &Path) -> BuildResult<Vec<PathBuf>> {
        let pub_spec = fs::read_to_string(from).wrap_error(FileOperation::Read, || from.into())?;

        let from_dir = from.parent().unwrap();
        let to_dir = to.parent().unwrap();

        let assets: BuildResult<Vec<PathBuf>> = Self::extract_assets(&pub_spec)?
            .iter()
            .filter_map(|asset| {
                let res = self.link_asset(from_dir, to_dir, asset);
                match res {
                    Ok(None) => None,
                    Ok(Some(value)) => Some(Ok(value)),
                    Err(err) => Some(Err(err)),
                }
            })
            .collect();

        Self::copy(from, to)?;
        assets
    }

    pub fn build_mode() -> String {
        let mut build_mode: String = std::env::var("PROFILE").unwrap();
        let profile = std::env::var("FLUTTER_PROFILE").unwrap_or_else(|_| "false".into());
        let profile = profile == "true" || profile == "1";
        if profile && build_mode != "release" {
            panic!("Profile option (FLUTTER_PROFILE) must only be enabled for release builds")
        }
        if profile {
            build_mode = "profile".into();
        }
        build_mode
    }

    fn target_platform() -> String {
        let env_arch = std::env::var("CARGO_CFG_TARGET_ARCH");
        let arch = match env_arch.as_deref() {
            Ok("x86_64") => "x64",
            Ok("aarch64") => "arm64",
            _ => panic!("Unsupported target architecture {:?}", env_arch),
        };
        match Flutter::target_os() {
            TargetOS::Mac => "darwin".into(),
            TargetOS::Windows => format!("windows-{}", arch),
            TargetOS::Linux => format!("linux-{}", arch),
        }
    }

    fn darwin_arch() -> Option<String> {
        let env_arch = std::env::var("CARGO_CFG_TARGET_ARCH");
        match Flutter::target_os() {
            TargetOS::Mac => match env_arch.as_deref() {
                Ok("x86_64") => Some("x86_64".into()),
                Ok("aarch64") => Some("arm64".into()),
                _ => panic!("Unsupported target architecture {:?}", env_arch),
            },
            _ => None,
        }
    }

    pub(crate) fn macosx_deployment_target() -> String {
        match Flutter::target_os() {
            TargetOS::Mac => {
                // FIXME: This needs better default
                std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "10.13".into())
            }
            _ => {
                panic!("Deployment target can only be called on Mac")
            }
        }
    }

    fn target_os() -> TargetOS {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS");
        match target_os.as_deref() {
            Ok("macos") => TargetOS::Mac,
            Ok("windows") => TargetOS::Windows,
            Ok("linux") => TargetOS::Linux,
            _ => panic!("Unsupported target operating system {:?}", target_os),
        }
    }

    fn update_package_config_paths<PathRef: AsRef<Path>>(
        &self,
        original: PathRef,
        new: PathRef,
        local_roots: &mut HashSet<PathBuf>,
    ) -> BuildResult<()> {
        let string =
            fs::read_to_string(&new).wrap_error(FileOperation::Read, || new.as_ref().into())?;

        let mut package_config: PackageConfig =
            serde_json::from_str(&string).map_err(|e| BuildError::JsonError {
                text: Some(string),
                source: e,
            })?;

        for package in &mut package_config.packages {
            if package.root_uri.starts_with("..") {
                // relative path
                let absolute = original.as_ref().parent().unwrap().join(&package.root_uri);

                let absolute = absolute
                    .canonicalize()
                    .wrap_error(FileOperation::Canonicalize, || absolute)?;

                {
                    // joining posix path with windows path results in posix
                    // path used as last segment verbatim; this is a workaround
                    let mut local_root = absolute.clone();
                    local_root.extend(Path::new(&package.package_uri).iter());
                    local_roots.insert(local_root);
                }

                // remove unc from windows canonicalize
                let absolute = simplified(&absolute);
                let mut absolute = absolute.to_slash_lossy();
                if !absolute.starts_with('/') {
                    absolute = format!("/{}", absolute);
                }
                absolute = format!("file://{}", absolute);
                package.root_uri = absolute;
            }
        }

        let serialized =
            serde_json::to_string_pretty(&package_config).map_err(|e| BuildError::JsonError {
                text: None,
                source: e,
            })?;

        fs::write(&new, &serialized).wrap_error(FileOperation::Write, || new.as_ref().into())?;

        Ok(())
    }

    fn create_flutter_command(&self) -> BuildResult<Command> {
        let executable = self.options.find_flutter_executable()?;
        if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(executable);
            Ok(c)
        } else {
            Ok(Command::new(executable))
        }
    }

    fn run_flutter_command(&self, command: Command) -> BuildResult<()> {
        run_command(command, "flutter")
    }

    fn run_flutter_assemble<PathRef: AsRef<Path>>(&self, working_dir: PathRef) -> BuildResult<()> {
        let rebased = pathdiff::diff_paths(&self.root_dir, &working_dir).unwrap();

        let actions: Vec<String> = match (&self.target_os, self.build_mode.as_str()) {
            (TargetOS::Mac, _) => vec![format!("{}_macos_bundle_flutter_assets", self.build_mode)],

            // quicker, no need to copy flutter artifacts, we'll do it ourselves
            (TargetOS::Windows, "debug") => vec!["kernel_snapshot".into(), "copy_assets".into()],

            // the only published action that builds desktop aot is *_bundle_*_assets; it also
            // produces other artifacts (i.e. flutter dll), but we ignore those and handle
            // artifacts on our own
            (TargetOS::Windows, _) => vec![format!("{}_bundle_windows_assets", self.build_mode)],

            // quicker, no need to copy flutter artifacts, we'll do it ourselves
            (TargetOS::Linux, "debug") => vec!["kernel_snapshot".into(), "copy_assets".into()],

            // Similar to Windows above
            (TargetOS::Linux, _) => vec![format!(
                "{}_bundle_{}_assets",
                self.build_mode, self.target_platform
            )],
        };

        // flutter help assemble is lying about how defines are passed in. They
        // need to be base64 encoded, concatenated and passed through --DartDefines
        let defines: Vec<String> = self
            .options
            .dart_defines
            .iter()
            .map(base64::encode)
            .collect();
        let defines = format!("--DartDefines={}", defines.join(","));

        let mut command = self.create_flutter_command()?;
        command.current_dir(&working_dir);

        if let Some(local_engine) = &self.options.local_engine {
            command.arg(format!("--local-engine={}", local_engine));

            command.arg(format!(
                "--local-engine-src-path={}",
                self.options.local_engine_src_path()?.to_slash_lossy()
            ));
        }
        command
            .arg("assemble")
            .arg("--output=.")
            .arg(format!("--define=BuildMode={}", self.build_mode))
            .arg(format!("--define=TargetPlatform={}", self.target_platform))
            .arg(format!(
                "--define=DarwinArchs={}",
                self.darwin_arch.as_ref().unwrap_or(&String::default())
            ))
            .arg(format!(
                "--define=TargetFile={}",
                rebased.join(&self.options.target_file).to_str().unwrap()
            ))
            .arg(defines)
            .arg("-v")
            .arg("--suppress-analytics")
            .args(actions);

        self.run_flutter_command(command)
    }

    fn emit_flutter_artifacts<PathRef: AsRef<Path>>(
        &self,
        working_dir: PathRef,
    ) -> BuildResult<()> {
        let artifacts_dir = get_artifacts_dir()?;
        let flutter_out_root = self.out_dir.join("flutter");
        let emitter = ArtifactsEmitter::new(self, flutter_out_root, artifacts_dir)?;

        let plugins = Plugins::new(self, &emitter);
        plugins.process()?;

        match self.target_os {
            TargetOS::Mac => {
                cargo_emit::rustc_link_search! {
                    working_dir.as_ref().to_str().unwrap() => "framework",
                };
                emitter.emit_app_framework()?;
                emitter.emit_external_libraries()?;
                emitter.emit_linker_flags()?;
            }
            TargetOS::Windows => {
                emitter.emit_flutter_data()?;
                emitter.emit_external_libraries()?;
                emitter.emit_linker_flags()?;
            }
            TargetOS::Linux => {
                emitter.emit_flutter_data()?;
                emitter.emit_external_libraries()?;
                emitter.emit_linker_flags()?;
            }
        }
        Ok(())
    }

    fn emit_flutter_checks(&self, roots: &HashSet<PathBuf>, assets: &[PathBuf]) -> BuildResult<()> {
        cargo_emit::rerun_if_changed! {
            self.root_dir.join("pubspec.yaml").to_str().unwrap(),
            self.root_dir.join("pubspec.lock").to_str().unwrap(),
        };

        for path in roots {
            self.emit_checks_for_dir(path)?;
        }

        for asset in assets {
            if asset.is_dir() {
                self.emit_checks_for_dir(asset)?;
            } else {
                cargo_emit::rerun_if_changed! {
                    asset.to_string_lossy()
                }
            }
        }

        cargo_emit::rerun_if_env_changed!("FLUTTER_PROFILE");

        Ok(())
    }

    fn emit_checks_for_dir(&self, path: &Path) -> BuildResult<()> {
        for entry in fs::read_dir(path).wrap_error(FileOperation::ReadDir, || path.into())? {
            let entry = entry.wrap_error(FileOperation::ReadDir, || path.into())?;
            let metadata = entry
                .metadata()
                .wrap_error(FileOperation::MetaData, || entry.path())?;
            if metadata.is_dir() {
                self.emit_checks_for_dir(entry.path().as_path())?;
            } else {
                cargo_emit::rerun_if_changed! {
                    entry.path().to_string_lossy(),
                }
            }
        }
        Ok(())
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64, BuildError> {
        fs::copy(&from, &to).wrap_error_with_src(
            FileOperation::Copy,
            || to.as_ref().into(),
            || from.as_ref().into(),
        )
    }

    pub fn precache(&self) -> BuildResult<()> {
        // no precaching necessary for local engines
        if self.options.local_engine.is_some() {
            return Ok(());
        }
        let engine_version = self
            .options
            .find_flutter_bin()?
            .join("internal")
            .join("engine.version");

        let engine_version = fs::read_to_string(&engine_version)
            .wrap_error(FileOperation::Read, || engine_version)?;

        let last_engine_version_path = self.out_dir.join("last_precached_engine_version");
        if last_engine_version_path.exists() {
            let last_engine_version = fs::read_to_string(&last_engine_version_path)
                .wrap_error(FileOperation::Read, || last_engine_version_path.clone())?;
            if last_engine_version == engine_version {
                return Ok(());
            }
        }

        // need to run flutter precache
        let mut command = self.create_flutter_command()?;
        command
            .arg("precache")
            .arg("-v")
            .arg("--suppress-analytics")
            .arg(match self.target_os {
                TargetOS::Mac => "--macos",
                TargetOS::Windows => "--windows",
                TargetOS::Linux => "--linux",
            });
        self.run_flutter_command(command)?;

        fs::write(&last_engine_version_path, &engine_version)
            .wrap_error(FileOperation::Write, || last_engine_version_path)?;

        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Package {
    root_uri: String,
    package_uri: String,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PackageConfig {
    packages: Vec<Package>,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

#[test]
fn test_extract_assets() {
    let pub_spec = r#"
flutter:
  assets:
    - asset1
    - asset2
  fonts:
    - family: Raleway
      fonts:
        - asset: fonts/Raleway-Regular.ttf
        - asset: fonts/Raleway-Italic.ttf
          style: italic
    - family: RobotoMono
      fonts:
        - asset: fonts/RobotoMono-Regular.ttf
        - asset: fonts/RobotoMono-Bold.ttf
          weight: 700
    "#;
    let assets = Flutter::extract_assets(pub_spec).unwrap();
    let expected: Vec<String> = vec![
        "asset1".into(),
        "asset2".into(),
        "fonts/Raleway-Regular.ttf".into(),
        "fonts/Raleway-Italic.ttf".into(),
        "fonts/RobotoMono-Regular.ttf".into(),
        "fonts/RobotoMono-Bold.ttf".into(),
    ];
    assert_eq!(assets, expected);
}
