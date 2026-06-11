use thiserror::Error;

#[derive(Debug, Error)]
pub enum CleanerError {
    #[error("path is outside the allowed root")]
    PathOutsideAllowedRoot,
    #[error("path cannot be resolved: {0}")]
    PathResolution(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CleanerError {
    fn from(value: std::io::Error) -> Self {
        CleanerError::Io(value.to_string())
    }
}
