use std::path::Path;

use c_drive_cleaner::paths::ensure_c_drive_path;

#[test]
fn accepts_absolute_c_drive_paths_only() {
    assert!(ensure_c_drive_path(Path::new(r"C:\Users\Example\AppData\Local")).is_ok());
    assert!(ensure_c_drive_path(Path::new(r"D:\Users\Example\AppData\Local")).is_err());
    assert!(ensure_c_drive_path(Path::new(r"\\server\share\AppData\Local")).is_err());
    assert!(ensure_c_drive_path(Path::new(r"C:relative\Temp")).is_err());
}
