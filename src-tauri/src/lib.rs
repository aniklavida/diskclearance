mod foundation;
mod platform;
mod storage;

use foundation::FoundationStatus;

#[tauri::command]
fn foundation_status() -> FoundationStatus {
    FoundationStatus::current()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![foundation_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
