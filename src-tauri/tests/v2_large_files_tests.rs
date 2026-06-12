use std::fs;
use std::path::Path;

use c_drive_cleaner::v2::large_files::{
    large_file_is_recommended_for_test, scan_large_files, visible_location_hint,
};
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

#[test]
fn large_file_location_hint_does_not_expose_user_or_parent_folder_names() {
    let hint = visible_location_hint(Path::new(r"C:\Users\Alice\big.iso"));
    assert!(!hint.contains("Alice"));
    assert!(!hint.contains(r"C:\Users\Alice"));
    assert_eq!(hint, r"C:\...\用户文件");
    assert_eq!(
        visible_location_hint(Path::new(r"relative\Alice\big.iso")),
        "文件夹"
    );

    let temp = tempfile::tempdir().unwrap();
    let user_like_dir = temp.path().join("Alice");
    fs::create_dir_all(&user_like_dir).unwrap();
    fs::write(user_like_dir.join("big.iso"), vec![0u8; 20]).unwrap();
    let mut payloads = Vec::new();

    let report = c_drive_cleaner::v2::large_files::scan_large_files_with_progress_for_test(
        LargeFileScanRequest {
            selected_drives: vec![],
            custom_folders: vec![user_like_dir.to_string_lossy().to_string()],
            min_size_bytes: 20,
            protected_paths: vec![],
            skip_system_dirs: true,
            skip_program_dirs: true,
        },
        |payload| payloads.push(payload),
    )
    .unwrap();

    assert_eq!(report.items.len(), 1);
    assert!(!report.items[0].visible_location_hint.contains("Alice"));
    assert!(payloads
        .iter()
        .all(|payload| !payload.current_location_hint.contains("Alice")));
}

#[test]
fn large_file_recommendation_is_limited_to_c_drive_user_profile_files() {
    assert!(large_file_is_recommended_for_test(
        Path::new(r"C:\Users\Alice\Downloads\big.iso"),
        false
    ));
    assert!(!large_file_is_recommended_for_test(
        Path::new(r"C:\Downloads\big.iso"),
        false
    ));
    assert!(!large_file_is_recommended_for_test(
        Path::new(r"D:\Users\Alice\Downloads\big.iso"),
        false
    ));
    assert!(!large_file_is_recommended_for_test(
        Path::new(r"C:\Users\Alice\Downloads\big.iso"),
        true
    ));
}
