use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum PlatformError {
    NotImplemented,
    UnknownError,
    GLibError { message: String },
}

pub type PlatformResult<T> = Result<T, PlatformError>;

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::NotImplemented => {
                write!(f, "Not Implemented")
            }
            PlatformError::UnknownError => {
                write!(f, "Unknown Error")
            }
            PlatformError::GLibError { message } => {
                write!(f, "GLibError: {}", message)
            }
        }
    }
}

impl std::error::Error for PlatformError {}
