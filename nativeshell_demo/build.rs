use nativeshell_build::{AppBundleOptions, BuildResult, Flutter, FlutterOptions, MacOSBundle};

fn build_flutter() -> BuildResult<()> {
    Flutter::build(FlutterOptions {
        local_engine: match Flutter::build_mode().as_str() {
            "debug" => Some("host_debug".into()),
            "release" => Some("host_release".into()),
            _ => None,
        },
        local_engine_src_path: None,
        ..Default::default()
    })?;

    if cfg!(target_os = "macos") {
        let options = AppBundleOptions {
            bundle_name: "NativeShellDemo.app".into(),
            bundle_display_name: "NativeShell Demo".into(),
            icon_file: "icons/AppIcon.icns".into(),
            ..Default::default()
        };
        let resources = MacOSBundle::build(options)?;
        resources.mkdir("icons")?;
        resources.link("resources/mac_icon.icns", "icons/AppIcon.icns")?;
    }

    Ok(())
}

fn main() {
    if let Err(error) = build_flutter() {
        println!("Build failed with error:\n{}", error);
        panic!();
    }

    // Windows symbols used for file_open_dialog example
    #[cfg(target_os = "windows")]
    {
        windows::build!(
            Windows::Win32::System::SystemServices::{
                TRUE
            },
            Windows::Win32::UI::WindowsAndMessaging::{
                GetOpenFileNameW,
            }
        )
    }
}
