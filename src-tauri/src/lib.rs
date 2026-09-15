pub mod foundation;
pub mod platform;
pub mod storage;

use std::sync::Arc;

use foundation::FoundationStatus;
use platform::PlatformAdapter;

#[tauri::command]
fn foundation_status(adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>) -> FoundationStatus {
    FoundationStatus::from_platform(adapter.inner().as_ref())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let adapter = platform::create_platform_adapter();
    tauri::Builder::default()
        .manage(adapter)
        .invoke_handler(tauri::generate_handler![foundation_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
