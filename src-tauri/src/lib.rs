pub mod admin;
pub mod analytics;
pub mod cleanup;
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod processes;
pub mod rules;
pub mod scan;
pub mod size;

use admin::AdminCleanupCapability;
use models::{CleanupResult, CleanupSelection, ScanReport};
use paths::ScanRoots;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

#[tauri::command]
fn scan_c_drive() -> Result<ScanReport, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    Ok(scan::scan_with_roots(&roots, drive_summary))
}

#[tauri::command]
fn get_admin_cleanup_capability() -> AdminCleanupCapability {
    admin::lightweight_admin_capability()
}

#[tauri::command]
fn execute_cleanup(selection: CleanupSelection) -> Result<CleanupResult, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    let report = scan::scan_with_roots(&roots, drive_summary);
    cleanup::execute_selected_cleanup(&selection, &report.items, &roots)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ping,
            get_admin_cleanup_capability,
            scan_c_drive,
            execute_cleanup
        ])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
