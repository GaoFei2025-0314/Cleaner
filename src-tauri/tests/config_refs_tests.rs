use std::fs;

use c_drive_cleaner::config_refs::{find_config_references, ConfigSearchRoots};

#[test]
fn detects_candidate_path_inside_plain_text_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let codex_dir = user_profile.join(".codex");
    let candidate = user_profile
        .join(".vscode")
        .join("extensions")
        .join("highagency.pencildev-0.6.51");

    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::write(
        codex_dir.join("config.toml"),
        format!("command = '{}\\\\out\\\\mcp-server-windows-x64.exe'", candidate.display()),
    )
    .expect("config");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert_eq!(refs.len(), 1);
    assert!(refs[0].display_name.contains(".codex"));
}

#[test]
fn detects_candidate_path_inside_root_claude_json_with_escaped_backslashes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let candidate = user_profile
        .join("AppData")
        .join("Local")
        .join("npm-cache")
        .join("_npx")
        .join("abc");
    let escaped = candidate.to_string_lossy().replace('\\', "\\\\");

    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::create_dir_all(&user_profile).expect("user profile");
    fs::write(
        user_profile.join(".claude.json"),
        format!(r#"{{"args":["{}\\node_modules\\exa-mcp-server\\index.cjs"]}}"#, escaped),
    )
    .expect("claude json");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert_eq!(refs.len(), 1);
    assert!(refs[0].display_name.contains(".claude.json"));
}

#[test]
fn ignores_binary_like_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let codex_dir = user_profile.join(".codex");
    let candidate = user_profile.join("AppData").join("Local").join("npm-cache");

    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::write(codex_dir.join("blob.bin"), [0_u8, 159, 146, 150]).expect("binary");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert!(refs.is_empty());
}
