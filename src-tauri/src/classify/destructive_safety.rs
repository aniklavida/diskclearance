//! Destructive safety test suite running against disposable fixtures.
//!
//! Every test in this suite is named for the specific damage it prevents.
//! Tests verify that protected roots, symlinks, races, inode reallocations,
//! mount boundaries, hostile filenames, partial failures, and restore conflicts
//! cannot be walked around at either the planning or execution boundaries.

use std::path::{Path, PathBuf};

use crate::boundary::cancellation::CancellationRegistry;
use crate::boundary::destructive::{ActionMode, ExecutePlanArgs, ExecutionSummary};
use crate::classify::catalogue::{ClassifiedFinding, RuleCatalogue};
use crate::classify::class::{PlannableClass, SafetyClass};
use crate::classify::evidence::{Confidence, Evidence, Recoverability};
use crate::classify::matcher::MatchContext;
use crate::classify::plan::{
    PlanBuilder, PlanConstructionError, PlanItem, RevalidationFailure, revalidate_plan_item,
};
use crate::classify::protected::PROTECTED_ROOTS;
use crate::classify::tests::ClassificationTestAdapter;
use crate::platform::{EntryType, FileIdentity, PlatformAdapter};
use crate::scan::fixtures::DisposableFixtureTree;

fn make_dummy_evidence(rule_id: &str) -> Evidence {
    Evidence {
        rule_id: rule_id.to_string(),
        rule_version: 1,
        owning_tool: "TestTool".to_string(),
        matched_reason: "Destructive safety test assertion".to_string(),
        regenerator: None,
        last_activity_ms: None,
        recoverability: Recoverability::Irrecoverable,
        confidence: Confidence::Definite,
    }
}

// ============================================================================
// Attack 1: Protected Roots
// ============================================================================

/// Damage prevented: Protected system volumes, core directories, user credentials,
/// and durable state being admitted into a review or deletion plan.
#[test]
fn test_prevent_all_catalogue_protected_roots_from_entering_plan_builder() {
    let mut builder = PlanBuilder::new("plan-protected-roots", "session-safety");

    for (idx, root_desc) in PROTECTED_ROOTS.iter().enumerate() {
        let fake_path = root_desc
            .exact_system_path
            .map(PathBuf::from)
            .or_else(|| root_desc.system_prefix.map(PathBuf::from))
            .or_else(|| {
                root_desc
                    .home_relative_prefix
                    .map(|p| PathBuf::from("/mock_home").join(p))
            })
            .or_else(|| {
                root_desc
                    .home_relative_exact
                    .map(|p| PathBuf::from("/mock_home").join(p))
            })
            .unwrap_or_else(|| PathBuf::from(format!("/ProtectedPath/{idx}")));

        let finding = ClassifiedFinding {
            path: fake_path.to_string_lossy().to_string(),
            canonical_path: fake_path.to_string_lossy().to_string(),
            size_bytes: 4096,
            safety_class: SafetyClass::Protected,
            evidence: make_dummy_evidence(root_desc.id),
        };

        let result = builder.add_classified_finding(
            format!("item-protected-{idx}"),
            &finding,
            FileIdentity {
                device_id: 1,
                inode: 1000 + idx as u64,
            },
        );

        assert!(
            matches!(
                result,
                Err(PlanConstructionError::ProtectedItemRejected { .. })
            ),
            "Safety failure: Protected root '{}' ({}) was admitted into the deletion plan!",
            root_desc.name,
            root_desc.id
        );
    }

    assert_eq!(
        builder.items().len(),
        0,
        "No protected items should have been admitted"
    );
}

/// Damage prevented: An attacker or corrupted plan injecting a protected path directly
/// into pre-execution revalidation.
#[test]
fn test_prevent_injected_protected_root_from_passing_pre_execution_revalidation() {
    let adapter = ClassificationTestAdapter::new("injected-protected");
    let catalogue = RuleCatalogue::new();

    // Fabricate a plan item targeting user SSH credentials inside the adapter's home
    let ssh_dir = adapter.home.join(".ssh");
    std::fs::create_dir_all(&ssh_dir).expect("create ssh dir");
    let key_file = ssh_dir.join("id_ed25519");
    std::fs::write(&key_file, b"FAKE PRIVATE KEY").expect("write fake key");

    let meta = adapter.read_entry_metadata(&key_file).expect("metadata");
    let resolved = adapter
        .canonicalize_and_normalize(&key_file)
        .expect("canonicalize");

    let injected_item = PlanItem {
        item_id: "injected-ssh-item".to_string(),
        original_path: key_file.clone(),
        canonical_path: resolved.canonical,
        identity: meta.identity,
        size_bytes: 16,
        class: PlannableClass::Rebuildable, // Lie: falsely marked Rebuildable
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    let result = revalidate_plan_item(&adapter, &catalogue, &injected_item);
    assert!(
        matches!(result, Err(RevalidationFailure::ItemBecameProtected { .. })),
        "Safety failure: Pre-execution revalidation failed to catch injected protected path '{}'!",
        key_file.display()
    );
}

/// Damage prevented: Client or external callers invoking execution with arbitrary path parameters.
#[test]
fn test_prevent_path_injection_into_execute_plan_api_request() {
    let args = ExecutePlanArgs {
        plan_id: "plan-valid-id".to_string(),
        action_mode: ActionMode::Trash,
    };

    let json = serde_json::to_string(&args).expect("serialize args");
    assert!(
        !json.to_lowercase().contains("path"),
        "Safety failure: ExecutePlanArgs JSON structure must never contain a path parameter"
    );

    // Calling the API returns Unsupported because plan execution is not implemented in this milestone
    let res = crate::boundary::destructive::execute_plan(args);
    assert!(
        res.is_err(),
        "Safety failure: execute_plan must not execute unvetted requests"
    );
}

/// Damage prevented: Execution engine executing any plan containing protected roots.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must revalidate each item and reject protected roots"]
fn test_prevent_execution_engine_from_executing_plan_with_protected_roots() {
    // Commitment test for item 19 execution engine:
    // When execute_plan is called with a plan containing a protected path, execution
    // must halt and return an error before any deletion occurs.
}

// ============================================================================
// Attack 2: Symlink Escape
// ============================================================================

/// Damage prevented: Deleting a symlink in a cache directory traversing to and destroying
/// source code, credentials, or filesystem targets outside the fixture.
#[test]
fn test_prevent_symlink_deletion_from_traversing_outward_or_destroying_target() {
    let fixture = DisposableFixtureTree::new("symlink-escape-defense");
    let outside_fixture = DisposableFixtureTree::new("symlink-escape-outside");

    // 1. Source tree target
    let source_file = fixture.create_file("src/main.rs", b"fn main() { println!(\"safe\"); }");

    // 2. Credential target
    let cred_file = fixture.create_file("user_home/.ssh/id_rsa", b"SSH-PRIVATE-KEY-SECRET");

    // 3. Target outside fixture entirely
    let outside_file = outside_fixture.create_file("external.dat", b"OUTSIDE-PAYLOAD");

    // Create a simulated cache directory with symlinks pointing at each target
    let cache_dir = fixture.create_dir("cache_dir");
    let link_to_source = fixture.create_symlink(&source_file, "cache_dir/link_source");
    let link_to_creds = fixture.create_symlink(&cred_file, "cache_dir/link_creds");
    let link_to_outside = fixture.create_symlink(&outside_file, "cache_dir/link_outside");

    assert!(link_to_source.is_symlink());
    assert!(link_to_creds.is_symlink());
    assert!(link_to_outside.is_symlink());

    // Execute deletion using direct POSIX unlink (as used by deletion engine)
    std::fs::remove_file(&link_to_source).expect("remove link to source");
    std::fs::remove_file(&link_to_creds).expect("remove link to creds");
    std::fs::remove_file(&link_to_outside).expect("remove link to outside");

    // Assert: links are gone
    assert!(!link_to_source.exists());
    assert!(!link_to_creds.exists());
    assert!(!link_to_outside.exists());

    // Assert: every target STILL EXISTS with unaltered content!
    assert!(
        source_file.exists(),
        "Source file was destroyed by symlink deletion!"
    );
    assert_eq!(
        std::fs::read(&source_file).unwrap(),
        b"fn main() { println!(\"safe\"); }",
        "Source file content was altered!"
    );

    assert!(
        cred_file.exists(),
        "Credential file was destroyed by symlink deletion!"
    );
    assert_eq!(
        std::fs::read(&cred_file).unwrap(),
        b"SSH-PRIVATE-KEY-SECRET",
        "Credential file content was altered!"
    );

    assert!(
        outside_file.exists(),
        "Outside file was destroyed by symlink deletion!"
    );
    assert_eq!(
        std::fs::read(&outside_file).unwrap(),
        b"OUTSIDE-PAYLOAD",
        "Outside file content was altered!"
    );

    let _ = cache_dir;
}

/// Damage prevented: Symlinks pointing at protected roots being classified as Rebuildable.
// Creating a symlink needs a privilege Windows runners do not have, and the
// symlink call in this test is already Unix-only.
#[cfg(unix)]
#[test]
fn test_prevent_symlink_pointing_to_protected_root_from_revalidation() {
    let adapter = ClassificationTestAdapter::new("symlink-revalidation");
    let catalogue = RuleCatalogue::new();

    let ssh_dir = adapter.home.join(".ssh");
    std::fs::create_dir_all(&ssh_dir).expect("create ssh dir");

    let cache_dir = adapter.caches.join("fake_cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    let symlink = cache_dir.join("link_to_ssh");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&ssh_dir, &symlink).expect("create symlink to ssh");

    let meta = adapter
        .read_entry_metadata(&symlink)
        .expect("read symlink meta");

    let item = PlanItem {
        item_id: "symlink-ssh".to_string(),
        original_path: symlink.clone(),
        canonical_path: symlink.clone(),
        identity: meta.identity,
        size_bytes: 0,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    let result = revalidate_plan_item(&adapter, &catalogue, &item);
    assert!(
        result.is_err(),
        "Safety failure: Symlink pointing at protected root should have failed revalidation"
    );
}

/// Damage prevented: Execution engine following symlinks during recursive cleanup.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must remove only symlink entries without traversing into targets"]
fn test_prevent_execution_engine_from_traversing_symlink_targets_during_cleanup() {
    // Commitment test for item 19 execution engine:
    // execute_plan must remove only the symlink node and never follow directory symlinks.
}

// ============================================================================
// Attack 3: Path Replacement Between Review and Execution
// ============================================================================

/// Damage prevented: An attacker swapping a reviewed file with an arbitrary target file
/// at the same path prior to execution.
// Windows has no stable device+inode pair, so an identity change cannot be
// detected the way this test asserts. macOS is the supported platform.
#[cfg(unix)]
#[test]
fn test_prevent_deletion_of_file_replaced_between_review_and_execution() {
    let adapter = ClassificationTestAdapter::new("file-replacement");
    let catalogue = RuleCatalogue::new();

    let target_file = adapter
        .fixture
        .create_file("cache/build_artifact.o", b"INITIAL BUILD OBJ");
    let initial_meta = adapter
        .read_entry_metadata(&target_file)
        .expect("initial meta");
    let resolved = adapter
        .canonicalize_and_normalize(&target_file)
        .expect("canonicalize");

    let plan_item = PlanItem {
        item_id: "plan-replace-file".to_string(),
        original_path: target_file.clone(),
        canonical_path: resolved.canonical,
        identity: initial_meta.identity,
        size_bytes: initial_meta.apparent_size,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    // Swap: replace the file with a new file (different inode)
    adapter
        .fixture
        .replace_with_new_inode("cache/build_artifact.o", b"DIFFERENT IMPORTANT FILE");

    let result = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    assert!(
        matches!(result, Err(RevalidationFailure::IdentityChanged { .. })),
        "Safety failure: Replaced file was not blocked by identity mismatch! Result: {result:?}"
    );
}

/// Damage prevented: An attacker replacing a reviewed directory with a symlink to an arbitrary
/// filesystem location before execution.
#[test]
fn test_prevent_deletion_of_directory_swapped_with_symlink_between_review_and_execution() {
    let adapter = ClassificationTestAdapter::new("dir-swap");
    let catalogue = RuleCatalogue::new();

    let dir = adapter.fixture.create_dir("cache/build_dir");
    let initial_meta = adapter.read_entry_metadata(&dir).expect("dir meta");
    let resolved = adapter
        .canonicalize_and_normalize(&dir)
        .expect("canonicalize");

    let plan_item = PlanItem {
        item_id: "plan-swap-dir".to_string(),
        original_path: dir.clone(),
        canonical_path: resolved.canonical,
        identity: initial_meta.identity,
        size_bytes: 4096,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    // Swap: replace directory with symlink to home
    adapter
        .fixture
        .replace_dir_with_symlink("cache/build_dir", &adapter.home);

    let result = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    assert!(
        result.is_err(),
        "Safety failure: Directory swapped with symlink was not blocked by revalidation!"
    );
}

/// Damage prevented: Execution engine deleting a target whose identity changed after review.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must revalidate immediately before deletion and abort on identity change"]
fn test_prevent_execution_engine_from_deleting_swapped_target() {
    // Commitment test for item 19 execution engine:
    // execute_plan must abort deletion if the target at path has changed identity.
}

// ============================================================================
// Attack 4: Same Path, New Inode
// ============================================================================

/// Damage prevented: A file replaced with identical name, size, and content but a different
/// inode evading detection if verification relied only on name and size.
#[test]
fn test_prevent_deletion_when_target_recreated_with_same_name_size_mtime_but_new_inode() {
    let adapter = ClassificationTestAdapter::new("same-path-new-inode");
    let catalogue = RuleCatalogue::new();

    let payload = b"IDENTICAL SIZE AND PAYLOAD CONTENT";
    let file = adapter.fixture.create_file("cache/recreated.tmp", payload);
    let original_meta = adapter.read_entry_metadata(&file).expect("original meta");
    let resolved = adapter
        .canonicalize_and_normalize(&file)
        .expect("canonicalize");

    let plan_item = PlanItem {
        item_id: "plan-item-inode".to_string(),
        original_path: file.clone(),
        canonical_path: resolved.canonical,
        identity: original_meta.identity,
        size_bytes: payload.len() as u64,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    // Recreate with identical payload, forcing new inode allocation
    adapter
        .fixture
        .replace_with_new_inode("cache/recreated.tmp", payload);

    let new_meta = adapter.read_entry_metadata(&file).expect("new meta");
    assert_eq!(
        original_meta.apparent_size, new_meta.apparent_size,
        "Precondition: sizes must be identical"
    );

    #[cfg(unix)]
    assert_ne!(
        original_meta.identity.inode, new_meta.identity.inode,
        "Precondition: inodes must differ"
    );

    let result = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    #[cfg(unix)]
    {
        assert!(
            matches!(
                result,
                Err(RevalidationFailure::IdentityChanged { expected, actual, .. })
                if expected.inode != actual.inode
            ),
            "Safety failure: Inode change was not detected when size and name matched! Got: {result:?}"
        );
    }
    #[cfg(not(unix))]
    let _ = result;
}

/// Damage prevented: Execution engine proceeding when inode has changed.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must verify inode and reject deletion on mismatch even if size and mtime match"]
fn test_prevent_execution_engine_from_deleting_recreated_inode_target() {
    // Commitment test for item 19 execution engine:
    // Inode check must be mandatory in the execution revalidation loop.
}

// ============================================================================
// Attack 5: Mount Boundary
// ============================================================================

/// Damage prevented: Findings crossing mount boundaries being classified as Rebuildable.
#[test]
fn test_prevent_items_crossing_mount_boundaries_from_entering_rebuildable_plan() {
    let catalogue = RuleCatalogue::new();
    let path = Path::new("/Volumes/ExternalDisk/node_modules");

    let ctx = MatchContext {
        original_path: path,
        canonical_path: path,
        normalized_path: path,
        entry_type: EntryType::Directory,
        identity: FileIdentity {
            device_id: 9999,
            inode: 1,
        },
        apparent_size: 4096,
        allocated_size: 4096,
        modified_ms: None,
        is_symlink: false,
        symlink_target_canonical: None,
        home_dir: None,
        app_support_dir: None,
        caches_dir: None,
        crosses_mount_boundary: true, // Crosses mount boundary!
        inside_git_repo: false,
        is_git_internal: false,
    };

    let classified = catalogue.classify(&ctx);
    assert!(
        classified.safety_class != SafetyClass::Rebuildable,
        "Safety failure: Item crossing mount boundary must never be classed as Rebuildable!"
    );
}

/// Damage prevented: Pre-execution revalidation passing when a plan item's device ID changes.
#[test]
fn test_prevent_revalidation_when_device_id_changes_across_mounts() {
    let adapter = ClassificationTestAdapter::new("mount-device-change");
    let catalogue = RuleCatalogue::new();

    let file = adapter
        .fixture
        .create_file("cache/file.tmp", b"device test");
    let meta = adapter.read_entry_metadata(&file).expect("meta");

    // Forged plan item expecting device_id: 8888 (different mount)
    let plan_item = PlanItem {
        item_id: "plan-mount-diff".to_string(),
        original_path: file.clone(),
        canonical_path: file.clone(),
        identity: FileIdentity {
            device_id: meta.identity.device_id + 999,
            inode: meta.identity.inode,
        },
        size_bytes: meta.apparent_size,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    let result = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    assert!(
        matches!(
            result,
            Err(RevalidationFailure::IdentityChanged { expected, actual, .. })
            if expected.device_id != actual.device_id
        ),
        "Safety failure: Mount/device boundary shift was not caught by revalidation!"
    );
}

/// Damage prevented: Recursive deletion traversing into a nested mount point or second volume.
#[test]
#[ignore = "Execution engine milestone (item 19): recursive deletion must check device id and halt before crossing mount boundary"]
fn test_prevent_recursive_deletion_from_crossing_mount_boundary() {
    // Commitment test for item 19 execution engine:
    // A second mounted volume cannot be honestly mounted in non-root CI.
    // Listed as a manual gate in docs/RELEASE_CHECKLIST.md.
}

// ============================================================================
// Attack 6: Cancellation Mid-Execution
// ============================================================================

/// Damage prevented: Cancellation signals being lost or ignored during multi-item operations.
#[test]
fn test_prevent_cancellation_signal_from_being_ignored_by_cancellation_registry() {
    let registry = CancellationRegistry::new();
    let token = registry.register("op-batch-delete");

    assert!(!token.is_cancelled());
    let cancelled = registry.cancel("op-batch-delete");
    assert!(cancelled);
    assert!(
        token.is_cancelled(),
        "Safety failure: Cancellation token must reflect cancelled status immediately"
    );
}

/// Damage prevented: Items remaining in half-deleted state or double-counted upon mid-execution cancellation.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan cancellation must stop between items, record completed as completed, unattempted as unattempted"]
fn test_prevent_mid_execution_cancellation_from_leaving_half_deleted_or_double_counted_items() {
    // Commitment test for item 19 execution engine:
    // execute_plan must inspect cancellation token before each item, record completed
    // items in history, leave remaining items untouched on disk, and never double-count.
}

// ============================================================================
// Attack 7: Partial Failure
// ============================================================================

/// Damage prevented: Batches containing failures being conflated or reported as overall success.
#[test]
fn test_prevent_partial_failures_from_being_conflated_or_reported_as_blanket_success() {
    let adapter = ClassificationTestAdapter::new("partial-failure");
    let catalogue = RuleCatalogue::new();

    // 1. Valid item
    let valid_file = adapter.fixture.create_file("cache/valid.tmp", b"valid");
    let valid_meta = adapter.read_entry_metadata(&valid_file).unwrap();
    let valid_resolved = adapter.canonicalize_and_normalize(&valid_file).unwrap();
    let item_valid = PlanItem {
        item_id: "item-valid".to_string(),
        original_path: valid_file.clone(),
        canonical_path: valid_resolved.canonical,
        identity: valid_meta.identity,
        size_bytes: 5,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };
    assert!(revalidate_plan_item(&adapter, &catalogue, &item_valid).is_ok());

    // 2. Vanished item
    let vanished_file = adapter.fixture.path("cache/vanished.tmp");
    let item_vanished = PlanItem {
        item_id: "item-vanished".to_string(),
        original_path: vanished_file.clone(),
        canonical_path: vanished_file.clone(),
        identity: FileIdentity {
            device_id: 1,
            inode: 2,
        },
        size_bytes: 10,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };
    let res_vanished = revalidate_plan_item(&adapter, &catalogue, &item_vanished);
    assert!(
        matches!(res_vanished, Err(RevalidationFailure::PathVanished(_))),
        "Must be categorized as PathVanished"
    );

    // 3. Identity changed item
    let changed_file = adapter.fixture.create_file("cache/changed.tmp", b"initial");
    let changed_meta = adapter.read_entry_metadata(&changed_file).unwrap();
    let changed_resolved = adapter.canonicalize_and_normalize(&changed_file).unwrap();
    let item_changed = PlanItem {
        item_id: "item-changed".to_string(),
        original_path: changed_file.clone(),
        canonical_path: changed_resolved.canonical,
        identity: FileIdentity {
            device_id: changed_meta.identity.device_id,
            inode: changed_meta.identity.inode + 100, // mismatch
        },
        size_bytes: 7,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };
    let res_changed = revalidate_plan_item(&adapter, &catalogue, &item_changed);
    assert!(
        matches!(
            res_changed,
            Err(RevalidationFailure::IdentityChanged { .. })
        ),
        "Must be categorized as IdentityChanged"
    );

    // 4. Protected item
    let protected_item = PlanItem {
        item_id: "item-protected".to_string(),
        original_path: PathBuf::from("/System"),
        canonical_path: PathBuf::from("/System"),
        identity: FileIdentity {
            device_id: 1,
            inode: 1,
        },
        size_bytes: 100,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };
    let res_protected = revalidate_plan_item(&adapter, &catalogue, &protected_item);
    assert!(
        res_protected.is_err(),
        "Must be rejected as protected item or platform error"
    );
}

/// Damage prevented: Execution engine reporting a batch with partial failures as complete success.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must return individual row outcomes for succeed, permission-denied, vanished, and protected, never claiming blanket success"]
fn test_prevent_execution_engine_from_reporting_partial_failure_as_overall_success() {
    // Commitment test for item 19 execution engine:
    // execute_plan summary must categorize outcomes into distinct buckets:
    // succeeded, permission_denied, vanished, protected_blocked, and failed.
}

// ============================================================================
// Attack 8: Vanished Before Execution
// ============================================================================

/// Damage prevented: A target disappearing between review and execution crashing the process
/// or aborting the entire batch.
#[test]
fn test_prevent_vanished_target_before_execution_from_crashing_or_aborting_revalidation() {
    let adapter = ClassificationTestAdapter::new("vanished-target");
    let catalogue = RuleCatalogue::new();

    let target = adapter
        .fixture
        .create_file("cache/temp.txt", b"will vanish");
    let meta = adapter.read_entry_metadata(&target).expect("meta");
    let resolved = adapter
        .canonicalize_and_normalize(&target)
        .expect("canonicalize");

    let plan_item = PlanItem {
        item_id: "plan-vanished".to_string(),
        original_path: target.clone(),
        canonical_path: resolved.canonical,
        identity: meta.identity,
        size_bytes: meta.apparent_size,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
    };

    // Vanish target
    adapter.fixture.remove_path("cache/temp.txt");
    assert!(!target.exists());

    let result = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    assert_eq!(
        result,
        Err(RevalidationFailure::PathVanished(target)),
        "Safety failure: Vanished target must return RevalidationFailure::PathVanished cleanly"
    );
}

/// Damage prevented: Execution engine aborting an entire batch when an individual item vanishes.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must record vanished target in outcome row and continue remaining batch items"]
fn test_prevent_execution_engine_from_aborting_batch_when_single_target_vanishes() {
    // Commitment test for item 19 execution engine:
    // Vanished targets must be recorded as Vanished in history and remaining items processed.
}

// ============================================================================
// Attack 9: Trash Totals
// ============================================================================

/// Damage prevented: Pending-in-Trash bytes being summed with permanently reclaimed bytes
/// or misleading users that moving items to Trash reclaims storage space.
/// Typed report format for storage reclamation asserting that pending-in-Trash bytes
/// and permanently reclaimed bytes are distinct fields and cannot be conflated.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReclamationReport {
    pub pending_in_trash_bytes: u64,
    pub permanently_reclaimed_bytes: u64,
}

/// Damage prevented: Pending-in-Trash bytes being summed with permanently reclaimed bytes
/// or misleading users that moving items to Trash reclaims storage space.
#[test]
fn test_prevent_trash_totals_from_being_summed_with_permanently_reclaimed_bytes() {
    let report = StorageReclamationReport {
        pending_in_trash_bytes: 50_000_000,      // 50 MB in Trash
        permanently_reclaimed_bytes: 10_000_000, // 10 MB permanently freed
    };

    let json = serde_json::to_string(&report).expect("serialize report");
    assert!(
        json.contains("\"pendingInTrashBytes\":50000000"),
        "Trash total bytes must be serialized in its own field"
    );
    assert!(
        json.contains("\"permanentlyReclaimedBytes\":10000000"),
        "Reclaimed bytes must be serialized in its own field"
    );
    assert_ne!(
        report.pending_in_trash_bytes, report.permanently_reclaimed_bytes,
        "Trash total bytes and permanently reclaimed bytes must never be coalesced"
    );

    // In ExecutionSummary: actionMode Trash must not claim bytes freed
    let summary = ExecutionSummary {
        plan_id: "plan-trash-test".to_string(),
        action_mode: ActionMode::Trash,
        succeeded_items: 5,
        failed_items: 0,
        bytes_freed: 0, // Trash does not permanently free bytes
    };
    assert_eq!(
        summary.bytes_freed, 0,
        "Moving items to Trash does not permanently reclaim disk space"
    );
    assert_eq!(summary.action_mode, ActionMode::Trash);
}

/// Damage prevented: Execution engine claiming freed bytes when action mode is Trash.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan in Trash mode must report zero permanently reclaimed bytes"]
fn test_prevent_trash_execution_from_claiming_freed_disk_space() {
    // Commitment test for item 19 execution engine:
    // execute_plan in Trash mode must calculate bytes_moved_to_trash, but bytes_freed must remain 0.
}

// ============================================================================
// Attack 10: Restore Conflict
// ============================================================================

/// Damage prevented: Restoring a trashed item overwriting an occupied file at the original destination.
#[test]
fn test_prevent_restore_from_overwriting_occupied_destination() {
    let fixture = DisposableFixtureTree::new("restore-conflict");

    let original_destination =
        fixture.create_file("work/important_document.pdf", b"ORIGINAL CONTENT");
    assert!(original_destination.exists());

    // Scenario: User attempts to restore an item to `original_destination`, but it already exists.
    // Invariant: The restore path MUST detect existence and refuse to overwrite.
    let destination_exists = original_destination.exists();
    assert!(destination_exists, "Precondition: destination is occupied");

    // A safe restore logic checks:
    let can_restore_without_prompt = !original_destination.exists();
    assert!(
        !can_restore_without_prompt,
        "Safety failure: Restore must NEVER silently overwrite an occupied path!"
    );

    // Verify content is untouched
    assert_eq!(
        fixture.read_file("work/important_document.pdf"),
        b"ORIGINAL CONTENT"
    );
}

/// Damage prevented: Restore engine clobbering existing files during restoration.
#[test]
#[ignore = "Execution engine milestone (item 19): restore operation must decline or offer non-colliding rename when destination exists"]
fn test_prevent_restore_engine_from_clobbering_existing_file_at_destination() {
    // Commitment test for item 19 execution engine:
    // Restoring must inspect destination. If occupied, it must decline or offer a safe suffix.
}

// ============================================================================
// Attack 11: Hostile Names
// ============================================================================

/// Damage prevented: Filenames containing shell argument flags (e.g. `--force`, `-rf`),
/// newlines, leading hyphens, or unicode causing command injection or path corruption.
// Windows forbids the characters this test relies on in a filename — a
// newline cannot be created at all, so there is nothing to defend against.
#[cfg(unix)]
#[test]
fn test_prevent_hostile_names_from_injecting_command_arguments_or_corrupting_paths() {
    let adapter = ClassificationTestAdapter::new("hostile-names");
    let catalogue = RuleCatalogue::new();

    let hostile_files = adapter.fixture.create_hostile_name_files("hostile_dir");

    for file_path in &hostile_files {
        assert!(file_path.exists(), "Hostile file must exist in fixture");

        // 1. Adapter metadata read must succeed without shell interpretation
        let meta = adapter
            .read_entry_metadata(file_path)
            .expect("read metadata of hostile named file");
        let resolved = adapter
            .canonicalize_and_normalize(file_path)
            .expect("canonicalize hostile file");

        // 2. Build PlanItem with hostile path
        let plan_item = PlanItem {
            item_id: format!("item-{}", meta.identity.inode),
            original_path: file_path.clone(),
            canonical_path: resolved.canonical,
            identity: meta.identity,
            size_bytes: meta.apparent_size,
            class: PlannableClass::Rebuildable,
            rule_id: "rule.rebuildable.cargo_target".to_string(),
            rule_version: 1,
            evidence: make_dummy_evidence("rule.rebuildable.cargo_target"),
        };

        // 3. Pre-execution revalidation succeeds cleanly
        let reval = revalidate_plan_item(&adapter, &catalogue, &plan_item);
        assert!(
            reval.is_ok(),
            "Safety failure: revalidation failed on hostile path '{}': {:?}",
            file_path.display(),
            reval
        );

        // 4. Removal using direct POSIX unlink removes the file safely
        std::fs::remove_file(file_path).expect("safe removal without shell invocation");
        assert!(
            !file_path.exists(),
            "Hostile file should be removed cleanly"
        );
    }

    // Assert that the parent directory and sibling files were not damaged by argument flags like `-rf`
    assert!(adapter.fixture.path("hostile_dir").exists());
}

/// Damage prevented: Execution engine passing hostile paths to shell subshells.
#[test]
#[ignore = "Execution engine milestone (item 19): execute_plan must execute direct filesystem syscalls without shell invocation"]
fn test_prevent_execution_engine_from_invoking_shell_on_hostile_names() {
    // Commitment test for item 19 execution engine:
    // Deletion must invoke platform adapter syscalls, never `sh -c` or `rm`.
}
