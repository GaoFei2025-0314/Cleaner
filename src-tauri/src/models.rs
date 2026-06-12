use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    Recommended,
    Optional,
    HighRisk,
    NotCleanable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourceCategory {
    System,
    CommonSoftware,
    Wechat,
    Qq,
    WorkChat,
    CloudDrive,
    InstallersOldVersions,
    OtherLarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CleanupAction {
    DirectDelete,
    ExplainOnly,
    BlockedByProcess,
    BlockedByConfigReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveSummary {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_category: SourceCategory,
    pub risk_level: RiskLevel,
    pub cleanup_action: CleanupAction,
    pub estimated_bytes: u64,
    pub default_selected: bool,
    pub user_visible_path_hint: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub technical_path: Option<String>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub drive_summary: DriveSummary,
    pub items: Vec<ScanItem>,
    pub partial: bool,
    pub scan_started_at: String,
    pub scan_finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSelection {
    pub selected_item_ids: Vec<String>,
    pub high_risk_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemResult {
    pub item_id: String,
    pub status: String,
    pub freed_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub results: Vec<CleanupItemResult>,
    pub total_freed_bytes: u64,
    pub finished_at: String,
}
