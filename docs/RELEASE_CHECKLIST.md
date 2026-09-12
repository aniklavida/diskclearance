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
