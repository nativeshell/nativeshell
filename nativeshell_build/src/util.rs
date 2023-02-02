use std::{
    env,
    fs::{self, canonicalize},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{error::IOResultExt, BuildError, BuildResult, FileOperation};

pub(super) fn get_artifacts_dir() -> BuildResult<PathBuf> {
    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    let artifacts_dir = out_dir.join("../../../");
    let artifacts_dir = artifacts_dir
        .canonicalize()
        .wrap_error(FileOperation::Canonicalize, || artifacts_dir)?;
    Ok(artifacts_dir)
}

pub(super) fn get_absolute_path<P: AsRef<Path>>(path: P) -> PathBuf {
    if path.as_ref().is_absolute() {
        path.as_ref().into()
    } else {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let project_path: PathBuf = manifest.into();
        project_path.join(path)
    }
}

pub(super) fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> BuildResult<()> {
    #[cfg(target_family = "windows")]
    {
        let src_meta = fs::metadata(src.as_ref())
            .wrap_error(crate::FileOperation::MetaData, || src.as_ref().into())?;
        let res = if src_meta.is_dir() {
            std::os::windows::fs::symlink_dir(&src, &dst)
        } else {
            std::os::windows::fs::symlink_file(&src, &dst)
        };
        if let Err(error) = &res {
            if error.raw_os_error() == Some(1314) {
                return Err(BuildError::OtherError(
                    "Unable to create a symlink. Please enable developer mode:\n\
                    https://docs.microsoft.com/en-us/windows/apps/get-started/\
                    enable-your-device-for-development"
                        .into(),
                ));
            }
        }
        res.wrap_error_with_src(
            FileOperation::SymLink,
            || dst.as_ref().into(),
            || src.as_ref().into(),
        )?;
    }
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(&src, &dst).wrap_error_with_src(
            FileOperation::SymLink,
            || dst.as_ref().into(),
            || src.as_ref().into(),
        )?;
    }
    Ok(())
}

pub(super) fn mkdir<P, Q>(target_path: P, sub_path: Option<Q>) -> BuildResult<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let target = match sub_path {
        Some(sub_path) => target_path.as_ref().join(sub_path.as_ref()),
        None => target_path.as_ref().into(),
    };
    fs::create_dir_all(&target).wrap_error(FileOperation::MkDir, || target.clone())?;
    Ok(target)
}

pub(super) fn run_command(mut command: Command, command_name: &str) -> BuildResult<()> {
    let output = command
        .output()
        .wrap_error(FileOperation::Command, || command_name.into())?;

    #[allow(unused_mut)]
    let mut success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // on windows we run flutter.bat through cmd /c, which unfortunately swallows
    // the error code; So instead we check the output
    #[cfg(target_os = "windows")]
    {
        if command_name == "flutter" {
            success = stdout.trim_end().ends_with("exiting with code 0")
        }
    }
    if !success {
        Err(BuildError::ToolError {
            command: format!("{command:?}"),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into(),
            stdout: stdout.into(),
        })
    } else {
        Ok(())
    }
}

pub(super) fn copy<P, Q>(src: P, dst: Q, allow_symlinks: bool) -> BuildResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if dst.as_ref().exists() {
        if allow_symlinks {
            let src_can = canonicalize(src.as_ref())
                .wrap_error(FileOperation::Canonicalize, || src.as_ref().into())?;
            let dst_can = canonicalize(dst.as_ref())
                .wrap_error(FileOperation::Canonicalize, || dst.as_ref().into())?;
            if src_can == dst_can {
                // nothing to do here; This is useful on windows where often the symlink
                // can't be deleted and recreated because something is using it
                return Ok(());
            }
        }
        fs::remove_file(dst.as_ref()).wrap_error(FileOperation::Remove, || dst.as_ref().into())?;
    }
    let src_meta =
        fs::metadata(&src).wrap_error(crate::FileOperation::MetaData, || src.as_ref().into())?;

    if !allow_symlinks {
        if src_meta.is_dir() {
            copy_dir::copy_dir(&src, &dst).wrap_error_with_src(
                FileOperation::CopyDir,
                || dst.as_ref().into(),
                || src.as_ref().into(),
            )?;
        } else {
            fs::copy(&src, &dst).wrap_error_with_src(
                FileOperation::Copy,
                || dst.as_ref().into(),
                || src.as_ref().into(),
            )?;
        }
    } else {
        symlink(src, dst)?
    }

    Ok(())
}

pub(super) fn copy_to<P, Q>(src: P, dst: Q, allow_symlinks: bool) -> BuildResult<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let file_name = src.as_ref().file_name().unwrap();
    copy(&src, dst.as_ref().join(file_name), allow_symlinks)
}

pub(super) fn find_executable<P: AsRef<Path>>(exe_name: P) -> Option<PathBuf> {
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
