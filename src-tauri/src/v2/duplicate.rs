use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use walkdir::WalkDir;

use crate::v2::history::append_history_entry;
use crate::v2::models::{
    DuplicateCleanupReport, DuplicateCleanupRequest, DuplicateFileEntry, DuplicateFileGroup,
    DuplicateFileType, DuplicateRecommendedAction, DuplicateScanReport, DuplicateScanRequest,
    HistoryEntry, OperationFinishedPayload, OperationModule, OperationProgressPayload,
    OperationStart, OperationStatus,
};
use crate::v2::operations::OperationRegistry;
use crate::v2::path_safety::{
    canonical_path_key, drive_label, is_protected_duplicate_path, selected_drive_to_root,
    should_skip_scan_location,
};
use crate::v2::recycle_bin::{RecycleBin, SystemRecycleBin};
use crate::v2::settings::sanitize_custom_extensions;

const SUSPECTED_SIZE_DELTA_PERCENT: u64 = 2;
const OPERATION_PROGRESS_EVENT: &str = "cleaner-operation-progress";
const OPERATION_FINISHED_EVENT: &str = "cleaner-operation-finished";

#[derive(Debug, Clone)]
struct CandidateFile {
    path: PathBuf,
    path_key: String,
    display_name: String,
    drive: String,
    visible_location_hint: String,
    size_bytes: u64,
    modified_at: String,
    protected: bool,
    normalized_stem: String,
}

#[derive(Debug, Clone)]
struct DuplicateBackendEntry {
    entry_id: String,
    group_id: String,
    path: PathBuf,
    path_key: String,
    protected: bool,
}

#[derive(Debug, Clone)]
struct DuplicateScanOutcome {
    report: DuplicateScanReport,
    backend_entries: Vec<DuplicateBackendEntry>,
}

#[derive(Debug, Clone)]
struct ResolvedCleanupEntry {
    entry: DuplicateBackendEntry,
    request_protected: bool,
}

#[derive(Default)]
pub struct DuplicateEntryRegistry {
    entries: Mutex<HashMap<String, DuplicateBackendEntry>>,
}

impl DuplicateEntryRegistry {
    fn replace_entries(&self, entries: Vec<DuplicateBackendEntry>) {
        let mut registry = self
            .entries
            .lock()
            .expect("duplicate entry registry lock poisoned");
        registry.clear();
        for entry in entries {
            registry.insert(entry.entry_id.clone(), entry);
        }
    }

    fn get(&self, entry_id: &str) -> Option<DuplicateBackendEntry> {
        self.entries
            .lock()
            .expect("duplicate entry registry lock poisoned")
            .get(entry_id)
            .cloned()
    }

    #[doc(hidden)]
    pub fn register_test_entry(&self, group_id: &str, entry_id: &str, path: &Path, protected: bool) {
        self.entries
            .lock()
            .expect("duplicate entry registry lock poisoned")
            .insert(
                entry_id.to_string(),
                DuplicateBackendEntry {
                    entry_id: entry_id.to_string(),
                    group_id: group_id.to_string(),
                    path: path.to_path_buf(),
                    path_key: canonical_path_key(path),
                    protected,
                },
            );
    }
}

pub fn scan_duplicate_files(request: DuplicateScanRequest) -> Result<DuplicateScanReport, String> {
    scan_duplicate_files_internal(request, |_| {}, None).map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_duplicate_files_with_before_hash_for_test<F>(
    request: DuplicateScanRequest,
    before_hash: F,
) -> Result<DuplicateScanReport, String>
where
    F: FnMut(&Path),
{
    scan_duplicate_files_internal(request, before_hash, None).map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_duplicate_files_cancellable_for_test<F>(
    request: DuplicateScanRequest,
    cancelled: &AtomicBool,
    before_hash: F,
) -> Result<DuplicateScanReport, String>
where
    F: FnMut(&Path),
{
    scan_duplicate_files_internal(request, before_hash, Some(cancelled)).map(|outcome| outcome.report)
}

fn scan_duplicate_files_internal<F>(
    request: DuplicateScanRequest,
    mut before_hash: F,
    cancelled: Option<&AtomicBool>,
) -> Result<DuplicateScanOutcome, String>
where
    F: FnMut(&Path),
{
    let scan_roots = scan_roots(&request);
    let allowed_extensions = allowed_extensions(&request.file_types, &request.custom_extensions);
    let mut skipped_locations = 0_u64;
    let mut candidates_by_size: BTreeMap<u64, Vec<CandidateFile>> = BTreeMap::new();
    let mut seen_paths = HashSet::new();
    let mut scanned_files = 0_u64;

    for root in scan_roots {
        check_cancelled(cancelled)?;
        if should_skip_scan_location(&root, &request.protected_paths) {
            skipped_locations += 1;
            continue;
        }

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

            if entry.file_type().is_dir()
                && path != root
                && should_skip_scan_location(path, &request.protected_paths)
            {
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
            let size_bytes = metadata.len();
            if size_bytes < request.min_size_bytes {
                continue;
            }
            if !extension_is_allowed(path, &allowed_extensions) {
                continue;
            }
            let path_key = canonical_path_key(path);
            if !seen_paths.insert(path_key.clone()) {
                continue;
            }

            scanned_files += 1;
            let candidate = CandidateFile {
                path: path.to_path_buf(),
                path_key,
                display_name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_string(),
                drive: drive_label(path),
                visible_location_hint: visible_location_hint(path),
                size_bytes,
                modified_at: format_modified_at(metadata.modified().ok()),
                protected: is_protected_duplicate_path(path, &request.protected_paths),
                normalized_stem: normalized_stem(path),
            };
            candidates_by_size
                .entry(size_bytes)
                .or_default()
                .push(candidate);
        }
    }

    let (mut strict_groups, skipped_hashes, backend_entries) =
        strict_duplicate_groups(&candidates_by_size, &mut before_hash, cancelled)?;
    skipped_locations += skipped_hashes;
    for group in &mut strict_groups {
        apply_duplicate_recommendations(group, &request.protected_paths);
    }

    let suspected_groups = if request.include_suspected {
        suspected_duplicate_groups(candidates_by_size.values().flatten().cloned().collect())
    } else {
        Vec::new()
    };

    let (total_reclaimable_bytes, c_drive_reclaimable_bytes, other_drive_reclaimable_bytes) =
        reclaimable_totals(&strict_groups);

    Ok(DuplicateScanOutcome {
        report: DuplicateScanReport {
            strict_groups,
            suspected_groups,
            scanned_files,
            skipped_locations,
            total_reclaimable_bytes,
            c_drive_reclaimable_bytes,
            other_drive_reclaimable_bytes,
        },
        backend_entries,
    })
}

pub fn apply_duplicate_recommendations(
    group: &mut DuplicateFileGroup,
    _protected_paths: &[String],
) {
    for file in &mut group.files {
        file.selected = false;
        file.recommended_action = DuplicateRecommendedAction::ManualReview;
    }

    if !group.strict_duplicate {
        group.reclaimable_bytes = 0;
        group.recommended_selection_reason =
            "Suspected duplicates require manual review".to_string();
        return;
    }

    let keep_index = preferred_keep_index(&group.files);
    let Some(keep_index) = keep_index else {
        group.recommended_selection_reason = "No safe cleanup recommendation".to_string();
        group.reclaimable_bytes = 0;
        return;
    };

    let mut reclaimable_bytes = 0_u64;
    for (index, file) in group.files.iter_mut().enumerate() {
        if index == keep_index || file.protected {
            file.selected = false;
            file.recommended_action = DuplicateRecommendedAction::Keep;
        } else {
            file.selected = true;
            file.recommended_action = DuplicateRecommendedAction::Clean;
            reclaimable_bytes += file.size_bytes;
        }
    }

    group.reclaimable_bytes = reclaimable_bytes;
    group.recommended_selection_reason =
        "Keep a non-C drive copy when available; clean duplicate C drive copies first".to_string();
}

pub fn run_duplicate_cleanup_with_recycle_bin(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
) -> DuplicateCleanupReport {
    run_duplicate_cleanup_internal(request, registry, recycle_bin, None)
        .unwrap_or_else(|report| report)
}

fn run_duplicate_cleanup_internal(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: Option<&AtomicBool>,
) -> Result<DuplicateCleanupReport, DuplicateCleanupReport> {
    let protected_paths = request.protected_paths.clone();
    let protected_override_confirmed = request.protected_override_confirmed;
    let mut report = DuplicateCleanupReport {
        processed_files: 0,
        success_count: 0,
        skipped_count: 0,
        failed_count: 0,
        freed_bytes: 0,
        c_drive_freed_bytes: 0,
        other_drive_freed_bytes: 0,
    };

    for group in request.groups {
        if check_cancelled(cancelled).is_err() {
            return Err(report);
        }

        let mut selected = Vec::new();
        let mut retained = Vec::new();
        let mut seen_selected_ids = HashSet::new();
        let mut seen_retained_ids = HashSet::new();

        for file in &group.files {
            let Some(entry) = registry.get(&file.entry_id) else {
                report.failed_count += 1;
                continue;
            };
            if entry.group_id != group.group_id {
                report.skipped_count += 1;
                continue;
            }
            let resolved = ResolvedCleanupEntry {
                entry,
                request_protected: file.protected,
            };
            if file.selected {
                if seen_selected_ids.insert(file.entry_id.clone()) {
                    selected.push(resolved);
                }
            } else if seen_retained_ids.insert(file.entry_id.clone()) {
                retained.push(resolved);
            }
        }

        if selected.is_empty() {
            continue;
        }
        if retained.is_empty() {
            report.skipped_count += selected.len() as u64;
            continue;
        }

        let retained_ids = retained
            .iter()
            .map(|entry| entry.entry.entry_id.clone())
            .collect::<HashSet<_>>();
        let retained_path_keys = retained
            .iter()
            .map(|entry| entry.entry.path_key.clone())
            .collect::<HashSet<_>>();
        let mut selected_path_keys = HashSet::new();
        for file in selected {
            if check_cancelled(cancelled).is_err() {
                return Err(report);
            }
            report.processed_files += 1;
            let path = &file.entry.path;
            if !path.exists() {
                report.failed_count += 1;
                continue;
            }
            if retained_ids.contains(&file.entry.entry_id)
                || retained_path_keys.contains(&file.entry.path_key)
                || !selected_path_keys.insert(file.entry.path_key.clone())
            {
                report.skipped_count += 1;
                continue;
            }
            if (file.request_protected
                || file.entry.protected
                || is_protected_duplicate_path(path, &protected_paths))
                && !protected_override_confirmed
            {
                report.skipped_count += 1;
                continue;
            }

            let Ok((size_bytes, hash)) = file_fingerprint(path) else {
                report.failed_count += 1;
                continue;
            };
            let retained_fingerprints = retained
                .iter()
                .filter_map(|entry| {
                    file_fingerprint(&entry.entry.path)
                        .ok()
                        .map(|fingerprint| (entry.entry.path_key.clone(), fingerprint))
                })
                .collect::<Vec<_>>();
            if !retained_fingerprints.iter().any(|(retained_path_key, fingerprint)| {
                retained_path_key != &file.entry.path_key && fingerprint == &(size_bytes, hash.clone())
            }) {
                report.skipped_count += 1;
                continue;
            }

            match recycle_bin.move_to_recycle_bin(path) {
                Ok(()) => {
                    report.success_count += 1;
                    report.freed_bytes += size_bytes;
                    if drive_label(path).eq_ignore_ascii_case("C:") {
                        report.c_drive_freed_bytes += size_bytes;
                    } else {
                        report.other_drive_freed_bytes += size_bytes;
                    }
                }
                Err(_) => report.failed_count += 1,
            }
        }
    }

    Ok(report)
}

pub fn start_duplicate_scan(
    app_handle: AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: DuplicateScanRequest,
) -> Result<OperationStart, String> {
    let token = operations.register();
    let operation_id = token.operation_id.clone();
    let operation_id_for_thread = operation_id.clone();
    let cancelled = token.cancelled.clone();

    std::thread::spawn(move || {
        emit_progress(
            &app_handle,
            &operation_id_for_thread,
            OperationModule::DuplicateScan,
            "scanning",
            5,
            0,
            0,
            0,
            0,
        );

        let result = if cancelled.load(Ordering::Relaxed) {
            Err("操作已取消".to_string())
        } else {
            scan_duplicate_files_internal(request, |_| {}, Some(&cancelled))
        };

        let (status, payload, message) = match result {
            Ok(outcome) => {
                let report = outcome.report;
                app_handle
                    .state::<DuplicateEntryRegistry>()
                    .replace_entries(outcome.backend_entries);
                emit_progress(
                    &app_handle,
                    &operation_id_for_thread,
                    OperationModule::DuplicateScan,
                    "finished",
                    100,
                    report.scanned_files,
                    report.strict_groups.len() as u64,
                    report
                        .strict_groups
                        .iter()
                        .map(|group| group.files.len() as u64)
                        .sum(),
                    report.total_reclaimable_bytes,
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
            OperationModule::DuplicateScan,
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

pub fn start_duplicate_cleanup(
    app_handle: AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: DuplicateCleanupRequest,
) -> Result<OperationStart, String> {
    let token = operations.register();
    let operation_id = token.operation_id.clone();
    let operation_id_for_thread = operation_id.clone();
    let cancelled = token.cancelled.clone();

    std::thread::spawn(move || {
        emit_progress(
            &app_handle,
            &operation_id_for_thread,
            OperationModule::DuplicateCleanup,
            "cleaning",
            5,
            0,
            0,
            0,
            0,
        );

        let (status, payload, message) =
            if cancelled.load(Ordering::Relaxed) {
                (
                    OperationStatus::Cancelled,
                    serde_json::Value::Null,
                    Some("操作已取消".to_string()),
                )
            } else {
                let started_at = now_rfc3339();
                let cleanup_result = {
                    let registry = app_handle.state::<DuplicateEntryRegistry>();
                    run_duplicate_cleanup_internal(
                        request,
                        &registry,
                        &SystemRecycleBin,
                        Some(&cancelled),
                    )
                };
                let cancelled_after_cleanup = cancelled.load(Ordering::Relaxed);
                let report = match cleanup_result {
                    Ok(report) | Err(report) => report,
                };
                let finished_at = now_rfc3339();
                let _ = append_history_entry(
                    &app_handle,
                    HistoryEntry {
                        history_id: uuid::Uuid::new_v4().to_string(),
                        module: OperationModule::DuplicateCleanup,
                        started_at,
                        finished_at,
                        total_bytes: report.freed_bytes,
                        freed_bytes: report.freed_bytes,
                        c_drive_freed_bytes: report.c_drive_freed_bytes,
                        other_drive_freed_bytes: report.other_drive_freed_bytes,
                        success_count: report.success_count,
                        skipped_count: report.skipped_count,
                        failed_count: report.failed_count,
                        error_categories: cleanup_error_categories(&report),
                    },
                );

                emit_progress(
                    &app_handle,
                    &operation_id_for_thread,
                    OperationModule::DuplicateCleanup,
                    "finished",
                    100,
                    0,
                    0,
                    0,
                    report.freed_bytes,
                );
                (
                    if cancelled_after_cleanup {
                        OperationStatus::Cancelled
                    } else {
                        OperationStatus::Completed
                    },
                    serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
                    cancelled_after_cleanup.then(|| "操作已取消".to_string()),
                )
            };

        emit_finished(
            &app_handle,
            &operation_id_for_thread,
            OperationModule::DuplicateCleanup,
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

fn scan_roots(request: &DuplicateScanRequest) -> Vec<PathBuf> {
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

fn allowed_extensions(
    file_types: &[DuplicateFileType],
    custom_extensions: &[String],
) -> Option<HashSet<String>> {
    if file_types.is_empty() && custom_extensions.is_empty() {
        return None;
    }

    let mut extensions = HashSet::new();
    for file_type in file_types {
        for extension in extensions_for_file_type(file_type) {
            extensions.insert(extension.to_string());
        }
    }
    let sanitized_custom = sanitize_custom_extensions(&custom_extensions.join(","));
    extensions.extend(sanitized_custom);
    Some(extensions)
}

fn extensions_for_file_type(file_type: &DuplicateFileType) -> &'static [&'static str] {
    match file_type {
        DuplicateFileType::Image => &["bmp", "gif", "jpeg", "jpg", "png", "webp"],
        DuplicateFileType::Document => &[
            "csv", "doc", "docx", "json", "log", "pdf", "ppt", "pptx", "txt", "xls", "xlsx", "xml",
        ],
        DuplicateFileType::Audio => &["flac", "m4a", "mp3", "ogg", "wav", "wma"],
        DuplicateFileType::Video => &["avi", "mkv", "mov", "mp4", "mpeg", "mpg", "wmv"],
        DuplicateFileType::Archive => &["7z", "gz", "rar", "tar", "tgz", "zip"],
        DuplicateFileType::Custom => &[],
    }
}

fn extension_is_allowed(path: &Path, allowed_extensions: &Option<HashSet<String>>) -> bool {
    let Some(allowed_extensions) = allowed_extensions else {
        return true;
    };
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| allowed_extensions.contains(&extension.to_ascii_lowercase()))
        .unwrap_or(false)
}

fn strict_duplicate_groups<F>(
    candidates_by_size: &BTreeMap<u64, Vec<CandidateFile>>,
    before_hash: &mut F,
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<DuplicateFileGroup>, u64, Vec<DuplicateBackendEntry>), String>
where
    F: FnMut(&Path),
{
    let mut groups = Vec::new();
    let mut backend_entries = Vec::new();
    let mut skipped_locations = 0_u64;

    for candidates in candidates_by_size.values().filter(|files| files.len() >= 2) {
        let mut by_hash: HashMap<String, Vec<CandidateFile>> = HashMap::new();
        for candidate in candidates {
            check_cancelled(cancelled)?;
            before_hash(&candidate.path);
            let hash = match hash_file_with_cancel(&candidate.path, cancelled) {
                Ok(hash) => hash,
                Err(error) if cancellation_requested(cancelled) => return Err(error),
                Err(_) => {
                    skipped_locations += 1;
                    continue;
                }
            };
            by_hash.entry(hash).or_default().push(candidate.clone());
        }

        for files in by_hash.into_values().filter(|files| files.len() >= 2) {
            let fingerprint_id = uuid::Uuid::new_v4().to_string();
            let group_id = uuid::Uuid::new_v4().to_string();
            let total_bytes = files.iter().map(|file| file.size_bytes).sum();
            let mut entries = files
                .into_iter()
                .map(|file| {
                    let entry_id = uuid::Uuid::new_v4().to_string();
                    backend_entries.push(DuplicateBackendEntry {
                        entry_id: entry_id.clone(),
                        group_id: group_id.clone(),
                        path: file.path.clone(),
                        path_key: file.path_key.clone(),
                        protected: file.protected,
                    });
                    duplicate_entry(file, entry_id, fingerprint_id.clone())
                })
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.display_name.cmp(&right.display_name));
            groups.push(DuplicateFileGroup {
                group_id,
                strict_duplicate: true,
                total_bytes,
                reclaimable_bytes: 0,
                files: entries,
                recommended_selection_reason:
                    "Strict duplicates share size and content fingerprint".to_string(),
            });
        }
    }

    Ok((groups, skipped_locations, backend_entries))
}

fn suspected_duplicate_groups(candidates: Vec<CandidateFile>) -> Vec<DuplicateFileGroup> {
    let mut candidates_by_stem: BTreeMap<String, Vec<CandidateFile>> = BTreeMap::new();
    for candidate in candidates {
        if candidate.normalized_stem.is_empty() {
            continue;
        }
        candidates_by_stem
            .entry(candidate.normalized_stem.clone())
            .or_default()
            .push(candidate);
    }

    let mut groups = Vec::new();
    for mut files in candidates_by_stem
        .into_values()
        .filter(|files| files.len() >= 2)
    {
        files.sort_by_key(|file| file.size_bytes);
        let smallest = files
            .first()
            .map(|file| file.size_bytes)
            .unwrap_or_default();
        let largest = files.last().map(|file| file.size_bytes).unwrap_or_default();
        if !within_suspected_delta(smallest, largest) {
            continue;
        }

        let total_bytes = files.iter().map(|file| file.size_bytes).sum();
        let mut entries = files
            .into_iter()
            .map(|file| {
                let mut entry = duplicate_entry(
                    file,
                    uuid::Uuid::new_v4().to_string(),
                    uuid::Uuid::new_v4().to_string(),
                );
                entry.recommended_action = DuplicateRecommendedAction::ManualReview;
                entry
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        groups.push(DuplicateFileGroup {
            group_id: uuid::Uuid::new_v4().to_string(),
            strict_duplicate: false,
            total_bytes,
            reclaimable_bytes: 0,
            files: entries,
            recommended_selection_reason: "Suspected duplicates require manual review".to_string(),
        });
    }

    groups
}

fn duplicate_entry(
    file: CandidateFile,
    entry_id: String,
    fingerprint_id: String,
) -> DuplicateFileEntry {
    DuplicateFileEntry {
        entry_id,
        display_name: file.display_name,
        drive: file.drive,
        visible_location_hint: file.visible_location_hint,
        size_bytes: file.size_bytes,
        modified_at: file.modified_at,
        hash_fingerprint_id: fingerprint_id,
        selected: false,
        protected: file.protected,
        recommended_action: DuplicateRecommendedAction::ManualReview,
    }
}

fn preferred_keep_index(files: &[DuplicateFileEntry]) -> Option<usize> {
    files
        .iter()
        .enumerate()
        .filter(|(_, file)| !file.protected && !file.drive.eq_ignore_ascii_case("C:"))
        .max_by(|(_, left), (_, right)| left.modified_at.cmp(&right.modified_at))
        .map(|(index, _)| index)
        .or_else(|| {
            files
                .iter()
                .enumerate()
                .filter(|(_, file)| !file.protected)
                .max_by(|(_, left), (_, right)| left.modified_at.cmp(&right.modified_at))
                .map(|(index, _)| index)
        })
}

fn reclaimable_totals(groups: &[DuplicateFileGroup]) -> (u64, u64, u64) {
    let mut total = 0_u64;
    let mut c_drive = 0_u64;
    let mut other_drive = 0_u64;

    for file in groups.iter().flat_map(|group| &group.files) {
        if file.recommended_action != DuplicateRecommendedAction::Clean {
            continue;
        }
        total += file.size_bytes;
        if file.drive.eq_ignore_ascii_case("C:") {
            c_drive += file.size_bytes;
        } else {
            other_drive += file.size_bytes;
        }
    }

    (total, c_drive, other_drive)
}

fn hash_file(path: &Path) -> Result<String, String> {
    hash_file_with_cancel(path, None)
}

fn hash_file_with_cancel(path: &Path, cancelled: Option<&AtomicBool>) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|_| "无法读取重复文件".to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancelled(cancelled)?;
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|_| "无法读取重复文件".to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn check_cancelled(cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if cancellation_requested(cancelled) {
        Err("操作已取消".to_string())
    } else {
        Ok(())
    }
}

fn cancellation_requested(cancelled: Option<&AtomicBool>) -> bool {
    cancelled
        .map(|cancelled| cancelled.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn file_fingerprint(path: &Path) -> Result<(u64, String), String> {
    let metadata = fs::metadata(path).map_err(|_| "无法读取重复文件".to_string())?;
    Ok((metadata.len(), hash_file(path)?))
}

fn visible_location_hint(path: &Path) -> String {
    let drive = drive_label(path);
    let parent_name = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("folder");
    if drive.is_empty() {
        parent_name.to_string()
    } else {
        format!("{drive}\\...\\{parent_name}")
    }
}

fn normalized_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn within_suspected_delta(smallest: u64, largest: u64) -> bool {
    if smallest == 0 {
        return largest == 0;
    }
    largest.saturating_sub(smallest) * 100 <= smallest * SUSPECTED_SIZE_DELTA_PERCENT
}

fn format_modified_at(modified: Option<SystemTime>) -> String {
    modified
        .and_then(|modified| OffsetDateTime::from(modified).format(&Rfc3339).ok())
        .unwrap_or_else(now_rfc3339)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "2026-06-12T00:00:00Z".to_string())
}

fn cleanup_error_categories(report: &DuplicateCleanupReport) -> Vec<String> {
    let mut categories = Vec::new();
    if report.skipped_count > 0 {
        categories.push("跳过受保护或已变化文件".to_string());
    }
    if report.failed_count > 0 {
        categories.push("回收站移动失败".to_string());
    }
    if categories.is_empty() {
        categories.push("无错误".to_string());
    }
    categories
}

fn emit_progress(
    app_handle: &AppHandle,
    operation_id: &str,
    module: OperationModule,
    stage: &str,
    percent: u8,
    scanned_files: u64,
    found_groups: u64,
    found_items: u64,
    found_bytes: u64,
) {
    let _ = app_handle.emit(
        OPERATION_PROGRESS_EVENT,
        OperationProgressPayload {
            operation_id: operation_id.to_string(),
            module,
            stage: stage.to_string(),
            percent,
            current_location_hint: String::new(),
            current_file_type: None,
            scanned_files,
            found_groups,
            found_items,
            found_bytes,
            processed_items: 0,
            success_count: 0,
            skipped_count: 0,
            failed_count: 0,
        },
    );
}

fn emit_finished(
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
