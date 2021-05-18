use file_open_dialog::FileOpenDialogService;
use nativeshell::{codec::Value, shell::{Context, ContextOptions, exec_bundle, register_observatory_listener}};

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod file_open_dialog;

fn main() {
    exec_bundle();
    register_observatory_listener();

    env_logger::builder().format_timestamp(None).init();

    let context = Context::new(ContextOptions {
        app_namespace: "NativeshellDemo".into(),
        ..Default::default()
    });

    let context = context.unwrap();

    let _file_open_dialog = FileOpenDialogService::new(context.clone());

    context
        .window_manager
        .borrow_mut()
        .create_window(Value::Null, None);

    context.run_loop.borrow().run();
}
