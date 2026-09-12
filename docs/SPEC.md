# DiskClearance v1.0 specification

## Product promise

DiskClearance helps people understand what consumes storage, decide what is safe to remove, and see exactly what happened. It is a focused storage product, not an optimizer or security suite.

## Primary users

- Mac users who need space but do not trust opaque cleaners.
- Developers carrying large build artifacts, simulators, containers, package caches, and local AI models.
- Advanced users who want evidence and control without living in the terminal.

## v1.0 requirements

### Scan and home

- Run one cancellable, read-only scan and retain already verified partial results after cancellation.
- Show capacity, used space, scanned space, inaccessible areas, and clearly separated reclaim estimates.
- Request broader permissions only when a selected scan surface needs them.

### Cleanup

- Group rebuildable caches and temporary artifacts by owning tool or application.
- Assign each result one class: Rebuildable, Review, or Protected.
- Show reason, size, last activity, path, recovery status, and selection state for every result.
- Never auto-select Review items; Protected items cannot be selected.

### Explore and duplicates

- Navigate scanned folders using list and treemap views without auto-selection.
- Find duplicate regular files using size grouping followed by exact-content hashing.
- Show the retained original and require explicit review before selecting copies.

### Applications

- List installed applications and their measured footprint.
- Show related files with explicit matching evidence and confidence.
- Treat uncertain matches as Review and leave them unselected.

### Developer and AI storage

- Explain storage used by build outputs, simulators, containers, package caches, and local model stores.
- Use tool-aware rules and distinguish rebuildable data from source, credentials, project state, and durable models.

### Plan, execute, and recover

- Build an immutable reviewed action plan before any mutation.
- Record canonical path, file identity, size, modification time, evidence, class, and intended action.
- Re-resolve and revalidate every target immediately before execution; changed targets are blocked.
- Offer Move to Trash as the default recoverable action and Delete Now as a separate irreversible action.
- State that Trash does not free space until emptied.
- Record successes, failures, skipped items, pending-in-Trash space, and permanently reclaimed space.
- Restore only when the exact trashed item still exists and its original destination is safe.

## Experience requirements

- Five primary destinations: Home, Cleanup, Explore, Applications, and History.
- First run: Welcome → Scan Mac → categorized results → review → action choice → honest completion.
- Calm, native-feeling light and dark interfaces; no fear, fake urgency, or inflated savings.
- Full keyboard navigation, visible focus, screen-reader labels, sufficient contrast, reduced motion, and non-color status cues.
- Deliberate empty, loading, partial, permission-limited, cancelled, error, and recovery states.

## Technical constraints

- macOS 13 Ventura and later is the first supported platform.
- Rust owns scans, evidence, plans, safety checks, database access, and filesystem mutation.
- Tauri 2 hosts a React and TypeScript interface.
- SQLite stores local scan cache, rule versions, reviewed plans, and operation history.
- Platform behavior is isolated behind adapters; Windows is planned after macOS v1.0.
- All data remains local. No account, telemetry, cloud control, or AI cleanup decisions in v1.0.

## Explicit exclusions

- Malware scanning, performance tuning, maintenance scripts, startup management, and app updating.
- Background or scheduled automatic deletion.
- Secure-overwrite promises on SSDs.
- Similar-image or fuzzy duplicate deletion.
- Windows or Linux support in the first release.

## v1.0 acceptance

- No protected path or changed-after-review target can be deleted in automated safety tests.
- Scanning can be cancelled and reports inaccessible or incomplete areas truthfully.
- Trash and permanent-deletion totals are never combined.
- Every destructive action has a reviewed manifest and auditable result.
- The complete macOS flow passes isolated-fixture, integration, clean-install, accessibility, and signed/notarized distribution checks.
- README, in-app copy, screenshots, demo, and release notes describe only verified behavior.
