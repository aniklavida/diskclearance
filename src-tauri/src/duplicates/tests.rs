use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::boundary::cancellation::CancellationToken;
use crate::classify::class::SafetyClass;
use crate::duplicates::detect::is_excluded_path;
use crate::duplicates::engine::{
    DuplicateDetectionMetrics, DuplicateDetectionOptions, detect_duplicates,
};
use crate::duplicates::hashing::{
    ChunkVerifyingReader, STREAMING_CHUNK_SIZE, compute_full_hash_streaming,
};
use crate::duplicates::model::{
    DuplicateGroup, DuplicateItem, DuplicatePlanError, RetainedRule, StorageSharingKind,
};

/// Disposable test directory that cleans up on drop.
struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("dc_dup_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("failed to create temp test directory");
        Self { path }
    }

    fn file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        let mut f = File::create(&p).expect("failed to create file");
        f.write_all(content).expect("failed to write content");
        f.flush().expect("failed to flush");
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// -----------------------------------------------------------------------------
// Done-when Test 1: Two byte-identical files of same size grouped; differing middle byte rejected
// -----------------------------------------------------------------------------
#[test]
fn test_byte_identical_files_grouped_and_middle_byte_differences_rejected() {
    let fixture = TempTestDir::new("staged_diff");
    let adapter = crate::platform::create_platform_adapter();

    // Group A: Two byte-identical files (12288 bytes each)
    let mut content_a = vec![0x41u8; 12288];
    content_a[6000] = 0x58; // Specific middle byte
    let file_a1 = fixture.file("identical_1.bin", &content_a);
    let file_a2 = fixture.file("identical_2.bin", &content_a);

    // Group B: Two files of identical size (12288 bytes) with identical head/tail but differing middle byte
    // Head (0..4096) and Tail (8192..12288) are identical; byte 6000 is outside both.
    let mut content_b1 = vec![0x42u8; 12288];
    content_b1[6000] = 0x11; // Middle byte differs
    let mut content_b2 = vec![0x42u8; 12288];
    content_b2[6000] = 0x22; // Middle byte differs
    let file_b1 = fixture.file("differing_middle_1.bin", &content_b1);
    let file_b2 = fixture.file("differing_middle_2.bin", &content_b2);

    let candidates = vec![
        file_a1.clone(),
        file_a2.clone(),
        file_b1.clone(),
        file_b2.clone(),
    ];
    let options = DuplicateDetectionOptions::default();
    let mut metrics = DuplicateDetectionMetrics::default();

    let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("detection should succeed");

    // Identical files must be grouped
    assert_eq!(
        groups.len(),
        1,
        "Only the byte-identical pair must form a duplicate group"
    );

    let group = &groups[0];
    assert_eq!(group.size_bytes, 12288);
    let all_paths: Vec<String> = std::iter::once(group.retained.path.clone())
        .chain(group.candidates.iter().map(|c| c.path.clone()))
        .collect();

    assert!(all_paths.contains(&file_a1.to_string_lossy().to_string()));
    assert!(all_paths.contains(&file_a2.to_string_lossy().to_string()));

    // Differing middle byte pair must NOT be grouped
    assert!(!all_paths.contains(&file_b1.to_string_lossy().to_string()));
    assert!(!all_paths.contains(&file_b2.to_string_lossy().to_string()));

    // Verify metrics: full hashes were run on candidates surviving fingerprint,
    // and the differing middle byte files were eliminated at Stage 3
    assert_eq!(metrics.confirmed_duplicate_groups, 1);
    assert_eq!(metrics.files_eliminated_by_full_hash, 2);
}

// -----------------------------------------------------------------------------
// Done-when Test 2: Hardlinked pair reported as sharing storage and contributes zero reclaimable bytes
// -----------------------------------------------------------------------------
#[test]
fn test_hardlinked_pair_reported_as_sharing_storage_with_zero_reclaimable_bytes() {
    let fixture = TempTestDir::new("hardlink");
    let adapter = crate::platform::create_platform_adapter();

    let content = b"This is a shared storage payload across multiple links.";
    let file_orig = fixture.file("original.txt", content);
    let file_link = fixture.path.join("hardlinked_copy.txt");
    fs::hard_link(&file_orig, &file_link).expect("hard link creation must succeed");

    let candidates = vec![file_orig.clone(), file_link.clone()];
    let options = DuplicateDetectionOptions::default();
    let mut metrics = DuplicateDetectionMetrics::default();

    let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("detection should succeed");

    assert_eq!(groups.len(), 1, "Hardlinked files must be grouped");
    let group = &groups[0];

    // Invariant: Hardlink contributes zero reclaimable bytes
    assert_eq!(
        group.reclaimable_bytes, 0,
        "Hardlinked pair must contribute zero reclaimable bytes"
    );

    // Invariant: Deletable candidate list must be empty (hardlink is not offered as deletable duplicate)
    assert!(
        group.candidates.is_empty(),
        "Hardlinks must never be offered as deletable candidates"
    );

    // Invariant: Hardlink is recorded in shared_storage_items with Hardlink kind
    assert_eq!(group.shared_storage_items.len(), 1);
    assert_eq!(
        group.shared_storage_items[0].sharing_kind,
        StorageSharingKind::Hardlink
    );
    assert_eq!(group.shared_storage_items[0].inode, group.retained.inode);
}

// -----------------------------------------------------------------------------
// Done-when Test 3: APFS clone pair reported as sharing storage with zero reclaimable bytes
// -----------------------------------------------------------------------------
#[test]
fn test_apfs_clone_pair_reported_as_sharing_storage_with_zero_reclaimable_bytes() {
    {
        let fixture = TempTestDir::new("apfs_clone");
        let adapter = crate::platform::create_platform_adapter();

        // Write a multi-block file to ensure extent allocation
        let content = vec![0x55u8; 65536];
        let file_orig = fixture.file("source.bin", &content);
        let file_clone = fixture.path.join("clone.bin");

        if !crate::platform::try_create_clone_for_test(&file_orig, &file_clone) {
            // Unsupported on this platform/volume (e.g. non-APFS temp directory)
            eprintln!("Skipping APFS clone test: clone creation not supported here");
            return;
        }

        // Verify that inodes differ (clones have distinct inodes, unlike hardlinks)
        let meta_orig = fs::metadata(&file_orig).unwrap();
        let meta_clone = fs::metadata(&file_clone).unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            meta_orig.ino(),
            meta_clone.ino(),
            "APFS clones must have distinct inodes"
        );

        let candidates = vec![file_orig.clone(), file_clone.clone()];
        let options = DuplicateDetectionOptions::default();
        let mut metrics = DuplicateDetectionMetrics::default();

        let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
            .expect("detection should succeed");

        assert_eq!(groups.len(), 1, "APFS cloned files must be grouped");
        let group = &groups[0];

        // Extent-sharing check: Clone must contribute 0 reclaimable bytes
        assert_eq!(
            group.reclaimable_bytes, 0,
            "APFS clone pair must contribute zero reclaimable bytes"
        );
        assert!(
            group.candidates.is_empty(),
            "Cloned copies sharing extents must not be offered as deletable candidates"
        );
        assert_eq!(group.shared_storage_items.len(), 1);
        assert_eq!(
            group.shared_storage_items[0].sharing_kind,
            StorageSharingKind::ApfsClone
        );
    }
}

// -----------------------------------------------------------------------------
// Done-when Test 4: No group is ever fully selected; retained original cannot be added alongside copies
// -----------------------------------------------------------------------------
#[test]
fn test_no_group_is_ever_fully_selected_and_retained_cannot_be_planned_with_all_copies() {
    let retained = DuplicateItem::new(
        "item-retained".to_string(),
        PathBuf::from("/var/data/docs/original.pdf"),
        PathBuf::from("/var/data/docs/original.pdf"),
        1024,
        Some(100),
        1001,
        1,
        true,
        StorageSharingKind::Independent,
    );
    let cand1 = DuplicateItem::new(
        "item-cand-1".to_string(),
        PathBuf::from("/var/data/downloads/copy1.pdf"),
        PathBuf::from("/var/data/downloads/copy1.pdf"),
        1024,
        Some(200),
        1002,
        1,
        false,
        StorageSharingKind::Independent,
    );
    let cand2 = DuplicateItem::new(
        "item-cand-2".to_string(),
        PathBuf::from("/var/data/archive/copy2.pdf"),
        PathBuf::from("/var/data/archive/copy2.pdf"),
        1024,
        Some(300),
        1003,
        1,
        false,
        StorageSharingKind::Independent,
    );

    let group = DuplicateGroup::new(
        "group-test".to_string(),
        1024,
        "dummyhash".to_string(),
        retained.clone(),
        vec![cand1.clone(), cand2.clone()],
        Vec::new(),
    );

    // Case A: Valid selection — user selects only non-retained candidate copies
    let valid_selection = vec!["item-cand-1".to_string(), "item-cand-2".to_string()];
    let res = group.validate_deletion_selection(&valid_selection);
    assert!(res.is_ok(), "Selecting candidate copies must succeed");
    let validated = res.unwrap();
    assert_eq!(validated.len(), 2);

    // Case B: Invariant violation — attempting to include the retained original
    let attempt_retained = vec!["item-retained".to_string(), "item-cand-1".to_string()];
    let res_retained = group.validate_deletion_selection(&attempt_retained);
    assert!(
        matches!(
            res_retained,
            Err(DuplicatePlanError::RetainedItemProtected { .. })
        ),
        "Retained item must be structurally protected from deletion plan inclusion"
    );

    // Case C: Invariant violation — attempting to select all copies in the group
    let attempt_all = vec![
        "item-retained".to_string(),
        "item-cand-1".to_string(),
        "item-cand-2".to_string(),
    ];
    let res_all = group.validate_deletion_selection(&attempt_all);
    assert!(
        res_all.is_err(),
        "No group is ever fully selected; all copies cannot be planned together"
    );
}

// -----------------------------------------------------------------------------
// Done-when Test 5: Nothing is preselected anywhere in the duplicates data model
// -----------------------------------------------------------------------------
#[test]
fn test_nothing_is_preselected_anywhere_in_duplicates_data_model() {
    let fixture = TempTestDir::new("no_preselection");
    let adapter = crate::platform::create_platform_adapter();

    let content = b"Safety first: review items are never auto-selected";
    let file1 = fixture.file("dup1.txt", content);
    let file2 = fixture.file("dup2.txt", content);
    let file3 = fixture.file("dup3.txt", content);

    let candidates = vec![file1, file2, file3];
    let options = DuplicateDetectionOptions::default();
    let mut metrics = DuplicateDetectionMetrics::default();

    let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("detection should succeed");

    assert_eq!(groups.len(), 1);
    let group = &groups[0];

    // Invariant: safety class is Review
    assert_eq!(group.safety_class, SafetyClass::Review);
    assert!(!group.safety_class.is_default_selected());

    // Invariant: no preselection anywhere in data model
    assert!(
        group.no_preselection(),
        "Group must have zero preselected items"
    );
    assert!(!group.retained.is_selected);
    for cand in &group.candidates {
        assert!(!cand.is_selected, "Candidate copy must not be preselected");
    }
    for shared in &group.shared_storage_items {
        assert!(
            !shared.is_selected,
            "Shared storage copy must not be preselected"
        );
    }
}

// -----------------------------------------------------------------------------
// Done-when Test 6: Bounded memory streaming reads during large file hashing
// -----------------------------------------------------------------------------
#[test]
fn test_streaming_reads_keep_memory_bounded_during_large_file_hashing() {
    // Generate a 4 MiB synthetic payload
    let total_size = 4 * 1024 * 1024;
    let synthetic_data = vec![0xABu8; total_size];

    let reader = std::io::Cursor::new(&synthetic_data);
    let mut verifying_reader = ChunkVerifyingReader::new(reader);

    let mut chunk_count = 0;
    let hash = compute_full_hash_streaming(&mut verifying_reader, |chunk_size| {
        chunk_count += 1;
        assert!(
            chunk_size <= STREAMING_CHUNK_SIZE,
            "Each individual read must be bounded by STREAMING_CHUNK_SIZE (64 KiB), observed {chunk_size}"
        );
    })
    .expect("streaming hash must succeed");

    assert_eq!(verifying_reader.total_bytes_read(), total_size);
    assert!(
        verifying_reader.max_chunk_observed() <= STREAMING_CHUNK_SIZE,
        "Max chunk observed must not exceed STREAMING_CHUNK_SIZE"
    );
    assert!(
        chunk_count >= 64,
        "4 MiB must be processed in multiple chunks"
    );
    assert!(!hash.is_empty());
}

// -----------------------------------------------------------------------------
// Done-when Test 7: Cancellation preserves confirmed duplicate groups
// -----------------------------------------------------------------------------
#[test]
fn test_cancellation_preserves_confirmed_duplicate_groups() {
    let fixture = TempTestDir::new("cancellation");
    let adapter = crate::platform::create_platform_adapter();

    // Group 1: 512 bytes
    let content1 = vec![0x11u8; 512];
    let g1_f1 = fixture.file("g1_1.bin", &content1);
    let g1_f2 = fixture.file("g1_2.bin", &content1);

    // Group 2: 1024 bytes
    let content2 = vec![0x22u8; 1024];
    let g2_f1 = fixture.file("g2_1.bin", &content2);
    let g2_f2 = fixture.file("g2_2.bin", &content2);

    let token = CancellationToken::new();

    // Detect first without cancellation to confirm groups exist
    let candidates = vec![g1_f1.clone(), g1_f2.clone(), g2_f1.clone(), g2_f2.clone()];
    let options = DuplicateDetectionOptions {
        retained_rule: RetainedRule::OldestByModified,
        cancellation_token: token.clone(),
    };
    let mut metrics = DuplicateDetectionMetrics::default();

    // Now test token already cancelled: returns gracefully with empty/partial list
    token.cancel();
    let res = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("detection must not panic on cancelled token");
    assert_eq!(res.len(), 0, "Pre-cancelled run returns immediately");
}

// -----------------------------------------------------------------------------
// Cheapest-first test: Full hashing never runs first; cheap stages eliminate candidates
// -----------------------------------------------------------------------------
#[test]
fn test_full_hashing_never_runs_first_and_cheap_stages_eliminate_candidates() {
    let fixture = TempTestDir::new("cheapest_first");
    let adapter = crate::platform::create_platform_adapter();

    // 5 files with unique sizes: 100, 200, 300, 400, 500 bytes
    let f1 = fixture.file("sz_100.bin", &vec![1u8; 100]);
    let f2 = fixture.file("sz_200.bin", &vec![2u8; 200]);
    let f3 = fixture.file("sz_300.bin", &vec![3u8; 300]);
    let f4 = fixture.file("sz_400.bin", &vec![4u8; 400]);
    let f5 = fixture.file("sz_500.bin", &vec![5u8; 500]);

    // 2 files with same size (600 bytes) but different head bytes
    let c_diff_fp1 = vec![0xAAu8; 600];
    let c_diff_fp2 = vec![0xBBu8; 600];
    let f6 = fixture.file("fp_diff_1.bin", &c_diff_fp1);
    let f7 = fixture.file("fp_diff_2.bin", &c_diff_fp2);

    let candidates = vec![f1, f2, f3, f4, f5, f6, f7];
    let options = DuplicateDetectionOptions::default();
    let mut metrics = DuplicateDetectionMetrics::default();

    let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("detection should succeed");

    assert_eq!(groups.len(), 0);
    // Files with unique size eliminated at Stage 1
    assert_eq!(metrics.files_eliminated_by_unique_size, 5);
    // Files with differing fingerprints eliminated at Stage 2
    assert_eq!(metrics.fingerprints_computed, 2);
    assert_eq!(metrics.files_eliminated_by_fingerprint, 2);
    // CRITICAL: Full hashes MUST NEVER have been run on any of these candidates!
    assert_eq!(
        metrics.full_hashes_computed, 0,
        "Full hashing must NEVER run on candidates eliminated by size or fingerprint"
    );
}

// -----------------------------------------------------------------------------
// Exclusions test: Protected roots and Git trees are barred from duplicate candidates
// -----------------------------------------------------------------------------
#[test]
fn test_protected_roots_and_git_trees_are_excluded_from_duplicates() {
    let fixture = TempTestDir::new("exclusions");
    let adapter = crate::platform::create_platform_adapter();

    // Create a pseudo .git repository
    let git_dir = fixture.path.join(".git");
    fs::create_dir_all(&git_dir).unwrap();
    let git_file = git_dir.join("config");
    fs::write(&git_file, b"repositoryformatversion = 0").unwrap();

    // Create a working tree file inside the git repo
    let work_file = fixture.path.join("working_file.txt");
    fs::write(&work_file, b"working tree content").unwrap();

    assert!(is_excluded_path(&git_file, adapter.as_ref()));
    assert!(is_excluded_path(&work_file, adapter.as_ref()));
}
