use std::path::PathBuf;

use crate::platform::{EntryType, FileIdentity};

/// Verified record of a traversed filesystem entry.
///
/// Apparent size and allocated size are tracked separately: APFS sparse files,
/// block rounding, and directory entries diverge sufficiently that conflating them
/// produces inaccurate reclaim estimates.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedEntry {
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub normalized_path: PathBuf,
    pub identity: FileIdentity,
    pub entry_type: EntryType,
    pub apparent_size: u64,
    pub allocated_size: u64,
    pub modified_ms: Option<u64>,
}
