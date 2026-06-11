use crate::models::{CleanupAction, RiskLevel, SourceCategory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleScope {
    UserLocalAppDataRelative(&'static str),
    UserProfileRelative(&'static str),
    WindowsRelative(&'static str),
    Absolute(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupRule {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub source_category: SourceCategory,
    pub risk_level: RiskLevel,
    pub cleanup_action: CleanupAction,
    pub default_selected: bool,
    pub scope: RuleScope,
    pub delete_contents_only: bool,
    pub min_age_minutes: u64,
}

pub fn builtin_rules() -> Vec<CleanupRule> {
    vec![
        CleanupRule {
            id: "user-temp",
            title: "用户临时文件",
            description: "软件运行时留下的临时材料，通常可以安全删除。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::DirectDelete,
            default_selected: true,
            scope: RuleScope::UserLocalAppDataRelative("Temp"),
            delete_contents_only: true,
            min_age_minutes: 10,
        },
        CleanupRule {
            id: "windows-temp",
            title: "Windows 临时文件",
            description: "系统和安装程序留下的临时材料，需要管理员权限；V0.1 只展示能力说明，不执行提权清理。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::RequiresAdmin,
            default_selected: false,
            scope: RuleScope::WindowsRelative("Temp"),
            delete_contents_only: true,
            min_age_minutes: 30,
        },
        CleanupRule {
            id: "windows-update-download",
            title: "Windows 更新下载缓存",
            description: "Windows 更新下载后的缓存文件，需要管理员权限；V0.1 只展示能力说明，不执行提权清理。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::RequiresAdmin,
            default_selected: false,
            scope: RuleScope::WindowsRelative("SoftwareDistribution\\Download"),
            delete_contents_only: true,
            min_age_minutes: 60,
        },
        CleanupRule {
            id: "wechat-data-root",
            title: "微信数据根目录",
            description: "微信数据根目录可能包含聊天数据库、图片、视频和文件。V0.1 不提供整目录删除。",
            source_category: SourceCategory::Wechat,
            risk_level: RiskLevel::NotCleanable,
            cleanup_action: CleanupAction::ExplainOnly,
            default_selected: false,
            scope: RuleScope::UserProfileRelative("Documents\\WeChat Files"),
            delete_contents_only: false,
            min_age_minutes: 0,
        },
        CleanupRule {
            id: "qq-data-root",
            title: "QQ 数据根目录",
            description: "QQ 数据根目录可能包含聊天数据库、图片、视频、群文件和下载文件。V0.1 不提供整目录删除。",
            source_category: SourceCategory::Qq,
            risk_level: RiskLevel::NotCleanable,
            cleanup_action: CleanupAction::ExplainOnly,
            default_selected: false,
            scope: RuleScope::UserProfileRelative("Documents\\Tencent Files"),
            delete_contents_only: false,
            min_age_minutes: 0,
        },
        CleanupRule {
            id: "vscode-cached-vsix",
            title: "VS Code 扩展安装包缓存",
            description: "VS Code 下载扩展时留下的安装包缓存，可重新下载。",
            source_category: SourceCategory::InstallersOldVersions,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::DirectDelete,
            default_selected: true,
            scope: RuleScope::UserProfileRelative("AppData\\Roaming\\Code\\CachedExtensionVSIXs"),
            delete_contents_only: false,
            min_age_minutes: 10,
        },
    ]
}
