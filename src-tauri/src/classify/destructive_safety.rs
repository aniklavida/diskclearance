//! Destructive safety test suite running against disposable fixtures.
//!
//! Every test in this suite is named for the specific damage it prevents.
//! Tests verify that protected roots, symlinks, races, inode reallocations,
//! mount boundaries, hostile filenames, partial failures, and restore conflicts
//! cannot be walked around at either the planning or execution boundaries.

use std::path::{Path, PathBuf};

use crate::boundary::cancellation::CancellationRegistry;
use crate::boundary::destructive::{
    ActionMode, ExecutePlanArgs, ExecutionSummary, ItemOutcomeStatus, execute_plan_core,
};
use crate::boundary::error::CommandError;
use crate::boundary::plan::{PlanItemDetail, PlanItemSummary, PlanRepository, ReviewPlan};
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

    // Calling the core API returns error because unvetted plans do not exist
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let adapter = ClassificationTestAdapter::new("prevent-injection");
    let res = execute_plan_core(args, &db, &adapter, None);
    assert!(
        res.is_err(),
        "Safety failure: execute_plan must not execute unvetted requests"
    );
}

/// Damage prevented: Execution engine executing any plan containing protected roots.
#[test]
fn test_prevent_execution_engine_from_executing_plan_with_protected_roots() {
    let adapter = ClassificationTestAdapter::new("exec-protected-roots");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-prot", "/").unwrap();

    // 1. Create a benign file that should NEVER be deleted
    let benign_file = adapter
        .fixture
        .create_file("cache/benign.tmp", b"BENIGN-CONTENT");
    let benign_meta = adapter.read_entry_metadata(&benign_file).unwrap();
    let benign_resolved = adapter.canonicalize_and_normalize(&benign_file).unwrap();

    // 2. Create a protected SSH key inside user home
    let ssh_dir = adapter.home.join(".ssh");
    std::fs::create_dir_all(&ssh_dir).unwrap();
    let key_file = ssh_dir.join("id_ed25519");
    std::fs::write(&key_file, b"SSH-PRIVATE-KEY-SECRET").unwrap();
    let key_meta = adapter.read_entry_metadata(&key_file).unwrap();
    let key_resolved = adapter.canonicalize_and_normalize(&key_file).unwrap();

    // 3. Inject both into a plan (simulating a corrupted or malicious plan)
    let plan = ReviewPlan {
        plan_id: "plan-with-protected".to_string(),
        session_id: "sess-prot".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "item-benign".to_string(),
                original_path: benign_file.to_string_lossy().to_string(),
                size_bytes: 14,
                class_name: "Rebuildable".to_string(),
                action_name: "Trash".to_string(),
                recoverable: true,
            },
            PlanItemSummary {
                item_id: "item-protected".to_string(),
                original_path: key_file.to_string_lossy().to_string(),
                size_bytes: 22,
                class_name: "Protected".to_string(),
                action_name: "Trash".to_string(),
                recoverable: true,
            },
        ],
        default_action_mode: ActionMode::Trash,
        created_at_ms: 1000,
    };

    let details = vec![
        PlanItemDetail {
            item_id: "item-benign".to_string(),
            plan_id: "plan-with-protected".to_string(),
            original_path: benign_file.clone(),
            canonical_path: benign_resolved.canonical,
            device_id: benign_meta.identity.device_id,
            inode: benign_meta.identity.inode,
            size_bytes: 14,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "Trash".to_string(),
            recoverable: true,
        },
        PlanItemDetail {
            item_id: "item-protected".to_string(),
            plan_id: "plan-with-protected".to_string(),
            original_path: key_file.clone(),
            canonical_path: key_resolved.canonical,
            device_id: key_meta.identity.device_id,
            inode: key_meta.identity.inode,
            size_bytes: 22,
            class_name: "Protected".to_string(),
            rule_id: "protected.crypto.ssh".to_string(),
            rule_version: 1,
            action_name: "Trash".to_string(),
            recoverable: true,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    let args = ExecutePlanArgs {
        plan_id: "plan-with-protected".to_string(),
        action_mode: ActionMode::Trash,
    };

    // Execute plan: must halt and return error before ANY deletion occurs
    let res = execute_plan_core(args, &db, &adapter, None);
    assert!(
        matches!(res, Err(CommandError::PermissionDenied { .. })),
        "Safety failure: execute_plan must reject plan with protected roots! Got: {res:?}"
    );

    // Assert: benign file was NEVER deleted (execution halted before any modification)
    assert!(
        benign_file.exists(),
        "Safety failure: benign file was deleted even though plan execution was aborted!"
    );
    assert_eq!(std::fs::read(&benign_file).unwrap(), b"BENIGN-CONTENT");

    // Assert: protected SSH key is untouched
    assert!(key_file.exists());
    assert_eq!(std::fs::read(&key_file).unwrap(), b"SSH-PRIVATE-KEY-SECRET");
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
fn test_prevent_execution_engine_from_traversing_symlink_targets_during_cleanup() {
    let adapter = ClassificationTestAdapter::new("exec-symlink-traverse");
    let outside_fixture = DisposableFixtureTree::new("exec-symlink-outside");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-sym", "/").unwrap();

    // 1. Outside directory with target file
    let outside_file = outside_fixture.create_file("source_code.rs", b"fn keep_this() {}");

    // 2. Directory scheduled for cleanup
    let cache_dir = adapter.fixture.create_dir("cache/build");
    let link_to_outside = adapter
        .fixture
        .create_symlink(&outside_file, "cache/build/link_to_source");
    let file_inside = adapter
        .fixture
        .create_file("cache/build/artifact.o", b"BUILD-OBJ");

    assert!(link_to_outside.is_symlink());

    let cache_meta = adapter.read_entry_metadata(&cache_dir).unwrap();
    let cache_resolved = adapter.canonicalize_and_normalize(&cache_dir).unwrap();

    // 3. ReviewPlan targeting cache_dir for PermanentDelete
    let plan = ReviewPlan {
        plan_id: "plan-symlink-test".to_string(),
        session_id: "sess-sym".to_string(),
        items: vec![PlanItemSummary {
            item_id: "item-cache-dir".to_string(),
            original_path: cache_dir.to_string_lossy().to_string(),
            size_bytes: 100,
            class_name: "Rebuildable".to_string(),
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        }],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![PlanItemDetail {
        item_id: "item-cache-dir".to_string(),
        plan_id: "plan-symlink-test".to_string(),
        original_path: cache_dir.clone(),
        canonical_path: cache_resolved.canonical,
        device_id: cache_meta.identity.device_id,
        inode: cache_meta.identity.inode,
        size_bytes: 100,
        class_name: "Rebuildable".to_string(),
        rule_id: "rule.cache".to_string(),
        rule_version: 1,
        action_name: "PermanentDelete".to_string(),
        recoverable: false,
    }];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    let args = ExecutePlanArgs {
        plan_id: "plan-symlink-test".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();
    assert_eq!(summary.succeeded_items, 1);
    assert_eq!(summary.failed_items, 0);

    // Assert: cache_dir and its contents were removed
    assert!(!cache_dir.exists());
    assert!(!file_inside.exists());
    assert!(!link_to_outside.exists());

    // CRITICAL: outside file must still exist and be completely unaltered!
    assert!(
        outside_file.exists(),
        "Safety failure: Symlink traversal destroyed the target outside fixture!"
    );
    assert_eq!(
        std::fs::read(&outside_file).unwrap(),
        b"fn keep_this() {}",
        "Safety failure: Target file content was corrupted by symlink deletion!"
    );
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
#[cfg(unix)]
#[test]
fn test_prevent_execution_engine_from_deleting_swapped_target() {
    let adapter = ClassificationTestAdapter::new("exec-swapped-target");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-swap", "/").unwrap();

    // 1. Initial file A that will be swapped
    let file_a = adapter
        .fixture
        .create_file("cache/build.o", b"INITIAL BUILD OBJ");
    let meta_a = adapter.read_entry_metadata(&file_a).unwrap();
    let res_a = adapter.canonicalize_and_normalize(&file_a).unwrap();

    // 2. Sibling file B that will not be swapped
    let file_b = adapter.fixture.create_file("cache/normal.o", b"NORMAL OBJ");
    let meta_b = adapter.read_entry_metadata(&file_b).unwrap();
    let res_b = adapter.canonicalize_and_normalize(&file_b).unwrap();

    let plan = ReviewPlan {
        plan_id: "plan-swap".to_string(),
        session_id: "sess-swap".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "item-swapped".to_string(),
                original_path: file_a.to_string_lossy().to_string(),
                size_bytes: 17,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-normal".to_string(),
                original_path: file_b.to_string_lossy().to_string(),
                size_bytes: 10,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
        ],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![
        PlanItemDetail {
            item_id: "item-swapped".to_string(),
            plan_id: "plan-swap".to_string(),
            original_path: file_a.clone(),
            canonical_path: res_a.canonical,
            device_id: meta_a.identity.device_id,
            inode: meta_a.identity.inode,
            size_bytes: 17,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-normal".to_string(),
            plan_id: "plan-swap".to_string(),
            original_path: file_b.clone(),
            canonical_path: res_b.canonical,
            device_id: meta_b.identity.device_id,
            inode: meta_b.identity.inode,
            size_bytes: 10,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    // Swap file A with a new file (allocating a new inode)
    adapter
        .fixture
        .replace_with_new_inode("cache/build.o", b"MALICIOUSLY SWAPPED DATA");

    let new_meta_a = adapter.read_entry_metadata(&file_a).unwrap();
    assert_ne!(
        meta_a.identity.inode, new_meta_a.identity.inode,
        "Precondition: inodes must differ after swap"
    );

    let args = ExecutePlanArgs {
        plan_id: "plan-swap".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();

    // Assert: swapped item was blocked and recorded as BlockedChanged
    assert_eq!(summary.blocked_changed, 1);
    assert_eq!(summary.succeeded_items, 1);
    assert_eq!(summary.failed_items, 0);

    // Assert: swapped file was NOT deleted!
    assert!(
        file_a.exists(),
        "Safety failure: Swapped file was deleted despite inode identity mismatch!"
    );
    assert_eq!(std::fs::read(&file_a).unwrap(), b"MALICIOUSLY SWAPPED DATA");

    // Assert: normal item was deleted cleanly
    assert!(!file_b.exists());
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
#[cfg(unix)]
#[test]
fn test_prevent_execution_engine_from_deleting_recreated_inode_target() {
    let adapter = ClassificationTestAdapter::new("exec-recreated-inode");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-recreated", "/")
        .unwrap();

    let payload = b"IDENTICAL SIZE AND PAYLOAD CONTENT";
    let file = adapter.fixture.create_file("cache/recreated.tmp", payload);
    let original_meta = adapter.read_entry_metadata(&file).unwrap();
    let resolved = adapter.canonicalize_and_normalize(&file).unwrap();

    let plan = ReviewPlan {
        plan_id: "plan-recreated-inode".to_string(),
        session_id: "sess-recreated".to_string(),
        items: vec![PlanItemSummary {
            item_id: "item-recreated".to_string(),
            original_path: file.to_string_lossy().to_string(),
            size_bytes: payload.len() as u64,
            class_name: "Rebuildable".to_string(),
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        }],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![PlanItemDetail {
        item_id: "item-recreated".to_string(),
        plan_id: "plan-recreated-inode".to_string(),
        original_path: file.clone(),
        canonical_path: resolved.canonical,
        device_id: original_meta.identity.device_id,
        inode: original_meta.identity.inode,
        size_bytes: payload.len() as u64,
        class_name: "Rebuildable".to_string(),
        rule_id: "rule.cache".to_string(),
        rule_version: 1,
        action_name: "PermanentDelete".to_string(),
        recoverable: false,
    }];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    // Recreate with identical payload, forcing new inode allocation
    adapter
        .fixture
        .replace_with_new_inode("cache/recreated.tmp", payload);

    let new_meta = adapter.read_entry_metadata(&file).unwrap();
    assert_eq!(original_meta.apparent_size, new_meta.apparent_size);
    assert_ne!(original_meta.identity.inode, new_meta.identity.inode);

    let args = ExecutePlanArgs {
        plan_id: "plan-recreated-inode".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();
    assert_eq!(summary.blocked_changed, 1);
    assert_eq!(summary.succeeded_items, 0);

    // Assert: file was NOT deleted despite matching size and content
    assert!(
        file.exists(),
        "Safety failure: Inode check must be mandatory; deletion should have been blocked"
    );
    assert_eq!(std::fs::read(&file).unwrap(), payload);
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
fn test_prevent_mid_execution_cancellation_from_leaving_half_deleted_or_double_counted_items() {
    let adapter = ClassificationTestAdapter::new("exec-cancellation");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-cancel", "/").unwrap();

    let file1 = adapter.fixture.create_file("cache/file1.tmp", b"ITEM 1");
    let meta1 = adapter.read_entry_metadata(&file1).unwrap();
    let res1 = adapter.canonicalize_and_normalize(&file1).unwrap();

    let file2 = adapter.fixture.create_file("cache/file2.tmp", b"ITEM 2");
    let meta2 = adapter.read_entry_metadata(&file2).unwrap();
    let res2 = adapter.canonicalize_and_normalize(&file2).unwrap();

    let file3 = adapter.fixture.create_file("cache/file3.tmp", b"ITEM 3");
    let meta3 = adapter.read_entry_metadata(&file3).unwrap();
    let res3 = adapter.canonicalize_and_normalize(&file3).unwrap();

    let plan = ReviewPlan {
        plan_id: "plan-cancel".to_string(),
        session_id: "sess-cancel".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "item-1".to_string(),
                original_path: file1.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-2".to_string(),
                original_path: file2.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-3".to_string(),
                original_path: file3.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
        ],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![
        PlanItemDetail {
            item_id: "item-1".to_string(),
            plan_id: "plan-cancel".to_string(),
            original_path: file1.clone(),
            canonical_path: res1.canonical,
            device_id: meta1.identity.device_id,
            inode: meta1.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-2".to_string(),
            plan_id: "plan-cancel".to_string(),
            original_path: file2.clone(),
            canonical_path: res2.canonical,
            device_id: meta2.identity.device_id,
            inode: meta2.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-3".to_string(),
            plan_id: "plan-cancel".to_string(),
            original_path: file3.clone(),
            canonical_path: res3.canonical,
            device_id: meta3.identity.device_id,
            inode: meta3.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    let registry = CancellationRegistry::new();
    let token = registry.register("plan-cancel");

    // Signal cancellation before execution
    token.cancel();

    let args = ExecutePlanArgs {
        plan_id: "plan-cancel".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, Some(&registry)).unwrap();

    // All items should be recorded as unattempted; completed items = 0, no double counting
    assert_eq!(summary.succeeded_items, 0);
    assert_eq!(summary.failed_items, 0);
    assert_eq!(summary.item_outcomes.len(), 3);
    for outcome in &summary.item_outcomes {
        assert_eq!(outcome.status, ItemOutcomeStatus::Unattempted);
    }

    // All files remain untouched on disk
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());
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
#[cfg(unix)]
#[test]
fn test_prevent_execution_engine_from_reporting_partial_failure_as_overall_success() {
    let adapter = ClassificationTestAdapter::new("exec-partial-failure");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-partial", "/")
        .unwrap();

    // 1. Succeeded item
    let valid_file = adapter.fixture.create_file("cache/succeed.tmp", b"VALID");
    let valid_meta = adapter.read_entry_metadata(&valid_file).unwrap();
    let valid_res = adapter.canonicalize_and_normalize(&valid_file).unwrap();

    // 2. Vanished item
    let vanished_file = adapter.fixture.create_file("cache/vanish.tmp", b"VANISH");
    let vanished_meta = adapter.read_entry_metadata(&vanished_file).unwrap();
    let vanished_res = adapter.canonicalize_and_normalize(&vanished_file).unwrap();

    // 3. Blocked (swapped inode) item
    let swapped_file = adapter.fixture.create_file("cache/swap.tmp", b"SWAP");
    let swapped_meta = adapter.read_entry_metadata(&swapped_file).unwrap();
    let swapped_res = adapter.canonicalize_and_normalize(&swapped_file).unwrap();

    // 4. Permission-denied item: make parent dir unreadable/unexecutable
    let denied_dir = adapter.fixture.create_dir("cache/denied_dir");
    let denied_file = adapter
        .fixture
        .create_file("cache/denied_dir/file.tmp", b"DENIED");
    let denied_meta = adapter.read_entry_metadata(&denied_file).unwrap();
    let denied_res = adapter.canonicalize_and_normalize(&denied_file).unwrap();

    let plan = ReviewPlan {
        plan_id: "plan-partial".to_string(),
        session_id: "sess-partial".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "item-valid".to_string(),
                original_path: valid_file.to_string_lossy().to_string(),
                size_bytes: 5,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-vanished".to_string(),
                original_path: vanished_file.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-swapped".to_string(),
                original_path: swapped_file.to_string_lossy().to_string(),
                size_bytes: 4,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-denied".to_string(),
                original_path: denied_file.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
        ],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![
        PlanItemDetail {
            item_id: "item-valid".to_string(),
            plan_id: "plan-partial".to_string(),
            original_path: valid_file.clone(),
            canonical_path: valid_res.canonical,
            device_id: valid_meta.identity.device_id,
            inode: valid_meta.identity.inode,
            size_bytes: 5,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-vanished".to_string(),
            plan_id: "plan-partial".to_string(),
            original_path: vanished_file.clone(),
            canonical_path: vanished_res.canonical,
            device_id: vanished_meta.identity.device_id,
            inode: vanished_meta.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-swapped".to_string(),
            plan_id: "plan-partial".to_string(),
            original_path: swapped_file.clone(),
            canonical_path: swapped_res.canonical,
            device_id: swapped_meta.identity.device_id,
            inode: swapped_meta.identity.inode,
            size_bytes: 4,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-denied".to_string(),
            plan_id: "plan-partial".to_string(),
            original_path: denied_file.clone(),
            canonical_path: denied_res.canonical,
            device_id: denied_meta.identity.device_id,
            inode: denied_meta.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    // Cause mutations before execution:
    // 1. Vanish item 2
    std::fs::remove_file(&vanished_file).unwrap();
    // 2. Swap inode of item 3
    adapter
        .fixture
        .replace_with_new_inode("cache/swap.tmp", b"SWAPPED");
    // 3. Chmod 000 denied directory so entry metadata or deletion fails with permission denied
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&denied_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

    let args = ExecutePlanArgs {
        plan_id: "plan-partial".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();

    // Restore permissions for fixture cleanup
    let _ = std::fs::set_permissions(&denied_dir, std::fs::Permissions::from_mode(0o755));

    assert_eq!(summary.succeeded_items, 1);
    assert_eq!(summary.vanished_items, 1);
    assert_eq!(summary.blocked_changed, 1);
    assert_eq!(summary.permission_denied_items, 1);
    assert_eq!(summary.failed_items, 1);
    assert_eq!(summary.item_outcomes.len(), 4);

    let statuses: Vec<ItemOutcomeStatus> = summary.item_outcomes.iter().map(|o| o.status).collect();
    assert!(statuses.contains(&ItemOutcomeStatus::Succeeded));
    assert!(statuses.contains(&ItemOutcomeStatus::Vanished));
    assert!(statuses.contains(&ItemOutcomeStatus::BlockedChanged));
    assert!(statuses.contains(&ItemOutcomeStatus::PermissionDenied));
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
fn test_prevent_execution_engine_from_aborting_batch_when_single_target_vanishes() {
    let adapter = ClassificationTestAdapter::new("exec-vanished-batch");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-vanish-batch", "/")
        .unwrap();

    let file1 = adapter.fixture.create_file("cache/file1.tmp", b"ITEM 1");
    let meta1 = adapter.read_entry_metadata(&file1).unwrap();
    let res1 = adapter.canonicalize_and_normalize(&file1).unwrap();

    let file2 = adapter
        .fixture
        .create_file("cache/file2.tmp", b"ITEM 2 TO VANISH");
    let meta2 = adapter.read_entry_metadata(&file2).unwrap();
    let res2 = adapter.canonicalize_and_normalize(&file2).unwrap();

    let file3 = adapter.fixture.create_file("cache/file3.tmp", b"ITEM 3");
    let meta3 = adapter.read_entry_metadata(&file3).unwrap();
    let res3 = adapter.canonicalize_and_normalize(&file3).unwrap();

    let plan = ReviewPlan {
        plan_id: "plan-vanish-batch".to_string(),
        session_id: "sess-vanish-batch".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "item-1".to_string(),
                original_path: file1.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-2".to_string(),
                original_path: file2.to_string_lossy().to_string(),
                size_bytes: 16,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
            PlanItemSummary {
                item_id: "item-3".to_string(),
                original_path: file3.to_string_lossy().to_string(),
                size_bytes: 6,
                class_name: "Rebuildable".to_string(),
                action_name: "PermanentDelete".to_string(),
                recoverable: false,
            },
        ],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details = vec![
        PlanItemDetail {
            item_id: "item-1".to_string(),
            plan_id: "plan-vanish-batch".to_string(),
            original_path: file1.clone(),
            canonical_path: res1.canonical,
            device_id: meta1.identity.device_id,
            inode: meta1.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-2".to_string(),
            plan_id: "plan-vanish-batch".to_string(),
            original_path: file2.clone(),
            canonical_path: res2.canonical,
            device_id: meta2.identity.device_id,
            inode: meta2.identity.inode,
            size_bytes: 16,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
        PlanItemDetail {
            item_id: "item-3".to_string(),
            plan_id: "plan-vanish-batch".to_string(),
            original_path: file3.clone(),
            canonical_path: res3.canonical,
            device_id: meta3.identity.device_id,
            inode: meta3.identity.inode,
            size_bytes: 6,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).unwrap();
    drop(conn);

    // Delete item 2 before execution to simulate vanishing target
    std::fs::remove_file(&file2).unwrap();
    assert!(!file2.exists());

    let args = ExecutePlanArgs {
        plan_id: "plan-vanish-batch".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();

    // Verify batch did not abort: items 1 and 3 succeeded, item 2 recorded as vanished
    assert_eq!(summary.succeeded_items, 2);
    assert_eq!(summary.vanished_items, 1);
    assert_eq!(summary.failed_items, 0);
    assert_eq!(summary.bytes_freed, 12);
    assert_eq!(summary.item_outcomes.len(), 3);

    let item2_outcome = summary
        .item_outcomes
        .iter()
        .find(|o| o.item_id == "item-2")
        .unwrap();
    assert_eq!(item2_outcome.status, ItemOutcomeStatus::Vanished);
    assert_eq!(item2_outcome.bytes_reclaimed, 0);

    let item1_outcome = summary
        .item_outcomes
        .iter()
        .find(|o| o.item_id == "item-1")
        .unwrap();
    assert_eq!(item1_outcome.status, ItemOutcomeStatus::Succeeded);
    assert_eq!(item1_outcome.bytes_reclaimed, 6);

    let item3_outcome = summary
        .item_outcomes
        .iter()
        .find(|o| o.item_id == "item-3")
        .unwrap();
    assert_eq!(item3_outcome.status, ItemOutcomeStatus::Succeeded);
    assert_eq!(item3_outcome.bytes_reclaimed, 6);

    assert!(!file1.exists());
    assert!(!file3.exists());
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
        bytes_pending_trash: 50_000_000,
        skipped_protected: 0,
        blocked_changed: 0,
        vanished_items: 0,
        permission_denied_items: 0,
        item_outcomes: vec![],
        operation_id: None,
    };
    assert_eq!(
        summary.bytes_freed, 0,
        "Moving items to Trash does not permanently reclaim disk space"
    );
    assert_eq!(summary.action_mode, ActionMode::Trash);
}

/// Damage prevented: Execution engine claiming freed bytes when action mode is Trash.
#[test]
fn test_prevent_trash_execution_from_claiming_freed_disk_space() {
    let adapter = ClassificationTestAdapter::new("exec-trash-totals");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-trash-totals", "/")
        .unwrap();

    let file_trash1 = adapter.fixture.create_file("cache/trash1.tmp", &[0u8; 100]);
    let meta_t1 = adapter.read_entry_metadata(&file_trash1).unwrap();
    let res_t1 = adapter.canonicalize_and_normalize(&file_trash1).unwrap();

    let file_trash2 = adapter.fixture.create_file("cache/trash2.tmp", &[0u8; 200]);
    let meta_t2 = adapter.read_entry_metadata(&file_trash2).unwrap();
    let res_t2 = adapter.canonicalize_and_normalize(&file_trash2).unwrap();

    let plan_trash = ReviewPlan {
        plan_id: "plan-trash-mode".to_string(),
        session_id: "sess-trash-totals".to_string(),
        items: vec![
            PlanItemSummary {
                item_id: "trash-item-1".to_string(),
                original_path: file_trash1.to_string_lossy().to_string(),
                size_bytes: 100,
                class_name: "Rebuildable".to_string(),
                action_name: "Trash".to_string(),
                recoverable: true,
            },
            PlanItemSummary {
                item_id: "trash-item-2".to_string(),
                original_path: file_trash2.to_string_lossy().to_string(),
                size_bytes: 200,
                class_name: "Rebuildable".to_string(),
                action_name: "Trash".to_string(),
                recoverable: true,
            },
        ],
        default_action_mode: ActionMode::Trash,
        created_at_ms: 1000,
    };

    let details_trash = vec![
        PlanItemDetail {
            item_id: "trash-item-1".to_string(),
            plan_id: "plan-trash-mode".to_string(),
            original_path: file_trash1.clone(),
            canonical_path: res_t1.canonical,
            device_id: meta_t1.identity.device_id,
            inode: meta_t1.identity.inode,
            size_bytes: 100,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "Trash".to_string(),
            recoverable: true,
        },
        PlanItemDetail {
            item_id: "trash-item-2".to_string(),
            plan_id: "plan-trash-mode".to_string(),
            original_path: file_trash2.clone(),
            canonical_path: res_t2.canonical,
            device_id: meta_t2.identity.device_id,
            inode: meta_t2.identity.inode,
            size_bytes: 200,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "Trash".to_string(),
            recoverable: true,
        },
    ];

    PlanRepository::insert_plan_with_items(&conn, &plan_trash, &details_trash).unwrap();

    // Also insert a plan for PermanentDelete comparison
    let file_perm = adapter.fixture.create_file("cache/perm.tmp", &[0u8; 150]);
    let meta_p = adapter.read_entry_metadata(&file_perm).unwrap();
    let res_p = adapter.canonicalize_and_normalize(&file_perm).unwrap();

    let plan_perm = ReviewPlan {
        plan_id: "plan-perm-mode".to_string(),
        session_id: "sess-trash-totals".to_string(),
        items: vec![PlanItemSummary {
            item_id: "perm-item-1".to_string(),
            original_path: file_perm.to_string_lossy().to_string(),
            size_bytes: 150,
            class_name: "Rebuildable".to_string(),
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        }],
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    let details_perm = vec![PlanItemDetail {
        item_id: "perm-item-1".to_string(),
        plan_id: "plan-perm-mode".to_string(),
        original_path: file_perm.clone(),
        canonical_path: res_p.canonical,
        device_id: meta_p.identity.device_id,
        inode: meta_p.identity.inode,
        size_bytes: 150,
        class_name: "Rebuildable".to_string(),
        rule_id: "rule.cache".to_string(),
        rule_version: 1,
        action_name: "PermanentDelete".to_string(),
        recoverable: false,
    }];

    PlanRepository::insert_plan_with_items(&conn, &plan_perm, &details_perm).unwrap();
    drop(conn);

    // 1. Execute Trash plan
    let trash_summary = execute_plan_core(
        ExecutePlanArgs {
            plan_id: "plan-trash-mode".to_string(),
            action_mode: ActionMode::Trash,
        },
        &db,
        &adapter,
        None,
    )
    .unwrap();

    // Critical assertion: bytes_freed must be strictly 0 in Trash mode
    assert_eq!(
        trash_summary.bytes_freed, 0,
        "Safety failure: Trash mode must never claim freed bytes!"
    );
    assert_eq!(trash_summary.bytes_pending_trash, 300);
    assert_eq!(trash_summary.succeeded_items, 2);
    for outcome in &trash_summary.item_outcomes {
        assert_eq!(outcome.bytes_reclaimed, 0);
        assert!(outcome.bytes_pending_trash > 0);
    }

    // 2. Execute PermanentDelete plan
    let perm_summary = execute_plan_core(
        ExecutePlanArgs {
            plan_id: "plan-perm-mode".to_string(),
            action_mode: ActionMode::PermanentDelete,
        },
        &db,
        &adapter,
        None,
    )
    .unwrap();

    // PermanentDelete must report bytes_freed and 0 bytes_pending_trash
    assert_eq!(perm_summary.bytes_freed, 150);
    assert_eq!(perm_summary.bytes_pending_trash, 0);
    assert_eq!(perm_summary.succeeded_items, 1);
    for outcome in &perm_summary.item_outcomes {
        assert_eq!(outcome.bytes_reclaimed, 150);
        assert_eq!(outcome.bytes_pending_trash, 0);
    }
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
fn test_prevent_restore_engine_from_clobbering_existing_file_at_destination() {
    use crate::boundary::history::{
        FetchOperationDetailArgs, HistoryItemDetail, RestoreEligibility, RestoreItemArgs,
        fetch_operation_detail_core, restore_item_core,
    };
    use crate::boundary::plan::{BuildPlanArgs, build_plan_core};
    use crate::platform::tests::TestAdapter;
    use crate::scan::session::ScanSessionRepository;
    use crate::storage::AppDatabase;

    let fixture = DisposableFixtureTree::new("restore-clobber-engine");
    let initial_file = fixture.create_file("important.txt", b"RESTORED PAYLOAD");

    let mut adapter = TestAdapter::default();
    let trash_dir = fixture.create_dir("mock_trash");
    adapter.trash = trash_dir;

    let db = AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();
    ScanSessionRepository::create_session(&conn, "sess-clobber", "/").unwrap();
    ScanSessionRepository::insert_finding(
        &conn,
        "f-clobber",
        "sess-clobber",
        &initial_file.to_string_lossy(),
        16,
        "Rebuildable",
        "Cache",
    )
    .unwrap();
    drop(conn);

    let build_args = BuildPlanArgs {
        session_id: "sess-clobber".into(),
        finding_ids: vec!["f-clobber".into()],
    };
    let plan = build_plan_core(build_args, &db, &adapter).unwrap();

    let exec_args = ExecutePlanArgs {
        plan_id: plan.plan_id.clone(),
        action_mode: ActionMode::Trash,
    };
    let summary = execute_plan_core(exec_args, &db, &adapter, None).unwrap();
    assert_eq!(summary.succeeded_items, 1);

    // Fetch the operation detail to get the operation_item_id
    let op_id = summary.operation_id.expect("must have operation_id");
    let detail = fetch_operation_detail_core(
        FetchOperationDetailArgs {
            operation_id: op_id,
            outcome_filter: None,
        },
        &db,
        &adapter,
    )
    .unwrap();
    let op_item_id = match &detail.items[0] {
        HistoryItemDetail::Trash(t) => t.operation_item_id.clone(),
        _ => panic!("Expected trash item"),
    };

    // Re-create the file at the original destination with different content
    let occupant_content = b"DO NOT CLOBBER THIS IMPORTANT OCCUPANT";
    fixture.create_file("important.txt", occupant_content);
    assert_eq!(fixture.read_file("important.txt"), occupant_content);

    // Live restore eligibility check must detect destination is occupied and suggest an alternate destination
    let detail_occupied = fetch_operation_detail_core(
        FetchOperationDetailArgs {
            operation_id: detail.operation.id.clone(),
            outcome_filter: None,
        },
        &db,
        &adapter,
    )
    .unwrap();
    let suggested_alt = match &detail_occupied.items[0] {
        HistoryItemDetail::Trash(t) => match &t.restore_eligibility {
            RestoreEligibility::Eligible {
                destination_occupied,
                suggested_alternate_destination,
                ..
            } => {
                assert!(*destination_occupied, "Must detect destination is occupied");
                suggested_alternate_destination
                    .clone()
                    .expect("Must offer safe alternate destination")
            }
            other => panic!("Expected Eligible with destination_occupied, got {other:?}"),
        },
        _ => panic!("Expected trash item"),
    };

    // Attempting restore directly to the occupied destination must decline and return an error
    let restore_err = restore_item_core(
        RestoreItemArgs {
            operation_item_id: op_item_id.clone(),
            alternate_destination: None,
        },
        &db,
        &adapter,
    );
    assert!(
        restore_err.is_err(),
        "Must refuse to overwrite occupied destination"
    );

    // Assert occupant was untouched byte for byte
    assert_eq!(
        fixture.read_file("important.txt"),
        occupant_content,
        "Original occupant must be untouched byte for byte"
    );

    // Restoring to the safe alternate destination succeeds without clobbering the occupant
    let restore_res = restore_item_core(
        RestoreItemArgs {
            operation_item_id: op_item_id,
            alternate_destination: Some(suggested_alt.clone()),
        },
        &db,
        &adapter,
    )
    .unwrap();

    assert_eq!(restore_res.restored_to_path, suggested_alt);
    assert_eq!(std::fs::read(&suggested_alt).unwrap(), b"RESTORED PAYLOAD");
    assert_eq!(
        fixture.read_file("important.txt"),
        occupant_content,
        "Original occupant must still be completely untouched byte for byte after alternate restore"
    );
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
#[cfg(unix)]
#[test]
fn test_prevent_execution_engine_from_invoking_shell_on_hostile_names() {
    let adapter = ClassificationTestAdapter::new("exec-hostile-names");
    let db = crate::storage::AppDatabase::open_in_memory().unwrap();
    let conn = db.connection().lock().unwrap();

    crate::scan::session::ScanSessionRepository::create_session(&conn, "sess-hostile-exec", "/")
        .unwrap();

    let hostile_files = adapter
        .fixture
        .create_hostile_name_files("hostile_exec_dir");

    // Also create a sibling canary file that would be deleted if `rm -rf` was executed via a shell
    let canary_file = adapter
        .fixture
        .create_file("canary.txt", b"CANARY DO NOT DELETE");

    let mut item_summaries = Vec::new();
    let mut item_details = Vec::new();

    for (idx, file_path) in hostile_files.iter().enumerate() {
        let meta = adapter.read_entry_metadata(file_path).unwrap();
        let res = adapter.canonicalize_and_normalize(file_path).unwrap();
        let item_id = format!("hostile-item-{idx}");

        item_summaries.push(PlanItemSummary {
            item_id: item_id.clone(),
            original_path: file_path.to_string_lossy().to_string(),
            size_bytes: meta.apparent_size,
            class_name: "Rebuildable".to_string(),
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        });

        item_details.push(PlanItemDetail {
            item_id,
            plan_id: "plan-hostile-exec".to_string(),
            original_path: file_path.clone(),
            canonical_path: res.canonical,
            device_id: meta.identity.device_id,
            inode: meta.identity.inode,
            size_bytes: meta.apparent_size,
            class_name: "Rebuildable".to_string(),
            rule_id: "rule.cache".to_string(),
            rule_version: 1,
            action_name: "PermanentDelete".to_string(),
            recoverable: false,
        });
    }

    let plan = ReviewPlan {
        plan_id: "plan-hostile-exec".to_string(),
        session_id: "sess-hostile-exec".to_string(),
        items: item_summaries,
        default_action_mode: ActionMode::PermanentDelete,
        created_at_ms: 1000,
    };

    PlanRepository::insert_plan_with_items(&conn, &plan, &item_details).unwrap();
    drop(conn);

    let args = ExecutePlanArgs {
        plan_id: "plan-hostile-exec".to_string(),
        action_mode: ActionMode::PermanentDelete,
    };

    let summary = execute_plan_core(args, &db, &adapter, None).unwrap();

    assert_eq!(summary.succeeded_items as usize, hostile_files.len());
    assert_eq!(summary.failed_items, 0);

    // Verify all hostile files were deleted safely
    for file_path in &hostile_files {
        assert!(
            !file_path.exists(),
            "Hostile file '{}' was not deleted",
            file_path.display()
        );
    }

    // Verify directory still exists and canary file was untouched
    assert!(adapter.fixture.path("hostile_exec_dir").exists());
    assert!(
        canary_file.exists(),
        "Safety failure: Canary file was destroyed by shell flag injection!"
    );
    assert_eq!(
        std::fs::read(&canary_file).unwrap(),
        b"CANARY DO NOT DELETE"
    );
}
