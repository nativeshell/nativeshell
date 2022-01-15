use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum PlatformError {
    UnknownError,
    LaunchEngineFailure,
    SendMessageFailure { channel: String },
    WindowsError(windows::core::Error),
    NotAvailable,
    OffsetOutOfScreenBounds,
    OtherError { error: String },
}

pub type PlatformResult<T> = Result<T, PlatformError>;

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::UnknownError => {
                write!(f, "Unknown Error")
            }
            PlatformError::SendMessageFailure { channel } => {
                write!(f, "Failed to send message on channel {}", channel)
            }
            PlatformError::LaunchEngineFailure => {
                write!(f, "Failed to launch Flutter engine")
            }
            PlatformError::WindowsError(error) => error.fmt(f),
            PlatformError::NotAvailable => {
                write!(f, "Feature is not available")
            }
            PlatformError::OffsetOutOfScreenBounds => {
                write!(f, "Given offset is out of screen bounds")
            }
            PlatformError::OtherError { error } => {
                write!(f, "{}", error)
            }
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<windows::core::Error> for PlatformError {
    fn from(src: windows::core::Error) -> PlatformError {
        PlatformError::WindowsError(src)
    }
}
