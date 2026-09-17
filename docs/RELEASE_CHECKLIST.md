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

The destructive-safety suite (`src-tauri/src/classify/destructive_safety.rs`) runs on hermetic disposable fixtures (`DisposableFixtureTree`). Two things are true of it and both matter:

**Running today, on every pull request.** These exercise code that exists:

- **Protected-path enforcement.** Every protected root in the rule catalogue is rejected by the plan builder, and `PlannableClass` has no protected variant, so a plan item cannot carry one at all.
- **Classification under attack.** Symlinks from a rebuildable cache into a source tree or a credential store, a path replaced between classification and re-read, a cache directory inside a working tree, a credential file named like a cache, a mount boundary crossed mid-rule, and a durable model store that must not be called rebuildable.
- **Fixture containment.** The harness asserts every path it touches is inside the fixture root, refuses to follow a symlink out of it, and bounds its own walk. Disabling any of those fails a named test.

**Written, committed and deliberately not running.** Eleven tests carry `#[ignore]` naming the execution-engine milestone. They describe behaviour `execute_plan` and the restore path must have and **do not yet have**, because that engine is not written:

- pre-execution revalidation rejecting protected roots and identity changes,
- deletion removing a symlink without traversing its target,
- inode reallocation caught where name, size and mtime match,
- recursive deletion halting at a mount boundary,
- cancellation leaving neither half-deleted nor double-counted items,
- partial failure producing one outcome row per item across four categories,
- a vanished target recorded rather than aborting the batch,
- Trash mode reporting zero permanently reclaimed bytes,
- restore declining to clobber an occupied destination,
- deletion issuing direct filesystem syscalls with no shell invocation.

They are commitments, not coverage. **Production deletion stays disabled until they are un-ignored and passing**, which is the gate this document means.

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
- [ ] Architecture support is stated precisely. **v1.0 is Apple Silicon only**; Intel Macs are not supported, and
      the release notes and README say so rather than leaving a reader to discover it.
- [ ] **Signing and notarization are deferred, so this release is unsigned.** Verify instead that the README and the
      release notes state "unsigned" plainly and give the right-click → Open step. Do not tick a signing box that
      nothing performed.
- [ ] _(Once an Apple Developer Program membership exists)_ Hardened runtime, signing, notarization and Gatekeeper
      checks pass. Before the first signed build, settle whether the certificate publishes an individual's legal name
      or an organisation name — a Developer ID signature is readable by anyone who runs `codesign -dv --verbose=4`.
- [ ] Fresh-machine install, upgrade, uninstall, and data-location behavior are documented and tested.
- [ ] Software bill of materials and dependency/security audit are recorded.

## Publication

- [ ] GitHub metadata, licence, security policy, contributor docs, and issue templates are current.
- [ ] A 60–90 second demo shows the tested end-to-end flow.
- [ ] `v1.0.0` tag points to the reviewed commit and release notes match the artifact.
- [ ] Release remains blocked if any data-loss or truthfulness issue is unresolved.
