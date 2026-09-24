use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

use crate::classify::class::SafetyClass;

/// Identifies whether two duplicate paths share underlying physical storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum StorageSharingKind {
    /// Independent copy on disk consuming separate storage blocks.
    Independent,
    /// Shares filesystem inode with another copy (hardlink); freeing it reclaims 0 bytes.
    Hardlink,
    /// Shares physical extent blocks with another copy (APFS copy-on-write clone); freeing it reclaims 0 bytes.
    ApfsClone,
}

/// Deterministic rule used to select the retained original within a duplicate group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RetainedRule {
    /// Retain the oldest copy by mtime; ties broken by shortest path, then lexicographical order.
    OldestByModified,
    /// Retain the newest copy by mtime; ties broken by shortest path, then lexicographical order.
    NewestByModified,
    /// Retain the copy with the shortest canonical path; ties broken by oldest mtime.
    ShortestPath,
}

impl Default for RetainedRule {
    fn default() -> Self {
        Self::OldestByModified
    }
}

/// Individual item within a duplicate group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateItem {
    pub id: String,
    pub path: String,
    pub canonical_path: String,
    pub size_bytes: u64,
    pub modified_ms: Option<u64>,
    pub inode: u64,
    pub device_id: u64,
    pub is_retained: bool,
    /// Selection status. Invariant: always false by default.
    pub is_selected: bool,
    pub sharing_kind: StorageSharingKind,
}

impl DuplicateItem {
    pub fn new(
        id: String,
        path: PathBuf,
        canonical_path: PathBuf,
        size_bytes: u64,
        modified_ms: Option<u64>,
        inode: u64,
        device_id: u64,
        is_retained: bool,
        sharing_kind: StorageSharingKind,
    ) -> Self {
        Self {
            id,
            path: path.to_string_lossy().to_string(),
            canonical_path: canonical_path.to_string_lossy().to_string(),
            size_bytes,
            modified_ms,
            inode,
            device_id,
            is_retained,
            is_selected: false,
            sharing_kind,
        }
    }
}

/// Errors returned during duplicate review plan construction and selection validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "details", rename_all = "camelCase")]
pub enum DuplicatePlanError {
    RetainedItemProtected { group_id: String, path: PathBuf },
    EntireGroupSelected { group_id: String },
    StorageSharingItemNotDeletable { group_id: String, path: PathBuf },
    ItemNotFound { group_id: String, item_id: String },
    NoItemsSelected { group_id: String },
}

impl std::fmt::Display for DuplicatePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RetainedItemProtected { group_id, path } => {
                write!(
                    f,
                    "Retained original '{}' in group '{}' is structurally protected from deletion",
                    path.display(),
                    group_id
                )
            }
            Self::EntireGroupSelected { group_id } => {
                write!(
                    f,
                    "Cannot select all copies in group '{}'; at least the retained original must remain",
                    group_id
                )
            }
            Self::StorageSharingItemNotDeletable { group_id, path } => {
                write!(
                    f,
                    "Item '{}' in group '{}' shares storage with another copy and contributes zero reclaimable bytes",
                    path.display(),
                    group_id
                )
            }
            Self::ItemNotFound { group_id, item_id } => {
                write!(
                    f,
                    "Item '{}' was not found in group '{}'",
                    item_id, group_id
                )
            }
            Self::NoItemsSelected { group_id } => {
                write!(f, "No items were selected in group '{}'", group_id)
            }
        }
    }
}

impl std::error::Error for DuplicatePlanError {}

/// Group of byte-identical regular files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub group_id: String,
    pub size_bytes: u64,
    pub content_hash: String,
    /// Duplicates always classify as Review to require explicit user decision.
    pub safety_class: SafetyClass,
    /// The structurally designated retained copy.
    pub retained: DuplicateItem,
    /// Independent candidate copies that can be chosen for deletion.
    pub candidates: Vec<DuplicateItem>,
    /// Copies that share storage (hardlinks/APFS clones) contributing zero reclaimable bytes.
    pub shared_storage_items: Vec<DuplicateItem>,
    /// Net reclaimable bytes (only counts independent candidates, not shared storage or retained).
    pub reclaimable_bytes: u64,
    /// Total apparent bytes across all copies in the group.
    pub total_group_bytes: u64,
}

impl DuplicateGroup {
    pub fn new(
        group_id: String,
        size_bytes: u64,
        content_hash: String,
        mut retained: DuplicateItem,
        mut candidates: Vec<DuplicateItem>,
        mut shared_storage_items: Vec<DuplicateItem>,
    ) -> Self {
        retained.is_retained = true;
        retained.is_selected = false;

        for c in &mut candidates {
            c.is_retained = false;
            c.is_selected = false;
        }

        for s in &mut shared_storage_items {
            s.is_retained = false;
            s.is_selected = false;
        }

        let reclaimable_bytes = candidates.len() as u64 * size_bytes;
        let total_group_bytes =
            (1 + candidates.len() + shared_storage_items.len()) as u64 * size_bytes;

        Self {
            group_id,
            size_bytes,
            content_hash,
            safety_class: SafetyClass::Review,
            retained,
            candidates,
            shared_storage_items,
            reclaimable_bytes,
            total_group_bytes,
        }
    }

    /// Total count of all copies in the group.
    pub fn total_item_count(&self) -> usize {
        1 + self.candidates.len() + self.shared_storage_items.len()
    }

    /// Asserts safety invariants on the group data structure.
    pub fn assert_invariants(&self) -> Result<(), &'static str> {
        if !self.retained.is_retained {
            return Err("retained item must have is_retained = true");
        }
        if self.retained.is_selected {
            return Err("retained item must not be preselected");
        }
        if self.safety_class != SafetyClass::Review {
            return Err("duplicate group safety class must be Review");
        }
        for c in &self.candidates {
            if c.is_retained {
                return Err("candidate cannot be marked retained");
            }
            if c.is_selected {
                return Err("candidate must not be preselected");
            }
            if c.sharing_kind != StorageSharingKind::Independent {
                return Err("candidate must have Independent storage sharing kind");
            }
        }
        for s in &self.shared_storage_items {
            if s.is_retained {
                return Err("shared storage item cannot be marked retained");
            }
            if s.is_selected {
                return Err("shared storage item must not be preselected");
            }
            if s.sharing_kind == StorageSharingKind::Independent {
                return Err("shared storage item must not have Independent storage sharing kind");
            }
        }
        if self.reclaimable_bytes != (self.candidates.len() as u64 * self.size_bytes) {
            return Err("reclaimable bytes must strictly equal independent candidate count * size");
        }
        Ok(())
    }

    /// Verifies that no item in this group is preselected.
    pub fn no_preselection(&self) -> bool {
        !self.retained.is_selected
            && self.candidates.iter().all(|c| !c.is_selected)
            && self.shared_storage_items.iter().all(|s| !s.is_selected)
    }

    /// Validates an explicit user selection against safety invariants:
    /// - Retained copy can NEVER be selected for deletion.
    /// - Items sharing storage cannot be offered for deletion.
    /// - A group can never have all copies selected.
    pub fn validate_deletion_selection(
        &self,
        selected_ids: &[String],
    ) -> Result<Vec<DuplicateItem>, DuplicatePlanError> {
        if selected_ids.is_empty() {
            return Err(DuplicatePlanError::NoItemsSelected {
                group_id: self.group_id.clone(),
            });
        }

        // Invariant: The retained original cannot be added to a deletion plan
        if selected_ids.contains(&self.retained.id) {
            return Err(DuplicatePlanError::RetainedItemProtected {
                group_id: self.group_id.clone(),
                path: PathBuf::from(&self.retained.path),
            });
        }

        // Invariant: Entire group cannot be selected
        if selected_ids.len() >= self.total_item_count() {
            return Err(DuplicatePlanError::EntireGroupSelected {
                group_id: self.group_id.clone(),
            });
        }

        let mut validated = Vec::new();
        for id in selected_ids {
            // Check if it belongs to shared storage
            if let Some(shared) = self.shared_storage_items.iter().find(|s| &s.id == id) {
                return Err(DuplicatePlanError::StorageSharingItemNotDeletable {
                    group_id: self.group_id.clone(),
                    path: PathBuf::from(&shared.path),
                });
            }

            // Find in candidates
            if let Some(candidate) = self.candidates.iter().find(|c| &c.id == id) {
                validated.push(candidate.clone());
            } else {
                return Err(DuplicatePlanError::ItemNotFound {
                    group_id: self.group_id.clone(),
                    item_id: id.clone(),
                });
            }
        }

        Ok(validated)
    }
}
