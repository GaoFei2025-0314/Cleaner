use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminCleanupCapability {
    pub available: bool,
    pub title: String,
    pub description: String,
    pub supported_items: Vec<String>,
}

pub fn lightweight_admin_capability() -> AdminCleanupCapability {
    AdminCleanupCapability {
        available: false,
        title: "系统轻量清理（V0.2 计划）".to_string(),
        description: "V0.1 只展示能力说明，不执行提权清理；后续版本可在用户主动授权后清理 Windows 临时目录、Windows 更新下载缓存和安全系统日志。"
            .to_string(),
        supported_items: vec![
            "Windows 临时目录".to_string(),
            "Windows 更新下载缓存".to_string(),
            "系统日志".to_string(),
        ],
    }
}
