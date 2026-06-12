use std::path::{Path, PathBuf};

const SYSTEM_PATHS: &[&str] = &[
    r"C:\Windows",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData",
];

pub fn is_protected_duplicate_path(path: &Path, protected_paths: &[String]) -> bool {
    let path_key = normalized_path_key(path);
    SYSTEM_PATHS
        .iter()
        .any(|system_path| path_is_same_or_child(&path_key, &normalized_string_key(system_path)))
        || protected_paths.iter().any(|protected_path| {
            path_is_same_or_child(&path_key, &normalized_string_key(protected_path))
        })
}

pub fn should_skip_scan_location(path: &Path, _protected_paths: &[String]) -> bool {
    let path_key = normalized_path_key(path);
    SYSTEM_PATHS
        .iter()
        .any(|system_path| path_is_same_or_child(&path_key, &normalized_string_key(system_path)))
}

pub fn selected_drive_to_root(drive: &str) -> Option<PathBuf> {
    let trimmed = drive.trim();
    if trimmed.len() == 2
        && trimmed.as_bytes()[0].is_ascii_alphabetic()
        && trimmed.as_bytes()[1] == b':'
    {
        Some(PathBuf::from(format!("{}\\", trimmed.to_ascii_uppercase())))
    } else if trimmed.len() == 1 && trimmed.as_bytes()[0].is_ascii_alphabetic() {
        Some(PathBuf::from(format!(
            "{}:\\",
            trimmed.to_ascii_uppercase()
        )))
    } else {
        None
    }
}

pub fn drive_label(path: &Path) -> String {
    let display = path.display().to_string();
    let bytes = display.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        format!("{}:", (bytes[0] as char).to_ascii_uppercase())
    } else {
        String::new()
    }
}

fn path_is_same_or_child(path_key: &str, protected_key: &str) -> bool {
    if protected_key.is_empty() {
        return false;
    }

    path_key == protected_key
        || path_key
            .strip_prefix(protected_key)
            .is_some_and(|tail| tail.starts_with('\\'))
}

fn normalized_path_key(path: &Path) -> String {
    normalized_string_key(&path.display().to_string())
}

fn normalized_string_key(path: &str) -> String {
    path.trim()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_ascii_lowercase()
}
