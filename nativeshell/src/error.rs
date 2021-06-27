use std::fmt::Display;

use crate::{codec::value::ValueError, shell::platform::error::PlatformError};

#[derive(Debug, Clone)]
pub enum Error {
    InvalidContext,
    InvalidEngineHandle,
    Platform(PlatformError),
    Value(ValueError),
    InvalidMenuHandle,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidContext => {
                write!(f, "Context was already destroyed")
            }
            Error::Platform(error) => Display::fmt(error, f),
            Error::InvalidEngineHandle => {
                write!(f, "Provided handle does not match any engine")
            }
            Error::Value(error) => Display::fmt(error, f),
            Error::InvalidMenuHandle => {
                write!(f, "Provided menu handle does not match any known menu")
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::error::Error for Error {}

impl From<PlatformError> for Error {
    fn from(src: PlatformError) -> Error {
        Error::Platform(src)
    }
}

impl From<ValueError> for Error {
    fn from(src: ValueError) -> Error {
        Error::Value(src)
    }
}
