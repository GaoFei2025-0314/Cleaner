use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::v2::models::HistoryEntry;

const HISTORY_FILE_NAME: &str = "history.json";
const COMMON_FILE_EXTENSIONS: &[&str] = &[
    "7z", "avi", "bmp", "csv", "dll", "doc", "docx", "exe", "gif", "gz", "iso", "jpeg", "jpg",
    "json", "log", "m4a", "mkv", "mov", "mp3", "mp4", "msi", "pdf", "png", "ppt", "pptx", "rar",
    "tar", "tgz", "txt", "webp", "xls", "xlsx", "xml", "zip",
];

pub fn list_operation_history(app_handle: &AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let history_path = history_file_path(app_handle)?;
    list_operation_history_at_path(&history_path)
}

pub fn list_operation_history_at_path(history_path: &Path) -> Result<Vec<HistoryEntry>, String> {
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
    clear_operation_history_at_path(&history_path)
}

pub fn clear_operation_history_at_path(history_path: &Path) -> Result<(), String> {
    match fs::remove_file(history_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("无法清空历史记录".to_string()),
    }
}

pub fn append_history_entry(
    app_handle: &AppHandle,
    entry: HistoryEntry,
) -> Result<HistoryEntry, String> {
    let history_path = history_file_path(app_handle)?;
    append_history_entry_at_path(&history_path, entry)
}

pub fn append_history_entry_at_path(
    history_path: &Path,
    entry: HistoryEntry,
) -> Result<HistoryEntry, String> {
    if !history_entry_is_desensitized(&entry) {
        return Err("历史记录包含未脱敏内容".to_string());
    }

    let mut entries = list_operation_history_at_path(history_path)?;
    entries.push(entry.clone());
    save_operation_history_at_path(history_path, &entries)?;
    Ok(entry)
}

pub fn save_operation_history(
    app_handle: &AppHandle,
    entries: &[HistoryEntry],
) -> Result<(), String> {
    let history_path = history_file_path(app_handle)?;
    save_operation_history_at_path(&history_path, entries)
}

pub fn save_operation_history_at_path(
    history_path: &Path,
    entries: &[HistoryEntry],
) -> Result<(), String> {
    if !entries.iter().all(history_entry_is_desensitized) {
        return Err("历史记录包含未脱敏内容".to_string());
    }

    if let Some(parent) = history_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| "无法保存历史记录".to_string())?;
        }
    }

    let content =
        serde_json::to_string_pretty(entries).map_err(|_| "无法保存历史记录".to_string())?;
    fs::write(history_path, content).map_err(|_| "无法保存历史记录".to_string())
}

pub fn history_entry_is_desensitized(entry: &HistoryEntry) -> bool {
    text_is_desensitized(&entry.history_id)
        && text_is_desensitized(&entry.started_at)
        && text_is_desensitized(&entry.finished_at)
        && entry
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

    if contains_common_extension_filename(trimmed) {
        return false;
    }

    !trimmed
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '.')
        .filter(|token| !token.is_empty())
        .any(token_contains_sensitive_value)
}

fn token_contains_sensitive_value(token: &str) -> bool {
    looks_like_raw_hash(token)
}

fn contains_common_extension_filename(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    COMMON_FILE_EXTENSIONS
        .iter()
        .any(|extension| lower.contains(&format!(".{extension}")))
}

fn looks_like_raw_hash(token: &str) -> bool {
    (32..=128).contains(&token.len())
        && token.chars().all(|character| character.is_ascii_hexdigit())
}
