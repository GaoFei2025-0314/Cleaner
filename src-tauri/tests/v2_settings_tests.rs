use c_drive_cleaner::v2::settings::{default_settings, sanitize_custom_extensions};

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
