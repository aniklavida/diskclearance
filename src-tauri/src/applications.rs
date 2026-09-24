use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::classify::class::SafetyClass;
use crate::classify::matcher::MatchContext;
use crate::classify::protected::evaluate_protected_roots;
use crate::platform::{
    ApplicationDataRoot, ApplicationDataRootKind, ApplicationMetadata, EntryType, FileIdentity,
    PlatformAdapter, PlatformError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MatchReason {
    ExactBundleIdentifier,
    DeveloperDirectory,
    SimilarName,
    SensitiveCredential,
    SharedComponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MatchStrength {
    Strong,
    Weak,
    Guess,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationMatchEvidence {
    pub reason: MatchReason,
    pub strength: MatchStrength,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RelatedLocationKind {
    ApplicationSupport,
    Caches,
    Preferences,
    Containers,
    GroupContainers,
    SavedApplicationState,
    Logs,
    LaunchAgents,
    Frameworks,
}

impl From<ApplicationDataRootKind> for RelatedLocationKind {
    fn from(value: ApplicationDataRootKind) -> Self {
        match value {
            ApplicationDataRootKind::ApplicationSupport => Self::ApplicationSupport,
            ApplicationDataRootKind::Caches => Self::Caches,
            ApplicationDataRootKind::Preferences => Self::Preferences,
            ApplicationDataRootKind::Containers => Self::Containers,
            ApplicationDataRootKind::GroupContainers => Self::GroupContainers,
            ApplicationDataRootKind::SavedApplicationState => Self::SavedApplicationState,
            ApplicationDataRootKind::Logs => Self::Logs,
            ApplicationDataRootKind::LaunchAgents => Self::LaunchAgents,
            ApplicationDataRootKind::Frameworks => Self::Frameworks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RelatedFile {
    pub path: PathBuf,
    pub name: String,
    pub location: RelatedLocationKind,
    pub measured_footprint_bytes: u64,
    pub safety_class: SafetyClass,
    pub selected_by_default: bool,
    pub offered_for_removal: bool,
    pub evidence: ApplicationMatchEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RemovalState {
    Ready,
    QuitRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationEntry {
    pub bundle_id: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub install_path: PathBuf,
    pub bundle_footprint_bytes: u64,
    pub related_files_footprint_bytes: u64,
    pub combined_footprint_bytes: u64,
    pub is_running: bool,
    pub can_remove: bool,
    pub bundle_selected_by_default: bool,
    pub removal_state: RemovalState,
    pub removal_explanation: String,
    pub related_files: Vec<RelatedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInventory {
    pub applications: Vec<ApplicationEntry>,
    pub orphans: Vec<RelatedFile>,
}

#[derive(Debug, Clone)]
struct Candidate {
    path: PathBuf,
    root: ApplicationDataRoot,
}

#[derive(Debug, Clone)]
struct Association {
    candidate: Candidate,
    reason: MatchReason,
    strength: MatchStrength,
    explanation: String,
}

pub fn discover_inventory(
    adapter: &dyn PlatformAdapter,
) -> Result<ApplicationInventory, PlatformError> {
    let bundle_paths = adapter.installed_application_bundles()?;
    let roots = adapter.application_data_roots()?;
    let candidates = collect_candidates(adapter, &roots)?;
    let mut applications = Vec::new();
    let mut component_usage: HashMap<String, usize> = HashMap::new();

    for path in &bundle_paths {
        let metadata = adapter.application_metadata(path)?;
        if metadata.is_system_application {
            continue;
        }
        let shared_components = adapter.application_shared_components(path)?;
        for component in &shared_components {
            *component_usage.entry(component.clone()).or_default() += 1;
        }
        applications.push((metadata, shared_components));
    }

    let mut entries = Vec::new();
    let mut associated_paths = HashSet::new();
    for (metadata, shared_components) in &applications {
        let associations = associations_for_application(
            metadata,
            &candidates,
            &component_usage,
            shared_components,
        );
        let related_files = related_files_for_application(adapter, &associations)?;
        associated_paths.extend(
            associations
                .iter()
                .map(|association| association.candidate.path.clone()),
        );
        let related_footprint = related_files
            .iter()
            .map(|related| related.measured_footprint_bytes)
            .sum();
        let is_running = adapter.is_application_running(&metadata.install_location);
        let removal_state = if is_running {
            RemovalState::QuitRequired
        } else {
            RemovalState::Ready
        };
        let removal_explanation = if is_running {
            "Quit this application before removing its bundle or related files."
        } else {
            "Application is not running and can enter a reviewed removal plan."
        }
        .to_string();
        entries.push(ApplicationEntry {
            bundle_id: metadata.bundle_identifier.clone(),
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            install_path: metadata.install_location.clone(),
            bundle_footprint_bytes: metadata.measured_footprint_bytes,
            related_files_footprint_bytes: related_footprint,
            combined_footprint_bytes: metadata
                .measured_footprint_bytes
                .saturating_add(related_footprint),
            is_running,
            can_remove: !is_running,
            bundle_selected_by_default: !is_running,
            removal_state,
            removal_explanation,
            related_files,
        });
    }

    let mut orphans = Vec::new();
    for candidate in &candidates {
        if associated_paths.contains(&candidate.path)
            || !is_exact_identity_candidate(&candidate.path, &candidate.root, None)
        {
            continue;
        }
        if let Some(mut related) = build_related_file(adapter, candidate)? {
            related.safety_class = SafetyClass::Review;
            related.selected_by_default = false;
            orphans.push(related);
        }
    }

    Ok(ApplicationInventory {
        applications: entries,
        orphans,
    })
}

fn collect_candidates(
    adapter: &dyn PlatformAdapter,
    roots: &[ApplicationDataRoot],
) -> Result<Vec<Candidate>, PlatformError> {
    let home = adapter.home_directory().ok().map(|path| path.path);
    let application_support = adapter
        .application_support_directory()
        .ok()
        .map(|path| path.path);
    let caches = adapter.caches_directory().ok().map(|path| path.path);
    let mut candidates = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root.path) else {
            continue;
        };
        for entry in entries.flatten() {
            collect_candidate_tree(
                adapter,
                root,
                entry.path(),
                0,
                home.as_deref(),
                application_support.as_deref(),
                caches.as_deref(),
                &mut candidates,
            );
        }
    }
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    candidates.dedup_by(|left, right| left.path == right.path);
    Ok(candidates)
}

#[allow(clippy::too_many_arguments)]
fn collect_candidate_tree(
    adapter: &dyn PlatformAdapter,
    root: &ApplicationDataRoot,
    path: PathBuf,
    depth: usize,
    home: Option<&Path>,
    application_support: Option<&Path>,
    caches: Option<&Path>,
    candidates: &mut Vec<Candidate>,
) {
    if candidates.len() >= 10_000 || depth > 4 {
        return;
    }
    let Ok(metadata) = adapter.read_entry_metadata(&path) else {
        return;
    };
    if metadata.entry_type == EntryType::Symlink
        || !adapter.path_is_owned_by_current_user(&path)
        || is_policy_protected(
            adapter,
            &path,
            home,
            application_support,
            caches,
            metadata.identity,
        )
    {
        return;
    }
    let is_directory = metadata.entry_type == EntryType::Directory;
    candidates.push(Candidate {
        path: path.clone(),
        root: root.clone(),
    });
    if is_directory && let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            collect_candidate_tree(
                adapter,
                root,
                entry.path(),
                depth + 1,
                home,
                application_support,
                caches,
                candidates,
            );
        }
    }
}

fn associations_for_application(
    metadata: &ApplicationMetadata,
    candidates: &[Candidate],
    component_usage: &HashMap<String, usize>,
    shared_components: &[String],
) -> Vec<Association> {
    let Some(bundle_identifier) = metadata.bundle_identifier.as_deref() else {
        return Vec::new();
    };
    let developer_name = developer_name(metadata);
    let mut associations = Vec::new();
    for candidate in candidates {
        if let Some(association) =
            shared_component_association(candidate, component_usage, shared_components)
        {
            associations.push(association);
            continue;
        }
        if is_exact_identity_candidate(&candidate.path, &candidate.root, Some(bundle_identifier)) {
            associations.push(Association {
                candidate: candidate.clone(),
                reason: MatchReason::ExactBundleIdentifier,
                strength: MatchStrength::Strong,
                explanation: format!("Matched by bundle identifier {bundle_identifier}"),
            });
            continue;
        }
        if has_ancestor_name(&candidate.path, &candidate.root.path, &developer_name) {
            associations.push(Association {
                candidate: candidate.clone(),
                reason: MatchReason::DeveloperDirectory,
                strength: MatchStrength::Weak,
                explanation: format!(
                    "Path is inside a directory named after developer {}",
                    metadata
                        .developer_name
                        .as_deref()
                        .unwrap_or("derived from the bundle identifier")
                ),
            });
            continue;
        }
        if names_are_similar(&candidate_name(&candidate.path), &metadata.name) {
            associations.push(Association {
                candidate: candidate.clone(),
                reason: MatchReason::SimilarName,
                strength: MatchStrength::Guess,
                explanation: "Name looks similar; this is a guess, not an identity match"
                    .to_string(),
            });
        }
    }
    associations
}

fn shared_component_association(
    candidate: &Candidate,
    component_usage: &HashMap<String, usize>,
    shared_components: &[String],
) -> Option<Association> {
    if candidate.root.kind != ApplicationDataRootKind::Frameworks {
        return None;
    }
    let name = candidate.path.file_name()?.to_string_lossy().to_string();
    if !shared_components.iter().any(|component| component == &name) {
        return None;
    }
    let using_applications = component_usage.get(&name).copied().unwrap_or_default();
    if using_applications < 2 {
        return None;
    }
    Some(Association {
        candidate: candidate.clone(),
        reason: MatchReason::SharedComponent,
        strength: MatchStrength::Excluded,
        explanation: format!(
            "{name} is used by {using_applications} installed applications and is excluded from removal"
        ),
    })
}

fn related_files_for_application(
    adapter: &dyn PlatformAdapter,
    associations: &[Association],
) -> Result<Vec<RelatedFile>, PlatformError> {
    let sensitive_paths = associations
        .iter()
        .filter(|association| is_sensitive_path(&association.candidate.path))
        .map(|association| association.candidate.path.clone())
        .collect::<HashSet<_>>();
    let mut files = Vec::new();
    for association in associations {
        if !is_top_level_path(
            &association.candidate.path,
            &association.candidate.root.path,
        ) && !is_sensitive_path(&association.candidate.path)
            && association.reason != MatchReason::SharedComponent
        {
            continue;
        }
        if let Some(mut related) = build_related_file(adapter, &association.candidate)? {
            related.evidence.reason = association.reason;
            related.evidence.strength = association.strength;
            related.evidence.explanation = association.explanation.clone();
            if association.reason == MatchReason::SharedComponent {
                related.offered_for_removal = false;
            }
            let protects_descendant = sensitive_paths.iter().any(|sensitive| {
                sensitive != &related.path && sensitive.starts_with(&association.candidate.path)
            });
            let is_sensitive = sensitive_paths.contains(&related.path);
            if is_sensitive || protects_descendant {
                related.evidence.reason = MatchReason::SensitiveCredential;
                related.evidence.explanation =
                    "Contains or is a licence, authentication token, or account credential"
                        .to_string();
                related.safety_class = SafetyClass::Protected;
            }
            if association.reason == MatchReason::SharedComponent {
                related.safety_class = SafetyClass::Protected;
            }
            related.selected_by_default = related.safety_class == SafetyClass::Rebuildable
                && related.evidence.strength == MatchStrength::Strong;
            files.push(related);
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.dedup_by(|left, right| left.path == right.path);
    Ok(files)
}

fn build_related_file(
    adapter: &dyn PlatformAdapter,
    candidate: &Candidate,
) -> Result<Option<RelatedFile>, PlatformError> {
    if !candidate.path.exists() && !candidate.path.is_symlink() {
        return Ok(None);
    }
    let footprint = match measure_footprint(adapter, &candidate.path) {
        Ok(value) => value,
        Err(PlatformError::NotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(Some(RelatedFile {
        path: candidate.path.clone(),
        name: candidate
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        location: candidate.root.kind.into(),
        measured_footprint_bytes: footprint,
        safety_class: SafetyClass::Review,
        selected_by_default: false,
        offered_for_removal: true,
        evidence: ApplicationMatchEvidence {
            reason: MatchReason::ExactBundleIdentifier,
            strength: MatchStrength::Strong,
            explanation: "Matched by bundle identifier".to_string(),
        },
    }))
}

fn measure_footprint(adapter: &dyn PlatformAdapter, path: &Path) -> Result<u64, PlatformError> {
    let mut pending = vec![path.to_path_buf()];
    let mut identities = HashSet::new();
    let mut total = 0_u64;
    while let Some(path) = pending.pop() {
        let metadata = adapter.read_entry_metadata(&path)?;
        if !identities.insert(metadata.identity) {
            continue;
        }
        total = total.saturating_add(metadata.allocated_size);
        if metadata.entry_type != EntryType::Directory {
            continue;
        }
        for entry in std::fs::read_dir(&path)
            .map_err(|error| PlatformError::Io(error.to_string()))?
            .flatten()
        {
            pending.push(entry.path());
        }
    }
    Ok(total)
}

fn developer_name(metadata: &ApplicationMetadata) -> String {
    if let Some(developer) = metadata.developer_name.as_deref() {
        return developer.to_string();
    }
    metadata
        .bundle_identifier
        .as_deref()
        .and_then(|identifier| identifier.split('.').rev().nth(1))
        .unwrap_or_default()
        .to_string()
}

fn is_exact_identity_candidate(
    path: &Path,
    root: &ApplicationDataRoot,
    bundle_identifier: Option<&str>,
) -> bool {
    let root_path = root.path.as_path();
    if let Some(bundle_identifier) = bundle_identifier {
        return path.ancestors().any(|ancestor| {
            ancestor != root_path
                && ancestor.starts_with(root_path)
                && identity_names(ancestor, root).contains(bundle_identifier)
        });
    }
    path.ancestors().any(|ancestor| {
        ancestor != root_path
            && ancestor.starts_with(root_path)
            && identity_names(ancestor, root)
                .iter()
                .any(|identity| is_bundle_identifier(identity))
    })
}

fn is_bundle_identifier(value: &str) -> bool {
    let mut components = value.split('.');
    components.clone().count() >= 2
        && components.all(|component| {
            !component.is_empty()
                && component
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
}

fn identity_names(path: &Path, root: &ApplicationDataRoot) -> HashSet<String> {
    let Ok(relative) = path.strip_prefix(&root.path) else {
        return HashSet::new();
    };
    let Some(name) = relative
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return HashSet::new();
    };
    let normalized = name
        .strip_suffix(".plist")
        .or_else(|| name.strip_suffix(".savedState"))
        .unwrap_or(&name);
    [
        normalized.to_string(),
        format!("group.{normalized}"),
        normalized
            .strip_prefix("group.")
            .unwrap_or(normalized)
            .to_string(),
    ]
    .into_iter()
    .collect()
}

fn has_ancestor_name(path: &Path, root: &Path, name: &str) -> bool {
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return false;
    }
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| {
            relative
                .ancestors()
                .find(|ancestor| ancestor.parent().is_some())
        })
        .and_then(Path::file_name)
        .map(|candidate| normalize_name(&candidate.to_string_lossy()) == normalized)
        .unwrap_or(false)
}

fn names_are_similar(candidate: &str, application_name: &str) -> bool {
    let candidate = normalize_name(candidate);
    let application = normalize_name(application_name);
    candidate.len() >= 4
        && application.len() >= 4
        && (candidate == application
            || candidate.contains(&application)
            || application.contains(&candidate))
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_top_level_path(path: &Path, root: &Path) -> bool {
    path.parent() == Some(root)
}

fn candidate_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn is_sensitive_path(path: &Path) -> bool {
    let name = candidate_name(path).to_ascii_lowercase();
    [
        "licence",
        "license",
        "activation",
        "entitlement",
        "auth-token",
        "authtoken",
        "credential",
        "password",
        "secret",
        "token",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

fn is_policy_protected(
    adapter: &dyn PlatformAdapter,
    path: &Path,
    home: Option<&Path>,
    application_support: Option<&Path>,
    caches: Option<&Path>,
    identity: FileIdentity,
) -> bool {
    let resolved = match adapter.canonicalize_and_normalize(path) {
        Ok(resolved) => resolved,
        Err(_) => return true,
    };
    let Ok(metadata) = adapter.read_entry_metadata(path) else {
        return true;
    };
    evaluate_protected_roots(&MatchContext {
        original_path: path,
        canonical_path: &resolved.canonical,
        normalized_path: &resolved.normalized,
        entry_type: metadata.entry_type,
        identity,
        apparent_size: metadata.apparent_size,
        allocated_size: metadata.allocated_size,
        modified_ms: metadata.modified_ms,
        is_symlink: metadata.entry_type == EntryType::Symlink,
        symlink_target_canonical: None,
        home_dir: home,
        app_support_dir: application_support,
        caches_dir: caches,
        crosses_mount_boundary: false,
        inside_git_repo: false,
        is_git_internal: false,
    })
    .is_some()
}

#[cfg(test)]
mod tests;
