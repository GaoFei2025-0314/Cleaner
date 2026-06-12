use std::fs;
use std::path::Path;

use c_drive_cleaner::v2::large_files::{
    large_file_is_recommended_for_test, large_file_should_skip_dir_for_test, scan_large_files,
    scan_large_files_with_backend_settings_for_test, visible_location_hint,
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
    assert_eq!(report.items[0].display_name, "压缩包 1");
    assert_eq!(report.items[0].size_bytes, 20);
}

#[test]
fn large_file_scan_report_serialization_does_not_expose_private_identifiers() {
    let temp = tempfile::tempdir().unwrap();
    let user_named_dir = temp.path().join("Alice");
    fs::create_dir_all(&user_named_dir).unwrap();
    let private_file_name =
        "Alice-private-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.zip";
    fs::write(user_named_dir.join(private_file_name), vec![0u8; 20]).unwrap();

    let report = scan_large_files(LargeFileScanRequest {
        selected_drives: vec![],
        custom_folders: vec![user_named_dir.to_string_lossy().to_string()],
        min_size_bytes: 20,
        protected_paths: vec![],
        skip_system_dirs: true,
        skip_program_dirs: true,
    })
    .unwrap();
    let json = serde_json::to_string(&report).unwrap();

    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].display_name, "压缩包 1");
    assert!(!json.contains(private_file_name));
    assert!(!json.contains(&user_named_dir.to_string_lossy().to_string()));
    assert!(!json.contains("Alice"));
    assert!(!json.contains("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
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
fn backend_protected_paths_mark_large_scan_items_when_request_omits_them() {
    let temp = tempfile::tempdir().unwrap();
    let protected_dir = temp.path().join("protected-root");
    fs::create_dir_all(&protected_dir).unwrap();
    let private_file_name = "Alice-private-video.mp4";
    fs::write(protected_dir.join(private_file_name), vec![0u8; 20]).unwrap();

    let report = scan_large_files_with_backend_settings_for_test(
        LargeFileScanRequest {
            selected_drives: vec![],
            custom_folders: vec![protected_dir.to_string_lossy().to_string()],
            min_size_bytes: 20,
            protected_paths: vec![],
            skip_system_dirs: true,
            skip_program_dirs: true,
        },
        Ok(vec![protected_dir.to_string_lossy().to_string()]),
    )
    .unwrap();
    let json = serde_json::to_string(&report).unwrap();

    assert_eq!(report.items.len(), 1);
    assert!(report.items[0].protected);
    assert!(!report.items[0].recommended);
    assert!(!json.contains(private_file_name));
    assert!(!json.contains(&protected_dir.to_string_lossy().to_string()));
    assert!(!json.contains("Alice"));
}

#[test]
fn large_file_scan_settings_failure_returns_path_free_error() {
    let temp = tempfile::tempdir().unwrap();
    let error = scan_large_files_with_backend_settings_for_test(
        LargeFileScanRequest {
            selected_drives: vec![],
            custom_folders: vec![temp.path().join("Alice").to_string_lossy().to_string()],
            min_size_bytes: 20,
            protected_paths: vec![],
            skip_system_dirs: true,
            skip_program_dirs: true,
        },
        Err(r"C:\Users\Alice\AppData\settings.json".to_string()),
    )
    .unwrap_err();

    assert!(error.contains("无法读取清理设置"));
    assert!(!error.contains("Alice"));
    assert!(!error.contains("settings.json"));
    assert!(!error.contains(r"C:\Users"));
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

#[test]
fn large_file_system_skip_uses_path_boundaries() {
    assert!(!large_file_should_skip_dir_for_test(
        Path::new(r"C:\Windows.old"),
        true,
        true
    ));
    assert!(large_file_should_skip_dir_for_test(
        Path::new(r"C:\Windows\Temp"),
        true,
        true
    ));
    assert!(!large_file_should_skip_dir_for_test(
        Path::new(r"C:\Program Files Backup"),
        true,
        true
    ));
    assert!(large_file_should_skip_dir_for_test(
        Path::new(r"C:\Program Files\App"),
        true,
        true
    ));
}

#[test]
fn large_file_scan_request_rejects_unknown_path_fields() {
    let payload = serde_json::json!({
        "selectedDrives": [],
        "customFolders": [],
        "minSizeBytes": 20,
        "protectedPaths": [],
        "skipSystemDirs": true,
        "skipProgramDirs": true,
        "rawPath": "C:\\Users\\Alice\\Downloads"
    });

    assert!(serde_json::from_value::<LargeFileScanRequest>(payload).is_err());
}
