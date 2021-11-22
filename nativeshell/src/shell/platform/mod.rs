pub use self::platform_impl::*;

// #[path = "null/mod.rs"]
// mod platform_impl;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform_impl;

#[cfg(target_os = "windows")]
#[path = "win32/mod.rs"]
mod platform_impl;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform_impl;

// Null implementation - include just to make sure that it compiles
#[allow(unused_imports, unused_variables, dead_code)]
#[path = "null/mod.rs"]
mod null;
