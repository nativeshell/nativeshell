use nativeshell_build::{BuildResult, Flutter, FlutterOptions};

fn build_flutter() -> BuildResult<()> {
    Flutter::build(FlutterOptions {
        ..Default::default()
    })?;

    Ok(())
}

fn main() {
    if let Err(error) = build_flutter() {
        println!("\n** Build failed with error **\n\n{}", error);
        panic!();
    }
}
