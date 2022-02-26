use nativeshell_build::Flutter;

#[path = "keyboard_map/gen_keyboard_map.rs"]
mod gen_keyboard_map;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        let files = ["src/shell/platform/macos/window_buttons.m"];
        let mut build = cc::Build::new();
        for file in files.iter() {
            build.file(file);
            cargo_emit::rerun_if_changed!(file);
        }
        build.flag("-fobjc-arc");
        build.compile("macos_extra");
    }

    cargo_emit::rerun_if_env_changed!("FLUTTER_PROFILE");
    if Flutter::build_mode() == "profile" {
        cargo_emit::rustc_cfg!("flutter_profile");
    }

    let target_system = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    gen_keyboard_map::generate_keyboard_map(&target_system).unwrap();

    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" => println!("cargo:rustc-link-lib=flutter_linux_gtk"),
        "macos" => {
            println!("cargo:rustc-link-lib=framework=FlutterMacOS");
            println!("cargo:rustc-link-arg=-rpath=@executable_path/../Frameworks");
        }
        "windows" => println!("cargo:rustc-link-lib=flutter_windows.dll"),
        os => panic!("unsupported os {}", os),
    }
}
