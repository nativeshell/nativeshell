use std::path::{Path, PathBuf};

use crate::{
    util::{get_absolute_path, get_artifacts_dir, mkdir, symlink},
    BuildResult,
};

pub struct Resources {
    pub(super) resources_dir: PathBuf,
}

impl Resources {
    pub fn new<P: AsRef<Path>>(folder_name: P) -> BuildResult<Self> {
        let dir = mkdir(get_artifacts_dir()?, Some(folder_name))?;
        Ok(Self { resources_dir: dir })
    }

    pub fn mkdir<P: AsRef<Path>>(&self, sub_path: P) -> BuildResult<()> {
        mkdir(&self.resources_dir, Some(sub_path))?;
        Ok(())
    }

    pub fn link<P, Q>(&self, src: P, dst: Q) -> BuildResult<()>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let src = get_absolute_path(src);
        let dst = self.resources_dir.join(dst);

        let dst = if dst.exists() {
            dst.join(src.file_name().unwrap())
        } else {
            dst
        };

        symlink(src, dst)?;

        Ok(())
    }
}
