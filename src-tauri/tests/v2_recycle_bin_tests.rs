use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use c_drive_cleaner::v2::recycle_bin::{RecycleBin, RecycleBinError};

#[derive(Default)]
struct RecordingRecycleBin {
    paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl RecycleBin for RecordingRecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError> {
        self.paths.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

#[test]
fn recording_recycle_bin_receives_exact_path() {
    let bin = RecordingRecycleBin::default();
    bin.move_to_recycle_bin(Path::new(r"C:\Users\Example\file.tmp"))
        .unwrap();
    assert_eq!(
        bin.paths.lock().unwrap()[0],
        PathBuf::from(r"C:\Users\Example\file.tmp")
    );
}
