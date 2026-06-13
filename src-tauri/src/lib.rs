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
pub mod v2;

use models::{CleanupResult, CleanupSelection, ScanReport};
use paths::ScanRoots;
use v2::duplicate::DuplicateEntryRegistry;
use v2::large_files::LargeFileRegistry;
use v2::models::{
    CleanerSettings, DuplicateCleanupRequest, DuplicateScanRequest, HistoryEntry,
    LargeFileScanRequest, MigrationRequest, OperationStart,
};
use v2::operations::OperationRegistry;

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
fn execute_cleanup(
    app_handle: tauri::AppHandle,
    selection: CleanupSelection,
) -> Result<CleanupResult, String> {
    let started_at = fixtures::now_iso();
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    let report = scan::scan_with_roots(&roots, drive_summary);
    let result = cleanup::execute_selected_cleanup(&selection, &report.items, &roots)?;
    let history_entry = cleanup::build_c_drive_cleanup_history_entry(&result, started_at);
    let _ = v2::history::append_history_entry(&app_handle, history_entry);
    Ok(result)
}

#[tauri::command]
fn cancel_operation(operation_id: String, operations: tauri::State<'_, OperationRegistry>) -> bool {
    operations.cancel(&operation_id)
}

#[tauri::command]
fn get_cleaner_settings(app_handle: tauri::AppHandle) -> Result<CleanerSettings, String> {
    v2::settings::get_cleaner_settings(&app_handle)
}

#[tauri::command]
fn save_cleaner_settings(
    app_handle: tauri::AppHandle,
    settings: CleanerSettings,
) -> Result<CleanerSettings, String> {
    v2::settings::save_cleaner_settings(&app_handle, settings)
}

#[tauri::command]
fn list_operation_history(app_handle: tauri::AppHandle) -> Result<Vec<HistoryEntry>, String> {
    v2::history::list_operation_history(&app_handle)
}

#[tauri::command]
fn clear_operation_history(app_handle: tauri::AppHandle) -> Result<(), String> {
    v2::history::clear_operation_history(&app_handle)
}

#[tauri::command]
fn start_duplicate_scan(
    app_handle: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: DuplicateScanRequest,
) -> Result<OperationStart, String> {
    v2::duplicate::start_duplicate_scan(app_handle, operations, request)
}

#[tauri::command]
fn start_duplicate_cleanup(
    app_handle: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: DuplicateCleanupRequest,
) -> Result<OperationStart, String> {
    v2::duplicate::start_duplicate_cleanup(app_handle, operations, request)
}

#[tauri::command]
fn start_large_file_scan(
    app_handle: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: LargeFileScanRequest,
) -> Result<OperationStart, String> {
    v2::large_files::start_large_file_scan(app_handle, operations, request)
}

#[tauri::command]
fn start_large_file_migration(
    app_handle: tauri::AppHandle,
    operations: tauri::State<'_, OperationRegistry>,
    request: MigrationRequest,
) -> Result<OperationStart, String> {
    v2::migration::start_large_file_migration(app_handle, operations, request)
}

pub fn run() {
    tauri::Builder::default()
        .manage(OperationRegistry::default())
        .manage(DuplicateEntryRegistry::default())
        .manage(LargeFileRegistry::default())
        .invoke_handler(tauri::generate_handler![
            ping,
            scan_c_drive,
            execute_cleanup,
            cancel_operation,
            get_cleaner_settings,
            save_cleaner_settings,
            list_operation_history,
            clear_operation_history,
            start_duplicate_scan,
            start_duplicate_cleanup,
            start_large_file_scan,
            start_large_file_migration
        ])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
