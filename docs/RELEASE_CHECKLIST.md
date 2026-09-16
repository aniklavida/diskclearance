# v1.0 release checklist

## Product truth

- [ ] Every advertised feature is implemented and tested.
- [ ] Unsupported platforms and operations are explicit.
- [ ] Savings, Trash, and permanently reclaimed totals are accurate.
- [ ] README, screenshots, demo, changelog, and in-app copy agree.

## Data safety

- [ ] Protected-path, symlink, race, mount-boundary, cancellation, and partial-failure suites pass.
- [ ] All destructive tests run only in disposable fixtures.
- [ ] Every action is backed by a reviewed plan and immediate revalidation.
- [ ] Restore is offered only for an exact item still available in Trash.
- [ ] A third-party security review or documented maintainer threat-model review is complete.

### Automated test coverage

The automated destructive-safety test suite (`src-tauri/src/classify/destructive_safety.rs`) running on hermetic disposable fixtures (`DisposableFixtureTree`) mechanically verifies:

- **Protected-path enforcement:** Every protected root in the rule catalogue is rejected by the plan builder and blocked by pre-execution revalidation.
- **Symlink escape:** Cache symlinks pointing to source trees, credential stores, or fixture exterior are unlinked without traversing into or mutating targets.
- **Race and replacement detection:** Path swaps (file replaced by another file, directory replaced by symlink) and same-path/new-inode reallocations are caught and blocked before execution.
- **Cancellation & partial failure:** Cancellation signals are honored, and partial failures (vanished, permission-denied, protected) produce distinct outcome categories without claiming blanket success.
- **Hostile filenames:** Paths containing CLI argument flags (`--force`, `-rf`), newlines, leading hyphens, and Unicode characters are handled strictly via direct filesystem syscalls without shell invocation.
- **Trash totals separation:** Pending-in-Trash bytes and permanently reclaimed bytes are maintained as distinct fields, and moving to Trash reports zero permanently reclaimed space.

### Manual data safety gates (not executable in standard CI)

- [ ] **Multi-volume mount boundary:** Verify on macOS with a real external APFS volume or disk image that recursive directory deletion never traverses across a mount point into a second volume (unprivileged CI runners cannot mount physical volumes).
- [ ] **macOS Finder Trash Put Back:** Verify that restore operations detect occupied destinations and decline overwrite.
- [ ] **Maintainer threat-model review:** Security review of the classification catalogue and execution revalidation pipeline before production deletion is enabled.

## Quality

- [ ] Frontend build, tests, formatting, Rust checks, and Rust tests pass.
- [ ] Large-tree, low-disk, permission-denied, interrupted, and corrupt-database cases pass.
- [ ] Keyboard, screen reader, contrast, reduced motion, light, and dark checks pass.
- [ ] Performance and memory budgets are measured on representative hardware.

## Distribution

- [ ] macOS 13 minimum is encoded and verified on the oldest supported system.
- [ ] Universal Apple Silicon and Intel artifact is built, or architecture support is stated precisely.
- [ ] Hardened runtime, signing, notarization, and Gatekeeper checks pass.
- [ ] Fresh-machine install, upgrade, uninstall, and data-location behavior are documented and tested.
- [ ] Software bill of materials and dependency/security audit are recorded.

## Publication

- [ ] GitHub metadata, licence, security policy, contributor docs, and issue templates are current.
- [ ] A 60–90 second demo shows the tested end-to-end flow.
- [ ] `v1.0.0` tag points to the reviewed commit and release notes match the artifact.
- [ ] Release remains blocked if any data-loss or truthfulness issue is unresolved.
