use std::fs;

use c_drive_cleaner::v2::history::{
    append_history_entry_at_path, clear_operation_history_at_path, history_entry_is_desensitized,
    list_operation_history_at_path,
};
use c_drive_cleaner::v2::models::{HistoryEntry, OperationModule};

#[test]
fn history_entry_rejects_paths_file_names_hashes_and_usernames() {
    let unsafe_entry = HistoryEntry {
        history_id: "h1".to_string(),
        module: OperationModule::DuplicateCleanup,
        started_at: "2026-06-12T00:00:00Z".to_string(),
        finished_at: "2026-06-12T00:00:01Z".to_string(),
        total_bytes: 10,
        freed_bytes: 10,
        c_drive_freed_bytes: 10,
        other_drive_freed_bytes: 0,
        success_count: 1,
        skipped_count: 0,
        failed_count: 0,
        error_categories: vec!["C:\\Users\\Administrator\\Desktop\\a.zip".to_string()],
    };

    assert!(!history_entry_is_desensitized(&unsafe_entry));
}

#[test]
fn history_entry_validates_all_string_fields_and_allows_iso_timestamps() {
    let safe_entry = history_entry_with_errors(vec!["权限不足".to_string()]);
    assert!(history_entry_is_desensitized(&safe_entry));

    let mut unsafe_history_id = safe_entry.clone();
    unsafe_history_id.history_id = "C:\\Users\\Administrator\\Desktop\\h1".to_string();
    assert!(!history_entry_is_desensitized(&unsafe_history_id));

    let mut unsafe_started_at = safe_entry.clone();
    unsafe_started_at.started_at = "C:\\Users\\Administrator\\Desktop\\started.txt".to_string();
    assert!(!history_entry_is_desensitized(&unsafe_started_at));

    let mut unsafe_finished_at = safe_entry;
    unsafe_finished_at.finished_at =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
    assert!(!history_entry_is_desensitized(&unsafe_finished_at));
}

#[test]
fn missing_history_file_returns_empty_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let history_path = temp_dir.path().join("missing").join("history.json");

    assert!(list_operation_history_at_path(&history_path)
        .unwrap()
        .is_empty());
}

#[test]
fn clearing_missing_history_file_succeeds() {
    let temp_dir = tempfile::tempdir().unwrap();
    let history_path = temp_dir.path().join("history.json");

    clear_operation_history_at_path(&history_path).unwrap();
}

#[test]
fn history_rejects_persisted_chinese_filename_and_raw_hash() {
    let temp_dir = tempfile::tempdir().unwrap();
    let history_path = temp_dir.path().join("history.json");
    fs::write(
        &history_path,
        serde_json::to_string(&vec![history_entry_with_errors(vec![
            "报告.pdf".to_string()
        ])])
        .unwrap(),
    )
    .unwrap();

    let filename_error = list_operation_history_at_path(&history_path).unwrap_err();
    assert_eq!(filename_error, "历史记录包含未脱敏内容");
    assert!(!filename_error.contains(&temp_dir.path().display().to_string()));

    fs::write(
        &history_path,
        serde_json::to_string(&vec![history_entry_with_errors(vec![
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        ])])
        .unwrap(),
    )
    .unwrap();

    let hash_error = list_operation_history_at_path(&history_path).unwrap_err();
    assert_eq!(hash_error, "历史记录包含未脱敏内容");
    assert!(!hash_error.contains(&temp_dir.path().display().to_string()));
}

#[test]
fn append_history_entry_creates_parent_directory_and_enforces_desensitization() {
    let temp_dir = tempfile::tempdir().unwrap();
    let history_path = temp_dir.path().join("nested").join("history.json");
    let safe_entry = history_entry_with_errors(vec!["权限不足".to_string()]);

    append_history_entry_at_path(&history_path, safe_entry.clone()).unwrap();
    assert_eq!(
        list_operation_history_at_path(&history_path).unwrap(),
        vec![safe_entry]
    );

    let error = append_history_entry_at_path(
        &history_path,
        history_entry_with_errors(vec!["照片.jpg".to_string()]),
    )
    .unwrap_err();
    assert_eq!(error, "历史记录包含未脱敏内容");
}

fn history_entry_with_errors(error_categories: Vec<String>) -> HistoryEntry {
    HistoryEntry {
        history_id: "h1".to_string(),
        module: OperationModule::DuplicateCleanup,
        started_at: "2026-06-12T00:00:00Z".to_string(),
        finished_at: "2026-06-12T00:00:01Z".to_string(),
        total_bytes: 10,
        freed_bytes: 10,
        c_drive_freed_bytes: 10,
        other_drive_freed_bytes: 0,
        success_count: 1,
        skipped_count: 0,
        failed_count: 0,
        error_categories,
    }
}
