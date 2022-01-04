use cocoa::{base::id, foundation::NSBundle};
use fs::canonicalize;
use log::warn;
use objc::{msg_send, sel, sel_impl};
use process_path::get_executable_path;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn is_running_bundled() -> bool {
    unsafe {
        let bundle: id = NSBundle::mainBundle();
        let identifier: id = msg_send![bundle, bundleIdentifier];
        !identifier.is_null()
    }
}

// Find bundle next to our executable that has a symlink to our executable
fn find_bundle_executable<P: AsRef<Path>>(executable_path: P) -> io::Result<Option<PathBuf>> {
    let parent = executable_path.as_ref().parent().unwrap();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;

        if entry.file_name().to_string_lossy().ends_with(".app") {
            let executables = entry.path().join("Contents/MacOS");
            for entry in fs::read_dir(executables)? {
                let entry = entry?;
                let meta = fs::symlink_metadata(entry.path())?;
                if meta.file_type().is_symlink() {
                    let resolved = canonicalize(entry.path())?;
                    if resolved == executable_path.as_ref() {
                        return Ok(Some(entry.path()));
                    }
                }
            }
        }
    }
    Ok(None)
}

pub(crate) fn macos_exec_bundle() {
    if is_running_bundled() {
        return;
    }

    let path = get_executable_path();
    if let Some(path) = path {
        let bundle_executable = find_bundle_executable(path);
        match bundle_executable {
            Ok(Some(bundle_executable)) => {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let err = exec::Command::new(bundle_executable).args(&args).exec();
                warn!("Exec failed with: {:?}", err);
            }
            Ok(None) => {}
            Err(error) => {
                warn!("Could not find bundle: {:?}", error);
            }
        }
    } else {
        warn!("Could not determine process executable path");
    }
}
