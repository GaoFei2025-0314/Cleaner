use std::fs;

use c_drive_cleaner::v2::large_files::scan_large_files;
use c_drive_cleaner::v2::models::{LargeFileScanRequest, OperationModule};

#[test]
fn scan_large_files_returns_items_at_or_above_threshold() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("small.zip"), vec![0u8; 10]).unwrap();
    fs::write(temp.path().join("large.zip"), vec![0u8; 20]).unwrap();

    let report = scan_large_files(LargeFileScanRequest {
        selected_drives: vec![],
        custom_folders: vec![temp.path().to_string_lossy().to_string()],
        min_size_bytes: 20,
        protected_paths: vec![],
        skip_system_dirs: true,
        skip_program_dirs: true,
    })
    .unwrap();

    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].display_name, "large.zip");
    assert_eq!(report.items[0].size_bytes, 20);
}

#[test]
fn protected_large_files_are_visible_but_not_recommended() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("protected.mp4");
    fs::write(&file, vec![0u8; 20]).unwrap();

    let report = scan_large_files(LargeFileScanRequest {
        selected_drives: vec![],
        custom_folders: vec![temp.path().to_string_lossy().to_string()],
        min_size_bytes: 20,
        protected_paths: vec![temp.path().to_string_lossy().to_string()],
        skip_system_dirs: true,
        skip_program_dirs: true,
    })
    .unwrap();

    assert_eq!(report.items.len(), 1);
    assert!(report.items[0].protected);
    assert!(!report.items[0].recommended);
}

#[test]
fn large_file_progress_is_path_free() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("large.zip"), vec![0u8; 20]).unwrap();
    let temp_text = temp.path().to_string_lossy().to_string();
    let mut payloads = Vec::new();

    let report = c_drive_cleaner::v2::large_files::scan_large_files_with_progress_for_test(
        LargeFileScanRequest {
            selected_drives: vec![],
            custom_folders: vec![temp_text.clone()],
            min_size_bytes: 20,
            protected_paths: vec![],
            skip_system_dirs: true,
            skip_program_dirs: true,
        },
        |payload| payloads.push(payload),
    )
    .unwrap();

    assert_eq!(report.items.len(), 1);
    assert!(payloads
        .iter()
        .any(|payload| payload.module == OperationModule::LargeFileScan));
    assert!(payloads
        .iter()
        .all(|payload| !payload.current_location_hint.contains(&temp_text)));
}
