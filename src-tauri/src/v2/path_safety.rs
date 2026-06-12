use std::path::{Path, PathBuf};

const SYSTEM_PATHS: &[&str] = &[
    r"C:\Windows",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData",
];

pub fn is_protected_duplicate_path(path: &Path, protected_paths: &[String]) -> bool {
    let path_key = canonical_path_key(path);
    SYSTEM_PATHS
        .iter()
        .any(|system_path| path_is_same_or_child(&path_key, &normalized_string_key(system_path)))
        || protected_paths.iter().any(|protected_path| {
            path_is_same_or_child(&path_key, &protected_path_key(Path::new(protected_path)))
        })
}

pub fn should_skip_scan_location(_path: &Path, _protected_paths: &[String]) -> bool {
    false
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
    let display = normalized_string_key(&path.display().to_string());
    let bytes = display.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        format!("{}:", (bytes[0] as char).to_ascii_uppercase())
    } else {
        String::new()
    }
}

pub fn canonical_path_key(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .map(|canonical| normalized_path_key(&canonical))
        .unwrap_or_else(|| normalized_path_key(path))
}

pub fn normalized_existing_or_logical_path_key(path: &Path) -> String {
    path.canonicalize()
        .ok()
        .map(|canonical| normalized_path_key(&canonical))
        .unwrap_or_else(|| fold_logical_path_key(&normalized_path_key(path)))
}

pub fn safe_target_path_key(path: &Path) -> String {
    let Some(existing_ancestor) = deepest_existing_ancestor(path) else {
        return normalized_existing_or_logical_path_key(path);
    };
    let Ok(remaining) = path.strip_prefix(&existing_ancestor) else {
        return normalized_existing_or_logical_path_key(path);
    };
    let ancestor_key = normalized_existing_or_logical_path_key(&existing_ancestor);
    let remaining_key = fold_logical_path_key(&normalized_path_key(remaining));
    if remaining_key.is_empty() {
        ancestor_key
    } else {
        fold_logical_path_key(&join_logical_path_key(&ancestor_key, &remaining_key))
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

fn protected_path_key(path: &Path) -> String {
    normalized_existing_or_logical_path_key(path)
}

fn normalized_path_key(path: &Path) -> String {
    normalized_string_key(&path.display().to_string())
}

fn deepest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| ancestor.exists())
        .map(Path::to_path_buf)
}

fn join_logical_path_key(parent_key: &str, child_key: &str) -> String {
    if parent_key.is_empty() {
        child_key.to_string()
    } else if child_key.is_empty() {
        parent_key.to_string()
    } else {
        format!("{parent_key}\\{child_key}")
    }
}

fn normalized_string_key(path: &str) -> String {
    let mut normalized = path
        .trim()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_ascii_lowercase();

    if let Some(remainder) = normalized
        .strip_prefix(r"\\?\unc\")
        .or_else(|| normalized.strip_prefix(r"\??\unc\"))
    {
        return format!(r"\\{remainder}");
    }

    loop {
        let stripped = normalized
            .strip_prefix(r"\\?\")
            .or_else(|| normalized.strip_prefix(r"\??\"));
        if let Some(remainder) = stripped {
            normalized = remainder.to_string();
        } else {
            break;
        }
    }

    normalized
}

fn fold_logical_path_key(path_key: &str) -> String {
    let (prefix, rest, rooted) = split_logical_path_prefix(path_key);
    let mut segments = Vec::new();

    for segment in rest.split('\\') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if let Some(last) = segments.last() {
                if last != ".." {
                    segments.pop();
                    continue;
                }
            }
            if !rooted {
                segments.push(segment.to_string());
            }
            continue;
        }
        segments.push(segment.to_string());
    }

    let tail = segments.join("\\");
    if prefix.is_empty() {
        tail
    } else if tail.is_empty() {
        prefix
    } else if rooted {
        format!("{prefix}\\{tail}")
    } else {
        format!("{prefix}{tail}")
    }
}

fn split_logical_path_prefix(path_key: &str) -> (String, &str, bool) {
    if let Some(rest) = path_key.strip_prefix(r"\\") {
        let mut parts = rest.split('\\');
        let server = parts.next().unwrap_or_default();
        let share = parts.next().unwrap_or_default();
        if !server.is_empty() && !share.is_empty() {
            let prefix = format!(r"\\{server}\{share}");
            let rest = path_key
                .get(prefix.len()..)
                .unwrap_or_default()
                .trim_start_matches('\\');
            return (prefix, rest, true);
        }
        return (r"\\".to_string(), rest, true);
    }

    let bytes = path_key.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let prefix = path_key[..2].to_string();
        let rest = path_key[2..].trim_start_matches('\\');
        return (prefix, rest, true);
    }

    if let Some(rest) = path_key.strip_prefix('\\') {
        return ("\\".to_string(), rest, true);
    }

    (String::new(), path_key, false)
}
