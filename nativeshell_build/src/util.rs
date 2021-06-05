use std::{
    fs,
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

    if !output.status.success() {
        Err(BuildError::ToolError {
            command: format!("{:?}", command),
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).into(),
            stdout: String::from_utf8_lossy(&output.stdout).into(),
        })
    } else {
        Ok(())
    }
}
