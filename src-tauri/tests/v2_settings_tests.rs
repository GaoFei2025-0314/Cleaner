use std::fs;

use c_drive_cleaner::v2::models::DuplicateDefaultStrategy;
use c_drive_cleaner::v2::settings::{
    default_settings, get_cleaner_settings_at_path, sanitize_custom_extensions,
    save_cleaner_settings_at_path,
};

#[test]
fn default_settings_match_v02_product_decisions() {
    let settings = default_settings();
    assert_eq!(settings.default_scan_drives, vec!["C:"]);
    assert_eq!(
        settings.large_file_default_threshold_bytes,
        500 * 1024 * 1024
    );
    assert_eq!(settings.history_retention_days, 30);
    assert!(!settings.desktop_shortcut_enabled);
    assert!(!settings.c_drive_context_menu_enabled);
    assert!(!settings.scheduled_scan_reminder_enabled);
}

#[test]
fn custom_extensions_accept_safe_tokens_and_drop_unsafe_tokens() {
    assert_eq!(
        sanitize_custom_extensions("jpg, .mp4;zip;*.exe;中文;tar.gz"),
        vec!["jpg", "mp4", "zip", "tar.gz"]
    );
}

#[test]
fn missing_settings_file_returns_defaults() {
    let temp_dir = tempfile::tempdir().unwrap();
    let settings_path = temp_dir.path().join("missing").join("settings.json");

    assert_eq!(
        get_cleaner_settings_at_path(&settings_path).unwrap(),
        default_settings()
    );
}

#[test]
fn saving_settings_creates_parent_directory_and_can_be_read_back() {
    let temp_dir = tempfile::tempdir().unwrap();
    let settings_path = temp_dir.path().join("nested").join("settings.json");
    let mut settings = default_settings();
    settings.protected_paths = vec!["D:\\Backups".to_string()];
    settings.duplicate_default_strategy = DuplicateDefaultStrategy::KeepOldest;
    settings.scheduled_scan_reminder_enabled = true;

    let saved = save_cleaner_settings_at_path(&settings_path, settings.clone()).unwrap();

    assert_eq!(saved, settings);
    assert!(settings_path.exists());
    assert_eq!(
        get_cleaner_settings_at_path(&settings_path).unwrap(),
        settings
    );
}

#[test]
fn invalid_settings_json_returns_path_free_chinese_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    let settings_path = temp_dir.path().join("settings.json");
    fs::write(&settings_path, "not json").unwrap();

    let error = get_cleaner_settings_at_path(&settings_path).unwrap_err();

    assert_eq!(error, "设置文件格式无效");
    assert!(!error.contains(&temp_dir.path().display().to_string()));
}
