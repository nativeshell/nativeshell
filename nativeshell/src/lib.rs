#![allow(clippy::new_without_default)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::type_complexity)]

pub mod codec;
pub mod shell;
pub mod util;

mod error;
pub use error::*;

pub use shell::{spawn, Context};
