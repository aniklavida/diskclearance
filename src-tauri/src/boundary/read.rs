use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::boundary::cancellation::CancellationRegistry;
use crate::boundary::error::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StartScanArgs {
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScanSessionHeader {
    pub session_id: String,
    pub started_at_ms: u64,
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanArgs {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanResult {
    pub session_id: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchFindingsPageArgs {
    pub session_id: String,
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FindingItem {
    pub id: String,
    pub path: String,
    pub size_bytes: u64,
    pub category: String,
    pub action_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FindingsPage {
    pub session_id: String,
    pub items: Vec<FindingItem>,
    pub next_cursor: Option<String>,
    pub total_estimated: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchFolderAggregateArgs {
    pub session_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderAggregateEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderAggregate {
    pub path: String,
    pub total_size_bytes: u64,
    pub file_count: u64,
    pub children: Vec<FolderAggregateEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationEntry {
    pub bundle_id: String,
    pub name: String,
    pub install_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInventory {
    pub applications: Vec<ApplicationEntry>,
}

#[tauri::command]
pub fn start_scan(
    _args: StartScanArgs,
    _cancellations: tauri::State<'_, CancellationRegistry>,
) -> Result<ScanSessionHeader, CommandError> {
    // Filesystem traversal is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "start_scan".into(),
        reason: "Filesystem traversal is not implemented in this milestone".into(),
    })
}

#[tauri::command]
pub fn cancel_scan(
    args: CancelScanArgs,
    cancellations: tauri::State<'_, CancellationRegistry>,
) -> Result<CancelScanResult, CommandError> {
    let cancelled = cancellations.cancel(&args.session_id);
    Ok(CancelScanResult {
        session_id: args.session_id,
        cancelled,
    })
}

#[tauri::command]
pub fn fetch_findings_page(_args: FetchFindingsPageArgs) -> Result<FindingsPage, CommandError> {
    // Paged findings retrieval is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "fetch_findings_page".into(),
        reason: "Scan storage and paged findings retrieval are not active".into(),
    })
}

#[tauri::command]
pub fn fetch_folder_aggregate(
    _args: FetchFolderAggregateArgs,
) -> Result<FolderAggregate, CommandError> {
    // Folder aggregate exploration is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "fetch_folder_aggregate".into(),
        reason: "Folder aggregation is not implemented in this milestone".into(),
    })
}

#[tauri::command]
pub fn fetch_application_inventory() -> Result<ApplicationInventory, CommandError> {
    // Application inventory inspection is planned for subsequent implementation.
    Err(CommandError::Unsupported {
        feature: "fetch_application_inventory".into(),
        reason: "Application inventory discovery is not implemented in this milestone".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_commands_return_typed_unsupported_error() {
        let err = CommandError::Unsupported {
            feature: "start_scan".into(),
            reason: "Filesystem traversal is not implemented in this milestone".into(),
        };
        assert_eq!(
            err,
            CommandError::Unsupported {
                feature: "start_scan".into(),
                reason: "Filesystem traversal is not implemented in this milestone".into(),
            }
        );
    }
}
