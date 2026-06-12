use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use c_drive_cleaner::v2::duplicate::{
    apply_duplicate_recommendations, run_duplicate_cleanup_with_recycle_bin,
    scan_duplicate_files, scan_duplicate_files_cancellable_for_test,
    scan_duplicate_files_with_before_hash_for_test, DuplicateEntryRegistry,
};
use c_drive_cleaner::v2::models::{
    DuplicateCleanupFileRequest, DuplicateCleanupGroupRequest, DuplicateCleanupRequest,
    DuplicateFileType, DuplicateRecommendedAction, DuplicateScanRequest,
};
use c_drive_cleaner::v2::path_safety::{
    is_protected_duplicate_path, should_skip_scan_location,
};
use c_drive_cleaner::v2::recycle_bin::{RecycleBin, RecycleBinError};

#[derive(Default)]
struct RecordingRecycleBin {
    paths: Arc<Mutex<Vec<std::path::PathBuf>>>,
}

impl RecycleBin for RecordingRecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError> {
        self.paths.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

#[test]
fn strict_duplicates_are_grouped_by_size_and_hash() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.txt"), b"same").unwrap();
    fs::write(temp.path().join("b.txt"), b"same").unwrap();
    fs::write(temp.path().join("c.txt"), b"different").unwrap();

    let report = scan_duplicate_files(DuplicateScanRequest {
        selected_drives: vec![],
        custom_folders: vec![temp.path().to_string_lossy().to_string()],
        file_types: vec![DuplicateFileType::Document],
        custom_extensions: vec![],
        include_suspected: false,
        min_size_bytes: 1,
        protected_paths: vec![],
    })
    .unwrap();

    assert_eq!(report.strict_groups.len(), 1);
    assert_eq!(report.strict_groups[0].files.len(), 2);
    assert!(report.strict_groups[0].strict_duplicate);
}

#[test]
fn c_drive_first_strategy_keeps_non_c_drive_copy_when_possible() {
    let mut group = c_drive_cleaner::v2::fixtures::duplicate_group_for_selection(vec![
        ("C:", "old-c-copy.txt", "2026-01-01T00:00:00Z"),
        ("D:", "backup-copy.txt", "2025-01-01T00:00:00Z"),
    ]);

    apply_duplicate_recommendations(&mut group, &[]);

    let c_file = group.files.iter().find(|file| file.drive == "C:").unwrap();
    let d_file = group.files.iter().find(|file| file.drive == "D:").unwrap();
    assert_eq!(
        c_file.recommended_action,
        c_drive_cleaner::v2::models::DuplicateRecommendedAction::Clean
    );
    assert_eq!(
        d_file.recommended_action,
        c_drive_cleaner::v2::models::DuplicateRecommendedAction::Keep
    );
}

#[test]
fn unreadable_candidate_during_hashing_is_skipped_without_aborting_scan() {
    let temp = tempfile::tempdir().unwrap();
    let keep_a = temp.path().join("keep-a.txt");
    let keep_b = temp.path().join("keep-b.txt");
    let disappearing = temp.path().join("disappearing.txt");
    fs::write(&keep_a, b"same").unwrap();
    fs::write(&keep_b, b"same").unwrap();
    fs::write(&disappearing, b"same").unwrap();

    let report = scan_duplicate_files_with_before_hash_for_test(
        DuplicateScanRequest {
            selected_drives: vec![],
            custom_folders: vec![temp.path().to_string_lossy().to_string()],
            file_types: vec![DuplicateFileType::Document],
            custom_extensions: vec![],
            include_suspected: false,
            min_size_bytes: 1,
            protected_paths: vec![],
        },
        |path| {
            if path.file_name().and_then(|name| name.to_str()) == Some("disappearing.txt") {
                let _ = fs::remove_file(path);
            }
        },
    )
    .unwrap();

    assert_eq!(report.strict_groups.len(), 1);
    assert_eq!(report.strict_groups[0].files.len(), 2);
    assert!(report.strict_groups[0]
        .files
        .iter()
        .all(|file| file.display_name.starts_with("keep-")));
    assert_eq!(report.skipped_locations, 1);
}

#[test]
fn protected_duplicate_files_are_reported_but_not_auto_selected() {
    let temp = tempfile::tempdir().unwrap();
    let protected_dir = temp.path().join("protected");
    let public_dir = temp.path().join("public");
    fs::create_dir_all(&protected_dir).unwrap();
    fs::create_dir_all(&public_dir).unwrap();
    fs::write(protected_dir.join("protected-copy.txt"), b"same").unwrap();
    fs::write(public_dir.join("public-copy.txt"), b"same").unwrap();

    let report = scan_duplicate_files(DuplicateScanRequest {
        selected_drives: vec![],
        custom_folders: vec![temp.path().to_string_lossy().to_string()],
        file_types: vec![DuplicateFileType::Document],
        custom_extensions: vec![],
        include_suspected: false,
        min_size_bytes: 1,
        protected_paths: vec![protected_dir.to_string_lossy().to_string()],
    })
    .unwrap();

    assert_eq!(report.strict_groups.len(), 1);
    let protected_file = report.strict_groups[0]
        .files
        .iter()
        .find(|file| file.display_name == "protected-copy.txt")
        .unwrap();
    assert!(protected_file.protected);
    assert!(!protected_file.selected);
    assert_ne!(
        protected_file.recommended_action,
        DuplicateRecommendedAction::Clean
    );
}

#[test]
fn built_in_protected_paths_are_marked_protected_but_not_hidden_from_scan() {
    assert!(is_protected_duplicate_path(
        Path::new(r"C:\Windows\Temp\x.txt"),
        &[]
    ));
    assert!(is_protected_duplicate_path(
        Path::new(r"\\?\C:\Windows\Temp\x.txt"),
        &[]
    ));
    assert!(!should_skip_scan_location(
        Path::new(r"C:\Windows"),
        &[]
    ));
}

#[test]
fn cleanup_wire_model_uses_entry_ids_not_raw_paths() {
    let request = DuplicateCleanupRequest {
        groups: vec![DuplicateCleanupGroupRequest {
            group_id: "group-1".to_string(),
            files: vec![DuplicateCleanupFileRequest {
                entry_id: "entry-1".to_string(),
                selected: true,
                protected: false,
            }],
        }],
        protected_paths: vec![],
        protected_override_confirmed: false,
    };

    let encoded = serde_json::to_string(&request).unwrap();

    assert!(encoded.contains("entryId"));
    assert!(!encoded.contains("\"path\""));
}

#[test]
fn overlapping_scan_roots_do_not_duplicate_the_same_physical_file() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(child.join("only.txt"), b"same").unwrap();

    let report = scan_duplicate_files(DuplicateScanRequest {
        selected_drives: vec![],
        custom_folders: vec![
            temp.path().to_string_lossy().to_string(),
            child.to_string_lossy().to_string(),
        ],
        file_types: vec![DuplicateFileType::Document],
        custom_extensions: vec![],
        include_suspected: false,
        min_size_bytes: 1,
        protected_paths: vec![],
    })
    .unwrap();

    assert!(report.strict_groups.is_empty());
    assert_eq!(report.scanned_files, 1);
}

#[test]
fn cleanup_skips_when_same_entry_is_selected_and_retained() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("same.txt");
    fs::write(&file, b"same").unwrap();
    let registry = DuplicateEntryRegistry::default();
    registry.register_test_entry("group-1", "entry-1", &file, false);
    let recycle_bin = RecordingRecycleBin::default();

    let report = run_duplicate_cleanup_with_recycle_bin(
        DuplicateCleanupRequest {
            groups: vec![DuplicateCleanupGroupRequest {
                group_id: "group-1".to_string(),
                files: vec![
                    DuplicateCleanupFileRequest {
                        entry_id: "entry-1".to_string(),
                        selected: true,
                        protected: false,
                    },
                    DuplicateCleanupFileRequest {
                        entry_id: "entry-1".to_string(),
                        selected: false,
                        protected: false,
                    },
                ],
            }],
            protected_paths: vec![],
            protected_override_confirmed: false,
        },
        &registry,
        &recycle_bin,
    );

    assert_eq!(report.success_count, 0);
    assert_eq!(report.skipped_count, 1);
    assert!(recycle_bin.paths.lock().unwrap().is_empty());
}

#[test]
fn cleanup_skips_when_retained_duplicate_disappears() {
    let temp = tempfile::tempdir().unwrap();
    let selected = temp.path().join("selected.txt");
    let retained = temp.path().join("retained.txt");
    fs::write(&selected, b"same").unwrap();
    fs::write(&retained, b"same").unwrap();
    let registry = DuplicateEntryRegistry::default();
    registry.register_test_entry("group-1", "selected", &selected, false);
    registry.register_test_entry("group-1", "retained", &retained, false);
    fs::remove_file(&retained).unwrap();
    let recycle_bin = RecordingRecycleBin::default();

    let report = run_duplicate_cleanup_with_recycle_bin(
        DuplicateCleanupRequest {
            groups: vec![DuplicateCleanupGroupRequest {
                group_id: "group-1".to_string(),
                files: vec![
                    DuplicateCleanupFileRequest {
                        entry_id: "selected".to_string(),
                        selected: true,
                        protected: false,
                    },
                    DuplicateCleanupFileRequest {
                        entry_id: "retained".to_string(),
                        selected: false,
                        protected: false,
                    },
                ],
            }],
            protected_paths: vec![],
            protected_override_confirmed: false,
        },
        &registry,
        &recycle_bin,
    );

    assert_eq!(report.success_count, 0);
    assert_eq!(report.skipped_count, 1);
    assert!(recycle_bin.paths.lock().unwrap().is_empty());
}

#[test]
fn cancellable_scan_core_stops_before_hashing_remaining_work() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.txt"), b"same").unwrap();
    fs::write(temp.path().join("b.txt"), b"same").unwrap();
    let cancelled = AtomicBool::new(false);

    let result = scan_duplicate_files_cancellable_for_test(
        DuplicateScanRequest {
            selected_drives: vec![],
            custom_folders: vec![temp.path().to_string_lossy().to_string()],
            file_types: vec![DuplicateFileType::Document],
            custom_extensions: vec![],
            include_suspected: false,
            min_size_bytes: 1,
            protected_paths: vec![],
        },
        &cancelled,
        |_| {
            cancelled.store(true, Ordering::Relaxed);
        },
    );

    assert_eq!(result.unwrap_err(), "操作已取消");
}
