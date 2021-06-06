#[macro_export]
macro_rules! include_flutter_plugins {
    () => {
        ::std::include!(::std::concat!(
            ::std::env!("OUT_DIR"),
            "/generated_plugins_registrar.rs"
        ));
    };
}
