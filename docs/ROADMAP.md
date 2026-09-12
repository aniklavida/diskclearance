# Roadmap

DiskClearance develops through internal milestones; the first marketed release is a complete v1.0.

## M0 — Foundation

- Buildable Tauri 2 + React/TypeScript + Rust shell.
- Public specification, architecture, design system, contribution, security, and release documents.
- CI on macOS and Windows for the portable foundation.
- Local database migration and platform-boundary spike.

**Done when:** a fresh clone passes documented checks on macOS, CI is green, and the repository makes no unsupported product claims.

## M1 — Safe scan and review

- Cancellable macOS scan with permission-limited and partial results.
- Protected-path policy and versioned Rebuildable/Review rules.
- Home, Cleanup, Explore, and review-plan UI states.
- SQLite scan cache and immutable review plans.
- Pre-execution identity revalidation using disposable fixtures.

**Done when:** users can understand and review real scan results, but production deletion remains disabled until safety tests pass.

## Later milestones

- **M2:** Trash execution, Delete Now, operation history, and restoration.
- **M3:** exact duplicates, application removal, and Developer & AI storage.
- **M4:** accessibility, performance, signed/notarized packaging, clean-install proof, documentation, and v1.0 release.
- **Post-v1:** Windows adapter and installer after the macOS product is stable.

Detailed work cards are created only for the active and next milestone.
