use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{BuildError, BuildResult, FileOperation, FlutterBuild, IOResultExt};

pub(super) struct ArtifactEmitter<'a> {
    build: &'a FlutterBuild,
    flutter_out_dir: PathBuf,
    flutter_build_dir: PathBuf,
    artifacts_out_dir: PathBuf,
}

impl<'a> ArtifactEmitter<'a> {
    pub fn new<P: AsRef<Path>>(
        build: &'a FlutterBuild,
        flutter_out_dir: P,
        artifacts_out_dir: P,
    ) -> Result<Self, BuildError> {
        Ok(Self {
            build,
            flutter_out_dir: flutter_out_dir.as_ref().into(),
            flutter_build_dir: Self::find_flutter_build_dir(flutter_out_dir)?,
            artifacts_out_dir: artifacts_out_dir.as_ref().into(),
        })
    }

    // assemble data folder
    pub fn emit_flutter_data(&self) -> BuildResult<()> {
        let data_dir = self.artifacts_out_dir.join("data");
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir).expect(&format!("Failed to remove {:?}", data_dir));
        }
        let assets_dst_dir = Self::mkdir(&data_dir, Some("flutter_assets"))?;
        let assets_src_dir = {
            let in_build = self.flutter_build_dir.join("flutter_assets");
            // kernel_snapshot/copy_assets - the flutter_assets folder is still inside build dir
            if in_build.exists() {
                in_build
            } else {
                // *_bundle_*_assets - moves the flutter_assets folder out
                self.flutter_out_dir.join("flutter_assets")
            }
        };
        for entry in fs::read_dir(&assets_src_dir)
            .wrap_error(FileOperation::ReadDir, assets_src_dir.clone())?
        {
            let entry = entry.wrap_error(FileOperation::Read, assets_src_dir.clone())?;
            Self::copy_to(entry.path(), &assets_dst_dir, false)?
        }

        let app_dill = self.flutter_build_dir.join("app.dill");
        if app_dill.exists() {
            Self::copy(
                self.flutter_build_dir.join("app.dill"),
                assets_dst_dir.join("kernel_blob.bin"),
                false,
            )?;
        }

        // windows AOT
        if cfg!(target_os = "windows") {
            let app_so = self.flutter_out_dir.join("windows").join("app.so");
            if app_so.exists() {
                Self::copy_to(&app_so, &data_dir, false)?;
            }
        }

        Ok(())
    }

    // MacOS only
    pub fn emit_app_framework(&self) -> BuildResult<()> {
        Self::copy_to(
            &self.flutter_out_dir.join("App.framework"),
            &self.artifacts_out_dir,
            true,
        )?;
        Ok(())
    }

    pub fn emit_external_libraries(&self) -> BuildResult<()> {
        let files = {
            if cfg!(target_os = "macos") {
                vec!["FlutterMacOS.framework"]
            } else if cfg!(target_os = "windows") {
                vec![
                    "flutter_windows.dll",
                    "flutter_windows.dll.lib",
                    "flutter_windows.dll.pdb",
                ]
            } else {
                panic!("Invalid target OS")
            }
        };
        let deps_out_dir = self.artifacts_out_dir.join("deps");
        let flutter_artifacts = self.find_artifacts_location(self.build.build_mode.as_str())?;
        let flutter_artifacts_debug = self.find_artifacts_location("debug")?;
        for file in files {
            let src = flutter_artifacts.join(file);
            if !src.exists() {
                return Err(BuildError::OtherError(format!(
                    "File {:?} does not exist. Try running 'flutter precache'.",
                    src
                )));
            }
            Self::copy_to(&src, &self.artifacts_out_dir, true)?;
            Self::copy_to(&src, &deps_out_dir, true)?;
        }

        let data_dir = self.artifacts_out_dir.join("data");
        let icu = flutter_artifacts_debug.join("icudtl.dat");
        if icu.exists() && data_dir.exists() {
            Self::copy_to(icu, data_dir, false)?;
        }
        Ok(())
    }

    pub fn emit_linker_flags(&self) -> BuildResult<()> {
        if cfg!(target_os = "macos") {
            cargo_emit::rustc_link_search! {
                self.artifacts_out_dir.to_string_lossy() => "framework",
            };
            cargo_emit::rustc_link_lib! {
                "FlutterMacOS" => "framework",
                "App" => "framework",
            };
        } else if cfg!(target_os = "windows") {
            cargo_emit::rustc_link_lib! {
                "flutter_windows.dll",
            };
            cargo_emit::rustc_link_search! {
                self.artifacts_out_dir.to_string_lossy(),
            };
        }

        Ok(())
    }

    fn find_flutter_build_dir<P: AsRef<Path>>(flutter_out_dir: P) -> Result<PathBuf, BuildError> {
        let last_build_id = flutter_out_dir.as_ref().join(".last_build_id");
        let last_build_id = fs::read_to_string(&last_build_id)
            .wrap_error(crate::FileOperation::Read, last_build_id)?;

        let res = flutter_out_dir
            .as_ref()
            .join(".dart_tool")
            .join("flutter_build")
            .join(last_build_id);

        if !res.exists() {
            Err(BuildError::OtherError(format!(
                "Flutter build directory at {:?} does not exist",
                res
            )))
        } else {
            Ok(res)
        }
    }

    fn copy<P, Q>(src: P, dst: Q, allow_symlinks: bool) -> BuildResult<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        if dst.as_ref().exists() {
            fs::remove_file(dst.as_ref()).wrap_error(FileOperation::Remove, dst.as_ref().into())?;
        }
        Self::copy_item(src, dst, allow_symlinks)
    }

    fn copy_to<P, Q>(src: P, dst: Q, allow_symlinks: bool) -> BuildResult<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let file_name = src.as_ref().file_name().unwrap();
        Self::copy(&src, dst.as_ref().join(file_name), allow_symlinks)
    }

    fn copy_item<P, Q>(src: P, dst: Q, allow_symlinks: bool) -> BuildResult<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let src_meta = fs::metadata(src.as_ref())
            .wrap_error(crate::FileOperation::MetaData, src.as_ref().into())?;

        if !allow_symlinks {
            if src_meta.is_dir() {
                copy_dir::copy_dir(&src, &dst).wrap_error_with_src(
                    FileOperation::CopyDir,
                    dst.as_ref().into(),
                    src.as_ref().into(),
                )?;
            } else {
                fs::copy(&src, &dst).wrap_error_with_src(
                    FileOperation::Copy,
                    dst.as_ref().into(),
                    src.as_ref().into(),
                )?;
            }
        } else {
            #[cfg(target_os = "windows")]
            {
                let res = if src_meta.is_dir() {
                    std::os::windows::fs::symlink_dir(&src, &dst)
                } else {
                    std::os::windows::fs::symlink_file(&src, &dst)
                };
                res.wrap_error_with_src(
                    FileOperation::SymLink,
                    dst.as_ref().into(),
                    src.as_ref().into(),
                )?;
            }
            #[cfg(target_os = "macos")]
            {
                std::os::unix::fs::symlink(&src, &dst).wrap_error_with_src(
                    FileOperation::SymLink,
                    dst.as_ref().into(),
                    src.as_ref().into(),
                )?;
            }
        }

        Ok(())
    }

    fn mkdir<P, Q>(target_path: P, sub_path: Option<Q>) -> BuildResult<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let target = match sub_path {
            Some(sub_path) => target_path.as_ref().join(sub_path.as_ref()),
            None => target_path.as_ref().into(),
        };
        fs::create_dir_all(&target).wrap_error(FileOperation::MkDir, target.clone())?;
        Ok(target)
    }

    fn find_executable<P: AsRef<Path>>(exe_name: P) -> Option<PathBuf> {
        env::var_os("PATH").and_then(|paths| {
            env::split_paths(&paths)
                .filter_map(|dir| {
                    let full_path = dir.join(&exe_name);
                    if full_path.is_file() {
                        Some(full_path)
                    } else {
                        None
                    }
                })
                .next()
        })
    }

    fn find_flutter_bin() -> Option<PathBuf> {
        let executable = if cfg!(target_os = "windows") {
            "flutter.bat"
        } else {
            "flutter"
        };
        let exe_path = Self::find_executable(executable);
        exe_path.and_then(|p| p.parent().map(Path::to_owned))
    }

    fn find_flutter_bundled_artifacts_location() -> Option<PathBuf> {
        Self::find_flutter_bin().map(|p| p.join("cache").join("artifacts").join("engine"))
    }

    fn find_local_engine_src_path() -> Option<PathBuf> {
        Self::find_flutter_bin()
            .and_then(|p| {
                p.parent()
                    .map(Path::to_owned)
                    .and_then(|p| p.parent().map(Path::to_owned))
            })
            .map(|p| p.join("engine").join("src"))
    }

    fn find_artifacts_location(&self, build_mode: &str) -> BuildResult<PathBuf> {
        let path: Option<PathBuf> = match self.build.options.local_engine.as_ref() {
            Some(local_engine) => {
                let engine_src_path = self
                    .build
                    .options
                    .local_engine_src_path
                    .clone()
                    .or_else(Self::find_local_engine_src_path);
                engine_src_path.map(|p| p.join("out").join(local_engine))
            }
            None => {
                let engine = match (&self.build.target_platform, build_mode) {
                    (platform, "debug") => platform.into(),
                    (platform, mode) => format!("{}-{}", platform, mode),
                };
                Self::find_flutter_bundled_artifacts_location().map(|p| p.join(engine))
            }
        };

        let path = path.ok_or(BuildError::OtherError(
            "Coud not determine flutter artifacts location; Please make sure that flutter is in PATH".into()))?;

        if !path.exists() {
            Err(BuildError::OtherError(format!(
                "Flutter artifact location path at {:?} does not exist! Try running 'flutter precache'",
                path
            )))
        } else {
            Ok(path)
        }
    }
}
