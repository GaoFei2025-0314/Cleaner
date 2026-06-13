use crate::v2::models::{DuplicateFileEntry, DuplicateFileGroup, DuplicateRecommendedAction};

pub fn duplicate_group_for_selection(files: Vec<(&str, &str, &str)>) -> DuplicateFileGroup {
    let entries = files
        .into_iter()
        .enumerate()
        .map(
            |(index, (drive, display_name, modified_at))| DuplicateFileEntry {
                entry_id: format!("entry-{index}"),
                display_name: display_name.to_string(),
                drive: drive.to_string(),
                visible_location_hint: format!("{} 盘 · 文件夹", drive.trim_end_matches(':')),
                size_bytes: 10,
                modified_at: modified_at.to_string(),
                hash_fingerprint_id: "fixture-fingerprint".to_string(),
                selected: false,
                protected: false,
                recommended_action: DuplicateRecommendedAction::ManualReview,
            },
        )
        .collect::<Vec<_>>();

    DuplicateFileGroup {
        group_id: "fixture-group".to_string(),
        strict_duplicate: true,
        total_bytes: entries.iter().map(|entry| entry.size_bytes).sum(),
        reclaimable_bytes: 0,
        files: entries,
        recommended_selection_reason: String::new(),
    }
}
