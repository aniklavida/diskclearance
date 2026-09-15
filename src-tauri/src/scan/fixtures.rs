use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Disposable test fixture creating an isolated directory tree strictly within std::env::temp_dir().
///
/// Asserts that no path read or written escapes the temporary tree or touches any real user directories.
pub struct DisposableFixtureTree {
    pub root: PathBuf,
}

impl DisposableFixtureTree {
    pub fn new(prefix: &str) -> Self {
        let temp_base = std::env::temp_dir();
        let unique = format!(
            "test-dc-fixture-{prefix}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = temp_base.join(unique);

        // Assert strictly not home directory or system root
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            assert!(
                path != home_path
                    && !path.starts_with(home_path.join("Desktop"))
                    && !path.starts_with(home_path.join("Documents")),
                "Fixture path must never be in user documents/desktop/home"
            );
        }
        assert_ne!(path, Path::new("/"));
        assert_ne!(path, Path::new("/System"));
        assert_ne!(path, Path::new("/Users"));

        std::fs::create_dir_all(&path).expect("failed to create fixture temp directory");
        Self { root: path }
    }

    /// Hard invariant check: panics if any candidate path escapes the temporary fixture.
    pub fn assert_path_in_fixture(&self, path: &Path) {
        let root_canon = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let path_canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        assert!(
            path.starts_with(&self.root) || path_canon.starts_with(&root_canon),
            "Safety invariant violated: path {} is outside fixture root {}",
            path.display(),
            self.root.display()
        );
    }

    pub fn create_dir(&self, rel: &str) -> PathBuf {
        let path = self.root.join(rel);
        self.assert_path_in_fixture(&path);
        std::fs::create_dir_all(&path).expect("failed to create directory in fixture");
        path
    }

    pub fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let path = self.root.join(rel);
        self.assert_path_in_fixture(&path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dir");
        }
        std::fs::write(&path, content).expect("failed to write file in fixture");
        path
    }

    pub fn create_hardlink(&self, src_rel: &str, dst_rel: &str) -> PathBuf {
        let src = self.root.join(src_rel);
        let dst = self.root.join(dst_rel);
        self.assert_path_in_fixture(&src);
        self.assert_path_in_fixture(&dst);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dir for hardlink");
        }
        std::fs::hard_link(&src, &dst).expect("failed to create hardlink");
        dst
    }

    pub fn create_symlink(&self, target: &Path, dst_rel: &str) -> PathBuf {
        let dst = self.root.join(dst_rel);
        self.assert_path_in_fixture(&dst);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dir for symlink");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, &dst).expect("failed to create unix symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target, &dst).expect("failed to create windows symlink");
        dst
    }

    pub fn create_permission_denied_dir(&self, rel: &str) -> PathBuf {
        let path = self.create_dir(rel);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
                .expect("failed to set 0o000 permission");
        }
        path
    }
}

impl Drop for DisposableFixtureTree {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Restore readable/executable permissions so remove_dir_all succeeds
            fn restore_perms(dir: &Path) {
                let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755));
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            restore_perms(&p);
                        } else {
                            let _ = std::fs::set_permissions(
                                &p,
                                std::fs::Permissions::from_mode(0o644),
                            );
                        }
                    }
                }
            }
            restore_perms(&self.root);
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
