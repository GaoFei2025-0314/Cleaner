use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::config_refs::{find_config_references, ConfigSearchRoots};
use crate::errors::CleanerError;
use crate::fixtures::now_iso;
use crate::models::{
    CleanupAction, CleanupItemResult, CleanupResult, CleanupSelection, RiskLevel, ScanItem,
};
use crate::paths::{ensure_under_root, resolve_rule_path, root_for_rule, ScanRoots};
use crate::processes::find_process_references;
use crate::rules::builtin_rules;

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

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
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
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(freed)
}

pub fn execute_selected_cleanup(
    selection: &CleanupSelection,
    items: &[ScanItem],
    roots: &ScanRoots,
) -> Result<CleanupResult, String> {
    validate_high_risk_confirmation(selection, items)?;
    let selected: HashSet<&str> = selection
        .selected_item_ids
        .iter()
        .map(String::as_str)
        .collect();
    let rules = builtin_rules();
    let mut results = Vec::new();

    for item in items.iter().filter(|item| selected.contains(item.id.as_str())) {
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
        if Path::new(path) != expected_path {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：扫描路径和规则路径不一致。", item.title),
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

        if let Err(error) = ensure_under_root(&expected_path, &root_for_rule(rule, roots)) {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：路径安全校验未通过：{}。", item.title, error),
            });
            continue;
        }

        if !find_config_references(
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
                message: format!("{} 已跳过：清理前复查发现仍被运行中的程序使用。", item.title),
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

        match delete_path_or_contents(&expected_path, rule.delete_contents_only) {
            Ok(freed) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "deleted".to_string(),
                freed_bytes: freed,
                message: format!("{} 已清理。", item.title),
            }),
            Err(error) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：{}。", item.title, error),
            }),
        }
    }

    Ok(build_cleanup_result(results))
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
        return path.metadata().ok().and_then(|metadata| metadata.modified().ok());
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
