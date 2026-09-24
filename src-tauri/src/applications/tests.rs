use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;

use crate::classify::class::SafetyClass;
use crate::platform::tests::TestAdapter;
use crate::platform::{ApplicationDataRoot, ApplicationDataRootKind, PlatformAdapter};
use crate::scan::fixtures::DisposableFixtureTree;

use super::{
    ApplicationMatchEvidence, MatchReason, MatchStrength, RemovalState, discover_inventory,
};

fn create_application_bundle(
    fixture: &DisposableFixtureTree,
    relative_path: &str,
    name: &str,
    bundle_identifier: &str,
    version: &str,
    developer_name: &str,
) -> PathBuf {
    let bundle = fixture.create_dir(relative_path);
    fixture.create_file(
        &format!("{relative_path}/Contents/Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{bundle_identifier}</string>
<key>CFBundleName</key><string>{name}</string>
<key>CFBundleShortVersionString</key><string>{version}</string>
<key>CFBundleDeveloperName</key><string>{developer_name}</string>
</dict></plist>"#
        )
        .as_bytes(),
    );
    fixture.create_file(
        &format!("{relative_path}/Contents/MacOS/{name}"),
        b"fixture executable",
    );
    fixture.create_file(
        &format!("{relative_path}/Contents/Resources/app-data.bin"),
        &[7_u8; 4097],
    );
    bundle
}

fn data_root(
    fixture: &DisposableFixtureTree,
    relative_path: &str,
    kind: ApplicationDataRootKind,
) -> ApplicationDataRoot {
    ApplicationDataRoot {
        path: fixture.create_dir(relative_path),
        kind,
    }
}

fn application_adapter(
    fixture: &DisposableFixtureTree,
    bundles: Vec<PathBuf>,
    roots: Vec<ApplicationDataRoot>,
) -> TestAdapter {
    TestAdapter::default()
        .with_application_bundles(bundles)
        .with_application_data_roots(roots)
        .with_home(fixture.create_dir("home"))
}

fn related_item<'a>(
    entry: &'a super::ApplicationEntry,
    path: &std::path::Path,
) -> &'a super::RelatedFile {
    entry
        .related_files
        .iter()
        .find(|item| item.path == path)
        .unwrap_or_else(|| panic!("missing related item {}", path.display()))
}

#[cfg(unix)]
#[test]
fn measured_footprint_matches_finders_allocated_footprint_for_a_real_fixture_bundle() {
    let fixture = DisposableFixtureTree::new("application-footprint");
    let bundle = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.2.3",
        "Acme",
    );

    let metadata = crate::platform::MacOsAdapter
        .application_metadata(&bundle)
        .expect("inspect fixture bundle");

    let adapter = crate::platform::MacOsAdapter;
    let expected = fixture
        .path("Applications/Alpha.app")
        .canonicalize()
        .unwrap();
    let mut stack = vec![expected.clone()];
    let mut identities = std::collections::HashSet::new();
    let mut allocated = 0_u64;
    while let Some(path) = stack.pop() {
        let metadata = adapter.read_entry_metadata(&path).unwrap();
        if !identities.insert(metadata.identity) {
            continue;
        }
        allocated += metadata.allocated_size;
        if metadata.entry_type == crate::platform::EntryType::Directory {
            for entry in std::fs::read_dir(path).unwrap().flatten() {
                stack.push(entry.path());
            }
        }
    }

    assert!(metadata.measured_footprint_bytes > 0);
    assert_eq!(metadata.measured_footprint_bytes, allocated);
    assert_eq!(
        metadata.bundle_identifier.as_deref(),
        Some("com.acme.alpha")
    );
    assert_eq!(metadata.name, "Alpha");
    assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
}

#[test]
fn related_match_missing_reason_or_strength_is_rejected() {
    let evidence = ApplicationMatchEvidence {
        reason: MatchReason::ExactBundleIdentifier,
        strength: MatchStrength::Strong,
        explanation: "Matched by bundle identifier".into(),
    };
    let mut value = serde_json::to_value(&evidence).unwrap();

    let mut missing_strength = value.clone();
    missing_strength.as_object_mut().unwrap().remove("strength");
    assert!(serde_json::from_value::<ApplicationMatchEvidence>(missing_strength).is_err());

    value.as_object_mut().unwrap().remove("reason");
    assert!(serde_json::from_value::<ApplicationMatchEvidence>(value).is_err());

    assert_eq!(
        serde_json::from_value::<ApplicationMatchEvidence>(json!({
            "reason": "exactBundleIdentifier",
            "strength": "strong",
            "explanation": "Matched by bundle identifier"
        }))
        .unwrap(),
        evidence
    );
}

#[test]
fn two_applications_sharing_a_developer_directory_have_no_selected_cross_association() {
    let fixture = DisposableFixtureTree::new("application-shared-developer");
    let alpha = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.0",
        "Acme",
    );
    let beta = create_application_bundle(
        &fixture,
        "Applications/Beta.app",
        "Beta",
        "com.acme.beta",
        "1.0",
        "Acme",
    );
    fixture.create_file("home/Library/Application Support/Acme/profile.json", b"{}");
    let adapter = application_adapter(
        &fixture,
        vec![alpha, beta],
        vec![data_root(
            &fixture,
            "home/Library/Application Support",
            ApplicationDataRootKind::ApplicationSupport,
        )],
    );

    let inventory = discover_inventory(&adapter).unwrap();
    let alpha_entry = inventory
        .applications
        .iter()
        .find(|entry| entry.bundle_id.as_deref() == Some("com.acme.alpha"))
        .unwrap();
    let beta_entry = inventory
        .applications
        .iter()
        .find(|entry| entry.bundle_id.as_deref() == Some("com.acme.beta"))
        .unwrap();
    let developer_path = fixture.path("home/Library/Application Support/Acme");
    let alpha_match = related_item(alpha_entry, &developer_path);
    let beta_match = related_item(beta_entry, &developer_path);

    assert_eq!(alpha_match.evidence.reason, MatchReason::DeveloperDirectory);
    assert_eq!(alpha_match.evidence.strength, MatchStrength::Weak);
    assert_eq!(beta_match.evidence, alpha_match.evidence);
    assert_eq!(alpha_match.safety_class, SafetyClass::Review);
    assert_eq!(beta_match.safety_class, SafetyClass::Review);
    assert!(!alpha_match.selected_by_default);
    assert!(!beta_match.selected_by_default);
}

#[test]
fn licence_file_and_auth_token_near_application_support_are_protected() {
    let fixture = DisposableFixtureTree::new("application-credentials");
    let bundle = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.0",
        "Acme",
    );
    fixture.create_file(
        "home/Library/Application Support/com.acme.alpha/Alpha.licence",
        b"licence",
    );
    fixture.create_file(
        "home/Library/Application Support/com.acme.alpha/Alpha.auth-token",
        b"token",
    );
    let adapter = application_adapter(
        &fixture,
        vec![bundle],
        vec![data_root(
            &fixture,
            "home/Library/Application Support",
            ApplicationDataRootKind::ApplicationSupport,
        )],
    );

    let inventory = discover_inventory(&adapter).unwrap();
    let entry = &inventory.applications[0];
    let support = fixture.path("home/Library/Application Support/com.acme.alpha");
    let licence = fixture.path("home/Library/Application Support/com.acme.alpha/Alpha.licence");
    let token = fixture.path("home/Library/Application Support/com.acme.alpha/Alpha.auth-token");

    assert_eq!(
        related_item(entry, &support).safety_class,
        SafetyClass::Protected
    );
    assert_eq!(
        related_item(entry, &licence).safety_class,
        SafetyClass::Protected
    );
    assert_eq!(
        related_item(entry, &token).safety_class,
        SafetyClass::Protected
    );
    assert!(!related_item(entry, &licence).selected_by_default);
    assert!(!related_item(entry, &token).selected_by_default);
}

#[test]
fn shared_framework_used_by_two_installed_applications_is_not_offered_for_removal() {
    let fixture = DisposableFixtureTree::new("application-shared-framework");
    let alpha = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.0",
        "Acme",
    );
    let beta = create_application_bundle(
        &fixture,
        "Applications/Beta.app",
        "Beta",
        "com.acme.beta",
        "1.0",
        "Acme",
    );
    for bundle in [&alpha, &beta] {
        fixture.create_file(
            &format!(
                "{}/Contents/Frameworks/Shared.framework/Shared",
                bundle.to_string_lossy()
            ),
            b"shared component",
        );
    }
    fixture.create_file(
        "home/Library/Frameworks/Shared.framework/Shared",
        b"shared component",
    );
    let adapter = application_adapter(
        &fixture,
        vec![alpha, beta],
        vec![data_root(
            &fixture,
            "home/Library/Frameworks",
            ApplicationDataRootKind::Frameworks,
        )],
    );

    let inventory = discover_inventory(&adapter).unwrap();
    let shared_path = fixture.path("home/Library/Frameworks/Shared.framework");
    assert!(inventory.applications.iter().all(|entry| {
        related_item(entry, &shared_path).evidence.reason == MatchReason::SharedComponent
            && !related_item(entry, &shared_path).offered_for_removal
            && !related_item(entry, &shared_path).selected_by_default
    }));
}

#[test]
fn orphan_leftovers_with_absent_owning_application_are_discoverable() {
    let fixture = DisposableFixtureTree::new("application-orphan");
    let leftover = fixture.create_file(
        "home/Library/Caches/com.gone.editor/cache.bin",
        &[0_u8; 2048],
    );
    let adapter = application_adapter(
        &fixture,
        Vec::new(),
        vec![data_root(
            &fixture,
            "home/Library/Caches",
            ApplicationDataRootKind::Caches,
        )],
    );

    let inventory = discover_inventory(&adapter).unwrap();
    assert!(inventory.applications.is_empty());
    let orphan = inventory
        .orphans
        .iter()
        .find(|item| item.path == leftover)
        .unwrap();

    assert_eq!(orphan.evidence.reason, MatchReason::ExactBundleIdentifier);
    assert_eq!(orphan.evidence.strength, MatchStrength::Strong);
    assert_eq!(orphan.safety_class, SafetyClass::Review);
    assert!(!orphan.selected_by_default);
}

#[test]
fn running_application_cannot_be_removed_and_explains_that_it_must_quit() {
    let fixture = DisposableFixtureTree::new("application-running");
    let alpha = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.0",
        "Acme",
    );
    let adapter = application_adapter(
        &fixture,
        vec![alpha.clone()],
        vec![data_root(
            &fixture,
            "home/Library/Caches",
            ApplicationDataRootKind::Caches,
        )],
    )
    .with_running_applications(vec![alpha]);
    let adapter = Arc::new(adapter);

    let inventory = discover_inventory(adapter.as_ref()).unwrap();
    assert_eq!(
        inventory.applications[0].removal_state,
        RemovalState::QuitRequired
    );
    assert!(!inventory.applications[0].can_remove);
    assert!(!inventory.applications[0].bundle_selected_by_default);
    assert!(
        inventory.applications[0]
            .removal_explanation
            .contains("Quit")
    );
}

#[test]
fn user_documents_are_never_discovered_as_application_state() {
    let fixture = DisposableFixtureTree::new("application-documents");
    let bundle = create_application_bundle(
        &fixture,
        "Applications/Alpha.app",
        "Alpha",
        "com.acme.alpha",
        "1.0",
        "Acme",
    );
    fixture.create_file("home/Documents/Alpha Project/source.txt", b"user work");
    let adapter = application_adapter(
        &fixture,
        vec![bundle],
        vec![data_root(
            &fixture,
            "home/Library/Application Support",
            ApplicationDataRootKind::ApplicationSupport,
        )],
    );

    let inventory = discover_inventory(&adapter).unwrap();
    assert!(
        inventory.applications[0]
            .related_files
            .iter()
            .all(|item| !item.path.to_string_lossy().contains("Documents"))
    );
}
