use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use c_drive_cleaner::v2::large_files::LargeFileRegistry;
use c_drive_cleaner::v2::migration::{
    copy_file_to_temp_with_cleanup_for_test, ensure_available_space_for_test,
    migration_operation_status_for_test,
    run_large_file_migration_cancellable_before_recycle_for_test,
    run_large_file_migration_cancellable_before_verify_for_test,
    run_large_file_migration_with_backend_protected_paths_for_test,
    run_large_file_migration_with_backend_settings_for_test,
    run_large_file_migration_with_recycle_bin, select_best_mount_key_for_target_for_test,
    target_conflicts_with_backend_protected_paths_for_test, validate_migration_target,
};
use c_drive_cleaner::v2::models::{
    LargeFileCategory, LargeFileItem, LargeFileScanReport, MigrationRequest, MigrationResult,
    OperationStatus, OriginalFilePolicy,
};
use c_drive_cleaner::v2::recycle_bin::{RecycleBin, RecycleBinError};

#[test]
fn migration_target_cannot_be_inside_source_folder() {
    let error = validate_migration_target(
        r"C:\Users\Example\Downloads\movie.mp4",
        r"C:\Users\Example\Downloads\Cleaner_MigratedFiles",
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于源文件目录内"));
}

#[test]
fn migration_target_with_traversal_cannot_resolve_inside_source_folder() {
    let error = validate_migration_target(
        r"C:\Users\Alice\Downloads\movie.mp4",
        r"C:\Users\Alice\Other\..\Downloads\Cleaner_MigratedFiles",
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能包含上级目录跳转"));
}

#[test]
fn migration_target_with_any_parent_traversal_is_rejected() {
    let error = validate_migration_target(
        r"C:\Users\Alice\Downloads\movie.mp4",
        r"D:\Safe\..\Cleaner_MigratedFiles",
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能包含上级目录跳转"));
}

#[test]
fn backend_protected_target_with_traversal_is_rejected() {
    let error = target_conflicts_with_backend_protected_paths_for_test(
        Path::new(r"C:\Other\..\Protected\Migrated"),
        &[r"C:\Protected".to_string()],
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能包含上级目录跳转"));
}

#[test]
fn system_protected_target_matching_uses_path_boundaries() {
    assert!(target_conflicts_with_backend_protected_paths_for_test(
        Path::new(r"C:\Windows.old\Cleaner_MigratedFiles"),
        &[],
    )
    .is_ok());

    let error = target_conflicts_with_backend_protected_paths_for_test(
        Path::new(r"C:\Windows\Temp\Cleaner_MigratedFiles"),
        &[],
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于受保护目录内"));
}

#[test]
fn migration_request_rejects_unknown_raw_path_field() {
    let payload = serde_json::json!({
        "selectedItemIds": ["item-1"],
        "scanReport": empty_report(),
        "targetFolder": "D:\\Cleaner_MigratedFiles",
        "originalFilePolicy": "keepOriginal",
        "protectedOverrideConfirmed": false,
        "rawPath": "C:\\Users\\Example\\Downloads\\movie.mp4"
    });

    assert!(serde_json::from_value::<MigrationRequest>(payload).is_err());
}

#[test]
fn migration_request_rejects_nested_raw_path_in_scan_report_item() {
    let payload = serde_json::json!({
        "selectedItemIds": ["item-1"],
        "scanReport": {
            "items": [{
                "itemId": "item-1",
                "displayName": "movie.mp4",
                "drive": "C:",
                "visibleLocationHint": "C:\\...\\用户文件",
                "sizeBytes": 11,
                "modifiedAt": "2026-06-12T00:00:00Z",
                "category": "video",
                "selected": true,
                "protected": false,
                "recommended": true,
                "rawPath": "C:\\Users\\Alice\\Downloads\\movie.mp4"
            }],
            "scannedFiles": 1,
            "skippedLocations": 0,
            "totalBytes": 11,
            "cDriveBytes": 11,
            "otherDriveBytes": 0
        },
        "targetFolder": "D:\\Cleaner_MigratedFiles",
        "originalFilePolicy": "keepOriginal",
        "protectedOverrideConfirmed": false
    });

    assert!(serde_json::from_value::<MigrationRequest>(payload).is_err());
}

#[test]
fn migration_rejects_relative_target_folder_without_copying() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let mut request = request_for(
        "item-1",
        false,
        Path::new("Cargo.toml"),
        OriginalFilePolicy::KeepOriginal,
    );
    request.target_folder = "Cargo.toml".to_string();

    let result = run_large_file_migration_with_recycle_bin(
        request,
        &registry,
        &RecordingRecycleBin::default(),
    );
    let json = serde_json::to_string(&result).unwrap();

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(result.item_results[0]
        .message
        .contains("目标文件夹必须是本地磁盘绝对路径"));
    assert!(!json.contains("Cargo.toml"));
    assert!(!json.contains(&source.to_string_lossy().to_string()));
}

#[test]
fn migration_rejects_empty_target_folder_without_copying() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let mut request = request_for(
        "item-1",
        false,
        temp.path(),
        OriginalFilePolicy::KeepOriginal,
    );
    request.target_folder = "   ".to_string();

    let result = run_large_file_migration_with_recycle_bin(
        request,
        &registry,
        &RecordingRecycleBin::default(),
    );
    let json = serde_json::to_string(&result).unwrap();

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(result.item_results[0]
        .message
        .contains("目标文件夹不能为空"));
    assert!(!json.contains(&source.to_string_lossy().to_string()));
    assert!(!json.contains("movie.mp4"));
}

#[test]
fn free_space_check_rejects_unmatched_target_disk() {
    let error = ensure_available_space_for_test(
        Path::new(r"Z:\Cleaner_MigratedFiles"),
        1,
        &[(r"C:", 1024)],
    )
    .unwrap_err();

    assert!(error.contains("无法识别目标磁盘"));
    assert!(!error.contains("Z:"));
    assert!(!error.contains("Cleaner_MigratedFiles"));
}

#[test]
fn target_inside_source_parent_is_rejected_for_temp_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("movie.mp4");
    fs::write(&source, b"movie").unwrap();
    let target = temp.path().join("Cleaner_MigratedFiles");

    let error = validate_migration_target(&source, &target).unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于源文件目录内"));
}

#[test]
fn migration_target_inside_symlink_ancestor_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    let real_target_parent = temp.path().join("real-target-parent");
    let link = temp.path().join("target-link");
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&real_target_parent).unwrap();
    if create_dir_symlink(&real_target_parent, &link).is_err() {
        return;
    }
    let source = source_dir.join("movie.mp4");
    fs::write(&source, b"movie").unwrap();

    let error = validate_migration_target(&source, link.join("Migrated")).unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于符号链接目录内"));
}

#[test]
fn migration_target_inside_broken_symlink_ancestor_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    let missing_target_parent = temp.path().join("missing-target-parent");
    let link = temp.path().join("broken-target-link");
    fs::create_dir_all(&source_dir).unwrap();
    if create_dir_symlink(&missing_target_parent, &link).is_err() {
        return;
    }
    let source = source_dir.join("movie.mp4");
    fs::write(&source, b"movie").unwrap();

    let error = validate_migration_target(&source, link.join("Migrated")).unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于符号链接目录内"));
}

#[test]
fn migration_copies_and_verifies_size_and_hash_before_recycle() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = RecordingRecycleBin::default();

    let result = run_large_file_migration_with_recycle_bin(
        request_for(
            "item-1",
            false,
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
    );

    assert_eq!(result.copied_count, 1);
    assert_eq!(result.moved_to_recycle_bin_count, 1);
    assert_eq!(fs::read(target.join("movie.mp4")).unwrap(), b"movie-bytes");
    assert_eq!(recycle_bin.moved_count(), 1);
}

#[test]
fn recycle_failure_counts_copied_but_not_freed() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = FailingRecycleBin;

    let result = run_large_file_migration_with_recycle_bin(
        request_for(
            "item-1",
            false,
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
    );

    assert_eq!(result.copied_count, 1);
    assert_eq!(result.moved_to_recycle_bin_count, 0);
    assert_eq!(result.total_copied_bytes, 11);
    assert_eq!(result.total_freed_bytes, 0);
    assert!(target.join("movie.mp4").exists());
    assert!(source.exists());
}

#[test]
fn migration_skips_protected_item_without_override() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, true);
    let recycle_bin = RecordingRecycleBin::default();

    let result = run_large_file_migration_with_recycle_bin(
        request_for("item-1", true, &target, OriginalFilePolicy::KeepOriginal),
        &registry,
        &recycle_bin,
    );

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.skipped_count, 1);
    assert!(!target.join("movie.mp4").exists());
}

#[test]
fn migration_skips_backend_protected_source_without_override() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = RecordingRecycleBin::default();

    let result = run_large_file_migration_with_backend_protected_paths_for_test(
        request_for(
            "item-1",
            false,
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
        &[source_dir.to_string_lossy().to_string()],
    );

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(recycle_bin.moved_count(), 0);
    assert!(!target.join("movie.mp4").exists());
}

#[test]
fn migration_skips_source_when_backend_protected_path_is_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("protected");
    let protected_link = temp.path().join("protected-link");
    fs::create_dir_all(&protected).unwrap();
    if create_dir_symlink(&protected, &protected_link).is_err() {
        return;
    }
    let source = protected.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = RecordingRecycleBin::default();

    let result = run_large_file_migration_with_backend_protected_paths_for_test(
        request_for(
            "item-1",
            false,
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
        &[protected_link.to_string_lossy().to_string()],
    );

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(recycle_bin.moved_count(), 0);
    assert!(!target.join("movie.mp4").exists());
}

#[test]
fn migration_settings_failure_fails_closed_without_copying() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = RecordingRecycleBin::default();

    let error = run_large_file_migration_with_backend_settings_for_test(
        request_for("item-1", false, &target, OriginalFilePolicy::KeepOriginal),
        &registry,
        &recycle_bin,
        Err(r"C:\Users\Alice\AppData\settings.json".to_string()),
    )
    .unwrap_err();

    assert!(error.contains("无法读取清理设置"));
    assert!(!error.contains("Alice"));
    assert!(!error.contains("settings.json"));
    assert!(!error.contains(r"C:\Users"));
    assert!(!target.exists());
    assert_eq!(recycle_bin.moved_count(), 0);
}

#[test]
fn migration_uses_registry_snapshot_after_start() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let first = source_dir.join("first.mp4");
    let second = source_dir.join("second.mp4");
    let target = temp.path().join("out");
    fs::write(&first, b"first-bytes").unwrap();
    fs::write(&second, b"second-byte").unwrap();
    let registry = registry_with_items(&[("item-1", &first, false), ("item-2", &second, false)]);
    let recycle_bin = RecordingRecycleBin::default();
    let cancelled = AtomicBool::new(false);
    let registry_cleared = AtomicBool::new(false);

    let result = run_large_file_migration_cancellable_before_recycle_for_test(
        request_for_items(
            &[("item-1", false), ("item-2", false)],
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
        &cancelled,
        |_| {
            if !registry_cleared.swap(true, Ordering::Relaxed) {
                registry.replace_entries(Vec::new());
            }
        },
    )
    .unwrap();

    assert_eq!(result.copied_count, 2);
    assert_eq!(result.failed_count, 0);
    assert!(target.join("first.mp4").exists());
    assert!(target.join("second.mp4").exists());
}

#[test]
fn completed_migration_status_ignores_late_cancel_flag() {
    assert_eq!(
        migration_operation_status_for_test(&Ok(empty_migration_result()), true),
        OperationStatus::Completed
    );
    assert_eq!(
        migration_operation_status_for_test(&Err(empty_migration_result()), false),
        OperationStatus::Cancelled
    );
}

#[test]
fn migration_rejects_target_inside_backend_protected_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let protected_root = temp.path().join("protected");
    let target = protected_root.join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);

    let result = run_large_file_migration_with_backend_protected_paths_for_test(
        request_for("item-1", false, &target, OriginalFilePolicy::KeepOriginal),
        &registry,
        &RecordingRecycleBin::default(),
        &[protected_root.to_string_lossy().to_string()],
    );

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(!target.join("movie.mp4").exists());
}

#[test]
fn cancellation_before_recycle_prevents_original_move() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let recycle_bin = RecordingRecycleBin::default();
    let cancelled = AtomicBool::new(false);

    let result = run_large_file_migration_cancellable_before_recycle_for_test(
        request_for(
            "item-1",
            false,
            &target,
            OriginalFilePolicy::MoveOriginalToRecycleBin,
        ),
        &registry,
        &recycle_bin,
        &cancelled,
        |_| cancelled.store(true, Ordering::Relaxed),
    )
    .unwrap_err();

    assert_eq!(result.copied_count, 1);
    assert_eq!(result.moved_to_recycle_bin_count, 0);
    assert!(source.exists());
    assert_eq!(recycle_bin.moved_count(), 0);
}

#[test]
fn cancellation_before_verify_removes_temp_copy_and_does_not_report_copied() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let cancelled = AtomicBool::new(false);

    let result = run_large_file_migration_cancellable_before_verify_for_test(
        request_for("item-1", false, &target, OriginalFilePolicy::KeepOriginal),
        &registry,
        &RecordingRecycleBin::default(),
        &cancelled,
        |_| cancelled.store(true, Ordering::Relaxed),
    )
    .unwrap_err();

    assert_eq!(result.copied_count, 0);
    assert_eq!(result.total_copied_bytes, 0);
    assert!(!target.join("movie.mp4").exists());
    if target.exists() {
        assert!(fs::read_dir(&target).unwrap().next().is_none());
    }
}

#[test]
fn copy_error_removes_partial_temp_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.bin");
    let temp_copy = temp.path().join(".cleaner-copy-partial.tmp");
    fs::write(&source, b"source").unwrap();

    let error = copy_file_to_temp_with_cleanup_for_test(&source, &temp_copy, |_, temp_path| {
        fs::write(temp_path, b"partial").unwrap();
        Err(io::Error::new(io::ErrorKind::Other, "copy failed"))
    })
    .unwrap_err();

    assert!(error.contains("复制失败"));
    assert!(!temp_copy.exists());
}

#[test]
fn migration_result_and_progress_are_path_free() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("movie.mp4");
    let target = temp.path().join("out");
    fs::write(&source, b"movie-bytes").unwrap();
    let registry = registry_with_item("item-1", &source, false);
    let source_text = source.to_string_lossy().to_string();
    let temp_text = temp.path().to_string_lossy().to_string();
    let mut payloads = Vec::new();

    let result = c_drive_cleaner::v2::migration::run_large_file_migration_with_progress_for_test(
        request_for("item-1", false, &target, OriginalFilePolicy::KeepOriginal),
        &registry,
        &RecordingRecycleBin::default(),
        |payload| payloads.push(payload),
    );
    let json = serde_json::to_string(&result).unwrap();

    assert!(!json.contains(&source_text));
    assert!(payloads
        .iter()
        .all(|payload| !payload.current_location_hint.contains(&temp_text)));
}

#[test]
fn protected_target_is_rejected_when_existing_ancestor_is_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("protected");
    let link = temp.path().join("link");
    fs::create_dir_all(&protected).unwrap();
    if create_dir_symlink(&protected, &link).is_err() {
        return;
    }

    let error = target_conflicts_with_backend_protected_paths_for_test(
        &link.join("Migrated"),
        &[protected.to_string_lossy().to_string()],
    )
    .unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于符号链接目录内"));
}

#[test]
fn free_space_mount_matching_uses_longest_boundary_match() {
    assert_eq!(
        select_best_mount_key_for_target_for_test(
            r"c:\mount\child",
            &[r"c:", r"c:\mount", r"c:\mountain"]
        ),
        Some(r"c:\mount".to_string())
    );
    assert_eq!(
        select_best_mount_key_for_target_for_test(r"c:\mountain\child", &[r"c:\mount"]),
        None
    );
    assert_eq!(
        select_best_mount_key_for_target_for_test(r"c:\mount", &[r"c:\mount"]),
        Some(r"c:\mount".to_string())
    );
}

fn empty_report() -> serde_json::Value {
    serde_json::json!({
        "items": [],
        "scannedFiles": 0,
        "skippedLocations": 0,
        "totalBytes": 0,
        "cDriveBytes": 0,
        "otherDriveBytes": 0
    })
}

fn registry_with_item(item_id: &str, path: &Path, protected: bool) -> LargeFileRegistry {
    let registry = LargeFileRegistry::default();
    registry.register_test_entry(item_id, path, protected);
    registry
}

fn registry_with_items(items: &[(&str, &Path, bool)]) -> LargeFileRegistry {
    let registry = LargeFileRegistry::default();
    for (item_id, path, protected) in items {
        registry.register_test_entry(item_id, path, *protected);
    }
    registry
}

fn request_for(
    item_id: &str,
    protected: bool,
    target: &Path,
    original_file_policy: OriginalFilePolicy,
) -> MigrationRequest {
    MigrationRequest {
        selected_item_ids: vec![item_id.to_string()],
        scan_report: LargeFileScanReport {
            items: vec![LargeFileItem {
                item_id: item_id.to_string(),
                display_name: "movie.mp4".to_string(),
                drive: String::new(),
                visible_location_hint: "folder".to_string(),
                size_bytes: 11,
                modified_at: "2026-06-12T00:00:00Z".to_string(),
                category: LargeFileCategory::Video,
                selected: true,
                protected,
                recommended: !protected,
            }],
            scanned_files: 1,
            skipped_locations: 0,
            total_bytes: 11,
            c_drive_bytes: 0,
            other_drive_bytes: 11,
        },
        target_folder: target.to_string_lossy().to_string(),
        original_file_policy,
        protected_override_confirmed: false,
    }
}

fn request_for_items(
    items: &[(&str, bool)],
    target: &Path,
    original_file_policy: OriginalFilePolicy,
) -> MigrationRequest {
    MigrationRequest {
        selected_item_ids: items
            .iter()
            .map(|(item_id, _)| (*item_id).to_string())
            .collect(),
        scan_report: LargeFileScanReport {
            items: items
                .iter()
                .map(|(item_id, protected)| LargeFileItem {
                    item_id: (*item_id).to_string(),
                    display_name: "movie.mp4".to_string(),
                    drive: String::new(),
                    visible_location_hint: "folder".to_string(),
                    size_bytes: 11,
                    modified_at: "2026-06-12T00:00:00Z".to_string(),
                    category: LargeFileCategory::Video,
                    selected: true,
                    protected: *protected,
                    recommended: !*protected,
                })
                .collect(),
            scanned_files: items.len() as u64,
            skipped_locations: 0,
            total_bytes: 11 * items.len() as u64,
            c_drive_bytes: 0,
            other_drive_bytes: 11 * items.len() as u64,
        },
        target_folder: target.to_string_lossy().to_string(),
        original_file_policy,
        protected_override_confirmed: false,
    }
}

fn empty_migration_result() -> MigrationResult {
    MigrationResult {
        copied_count: 0,
        moved_to_recycle_bin_count: 0,
        skipped_count: 0,
        failed_count: 0,
        total_copied_bytes: 0,
        total_freed_bytes: 0,
        c_drive_freed_bytes: 0,
        item_results: Vec::new(),
    }
}

#[derive(Default)]
struct RecordingRecycleBin {
    moved: std::sync::Mutex<Vec<String>>,
}

impl RecordingRecycleBin {
    fn moved_count(&self) -> usize {
        self.moved.lock().unwrap().len()
    }
}

impl RecycleBin for RecordingRecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError> {
        self.moved.lock().unwrap().push(path.display().to_string());
        Ok(())
    }
}

struct FailingRecycleBin;

impl RecycleBin for FailingRecycleBin {
    fn move_to_recycle_bin(&self, _path: &Path) -> Result<(), RecycleBinError> {
        Err(RecycleBinError::Failed("no recycle bin".to_string()))
    }
}

#[cfg(windows)]
fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(original, link)
}

#[cfg(unix)]
fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}
