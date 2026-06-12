use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use c_drive_cleaner::cleanup::{
    delete_path_contents, execute_selected_cleanup, execute_selected_cleanup_with_recycle_bin,
    validate_high_risk_confirmation,
};
use c_drive_cleaner::models::{
    CleanupAction, CleanupSelection, RiskLevel, ScanItem, SourceCategory,
};
use c_drive_cleaner::paths::ScanRoots;
use c_drive_cleaner::v2::recycle_bin::{RecycleBin, RecycleBinError};

#[cfg(windows)]
use std::os::windows::fs::symlink_dir;

#[derive(Default)]
struct RecordingRecycleBin {
    paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl RecycleBin for RecordingRecycleBin {
    fn move_to_recycle_bin(&self, path: &Path) -> Result<(), RecycleBinError> {
        self.paths.lock().unwrap().push(path.to_path_buf());
        Ok(())
    }
}

struct FailingRecycleBin;

impl RecycleBin for FailingRecycleBin {
    fn move_to_recycle_bin(&self, _path: &Path) -> Result<(), RecycleBinError> {
        Err(RecycleBinError::Failed("access denied".to_string()))
    }
}

struct CleanupFixture {
    _temp: tempfile::TempDir,
    roots: ScanRoots,
    target: PathBuf,
}

#[cfg(windows)]
fn vscode_cached_vsix_fixture() -> CleanupFixture {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("User");
    let local_app_data = user_profile.join("AppData").join("Local");
    let windows_dir = temp.path().join("Windows");
    let target = user_profile
        .join("AppData")
        .join("Roaming")
        .join("Code")
        .join("CachedExtensionVSIXs");

    fs::create_dir_all(target.parent().expect("target parent")).expect("target parent");
    fs::create_dir_all(&local_app_data).expect("local app data");
    fs::create_dir_all(&windows_dir).expect("windows dir");
    fs::write(&target, "abc").expect("target file");
    make_old_file(&target);

    CleanupFixture {
        _temp: temp,
        roots: ScanRoots {
            c_drive: PathBuf::from(r"C:\"),
            user_profile,
            local_app_data,
            windows_dir,
        },
        target,
    }
}

#[cfg(windows)]
fn make_old_file(path: &Path) {
    let old = SystemTime::now()
        .checked_sub(Duration::from_secs(60 * 60))
        .expect("old timestamp");
    let file = File::options()
        .read(true)
        .write(true)
        .open(path)
        .expect("old file");
    let times = FileTimes::new().set_accessed(old).set_modified(old);
    file.set_times(times).expect("set old file timestamp");
}

#[cfg(windows)]
fn vscode_cached_vsix_item(path: &Path) -> ScanItem {
    ScanItem {
        id: "vscode-cached-vsix".to_string(),
        title: "VS Code 扩展安装包缓存".to_string(),
        description: "VS Code 下载扩展时留下的安装包缓存，可重新下载。".to_string(),
        source_category: SourceCategory::InstallersOldVersions,
        risk_level: RiskLevel::Recommended,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 3,
        default_selected: true,
        user_visible_path_hint: "VS Code 扩展安装包缓存".to_string(),
        technical_path: Some(path.to_string_lossy().into_owned()),
        reasons: vec![],
        warnings: vec![],
    }
}

#[test]
fn deletes_contents_without_deleting_parent_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Temp");
    fs::create_dir_all(&parent).expect("parent");
    fs::write(parent.join("old.log"), "abc").expect("file");

    let freed = delete_path_contents(&parent).expect("deleted");

    assert!(parent.exists());
    assert!(!parent.join("old.log").exists());
    assert_eq!(freed, 3);
}

#[test]
fn rejects_high_risk_without_second_confirmation() {
    let item = ScanItem {
        id: "wechat-video-cache".to_string(),
        title: "微信视频缓存子目录".to_string(),
        description: "精确命中的微信视频缓存。".to_string(),
        source_category: SourceCategory::Wechat,
        risk_level: RiskLevel::HighRisk,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 10,
        default_selected: false,
        user_visible_path_hint: "微信视频缓存子目录".to_string(),
        technical_path: None,
        reasons: vec![],
        warnings: vec![],
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["wechat-video-cache".to_string()],
        high_risk_confirmed: false,
    };

    assert!(validate_high_risk_confirmation(&selection, &[item]).is_err());
}

#[test]
fn reports_selected_item_that_disappeared_before_cleanup() {
    let roots = ScanRoots {
        c_drive: PathBuf::from(r"C:\"),
        user_profile: PathBuf::from(r"C:\Users\Example"),
        local_app_data: PathBuf::from(r"C:\Users\Example\AppData\Local"),
        windows_dir: PathBuf::from(r"C:\Windows"),
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["user-temp".to_string()],
        high_risk_confirmed: false,
    };

    let result = execute_selected_cleanup(&selection, &[], &roots).expect("cleanup result");

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].item_id, "user-temp");
    assert_eq!(result.results[0].status, "skipped");
}

#[test]
fn rejects_selected_cleanup_outside_c_drive_even_when_rule_matches() {
    let roots = ScanRoots {
        c_drive: PathBuf::from(r"C:\"),
        user_profile: PathBuf::from(r"D:\Users\Example"),
        local_app_data: PathBuf::from(r"D:\Users\Example\AppData\Local"),
        windows_dir: PathBuf::from(r"C:\Windows"),
    };
    let item = ScanItem {
        id: "user-temp".to_string(),
        title: "用户临时文件".to_string(),
        description: "临时文件".to_string(),
        source_category: SourceCategory::System,
        risk_level: RiskLevel::Recommended,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 10,
        default_selected: true,
        user_visible_path_hint: "当前用户临时目录".to_string(),
        technical_path: Some(r"D:\Users\Example\AppData\Local\Temp".to_string()),
        reasons: vec![],
        warnings: vec![],
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["user-temp".to_string()],
        high_risk_confirmed: false,
    };

    let result = execute_selected_cleanup(&selection, &[item], &roots).expect("cleanup result");

    assert_eq!(result.results[0].status, "failed");
    assert!(result.results[0].message.contains("C"));
}

#[test]
#[cfg(windows)]
fn execute_selected_cleanup_reports_recycle_bin_success() {
    let fixture = vscode_cached_vsix_fixture();
    let item = vscode_cached_vsix_item(&fixture.target);
    let selection = CleanupSelection {
        selected_item_ids: vec!["vscode-cached-vsix".to_string()],
        high_risk_confirmed: false,
    };
    let recycle_bin = RecordingRecycleBin::default();

    let result = execute_selected_cleanup_with_recycle_bin(
        &selection,
        &[item],
        &fixture.roots,
        &recycle_bin,
    )
    .expect("cleanup result");

    assert_eq!(result.results[0].status, "deleted");
    assert_eq!(result.results[0].freed_bytes, 3);
    assert_eq!(
        result.results[0].message,
        "VS Code 扩展安装包缓存 已移入回收站。"
    );
    assert_eq!(recycle_bin.paths.lock().unwrap()[0], fixture.target);
}

#[test]
#[cfg(windows)]
fn execute_selected_cleanup_reports_recycle_bin_failure() {
    let fixture = vscode_cached_vsix_fixture();
    let item = vscode_cached_vsix_item(&fixture.target);
    let selection = CleanupSelection {
        selected_item_ids: vec!["vscode-cached-vsix".to_string()],
        high_risk_confirmed: false,
    };

    let result = execute_selected_cleanup_with_recycle_bin(
        &selection,
        &[item],
        &fixture.roots,
        &FailingRecycleBin,
    )
    .expect("cleanup result");

    assert_eq!(result.results[0].status, "failed");
    assert_eq!(result.results[0].freed_bytes, 0);
    assert_eq!(
        result.results[0].message,
        "VS Code 扩展安装包缓存 移入回收站失败，本项未清理。"
    );
}

#[test]
#[cfg(windows)]
fn delete_path_contents_skips_directory_symlinks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Temp");
    let outside = temp.path().join("Outside");
    let link = parent.join("linked-dir");
    fs::create_dir_all(&parent).expect("parent");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), "keep").expect("outside file");

    if symlink_dir(&outside, &link).is_err() {
        return;
    }

    delete_path_contents(&parent).expect("deleted");

    assert!(outside.join("keep.txt").exists());
    assert!(link.exists());
}
