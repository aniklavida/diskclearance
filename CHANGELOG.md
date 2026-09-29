# Changelog

All notable changes will be documented here.

The project follows [Semantic Versioning](https://semver.org/). There is no public release yet.

## Unreleased

### Added

- Cancellable Mac storage scan with partial results retained upon interruption and truthful coverage evidence for permission-restricted areas.
- Three-class classification catalogue assigning findings to Rebuildable, Review, or Protected, with each Rebuildable rule declaring an automated regeneration path and cost.
- Protected-path safety engine preventing protected filesystem roots from entering cleanup plans and blocking outward traversal over symlinks.
- Review plan builder and immediate pre-execution revalidation pipeline rejecting swapped, modified, or vanished targets prior to any destructive operation.
- Dual execution modes distinguishing default Move to Trash from irreversible permanent deletion, requiring explicit secondary acknowledgement for permanent removal.
- Segregated storage accounting reporting pending-in-Trash bytes separately from permanently freed space, preventing Trash operations from falsely claiming immediate reclamation.
- SQLite-backed operation history (schema version 4) with audit logs, lifetime reclamation metrics, and safe Trash restoration featuring destination collision detection and alternate path proposals.
- Exact-content duplicate detection using staged candidate screening (size, sample fingerprint, streaming SHA-256) with zero preselection, exclusion of git repositories and protected roots, and APFS clone / hardlink detection.
- Application removal evidence separating bundle footprints from related state, classifying match confidence, and protecting running applications, shared frameworks, and credentials.
- Developer and local AI storage rules covering Xcode, Cargo, npm, Yarn, pnpm, Gradle, Maven, Go, Python virtual environments, Docker caches vs volumes, and local model weights.
- Explore view featuring an interactive disk usage treemap paired with an accessible folder table equivalent and contextual metadata inspector.
- Automated accessibility and design token test suite verifying WCAG AA contrast across light and dark modes, 32×32px minimum targets, 2px focus-visible rings, reduced-motion overrides, screen-reader labels, and 760×560px non-occluded review tray clearance.
- Measured performance budgets establishing flat resident memory during 1,000,000-entry scans, 1.38ms time to first result, 0.15ms cancellation latency, and 64 KiB bounded read buffer during streaming hash computation on Apple M4 hardware.
- Buildable Tauri 2, React, TypeScript, and Rust application shell with isolated platform adapter architecture.
- Public specification, architecture, design requirements, roadmap, and release checklist.
- CI validation suite covering TypeScript types, Prettier formatting, Vitest frontend tests, cargo fmt, cargo test, and cargo check across macOS and Windows.
