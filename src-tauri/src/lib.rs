pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod rules;
pub mod size;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
