#![allow(clippy::let_unit_value)]
#![allow(clippy::new_without_default)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::type_complexity)]
#![allow(clippy::await_holding_refcell_ref)]

pub mod codec;
pub mod shell;
pub mod util;

mod error;
pub use error::*;

pub use shell::{spawn, Context};
