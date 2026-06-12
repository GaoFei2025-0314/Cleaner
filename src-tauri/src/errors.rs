use thiserror::Error;

#[derive(Debug, Error)]
pub enum CleanerError {
    #[error("path is outside the allowed root")]
    PathOutsideAllowedRoot,
    #[error("path is outside the C drive")]
    PathOutsideCDrive,
    #[error("path cannot be resolved: {0}")]
    PathResolution(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("operation was cancelled")]
    OperationCancelled,
    #[error("recycle bin error: {0}")]
    RecycleBin(String),
    #[error("copy failed: {0}")]
    CopyFailed(String),
    #[error("verification failed: {0}")]
    VerificationFailed(String),
    #[error("settings error: {0}")]
    Settings(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

impl From<std::io::Error> for CleanerError {
    fn from(value: std::io::Error) -> Self {
        CleanerError::Io(value.to_string())
    }
}
