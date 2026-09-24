use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::classify::catalogue::{ClassifiedFinding, RuleCatalogue};
use crate::classify::class::{
    PlannableClass, PlannableSafetyMarker, ProtectedClassRejection, SafetyClass,
};
use crate::classify::evidence::{Evidence, EvidenceValidationError};
use crate::classify::matcher::MatchContext;
use crate::platform::{EntryType, FileIdentity, PlatformAdapter, PlatformError};

/// Typed errors when attempting to admit a finding into a deletion plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "camelCase")]
pub enum PlanConstructionError {
    /// Protected roots and assets are barred from entering a plan.
    ProtectedItemRejected {
        path: PathBuf,
        rule_id: String,
        reason: String,
    },
    /// An evidence record was missing or empty.
    InvalidEvidence(EvidenceValidationError),
    /// The safety class could not be converted to a plannable class.
    NonPlannableClass(ProtectedClassRejection),
}

impl std::fmt::Display for PlanConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProtectedItemRejected {
                path,
                rule_id,
                reason,
            } => {
                write!(
                    f,
                    "Protected item at '{}' cannot enter a deletion plan (rule: {}, reason: {})",
                    path.display(),
                    rule_id,
                    reason
                )
            }
            Self::InvalidEvidence(err) => write!(f, "Invalid evidence: {err}"),
            Self::NonPlannableClass(err) => write!(f, "Non-plannable class: {err}"),
        }
    }
}

impl std::error::Error for PlanConstructionError {}

/// An immutable item admitted into a review plan.
///
/// Notice the `class` field is strictly typed as `PlannableClass`, which only
/// possesses `Rebuildable` and `Review` variants. It is impossible to construct
/// or represent a `PlanItem` holding `SafetyClass::Protected`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub item_id: String,
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub identity: FileIdentity,
    pub size_bytes: u64,
    pub class: PlannableClass,
    pub rule_id: String,
    pub rule_version: u32,
    pub evidence: Evidence,
}

/// A statically typed finding for compile-time verified plan admission.
///
/// Constructing a `StaticallyClassifiedFinding<ProtectedMarker>` will fail to compile
/// when passed to `PlanBuilder::add_static_finding` because `ProtectedMarker` does not
/// implement `PlannableSafetyMarker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticallyClassifiedFinding<M> {
    pub path: PathBuf,
    pub canonical_path: PathBuf,
    pub identity: FileIdentity,
    pub size_bytes: u64,
    pub marker: M,
    pub evidence: Evidence,
}

impl<M> StaticallyClassifiedFinding<M> {
    pub fn new(
        marker: M,
        path: PathBuf,
        canonical_path: PathBuf,
        identity: FileIdentity,
        size_bytes: u64,
        evidence: Evidence,
    ) -> Self {
        Self {
            path,
            canonical_path,
            identity,
            size_bytes,
            marker,
            evidence,
        }
    }
}

/// Builder for constructing immutable deletion plans with strict safety gating.
#[derive(Debug, Clone)]
pub struct PlanBuilder {
    pub plan_id: String,
    pub session_id: String,
    items: Vec<PlanItem>,
}

impl PlanBuilder {
    pub fn new(plan_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            session_id: session_id.into(),
            items: Vec::new(),
        }
    }

    /// Admits a finding with compile-time proof that its class is plannable.
    ///
    /// If called with `StaticallyClassifiedFinding<ProtectedMarker>`, this method
    /// fails compilation with trait bound `ProtectedMarker: PlannableSafetyMarker` not satisfied.
    pub fn add_static_finding<M: PlannableSafetyMarker>(
        &mut self,
        item_id: impl Into<String>,
        finding: StaticallyClassifiedFinding<M>,
    ) -> Result<&PlanItem, PlanConstructionError> {
        finding
            .evidence
            .validate()
            .map_err(PlanConstructionError::InvalidEvidence)?;

        let plan_item = PlanItem {
            item_id: item_id.into(),
            original_path: finding.path,
            canonical_path: finding.canonical_path,
            identity: finding.identity,
            size_bytes: finding.size_bytes,
            class: M::plannable_class(),
            rule_id: finding.evidence.rule_id.clone(),
            rule_version: finding.evidence.rule_version,
            evidence: finding.evidence,
        };

        self.items.push(plan_item);
        Ok(self.items.last().unwrap())
    }

    /// Dynamically admits a classified scan finding into the plan.
    ///
    /// Rejects any finding whose safety class is `Protected` with a typed error.
    pub fn add_classified_finding(
        &mut self,
        item_id: impl Into<String>,
        finding: &ClassifiedFinding,
        identity: FileIdentity,
    ) -> Result<&PlanItem, PlanConstructionError> {
        // Enforce evidence validity
        finding
            .evidence
            .validate()
            .map_err(PlanConstructionError::InvalidEvidence)?;

        // Enforce non-protected class
        let plannable_class = match finding.safety_class {
            SafetyClass::Protected => {
                return Err(PlanConstructionError::ProtectedItemRejected {
                    path: PathBuf::from(&finding.path),
                    rule_id: finding.evidence.rule_id.clone(),
                    reason: finding.evidence.matched_reason.clone(),
                });
            }
            SafetyClass::Rebuildable => PlannableClass::Rebuildable,
            SafetyClass::Review => PlannableClass::Review,
        };

        let plan_item = PlanItem {
            item_id: item_id.into(),
            original_path: PathBuf::from(&finding.path),
            canonical_path: PathBuf::from(&finding.canonical_path),
            identity,
            size_bytes: finding.size_bytes,
            class: plannable_class,
            rule_id: finding.evidence.rule_id.clone(),
            rule_version: finding.evidence.rule_version,
            evidence: finding.evidence.clone(),
        };

        self.items.push(plan_item);
        Ok(self.items.last().unwrap())
    }

    pub fn items(&self) -> &[PlanItem] {
        &self.items
    }
}

/// Reason why pre-execution revalidation failed for a plan item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "camelCase")]
pub enum RevalidationFailure {
    PathVanished(PathBuf),
    IdentityChanged {
        path: PathBuf,
        expected: FileIdentity,
        actual: FileIdentity,
    },
    CanonicalPathChanged {
        path: PathBuf,
        expected: PathBuf,
        actual: PathBuf,
    },
    ItemBecameProtected {
        path: PathBuf,
        rule_id: String,
        reason: String,
    },
    RuleVersionChanged {
        rule_id: String,
        plan_version: u32,
        current_version: u32,
    },
    PlatformError(String),
}

impl std::fmt::Display for RevalidationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathVanished(path) => {
                write!(f, "path vanished before execution: {}", path.display())
            }
            Self::IdentityChanged {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "file identity changed for '{}' (expected dev:{} ino:{}, got dev:{} ino:{})",
                    path.display(),
                    expected.device_id,
                    expected.inode,
                    actual.device_id,
                    actual.inode
                )
            }
            Self::CanonicalPathChanged {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "canonical path shifted for '{}' (expected '{}', got '{}')",
                    path.display(),
                    expected.display(),
                    actual.display()
                )
            }
            Self::ItemBecameProtected {
                path,
                rule_id,
                reason,
            } => {
                write!(
                    f,
                    "target at '{}' became protected under rule {} ({})",
                    path.display(),
                    rule_id,
                    reason
                )
            }
            Self::RuleVersionChanged {
                rule_id,
                plan_version,
                current_version,
            } => {
                write!(
                    f,
                    "rule '{rule_id}' version mismatch (plan had v{plan_version}, catalogue has v{current_version})"
                )
            }
            Self::PlatformError(err) => write!(f, "platform error during revalidation: {err}"),
        }
    }
}

impl std::error::Error for RevalidationFailure {}

/// Revalidates an individual plan item immediately before execution.
///
/// Checks that:
/// 1. The target path still exists and is accessible.
/// 2. The canonical path has not changed.
/// 3. The exact filesystem identity (device and inode) has not changed.
/// 4. The target has not become protected under the current rule set.
/// 5. The rule version under which the item was admitted matches the current catalogue.
pub fn revalidate_plan_item(
    adapter: &dyn PlatformAdapter,
    catalogue: &RuleCatalogue,
    item: &PlanItem,
) -> Result<(), RevalidationFailure> {
    // 1. Check entry metadata & identity
    let metadata = match adapter.read_entry_metadata(&item.original_path) {
        Ok(meta) => meta,
        Err(PlatformError::NotFound(_)) => {
            return Err(RevalidationFailure::PathVanished(
                item.original_path.clone(),
            ));
        }
        Err(err) => return Err(RevalidationFailure::PlatformError(err.to_string())),
    };

    if metadata.identity != item.identity {
        return Err(RevalidationFailure::IdentityChanged {
            path: item.original_path.clone(),
            expected: item.identity,
            actual: metadata.identity,
        });
    }

    // 2. Canonical path check
    let resolved = adapter
        .canonicalize_and_normalize(&item.original_path)
        .map_err(|e| RevalidationFailure::PlatformError(e.to_string()))?;

    if resolved.canonical != item.canonical_path {
        return Err(RevalidationFailure::CanonicalPathChanged {
            path: item.original_path.clone(),
            expected: item.canonical_path.clone(),
            actual: resolved.canonical,
        });
    }

    // 3. Re-classify with current catalogue to verify it didn't become protected
    let home = adapter.home_directory().ok().map(|d| d.path);
    let app_support = adapter.application_support_directory().ok().map(|d| d.path);
    let caches = adapter.caches_directory().ok().map(|d| d.path);

    let match_ctx = MatchContext {
        original_path: &item.original_path,
        canonical_path: &resolved.canonical,
        normalized_path: &resolved.normalized,
        entry_type: metadata.entry_type,
        identity: metadata.identity,
        apparent_size: metadata.apparent_size,
        allocated_size: metadata.allocated_size,
        modified_ms: metadata.modified_ms,
        is_symlink: metadata.entry_type == EntryType::Symlink,
        symlink_target_canonical: None,
        home_dir: home.as_deref(),
        app_support_dir: app_support.as_deref(),
        caches_dir: caches.as_deref(),
        crosses_mount_boundary: false,
        inside_git_repo: false,
        is_git_internal: false,
    };

    let reclassified = catalogue.classify(&match_ctx);
    if reclassified.safety_class.is_protected() {
        return Err(RevalidationFailure::ItemBecameProtected {
            path: item.original_path.clone(),
            rule_id: reclassified.evidence.rule_id,
            reason: reclassified.evidence.matched_reason,
        });
    }

    // 4. Verify rule version matches current rule catalogue version
    if let Some(desc) = catalogue
        .all_descriptors()
        .iter()
        .find(|r| r.id == item.rule_id)
        && desc.version != item.rule_version
    {
        return Err(RevalidationFailure::RuleVersionChanged {
            rule_id: item.rule_id.clone(),
            plan_version: item.rule_version,
            current_version: desc.version,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::class::RebuildableMarker;
    use crate::classify::evidence::{Confidence, Recoverability};

    #[test]
    fn test_plan_builder_admits_rebuildable_and_review() {
        let mut builder = PlanBuilder::new("plan-1", "session-1");
        let evidence = Evidence {
            rule_id: "rule.rebuildable.cargo_target".to_string(),
            rule_version: 1,
            owning_tool: "Cargo".to_string(),
            matched_reason: "Matches Cargo target directory".to_string(),
            regenerator: Some("cargo build".to_string()),
            last_activity_ms: Some(100),
            regeneration_cost: Some("minutes to hours".to_string()),
            recoverability: Recoverability::RebuildableByTool {
                command: "cargo build".to_string(),
            },
            confidence: Confidence::Definite,
        };

        let finding = ClassifiedFinding {
            path: "/tmp/project/target".to_string(),
            canonical_path: "/tmp/project/target".to_string(),
            size_bytes: 5000,
            safety_class: SafetyClass::Rebuildable,
            evidence,
        };

        let res = builder.add_classified_finding(
            "item-1",
            &finding,
            FileIdentity {
                device_id: 1,
                inode: 10,
            },
        );
        assert!(res.is_ok());
        assert_eq!(builder.items().len(), 1);
        assert_eq!(builder.items()[0].class, PlannableClass::Rebuildable);
    }

    #[test]
    fn test_plan_builder_rejects_protected_finding() {
        let mut builder = PlanBuilder::new("plan-1", "session-1");
        let evidence = Evidence {
            rule_id: "protected.system.root".to_string(),
            rule_version: 1,
            owning_tool: "macOS System".to_string(),
            matched_reason: "Root system volume".to_string(),
            regenerator: None,
            last_activity_ms: None,
            regeneration_cost: None,
            recoverability: Recoverability::Irrecoverable,
            confidence: Confidence::Definite,
        };

        let finding = ClassifiedFinding {
            path: "/System".to_string(),
            canonical_path: "/System".to_string(),
            size_bytes: 100000,
            safety_class: SafetyClass::Protected,
            evidence,
        };

        let res = builder.add_classified_finding(
            "item-1",
            &finding,
            FileIdentity {
                device_id: 1,
                inode: 1,
            },
        );
        assert!(matches!(
            res,
            Err(PlanConstructionError::ProtectedItemRejected { .. })
        ));
        assert_eq!(builder.items().len(), 0);
    }

    #[test]
    fn test_static_plan_builder_with_plannable_marker() {
        let mut builder = PlanBuilder::new("plan-1", "session-1");
        let evidence = Evidence {
            rule_id: "rule.rebuildable.cargo_target".to_string(),
            rule_version: 1,
            owning_tool: "Cargo".to_string(),
            matched_reason: "Cargo build".to_string(),
            regenerator: Some("cargo build".to_string()),
            last_activity_ms: None,
            regeneration_cost: Some("minutes to hours".to_string()),
            recoverability: Recoverability::RebuildableByTool {
                command: "cargo build".to_string(),
            },
            confidence: Confidence::Definite,
        };

        let static_finding = StaticallyClassifiedFinding::new(
            RebuildableMarker,
            PathBuf::from("/tmp/target"),
            PathBuf::from("/tmp/target"),
            FileIdentity {
                device_id: 1,
                inode: 2,
            },
            1024,
            evidence,
        );

        let res = builder.add_static_finding("item-1", static_finding);
        assert!(res.is_ok());
    }
}
