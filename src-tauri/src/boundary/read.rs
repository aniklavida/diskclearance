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

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::boundary::events::{
    CoverageWarningPayload, EVENT_COVERAGE_WARNING, EVENT_SCAN_PROGRESS, EVENT_TERMINAL_COMPLETION,
    ScanProgressPayload, TerminalCompletionPayload, TerminalStatus,
};
use crate::boundary::throttle::{ProgressSink, ProgressThrottler};
use crate::platform::PlatformAdapter;
use crate::scan::session::{ScanSessionRepository, generate_session_id};
use crate::scan::traversal::{TraversalOptions, walk_roots};
use crate::storage::AppDatabase;

pub trait ScanEventSink: Send + 'static {
    fn emit_progress(&mut self, payload: ScanProgressPayload);
    fn emit_warning(&mut self, payload: CoverageWarningPayload);
    fn emit_terminal(&mut self, payload: TerminalCompletionPayload);
}

pub struct TauriScanEventSink {
    pub app: tauri::AppHandle,
}

impl ScanEventSink for TauriScanEventSink {
    fn emit_progress(&mut self, payload: ScanProgressPayload) {
        use tauri::Emitter;
        let _ = self.app.emit(EVENT_SCAN_PROGRESS, payload);
    }

    fn emit_warning(&mut self, payload: CoverageWarningPayload) {
        use tauri::Emitter;
        let _ = self.app.emit(EVENT_COVERAGE_WARNING, payload);
    }

    fn emit_terminal(&mut self, payload: TerminalCompletionPayload) {
        use tauri::Emitter;
        let _ = self.app.emit(EVENT_TERMINAL_COMPLETION, payload);
    }
}

struct SharedSink<S: ScanEventSink>(Arc<std::sync::Mutex<S>>);

impl<S: ScanEventSink> ProgressSink for SharedSink<S> {
    fn emit_progress(&mut self, payload: ScanProgressPayload) {
        if let Ok(mut locked) = self.0.lock() {
            locked.emit_progress(payload);
        }
    }
}

pub fn start_scan_core<S: ScanEventSink>(
    args: StartScanArgs,
    adapter: Arc<dyn PlatformAdapter>,
    database: Arc<AppDatabase>,
    cancellations: &CancellationRegistry,
    sink: S,
) -> Result<ScanSessionHeader, CommandError> {
    let session_id = generate_session_id();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let started_at_ms = now.as_millis() as u64;

    let roots: Vec<PathBuf> = if args.roots.is_empty() {
        adapter
            .default_scan_roots()
            .map_err(|e| CommandError::Unsupported {
                feature: "start_scan".into(),
                reason: format!("failed to resolve default roots: {e}"),
            })?
            .into_iter()
            .map(|r| r.path)
            .collect()
    } else {
        args.roots.iter().map(PathBuf::from).collect()
    };

    let root_str = roots
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    {
        let conn = database
            .connection()
            .lock()
            .map_err(|e| CommandError::Unsupported {
                feature: "start_scan".into(),
                reason: format!("database lock poisoned: {e}"),
            })?;
        ScanSessionRepository::create_session(&conn, &session_id, &root_str).map_err(|e| {
            CommandError::Unsupported {
                feature: "start_scan".into(),
                reason: format!("failed to initialize scan session: {e}"),
            }
        })?;
    }

    let token = cancellations.register(&session_id);
    let session_id_bg = session_id.clone();
    let roots_bg = roots.clone();
    let adapter_bg = adapter.clone();
    let database_bg = database.clone();
    let token_bg = token.clone();
    let sink_arc = Arc::new(std::sync::Mutex::new(sink));

    std::thread::spawn(move || {
        let sink_for_progress = SharedSink(Arc::clone(&sink_arc));
        let mut throttler = ProgressThrottler::new(Duration::from_millis(50), sink_for_progress);

        let sink_for_warnings = Arc::clone(&sink_arc);
        let result = walk_roots(
            TraversalOptions {
                session_id: session_id_bg.clone(),
                roots: roots_bg,
                adapter: adapter_bg.as_ref(),
                cancellation_token: &token_bg,
                tracker: None,
                hooks: None,
                collect_entries: false,
            },
            |progress| {
                throttler.record(progress, Instant::now());
            },
            |entry| {
                if let Ok(conn) = database_bg.connection().lock() {
                    let finding_id = format!("{}-{}", session_id_bg, entry.identity.inode);
                    let _ = ScanSessionRepository::insert_finding(
                        &conn,
                        &finding_id,
                        &session_id_bg,
                        &entry.normalized_path.to_string_lossy(),
                        entry.allocated_size,
                        "Review",
                        "Scanned",
                    );
                }
            },
            move |warning| {
                if let Ok(mut locked) = sink_for_warnings.lock() {
                    locked.emit_warning(warning);
                }
            },
        );

        throttler.flush();

        let status_str = if result.coverage.is_complete {
            "completed"
        } else {
            "cancelled"
        };

        if let Ok(conn) = database_bg.connection().lock() {
            let _ = ScanSessionRepository::complete_session(
                &conn,
                &session_id_bg,
                status_str,
                &result.coverage,
            );
        }

        let terminal_status = if result.coverage.is_complete {
            TerminalStatus::Completed
        } else {
            TerminalStatus::Cancelled
        };

        if let Ok(mut locked) = sink_arc.lock() {
            locked.emit_terminal(TerminalCompletionPayload {
                session_id: session_id_bg.clone(),
                status: terminal_status,
                total_items: result.coverage.items_scanned,
                total_bytes: result.coverage.total_allocated_bytes,
                summary_message: format!(
                    "Scan {} with {} items and {} bytes",
                    status_str,
                    result.coverage.items_scanned,
                    result.coverage.total_allocated_bytes
                ),
            });
        }
    });

    Ok(ScanSessionHeader {
        session_id,
        started_at_ms,
        roots: roots
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    })
}

#[tauri::command]
pub fn start_scan(
    app: tauri::AppHandle,
    args: StartScanArgs,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
    database: tauri::State<'_, Arc<AppDatabase>>,
    cancellations: tauri::State<'_, CancellationRegistry>,
) -> Result<ScanSessionHeader, CommandError> {
    let sink = TauriScanEventSink { app };
    start_scan_core(
        args,
        adapter.inner().clone(),
        database.inner().clone(),
        cancellations.inner(),
        sink,
    )
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
pub fn fetch_findings_page(
    args: FetchFindingsPageArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
) -> Result<FindingsPage, CommandError> {
    let conn = database
        .connection()
        .lock()
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_findings_page".into(),
            reason: format!("database lock error: {e}"),
        })?;

    let (items, next_cursor, total_estimated) = ScanSessionRepository::fetch_findings_page(
        &conn,
        &args.session_id,
        args.cursor.as_deref(),
        args.limit,
    )
    .map_err(|e| CommandError::Unsupported {
        feature: "fetch_findings_page".into(),
        reason: e.to_string(),
    })?;

    Ok(FindingsPage {
        session_id: args.session_id,
        items,
        next_cursor,
        total_estimated,
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
    use crate::platform::tests::TestAdapter;
    use crate::scan::fixtures::DisposableFixtureTree;
    use std::sync::Mutex;

    #[derive(Default, Clone)]
    struct MockSink {
        progress: Arc<Mutex<Vec<ScanProgressPayload>>>,
        warnings: Arc<Mutex<Vec<CoverageWarningPayload>>>,
        terminals: Arc<Mutex<Vec<TerminalCompletionPayload>>>,
    }

    impl ScanEventSink for MockSink {
        fn emit_progress(&mut self, payload: ScanProgressPayload) {
            self.progress.lock().unwrap().push(payload);
        }

        fn emit_warning(&mut self, payload: CoverageWarningPayload) {
            self.warnings.lock().unwrap().push(payload);
        }

        fn emit_terminal(&mut self, payload: TerminalCompletionPayload) {
            self.terminals.lock().unwrap().push(payload);
        }
    }

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

    #[test]
    fn test_start_scan_core_and_fetch_findings_page() {
        let fixture = DisposableFixtureTree::new("boundary-read");
        let scan_root = fixture.create_dir("root");
        fixture.create_file("root/item_a.txt", b"content a");
        fixture.create_file("root/item_b.txt", b"content b");

        let adapter = Arc::new(TestAdapter::default());
        let database = AppDatabase::open_in_memory().expect("open database");
        let cancellations = CancellationRegistry::new();
        let sink = MockSink::default();
        let terminals = Arc::clone(&sink.terminals);

        let header = start_scan_core(
            StartScanArgs {
                roots: vec![scan_root.to_string_lossy().to_string()],
            },
            adapter,
            database.clone(),
            &cancellations,
            sink,
        )
        .expect("start scan core");

        assert!(!header.session_id.is_empty());
        assert_eq!(header.roots.len(), 1);

        // Wait for background worker to emit terminal completion
        let start = Instant::now();
        while terminals.lock().unwrap().is_empty() && start.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(terminals.lock().unwrap().len(), 1);

        let conn = database.connection().lock().unwrap();
        let (items, next_cursor, total) =
            ScanSessionRepository::fetch_findings_page(&conn, &header.session_id, None, 10)
                .expect("fetch findings page");

        assert!(items.len() >= 2);
        assert_eq!(next_cursor, None);
        assert!(total >= 2);
    }
}
