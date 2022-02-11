use nativeshell_build::{BuildResult, Flutter, FlutterOptions};

fn build_flutter() -> BuildResult<()> {
    Flutter::build(FlutterOptions {
        ..Default::default()
    })?;

    Ok(())
}

fn main() {
    // We would normally check for RA_RUSTC_WRAPPER, but it seems that it's
    // not always set?
    if std::env::var("VSCODE_PID").is_ok() {
        // Do not build flutter when running under rust analyzer
        return;
    }
    if let Err(error) = build_flutter() {
        println!("\n** Build failed with error **\n\n{}", error);
        for e in std::env::vars() {
            println!("{} - {}", e.0, e.1);
        }
        panic!();
    }
}
