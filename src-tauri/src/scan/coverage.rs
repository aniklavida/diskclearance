use std::path::PathBuf;

/// Reason why a filesystem scope could not be completely traversed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "camelCase")]
pub enum SkipReason {
    PermissionDenied(String),
    MountBoundary { root_device: u64, path_device: u64 },
    Vanished,
    IoError(String),
}

/// Record of an unvisited or partially unvisited scope with explicit reason.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedScope {
    pub path: PathBuf,
    pub reason: SkipReason,
}

/// First-class accounting of coverage, tracked sizes, and skipped areas.
///
/// If any scopes were skipped due to permission denial or mount boundaries,
/// the estimate represents a lower bound ("at least this much") rather than
/// a false exact total.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageSummary {
    pub total_apparent_bytes: u64,
    pub total_allocated_bytes: u64,
    pub items_scanned: u64,
    pub files_count: u64,
    pub directories_count: u64,
    pub symlinks_count: u64,
    pub skipped_permissions_count: u64,
    pub skipped_mount_boundaries_count: u64,
    pub skipped_vanished_count: u64,
    pub skipped_scopes: Vec<SkippedScope>,
    pub is_complete: bool,
}

impl CoverageSummary {
    /// True only if all requested roots were walked without permission blocks,
    /// mount exclusions, or mid-walk cancellations.
    pub fn is_exact(&self) -> bool {
        self.is_complete
            && self.skipped_permissions_count == 0
            && self.skipped_mount_boundaries_count == 0
    }
}
