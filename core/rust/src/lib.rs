#![allow(clippy::new_without_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::module_inception)]

mod context;
mod ffi;
mod handle;
mod message_channel;
mod native_pointer;
mod platform;
mod run_loop;
mod util;
mod value;

pub use context::*;
pub use ffi::*;
pub use handle::*;
pub use message_channel::*;
pub use native_pointer::*;
pub use run_loop::*;
pub use value::*;

#[cfg(feature = "nativeshell_derive")]
pub mod derive_internal;

#[cfg(feature = "nativeshell_derive")]
pub use nativeshell_derive::*;
