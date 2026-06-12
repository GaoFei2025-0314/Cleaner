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
