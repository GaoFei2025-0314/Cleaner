use crate::config_refs::{find_config_references, ConfigSearchRoots};
use crate::fixtures::now_iso;
use crate::models::{CleanupAction, DriveSummary, RiskLevel, ScanItem, ScanReport};
use crate::paths::{resolve_rule_path, ScanRoots};
use crate::processes::find_process_references;
use crate::rules::{builtin_rules, CleanupRule};
use crate::size::path_size_bytes;

pub fn scan_with_roots(roots: &ScanRoots, drive_summary: DriveSummary) -> ScanReport {
    let started = now_iso();
    let mut items = Vec::new();

    for rule in builtin_rules() {
        let path = resolve_rule_path(&rule, roots);
        let estimated_bytes = path_size_bytes(&path);
        if estimated_bytes == 0 && !path.exists() {
            continue;
        }
        items.push(build_item(
            rule,
            roots,
            path.to_string_lossy().to_string(),
            estimated_bytes,
        ));
    }

    ScanReport {
        drive_summary,
        items,
        partial: false,
        scan_started_at: started,
        scan_finished_at: now_iso(),
    }
}

fn build_item(
    rule: CleanupRule,
    roots: &ScanRoots,
    technical_path: String,
    estimated_bytes: u64,
) -> ScanItem {
    let config_refs = find_config_references(
        std::path::Path::new(&technical_path),
        &ConfigSearchRoots {
            user_profile: roots.user_profile.clone(),
        },
    );
    let process_refs = find_process_references(std::path::Path::new(&technical_path));

    let mut risk_level = rule.risk_level.clone();
    let mut cleanup_action = rule.cleanup_action.clone();
    let mut default_selected = rule.default_selected;
    let mut reasons = vec![format!("规则命中：{}", rule.title)];
    let mut warnings = Vec::new();

    if !config_refs.is_empty() {
        risk_level = RiskLevel::NotCleanable;
        cleanup_action = CleanupAction::BlockedByConfigReference;
        default_selected = false;
        reasons.push("这个位置正在被工具配置引用。".to_string());
        warnings.push("清理可能导致工具启动失败，已自动跳过。".to_string());
    } else if !process_refs.is_empty() {
        risk_level = RiskLevel::NotCleanable;
        cleanup_action = CleanupAction::BlockedByProcess;
        default_selected = false;
        reasons.push("这个位置正在被运行中的程序使用。".to_string());
        warnings.push("请关闭相关软件后重新扫描。".to_string());
    }

    ScanItem {
        id: rule.id.to_string(),
        title: rule.title.to_string(),
        description: rule.description.to_string(),
        source_category: rule.source_category,
        risk_level,
        cleanup_action,
        estimated_bytes,
        default_selected,
        user_visible_path_hint: user_visible_hint(rule.id),
        technical_path: Some(technical_path),
        reasons,
        warnings,
    }
}

fn user_visible_hint(rule_id: &str) -> String {
    match rule_id {
        "user-temp" => "当前用户临时目录".to_string(),
        "windows-temp" => "Windows 临时目录".to_string(),
        "windows-update-download" => "Windows 更新下载缓存".to_string(),
        "wechat-data-root" => "微信用户数据根目录".to_string(),
        "qq-data-root" => "QQ 用户数据根目录".to_string(),
        "vscode-cached-vsix" => "VS Code 扩展安装包缓存".to_string(),
        _ => "C 盘应用数据目录".to_string(),
    }
}
