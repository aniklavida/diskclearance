# v1.0 release checklist

## Product truth

- [ ] Every advertised feature is implemented and tested.
  - _Unticked: Core cleanup features are implemented and tested, but oldest-supported macOS (macOS 13) clean-install verification and code signing are deferred / pending._
- [x] Unsupported platforms and operations are explicit.
  - _Verified: Apple Silicon only (Intel Macs unsupported), Windows unsupported (compile check only), and no privileged helper (elevated operations out of scope) are stated in README.md, docs/SPEC.md, and enforced by `platform::tests::unsupported_adapter_returns_typed_unsupported_for_all_capabilities`._
- [x] Savings, Trash, and permanently reclaimed totals are accurate.
  - _Verified: Dual accounting tested by `classify::destructive_safety::test_prevent_trash_totals_from_being_summed_with_permanently_reclaimed_bytes`, `classify::destructive_safety::test_prevent_trash_execution_from_claiming_freed_disk_space`, `src/App.test.tsx` (reclaim-trash vs reclaim-freed), and duplicate zero-reclaim tests (`duplicates::tests::test_hardlinked_pair_reported_as_sharing_storage_with_zero_reclaimable_bytes`, `duplicates::tests::test_apfs_clone_pair_reported_as_sharing_storage_with_zero_reclaimable_bytes`)._
- [ ] README, screenshots, demo, changelog, and in-app copy agree.
  - _Unticked: README, changelog, and in-app copy agree on capabilities and constraints; however, demo recording and packaged v1.0 screenshots have not yet been produced (out of scope for truthfulness gate, pending maintainer)._

## Data safety

- [x] Protected-path, symlink, race, mount-boundary, cancellation, and partial-failure suites pass.
  - _Verified: 22 automated destructive-safety tests pass in CI (`classify::destructive_safety::*`), including `test_prevent_symlink_deletion_from_traversing_outward_or_destroying_target`, `test_prevent_execution_engine_from_deleting_swapped_target`, `test_prevent_revalidation_when_device_id_changes_across_mounts`, `test_prevent_mid_execution_cancellation_from_leaving_half_deleted_or_double_counted_items`, and `test_prevent_partial_failures_from_being_conflated_or_reported_as_blanket_success`._
- [x] All destructive tests run only in disposable fixtures.
  - _Verified: Enforced by `scan::traversal::tests::test_no_test_touches_any_path_outside_its_temporary_directory` and `DisposableFixtureTree` containment tests (`scan::fixtures::tests::harness_asserts_containment_and_blocks_traversal_escape`, `scan::fixtures::tests::harness_asserts_containment_and_blocks_absolute_escape`)._
- [x] Every action is backed by a reviewed plan and immediate revalidation.
  - _Verified: Plan builder and revalidation pipeline tested in `classify::plan::tests::test_plan_builder_admits_rebuildable_and_review`, `classify::destructive_safety::test_prevent_injected_protected_root_from_passing_pre_execution_revalidation`, and frontend boundary client (`src/boundary/client.test.ts`)._
- [x] Restore is offered only for an exact item still available in Trash.
  - _Verified: `classify::destructive_safety::test_prevent_restore_from_overwriting_occupied_destination`, `classify::destructive_safety::test_prevent_restore_engine_from_clobbering_existing_file_at_destination`, and `src/App.test.tsx` (permanent deletions offer no restore button; destination collision detection with alternate offer)._
- [ ] A third-party security review or documented maintainer threat-model review is complete.
  - _Unticked: Formal third-party audit and maintainer threat-model review signoff have not yet been conducted._

### Automated test coverage

The destructive-safety suite (`src-tauri/src/classify/destructive_safety.rs`) runs on hermetic disposable fixtures (`DisposableFixtureTree`).

**Running and passing in CI on every pull request:**

- **Protected-path enforcement:** Every protected root in the rule catalogue is rejected by the plan builder (`classify::plan::tests::test_plan_builder_rejects_protected_finding`), and `PlannableClass` has no protected variant (`classify::tests::test_every_protected_root_unrepresentable_as_plan_item`).
- **Classification under attack:** Symlinks from a rebuildable cache into a source tree or credential store, path replaced between classification and re-read, cache inside working tree, credential file named like a cache, mount boundary crossed mid-rule, and durable model stores refusing Rebuildable class (`classify::tests::*`).
- **Fixture containment:** Harness asserts every path it touches is inside the fixture root, refuses to follow symlinks outward, and bounds traversal (`scan::fixtures::tests::*`).
- **Pre-execution revalidation:** Revalidation pipeline rejects protected roots, swapped targets, recreated inodes, vanished targets, and device ID shifts across mounts (`classify::destructive_safety::test_prevent_execution_engine_from_deleting_swapped_target`, `test_prevent_execution_engine_from_deleting_recreated_inode_target`, `test_prevent_revalidation_when_device_id_changes_across_mounts`).
- **Symlink deletion safety:** Deletion removes symlinks without traversing targets (`classify::destructive_safety::test_prevent_symlink_deletion_from_traversing_outward_or_destroying_target`).
- **Cancellation mid-execution:** Cancellation leaves neither half-deleted nor double-counted items (`classify::destructive_safety::test_prevent_mid_execution_cancellation_from_leaving_half_deleted_or_double_counted_items`).
- **Partial failure reporting:** Partial failure produces individual outcome records without conflating failures into overall success (`classify::destructive_safety::test_prevent_partial_failures_from_being_conflated_or_reported_as_blanket_success`).
- **Hostile filename isolation:** Direct filesystem syscalls with no shell invocation (`classify::destructive_safety::test_prevent_execution_engine_from_invoking_shell_on_hostile_names`).
- **Truthful accounting:** Move to Trash reports zero permanently reclaimed bytes and segregates pending Trash totals (`classify::destructive_safety::test_prevent_trash_execution_from_claiming_freed_disk_space`, `test_prevent_trash_totals_from_being_summed_with_permanently_reclaimed_bytes`).
- **Safe restore:** Destination collision detection blocks clobbering existing files (`classify::destructive_safety::test_prevent_restore_from_overwriting_occupied_destination`, `test_prevent_restore_engine_from_clobbering_existing_file_at_destination`).

**Committed and deliberately ignored in automated CI:**

- `classify::destructive_safety::test_prevent_recursive_deletion_from_crossing_mount_boundary`: carries `#[ignore]` because mounting a physical secondary APFS volume requires root privileges unavailable in standard CI runners; tracked as a manual gate below.

### Manual data safety gates (not executable in standard CI)

- [ ] **Multi-volume mount boundary:** Verify on macOS with a real external APFS volume or disk image that recursive directory deletion never traverses across a mount point into a second volume (unprivileged CI runners cannot mount physical volumes).
  - _Unticked: Requires manual test with an attached physical volume or disk image (`classify::destructive_safety::test_prevent_recursive_deletion_from_crossing_mount_boundary` remains `#[ignore]` until verified)._
- [ ] **macOS Finder Trash Put Back:** Verify that restore operations detect occupied destinations and decline overwrite.
  - _Unticked: Automated adapter simulation passes (`platform::tests::test_adapter_simulates_trash_lifecycle`), but manual end-to-end verification with macOS Finder Trash lifecycle on physical hardware is pending._
- [ ] **Maintainer threat-model review:** Security review of the classification catalogue and execution revalidation pipeline before production deletion is enabled.
  - _Unticked: Pending formal maintainer security signoff._

## Quality

- [x] Frontend build, tests, formatting, Rust checks, and Rust tests pass.
  - _Verified: `npm run check` clean on macOS — TypeScript types check clean, Prettier format check clean, Vitest 58/58 tests pass across 6 test files, Vite build completes cleanly, `cargo fmt --check` clean, `cargo test` 120 passed (0 failed, 1 ignored for unprivileged CI multi-volume mount), and `cargo check` clean._
- [x] Large-tree, low-disk, permission-denied, interrupted, and corrupt-database cases exercised; measured behavior is recorded in [`docs/PERFORMANCE.md`](PERFORMANCE.md).
- [x] Automated accessibility and responsive conformance suite passes:
  - Contrast check passes on every semantic token pair in light and dark mode (`src/tokens.test.ts`).
  - Focus-visible presence check (2px accent outline, 2px offset) across interactive elements (`src/tokens.test.ts`).
  - Interactive element minimum target size check (>= 32×32 px) (`src/tokens.test.ts`).
  - Keyboard traversal of Home, Cleanup, Explore, and confirmation sheets with focus trapping and focus restoration (`src/accessibility.test.tsx`).
  - Screen reader VoiceOver finding row labels and accessible status cues (`src/accessibility.test.tsx`).
  - Navigable folder table equivalent for treemap (`src/accessibility.test.tsx`).
  - Reduced Motion chart animation and transition elimination (`src/tokens.test.ts`).
  - Responsive layout and review tray non-occluding clearance at 760×560 px (`src/accessibility.test.tsx`).
  - _Verified: 17 token tests in `src/tokens.test.ts` and 12 accessibility tests in `src/accessibility.test.tsx` pass in Vitest._
- [x] Performance and memory budgets are measured on the recorded Apple M4 / macOS 26.3 build 25D125 host; see [`docs/PERFORMANCE.md`](PERFORMANCE.md) for the exact measurements and scope.

### Manual accessibility and responsive gates (not executable in standard CI)

- [ ] **macOS VoiceOver hardware pass:** Navigate the built application using native VoiceOver (`Option-Command-F5`) on Apple Silicon hardware:
  - Verify finding rows announce entity name, size, safety class, selection state, recoverability, and available keyboard actions without audio clipping.
  - Verify live scan announcements are throttled at the emitter so assistive technology speech output is not flooded.
  - Verify the Explore view's folder table equivalent provides full keyboard traversal and VoiceOver table reading order (`VO-arrows`).
  - Verify destructive confirmation sheets announce action title, consequence in text, and disabled state on the permanent delete button until acknowledged.
  - _Unticked: Automated label and ARIA attributes pass (`src/accessibility.test.tsx`), but manual physical VoiceOver hardware audition is pending._
- [ ] **200% zoom and large-text display verification:**
  - Verify application usability at the minimum window size (`760 × 560 px`) with macOS Accessibility Display "Larger Text" enabled and zoom at 200%.
  - Verify that navigation controls, finding rows, review tray figures, and modal dialogs remain visible, reachable, and wrap without truncation or clipping.
  - _Unticked: Automated viewport clearance passes (`src/accessibility.test.tsx`), but manual hardware visual inspection under 200% display zoom is pending._

## Distribution

- [ ] macOS 13 minimum is encoded and verified on the oldest supported system.
  - _Unticked: Clean-install and oldest-supported macOS (macOS 13 Ventura) verification has not been performed; testing has occurred exclusively on the Apple M4 development host (macOS 26.3 build 25D125)._
- [x] Architecture support is stated precisely. **v1.0 is Apple Silicon only**; Intel Macs are not supported, and the release notes and README say so rather than leaving a reader to discover it.
  - _Verified: README.md, CHANGELOG.md, and docs/SPEC.md explicitly declare Apple Silicon only and Intel Macs unsupported._
- [x] **Signing and notarization are deferred, so this release is unsigned.** Verify instead that the README and the release notes state "unsigned" plainly and give the right-click → Open step. Do not tick a signing box that nothing performed.
  - _Verified: README.md and CHANGELOG.md state "unsigned" plainly and document the right-click → Open step._
- [ ] _(Once an Apple Developer Program membership exists)_ Hardened runtime, signing, notarization and Gatekeeper checks pass. Before the first signed build, settle whether the certificate publishes an individual's legal name or an organisation name — a Developer ID signature is readable by anyone who runs `codesign -dv --verbose=4`.
  - _Unticked: Code signing and notarization are deferred; Apple Developer Program membership not yet configured._
- [ ] Fresh-machine install, upgrade, uninstall, and data-location behavior are documented and tested.
  - _Unticked: Fresh-machine installation and upgrade lifecycle tests have not yet been performed._
- [ ] Software bill of materials and dependency/security audit are recorded.
  - _Unticked: Dependency licences and security advisory audits are documented in `docs/DEPENDENCIES.md`, and `scripts/generate-sbom.sh` is provided; release SBOM generation is deferred until release artifact packaging._

## Publication

- [ ] GitHub metadata, licence, security policy, contributor docs, and issue templates are current.
  - _Unticked: Pending final maintainer pre-release verification of repository settings, release metadata, and public URLs._
- [ ] A 60–90 second demo shows the tested end-to-end flow.
  - _Unticked: Demo recording has not yet been produced (out of scope, left for maintainer)._
- [ ] `v1.0.0` tag points to the reviewed commit and release notes match the artifact.
  - _Unticked: `v1.0.0` tag has not been created (out of scope, left for maintainer)._
- [ ] Release remains blocked if any data-loss or truthfulness issue is unresolved.
  - _Unticked: Release gate remains active; release is blocked until maintainer completes manual gates and publishes release._
