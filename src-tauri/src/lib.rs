pub mod fixtures;
pub mod models;
pub mod rules;

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
