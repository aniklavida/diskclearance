use std::path::{Path, PathBuf};

use crate::classify::matcher::MatchContext;
use crate::classify::protected::evaluate_protected_roots;
use crate::duplicates::model::StorageSharingKind;
use crate::platform::{EntryType, FileIdentity, PlatformAdapter};

/// Candidate file record before grouping.
#[derive(Debug, Clone)]
pub struct CandidateFile {
    pub path: PathBuf,
    pub canonical_path: PathBuf,
    pub normalized_path: PathBuf,
    pub identity: FileIdentity,
    pub size_bytes: u64,
    pub modified_ms: Option<u64>,
}

/// Checks if any component of a path is `.git`.
pub fn is_git_internal(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}

/// Checks if a path resides inside a Git repository working tree.
pub fn is_inside_git_repo(path: &Path) -> bool {
    if is_git_internal(path) {
        return true;
    }
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return true;
        }
        current = dir.parent();
    }
    false
}

/// Determines whether a path is excluded from duplicate detection according to:
/// - Protected roots (macOS system paths, credentials, crypto material, cloud sync folders, etc.)
/// - Git internal state (.git)
/// - Git working tree files
pub fn is_excluded_path(path: &Path, adapter: &dyn PlatformAdapter) -> bool {
    if is_git_internal(path) || is_inside_git_repo(path) {
        return true;
    }

    let resolved = match adapter.canonicalize_and_normalize(path) {
        Ok(r) => r,
        Err(_) => return true,
    };

    let identity = match adapter.file_identity(path) {
        Ok(id) => id,
        Err(_) => return true,
    };

    let metadata = match adapter.read_entry_metadata(path) {
        Ok(m) => m,
        Err(_) => return true,
    };

    let home = adapter.home_directory().ok().map(|d| d.path);
    let app_support = adapter.application_support_directory().ok().map(|d| d.path);
    let caches = adapter.caches_directory().ok().map(|d| d.path);

    let match_ctx = MatchContext {
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
        home_dir: home.as_deref(),
        app_support_dir: app_support.as_deref(),
        caches_dir: caches.as_deref(),
        crosses_mount_boundary: false,
        inside_git_repo: true, // We already checked above; but set to ensure any git matcher triggers
        is_git_internal: false,
    };

    evaluate_protected_roots(&match_ctx).is_some()
}

/// Identifies whether a candidate copy shares physical storage with a retained original or reference file.
///
/// 1. Hardlink: Inode and device IDs match (`nlink > 1`).
/// 2. APFS clone: Inodes differ, but extent offset query reveals shared underlying disk blocks.
/// 3. Independent: Files occupy separate storage allocations.
///
/// Extent-offset lookup is platform-specific and lives entirely behind
/// `PlatformAdapter::file_extent_offset` in `platform.rs` — the one file
/// carrying operating-system conditionals — so this module has none.
pub fn detect_storage_sharing(
    candidate: &CandidateFile,
    reference: &CandidateFile,
    adapter: &dyn PlatformAdapter,
) -> StorageSharingKind {
    // Stage 1: Hardlink detection (same inode on the same device)
    if candidate.identity.device_id == reference.identity.device_id
        && candidate.identity.inode == reference.identity.inode
    {
        return StorageSharingKind::Hardlink;
    }

    // Stage 2: APFS copy-on-write clone detection
    if candidate.identity.device_id == reference.identity.device_id && candidate.size_bytes > 0 {
        if let (Some(cand_ext), Some(ref_ext)) = (
            adapter.file_extent_offset(&candidate.path, candidate.size_bytes),
            adapter.file_extent_offset(&reference.path, reference.size_bytes),
        ) {
            if cand_ext == ref_ext {
                return StorageSharingKind::ApfsClone;
            }
        }
    }

    StorageSharingKind::Independent
}
