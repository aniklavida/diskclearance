use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Number of bytes sampled from the head and tail of a file during the fingerprint stage.
pub const FINGERPRINT_SAMPLE_SIZE: u64 = 4096;

/// Bounded buffer size for streaming full-content hashing (64 KiB).
pub const STREAMING_CHUNK_SIZE: usize = 64 * 1024;

/// Computes a fast fingerprint of a regular file using its size, head bytes, and tail bytes.
///
/// This stage cheaply eliminates non-matching files without reading their full content.
pub fn compute_quick_fingerprint<P: AsRef<Path>>(
    path: P,
    file_size: u64,
) -> std::io::Result<[u8; 32]> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();

    // Include file size in fingerprint domain
    hasher.update(file_size.to_le_bytes());

    let mut head_buf = [0u8; FINGERPRINT_SAMPLE_SIZE as usize];
    let to_read_head = std::cmp::min(file_size, FINGERPRINT_SAMPLE_SIZE) as usize;
    file.read_exact(&mut head_buf[..to_read_head])?;
    hasher.update(&head_buf[..to_read_head]);

    if file_size > FINGERPRINT_SAMPLE_SIZE {
        let tail_start = if file_size > 2 * FINGERPRINT_SAMPLE_SIZE {
            file_size - FINGERPRINT_SAMPLE_SIZE
        } else {
            FINGERPRINT_SAMPLE_SIZE
        };
        file.seek(SeekFrom::Start(tail_start))?;
        let tail_len = (file_size - tail_start) as usize;
        let mut tail_buf = [0u8; FINGERPRINT_SAMPLE_SIZE as usize];
        file.read_exact(&mut tail_buf[..tail_len])?;
        hasher.update(&tail_buf[..tail_len]);
    }

    Ok(hasher.finalize().into())
}

/// Computes collision-resistant SHA-256 hash using streaming reads with a fixed chunk buffer.
///
/// Bounded memory guarantee: Reads are strictly bounded by `STREAMING_CHUNK_SIZE` (64 KiB),
/// ensuring peak memory remains flat regardless of file size.
pub fn compute_full_hash_streaming<R: Read, F: FnMut(usize)>(
    mut reader: R,
    mut on_chunk_read: F,
) -> std::io::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; STREAMING_CHUNK_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        on_chunk_read(bytes_read);
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}

/// A reader wrapper that verifies all `read` operations respect a fixed maximum buffer size.
pub struct ChunkVerifyingReader<R> {
    inner: R,
    max_chunk_observed: usize,
    total_bytes_read: usize,
}

impl<R: Read> ChunkVerifyingReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            max_chunk_observed: 0,
            total_bytes_read: 0,
        }
    }

    pub fn max_chunk_observed(&self) -> usize {
        self.max_chunk_observed
    }

    pub fn total_bytes_read(&self) -> usize {
        self.total_bytes_read
    }
}

impl<R: Read> Read for ChunkVerifyingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > self.max_chunk_observed {
            self.max_chunk_observed = n;
        }
        self.total_bytes_read += n;
        Ok(n)
    }
}
