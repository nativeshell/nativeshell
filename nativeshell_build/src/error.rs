use std::{fmt::Display, io, path::PathBuf, process::ExitStatus};

#[derive(Debug)]
pub enum FileOperation {
    CreateDir,
    Copy,
    Move,
    Remove,
    Read,
    Write,
    SymLink,
    MetaData,
    CopyDir,
    MkDir,
    ReadDir,
    Canonicalize,
    Command,
}
#[derive(Debug)]
pub enum BuildError {
    FlutterToolError {
        command: String,
        status: ExitStatus,
        stderr: String,
        stdout: String,
    },
    FileOperationError {
        operation: FileOperation,
        path: PathBuf,
        source_path: Option<PathBuf>,
        source: io::Error,
    },
    JsonError {
        text: Option<String>,
        source: serde_json::Error,
    },
    OtherError(String),
}

pub type BuildResult<T> = Result<T, BuildError>;

impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::FlutterToolError {
                command,
                status,
                stderr,
                stdout,
            } => {
                write!(
                    f,
                    "Flutter Tool Failed!\nStatus: {:?}\nCommand: {:?}\nStderr:\n{}\nStdout:\n{}",
                    status, command, stderr, stdout
                )
            }
            BuildError::FileOperationError {
                operation,
                path,
                source_path,
                source,
            } => match source_path {
                Some(source_path) => {
                    write!(
                        f,
                        "File operation failed: {:?}, target path: {:?}, source path: {:?}, error: {}",
                        operation, path, source_path, source
                    )
                }
                None => {
                    write!(
                        f,
                        "File operation failed: {:?}, path: {:?}, error: {}",
                        operation, path, source
                    )
                }
            },
            BuildError::JsonError { text, source } => {
                write!(f, "JSON operation failed: ${}", source)?;
                if let Some(text) = text {
                    write!(f, "Text:\n{}", text)?;
                }
                Ok(())
            }
            BuildError::OtherError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

impl std::error::Error for BuildError {}

pub(super) trait IOResultExt<T> {
    fn wrap_error(self, operation: FileOperation, path: PathBuf) -> BuildResult<T>;
    fn wrap_error_with_src(
        self,
        operation: FileOperation,
        path: PathBuf,
        source_path: PathBuf,
    ) -> Result<T, BuildError>;
}

impl<T> IOResultExt<T> for io::Result<T> {
    fn wrap_error(self, operation: FileOperation, path: PathBuf) -> BuildResult<T> {
        self.map_err(|e| BuildError::FileOperationError {
            operation,
            path,
            source_path: None,
            source: e,
        })
    }

    fn wrap_error_with_src(
        self,
        operation: FileOperation,
        path: PathBuf,
        source_path: PathBuf,
    ) -> Result<T, BuildError> {
        self.map_err(|e| BuildError::FileOperationError {
            operation,
            path,
            source_path: Some(source_path),
            source: e,
        })
    }
}
