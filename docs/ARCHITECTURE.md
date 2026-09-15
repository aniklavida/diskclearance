# Architecture

## Boundaries

```text
React/TypeScript UI
        │ typed commands and events
        ▼
Tauri application boundary
        │
        ▼
Rust domain core ─── SQLite repositories
        │
        ▼
Platform adapter ─── macOS first, Windows later
```

The renderer never receives unrestricted filesystem authority. It requests typed read or action-plan operations; Rust validates inputs and emits typed progress and results.

## Rust domains

- **scan:** bounded, cancellable traversal and partial-result reporting.
- **classify:** versioned rules, evidence, and Rebuildable/Review/Protected classification.
- **explore:** aggregate folder sizes and visualisation data.
- **duplicates:** size grouping, staged hashing, and exact-content groups.
- **applications:** application inventory and explainable related-file evidence.
- **plan:** immutable review snapshots and pre-execution revalidation.
- **execute:** Trash and permanent-delete adapters with per-item results.
- **history:** operation manifests, outcomes, pending Trash, and eligible restoration.
- **storage:** SQLite migrations and repositories.

## Platform boundary

`PlatformAdapter` owns path discovery, permissions, Trash, application metadata, filesystem identity, and other OS-specific behavior. The adapter is resolved once at application startup into managed Tauri state. Domain modules depend on `&dyn PlatformAdapter` rather than conditional compilation (`#[cfg(target_os)]`), ensuring core domain logic is decoupled from OS specifics.

### Contract capabilities and status

- **Path discovery:**
  - Discover standard locations (home, application support, caches, Trash directory, application directories, and default scan roots) returning typed `DiscoveredPath` and `ScanRoot` structures.
  - Status on macOS: **implemented and tested**.
  - Status on Windows: **unsupported**.
- **Permissions:**
  - Query scope readability (`ScopePermission`) without triggering immediate prompts, and map restricted paths to their required System Settings destination (`SettingsDestination`).
  - Status on macOS: **implemented and tested**.
  - Status on Windows: **unsupported**.
- **Filesystem identity:**
  - Query device and inode identities (`FileIdentity`), resolve and normalise canonical paths including macOS firmlinks and `/System/Volumes/Data` aliasing (`ResolvedPath`), and detect filesystem mount boundaries (`MountBoundary`).
  - Status on macOS: **implemented and tested**.
  - Status on Windows: **unsupported**.
- **Trash lifecycle:**
  - Move items to Trash while preserving Put Back origin (`TrashedItem`), enumerate current Trash contents, and verify whether a previously trashed item is still present (`TrashedItemStatus`).
  - Status: **unsupported** (planned for milestone M2). Returns typed unsupported error.
- **Application metadata:**
  - Inspect installed bundle identifiers, version strings, install paths, and measured footprints (`ApplicationMetadata`).
  - Status: **unsupported** (planned for milestone M3). Returns typed unsupported error.

### Implementations

1. `MacOsAdapter`: macOS platform adapter implementing path discovery, permissions, and filesystem identity, with Trash and application metadata returning typed unsupported errors.
2. `WindowsAdapter`: stub implementation compiling as a portability check in CI, returning typed unsupported errors for all capabilities. Windows is not supported in the current milestone.
3. `UnsupportedAdapter`: fallback implementation returning typed unsupported errors for every capability, providing a compiler-enforced checklist for new platform ports.
4. `TestAdapter`: hermetic in-memory implementation available in tests to report scripted paths, permissions, identities, and simulated Trash lifecycle without touching real filesystems.

A privileged helper is not part of the default architecture. If a macOS operation cannot be safely completed in-process, it may use a narrowly scoped, code-signature-validated XPC helper with explicit commands and no arbitrary path execution.

## Safety invariants

1. Scans do not mutate the filesystem.
2. Protected items are unselectable.
3. Review items are never selected by default.
4. Every operation executes from a reviewed snapshot.
5. Canonical path and file identity are checked again immediately before mutation.
6. Symlinks are not followed across an unreviewed boundary.
7. Tests use disposable temporary fixtures, never a home directory.
8. Partial failure is recorded per item; it is never reported as complete success.
9. History does not imply recoverability after permanent deletion.

## Data model

SQLite uses forward-only migrations. Core records include scan sessions, findings, rule versions, review plans, plan items, operations, operation items, and restore outcomes. Paths stored for history are local-only and must never be included in telemetry or public diagnostics.

### Storage and migration lifecycle

- **Database location:** Resolved via `PlatformAdapter::application_support_directory()` under `com.aniklavida.diskclearance/diskclearance.db`. Opened once at application startup and held in managed Tauri state.
- **Connection configuration:** Every connection enforces `PRAGMA foreign_keys = ON` and `PRAGMA journal_mode = WAL`.
- **Migration runner:** Forward-only and append-only. Migrations execute in numeric order inside a single transaction per migration that commits both DDL changes and the `schema_migrations` record atomically. A failed migration rolls back completely without leaving partial schema.
- **Downgrade protection:** The runner inspects the highest recorded schema version and refuses to open any database whose version exceeds the binary's known migrations.
- **Corrupt database recovery:** If corruption or malformed database files are detected during opening or integrity validation, the existing file (and any `-wal`/`-shm` sidecars) is preserved by renaming with a `.corrupt.<timestamp>` suffix. A clean database is then created and initialized, and the recovery event is surfaced.
- **Schema status:**
  - Migration `0001` (foundation `schema_migrations` table): **implemented and tested**.
  - Migration `0002` (core tables: `scan_sessions`, `findings`, `rule_versions`): **implemented and tested**.
  - Subsequent tables (`review_plans`, `plan_items`, `operations`, `operation_items`, `restore_outcomes`): **planned for v1.0**.

## Performance model

Traversal and hashing use bounded concurrency. UI updates are throttled and streamed so cancellation stays responsive. Duplicate hashing proceeds by size, quick fingerprint, then full content; full hashing is never the first pass. Memory usage must be bounded independently of item count.

## Distribution

Development builds may be unsigned. Public macOS releases require hardened runtime, code signing, notarization, and clean-install verification. Windows packaging and signing begin only after the macOS v1.0 milestone.
