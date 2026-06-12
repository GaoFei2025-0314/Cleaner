use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use tauri::{AppHandle, Emitter, Manager};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use walkdir::WalkDir;

use crate::v2::models::{
    LargeFileCategory, LargeFileItem, LargeFileScanReport, LargeFileScanRequest,
    OperationFinishedPayload, OperationModule, OperationProgressPayload, OperationStart,
    OperationStatus,
};
use crate::v2::operations::OperationRegistry;
use crate::v2::path_safety::{
    canonical_path_key, drive_label, is_protected_duplicate_path, selected_drive_to_root,
};

const DEFAULT_MIN_SIZE_BYTES: u64 = 500 * 1024 * 1024;
const OPERATION_PROGRESS_EVENT: &str = "cleaner-operation-progress";
const OPERATION_FINISHED_EVENT: &str = "cleaner-operation-finished";

#[derive(Debug, Clone)]
pub struct LargeFileBackendEntry {
    pub item_id: String,
    pub path: PathBuf,
    pub path_key: String,
    pub size_bytes: u64,
    pub drive: String,
    pub category: LargeFileCategory,
    pub protected: bool,
}

#[derive(Default)]
pub struct LargeFileRegistry {
    entries: Mutex<HashMap<String, LargeFileBackendEntry>>,
}

impl LargeFileRegistry {
    pub fn replace_entries(&self, entries: Vec<LargeFileBackendEntry>) {
        let mut registry = self
            .entries
            .lock()
            .expect("large file registry lock poisoned");
        registry.clear();
        for entry in entries {
            registry.insert(entry.item_id.clone(), entry);
        }
    }

    pub fn get(&self, item_id: &str) -> Option<LargeFileBackendEntry> {
        self.entries
            .lock()
            .expect("large file registry lock poisoned")
            .get(item_id)
            .cloned()
    }

    #[doc(hidden)]
    pub fn register_test_entry(&self, item_id: &str, path: &Path, protected: bool) {
        let size_bytes = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        self.entries
            .lock()
            .expect("large file registry lock poisoned")
            .insert(
                item_id.to_string(),
                LargeFileBackendEntry {
                    item_id: item_id.to_string(),
                    path: path.to_path_buf(),
                    path_key: canonical_path_key(path),
                    size_bytes,
                    drive: drive_label(path),
                    category: categorize_large_file(path),
                    protected,
                },
            );
    }
}

#[derive(Debug, Clone)]
struct ScanOutcome {
    report: LargeFileScanReport,
    backend_entries: Vec<LargeFileBackendEntry>,
}

pub fn scan_large_files(request: LargeFileScanRequest) -> Result<LargeFileScanReport, String> {
    let mut progress = |_| {};
    scan_large_files_internal(request, None, &mut progress, "").map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_large_files_with_progress_for_test<P>(
    request: LargeFileScanRequest,
    mut progress: P,
) -> Result<LargeFileScanReport, String>
where
    P: FnMut(OperationProgressPayload),
{
    scan_large_files_internal(request, None, &mut progress, "test").map(|outcome| outcome.report)
}

fn scan_large_files_internal<P>(
    request: LargeFileScanRequest,
    cancelled: Option<&AtomicBool>,
    progress: &mut P,
    operation_id: &str,
) -> Result<ScanOutcome, String>
where
    P: FnMut(OperationProgressPayload),
{
    let min_size_bytes = if request.min_size_bytes == 0 {
        DEFAULT_MIN_SIZE_BYTES
    } else {
        request.min_size_bytes
    };
    let mut items = Vec::new();
    let mut backend_entries = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut scanned_files = 0_u64;
    let mut skipped_locations = 0_u64;

    for root in scan_roots(&request) {
        check_cancelled(cancelled)?;
        if !root.exists() || !root.is_dir() {
            skipped_locations += 1;
            continue;
        }

        let mut entries = WalkDir::new(&root).follow_links(false).into_iter();
        while let Some(entry) = entries.next() {
            check_cancelled(cancelled)?;
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    skipped_locations += 1;
                    continue;
                }
            };
            let path = entry.path();
            if entry.file_type().is_dir() && path != root && should_skip_dir(path, &request) {
                skipped_locations += 1;
                entries.skip_current_dir();
                continue;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    skipped_locations += 1;
                    continue;
                }
            };
            scanned_files += 1;
            if metadata.len() < min_size_bytes {
                continue;
            }
            let path_key = canonical_path_key(path);
            if !seen_paths.insert(path_key.clone()) {
                continue;
            }
            let item_id = uuid::Uuid::new_v4().to_string();
            let drive = drive_label(path);
            let protected = is_protected_duplicate_path(path, &request.protected_paths);
            let category = categorize_large_file(path);
            let size_bytes = metadata.len();
            let recommended = large_file_is_recommended(path, protected, &request);
            let item = LargeFileItem {
                item_id: item_id.clone(),
                display_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string(),
                drive: drive.clone(),
                visible_location_hint: visible_location_hint(path),
                size_bytes,
                modified_at: format_modified_at(metadata.modified().ok()),
                category: category.clone(),
                selected: recommended,
                protected,
                recommended,
            };
            backend_entries.push(LargeFileBackendEntry {
                item_id,
                path: path.to_path_buf(),
                path_key,
                size_bytes,
                drive,
                category,
                protected,
            });
            items.push(item);
            emit_scan_progress(
                progress,
                operation_id,
                "scanning",
                scan_progress_percent(scanned_files),
                visible_location_hint(path),
                scanned_files,
                items.len() as u64,
                items.iter().map(|item| item.size_bytes).sum(),
            );
        }
    }

    items.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    let total_bytes = items.iter().map(|item| item.size_bytes).sum();
    let c_drive_bytes = items
        .iter()
        .filter(|item| item.drive.eq_ignore_ascii_case("C:"))
        .map(|item| item.size_bytes)
        .sum();

    Ok(ScanOutcome {
        report: LargeFileScanReport {
            items,
            scanned_files,
            skipped_locations,
            total_bytes,
            c_drive_bytes,
            other_drive_bytes: total_bytes - c_drive_bytes,
        },
        backend_entries,
    })
}

pub fn start_large_file_scan(
    app_handle: AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: LargeFileScanRequest,
) -> Result<OperationStart, String> {
    let token = operations.register();
    let operation_id = token.operation_id.clone();
    let operation_id_for_thread = operation_id.clone();
    let cancelled = token.cancelled.clone();

    std::thread::spawn(move || {
        emit_progress(
            &app_handle,
            progress_payload(
                &operation_id_for_thread,
                OperationModule::LargeFileScan,
                "scanning",
                5,
                String::new(),
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
        );

        let result = if cancelled.load(Ordering::Relaxed) {
            Err("操作已取消".to_string())
        } else {
            let mut progress = |payload| emit_progress(&app_handle, payload);
            scan_large_files_internal(
                request,
                Some(&cancelled),
                &mut progress,
                &operation_id_for_thread,
            )
        };

        let (status, payload, message) = match result {
            Ok(outcome) => {
                app_handle
                    .state::<LargeFileRegistry>()
                    .replace_entries(outcome.backend_entries);
                let report = outcome.report;
                emit_progress(
                    &app_handle,
                    progress_payload(
                        &operation_id_for_thread,
                        OperationModule::LargeFileScan,
                        "finished",
                        100,
                        String::new(),
                        report.scanned_files,
                        0,
                        report.items.len() as u64,
                        report.total_bytes,
                        0,
                        0,
                        0,
                        0,
                    ),
                );
                (
                    OperationStatus::Completed,
                    serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
                    None,
                )
            }
            Err(error) if error == "操作已取消" => (
                OperationStatus::Cancelled,
                serde_json::Value::Null,
                Some(error),
            ),
            Err(error) => (
                OperationStatus::Failed,
                serde_json::Value::Null,
                Some(error),
            ),
        };
        emit_finished(
            &app_handle,
            &operation_id_for_thread,
            OperationModule::LargeFileScan,
            status,
            payload,
            message,
        );
        app_handle
            .state::<OperationRegistry>()
            .finish(&operation_id_for_thread);
    });

    Ok(OperationStart { operation_id })
}

pub fn categorize_large_file(path: &Path) -> LargeFileCategory {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "avi" | "mkv" | "mov" | "mp4" | "mpeg" | "mpg" | "wmv" => LargeFileCategory::Video,
        "7z" | "gz" | "rar" | "tar" | "tgz" | "zip" => LargeFileCategory::Archive,
        "exe" | "msi" => LargeFileCategory::Installer,
        "dmg" | "img" | "iso" | "vhd" | "vhdx" => LargeFileCategory::DiskImage,
        "csv" | "doc" | "docx" | "pdf" | "ppt" | "pptx" | "txt" | "xls" | "xlsx" => {
            LargeFileCategory::Document
        }
        _ => LargeFileCategory::Other,
    }
}

pub fn visible_location_hint(path: &Path) -> String {
    let drive = drive_label(path);
    if drive.is_empty() {
        "文件夹".to_string()
    } else if is_c_drive_user_profile_file(path) {
        format!("{drive}\\...\\用户文件")
    } else {
        format!("{drive}\\...\\文件夹")
    }
}

#[doc(hidden)]
pub fn large_file_is_recommended_for_test(path: &Path, protected: bool) -> bool {
    let request = LargeFileScanRequest {
        selected_drives: Vec::new(),
        custom_folders: Vec::new(),
        min_size_bytes: 0,
        protected_paths: Vec::new(),
        skip_system_dirs: true,
        skip_program_dirs: true,
    };
    large_file_is_recommended(path, protected, &request)
}

#[doc(hidden)]
pub fn large_file_should_skip_dir_for_test(
    path: &Path,
    skip_system_dirs: bool,
    skip_program_dirs: bool,
) -> bool {
    let request = LargeFileScanRequest {
        selected_drives: Vec::new(),
        custom_folders: Vec::new(),
        min_size_bytes: 0,
        protected_paths: Vec::new(),
        skip_system_dirs,
        skip_program_dirs,
    };
    should_skip_dir(path, &request)
}

pub fn check_cancelled(cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if cancelled
        .map(|cancelled| cancelled.load(Ordering::Relaxed))
        .unwrap_or(false)
    {
        Err("操作已取消".to_string())
    } else {
        Ok(())
    }
}

fn scan_roots(request: &LargeFileScanRequest) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for drive in &request.selected_drives {
        if let Some(root) = selected_drive_to_root(drive) {
            roots.push(root);
        }
    }
    for folder in &request.custom_folders {
        let path = PathBuf::from(folder);
        if path.is_absolute() {
            roots.push(path);
        }
    }
    roots
}

fn should_skip_dir(path: &Path, request: &LargeFileScanRequest) -> bool {
    let key = canonical_path_key(path);
    (request.skip_system_dirs && key_is_same_or_child(&key, r"c:\windows"))
        || (request.skip_program_dirs
            && [
                r"c:\program files",
                r"c:\program files (x86)",
                r"c:\programdata",
            ]
            .iter()
            .any(|program_dir| key_is_same_or_child(&key, program_dir)))
}

fn large_file_is_recommended(path: &Path, protected: bool, request: &LargeFileScanRequest) -> bool {
    !protected && is_c_drive_user_profile_file(path) && !should_skip_dir(path, request)
}

fn key_is_same_or_child(path_key: &str, parent_key: &str) -> bool {
    if parent_key.is_empty() {
        return false;
    }
    path_key == parent_key
        || path_key
            .strip_prefix(parent_key)
            .is_some_and(|tail| tail.starts_with('\\'))
}

fn is_c_drive_user_profile_file(path: &Path) -> bool {
    let key = canonical_path_key(path);
    key.strip_prefix(r"c:\users\")
        .is_some_and(|tail| !tail.is_empty() && tail.contains('\\'))
}

fn scan_progress_percent(scanned_files: u64) -> u8 {
    (5 + scanned_files.min(94) as u8).min(99)
}

fn emit_scan_progress(
    progress: &mut impl FnMut(OperationProgressPayload),
    operation_id: &str,
    stage: &str,
    percent: u8,
    current_location_hint: String,
    scanned_files: u64,
    found_items: u64,
    found_bytes: u64,
) {
    progress(progress_payload(
        operation_id,
        OperationModule::LargeFileScan,
        stage,
        percent,
        current_location_hint,
        scanned_files,
        0,
        found_items,
        found_bytes,
        0,
        0,
        0,
        0,
    ));
}

pub fn progress_payload(
    operation_id: &str,
    module: OperationModule,
    stage: &str,
    percent: u8,
    current_location_hint: String,
    scanned_files: u64,
    found_groups: u64,
    found_items: u64,
    found_bytes: u64,
    processed_items: u64,
    success_count: u64,
    skipped_count: u64,
    failed_count: u64,
) -> OperationProgressPayload {
    OperationProgressPayload {
        operation_id: operation_id.to_string(),
        module,
        stage: stage.to_string(),
        percent: percent.min(100),
        current_location_hint,
        current_file_type: None,
        scanned_files,
        found_groups,
        found_items,
        found_bytes,
        processed_items,
        success_count,
        skipped_count,
        failed_count,
    }
}

fn emit_progress(app_handle: &AppHandle, payload: OperationProgressPayload) {
    let _ = app_handle.emit(OPERATION_PROGRESS_EVENT, payload);
}

pub fn emit_finished(
    app_handle: &AppHandle,
    operation_id: &str,
    module: OperationModule,
    status: OperationStatus,
    result: serde_json::Value,
    message: Option<String>,
) {
    let _ = app_handle.emit(
        OPERATION_FINISHED_EVENT,
        OperationFinishedPayload {
            operation_id: operation_id.to_string(),
            module,
            status,
            result,
            message,
        },
    );
}

fn format_modified_at(modified: Option<SystemTime>) -> String {
    modified
        .and_then(|modified| OffsetDateTime::from(modified).format(&Rfc3339).ok())
        .unwrap_or_else(now_rfc3339)
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "2026-06-12T00:00:00Z".to_string())
}
