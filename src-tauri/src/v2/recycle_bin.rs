use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum RecycleBinError {
    #[error("{0}")]
    Failed(String),
}

pub trait RecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError>;
}

pub struct SystemRecycleBin;

impl RecycleBin for SystemRecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError> {
        trash::delete(path).map_err(|error| RecycleBinError::Failed(error.to_string()))
    }
}
