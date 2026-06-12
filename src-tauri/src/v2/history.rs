use std::fs;
use std::io;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::v2::models::HistoryEntry;

const HISTORY_FILE_NAME: &str = "history.json";

pub fn list_operation_history(app_handle: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let history_path = history_file_path(app_handle)?;

    let entries = match fs::read_to_string(history_path) {
        Ok(content) if content.trim().is_empty() => Vec::new(),
        Ok(content) => serde_json::from_str::<Vec<HistoryEntry>>(&content)
            .map_err(|_| "历史记录格式无效".to_string())?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err("无法读取历史记录".to_string()),
    };

    if entries.iter().all(history_entry_is_desensitized) {
        Ok(entries)
    } else {
        Err("历史记录包含未脱敏内容".to_string())
    }
}

pub fn clear_operation_history(app_handle: &AppHandle) -> Result<(), String> {
    let history_path = history_file_path(app_handle)?;

    match fs::remove_file(history_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("无法清空历史记录".to_string()),
    }
}

pub fn history_entry_is_desensitized(entry: &HistoryEntry) -> bool {
    entry
        .error_categories
        .iter()
        .all(|category| text_is_desensitized(category))
}

fn history_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|path| path.join(HISTORY_FILE_NAME))
        .map_err(|_| "无法访问应用数据目录".to_string())
}

fn text_is_desensitized(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains(":\\")
        || lower.contains(":/")
        || lower.contains('\\')
        || lower.contains('/')
        || lower.contains("administrator")
    {
        return false;
    }

    if let Ok(username) = std::env::var("USERNAME") {
        if !username.trim().is_empty() && lower.contains(&username.trim().to_ascii_lowercase()) {
            return false;
        }
    }

    !trimmed
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .filter(|token| !token.is_empty())
        .any(token_contains_sensitive_value)
}

fn token_contains_sensitive_value(token: &str) -> bool {
    looks_like_file_name(token) || looks_like_raw_hash(token)
}

fn looks_like_file_name(token: &str) -> bool {
    let Some((stem, extension)) = token.rsplit_once('.') else {
        return false;
    };

    !stem.is_empty()
        && (1..=8).contains(&extension.len())
        && extension
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn looks_like_raw_hash(token: &str) -> bool {
    (32..=128).contains(&token.len())
        && token.chars().all(|character| character.is_ascii_hexdigit())
}
