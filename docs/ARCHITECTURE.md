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

### IPC boundary and typed contracts

- **Generated boundary definitions:** All IPC boundary types are generated from Rust definitions into `src/types/bindings.ts`. CI enforces that committed TypeScript definitions match the Rust definitions, preventing type drift.
- **Capability-split command vocabulary:**
  - _Read:_ `foundation_status`, `start_scan`, `cancel_scan`, `fetch_findings_page` (paged with cursors), `fetch_folder_aggregate`, `fetch_application_inventory`.
  - _Plan:_ `build_plan`, `fetch_plan`, `revalidate_plan`.
  - _Destructive:_ `execute_plan` accepting only a plan identifier and an explicit action mode (`trash` or `permanentDelete`). Destructive commands never accept filesystem paths.
- **Status of domain command handlers:**
  - `foundation_status`: **implemented and tested**.
  - `start_scan`, `cancel_scan`, and `fetch_findings_page`: **implemented and tested** on macOS; unsupported on Windows.
  - Planning (`build_plan`, `fetch_plan`, `revalidate_plan`), execution (`execute_plan`), and exploration/inventory command handlers: **unsupported** in the current milestone (returning typed `unsupported` errors until subsequent domain implementations arrive).
- **Discriminated error model:** Every command returns a discriminated result (`permissionDenied`, `pathVanished`, `changedAfterReview`, `unsupported`) ensuring failure reasons are machine-readable.
- **Throttled event streaming:** Events (`scan:progress`, `scan:verified-count`, `scan:warning`, `execute:item-outcome`, `scan:terminal`) are coalesced by an emitter-side progress throttler against a time budget before IPC emission, preventing assistive technology flooding.
- **Cancellation channel:** Work cancellation is controlled via an explicit cancellation registry and thread-safe tokens, ensuring cancelling stops execution in the core rather than merely dropping the listener.

## Rust domains

- **scan:** bounded, cancellable traversal and partial-result reporting (**implemented and tested** on macOS).
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
- **Entry metadata inspection:**
  - Inspect apparent size and allocated block footprint, file types, modification timestamps, and link counts without following symlinks into unrequested scopes (`EntryMetadata`).
  - Status on macOS: **implemented and tested**.
  - Status on Windows: **unsupported**.

### Implementations

1. `MacOsAdapter`: macOS platform adapter implementing path discovery, permissions, filesystem identity, and entry metadata inspection, with Trash and application metadata returning typed unsupported errors.
2. `WindowsAdapter`: stub implementation compiling as a portability check in CI, returning typed unsupported errors for all capabilities. Windows is not supported in the current milestone.
3. `UnsupportedAdapter`: fallback implementation returning typed unsupported errors for every capability, providing a compiler-enforced checklist for new platform ports.
4. `TestAdapter`: hermetic in-memory implementation available in tests to report scripted paths, permissions, identities, entry metadata, and simulated Trash lifecycle without touching real filesystems.

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

**v1.0 ships unsigned.** Code signing and notarization are _planned_, not implemented: they require an Apple Developer Program membership that does not exist yet. Until it does, a downloaded build is not notarized, and macOS will require the user to right-click → Open, or clear the quarantine attribute, the first time they run it. The README says so plainly.

When signing does arrive, a public macOS release will require hardened runtime, code signing, notarization and clean-install verification. That is a future requirement, not a current property of any build.

**v1.0 is an Apple Silicon build only.** Intel Macs are not supported. A universal build is additive and can be added later without invalidating anything already released.

Windows packaging and signing begin only after the macOS v1.0 milestone.
