use c_drive_cleaner::analytics::{build_scan_analytics_event, SpaceBucket};

#[test]
fn analytics_event_uses_space_buckets() {
    let event = build_scan_analytics_event("0.1.0", 6_000_000_000, vec!["system".to_string()]);
    assert_eq!(event.freed_space_bucket, SpaceBucket::FiveGbPlus);
}

#[test]
fn analytics_event_does_not_include_paths_or_usernames() {
    let event = build_scan_analytics_event(
        "0.1.0",
        42,
        vec!["C:\\Users\\Administrator\\AppData\\Local\\Temp\\abc.txt".to_string()],
    );
    let encoded = serde_json::to_string(&event).expect("json");
    assert!(!encoded.contains("Administrator"));
    assert!(!encoded.contains("AppData"));
    assert!(!encoded.contains("abc.txt"));
}
