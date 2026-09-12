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

`PlatformAdapter` owns path discovery, permissions, Trash, application metadata, filesystem identity, and other OS-specific behavior. macOS is implemented first. Windows must implement the same domain contracts rather than introducing OS checks throughout the core.

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

## Performance model

Traversal and hashing use bounded concurrency. UI updates are throttled and streamed so cancellation stays responsive. Duplicate hashing proceeds by size, quick fingerprint, then full content; full hashing is never the first pass. Memory usage must be bounded independently of item count.

## Distribution

Development builds may be unsigned. Public macOS releases require hardened runtime, code signing, notarization, and clean-install verification. Windows packaging and signing begin only after the macOS v1.0 milestone.
