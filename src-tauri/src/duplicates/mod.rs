//! Exact-content duplicate detection and storage sharing analysis.
//!
//! Staged comparison:
//! 1. Group candidates by file size.
//! 2. Fast fingerprint of head and tail bytes.
//! 3. Full-content collision-resistant SHA-256 hash strictly on survivors.

pub mod detect;
pub mod engine;
pub mod hashing;
pub mod model;

#[cfg(test)]
pub mod tests;

pub use detect::{CandidateFile, detect_storage_sharing, is_excluded_path};
pub use engine::{
    DuplicateDetectionMetrics, DuplicateDetectionOptions, detect_duplicates, sort_for_retained,
};
pub use hashing::{
    ChunkVerifyingReader, FINGERPRINT_SAMPLE_SIZE, STREAMING_CHUNK_SIZE,
    compute_full_hash_streaming, compute_quick_fingerprint,
};
pub use model::{
    DuplicateGroup, DuplicateItem, DuplicatePlanError, RetainedRule, StorageSharingKind,
};
