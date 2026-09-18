pub mod boundary;
pub mod classify;
pub mod foundation;
pub mod platform;
pub mod scan;
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
    let database = match storage::open_database_for_adapter(adapter.as_ref()) {
        Ok(db) => db,
        Err(storage::StorageError::Platform(platform::PlatformError::Unsupported(_))) => {
            storage::AppDatabase::open_in_memory().expect("failed to open in-memory database")
        }
        Err(err) => panic!("failed to open application database: {err}"),
    };
    let cancellations = boundary::CancellationRegistry::new();

    tauri::Builder::default()
        .manage(adapter)
        .manage(database)
        .manage(cancellations)
        .invoke_handler(tauri::generate_handler![
            foundation_status,
            boundary::read::start_scan,
            boundary::read::cancel_scan,
            boundary::read::fetch_findings_page,
            boundary::read::fetch_folder_aggregate,
            boundary::read::fetch_application_inventory,
            boundary::read::fetch_storage_reclamation_report,
            boundary::plan::build_plan,
            boundary::plan::fetch_plan,
            boundary::plan::revalidate_plan,
            boundary::destructive::execute_plan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
