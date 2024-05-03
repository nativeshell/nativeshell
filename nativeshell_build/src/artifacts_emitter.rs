use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    util::{copy, copy_to, mkdir},
    BuildError, BuildResult, FileOperation, Flutter, IOResultExt, TargetOS,
};

pub(super) struct ArtifactsEmitter<'a> {
    build: &'a Flutter<'a>,
    flutter_out_dir: PathBuf,
    flutter_build_dir: PathBuf,
    artifacts_out_dir: PathBuf,
    is_local_engine: bool,
}

impl<'a> ArtifactsEmitter<'a> {
    pub fn new<P: AsRef<Path>>(
        build: &'a Flutter,
        flutter_out_dir: P,
        artifacts_out_dir: P,
        is_local_engine: bool,
    ) -> Result<Self, BuildError> {
        Ok(Self {
            build,
            flutter_out_dir: flutter_out_dir.as_ref().into(),
            flutter_build_dir: Self::find_flutter_build_dir(flutter_out_dir)?,
            artifacts_out_dir: artifacts_out_dir.as_ref().into(),
            is_local_engine,
        })
    }

    // assemble data folder
    pub fn emit_flutter_data(&self) -> BuildResult<()> {
        let data_dir = self.artifacts_out_dir.join("data");
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir)
                .wrap_error(FileOperation::RemoveDir, || data_dir.clone())?;
        }
        let assets_dst_dir = mkdir(&data_dir, Some("flutter_assets"))?;
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
            .wrap_error(FileOperation::ReadDir, || assets_src_dir.clone())?
        {
            let entry = entry.wrap_error(FileOperation::Read, || assets_src_dir.clone())?;
            copy_to(entry.path(), &assets_dst_dir, false)?
        }

        if self.build.build_mode == "debug" {
            let app_dill = self.flutter_build_dir.join("app.dill");
            if app_dill.exists() {
                copy(
                    self.flutter_build_dir.join("app.dill"),
                    assets_dst_dir.join("kernel_blob.bin"),
                    false,
                )?;
            }
        }

        // windows AOT
        if cfg!(target_os = "windows") {
            let app_so = self.flutter_out_dir.join("windows").join("app.so");
            if app_so.exists() {
                copy_to(&app_so, &data_dir, false)?;
            }
        }

        // linux AOT
        if cfg!(target_os = "linux") {
            // on linux the lib path is hardcoded, but all libraries go to "lib"
            // (RUNPATH being $ORIGIN/lib)
            let app_so = self.flutter_out_dir.join("lib").join("libapp.so");
            if app_so.exists() {
                let lib_dir = mkdir(&self.artifacts_out_dir, Some("lib"))?;
                copy_to(&app_so, lib_dir, false)?;
            }
        }

        Ok(())
    }

    // MacOS only
    pub fn emit_app_framework(&self) -> BuildResult<()> {
        copy_to(
            self.flutter_out_dir.join("App.framework"),
            &self.artifacts_out_dir,
            true,
        )?;
        Ok(())
    }

    pub fn emit_native_assets(&self, target_os: &TargetOS) -> BuildResult<()> {
        let os_dir: &str = match target_os {
            TargetOS::Mac => "macos",
            TargetOS::Windows => "windows",
            TargetOS::Linux => "linux",
        };
        let source_dir = self
            .flutter_out_dir
            .join("build")
            .join("native_assets")
            .join(os_dir);
        if source_dir.exists() {
            let dest_dir = self.artifacts_out_dir.join("native_assets");
            copy(&source_dir, &dest_dir, true)?;

            let artifacts_out_dir = {
                if cfg!(target_os = "linux") {
                    // RUNPATH is set to $origin/lib
                    mkdir(&self.artifacts_out_dir, Some("lib"))?
                } else {
                    self.artifacts_out_dir.clone()
                }
            };

            // copy individual files / frameworks
            for entry in
                fs::read_dir(&dest_dir).wrap_error(FileOperation::ReadDir, || dest_dir.clone())?
            {
                let entry = entry.wrap_error(FileOperation::Read, || dest_dir.clone())?;
                let entry_path = entry.path();
                copy_to(entry_path, &artifacts_out_dir, true)?;
            }
        }
        Ok(())
    }

    pub fn emit_external_libraries(&self) -> BuildResult<()> {
        let flutter_artifacts = self.find_artifacts_location(self.build.build_mode.as_str())?;

        let files = {
            if cfg!(target_os = "macos") {
                if !self.is_local_engine
                    && flutter_artifacts.join("FlutterMacOS.xcframework").exists()
                {
                    vec!["FlutterMacOS.xcframework/macos-arm64_x86_64/FlutterMacOS.framework"]
                } else {
                    vec!["FlutterMacOS.framework"]
                }
            } else if cfg!(target_os = "windows") {
                vec![
                    "flutter_windows.dll",
                    "flutter_windows.dll.lib",
                    "flutter_windows.dll.pdb",
                ]
            } else if cfg!(target_os = "linux") {
                vec!["libflutter_linux_gtk.so"]
            } else {
                panic!("Invalid target OS")
            }
        };

        let artifacts_out_dir = {
            if cfg!(target_os = "linux") {
                // RUNPATH is set to $origin/lib
                mkdir(&self.artifacts_out_dir, Some("lib"))?
            } else {
                self.artifacts_out_dir.clone()
            }
        };

        let deps_out_dir = self.artifacts_out_dir.join("deps");
        let flutter_artifacts_debug = self.find_artifacts_location("debug")?;
        for file in files {
            // on linux the unstripped libraries in local engine build are in
            // lib.unstripped folder; so if unstripped version exists we prefer that
            let unstripped = flutter_artifacts.join("lib.unstripped").join(file);
            let src = if unstripped.exists() {
                unstripped
            } else {
                flutter_artifacts.join(file)
            };
            if !src.exists() {
                return Err(BuildError::OtherError(format!(
                    "File {src:?} does not exist. Try running 'flutter precache'."
                )));
            }
            // false positives :-/
            #[allow(clippy::needless_borrow)]
            copy_to(&src, &artifacts_out_dir, true)?;
            #[allow(clippy::needless_borrow)]
            copy_to(&src, &deps_out_dir, true)?;
        }

        let data_dir = self.artifacts_out_dir.join("data");
        let icu = flutter_artifacts_debug.join("icudtl.dat");
        if icu.exists() && data_dir.exists() {
            copy_to(icu, data_dir, false)?;
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
        } else if cfg!(target_os = "linux") {
            cargo_emit::rustc_link_search! {
                self.artifacts_out_dir.join("lib").to_string_lossy(),
            };
        }

        Ok(())
    }

    fn find_flutter_build_dir<P: AsRef<Path>>(flutter_out_dir: P) -> Result<PathBuf, BuildError> {
        let last_build_id = flutter_out_dir.as_ref().join(".last_build_id");
        let last_build_id = fs::read_to_string(&last_build_id)
            .wrap_error(crate::FileOperation::Read, || last_build_id)?;

        let res = flutter_out_dir
            .as_ref()
            .join(".dart_tool")
            .join("flutter_build")
            .join(last_build_id);

        if !res.exists() {
            Err(BuildError::OtherError(format!(
                "Flutter build directory at {res:?} does not exist"
            )))
        } else {
            Ok(res)
        }
    }

    fn find_flutter_bundled_artifacts_location(&self) -> BuildResult<PathBuf> {
        Ok(self
            .build
            .options
            .find_flutter_bin()?
            .join("cache")
            .join("artifacts")
            .join("engine"))
    }

    pub(super) fn find_artifacts_location(&self, build_mode: &str) -> BuildResult<PathBuf> {
        let path = match self.build.options.local_engine.as_ref() {
            Some(local_engine) => {
                let engine_src_path = self.build.options.local_engine_src_path()?;
                engine_src_path.join("out").join(local_engine)
            }
            None => {
                let platform = match self.build.target_platform.as_str() {
                    "darwin" => {
                        let darwin_os = match self.build.darwin_arch.as_ref().unwrap().as_str() {
                            "x86_64" => "x64",
                            "arm64" => "x64", // Flutter now ships with universal framework, but in x64 folder
                            other => other,
                        };
                        format!("{}-{}", self.build.target_platform, darwin_os)
                    }
                    other => other.into(),
                };

                let engine = match (&platform, build_mode) {
                    (platform, "debug") => platform.into(),
                    (platform, mode) => format!("{platform}-{mode}"),
                };
                self.find_flutter_bundled_artifacts_location()?.join(engine)
            }
        };

        if !path.exists() {
            Err(BuildError::OtherError(format!(
                "Flutter artifact location path at {path:?} does not exist! Try running 'flutter precache'"
            )))
        } else {
            Ok(path)
        }
    }
}
