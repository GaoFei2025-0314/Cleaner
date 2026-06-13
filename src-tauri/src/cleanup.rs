use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

use crate::config_refs::{find_config_references, ConfigSearchRoots};
use crate::errors::CleanerError;
use crate::fixtures::now_iso;
use crate::models::{
    CleanupAction, CleanupItemResult, CleanupResult, CleanupSelection, RiskLevel, ScanItem,
};
use crate::paths::{
    ensure_c_drive_path, ensure_under_root, resolve_rule_path, root_for_rule, ScanRoots,
};
use crate::processes::find_process_references;
use crate::rules::builtin_rules;
use crate::v2::models::{HistoryEntry, OperationModule};
use crate::v2::recycle_bin::{RecycleBin, RecycleBinError, SystemRecycleBin};

pub fn validate_high_risk_confirmation(
    selection: &CleanupSelection,
    items: &[ScanItem],
) -> Result<(), String> {
    let selected: HashSet<&str> = selection
        .selected_item_ids
        .iter()
        .map(String::as_str)
        .collect();
    let has_high_risk = items
        .iter()
        .any(|item| selected.contains(item.id.as_str()) && item.risk_level == RiskLevel::HighRisk);

    if has_high_risk && !selection.high_risk_confirmed {
        Err("高风险项目需要二次确认。".to_string())
    } else {
        Ok(())
    }
}

pub fn delete_path_contents(path: &Path) -> Result<u64, CleanerError> {
    let mut freed = 0;
    if !path.exists() {
        return Ok(0);
    }
    if is_link_or_reparse_point(path)? {
        return Err(CleanerError::PathResolution(
            "refusing to delete a link or reparse point".to_string(),
        ));
    }

    let root = path.canonicalize()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if is_link_or_reparse_point(&child)? {
            continue;
        }
        let child_root = match child.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !child_root.starts_with(&root) {
            return Err(CleanerError::PathOutsideAllowedRoot);
        }
        let size = crate::size::path_size_bytes(&child);
        if child.is_dir() {
            fs::remove_dir_all(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
        freed += size;
    }

    Ok(freed)
}

pub fn delete_path_or_contents(path: &Path, contents_only: bool) -> Result<u64, CleanerError> {
    if contents_only {
        return delete_path_contents(path);
    }

    let freed = crate::size::path_size_bytes(path);
    if !path.exists() {
        return Ok(0);
    }
    if is_link_or_reparse_point(path)? {
        return Err(CleanerError::PathResolution(
            "refusing to delete a link or reparse point".to_string(),
        ));
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(freed)
}

pub fn recycle_path_contents(
    path: &Path,
    recycle_bin: &impl RecycleBin,
) -> Result<u64, CleanerError> {
    let mut freed = 0;
    if !path.exists() {
        return Ok(0);
    }
    if is_link_or_reparse_point(path)? {
        return Err(CleanerError::PathResolution(
            "refusing to move a link or reparse point to recycle bin".to_string(),
        ));
    }

    let root = path.canonicalize()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if is_link_or_reparse_point(&child)? {
            continue;
        }
        let child_root = match child.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !child_root.starts_with(&root) {
            return Err(CleanerError::PathOutsideAllowedRoot);
        }
        let size = crate::size::path_size_bytes(&child);
        recycle_bin
            .move_to_recycle_bin(&child)
            .map_err(recycle_bin_error)?;
        freed += size;
    }

    Ok(freed)
}

pub fn recycle_path_or_contents(
    path: &Path,
    contents_only: bool,
    recycle_bin: &impl RecycleBin,
) -> Result<u64, CleanerError> {
    if contents_only {
        return recycle_path_contents(path, recycle_bin);
    }

    if !path.exists() {
        return Ok(0);
    }
    if is_link_or_reparse_point(path)? {
        return Err(CleanerError::PathResolution(
            "refusing to move a link or reparse point to recycle bin".to_string(),
        ));
    }
    let freed = crate::size::path_size_bytes(path);
    recycle_bin
        .move_to_recycle_bin(path)
        .map_err(recycle_bin_error)?;
    Ok(freed)
}

fn recycle_bin_error(error: RecycleBinError) -> CleanerError {
    CleanerError::RecycleBin(error.to_string())
}

pub fn execute_selected_cleanup(
    selection: &CleanupSelection,
    items: &[ScanItem],
    roots: &ScanRoots,
) -> Result<CleanupResult, String> {
    let recycle_bin = SystemRecycleBin;
    execute_selected_cleanup_with_recycle_bin(selection, items, roots, &recycle_bin)
}

pub fn execute_selected_cleanup_with_recycle_bin(
    selection: &CleanupSelection,
    items: &[ScanItem],
    roots: &ScanRoots,
    recycle_bin: &impl RecycleBin,
) -> Result<CleanupResult, String> {
    validate_high_risk_confirmation(selection, items)?;
    let rules = builtin_rules();
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for selected_id in &selection.selected_item_ids {
        if !seen.insert(selected_id.as_str()) {
            continue;
        }

        let Some(item) = items.iter().find(|item| item.id == *selected_id) else {
            results.push(CleanupItemResult {
                item_id: selected_id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{selected_id} 已跳过：清理前复查未再命中该项目。"),
            });
            continue;
        };

        let Some(rule) = rules.iter().find(|rule| rule.id == item.id) else {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：找不到对应安全规则。", item.title),
            });
            continue;
        };

        if item.risk_level == RiskLevel::NotCleanable {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：安全规则不允许清理。", item.title),
            });
            continue;
        }

        if item.cleanup_action != CleanupAction::DirectDelete
            || rule.cleanup_action != CleanupAction::DirectDelete
        {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：需要其他处理方式。", item.title),
            });
            continue;
        }

        let Some(path) = item.technical_path.as_ref() else {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：缺少路径信息。", item.title),
            });
            continue;
        };

        let expected_path = resolve_rule_path(rule, roots);
        let expected_root = root_for_rule(rule, roots);
        if Path::new(path) != expected_path {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：扫描路径和规则路径不一致。", item.title),
            });
            continue;
        }

        if let Err(error) =
            ensure_c_drive_path(&expected_root).and_then(|_| ensure_c_drive_path(&expected_path))
        {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：路径不在 C 盘范围内：{}。", item.title, error),
            });
            continue;
        }

        if !expected_path.exists() {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：路径已不存在。", item.title),
            });
            continue;
        }

        if let Err(error) = ensure_under_root(&expected_path, &expected_root) {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：路径安全校验未通过：{}。", item.title, error),
            });
            continue;
        }

        if rule.protect_config_references
            && !find_config_references(
                &expected_path,
                &ConfigSearchRoots {
                    user_profile: roots.user_profile.clone(),
                },
            )
            .is_empty()
        {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：清理前复查发现仍被配置引用。", item.title),
            });
            continue;
        }

        if !find_process_references(&expected_path).is_empty() {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!(
                    "{} 已跳过：清理前复查发现仍被运行中的程序使用。",
                    item.title
                ),
            });
            continue;
        }

        if !path_is_old_enough(&expected_path, rule.min_age_minutes) {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：最近仍有修改，暂不清理。", item.title),
            });
            continue;
        }

        match recycle_path_or_contents(&expected_path, rule.delete_contents_only, recycle_bin) {
            Ok(freed) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "deleted".to_string(),
                freed_bytes: freed,
                message: format!("{} 已移入回收站。", item.title),
            }),
            Err(_error) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 移入回收站失败，本项未清理。", item.title),
            }),
        }
    }

    Ok(build_cleanup_result(results))
}

fn is_link_or_reparse_point(path: &Path) -> Result<bool, CleanerError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }

    #[cfg(windows)]
    {
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

fn path_is_old_enough(path: &Path, min_age_minutes: u64) -> bool {
    if min_age_minutes == 0 {
        return true;
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(min_age_minutes * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    newest_modified_time(path)
        .map(|modified| modified <= cutoff)
        .unwrap_or(false)
}

fn newest_modified_time(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return path
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok());
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter_map(|metadata| metadata.modified().ok())
        .max()
}

pub fn build_cleanup_result(results: Vec<CleanupItemResult>) -> CleanupResult {
    let total = results.iter().map(|result| result.freed_bytes).sum();
    CleanupResult {
        results,
        total_freed_bytes: total,
        finished_at: now_iso(),
    }
}

pub fn build_c_drive_cleanup_history_entry(
    result: &CleanupResult,
    started_at: impl Into<String>,
) -> HistoryEntry {
    let success_count = result
        .results
        .iter()
        .filter(|item| item.status == "deleted")
        .count() as u64;
    let skipped_count = result
        .results
        .iter()
        .filter(|item| item.status == "skipped")
        .count() as u64;
    let failed_count = result
        .results
        .iter()
        .filter(|item| item.status != "deleted" && item.status != "skipped")
        .count() as u64;

    let mut error_categories = Vec::new();
    if failed_count > 0 {
        error_categories.push("部分项目失败".to_string());
    }
    if skipped_count > 0 {
        error_categories.push("部分项目跳过".to_string());
    }
    if error_categories.is_empty() {
        error_categories.push("无错误".to_string());
    }

    HistoryEntry {
        history_id: format!("history-cdrive-{}", uuid::Uuid::new_v4()),
        module: OperationModule::CDriveCleanup,
        started_at: started_at.into(),
        finished_at: result.finished_at.clone(),
        total_bytes: result.total_freed_bytes,
        freed_bytes: result.total_freed_bytes,
        c_drive_freed_bytes: result.total_freed_bytes,
        other_drive_freed_bytes: 0,
        success_count,
        skipped_count,
        failed_count,
        error_categories,
    }
}
