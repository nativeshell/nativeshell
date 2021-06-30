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
