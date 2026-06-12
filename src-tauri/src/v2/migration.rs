use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use sha2::{Digest, Sha256};
use sysinfo::Disks;
use tauri::{AppHandle, Emitter, Manager};

use crate::v2::history::append_history_entry;
use crate::v2::large_files::{
    check_cancelled, emit_finished, now_rfc3339, progress_payload, visible_location_hint,
    LargeFileBackendEntry, LargeFileRegistry,
};
use crate::v2::models::{
    HistoryEntry, LargeFileCategory, MigrationItemResult, MigrationItemStatus, MigrationRequest,
    MigrationResult, OperationModule, OperationProgressPayload, OperationStart, OperationStatus,
    OriginalFilePolicy,
};
use crate::v2::operations::OperationRegistry;
use crate::v2::path_safety::{canonical_path_key, drive_label};
use crate::v2::recycle_bin::{RecycleBin, SystemRecycleBin};

pub fn validate_migration_target(
    source_file: impl AsRef<Path>,
    target_folder: impl AsRef<Path>,
) -> Result<(), String> {
    let source_file = source_file.as_ref();
    let target_folder = target_folder.as_ref();
    let source_parent = source_file
        .parent()
        .ok_or_else(|| "无法识别源文件目录".to_string())?;
    let source_parent_key = canonical_path_key(source_parent);
    let target_key = canonical_path_key(target_folder);

    if target_key == source_parent_key
        || target_key
            .strip_prefix(&source_parent_key)
            .is_some_and(|tail| tail.starts_with('\\'))
    {
        return Err("目标位置不能位于源文件目录内".to_string());
    }

    Ok(())
}

pub fn run_large_file_migration_with_recycle_bin(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
) -> MigrationResult {
    let mut progress = |_| {};
    run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        None,
        |_| {},
        &mut progress,
        "",
    )
    .unwrap_or_else(|report| report)
}

#[doc(hidden)]
pub fn run_large_file_migration_with_progress_for_test<P>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    mut progress: P,
) -> MigrationResult
where
    P: FnMut(OperationProgressPayload),
{
    run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        None,
        |_| {},
        &mut progress,
        "test",
    )
    .unwrap_or_else(|report| report)
}

#[doc(hidden)]
pub fn run_large_file_migration_cancellable_before_recycle_for_test<F>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: &AtomicBool,
    before_recycle: F,
) -> Result<MigrationResult, MigrationResult>
where
    F: FnMut(&Path),
{
    let mut progress = |_| {};
    run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        Some(cancelled),
        before_recycle,
        &mut progress,
        "test",
    )
}

fn run_large_file_migration_internal<P>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: Option<&AtomicBool>,
    mut before_recycle: impl FnMut(&Path),
    progress: &mut P,
    operation_id: &str,
) -> Result<MigrationResult, MigrationResult>
where
    P: FnMut(OperationProgressPayload),
{
    let mut result = MigrationResult {
        copied_count: 0,
        moved_to_recycle_bin_count: 0,
        skipped_count: 0,
        failed_count: 0,
        total_copied_bytes: 0,
        total_freed_bytes: 0,
        c_drive_freed_bytes: 0,
        item_results: Vec::new(),
    };
    let selected_ids = request
        .selected_item_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let report_items = request
        .scan_report
        .items
        .iter()
        .map(|item| {
            (
                item.item_id.clone(),
                (item.protected, item.category.clone()),
            )
        })
        .collect::<HashMap<_, _>>();
    let total_items = selected_ids.len() as u64;
    let mut processed_items = 0_u64;

    for item_id in selected_ids {
        if check_cancelled(cancelled).is_err() {
            return Err(result);
        }
        processed_items += 1;
        let Some(entry) = registry.get(&item_id) else {
            result.failed_count += 1;
            push_item_result(
                &mut result,
                item_id,
                LargeFileCategory::Other,
                MigrationItemStatus::Failed,
                0,
                0,
                "文件记录已过期".to_string(),
            );
            emit_migration_progress(
                progress,
                operation_id,
                &result,
                processed_items,
                total_items,
                "",
            );
            continue;
        };
        let report_protected = report_items
            .get(&item_id)
            .map(|(protected, _)| *protected)
            .unwrap_or(false);
        let category = report_items
            .get(&item_id)
            .map(|(_, category)| category.clone())
            .unwrap_or_else(|| entry.category.clone());
        if (entry.protected || report_protected) && !request.protected_override_confirmed {
            result.skipped_count += 1;
            push_item_result(
                &mut result,
                item_id,
                category,
                MigrationItemStatus::Skipped,
                0,
                0,
                "受保护文件已跳过".to_string(),
            );
            emit_migration_progress(
                progress,
                operation_id,
                &result,
                processed_items,
                total_items,
                &visible_location_hint(&entry.path),
            );
            continue;
        }

        match migrate_one_item(
            &request,
            &entry,
            recycle_bin,
            cancelled,
            &mut before_recycle,
        ) {
            Ok(item_result) => {
                match item_result.status {
                    MigrationItemStatus::Copied => result.copied_count += 1,
                    MigrationItemStatus::CopiedAndFreed => {
                        result.copied_count += 1;
                        result.moved_to_recycle_bin_count += 1;
                    }
                    MigrationItemStatus::Skipped => result.skipped_count += 1,
                    MigrationItemStatus::Failed => result.failed_count += 1,
                }
                result.total_copied_bytes += item_result.bytes_copied;
                result.total_freed_bytes += item_result.bytes_freed;
                if entry.drive.eq_ignore_ascii_case("C:") {
                    result.c_drive_freed_bytes += item_result.bytes_freed;
                }
                result.item_results.push(item_result);
            }
            Err(ItemFailure::Cancelled) => return Err(result),
            Err(ItemFailure::CancelledAfterCopy(item_result)) => {
                result.copied_count += 1;
                result.total_copied_bytes += item_result.bytes_copied;
                result.item_results.push(item_result);
                return Err(result);
            }
            Err(ItemFailure::Failed(message)) => {
                result.failed_count += 1;
                push_item_result(
                    &mut result,
                    item_id,
                    category,
                    MigrationItemStatus::Failed,
                    0,
                    0,
                    message,
                );
            }
        }
        emit_migration_progress(
            progress,
            operation_id,
            &result,
            processed_items,
            total_items,
            &visible_location_hint(&entry.path),
        );
    }

    Ok(result)
}

enum ItemFailure {
    Cancelled,
    CancelledAfterCopy(MigrationItemResult),
    Failed(String),
}

fn migrate_one_item(
    request: &MigrationRequest,
    entry: &LargeFileBackendEntry,
    recycle_bin: &impl RecycleBin,
    cancelled: Option<&AtomicBool>,
    before_recycle: &mut impl FnMut(&Path),
) -> Result<MigrationItemResult, ItemFailure> {
    check_cancelled(cancelled).map_err(|_| ItemFailure::Cancelled)?;
    if !entry.path.exists() {
        return Err(ItemFailure::Failed("源文件不存在".to_string()));
    }
    let target_folder = resolve_target_folder(&request.target_folder, &entry.path)?;
    validate_migration_target(&entry.path, &target_folder).map_err(ItemFailure::Failed)?;
    reject_protected_target(&target_folder).map_err(ItemFailure::Failed)?;
    fs::create_dir_all(&target_folder)
        .map_err(|_| ItemFailure::Failed("无法创建目标文件夹".to_string()))?;
    ensure_available_space(&target_folder, entry.size_bytes)
        .map_err(|_| ItemFailure::Failed("目标磁盘空间不足".to_string()))?;
    let target_path = unique_target_path(&target_folder, &entry.path)
        .ok_or_else(|| ItemFailure::Failed("无法生成目标文件名".to_string()))?;

    check_cancelled(cancelled).map_err(|_| ItemFailure::Cancelled)?;
    fs::copy(&entry.path, &target_path).map_err(|_| ItemFailure::Failed("复制失败".to_string()))?;
    check_cancelled(cancelled).map_err(|_| ItemFailure::Cancelled)?;
    verify_copied_file(&entry.path, &target_path, cancelled).map_err(|error| {
        if cancellation_requested(cancelled) {
            ItemFailure::Cancelled
        } else {
            ItemFailure::Failed(error)
        }
    })?;

    if request.original_file_policy == OriginalFilePolicy::KeepOriginal {
        return Ok(MigrationItemResult {
            item_id: entry.item_id.clone(),
            status: MigrationItemStatus::Copied,
            category: entry.category.clone(),
            bytes_copied: entry.size_bytes,
            bytes_freed: 0,
            message: "已复制".to_string(),
        });
    }

    before_recycle(&entry.path);
    if check_cancelled(cancelled).is_err() {
        return Err(ItemFailure::CancelledAfterCopy(MigrationItemResult {
            item_id: entry.item_id.clone(),
            status: MigrationItemStatus::Copied,
            category: entry.category.clone(),
            bytes_copied: entry.size_bytes,
            bytes_freed: 0,
            message: "已复制，操作已取消".to_string(),
        }));
    }
    match recycle_bin.move_to_recycle_bin(&entry.path) {
        Ok(()) => Ok(MigrationItemResult {
            item_id: entry.item_id.clone(),
            status: MigrationItemStatus::CopiedAndFreed,
            category: entry.category.clone(),
            bytes_copied: entry.size_bytes,
            bytes_freed: entry.size_bytes,
            message: "已复制并移入回收站".to_string(),
        }),
        Err(_) => Ok(MigrationItemResult {
            item_id: entry.item_id.clone(),
            status: MigrationItemStatus::Copied,
            category: entry.category.clone(),
            bytes_copied: entry.size_bytes,
            bytes_freed: 0,
            message: "已复制，原文件未移入回收站".to_string(),
        }),
    }
}

pub fn start_large_file_migration(
    app_handle: AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: MigrationRequest,
) -> Result<OperationStart, String> {
    let token = operations.register();
    let operation_id = token.operation_id.clone();
    let operation_id_for_thread = operation_id.clone();
    let cancelled = token.cancelled.clone();

    std::thread::spawn(move || {
        let started_at = now_rfc3339();
        let mut progress = |payload| {
            let _ = app_handle.emit("cleaner-operation-progress", payload);
        };
        progress(progress_payload(
            &operation_id_for_thread,
            OperationModule::LargeFileMigration,
            "migrating",
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
        ));
        let migration_result = {
            let registry = app_handle.state::<LargeFileRegistry>();
            run_large_file_migration_internal(
                request,
                &registry,
                &SystemRecycleBin,
                Some(&cancelled),
                |_| {},
                &mut progress,
                &operation_id_for_thread,
            )
        };
        let cancelled_after = cancelled.load(Ordering::Relaxed);
        let report = match migration_result {
            Ok(report) | Err(report) => report,
        };
        if !cancelled_after {
            let _ = append_history_entry(
                &app_handle,
                HistoryEntry {
                    history_id: uuid::Uuid::new_v4().to_string(),
                    module: OperationModule::LargeFileMigration,
                    started_at,
                    finished_at: now_rfc3339(),
                    total_bytes: report.total_copied_bytes,
                    freed_bytes: report.total_freed_bytes,
                    c_drive_freed_bytes: report.c_drive_freed_bytes,
                    other_drive_freed_bytes: report
                        .total_freed_bytes
                        .saturating_sub(report.c_drive_freed_bytes),
                    success_count: report.copied_count,
                    skipped_count: report.skipped_count,
                    failed_count: report.failed_count,
                    error_categories: migration_error_categories(&report),
                },
            );
        }
        progress(progress_payload(
            &operation_id_for_thread,
            OperationModule::LargeFileMigration,
            "finished",
            100,
            String::new(),
            0,
            0,
            0,
            report.total_freed_bytes,
            report.copied_count + report.skipped_count + report.failed_count,
            report.copied_count,
            report.skipped_count,
            report.failed_count,
        ));
        emit_finished(
            &app_handle,
            &operation_id_for_thread,
            OperationModule::LargeFileMigration,
            if cancelled_after {
                OperationStatus::Cancelled
            } else {
                OperationStatus::Completed
            },
            serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
            cancelled_after.then(|| "操作已取消".to_string()),
        );
        app_handle
            .state::<OperationRegistry>()
            .finish(&operation_id_for_thread);
    });

    Ok(OperationStart { operation_id })
}

fn resolve_target_folder(target_folder: &str, source_path: &Path) -> Result<PathBuf, ItemFailure> {
    if !target_folder.trim().is_empty() {
        return Ok(PathBuf::from(target_folder));
    }
    let drive = drive_label(source_path);
    if drive.is_empty() {
        return Err(ItemFailure::Failed("目标文件夹不能为空".to_string()));
    }
    Ok(PathBuf::from(format!("{drive}\\Cleaner_MigratedFiles")))
}

fn reject_protected_target(target_folder: &Path) -> Result<(), String> {
    let key = canonical_path_key(target_folder);
    if key.starts_with(r"c:\windows")
        || key.starts_with(r"c:\program files")
        || key.starts_with(r"c:\program files (x86)")
        || key.starts_with(r"c:\programdata")
    {
        Err("目标位置不能位于受保护目录内".to_string())
    } else {
        Ok(())
    }
}

fn ensure_available_space(target_folder: &Path, needed_bytes: u64) -> Result<(), String> {
    let disks = Disks::new_with_refreshed_list();
    let target_key = canonical_path_key(target_folder);
    for disk in disks.iter() {
        let mount_key = canonical_path_key(disk.mount_point());
        if !mount_key.is_empty() && target_key.starts_with(&mount_key) {
            if disk.available_space() >= needed_bytes {
                return Ok(());
            }
            return Err("目标磁盘空间不足".to_string());
        }
    }
    Ok(())
}

fn unique_target_path(target_folder: &Path, source_path: &Path) -> Option<PathBuf> {
    let file_name = source_path.file_name()?.to_string_lossy();
    let initial = target_folder.join(file_name.as_ref());
    if !initial.exists() {
        return Some(initial);
    }
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("file");
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str());
    for index in 1..=999 {
        let name = match extension {
            Some(extension) if !extension.is_empty() => format!("{stem} ({index}).{extension}"),
            _ => format!("{stem} ({index})"),
        };
        let candidate = target_folder.join(name);
        if !candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn verify_copied_file(
    source: &Path,
    target: &Path,
    cancelled: Option<&AtomicBool>,
) -> Result<(), String> {
    let source_metadata = fs::metadata(source).map_err(|_| "无法读取源文件".to_string())?;
    let target_metadata = fs::metadata(target).map_err(|_| "无法读取目标文件".to_string())?;
    if source_metadata.len() != target_metadata.len() {
        return Err("复制校验失败".to_string());
    }
    let source_hash = hash_file_with_cancel(source, cancelled)?;
    let target_hash = hash_file_with_cancel(target, cancelled)?;
    if source_hash != target_hash {
        return Err("复制校验失败".to_string());
    }
    Ok(())
}

fn hash_file_with_cancel(path: &Path, cancelled: Option<&AtomicBool>) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|_| "无法读取文件".to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancelled(cancelled)?;
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|_| "无法读取文件".to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn cancellation_requested(cancelled: Option<&AtomicBool>) -> bool {
    cancelled
        .map(|cancelled| cancelled.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn push_item_result(
    result: &mut MigrationResult,
    item_id: String,
    category: LargeFileCategory,
    status: MigrationItemStatus,
    bytes_copied: u64,
    bytes_freed: u64,
    message: String,
) {
    result.item_results.push(MigrationItemResult {
        item_id,
        status,
        category,
        bytes_copied,
        bytes_freed,
        message,
    });
}

fn emit_migration_progress(
    progress: &mut impl FnMut(OperationProgressPayload),
    operation_id: &str,
    report: &MigrationResult,
    processed_items: u64,
    total_items: u64,
    current_location_hint: &str,
) {
    let percent = if total_items == 0 {
        100
    } else {
        5 + (processed_items.saturating_mul(94) / total_items).min(94) as u8
    };
    progress(progress_payload(
        operation_id,
        OperationModule::LargeFileMigration,
        "migrating",
        percent,
        current_location_hint.to_string(),
        0,
        0,
        0,
        report.total_freed_bytes,
        processed_items,
        report.copied_count,
        report.skipped_count,
        report.failed_count,
    ));
}

fn migration_error_categories(report: &MigrationResult) -> Vec<String> {
    let mut categories = Vec::new();
    if report.skipped_count > 0 {
        categories.push("跳过受保护或无效文件".to_string());
    }
    if report.failed_count > 0 {
        categories.push("迁移失败".to_string());
    }
    if categories.is_empty() {
        categories.push("无错误".to_string());
    }
    categories
}
