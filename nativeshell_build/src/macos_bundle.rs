use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    error::IOResultExt,
    util::{get_absolute_path, get_artifacts_dir, mkdir, symlink},
    BuildResult, FileOperation, Flutter, Resources,
};

#[derive(Debug, Clone)]
pub struct AppBundleOptions {
    pub bundle_identifier: String,
    pub bundle_name: String,
    pub bundle_display_name: String,
    pub bundle_version: String,
    pub bundle_short_version_string: String,
    pub minimum_system_version: String,
    pub executable_path: PathBuf, // relative path to executable
    pub icon_file: PathBuf,       // Relative to bundle Resources dir
    pub info_plist_template: Option<PathBuf>, // path relative to CARGO_MANIFEST_DIR
    pub info_plist_additional_args: HashMap<String, String>,
}

pub struct AppBundleResult {}

impl Default for AppBundleOptions {
    fn default() -> Self {
        Self {
            bundle_identifier: "dev.nativeshell.example".into(),
            bundle_name: format!("{}.app", std::env::var("CARGO_PKG_NAME").unwrap()),
            bundle_display_name: std::env::var("CARGO_PKG_NAME").unwrap(),
            bundle_version: std::env::var("CARGO_PKG_VERSION").unwrap(),
            bundle_short_version_string: std::env::var("CARGO_PKG_VERSION").unwrap(),
            minimum_system_version: Flutter::macosx_deployment_target(),
            executable_path: std::env::var("CARGO_PKG_NAME").unwrap().into(),
            icon_file: "App.icns".into(),
            info_plist_template: None,
            info_plist_additional_args: HashMap::new(),
        }
    }
}

pub struct MacOSBundle {
    options: AppBundleOptions,
}

impl MacOSBundle {
    pub fn build(options: AppBundleOptions) -> BuildResult<Resources> {
        let bundle = MacOSBundle::new(options);
        bundle.do_build()
    }

    fn new(options: AppBundleOptions) -> Self {
        MacOSBundle { options }
    }

    fn do_build(&self) -> BuildResult<Resources> {
        let artifacts_dir = get_artifacts_dir()?;

        let bundle_path = artifacts_dir.join(&self.options.bundle_name);

        if bundle_path.exists() {
            std::fs::remove_dir_all(&bundle_path)
                .wrap_error(FileOperation::RemoveDir, || bundle_path.clone())?;
        }

        mkdir::<_, PathBuf>(&bundle_path, None)?;
        let contents = mkdir(&bundle_path, Some("Contents"))?;
        let macos = mkdir(&contents, Some("MacOS"))?;
        let bundle_executable_path = macos.join(self.options.executable_path.file_name().unwrap());

        symlink(
            artifacts_dir.join(&self.options.executable_path),
            bundle_executable_path,
        )?;

        self.write_info_plist(&contents)?;

        let resources_dir = mkdir(&contents, Some("Resources"))?;

        Ok(Resources { resources_dir })
    }

    fn write_info_plist<P: AsRef<Path>>(&self, contents: P) -> BuildResult<()> {
        let mut template = self.get_info_plist_template()?;
        Self::replace_plist_value(
            &mut template,
            "BUNDLE_IDENTIFIER",
            &self.options.bundle_identifier,
        );
        Self::replace_plist_value(
            &mut template,
            "BUNDLE_EXECUTABLE",
            &self
                .options
                .executable_path
                .file_name()
                .unwrap()
                .to_string_lossy(),
        );
        Self::replace_plist_value(
            &mut template,
            "ICON_FILE",
            &self.options.icon_file.to_string_lossy(),
        );
        Self::replace_plist_value(
            &mut template,
            "BUNDLE_NAME",
            &self.options.bundle_display_name,
        );
        Self::replace_plist_value(
            &mut template,
            "MINIMUM_SYSTEM_VERSION",
            &self.options.minimum_system_version,
        );
        Self::replace_plist_value(
            &mut template,
            "BUNDLE_VERSION",
            &self.options.bundle_version,
        );
        Self::replace_plist_value(
            &mut template,
            "BUNDLE_SHORT_VERSION_STRING",
            &self.options.bundle_short_version_string,
        );

        for (key, value) in &self.options.info_plist_additional_args {
            Self::replace_plist_value(&mut template, key, value);
        }

        let plist = contents.as_ref().join("Info.plist");
        std::fs::write(&plist, template).wrap_error(FileOperation::Write, || plist)?;

        Ok(())
    }

    fn replace_plist_value(plist: &mut String, key: &str, value: &str) {
        *plist = plist.replace(&format!("${{{key}}}"), value);
    }

    fn get_info_plist_template(&self) -> BuildResult<String> {
        match &self.options.info_plist_template {
            Some(path) => {
                let path = get_absolute_path(path);
                let content = std::fs::read_to_string(&path);
                content.wrap_error(FileOperation::Read, || path)
            }
            None => Ok(include_str!("res/macos/Info.plist").into()),
        }
    }
}
