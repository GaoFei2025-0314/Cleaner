use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf, Prefix};
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
use crate::v2::path_safety::{
    is_protected_duplicate_path, normalized_existing_or_logical_path_key, safe_target_path_key,
};
use crate::v2::recycle_bin::{RecycleBin, SystemRecycleBin};

pub fn validate_migration_target(
    source_file: impl AsRef<Path>,
    target_folder: impl AsRef<Path>,
) -> Result<(), String> {
    let source_file = source_file.as_ref();
    let target_folder = target_folder.as_ref();
    reject_parent_dir_traversal(target_folder)?;
    reject_target_link_ancestor(target_folder)?;
    let source_parent = source_file
        .parent()
        .ok_or_else(|| "无法识别源文件目录".to_string())?;
    let source_parent_key = normalized_existing_or_logical_path_key(source_parent);
    let target_key = safe_target_path_key(target_folder);

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
pub fn target_conflicts_with_backend_protected_paths_for_test(
    target_folder: &Path,
    backend_protected_paths: &[String],
) -> Result<(), String> {
    reject_protected_target(target_folder, backend_protected_paths)
}

#[doc(hidden)]
pub fn copy_file_to_temp_with_cleanup_for_test<F>(
    source: &Path,
    temp_path: &Path,
    copy_file: F,
) -> Result<(), String>
where
    F: FnOnce(&Path, &Path) -> io::Result<u64>,
{
    copy_file_to_temp_with_cleanup(source, temp_path, copy_file).map_err(|_| "复制失败".to_string())
}

#[doc(hidden)]
pub fn select_best_mount_key_for_target_for_test(
    target_key: &str,
    mount_keys: &[&str],
) -> Option<String> {
    select_best_mount_key_for_target(target_key, mount_keys.iter().copied())
}

#[doc(hidden)]
pub fn ensure_available_space_for_test(
    target_folder: &Path,
    needed_bytes: u64,
    mounts: &[(&str, u64)],
) -> Result<(), String> {
    let target_key = safe_target_path_key(target_folder);
    ensure_available_space_for_mounts(
        &target_key,
        needed_bytes,
        mounts.iter().map(|(mount_key, available_space)| {
            (
                normalized_existing_or_logical_path_key(Path::new(mount_key)),
                *available_space,
            )
        }),
    )
}

#[doc(hidden)]
pub fn migration_operation_status_for_test(
    migration_result: &Result<MigrationResult, MigrationResult>,
    cancelled_after_completion: bool,
) -> OperationStatus {
    migration_operation_status(migration_result, cancelled_after_completion)
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
        &[],
        None,
        |_| {},
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
        &[],
        Some(cancelled),
        before_recycle,
        |_| {},
        &mut progress,
        "test",
    )
}

#[doc(hidden)]
pub fn run_large_file_migration_cancellable_before_verify_for_test<F>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    cancelled: &AtomicBool,
    before_verify: F,
) -> Result<MigrationResult, MigrationResult>
where
    F: FnMut(&Path),
{
    let mut progress = |_| {};
    run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        &[],
        Some(cancelled),
        |_| {},
        before_verify,
        &mut progress,
        "test",
    )
}

#[doc(hidden)]
pub fn run_large_file_migration_with_backend_protected_paths_for_test(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    backend_protected_paths: &[String],
) -> MigrationResult {
    let mut progress = |_| {};
    run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        backend_protected_paths,
        None,
        |_| {},
        |_| {},
        &mut progress,
        "test",
    )
    .unwrap_or_else(|report| report)
}

#[doc(hidden)]
pub fn run_large_file_migration_with_backend_settings_for_test(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    backend_protected_paths: Result<Vec<String>, String>,
) -> Result<MigrationResult, String> {
    let mut progress = |_| {};
    run_large_file_migration_with_backend_settings(
        request,
        registry,
        recycle_bin,
        backend_protected_paths,
        None,
        |_| {},
        |_| {},
        &mut progress,
        "test",
    )
    .map(|result| result.unwrap_or_else(|report| report))
}

fn run_large_file_migration_with_backend_settings<P>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    backend_protected_paths: Result<Vec<String>, String>,
    cancelled: Option<&AtomicBool>,
    before_recycle: impl FnMut(&Path),
    before_verify: impl FnMut(&Path),
    progress: &mut P,
    operation_id: &str,
) -> Result<Result<MigrationResult, MigrationResult>, String>
where
    P: FnMut(OperationProgressPayload),
{
    let backend_protected_paths =
        backend_protected_paths.map_err(|_| migration_settings_unavailable_error())?;
    Ok(run_large_file_migration_internal(
        request,
        registry,
        recycle_bin,
        &backend_protected_paths,
        cancelled,
        before_recycle,
        before_verify,
        progress,
        operation_id,
    ))
}

fn run_large_file_migration_internal<P>(
    request: MigrationRequest,
    registry: &LargeFileRegistry,
    recycle_bin: &impl RecycleBin,
    backend_protected_paths: &[String],
    cancelled: Option<&AtomicBool>,
    mut before_recycle: impl FnMut(&Path),
    mut before_verify: impl FnMut(&Path),
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
    let selected_entries = snapshot_selected_entries(&request.selected_item_ids, registry);
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
    let total_items = selected_entries.len() as u64;
    let mut processed_items = 0_u64;

    for (item_id, entry) in selected_entries {
        if check_cancelled(cancelled).is_err() {
            return Err(result);
        }
        processed_items += 1;
        let Some(entry) = entry else {
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
        let backend_protected = is_protected_duplicate_path(&entry.path, backend_protected_paths);
        if (entry.protected || report_protected || backend_protected)
            && !request.protected_override_confirmed
        {
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
            backend_protected_paths,
            cancelled,
            &mut before_recycle,
            &mut before_verify,
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

fn snapshot_selected_entries(
    selected_item_ids: &[String],
    registry: &LargeFileRegistry,
) -> Vec<(String, Option<LargeFileBackendEntry>)> {
    let mut seen = HashSet::new();
    selected_item_ids
        .iter()
        .filter_map(|item_id| {
            if seen.insert(item_id.clone()) {
                Some((item_id.clone(), registry.get(item_id)))
            } else {
                None
            }
        })
        .collect()
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
    backend_protected_paths: &[String],
    cancelled: Option<&AtomicBool>,
    before_recycle: &mut impl FnMut(&Path),
    before_verify: &mut impl FnMut(&Path),
) -> Result<MigrationItemResult, ItemFailure> {
    check_cancelled(cancelled).map_err(|_| ItemFailure::Cancelled)?;
    if !entry.path.exists() {
        return Err(ItemFailure::Failed("源文件不存在".to_string()));
    }
    let target_folder = resolve_target_folder(&request.target_folder, &entry.path)?;
    validate_migration_target(&entry.path, &target_folder).map_err(ItemFailure::Failed)?;
    reject_protected_target(&target_folder, backend_protected_paths)
        .map_err(ItemFailure::Failed)?;
    fs::create_dir_all(&target_folder)
        .map_err(|_| ItemFailure::Failed("无法创建目标文件夹".to_string()))?;
    ensure_available_space(&target_folder, entry.size_bytes).map_err(ItemFailure::Failed)?;
    let target_path = unique_target_path(&target_folder, &entry.path)
        .ok_or_else(|| ItemFailure::Failed("无法生成目标文件名".to_string()))?;
    let temp_path = unique_temp_copy_path(&target_folder)
        .ok_or_else(|| ItemFailure::Failed("无法生成临时文件名".to_string()))?;

    check_cancelled(cancelled).map_err(|_| ItemFailure::Cancelled)?;
    copy_file_to_temp_with_cleanup(&entry.path, &temp_path, |source, target| {
        fs::copy(source, target)
    })
    .map_err(|_| ItemFailure::Failed("复制失败".to_string()))?;
    before_verify(&temp_path);
    if check_cancelled(cancelled).is_err() {
        remove_temp_copy(&temp_path);
        return Err(ItemFailure::Cancelled);
    }
    verify_copied_file(&entry.path, &temp_path, cancelled).map_err(|error| {
        remove_temp_copy(&temp_path);
        if cancellation_requested(cancelled) {
            ItemFailure::Cancelled
        } else {
            ItemFailure::Failed(error)
        }
    })?;
    if check_cancelled(cancelled).is_err() {
        remove_temp_copy(&temp_path);
        return Err(ItemFailure::Cancelled);
    }
    fs::rename(&temp_path, &target_path).map_err(|_| {
        remove_temp_copy(&temp_path);
        ItemFailure::Failed("复制失败".to_string())
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
            status: MigrationItemStatus::Failed,
            category: entry.category.clone(),
            bytes_copied: 0,
            bytes_freed: 0,
            message: "原文件移入回收站失败，已保留复制文件和原文件".to_string(),
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
            let backend_protected_paths = crate::v2::settings::get_cleaner_settings(&app_handle)
                .map(|settings| settings.protected_paths);
            run_large_file_migration_with_backend_settings(
                request,
                &registry,
                &SystemRecycleBin,
                backend_protected_paths,
                Some(&cancelled),
                |_| {},
                |_| {},
                &mut progress,
                &operation_id_for_thread,
            )
        };
        let migration_result = match migration_result {
            Ok(migration_result) => migration_result,
            Err(error) => {
                progress(progress_payload(
                    &operation_id_for_thread,
                    OperationModule::LargeFileMigration,
                    "finished",
                    100,
                    String::new(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    1,
                ));
                emit_finished(
                    &app_handle,
                    &operation_id_for_thread,
                    OperationModule::LargeFileMigration,
                    OperationStatus::Failed,
                    serde_json::Value::Null,
                    Some(error),
                );
                app_handle
                    .state::<OperationRegistry>()
                    .finish(&operation_id_for_thread);
                return;
            }
        };
        let operation_status =
            migration_operation_status(&migration_result, cancelled.load(Ordering::Relaxed));
        let report = match migration_result {
            Ok(report) | Err(report) => report,
        };
        if operation_status == OperationStatus::Completed {
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
            operation_status.clone(),
            serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
            (operation_status == OperationStatus::Cancelled).then(|| "操作已取消".to_string()),
        );
        app_handle
            .state::<OperationRegistry>()
            .finish(&operation_id_for_thread);
    });

    Ok(OperationStart { operation_id })
}

fn migration_operation_status(
    migration_result: &Result<MigrationResult, MigrationResult>,
    _cancelled_after_completion: bool,
) -> OperationStatus {
    if migration_result.is_err() {
        OperationStatus::Cancelled
    } else {
        OperationStatus::Completed
    }
}

fn resolve_target_folder(target_folder: &str, _source_path: &Path) -> Result<PathBuf, ItemFailure> {
    let trimmed = target_folder.trim();
    if trimmed.is_empty() {
        return Err(ItemFailure::Failed("目标文件夹不能为空".to_string()));
    }
    let path = PathBuf::from(trimmed);
    if !is_local_windows_drive_absolute_path(&path) {
        return Err(ItemFailure::Failed(
            "目标文件夹必须是本地磁盘绝对路径".to_string(),
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_target_folder_rejects_empty_target_without_source_drive_default() {
        let error =
            resolve_target_folder("", Path::new(r"C:\Users\Alice\Downloads\private-movie.mp4"));

        assert_failed_message(error, "目标文件夹不能为空");
    }

    #[test]
    fn resolve_target_folder_rejects_relative_target() {
        let error = resolve_target_folder(
            "Cleaner_MigratedFiles",
            Path::new(r"C:\Users\Alice\Downloads\private-movie.mp4"),
        );

        assert_failed_message(error, "目标文件夹必须是本地磁盘绝对路径");
    }

    #[test]
    fn resolve_target_folder_rejects_unc_target() {
        let error = resolve_target_folder(
            r"\\server\share\Cleaner_MigratedFiles",
            Path::new(r"C:\Users\Alice\Downloads\private-movie.mp4"),
        );

        assert_failed_message(error, "目标文件夹必须是本地磁盘绝对路径");
    }

    fn assert_failed_message(result: Result<PathBuf, ItemFailure>, expected: &str) {
        match result {
            Err(ItemFailure::Failed(message)) => {
                assert!(message.contains(expected));
                assert!(!message.contains("Alice"));
                assert!(!message.contains("private-movie.mp4"));
            }
            _ => panic!("expected failed item result"),
        }
    }
}

fn reject_protected_target(
    target_folder: &Path,
    backend_protected_paths: &[String],
) -> Result<(), String> {
    reject_parent_dir_traversal(target_folder)?;
    reject_target_link_ancestor(target_folder)?;
    let key = safe_target_path_key(target_folder);
    if [
        r"c:\windows",
        r"c:\program files",
        r"c:\program files (x86)",
        r"c:\programdata",
    ]
    .iter()
    .any(|protected_key| key_is_same_or_child(&key, protected_key))
        || backend_protected_paths.iter().any(|protected_path| {
            key_is_same_or_child(
                &key,
                &normalized_existing_or_logical_path_key(Path::new(protected_path)),
            )
        })
    {
        Err("目标位置不能位于受保护目录内".to_string())
    } else {
        Ok(())
    }
}

fn reject_target_link_ancestor(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
                    return Err("目标位置不能位于符号链接目录内".to_string());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => return Err("目标位置不能位于符号链接目录内".to_string()),
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn reject_parent_dir_traversal(path: &Path) -> Result<(), String> {
    if path
        .display()
        .to_string()
        .replace('/', "\\")
        .split('\\')
        .any(|segment| segment == "..")
    {
        Err("目标位置不能包含上级目录跳转".to_string())
    } else {
        Ok(())
    }
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

fn ensure_available_space(target_folder: &Path, needed_bytes: u64) -> Result<(), String> {
    let disks = Disks::new_with_refreshed_list();
    let target_key = safe_target_path_key(target_folder);
    ensure_available_space_for_mounts(
        &target_key,
        needed_bytes,
        disks.iter().map(|disk| {
            (
                normalized_existing_or_logical_path_key(disk.mount_point()),
                disk.available_space(),
            )
        }),
    )
}

fn ensure_available_space_for_mounts(
    target_key: &str,
    needed_bytes: u64,
    mounts: impl IntoIterator<Item = (String, u64)>,
) -> Result<(), String> {
    let mut best_match: Option<(usize, u64)> = None;
    for (mount_key, available_space) in mounts {
        if mount_key_matches_target(&target_key, &mount_key) {
            let mount_len = mount_key.len();
            if best_match
                .map(|(best_len, _)| mount_len > best_len)
                .unwrap_or(true)
            {
                best_match = Some((mount_len, available_space));
            }
        }
    }

    match best_match {
        Some((_, available_space)) if available_space < needed_bytes => {
            Err("目标磁盘空间不足".to_string())
        }
        Some(_) => Ok(()),
        None => Err("无法识别目标磁盘".to_string()),
    }
}

fn is_local_windows_drive_absolute_path(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        components.next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    ) && matches!(components.next(), Some(Component::RootDir))
}

fn select_best_mount_key_for_target<'a>(
    target_key: &str,
    mount_keys: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    mount_keys
        .into_iter()
        .filter(|mount_key| mount_key_matches_target(target_key, mount_key))
        .max_by_key(|mount_key| mount_key.len())
        .map(|mount_key| mount_key.to_string())
}

fn mount_key_matches_target(target_key: &str, mount_key: &str) -> bool {
    if mount_key.is_empty() {
        return false;
    }
    target_key == mount_key
        || target_key
            .strip_prefix(mount_key)
            .is_some_and(|tail| tail.starts_with('\\'))
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

fn unique_temp_copy_path(target_folder: &Path) -> Option<PathBuf> {
    for _ in 0..=999 {
        let candidate = target_folder.join(format!(".cleaner-copy-{}.tmp", uuid::Uuid::new_v4()));
        if !candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn copy_file_to_temp_with_cleanup<F>(
    source: &Path,
    temp_path: &Path,
    copy_file: F,
) -> Result<(), ()>
where
    F: FnOnce(&Path, &Path) -> io::Result<u64>,
{
    match copy_file(source, temp_path) {
        Ok(_) => Ok(()),
        Err(_) => {
            remove_temp_copy(temp_path);
            Err(())
        }
    }
}

fn remove_temp_copy(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
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

fn migration_settings_unavailable_error() -> String {
    "无法读取清理设置，迁移已停止".to_string()
}
