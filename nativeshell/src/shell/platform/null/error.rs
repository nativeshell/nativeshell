use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum PlatformError {
    NotImplemented,
    UnknownError,
}

pub type PlatformResult<T> = Result<T, PlatformError>;

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PlatformError {}
