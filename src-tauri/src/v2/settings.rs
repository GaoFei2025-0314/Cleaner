use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::v2::models::{CleanerSettings, DuplicateDefaultStrategy};

const SETTINGS_FILE_NAME: &str = "settings.json";

pub fn default_settings() -> CleanerSettings {
    CleanerSettings {
        protected_paths: vec![],
        default_scan_drives: vec!["C:".to_string()],
        duplicate_default_strategy: DuplicateDefaultStrategy::CDriveFirstKeepNewest,
        large_file_default_threshold_bytes: 500 * 1024 * 1024,
        history_retention_days: 30,
        desktop_shortcut_enabled: false,
        c_drive_context_menu_enabled: false,
        scheduled_scan_reminder_enabled: false,
    }
}

pub fn sanitize_custom_extensions(input: &str) -> Vec<String> {
    let mut extensions = Vec::new();

    for raw_token in input.split([',', ';', '\n', '\r', '\t', ' ']) {
        let extension = raw_token
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();

        if is_safe_extension(&extension) && !extensions.contains(&extension) {
            extensions.push(extension);
        }
    }

    extensions
}

pub fn get_cleaner_settings(app_handle: &AppHandle) -> Result<CleanerSettings, String> {
    let settings_path = settings_file_path(app_handle)?;
    get_cleaner_settings_at_path(&settings_path)
}

pub fn get_cleaner_settings_at_path(settings_path: &Path) -> Result<CleanerSettings, String> {
    match fs::read_to_string(settings_path) {
        Ok(content) if content.trim().is_empty() => Ok(default_settings()),
        Ok(content) => serde_json::from_str(&content).map_err(|_| "设置文件格式无效".to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(default_settings()),
        Err(_) => Err("无法读取清理设置".to_string()),
    }
}

pub fn save_cleaner_settings(
    app_handle: &AppHandle,
    settings: CleanerSettings,
) -> Result<CleanerSettings, String> {
    let settings_path = settings_file_path(app_handle)?;
    save_cleaner_settings_at_path(&settings_path, settings)
}

pub fn save_cleaner_settings_at_path(
    settings_path: &Path,
    settings: CleanerSettings,
) -> Result<CleanerSettings, String> {
    if let Some(parent) = settings_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| "无法保存清理设置".to_string())?;
        }
    }
    let content =
        serde_json::to_string_pretty(&settings).map_err(|_| "无法保存清理设置".to_string())?;
    fs::write(settings_path, content).map_err(|_| "无法保存清理设置".to_string())?;

    Ok(settings)
}

fn settings_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|path| path.join(SETTINGS_FILE_NAME))
        .map_err(|_| "无法访问应用数据目录".to_string())
}

fn is_safe_extension(extension: &str) -> bool {
    if extension.is_empty() || extension.len() > 32 {
        return false;
    }

    extension.split('.').all(|segment| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
    })
}
