use std::collections::HashMap;
use std::path::Path;

use crate::boundary::cancellation::CancellationToken;
use crate::duplicates::detect::{CandidateFile, detect_storage_sharing, is_excluded_path};
use crate::duplicates::hashing::{compute_full_hash_streaming, compute_quick_fingerprint};
use crate::duplicates::model::{DuplicateGroup, DuplicateItem, RetainedRule, StorageSharingKind};
use crate::platform::{EntryType, PlatformAdapter};

/// Execution options for duplicate detection.
#[derive(Debug, Clone)]
pub struct DuplicateDetectionOptions {
    pub retained_rule: RetainedRule,
    pub cancellation_token: CancellationToken,
}

impl Default for DuplicateDetectionOptions {
    fn default() -> Self {
        Self {
            retained_rule: RetainedRule::OldestByModified,
            cancellation_token: CancellationToken::new(),
        }
    }
}

/// Metrics recorded during staged duplicate detection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateDetectionMetrics {
    pub total_candidates_inspected: usize,
    pub excluded_protected_or_git_count: usize,
    pub files_entering_size_grouping: usize,
    pub size_groups_count: usize,
    pub files_eliminated_by_unique_size: usize,
    pub fingerprints_computed: usize,
    pub files_eliminated_by_fingerprint: usize,
    pub full_hashes_computed: usize,
    pub files_eliminated_by_full_hash: usize,
    pub confirmed_duplicate_groups: usize,
    pub confirmed_duplicate_files: usize,
}

/// Sorts candidates according to the stated retained rule so the retained original is chosen deterministically.
pub fn sort_for_retained(candidates: &mut [CandidateFile], rule: RetainedRule) {
    match rule {
        RetainedRule::OldestByModified => {
            candidates.sort_by(|a, b| {
                let m_a = a.modified_ms.unwrap_or(u64::MAX);
                let m_b = b.modified_ms.unwrap_or(u64::MAX);
                m_a.cmp(&m_b)
                    .then_with(|| a.path.as_os_str().len().cmp(&b.path.as_os_str().len()))
                    .then_with(|| a.canonical_path.cmp(&b.canonical_path))
            });
        }
        RetainedRule::NewestByModified => {
            candidates.sort_by(|a, b| {
                let m_a = a.modified_ms.unwrap_or(0);
                let m_b = b.modified_ms.unwrap_or(0);
                m_b.cmp(&m_a)
                    .then_with(|| a.path.as_os_str().len().cmp(&b.path.as_os_str().len()))
                    .then_with(|| a.canonical_path.cmp(&b.canonical_path))
            });
        }
        RetainedRule::ShortestPath => {
            candidates.sort_by(|a, b| {
                a.path
                    .as_os_str()
                    .len()
                    .cmp(&b.path.as_os_str().len())
                    .then_with(|| {
                        let m_a = a.modified_ms.unwrap_or(u64::MAX);
                        let m_b = b.modified_ms.unwrap_or(u64::MAX);
                        m_a.cmp(&m_b)
                    })
                    .then_with(|| a.canonical_path.cmp(&b.canonical_path))
            });
        }
    }
}

/// Converts a 32-byte digest into a lowercase hex string.
pub fn to_hex(bytes: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

/// Staged duplicate detection engine:
/// 1. Group candidates by file size.
/// 2. Quick fingerprint of head and tail bytes.
/// 3. Full streaming SHA-256 hash only on survivors of both prior stages.
pub fn detect_duplicates<P: AsRef<Path>>(
    file_paths: &[P],
    adapter: &dyn PlatformAdapter,
    options: &DuplicateDetectionOptions,
    metrics: &mut DuplicateDetectionMetrics,
) -> Result<Vec<DuplicateGroup>, std::io::Error> {
    metrics.total_candidates_inspected = file_paths.len();

    // --- Filter step ---
    let mut initial_candidates = Vec::new();
    for p in file_paths {
        if options.cancellation_token.is_cancelled() {
            return Ok(Vec::new());
        }

        let path = p.as_ref();
        let meta = match adapter.read_entry_metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Exact content duplicate detection operates strictly on regular files
        if meta.entry_type != EntryType::File {
            continue;
        }

        // 0-byte files have no content to duplicate and free 0 bytes
        if meta.apparent_size == 0 {
            continue;
        }

        // Exclusion check: protected roots, cloud sync, and git working trees are barred
        if is_excluded_path(path, adapter) {
            metrics.excluded_protected_or_git_count += 1;
            continue;
        }

        let resolved = match adapter.canonicalize_and_normalize(path) {
            Ok(r) => r,
            Err(_) => continue,
        };

        initial_candidates.push(CandidateFile {
            path: path.to_path_buf(),
            canonical_path: resolved.canonical,
            normalized_path: resolved.normalized,
            identity: meta.identity,
            size_bytes: meta.apparent_size,
            modified_ms: meta.modified_ms,
        });
    }

    // --- Stage 1: Group by file size ---
    metrics.files_entering_size_grouping = initial_candidates.len();
    let mut size_map: HashMap<u64, Vec<CandidateFile>> = HashMap::new();
    for cand in initial_candidates {
        size_map.entry(cand.size_bytes).or_default().push(cand);
    }

    let mut size_survivor_groups: Vec<Vec<CandidateFile>> = Vec::new();
    for (_size, group) in size_map {
        if group.len() >= 2 {
            size_survivor_groups.push(group);
        } else {
            metrics.files_eliminated_by_unique_size += group.len();
        }
    }
    metrics.size_groups_count = size_survivor_groups.len();

    if options.cancellation_token.is_cancelled() {
        return Ok(Vec::new());
    }

    // --- Stage 2: Quick fingerprint (head + tail bytes) ---
    let mut fingerprint_survivor_groups: Vec<Vec<CandidateFile>> = Vec::new();
    for group in size_survivor_groups {
        if options.cancellation_token.is_cancelled() {
            return Ok(Vec::new());
        }

        let mut fp_map: HashMap<[u8; 32], Vec<CandidateFile>> = HashMap::new();
        for cand in group {
            if options.cancellation_token.is_cancelled() {
                return Ok(Vec::new());
            }

            match compute_quick_fingerprint(&cand.path, cand.size_bytes) {
                Ok(fp) => {
                    metrics.fingerprints_computed += 1;
                    fp_map.entry(fp).or_default().push(cand);
                }
                Err(_) => {
                    // Unreadable file vanishes or lacks permission; drop silently from group
                    continue;
                }
            }
        }

        for (_fp, fp_group) in fp_map {
            if fp_group.len() >= 2 {
                fingerprint_survivor_groups.push(fp_group);
            } else {
                metrics.files_eliminated_by_fingerprint += fp_group.len();
            }
        }
    }

    // --- Stage 3: Full collision-resistant hash (streaming SHA-256) on survivors only ---
    let mut confirmed_groups: Vec<DuplicateGroup> = Vec::new();
    let mut group_counter = 1;

    for group in fingerprint_survivor_groups {
        // If cancelled mid-stream, preserve and return all groups already confirmed
        if options.cancellation_token.is_cancelled() {
            break;
        }

        let mut hash_map: HashMap<[u8; 32], Vec<CandidateFile>> = HashMap::new();
        for cand in group {
            if options.cancellation_token.is_cancelled() {
                break;
            }

            let file = match std::fs::File::open(&cand.path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            match compute_full_hash_streaming(file, |_| {}) {
                Ok(hash) => {
                    metrics.full_hashes_computed += 1;
                    hash_map.entry(hash).or_default().push(cand);
                }
                Err(_) => continue,
            }
        }

        for (hash, mut confirmed_files) in hash_map {
            if confirmed_files.len() >= 2 {
                // Deterministically choose the retained original
                sort_for_retained(&mut confirmed_files, options.retained_rule);

                let retained_cand = confirmed_files.remove(0);
                let retained_item = DuplicateItem::new(
                    format!("dup-{}-retained", group_counter),
                    retained_cand.path.clone(),
                    retained_cand.canonical_path.clone(),
                    retained_cand.size_bytes,
                    retained_cand.modified_ms,
                    retained_cand.identity.inode,
                    retained_cand.identity.device_id,
                    true,
                    StorageSharingKind::Independent,
                );

                let mut candidates = Vec::new();
                let mut shared_storage_items = Vec::new();

                for (idx, copy) in confirmed_files.into_iter().enumerate() {
                    let sharing_kind = detect_storage_sharing(&copy, &retained_cand, adapter);
                    let item = DuplicateItem::new(
                        format!("dup-{}-copy-{}", group_counter, idx + 1),
                        copy.path,
                        copy.canonical_path,
                        copy.size_bytes,
                        copy.modified_ms,
                        copy.identity.inode,
                        copy.identity.device_id,
                        false,
                        sharing_kind,
                    );

                    match sharing_kind {
                        StorageSharingKind::Independent => candidates.push(item),
                        StorageSharingKind::Hardlink | StorageSharingKind::ApfsClone => {
                            shared_storage_items.push(item)
                        }
                    }
                }

                let hash_hex = to_hex(&hash);
                let group = DuplicateGroup::new(
                    format!("group-{}", group_counter),
                    retained_cand.size_bytes,
                    hash_hex,
                    retained_item,
                    candidates,
                    shared_storage_items,
                );

                group
                    .assert_invariants()
                    .expect("constructed duplicate group must satisfy safety invariants");

                metrics.confirmed_duplicate_groups += 1;
                metrics.confirmed_duplicate_files += group.total_item_count();
                confirmed_groups.push(group);
                group_counter += 1;
            } else {
                metrics.files_eliminated_by_full_hash += confirmed_files.len();
            }
        }
    }

    Ok(confirmed_groups)
}
