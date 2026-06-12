use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use c_drive_cleaner::v2::large_files::LargeFileRegistry;
use c_drive_cleaner::v2::migration::{
    run_large_file_migration_cancellable_before_recycle_for_test,
    run_large_file_migration_with_backend_protected_paths_for_test,
    run_large_file_migration_with_recycle_bin, validate_migration_target,
};
use c_drive_cleaner::v2::models::{
    LargeFileCategory, LargeFileItem, LargeFileScanReport, MigrationRequest, OriginalFilePolicy,
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
fn target_inside_source_parent_is_rejected_for_temp_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("movie.mp4");
    fs::write(&source, b"movie").unwrap();
    let target = temp.path().join("Cleaner_MigratedFiles");

    let error = validate_migration_target(&source, &target).unwrap_err();

    assert!(error.to_string().contains("目标位置不能位于源文件目录内"));
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
