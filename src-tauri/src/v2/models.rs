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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanRequest {
    pub selected_drives: Vec<String>,
    pub custom_folders: Vec<String>,
    pub file_types: Vec<DuplicateFileType>,
    pub custom_extensions: Vec<String>,
    pub include_suspected: bool,
    pub min_size_bytes: u64,
    pub protected_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanReport {
    pub strict_groups: Vec<DuplicateFileGroup>,
    pub suspected_groups: Vec<DuplicateFileGroup>,
    pub scanned_files: u64,
    pub skipped_locations: u64,
    pub total_reclaimable_bytes: u64,
    pub c_drive_reclaimable_bytes: u64,
    pub other_drive_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileGroup {
    pub group_id: String,
    pub strict_duplicate: bool,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub files: Vec<DuplicateFileEntry>,
    pub recommended_selection_reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileEntry {
    pub entry_id: String,
    pub display_name: String,
    pub drive: String,
    pub visible_location_hint: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub hash_fingerprint_id: String,
    pub selected: bool,
    pub protected: bool,
    pub recommended_action: DuplicateRecommendedAction,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCleanupRequest {
    pub groups: Vec<DuplicateCleanupGroupRequest>,
    pub protected_paths: Vec<String>,
    pub protected_override_confirmed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCleanupGroupRequest {
    pub group_id: String,
    pub files: Vec<DuplicateCleanupFileRequest>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCleanupFileRequest {
    pub path: String,
    pub selected: bool,
    pub protected: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCleanupReport {
    pub processed_files: u64,
    pub success_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
    pub freed_bytes: u64,
    pub c_drive_freed_bytes: u64,
    pub other_drive_freed_bytes: u64,
}
