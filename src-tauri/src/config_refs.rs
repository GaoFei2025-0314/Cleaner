use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSearchRoots {
    pub user_profile: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigReference {
    pub display_name: String,
}

pub fn find_config_references(
    candidate_path: &Path,
    roots: &ConfigSearchRoots,
) -> Vec<ConfigReference> {
    let candidate = normalize_for_search(candidate_path);
    let known_dirs = [
        roots.user_profile.join(".codex"),
        roots.user_profile.join(".claude"),
        roots.user_profile.join(".cursor"),
        roots.user_profile.join(".vscode"),
        roots.user_profile.join(".trae"),
    ];
    let known_files = [
        roots.user_profile.join(".claude.json"),
        roots.user_profile.join(".codex.json"),
    ];

    let mut refs = Vec::new();
    for file in known_files {
        scan_config_file(&file, &candidate, roots, &mut refs);
    }

    for dir in known_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&dir)
            .max_depth(4)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            scan_config_file(entry.path(), &candidate, roots, &mut refs);
        }
    }

    refs
}

fn scan_config_file(
    path: &Path,
    candidate: &str,
    roots: &ConfigSearchRoots,
    refs: &mut Vec<ConfigReference>,
) {
    if !path.exists() || !is_plain_text_candidate(path) {
        return;
    }

    if let Ok(bytes) = fs::read(path) {
        if looks_binary(&bytes) {
            return;
        }
        let text = String::from_utf8_lossy(&bytes);
        if normalize_for_search_text(&text).contains(candidate) {
            refs.push(ConfigReference {
                display_name: path_display_without_username(path, &roots.user_profile),
            });
        }
    }
}

fn is_plain_text_candidate(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()).unwrap_or(""),
        "json" | "jsonl" | "toml" | "yaml" | "yml" | "ini" | "txt" | "config" | "conf"
    )
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(512).any(|byte| *byte == 0)
}

fn normalize_for_search(path: &Path) -> String {
    normalize_slashes(path.to_string_lossy().replace('/', "\\").to_lowercase())
}

fn normalize_for_search_text(text: &str) -> String {
    normalize_slashes(text.replace('/', "\\").to_lowercase())
}

fn normalize_slashes(input: String) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous_was_slash = false;
    for character in input.chars() {
        if character == '\\' {
            if !previous_was_slash {
                output.push(character);
            }
            previous_was_slash = true;
        } else {
            output.push(character);
            previous_was_slash = false;
        }
    }
    output
}

fn path_display_without_username(path: &Path, user_profile: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(user_profile) {
        format!("用户配置\\{}", relative.display())
    } else {
        "用户配置文件".to_string()
    }
}
