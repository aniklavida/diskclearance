use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FlatFixture {
    pub root: PathBuf,
}

impl FlatFixture {
    pub fn new(prefix: &str, files: usize) -> Self {
        let temp_dir = std::env::temp_dir();
        let unique = format!(
            "diskclearance-performance-{prefix}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let root = temp_dir.join(unique);
        assert!(root.starts_with(&temp_dir));
        std::fs::create_dir_all(&root).expect("create performance fixture");

        let directory_count = if files == 0 { 0 } else { files.min(1000) };
        for directory in 0..directory_count {
            let directory_path = root.join(format!("dir_{directory:04}"));
            std::fs::create_dir_all(&directory_path).expect("create performance directory");
        }
        let file_count = files.saturating_sub(directory_count);
        for file in 0..file_count {
            let directory = root.join(format!("dir_{:04}", file / 1000));
            File::create(directory.join(format!("file_{file:07}")))
                .expect("create performance file");
        }

        Self { root }
    }
}

impl Drop for FlatFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

pub fn peak_resident_bytes() -> Option<u64> {
    crate::platform::peak_resident_bytes()
}

pub fn write_repeated_file(path: &Path, bytes: u64, seed: u8) {
    let mut file = File::create(path).expect("create benchmark file");
    let chunk = vec![seed; 1024 * 1024];
    let mut remaining = bytes;
    while remaining > 0 {
        let amount = remaining.min(chunk.len() as u64) as usize;
        file.write_all(&chunk[..amount])
            .expect("write benchmark file");
        remaining -= amount as u64;
    }
    file.flush().expect("flush benchmark file");
}
