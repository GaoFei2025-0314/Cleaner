use std::fs;

use c_drive_cleaner::cleanup::{delete_path_contents, validate_high_risk_confirmation};
use c_drive_cleaner::models::{
    CleanupAction, CleanupSelection, RiskLevel, ScanItem, SourceCategory,
};

#[test]
fn deletes_contents_without_deleting_parent_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Temp");
    fs::create_dir_all(&parent).expect("parent");
    fs::write(parent.join("old.log"), "abc").expect("file");

    let freed = delete_path_contents(&parent).expect("deleted");

    assert!(parent.exists());
    assert!(!parent.join("old.log").exists());
    assert_eq!(freed, 3);
}

#[test]
fn rejects_high_risk_without_second_confirmation() {
    let item = ScanItem {
        id: "wechat-video-cache".to_string(),
        title: "微信视频缓存子目录".to_string(),
        description: "精确命中的微信视频缓存。".to_string(),
        source_category: SourceCategory::Wechat,
        risk_level: RiskLevel::HighRisk,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 10,
        default_selected: false,
        user_visible_path_hint: "微信视频缓存子目录".to_string(),
        technical_path: None,
        reasons: vec![],
        warnings: vec![],
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["wechat-video-cache".to_string()],
        high_risk_confirmed: false,
        request_admin_mode: false,
    };

    assert!(validate_high_risk_confirmation(&selection, &[item]).is_err());
}
