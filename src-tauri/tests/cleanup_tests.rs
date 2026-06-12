use std::fs;
use std::path::PathBuf;

use c_drive_cleaner::cleanup::{
    delete_path_contents, execute_selected_cleanup, validate_high_risk_confirmation,
};
use c_drive_cleaner::models::{
    CleanupAction, CleanupSelection, RiskLevel, ScanItem, SourceCategory,
};
use c_drive_cleaner::paths::ScanRoots;

#[cfg(windows)]
use std::os::windows::fs::symlink_dir;

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
    };

    assert!(validate_high_risk_confirmation(&selection, &[item]).is_err());
}

#[test]
fn reports_selected_item_that_disappeared_before_cleanup() {
    let roots = ScanRoots {
        c_drive: PathBuf::from(r"C:\"),
        user_profile: PathBuf::from(r"C:\Users\Example"),
        local_app_data: PathBuf::from(r"C:\Users\Example\AppData\Local"),
        windows_dir: PathBuf::from(r"C:\Windows"),
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["user-temp".to_string()],
        high_risk_confirmed: false,
    };

    let result = execute_selected_cleanup(&selection, &[], &roots).expect("cleanup result");

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].item_id, "user-temp");
    assert_eq!(result.results[0].status, "skipped");
}

#[test]
fn rejects_selected_cleanup_outside_c_drive_even_when_rule_matches() {
    let roots = ScanRoots {
        c_drive: PathBuf::from(r"C:\"),
        user_profile: PathBuf::from(r"D:\Users\Example"),
        local_app_data: PathBuf::from(r"D:\Users\Example\AppData\Local"),
        windows_dir: PathBuf::from(r"C:\Windows"),
    };
    let item = ScanItem {
        id: "user-temp".to_string(),
        title: "用户临时文件".to_string(),
        description: "临时文件".to_string(),
        source_category: SourceCategory::System,
        risk_level: RiskLevel::Recommended,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 10,
        default_selected: true,
        user_visible_path_hint: "当前用户临时目录".to_string(),
        technical_path: Some(r"D:\Users\Example\AppData\Local\Temp".to_string()),
        reasons: vec![],
        warnings: vec![],
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["user-temp".to_string()],
        high_risk_confirmed: false,
    };

    let result = execute_selected_cleanup(&selection, &[item], &roots).expect("cleanup result");

    assert_eq!(result.results[0].status, "failed");
    assert!(result.results[0].message.contains("C"));
}

#[test]
#[cfg(windows)]
fn delete_path_contents_skips_directory_symlinks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Temp");
    let outside = temp.path().join("Outside");
    let link = parent.join("linked-dir");
    fs::create_dir_all(&parent).expect("parent");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), "keep").expect("outside file");

    if symlink_dir(&outside, &link).is_err() {
        return;
    }

    delete_path_contents(&parent).expect("deleted");

    assert!(outside.join("keep.txt").exists());
    assert!(link.exists());
}
