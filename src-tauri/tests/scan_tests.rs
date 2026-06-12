use std::fs;

use c_drive_cleaner::models::{CleanupAction, DriveSummary, RiskLevel};
use c_drive_cleaner::paths::ScanRoots;
use c_drive_cleaner::scan::scan_with_roots;

#[test]
fn user_temp_stays_selectable_when_config_references_a_child_temp_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let local_app_data = user_profile.join("AppData").join("Local");
    let user_temp = local_app_data.join("Temp");
    let codex_dir = user_profile.join(".codex");

    fs::create_dir_all(&user_temp).expect("user temp");
    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::write(user_temp.join("old.tmp"), "abc").expect("temp file");
    fs::write(
        codex_dir.join("config.toml"),
        format!(
            "clipboard = '{}'",
            user_temp.join("codex-clipboard-image.png").display()
        ),
    )
    .expect("config");

    let report = scan_with_roots(
        &ScanRoots {
            c_drive: temp.path().to_path_buf(),
            user_profile,
            local_app_data,
            windows_dir: temp.path().join("Windows"),
        },
        DriveSummary {
            drive: "C:".to_string(),
            total_bytes: 1000,
            free_bytes: 500,
        },
    );
    let user_temp_item = report
        .items
        .iter()
        .find(|item| item.id == "user-temp")
        .expect("user-temp item");

    assert_eq!(user_temp_item.risk_level, RiskLevel::Recommended);
    assert_eq!(user_temp_item.cleanup_action, CleanupAction::DirectDelete);
    assert!(user_temp_item.default_selected);
}
