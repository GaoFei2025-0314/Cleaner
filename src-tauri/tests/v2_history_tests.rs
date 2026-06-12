use c_drive_cleaner::v2::history::history_entry_is_desensitized;
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
