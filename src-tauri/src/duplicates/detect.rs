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

/// Queries the physical device extent offset on macOS using the F_LOG2PHYS_EXT fcntl.
///
/// Returns `Some(device_offset)` if the file is backed by physical storage blocks,
/// or `None` if unsupported, zero-sized, or an error occurs.
#[cfg(target_os = "macos")]
pub fn get_file_extent_offset(path: &Path, size_bytes: u64) -> Option<i64> {
    use std::os::unix::io::AsRawFd;

    if size_bytes == 0 {
        return None;
    }

    let file = std::fs::File::open(path).ok()?;
    let fd = file.as_raw_fd();

    #[repr(C, packed(4))]
    struct Log2Phys {
        l2p_flags: u32,
        l2p_contigbytes: i64,
        l2p_devoffset: i64,
    }

    let mut l2p = Log2Phys {
        l2p_flags: 0,
        l2p_contigbytes: size_bytes as i64,
        l2p_devoffset: 0,
    };

    const F_LOG2PHYS_EXT: std::ffi::c_int = 65;
    unsafe extern "C" {
        fn fcntl(fd: std::ffi::c_int, cmd: std::ffi::c_int, ...) -> std::ffi::c_int;
    }

    let ret = unsafe { fcntl(fd, F_LOG2PHYS_EXT, &mut l2p) };
    if ret == 0 && l2p.l2p_devoffset > 0 {
        Some(l2p.l2p_devoffset)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_file_extent_offset(_path: &Path, _size_bytes: u64) -> Option<i64> {
    None
}

/// Identifies whether a candidate copy shares physical storage with a retained original or reference file.
///
/// 1. Hardlink: Inode and device IDs match (`nlink > 1`).
/// 2. APFS clone: Inodes differ, but extent offset query reveals shared underlying disk blocks.
/// 3. Independent: Files occupy separate storage allocations.
pub fn detect_storage_sharing(
    candidate: &CandidateFile,
    reference: &CandidateFile,
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
            get_file_extent_offset(&candidate.path, candidate.size_bytes),
            get_file_extent_offset(&reference.path, reference.size_bytes),
        ) {
            if cand_ext == ref_ext {
                return StorageSharingKind::ApfsClone;
            }
        }
    }

    StorageSharingKind::Independent
}
