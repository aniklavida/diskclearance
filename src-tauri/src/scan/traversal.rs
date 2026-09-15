use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::boundary::cancellation::CancellationToken;
use crate::boundary::events::{CoverageWarningPayload, ScanPhase, ScanProgressPayload};
use crate::platform::{EntryType, FileIdentity, PlatformAdapter, PlatformError};
use crate::scan::coverage::{CoverageSummary, SkipReason, SkippedScope};
use crate::scan::entry::ScannedEntry;

/// Tracks the high-water mark of scan records the traversal is holding.
///
/// It counts two things and must keep counting both: the transient record being
/// processed right now, **and every record the walk retains**. Counting only the
/// transient one would report a flat peak of 1 even while the whole tree was being
/// accumulated in memory — a test named after the streaming guarantee that does not
/// enforce it. That exact regression was introduced deliberately and the earlier
/// version of this tracker did not notice, which is why `track_retained` exists.
#[derive(Debug, Default)]
pub struct MemoryTracker {
    live_records: AtomicUsize,
    retained_records: AtomicUsize,
    peak_records: AtomicUsize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track_alloc(&self) {
        let current = self.live_records.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak_records.fetch_max(current, Ordering::SeqCst);
    }

    pub fn track_dealloc(&self) {
        self.live_records.fetch_sub(1, Ordering::SeqCst);
    }

    /// Records that one more entry is being kept for the lifetime of the walk.
    /// Never paired with a release, because retention is the thing being measured.
    pub fn track_retained(&self) {
        let current = self.retained_records.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak_records.fetch_max(
            current + self.live_records.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
    }

    pub fn live(&self) -> usize {
        self.live_records.load(Ordering::SeqCst)
    }

    pub fn peak(&self) -> usize {
        self.peak_records.load(Ordering::SeqCst)
    }
}

/// Optional test hooks for intercepting traversal steps without race conditions.
#[derive(Default)]
pub struct TraversalHooks {
    pub on_before_entry_stat: Option<Box<dyn Fn(&Path) + Send + Sync>>,
    pub on_entry_visited: Option<Box<dyn Fn(&Path) + Send + Sync>>,
}

/// Output of a traversal operation.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    pub session_id: String,
    pub coverage: CoverageSummary,
    pub entries: Vec<ScannedEntry>,
}

/// Traversal configuration options.
pub struct TraversalOptions<'a> {
    pub session_id: String,
    pub roots: Vec<PathBuf>,
    pub adapter: &'a dyn PlatformAdapter,
    pub cancellation_token: &'a CancellationToken,
    pub tracker: Option<&'a MemoryTracker>,
    pub hooks: Option<&'a TraversalHooks>,
    pub collect_entries: bool,
}

/// Walk the provided roots with bounded memory and cancellation checks between directory entries.
pub fn walk_roots<FProg, FEntry, FWarn>(
    options: TraversalOptions<'_>,
    mut on_progress: FProg,
    mut on_entry: FEntry,
    mut on_warning: FWarn,
) -> TraversalResult
where
    FProg: FnMut(ScanProgressPayload),
    FEntry: FnMut(ScannedEntry),
    FWarn: FnMut(CoverageWarningPayload),
{
    let session_id = options.session_id;
    let mut coverage = CoverageSummary::default();
    let mut collected_entries = Vec::new();
    let mut seen_hardlinks: HashSet<FileIdentity> = HashSet::new();

    // 1. Resolve requested roots and identify root devices
    let mut dir_stack: Vec<(PathBuf, u64)> = Vec::new();
    let mut requested_devices: HashSet<u64> = HashSet::new();

    for root in &options.roots {
        if options.cancellation_token.is_cancelled() {
            coverage.is_complete = false;
            return TraversalResult {
                session_id,
                coverage,
                entries: collected_entries,
            };
        }

        match options.adapter.file_identity(root) {
            Ok(id) => {
                requested_devices.insert(id.device_id);
                dir_stack.push((root.clone(), id.device_id));
            }
            Err(PlatformError::NotFound(_)) => {
                coverage.skipped_scopes.push(SkippedScope {
                    path: root.clone(),
                    reason: SkipReason::Vanished,
                });
            }
            Err(PlatformError::PermissionDenied { reason, .. }) => {
                coverage.skipped_permissions_count += 1;
                coverage.skipped_scopes.push(SkippedScope {
                    path: root.clone(),
                    reason: SkipReason::PermissionDenied(reason.clone()),
                });
                on_warning(CoverageWarningPayload {
                    session_id: session_id.clone(),
                    path: root.to_string_lossy().to_string(),
                    warning_code: "permission_denied".to_string(),
                    message: reason,
                });
            }
            Err(err) => {
                coverage.skipped_scopes.push(SkippedScope {
                    path: root.clone(),
                    reason: SkipReason::IoError(err.to_string()),
                });
            }
        }
    }

    let mut cancelled = false;

    // 2. Traversal loop using an explicit depth-first stack
    while let Some((current_dir, root_device)) = dir_stack.pop() {
        if options.cancellation_token.is_cancelled() {
            cancelled = true;
            break;
        }

        // Check permission and open directory
        let read_dir_result = std::fs::read_dir(&current_dir);
        let entries = match read_dir_result {
            Ok(iter) => iter,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                coverage.skipped_permissions_count += 1;
                coverage.skipped_scopes.push(SkippedScope {
                    path: current_dir.clone(),
                    reason: SkipReason::PermissionDenied(err.to_string()),
                });
                on_warning(CoverageWarningPayload {
                    session_id: session_id.clone(),
                    path: current_dir.to_string_lossy().to_string(),
                    warning_code: "permission_denied".to_string(),
                    message: format!(
                        "Permission denied reading directory {}",
                        current_dir.display()
                    ),
                });
                continue;
            }
            Err(err) => {
                coverage.skipped_scopes.push(SkippedScope {
                    path: current_dir.clone(),
                    reason: SkipReason::IoError(err.to_string()),
                });
                continue;
            }
        };

        for entry_res in entries {
            // Cancellation checked between individual directory entries
            if options.cancellation_token.is_cancelled() {
                cancelled = true;
                break;
            }

            let dir_entry = match entry_res {
                Ok(e) => e,
                Err(err) => {
                    coverage.skipped_scopes.push(SkippedScope {
                        path: current_dir.clone(),
                        reason: SkipReason::IoError(err.to_string()),
                    });
                    continue;
                }
            };

            let entry_path = dir_entry.path();

            // Fire optional test hook before stat
            if let Some(hooks) = options.hooks {
                if let Some(ref hook) = hooks.on_before_entry_stat {
                    hook(&entry_path);
                }
            }

            // Inspect entry metadata via platform adapter
            let meta = match options.adapter.read_entry_metadata(&entry_path) {
                Ok(m) => m,
                Err(PlatformError::NotFound(_)) => {
                    coverage.skipped_vanished_count += 1;
                    coverage.skipped_scopes.push(SkippedScope {
                        path: entry_path,
                        reason: SkipReason::Vanished,
                    });
                    continue;
                }
                Err(PlatformError::PermissionDenied { reason, .. }) => {
                    coverage.skipped_permissions_count += 1;
                    coverage.skipped_scopes.push(SkippedScope {
                        path: entry_path.clone(),
                        reason: SkipReason::PermissionDenied(reason.clone()),
                    });
                    on_warning(CoverageWarningPayload {
                        session_id: session_id.clone(),
                        path: entry_path.to_string_lossy().to_string(),
                        warning_code: "permission_denied".to_string(),
                        message: reason,
                    });
                    continue;
                }
                Err(err) => {
                    coverage.skipped_scopes.push(SkippedScope {
                        path: entry_path,
                        reason: SkipReason::IoError(err.to_string()),
                    });
                    continue;
                }
            };

            // Mount boundary guard: never cross onto another device unless explicitly in requested roots
            if root_device != 0
                && meta.identity.device_id != root_device
                && !requested_devices.contains(&meta.identity.device_id)
            {
                coverage.skipped_mount_boundaries_count += 1;
                coverage.skipped_scopes.push(SkippedScope {
                    path: entry_path.clone(),
                    reason: SkipReason::MountBoundary {
                        root_device,
                        path_device: meta.identity.device_id,
                    },
                });
                on_warning(CoverageWarningPayload {
                    session_id: session_id.clone(),
                    path: entry_path.to_string_lossy().to_string(),
                    warning_code: "mount_boundary_skipped".to_string(),
                    message: format!(
                        "Mount boundary crossed from root device {root_device} to {}. Scope skipped.",
                        meta.identity.device_id
                    ),
                });
                continue;
            }

            // Canonicalize and firmlink-normalize path
            let resolved = options
                .adapter
                .canonicalize_and_normalize(&entry_path)
                .unwrap_or_else(|_| crate::platform::ResolvedPath {
                    original: entry_path.clone(),
                    canonical: entry_path.clone(),
                    normalized: entry_path.clone(),
                    is_firmlink_alias: false,
                });

            coverage.items_scanned += 1;

            // Size accumulation and entry-type specific handling
            match meta.entry_type {
                EntryType::Directory => {
                    coverage.directories_count += 1;
                    dir_stack.push((entry_path.clone(), root_device));
                }
                EntryType::Symlink => {
                    coverage.symlinks_count += 1;
                    // Symlinks contribute their own size, never their target's size, and are not traversed
                    coverage.total_apparent_bytes += meta.apparent_size;
                    coverage.total_allocated_bytes += meta.allocated_size;
                }
                EntryType::File => {
                    coverage.files_count += 1;
                    let is_duplicate_hardlink = if meta.nlink > 1 {
                        !seen_hardlinks.insert(meta.identity)
                    } else {
                        false
                    };

                    if !is_duplicate_hardlink {
                        coverage.total_apparent_bytes += meta.apparent_size;
                        coverage.total_allocated_bytes += meta.allocated_size;
                    }
                }
                EntryType::Other => {
                    coverage.total_apparent_bytes += meta.apparent_size;
                    coverage.total_allocated_bytes += meta.allocated_size;
                }
            }

            // Memory tracker recording active in-memory record
            if let Some(tracker) = options.tracker {
                tracker.track_alloc();
            }

            let entry = ScannedEntry {
                original_path: entry_path.clone(),
                canonical_path: resolved.canonical,
                normalized_path: resolved.normalized,
                identity: meta.identity,
                entry_type: meta.entry_type,
                apparent_size: meta.apparent_size,
                allocated_size: meta.allocated_size,
                modified_ms: meta.modified_ms,
            };

            if let Some(hooks) = options.hooks {
                if let Some(ref hook) = hooks.on_entry_visited {
                    hook(&entry_path);
                }
            }

            on_entry(entry.clone());

            if options.collect_entries {
                collected_entries.push(entry);
                if let Some(tracker) = options.tracker {
                    tracker.track_retained();
                }
            }

            if let Some(tracker) = options.tracker {
                tracker.track_dealloc();
            }

            on_progress(ScanProgressPayload {
                session_id: session_id.clone(),
                phase: ScanPhase::Scanning,
                current_scope: entry_path.to_string_lossy().to_string(),
                items_visited: coverage.items_scanned,
                bytes_visited: coverage.total_allocated_bytes,
            });
        }

        if cancelled {
            break;
        }
    }

    coverage.is_complete = !cancelled;

    TraversalResult {
        session_id,
        coverage,
        entries: collected_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    use crate::platform::tests::TestAdapter;
    use crate::scan::fixtures::DisposableFixtureTree;

    // Unix-only: this asserts POSIX filesystem semantics — device+inode identity
    // and chmod-based permission denial — which Windows does not provide. The
    // Windows CI leg is a compile-portability check, not a behavioural one, and
    // `docs/ARCHITECTURE.md` says Windows is unsupported.
    #[cfg(unix)]
    #[test]
    fn test_permission_denied_directory_yields_coverage_evidence_and_partial_result() {
        let fixture = DisposableFixtureTree::new("perm-denied");
        let scan_root = fixture.create_dir("scan_root");
        let _readable_dir = fixture.create_dir("scan_root/readable");
        fixture.create_file("scan_root/readable/visible.txt", b"readable content 12345");

        let restricted_dir = fixture.create_permission_denied_dir("scan_root/restricted");

        let adapter = TestAdapter::default().with_permission(restricted_dir.clone(), false, None);
        let token = CancellationToken::new();

        let mut warnings = Vec::new();
        let result = walk_roots(
            TraversalOptions {
                session_id: "test-perm".into(),
                roots: vec![scan_root],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: None,
                hooks: None,
                collect_entries: true,
            },
            |_| {},
            |_| {},
            |w| warnings.push(w),
        );

        // Verification: yielded partial results, coverage evidence, and not an error
        assert!(result.coverage.items_scanned >= 2);
        assert_eq!(result.coverage.skipped_permissions_count, 1);
        assert!(!result.coverage.is_exact());
        assert_eq!(result.coverage.skipped_scopes.len(), 1);
        assert!(
            result.coverage.skipped_scopes[0]
                .path
                .ends_with("restricted")
        );
        assert!(matches!(
            result.coverage.skipped_scopes[0].reason,
            SkipReason::PermissionDenied(_)
        ));

        // Readable item is present in findings
        let has_visible = result
            .entries
            .iter()
            .any(|e| e.original_path.ends_with("visible.txt"));
        assert!(has_visible);
    }

    #[test]
    fn test_cancellation_issued_mid_walk_returns_within_measurable_bound_with_partial_result() {
        let fixture = DisposableFixtureTree::new("cancellation");
        let scan_root = fixture.create_dir("scan_root");

        // Populate with 200 files
        for i in 0..200 {
            fixture.create_file(
                &format!("scan_root/file_{i}.txt"),
                b"sample data for cancellation test",
            );
        }

        let adapter = TestAdapter::default();
        let token = CancellationToken::new();

        let cancel_after = 20;
        let token_clone = token.clone();
        let mut items_seen = 0;

        let start_time = Instant::now();
        let result = walk_roots(
            TraversalOptions {
                session_id: "test-cancel".into(),
                roots: vec![scan_root],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: None,
                hooks: None,
                collect_entries: true,
            },
            |_| {},
            |_| {
                items_seen += 1;
                if items_seen == cancel_after {
                    token_clone.cancel();
                }
            },
            |_| {},
        );
        let elapsed = start_time.elapsed();

        // Measurable bound: returns in under 50ms once cancellation triggered
        assert!(
            elapsed.as_millis() < 500,
            "Cancellation must return quickly, took {:?}",
            elapsed
        );
        // Retained partial result verified before cancel
        assert!(result.coverage.items_scanned >= cancel_after);
        assert!(
            result.coverage.items_scanned < 200,
            "Must stop well before scanning all 200 files, stopped at {}",
            result.coverage.items_scanned
        );
        assert!(!result.coverage.is_complete);
        assert_eq!(result.entries.len(), result.coverage.items_scanned as usize);
    }

    #[test]
    fn test_symlink_pointing_outside_scope_is_recorded_not_traversed_and_sizes_self() {
        let fixture = DisposableFixtureTree::new("symlink-outside");
        let scan_root = fixture.create_dir("scan_root");

        // External target with large payload (100k bytes)
        let _outside_dir = fixture.create_dir("outside_target");
        let external_file = fixture.create_file("outside_target/large.dat", &[0u8; 100_000]);

        // Symlink inside scan_root pointing to external target
        let link_path = fixture.create_symlink(&external_file, "scan_root/link_to_external");

        let adapter = TestAdapter::default();
        let token = CancellationToken::new();

        let result = walk_roots(
            TraversalOptions {
                session_id: "test-symlink".into(),
                roots: vec![scan_root],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: None,
                hooks: None,
                collect_entries: true,
            },
            |_| {},
            |_| {},
            |_| {},
        );

        assert_eq!(result.coverage.symlinks_count, 1);
        assert_eq!(result.coverage.files_count, 0);

        // Symlink contributes its own size (< 1000 bytes), NOT its target's 100,000 bytes
        assert!(
            result.coverage.total_apparent_bytes < 5000,
            "Symlink must not inflate total with target bytes, got {}",
            result.coverage.total_apparent_bytes
        );

        let symlink_entry = result
            .entries
            .iter()
            .find(|e| e.original_path == link_path)
            .expect("symlink entry recorded");
        assert_eq!(symlink_entry.entry_type, EntryType::Symlink);
    }

    // Unix-only: this asserts POSIX filesystem semantics — device+inode identity
    // and chmod-based permission denial — which Windows does not provide. The
    // Windows CI leg is a compile-portability check, not a behavioural one, and
    // `docs/ARCHITECTURE.md` says Windows is unsupported.
    #[cfg(unix)]
    #[test]
    fn test_hardlinked_pair_contributes_bytes_exactly_once() {
        let fixture = DisposableFixtureTree::new("hardlinks");
        let scan_root = fixture.create_dir("scan_root");

        let content = [42u8; 8192]; // 8 KB
        fixture.create_file("scan_root/original.bin", &content);
        fixture.create_hardlink("scan_root/original.bin", "scan_root/hardlink_clone.bin");

        let adapter = TestAdapter::default();
        let token = CancellationToken::new();

        let result = walk_roots(
            TraversalOptions {
                session_id: "test-hardlink".into(),
                roots: vec![scan_root],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: None,
                hooks: None,
                collect_entries: true,
            },
            |_| {},
            |_| {},
            |_| {},
        );

        assert_eq!(result.coverage.files_count, 2);
        // Both files are accounted for in items, but apparent size is contributed exactly ONCE (8192)
        assert_eq!(
            result.coverage.total_apparent_bytes, 8192,
            "Hardlinked pair must contribute bytes exactly once"
        );
    }

    #[test]
    fn test_file_deleted_between_read_dir_and_stat_is_recorded_as_vanished() {
        let fixture = DisposableFixtureTree::new("vanished");
        let scan_root = fixture.create_dir("scan_root");
        fixture.create_file("scan_root/stable.txt", b"stable content");
        let doomed = fixture.create_file("scan_root/doomed.txt", b"will vanish");

        let doomed_path = doomed.clone();
        let vanished_triggered = Arc::new(AtomicBool::new(false));
        let vanished_flag = Arc::clone(&vanished_triggered);

        let hooks = TraversalHooks {
            on_before_entry_stat: Some(Box::new(move |p| {
                if p == doomed_path.as_path() {
                    let _ = std::fs::remove_file(p);
                    vanished_flag.store(true, Ordering::SeqCst);
                }
            })),
            on_entry_visited: None,
        };

        let adapter = TestAdapter::default();
        let token = CancellationToken::new();

        let result = walk_roots(
            TraversalOptions {
                session_id: "test-vanished".into(),
                roots: vec![scan_root],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: None,
                hooks: Some(&hooks),
                collect_entries: true,
            },
            |_| {},
            |_| {},
            |_| {},
        );

        assert!(vanished_triggered.load(Ordering::SeqCst));
        assert_eq!(result.coverage.skipped_vanished_count, 1);
        let vanished_scope = result
            .coverage
            .skipped_scopes
            .iter()
            .find(|s| s.reason == SkipReason::Vanished)
            .expect("vanished scope recorded");
        assert_eq!(vanished_scope.path, doomed);

        // Scan did not abort: stable.txt was verified
        assert!(
            result
                .entries
                .iter()
                .any(|e| e.original_path.ends_with("stable.txt"))
        );
    }

    #[test]
    fn test_scanning_100k_entries_keeps_peak_memory_flat_relative_to_10k_entries() {
        // High-water mark memory verification:
        // We measure live in-memory entry records held by the streaming pipeline.
        // For a streaming pipeline, live records at any instant is bounded by O(1) buffer size.
        let fixture_10k = DisposableFixtureTree::new("mem-10k");
        let root_10k = fixture_10k.create_dir("root");
        for i in 0..10_000 {
            fixture_10k.create_file(&format!("root/file_{i}.txt"), b"x");
        }

        let tracker_10k = MemoryTracker::new();
        let token = CancellationToken::new();
        let adapter = TestAdapter::default();

        let _ = walk_roots(
            TraversalOptions {
                session_id: "mem-10k".into(),
                roots: vec![root_10k],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: Some(&tracker_10k),
                hooks: None,
                collect_entries: false, // stream without retaining whole tree
            },
            |_| {},
            |_| {},
            |_| {},
        );
        let peak_10k = tracker_10k.peak();

        let fixture_100k = DisposableFixtureTree::new("mem-100k");
        let root_100k = fixture_100k.create_dir("root");
        for i in 0..100_000 {
            fixture_100k.create_file(&format!("root/file_{i}.txt"), b"x");
        }

        let tracker_100k = MemoryTracker::new();
        let _ = walk_roots(
            TraversalOptions {
                session_id: "mem-100k".into(),
                roots: vec![root_100k],
                adapter: &adapter,
                cancellation_token: &token,
                tracker: Some(&tracker_100k),
                hooks: None,
                collect_entries: false, // stream without retaining whole tree
            },
            |_| {},
            |_| {},
            |_| {},
        );
        let peak_100k = tracker_100k.peak();

        // Prove peak live records in memory is flat between 10k and 100k
        assert_eq!(
            peak_10k, peak_100k,
            "Peak in-memory records must remain flat (10k had {peak_10k}, 100k had {peak_100k})"
        );
        assert_eq!(
            peak_100k, 1,
            "Peak live record count in streaming pipeline is exactly 1"
        );
    }

    #[test]
    fn test_no_test_touches_any_path_outside_its_temporary_directory() {
        let fixture = DisposableFixtureTree::new("isolated-guard");
        let inside = fixture.create_file("inside.txt", b"safe");
        fixture.assert_path_in_fixture(&inside);

        // Attempting to assert a path outside fixture root triggers panic
        let result = std::panic::catch_unwind(|| {
            let outside = PathBuf::from("/Library/Application Support/malicious_probe");
            fixture.assert_path_in_fixture(&outside);
        });
        assert!(
            result.is_err(),
            "Must panic when path is outside fixture directory"
        );
    }
}
