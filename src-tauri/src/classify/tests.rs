use std::path::{Path, PathBuf};

use crate::classify::catalogue::{ClassifiedFinding, RuleCatalogue};
use crate::classify::class::{
    PlannableClass, PlannableSafetyMarker, RebuildableMarker, ReviewMarker, SafetyClass,
    filter_default_selected,
};
use crate::classify::evidence::{Confidence, Evidence, EvidenceValidationError, Recoverability};
use crate::classify::matcher::MatchContext;
use crate::classify::plan::{
    PlanBuilder, PlanConstructionError, PlanItem, RevalidationFailure, revalidate_plan_item,
};
use crate::classify::protected::PROTECTED_ROOTS;
use crate::platform::{
    DiscoveredPath, EntryMetadata, EntryType, FileIdentity, MountBoundary, PathKind,
    PlatformAdapter, PlatformError, ResolvedPath, ScanRoot, ScopePermission, SettingsDestination,
    TrashedItem, TrashedItemStatus,
};
use crate::scan::fixtures::DisposableFixtureTree;

/// Mock test platform adapter for hermetic classification tests.
pub struct ClassificationTestAdapter {
    pub fixture: DisposableFixtureTree,
    pub home: PathBuf,
    pub caches: PathBuf,
    pub app_support: PathBuf,
}

impl ClassificationTestAdapter {
    pub fn new(prefix: &str) -> Self {
        let fixture = DisposableFixtureTree::new(prefix);
        let home = fixture.create_dir("user_home");
        let caches = fixture.create_dir("user_home/Library/Caches");
        let app_support = fixture.create_dir("user_home/Library/Application Support");

        Self {
            fixture,
            home,
            caches,
            app_support,
        }
    }

    pub fn make_context<'a>(
        &'a self,
        path: &'a Path,
        canonical: &'a Path,
        identity: FileIdentity,
        entry_type: EntryType,
        is_symlink: bool,
        symlink_target_canonical: Option<PathBuf>,
        crosses_mount_boundary: bool,
        inside_git_repo: bool,
        is_git_internal: bool,
    ) -> MatchContext<'a> {
        MatchContext {
            original_path: path,
            canonical_path: canonical,
            normalized_path: canonical,
            entry_type,
            identity,
            apparent_size: 4096,
            allocated_size: 4096,
            modified_ms: Some(1700000000),
            is_symlink,
            symlink_target_canonical,
            home_dir: Some(&self.home),
            app_support_dir: Some(&self.app_support),
            caches_dir: Some(&self.caches),
            crosses_mount_boundary,
            inside_git_repo,
            is_git_internal,
        }
    }
}

impl PlatformAdapter for ClassificationTestAdapter {
    fn platform_name(&self) -> &'static str {
        "classification_test_adapter"
    }

    fn home_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Ok(DiscoveredPath {
            path: self.home.clone(),
            kind: PathKind::Home,
        })
    }

    fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Ok(DiscoveredPath {
            path: self.app_support.clone(),
            kind: PathKind::ApplicationSupport,
        })
    }

    fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Ok(DiscoveredPath {
            path: self.caches.clone(),
            kind: PathKind::Caches,
        })
    }

    fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("trash"))
    }

    fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError> {
        Ok(vec![])
    }

    fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError> {
        Ok(vec![])
    }

    fn check_read_permission(&self, scope: &Path) -> Result<ScopePermission, PlatformError> {
        Ok(ScopePermission {
            scope: scope.to_path_buf(),
            is_readable: true,
            required_settings: None,
        })
    }

    fn settings_destination_for_scope(
        &self,
        _scope: &Path,
    ) -> Result<SettingsDestination, PlatformError> {
        Ok(SettingsDestination::SystemSettings)
    }

    fn move_to_trash(&self, path: &Path) -> Result<TrashedItem, PlatformError> {
        if !path.exists() && !path.is_symlink() {
            return Err(PlatformError::NotFound(path.to_path_buf()));
        }
        let trash_dir = self.fixture.path("simulated_trash");
        std::fs::create_dir_all(&trash_dir).map_err(|e| PlatformError::Io(e.to_string()))?;
        let filename = path.file_name().unwrap_or_default();
        let trashed_path = trash_dir.join(filename);
        std::fs::rename(path, &trashed_path).map_err(|e| PlatformError::Io(e.to_string()))?;
        Ok(TrashedItem {
            trashed_path,
            original_path: path.to_path_buf(),
            display_name: filename.to_string_lossy().to_string(),
        })
    }

    fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError> {
        Ok(vec![])
    }

    fn check_trashed_item_exists(
        &self,
        item: &TrashedItem,
    ) -> Result<TrashedItemStatus, PlatformError> {
        if item.trashed_path.exists() {
            Ok(TrashedItemStatus::Present(item.clone()))
        } else {
            Ok(TrashedItemStatus::Missing(item.trashed_path.clone()))
        }
    }

    fn restore_from_trash(
        &self,
        trashed_path: &Path,
        destination_path: &Path,
    ) -> Result<(), PlatformError> {
        if destination_path.exists() {
            return Err(PlatformError::Io(format!(
                "Destination '{}' already exists; refusing to overwrite",
                destination_path.display()
            )));
        }
        if let Some(parent) = destination_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| PlatformError::Io(e.to_string()))?;
            }
        }
        std::fs::rename(trashed_path, destination_path)
            .map_err(|e| PlatformError::Io(e.to_string()))
    }

    fn application_metadata(
        &self,
        path: &Path,
    ) -> Result<crate::platform::ApplicationMetadata, PlatformError> {
        Err(PlatformError::NotFound(path.to_path_buf()))
    }

    fn file_identity(&self, path: &Path) -> Result<FileIdentity, PlatformError> {
        let meta = std::fs::symlink_metadata(path).map_err(|e| PlatformError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(FileIdentity {
                device_id: meta.dev(),
                inode: meta.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            Err(PlatformError::Unsupported(
                "filesystem identity (device_id and inode) is not available on this platform",
            ))
        }
    }

    fn canonicalize_and_normalize(&self, path: &Path) -> Result<ResolvedPath, PlatformError> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        Ok(ResolvedPath {
            original: path.to_path_buf(),
            canonical: canonical.clone(),
            normalized: canonical,
            is_firmlink_alias: false,
        })
    }

    fn check_mount_boundary(
        &self,
        base: &Path,
        target: &Path,
    ) -> Result<MountBoundary, PlatformError> {
        let b = self.file_identity(base)?;
        let t = self.file_identity(target)?;
        Ok(MountBoundary {
            root_device_id: b.device_id,
            path_device_id: t.device_id,
            crosses_boundary: b.device_id != t.device_id,
        })
    }

    fn read_entry_metadata(&self, path: &Path) -> Result<EntryMetadata, PlatformError> {
        let meta = std::fs::symlink_metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound(path.to_path_buf()),
            std::io::ErrorKind::PermissionDenied => PlatformError::PermissionDenied {
                scope: path.to_path_buf(),
                reason: e.to_string(),
                settings_target: None,
            },
            _ => PlatformError::Io(e.to_string()),
        })?;
        let entry_type = if meta.is_symlink() {
            EntryType::Symlink
        } else if meta.is_dir() {
            EntryType::Directory
        } else if meta.is_file() {
            EntryType::File
        } else {
            EntryType::Other
        };

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(EntryMetadata {
                identity: FileIdentity {
                    device_id: meta.dev(),
                    inode: meta.ino(),
                },
                entry_type,
                apparent_size: meta.len(),
                allocated_size: meta.blocks() * 512,
                modified_ms: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64),
                nlink: meta.nlink(),
            })
        }
        #[cfg(not(unix))]
        {
            let (dev, ino) = match self.file_identity(path) {
                Ok(id) => (id.device_id, id.inode),
                Err(_) => (0, 0),
            };
            Ok(EntryMetadata {
                identity: FileIdentity {
                    device_id: dev,
                    inode: ino,
                },
                entry_type,
                apparent_size: meta.len(),
                allocated_size: meta.len(),
                modified_ms: None,
                nlink: 1,
            })
        }
    }

    fn file_extent_offset(&self, _path: &Path, _size_bytes: u64) -> Option<i64> {
        None
    }
}

// ---------------------------------------------------------------------------
// Done-when Test 1: Protected root unrepresentability
// ---------------------------------------------------------------------------

#[test]
fn test_every_protected_root_unrepresentable_as_plan_item() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("protected-roots");

    for root in PROTECTED_ROOTS {
        let path = if let Some(exact) = root.exact_system_path {
            PathBuf::from(exact)
        } else if let Some(prefix) = root.system_prefix {
            PathBuf::from(prefix).join("sample_item")
        } else if let Some(rel) = root.home_relative_prefix {
            adapter.home.join(rel).join("sample_item")
        } else if let Some(rel) = root.home_relative_exact {
            adapter.home.join(rel)
        } else {
            // Source control internal
            adapter.home.join("project/.git/HEAD")
        };

        let is_git_internal = root.id == "protected.source_control.git_internal";
        let ctx = adapter.make_context(
            &path,
            &path,
            FileIdentity {
                device_id: 1,
                inode: 1,
            },
            EntryType::File,
            false,
            None,
            false,
            is_git_internal,
            is_git_internal,
        );

        let classified = catalogue.classify(&ctx);
        assert_eq!(
            classified.safety_class,
            SafetyClass::Protected,
            "Protected root '{}' must classify as Protected",
            root.id
        );

        // Attempting to admit this finding into a plan must return a typed rejection
        let mut builder = PlanBuilder::new("plan-protected", "session-protected");
        let result = builder.add_classified_finding(
            "item-id",
            &classified,
            FileIdentity {
                device_id: 1,
                inode: 1,
            },
        );

        assert!(
            matches!(
                result,
                Err(PlanConstructionError::ProtectedItemRejected { .. })
            ),
            "Protected root '{}' must be rejected with ProtectedItemRejected",
            root.id
        );

        // Assert plan items remain strictly empty
        assert_eq!(
            builder.items().len(),
            0,
            "Plan must contain zero items after rejection of protected root '{}'",
            root.id
        );
    }
}

// ---------------------------------------------------------------------------
// Done-when Test 2: Default selection invariant
// ---------------------------------------------------------------------------

#[test]
fn test_no_review_class_finding_is_selected_by_default() {
    // Construct a fixture composed strictly of Review items
    let review_items: Vec<(String, SafetyClass)> = vec![
        ("download-archive.zip".to_string(), SafetyClass::Review),
        ("local-model-weights.bin".to_string(), SafetyClass::Review),
        ("untracked-source.rs".to_string(), SafetyClass::Review),
        ("heuristic-cache.tmp".to_string(), SafetyClass::Review),
        ("duplicate-copy.jpg".to_string(), SafetyClass::Review),
    ];

    let selected = filter_default_selected(&review_items);
    assert!(
        selected.is_empty(),
        "Default selection must be completely empty for Review-class items, got: {selected:?}"
    );

    // Also assert directly on the SafetyClass method
    assert!(
        !SafetyClass::Review.is_default_selected(),
        "SafetyClass::Review must not be default selected"
    );
    assert!(
        !SafetyClass::Protected.is_default_selected(),
        "SafetyClass::Protected must not be default selected"
    );
    assert!(
        SafetyClass::Rebuildable.is_default_selected(),
        "SafetyClass::Rebuildable may be default selected"
    );
}

// ---------------------------------------------------------------------------
// Done-when Test 3: Evidence completeness
// ---------------------------------------------------------------------------

#[test]
fn test_every_finding_carries_rule_and_non_empty_evidence() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("evidence-completeness");

    let test_paths = vec![
        adapter.home.join(".cargo/registry/cache"),
        adapter.home.join("Downloads/file.pdf"),
        adapter.home.join(".ssh/id_ed25519"),
    ];

    for path in test_paths {
        let ctx = adapter.make_context(
            &path,
            &path,
            FileIdentity {
                device_id: 1,
                inode: 1,
            },
            EntryType::File,
            false,
            None,
            false,
            false,
            false,
        );

        let classified = catalogue.classify(&ctx);
        assert!(!classified.evidence.rule_id.trim().is_empty());
        assert!(classified.evidence.rule_version > 0);
        assert!(!classified.evidence.owning_tool.trim().is_empty());
        assert!(!classified.evidence.matched_reason.trim().is_empty());
        assert!(classified.evidence.validate().is_ok());
    }

    // Now verify that any finding with empty evidence is rejected
    let mut builder = PlanBuilder::new("plan-ev", "session-ev");
    let mut bad_evidence = Evidence {
        rule_id: "".to_string(),
        rule_version: 1,
        owning_tool: "TestTool".to_string(),
        matched_reason: "Reason".to_string(),
        regenerator: None,
        last_activity_ms: None,
        recoverability: Recoverability::TrashRecoverable,
        confidence: Confidence::Definite,
    };

    let bad_finding = ClassifiedFinding {
        path: "/tmp/rebuildable".to_string(),
        canonical_path: "/tmp/rebuildable".to_string(),
        size_bytes: 100,
        safety_class: SafetyClass::Rebuildable,
        evidence: bad_evidence.clone(),
    };

    let res = builder.add_classified_finding(
        "bad-1",
        &bad_finding,
        FileIdentity {
            device_id: 1,
            inode: 1,
        },
    );
    assert_eq!(
        res,
        Err(PlanConstructionError::InvalidEvidence(
            EvidenceValidationError::EmptyRuleId
        ))
    );

    bad_evidence.rule_id = "rule.id".to_string();
    bad_evidence.owning_tool = "".to_string();
    let bad_finding2 = ClassifiedFinding {
        path: "/tmp/rebuildable".to_string(),
        canonical_path: "/tmp/rebuildable".to_string(),
        size_bytes: 100,
        safety_class: SafetyClass::Rebuildable,
        evidence: bad_evidence,
    };
    let res2 = builder.add_classified_finding(
        "bad-2",
        &bad_finding2,
        FileIdentity {
            device_id: 1,
            inode: 1,
        },
    );
    assert_eq!(
        res2,
        Err(PlanConstructionError::InvalidEvidence(
            EvidenceValidationError::EmptyOwningTool
        ))
    );
}

// ---------------------------------------------------------------------------
// Done-when Test 4: The Six Named Tests
// ---------------------------------------------------------------------------

/// 1. A symlink from a Rebuildable cache into a source tree
#[test]
fn test_symlink_from_rebuildable_cache_into_source_tree() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("symlink-source");

    // Create source tree and cache inside fixture
    let _source_dir = adapter.fixture.create_dir("user_projects/my_project/src");
    let source_file = adapter
        .fixture
        .create_file("user_projects/my_project/src/lib.rs", b"pub fn work() {}");
    let cache_dir = adapter.caches.join("my_app_cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");

    let symlink_path = cache_dir.join("link_to_source");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source_file, &symlink_path).expect("create symlink");

    let ctx = adapter.make_context(
        &symlink_path,
        &symlink_path,
        FileIdentity {
            device_id: 1,
            inode: 200,
        },
        EntryType::Symlink,
        true,
        Some(source_file.clone()),
        false,
        true, // Target is inside user source repo / project
        false,
    );

    let classified = catalogue.classify(&ctx);
    assert_ne!(
        classified.safety_class,
        SafetyClass::Rebuildable,
        "Symlink into a source tree must never be classified as Rebuildable"
    );
    assert_eq!(
        classified.safety_class,
        SafetyClass::Review,
        "Symlink into a source tree must be classified as Review"
    );
}

/// 2. A path replaced between classification and re-read
#[test]
fn test_path_replaced_between_classification_and_reread() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("replaced-path");

    let file_path = adapter
        .fixture
        .create_file("user_home/.cargo/registry/cache/item.crate", b"original");
    let initial_meta = adapter
        .read_entry_metadata(&file_path)
        .expect("read initial meta");

    let plan_item = PlanItem {
        item_id: "plan-item-replace".to_string(),
        original_path: file_path.clone(),
        canonical_path: file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone()),
        identity: initial_meta.identity,
        size_bytes: initial_meta.apparent_size,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 1,
        evidence: Evidence {
            rule_id: "rule.rebuildable.cargo_target".to_string(),
            rule_version: 1,
            owning_tool: "Cargo".to_string(),
            matched_reason: "Matches Cargo cache".to_string(),
            regenerator: Some("cargo build".to_string()),
            last_activity_ms: None,
            recoverability: Recoverability::RebuildableByTool {
                command: "cargo build".to_string(),
            },
            confidence: Confidence::Definite,
        },
    };

    // Before replacement: revalidation succeeds
    assert!(revalidate_plan_item(&adapter, &catalogue, &plan_item).is_ok());

    // Replace the file: remove it and create a new file (different inode or replaced with symlink)
    std::fs::remove_file(&file_path).expect("remove file");
    let replaced_file = adapter
        .fixture
        .create_file("user_home/.cargo/registry/cache/item.crate", b"replaced");
    let new_meta = adapter
        .read_entry_metadata(&replaced_file)
        .expect("read new meta");

    #[cfg(unix)]
    {
        // On Unix, the new file has a different inode or we verify identity mismatch
        if new_meta.identity != initial_meta.identity {
            let reval = revalidate_plan_item(&adapter, &catalogue, &plan_item);
            assert!(
                matches!(reval, Err(RevalidationFailure::IdentityChanged { .. })),
                "Revalidation must detect file identity change: got {reval:?}"
            );
        }
    }

    // Now replace with a symlink to /System
    std::fs::remove_file(&file_path).ok();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(Path::new("/System"), &file_path).expect("create symlink");
        let reval = revalidate_plan_item(&adapter, &catalogue, &plan_item);
        assert!(
            reval.is_err(),
            "Revalidation must reject path replaced with symlink to /System"
        );
    }
}

/// 3. A cache directory sitting inside a .git working tree
#[test]
fn test_cache_directory_sitting_inside_git_working_tree() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("git-working-tree");

    let _repo_dir = adapter.fixture.create_dir("user_home/my_repo");
    let _git_dir = adapter.fixture.create_dir("user_home/my_repo/.git");
    let internal_cache = adapter.fixture.create_dir("user_home/my_repo/.git/cache");
    let working_tree_cache = adapter.fixture.create_dir("user_home/my_repo/cache");

    // Inside .git internal: must be Protected
    let ctx_internal = adapter.make_context(
        &internal_cache,
        &internal_cache,
        FileIdentity {
            device_id: 1,
            inode: 301,
        },
        EntryType::Directory,
        false,
        None,
        false,
        true,
        true, // is_git_internal
    );
    let classified_internal = catalogue.classify(&ctx_internal);
    assert_eq!(
        classified_internal.safety_class,
        SafetyClass::Protected,
        "A cache inside .git must be Protected"
    );

    // Sitting inside working tree (not internal): must be Review, not Rebuildable
    let ctx_working_tree = adapter.make_context(
        &working_tree_cache,
        &working_tree_cache,
        FileIdentity {
            device_id: 1,
            inode: 302,
        },
        EntryType::Directory,
        false,
        None,
        false,
        true,  // inside_git_repo
        false, // not is_git_internal
    );
    let classified_working_tree = catalogue.classify(&ctx_working_tree);
    assert_eq!(
        classified_working_tree.safety_class,
        SafetyClass::Review,
        "A cache sitting inside a .git working tree must be classified as Review"
    );
    assert!(
        !classified_working_tree.safety_class.is_default_selected(),
        "A cache sitting inside a .git working tree must never be selected by default"
    );
}

/// 4. A credential file named like a cache
#[test]
fn test_credential_file_named_like_cache() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("cred-cache");

    let aws_cache = adapter
        .fixture
        .create_file("user_home/.aws/credentials.cache", b"aws_secret_key=XYZ");
    let ssh_cache = adapter
        .fixture
        .create_file("user_home/.ssh/id_rsa.cache", b"PRIVATE_KEY");

    for cred_path in [aws_cache, ssh_cache] {
        let ctx = adapter.make_context(
            &cred_path,
            &cred_path,
            FileIdentity {
                device_id: 1,
                inode: 401,
            },
            EntryType::File,
            false,
            None,
            false,
            false,
            false,
        );

        let classified = catalogue.classify(&ctx);
        assert_eq!(
            classified.safety_class,
            SafetyClass::Protected,
            "Credential file '{}' named like a cache must be strictly Protected",
            cred_path.display()
        );

        // Assert unrepresentable as plan item
        let mut builder = PlanBuilder::new("plan-cred", "session-cred");
        let admitted = builder.add_classified_finding(
            "cred-item",
            &classified,
            FileIdentity {
                device_id: 1,
                inode: 401,
            },
        );
        assert!(
            matches!(
                admitted,
                Err(PlanConstructionError::ProtectedItemRejected { .. })
            ),
            "Credential file named like a cache must be rejected from plan"
        );
    }
}

/// 5. A mount boundary crossed mid-rule
#[test]
fn test_mount_boundary_crossed_mid_rule() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("mount-boundary");

    let cache_dir = adapter.caches.join("Homebrew");
    std::fs::create_dir_all(&cache_dir).expect("create brew cache");

    // Traversal crosses mount boundary mid-rule (e.g. into an external volume or separate device)
    let ctx = adapter.make_context(
        &cache_dir,
        &cache_dir,
        FileIdentity {
            device_id: 999, // Different device id
            inode: 501,
        },
        EntryType::Directory,
        false,
        None,
        true, // crosses_mount_boundary = true
        false,
        false,
    );

    let classified = catalogue.classify(&ctx);
    assert_ne!(
        classified.safety_class,
        SafetyClass::Rebuildable,
        "A path crossing a mount boundary must never be classified Rebuildable"
    );
    assert_eq!(
        classified.safety_class,
        SafetyClass::Review,
        "A path crossing a mount boundary mid-rule must be downgraded to Review"
    );
}

/// 6. A durable model store that must not be classed Rebuildable
#[test]
fn test_durable_model_store_must_not_be_classed_rebuildable() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("durable-models");

    let ollama_models = adapter.fixture.create_dir("user_home/.ollama/models");
    let user_models = adapter.fixture.create_dir("user_home/models");
    let hf_hub = adapter
        .fixture
        .create_dir("user_home/.cache/huggingface/hub");

    for model_path in [ollama_models, user_models, hf_hub] {
        let ctx = adapter.make_context(
            &model_path,
            &model_path,
            FileIdentity {
                device_id: 1,
                inode: 601,
            },
            EntryType::Directory,
            false,
            None,
            false,
            false,
            false,
        );

        let classified = catalogue.classify(&ctx);
        assert_ne!(
            classified.safety_class,
            SafetyClass::Rebuildable,
            "Durable model store '{}' must NOT be classed Rebuildable",
            model_path.display()
        );
        assert_eq!(
            classified.safety_class,
            SafetyClass::Review,
            "Durable model store '{}' must be classed Review",
            model_path.display()
        );
        assert!(
            !classified.safety_class.is_default_selected(),
            "Durable model store must not be selected by default"
        );
    }
}

// ---------------------------------------------------------------------------
// Rule Catalogue Snapshot Test
// ---------------------------------------------------------------------------

#[test]
fn test_rule_catalogue_snapshot() {
    let catalogue = RuleCatalogue::new();
    let current_snapshot = catalogue.snapshot_json();

    if std::env::var("UPDATE_SNAPSHOT").is_ok() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/classify/rule_catalogue_snapshot.json");
        std::fs::write(&path, &current_snapshot).expect("failed to update snapshot");
        return;
    }

    let committed_snapshot = include_str!("rule_catalogue_snapshot.json");
    // Compare with line endings normalised. The generated JSON always uses LF;
    // a checkout that rewrites the committed file to CRLF would otherwise fail
    // this test on that platform alone, which says nothing about the catalogue.
    // `.gitattributes` pins the file to LF as well — this is the second lock.
    assert_eq!(
        current_snapshot.replace("\r\n", "\n").trim(),
        committed_snapshot.replace("\r\n", "\n").trim(),
        "Rule catalogue snapshot differs from committed snapshot. Any change must be deliberate."
    );
}

// ---------------------------------------------------------------------------
// Rule Version Revalidation Check
// ---------------------------------------------------------------------------

#[test]
fn test_rule_version_change_triggers_revalidation_failure() {
    let catalogue = RuleCatalogue::new();
    let adapter = ClassificationTestAdapter::new("rule-version-reval");

    let file_path = adapter
        .fixture
        .create_file("user_home/.cargo/registry/cache/test.crate", b"data");
    let meta = adapter.read_entry_metadata(&file_path).expect("read meta");

    let plan_item = PlanItem {
        item_id: "plan-v1".to_string(),
        original_path: file_path.clone(),
        canonical_path: file_path.canonicalize().unwrap_or(file_path.clone()),
        identity: meta.identity,
        size_bytes: meta.apparent_size,
        class: PlannableClass::Rebuildable,
        rule_id: "rule.rebuildable.cargo_target".to_string(),
        rule_version: 999, // Rule version mismatch (plan was reviewed under v999, catalogue has v1)
        evidence: Evidence {
            rule_id: "rule.rebuildable.cargo_target".to_string(),
            rule_version: 999,
            owning_tool: "Cargo".to_string(),
            matched_reason: "Matches Cargo cache".to_string(),
            regenerator: Some("cargo build".to_string()),
            last_activity_ms: None,
            recoverability: Recoverability::RebuildableByTool {
                command: "cargo build".to_string(),
            },
            confidence: Confidence::Definite,
        },
    };

    let reval = revalidate_plan_item(&adapter, &catalogue, &plan_item);
    assert!(
        matches!(
            reval,
            Err(RevalidationFailure::RuleVersionChanged {
                plan_version: 999,
                current_version: 1,
                ..
            })
        ),
        "Rule version mismatch must fail pre-execution revalidation"
    );
}

// ---------------------------------------------------------------------------
// Compile-time static marker test
// ---------------------------------------------------------------------------

#[test]
fn test_statically_classified_finding_markers() {
    assert_eq!(
        RebuildableMarker::plannable_class(),
        PlannableClass::Rebuildable
    );
    assert_eq!(RebuildableMarker::safety_class(), SafetyClass::Rebuildable);
    assert_eq!(ReviewMarker::plannable_class(), PlannableClass::Review);
    assert_eq!(ReviewMarker::safety_class(), SafetyClass::Review);

    // Note: ProtectedMarker does NOT implement PlannableSafetyMarker.
    // Attempting to invoke PlanBuilder::add_static_finding with ProtectedMarker
    // is a compile-time error.
}
