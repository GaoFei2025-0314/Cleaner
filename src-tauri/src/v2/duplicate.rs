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
    let mut progress = |_| {};
    scan_duplicate_files_internal(request, |_| {}, None, &mut progress, "")
        .map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_duplicate_files_with_backend_settings_for_test(
    request: DuplicateScanRequest,
    backend_protected_paths: Result<Vec<String>, String>,
) -> Result<DuplicateScanReport, String> {
    let mut progress = |_| {};
    scan_duplicate_files_with_backend_settings(
        request,
        backend_protected_paths,
        None,
        &mut progress,
        "test",
    )
    .map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_duplicate_files_with_before_hash_for_test<F>(
    request: DuplicateScanRequest,
    before_hash: F,
) -> Result<DuplicateScanReport, String>
where
    F: FnMut(&Path),
{
    let mut progress = |_| {};
    scan_duplicate_files_internal(request, before_hash, None, &mut progress, "")
        .map(|outcome| outcome.report)
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
    let mut progress = |_| {};
    scan_duplicate_files_internal(request, before_hash, Some(cancelled), &mut progress, "")
        .map(|outcome| outcome.report)
}

#[doc(hidden)]
pub fn scan_duplicate_files_with_progress_for_test<P>(
    request: DuplicateScanRequest,
    mut progress: P,
) -> Result<DuplicateScanReport, String>
where
    P: FnMut(OperationProgressPayload),
{
    scan_duplicate_files_internal(request, |_| {}, None, &mut progress, "test")
        .map(|outcome| outcome.report)
}

fn scan_duplicate_files_internal<F, P>(
    request: DuplicateScanRequest,
    mut before_hash: F,
    cancelled: Option<&AtomicBool>,
    progress: &mut P,
    operation_id: &str,
) -> Result<DuplicateScanOutcome, String>
where
    F: FnMut(&Path),
    P: FnMut(OperationProgressPayload),
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
                drive: drive_label(path),
                visible_location_hint: visible_location_hint(path),
                size_bytes,
                modified_at: format_modified_at(metadata.modified().ok()),
                protected: is_protected_duplicate_path(path, &request.protected_paths),
                normalized_stem: normalized_stem(path),
            };
            emit_scan_progress(
                progress,
                operation_id,
                "scanning",
                scan_progress_percent(scanned_files, 10),
                candidate.visible_location_hint.clone(),
                scanned_files,
                0,
                0,
                0,
            );
            candidates_by_size
                .entry(size_bytes)
                .or_default()
                .push(candidate);
        }
    }

    let (mut strict_groups, skipped_hashes, backend_entries) =
        strict_duplicate_groups(
            &candidates_by_size,
            &mut before_hash,
            cancelled,
            progress,
            operation_id,
            scanned_files,
        )?;
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

fn scan_duplicate_files_with_backend_settings<P>(
    request: DuplicateScanRequest,
    backend_protected_paths: Result<Vec<String>, String>,
    cancelled: Option<&AtomicBool>,
    progress: &mut P,
    operation_id: &str,
) -> Result<DuplicateScanOutcome, String>
where
    P: FnMut(OperationProgressPayload),
{
    let backend_protected_paths =
        backend_protected_paths.map_err(|_| scan_settings_unavailable_error())?;
    scan_duplicate_files_internal(
        request_with_backend_protected_paths(request, backend_protected_paths),
        |_| {},
        cancelled,
        progress,
        operation_id,
    )
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
    let mut progress = |_| {};
    run_duplicate_cleanup_internal(
        request,
        registry,
        recycle_bin,
        &[],
        None,
        |_| {},
        |_| {},
        &mut progress,
        "",
    )
        .unwrap_or_else(|report| report)
}

#[doc(hidden)]
pub fn run_duplicate_cleanup_cancellable_for_test<F>(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: &AtomicBool,
    backend_protected_paths: &[String],
    before_selected_fingerprint: F,
) -> Result<DuplicateCleanupReport, String>
where
    F: FnMut(&Path),
{
    let mut progress = |_| {};
    run_duplicate_cleanup_internal(
        request,
        registry,
        recycle_bin,
        backend_protected_paths,
        Some(cancelled),
        before_selected_fingerprint,
        |_| {},
        &mut progress,
        "test",
    )
    .map_err(|_| "操作已取消".to_string())
}

#[doc(hidden)]
pub fn run_duplicate_cleanup_cancellable_before_recycle_for_test<F>(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: &AtomicBool,
    backend_protected_paths: &[String],
    before_recycle: F,
) -> Result<DuplicateCleanupReport, String>
where
    F: FnMut(&Path),
{
    let mut progress = |_| {};
    run_duplicate_cleanup_internal(
        request,
        registry,
        recycle_bin,
        backend_protected_paths,
        Some(cancelled),
        |_| {},
        before_recycle,
        &mut progress,
        "test",
    )
    .map_err(|_| "操作已取消".to_string())
}

#[doc(hidden)]
pub fn run_duplicate_cleanup_with_progress_for_test<P>(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
    mut progress: P,
) -> DuplicateCleanupReport
where
    P: FnMut(OperationProgressPayload),
{
    run_duplicate_cleanup_internal(
        request,
        registry,
        recycle_bin,
        &[],
        None,
        |_| {},
        |_| {},
        &mut progress,
        "test",
    )
    .unwrap_or_else(|report| report)
}

fn run_duplicate_cleanup_internal<P>(
    request: DuplicateCleanupRequest,
    registry: &DuplicateEntryRegistry,
    recycle_bin: &impl RecycleBin,
    backend_protected_paths: &[String],
    cancelled: Option<&AtomicBool>,
    mut before_selected_fingerprint: impl FnMut(&Path),
    mut before_recycle: impl FnMut(&Path),
    progress: &mut P,
    operation_id: &str,
) -> Result<DuplicateCleanupReport, DuplicateCleanupReport>
where
    P: FnMut(OperationProgressPayload),
{
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
    let total_selected = request
        .groups
        .iter()
        .flat_map(|group| &group.files)
        .filter(|file| file.selected)
        .count() as u64;

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
            emit_cleanup_progress(
                progress,
                operation_id,
                &report,
                total_selected,
                selected
                    .first()
                    .map(|file| visible_location_hint(&file.entry.path))
                    .unwrap_or_default(),
            );
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
                emit_cleanup_progress(
                    progress,
                    operation_id,
                    &report,
                    total_selected,
                    visible_location_hint(path),
                );
                continue;
            }
            if retained_ids.contains(&file.entry.entry_id)
                || retained_path_keys.contains(&file.entry.path_key)
                || !selected_path_keys.insert(file.entry.path_key.clone())
            {
                report.skipped_count += 1;
                emit_cleanup_progress(
                    progress,
                    operation_id,
                    &report,
                    total_selected,
                    visible_location_hint(path),
                );
                continue;
            }
            if (file.request_protected
                || file.entry.protected
                || is_protected_duplicate_path(path, backend_protected_paths))
                && !protected_override_confirmed
            {
                report.skipped_count += 1;
                emit_cleanup_progress(
                    progress,
                    operation_id,
                    &report,
                    total_selected,
                    visible_location_hint(path),
                );
                continue;
            }

            before_selected_fingerprint(path);
            let (size_bytes, hash) = match file_fingerprint_with_cancel(path, cancelled) {
                Ok(fingerprint) => fingerprint,
                Err(_) if cancellation_requested(cancelled) => return Err(report),
                Err(_) => {
                    report.failed_count += 1;
                    emit_cleanup_progress(
                        progress,
                        operation_id,
                        &report,
                        total_selected,
                        visible_location_hint(path),
                    );
                    continue;
                }
            };
            let mut retained_fingerprints = Vec::new();
            for entry in &retained {
                match file_fingerprint_with_cancel(&entry.entry.path, cancelled) {
                    Ok(fingerprint) => {
                        retained_fingerprints.push((entry.entry.path_key.clone(), fingerprint));
                    }
                    Err(_) if cancellation_requested(cancelled) => return Err(report),
                    Err(_) => {}
                }
            }
            if !retained_fingerprints.iter().any(|(retained_path_key, fingerprint)| {
                retained_path_key != &file.entry.path_key && fingerprint == &(size_bytes, hash.clone())
            }) {
                report.skipped_count += 1;
                emit_cleanup_progress(
                    progress,
                    operation_id,
                    &report,
                    total_selected,
                    visible_location_hint(path),
                );
                continue;
            }

            before_recycle(path);
            if check_cancelled(cancelled).is_err() {
                return Err(report);
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
            emit_cleanup_progress(
                progress,
                operation_id,
                &report,
                total_selected,
                visible_location_hint(path),
            );
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
            let mut progress = |payload| {
                emit_progress_payload(&app_handle, payload);
            };
            let backend_protected_paths = crate::v2::settings::get_cleaner_settings(&app_handle)
                .map(|settings| settings.protected_paths);
            scan_duplicate_files_with_backend_settings(
                request,
                backend_protected_paths,
                Some(&cancelled),
                &mut progress,
                &operation_id_for_thread,
            )
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
                match crate::v2::settings::get_cleaner_settings(&app_handle) {
                    Err(_) => (
                        OperationStatus::Failed,
                        serde_json::Value::Null,
                        Some(cleanup_settings_unavailable_error()),
                    ),
                    Ok(settings) => {
                        let started_at = now_rfc3339();
                        let backend_protected_paths = settings.protected_paths;
                        let cleanup_result = {
                            let registry = app_handle.state::<DuplicateEntryRegistry>();
                            let mut progress = |payload| {
                                emit_progress_payload(&app_handle, payload);
                            };
                            run_duplicate_cleanup_internal(
                                request,
                                &registry,
                                &SystemRecycleBin,
                                &backend_protected_paths,
                                Some(&cancelled),
                                |_| {},
                                |_| {},
                                &mut progress,
                                &operation_id_for_thread,
                            )
                        };
                        let cancelled_after_cleanup = cancelled.load(Ordering::Relaxed);
                        let report = match cleanup_result {
                            Ok(report) | Err(report) => report,
                        };
                        let finished_at = now_rfc3339();
                        if !cancelled_after_cleanup {
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
                        }

                        emit_progress_payload(
                            &app_handle,
                            cleanup_finished_progress_payload(&operation_id_for_thread, &report),
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
                    }
                }
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

fn request_with_backend_protected_paths(
    mut request: DuplicateScanRequest,
    backend_protected_paths: Vec<String>,
) -> DuplicateScanRequest {
    let mut seen = request
        .protected_paths
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    for protected_path in backend_protected_paths {
        if seen.insert(protected_path.clone()) {
            request.protected_paths.push(protected_path);
        }
    }
    request
}

fn scan_settings_unavailable_error() -> String {
    "无法读取清理设置，扫描已停止".to_string()
}

fn cleanup_settings_unavailable_error() -> String {
    "无法读取清理设置，清理已停止".to_string()
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
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();
    allowed_extensions
        .iter()
        .any(|extension| file_name.ends_with(&format!(".{extension}")))
}

fn strict_duplicate_groups<F, P>(
    candidates_by_size: &BTreeMap<u64, Vec<CandidateFile>>,
    before_hash: &mut F,
    cancelled: Option<&AtomicBool>,
    progress: &mut P,
    operation_id: &str,
    scanned_files: u64,
) -> Result<(Vec<DuplicateFileGroup>, u64, Vec<DuplicateBackendEntry>), String>
where
    F: FnMut(&Path),
    P: FnMut(OperationProgressPayload),
{
    let mut groups = Vec::new();
    let mut backend_entries = Vec::new();
    let mut skipped_locations = 0_u64;
    let mut hashed_files = 0_u64;

    for candidates in candidates_by_size.values().filter(|files| files.len() >= 2) {
        let mut by_hash: HashMap<String, Vec<CandidateFile>> = HashMap::new();
        for candidate in candidates {
            check_cancelled(cancelled)?;
            before_hash(&candidate.path);
            hashed_files += 1;
            emit_scan_progress(
                progress,
                operation_id,
                "hashing",
                scan_progress_percent(hashed_files, 60),
                candidate.visible_location_hint.clone(),
                scanned_files,
                groups.len() as u64,
                groups.iter().map(|group: &DuplicateFileGroup| group.files.len() as u64).sum(),
                groups.iter().map(|group| group.reclaimable_bytes).sum(),
            );
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
                .enumerate()
                .map(|(index, file)| {
                    let entry_id = uuid::Uuid::new_v4().to_string();
                    backend_entries.push(DuplicateBackendEntry {
                        entry_id: entry_id.clone(),
                        group_id: group_id.clone(),
                        path: file.path.clone(),
                        path_key: file.path_key.clone(),
                        protected: file.protected,
                    });
                    duplicate_entry(file, entry_id, fingerprint_id.clone(), false, index + 1)
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
            emit_scan_progress(
                progress,
                operation_id,
                "hashing",
                scan_progress_percent(hashed_files, 70),
                groups
                    .last()
                    .and_then(|group| group.files.first())
                    .map(|file| file.visible_location_hint.clone())
                    .unwrap_or_default(),
                scanned_files,
                groups.len() as u64,
                groups.iter().map(|group| group.files.len() as u64).sum(),
                groups.iter().map(|group| group.total_bytes).sum(),
            );
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
            .enumerate()
            .map(|(index, file)| {
                let mut entry = duplicate_entry(
                    file,
                    uuid::Uuid::new_v4().to_string(),
                    uuid::Uuid::new_v4().to_string(),
                    true,
                    index + 1,
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
    suspected: bool,
    ordinal: usize,
) -> DuplicateFileEntry {
    DuplicateFileEntry {
        entry_id,
        display_name: anonymized_duplicate_display_name(suspected, ordinal),
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

fn file_fingerprint_with_cancel(
    path: &Path,
    cancelled: Option<&AtomicBool>,
) -> Result<(u64, String), String> {
    check_cancelled(cancelled)?;
    let metadata = fs::metadata(path).map_err(|_| "无法读取重复文件".to_string())?;
    Ok((metadata.len(), hash_file_with_cancel(path, cancelled)?))
}

fn visible_location_hint(path: &Path) -> String {
    let drive = drive_label(path);
    if drive.is_empty() {
        "文件夹".to_string()
    } else {
        format!("{} 盘 · 文件夹", drive.trim_end_matches(':'))
    }
}

fn anonymized_duplicate_display_name(suspected: bool, ordinal: usize) -> String {
    if suspected {
        format!("疑似重复 {ordinal}")
    } else {
        format!("重复文件 {ordinal}")
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

fn scan_progress_percent(count: u64, base: u8) -> u8 {
    let increment = count.min(29) as u8;
    base.saturating_add(increment).min(99)
}

fn cleanup_progress_percent(processed_items: u64, total_items: u64) -> u8 {
    if total_items == 0 {
        return 99;
    }
    let percent = 10 + (processed_items.saturating_mul(89) / total_items).min(89) as u8;
    percent.min(99)
}

fn emit_scan_progress(
    progress: &mut impl FnMut(OperationProgressPayload),
    operation_id: &str,
    stage: &str,
    percent: u8,
    current_location_hint: String,
    scanned_files: u64,
    found_groups: u64,
    found_items: u64,
    found_bytes: u64,
) {
    progress(progress_payload(
        operation_id,
        OperationModule::DuplicateScan,
        stage,
        percent,
        current_location_hint,
        scanned_files,
        found_groups,
        found_items,
        found_bytes,
        0,
        0,
        0,
        0,
    ));
}

fn emit_cleanup_progress(
    progress: &mut impl FnMut(OperationProgressPayload),
    operation_id: &str,
    report: &DuplicateCleanupReport,
    total_items: u64,
    current_location_hint: String,
) {
    progress(progress_payload(
        operation_id,
        OperationModule::DuplicateCleanup,
        "cleaning",
        cleanup_progress_percent(report.processed_files, total_items),
        current_location_hint,
        0,
        0,
        0,
        report.freed_bytes,
        report.processed_files,
        report.success_count,
        report.skipped_count,
        report.failed_count,
    ));
}

fn progress_payload(
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
        percent: percent.min(99),
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

fn cleanup_finished_progress_payload(
    operation_id: &str,
    report: &DuplicateCleanupReport,
) -> OperationProgressPayload {
    let mut payload = progress_payload(
        operation_id,
        OperationModule::DuplicateCleanup,
        "finished",
        99,
        String::new(),
        0,
        0,
        0,
        report.freed_bytes,
        report.processed_files,
        report.success_count,
        report.skipped_count,
        report.failed_count,
    );
    payload.percent = 100;
    payload
}

#[doc(hidden)]
pub fn cleanup_finished_progress_for_test(
    operation_id: &str,
    report: &DuplicateCleanupReport,
) -> OperationProgressPayload {
    cleanup_finished_progress_payload(operation_id, report)
}

fn emit_progress_payload(app_handle: &AppHandle, payload: OperationProgressPayload) {
    let _ = app_handle.emit(OPERATION_PROGRESS_EVENT, payload);
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
    let mut payload = progress_payload(
        operation_id,
        module,
        stage,
        percent,
        String::new(),
        scanned_files,
        found_groups,
        found_items,
        found_bytes,
        0,
        0,
        0,
        0,
    );
    payload.percent = percent;
    emit_progress_payload(app_handle, payload);
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
