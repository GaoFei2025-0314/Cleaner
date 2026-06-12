use std::path::{Component, Path, PathBuf, Prefix};

use crate::errors::CleanerError;
use crate::rules::{CleanupRule, RuleScope};

#[derive(Debug, Clone)]
pub struct ScanRoots {
    pub c_drive: PathBuf,
    pub user_profile: PathBuf,
    pub local_app_data: PathBuf,
    pub windows_dir: PathBuf,
}

impl ScanRoots {
    pub fn from_current_user() -> Result<Self, CleanerError> {
        let user_profile = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or_else(|| CleanerError::PathResolution("USERPROFILE is not set".to_string()))?;
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| CleanerError::PathResolution("LOCALAPPDATA is not set".to_string()))?;
        let roots = Self {
            c_drive: PathBuf::from(r"C:\"),
            user_profile,
            local_app_data,
            windows_dir: PathBuf::from(r"C:\Windows"),
        };
        roots.ensure_c_drive_scope()?;
        Ok(roots)
    }

    pub fn ensure_c_drive_scope(&self) -> Result<(), CleanerError> {
        ensure_c_drive_path(&self.c_drive)?;
        ensure_c_drive_path(&self.user_profile)?;
        ensure_c_drive_path(&self.local_app_data)?;
        ensure_c_drive_path(&self.windows_dir)?;
        Ok(())
    }
}

pub fn resolve_rule_path(rule: &CleanupRule, roots: &ScanRoots) -> PathBuf {
    match &rule.scope {
        RuleScope::UserLocalAppDataRelative(relative) => roots.local_app_data.join(relative),
        RuleScope::UserProfileRelative(relative) => roots.user_profile.join(relative),
        RuleScope::WindowsRelative(relative) => roots.windows_dir.join(relative),
        RuleScope::Absolute(path) => PathBuf::from(path),
    }
}

pub fn root_for_rule(rule: &CleanupRule, roots: &ScanRoots) -> PathBuf {
    match &rule.scope {
        RuleScope::UserLocalAppDataRelative(_) => roots.local_app_data.clone(),
        RuleScope::UserProfileRelative(_) => roots.user_profile.clone(),
        RuleScope::WindowsRelative(_) => roots.windows_dir.clone(),
        RuleScope::Absolute(_) => roots.c_drive.clone(),
    }
}

pub fn ensure_under_root(path: &Path, root: &Path) -> Result<(), CleanerError> {
    let path = path.canonicalize()?;
    let root = root.canonicalize()?;
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(CleanerError::PathOutsideAllowedRoot)
    }
}

pub fn ensure_c_drive_path(path: &Path) -> Result<(), CleanerError> {
    if is_c_drive_path(path) {
        Ok(())
    } else {
        Err(CleanerError::PathOutsideCDrive)
    }
}

pub fn is_c_drive_path(path: &Path) -> bool {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return false;
    };
    let Some(Component::RootDir) = components.next() else {
        return false;
    };

    matches!(
        prefix.kind(),
        Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) if letter.eq_ignore_ascii_case(&b'C')
    )
}
