#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationModule {
    DuplicateScan,
    DuplicateCleanup,
    LargeFileScan,
    LargeFileMigration,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DuplicateFileType {
    Image,
    Document,
    Audio,
    Video,
    Archive,
    Custom,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DuplicateGroupKind {
    Strict,
    Suspected,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DuplicateRecommendedAction {
    Keep,
    Clean,
    ManualReview,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OriginalFilePolicy {
    KeepOriginal,
    MoveOriginalToRecycleBin,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LargeFileCategory {
    Video,
    Archive,
    Installer,
    DiskImage,
    Document,
    Other,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationStart {
    pub operation_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgressPayload {
    pub operation_id: String,
    pub module: OperationModule,
    pub stage: String,
    pub percent: u8,
    pub current_location_hint: String,
    pub current_file_type: Option<String>,
    pub scanned_files: u64,
    pub found_groups: u64,
    pub found_items: u64,
    pub found_bytes: u64,
    pub processed_items: u64,
    pub success_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OperationFinishedPayload {
    pub operation_id: String,
    pub module: OperationModule,
    pub status: OperationStatus,
    pub result: serde_json::Value,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanerSettings {
    pub protected_paths: Vec<String>,
    pub default_scan_drives: Vec<String>,
    pub duplicate_default_strategy: DuplicateDefaultStrategy,
    pub large_file_default_threshold_bytes: u64,
    pub history_retention_days: u32,
    pub desktop_shortcut_enabled: bool,
    pub c_drive_context_menu_enabled: bool,
    pub scheduled_scan_reminder_enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DuplicateDefaultStrategy {
    CDriveFirstKeepNewest,
    KeepNewest,
    KeepOldest,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub history_id: String,
    pub module: OperationModule,
    pub started_at: String,
    pub finished_at: String,
    pub total_bytes: u64,
    pub freed_bytes: u64,
    pub c_drive_freed_bytes: u64,
    pub other_drive_freed_bytes: u64,
    pub success_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
    pub error_categories: Vec<String>,
}
