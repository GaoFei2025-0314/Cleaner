use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SpaceBucket {
    ZeroToOneGb,
    OneToFiveGb,
    FiveGbPlus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsEvent {
    pub app_version: String,
    pub event_name: String,
    pub freed_space_bucket: SpaceBucket,
    pub categories: Vec<String>,
}

pub fn build_scan_analytics_event(
    app_version: &str,
    freed_bytes: u64,
    raw_categories: Vec<String>,
) -> AnalyticsEvent {
    AnalyticsEvent {
        app_version: app_version.to_string(),
        event_name: "cleanup_completed".to_string(),
        freed_space_bucket: bucket_for_bytes(freed_bytes),
        categories: raw_categories.into_iter().map(sanitize_category).collect(),
    }
}

fn bucket_for_bytes(bytes: u64) -> SpaceBucket {
    let one_gb = 1024_u64.pow(3);
    if bytes < one_gb {
        SpaceBucket::ZeroToOneGb
    } else if bytes < one_gb * 5 {
        SpaceBucket::OneToFiveGb
    } else {
        SpaceBucket::FiveGbPlus
    }
}

fn sanitize_category(input: String) -> String {
    match input.as_str() {
        "system" | "wechat" | "qq" | "workChat" | "cloudDrive" | "installersOldVersions"
        | "commonSoftware" => input,
        _ => "other".to_string(),
    }
}
