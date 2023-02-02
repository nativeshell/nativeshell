use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum PlatformError {
    UnknownError,
    LaunchEngineFailure,
    SendMessageFailure { channel: String },
    NotAvailable,
    NoEventFound,
}

pub type PlatformResult<T> = Result<T, PlatformError>;

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::UnknownError => {
                write!(f, "Unknown Error")
            }
            PlatformError::SendMessageFailure { channel } => {
                write!(f, "Failed to send message on channel {channel}")
            }
            PlatformError::LaunchEngineFailure => {
                write!(f, "Failed to launch Flutter engine")
            }
            PlatformError::NoEventFound => {
                write!(
                    f,
                    "Action requires prior mouse event and the event was not found"
                )
            }
            PlatformError::NotAvailable => {
                write!(f, "Feature is not available")
            }
        }
    }
}

impl std::error::Error for PlatformError {}
