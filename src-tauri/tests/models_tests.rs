use c_drive_cleaner::models::{CleanupAction, RiskLevel, ScanItem, SourceCategory};

#[test]
fn scan_item_serialization_does_not_expose_technical_path() {
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
        technical_path: Some(r"C:\Users\Administrator\AppData\Local\Temp".to_string()),
        reasons: vec![],
        warnings: vec![],
    };

    let encoded = serde_json::to_string(&item).expect("json");

    assert!(!encoded.contains("technicalPath"));
    assert!(!encoded.contains("Administrator"));
    assert!(!encoded.contains("AppData"));
}
