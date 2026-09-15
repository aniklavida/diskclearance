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

/// Hard ceiling on how much teardown will touch. A fixture is a handful of
/// files; anything approaching this means the walk has left the tree.
#[cfg(unix)]
const MAX_TEARDOWN_ENTRIES: usize = 10_000;

/// Restores permissions inside `root` so `remove_dir_all` can succeed, and
/// returns how many entries it touched.
///
/// Three properties this must have, each of which it previously did not:
///
/// 1. **It never follows a symlink.** `Path::is_dir` follows links, and
///    `set_permissions` follows them too, so a symlink in the fixture pointing
///    anywhere on the filesystem made teardown walk and `chmod` that target
///    instead. A fixture whose whole purpose is to contain a symlink to a
///    system path — which is exactly what the classification tests build —
///    turned teardown into a recursive `chmod` of everything reachable from it.
///    `DirEntry::file_type` does not follow; symlinks are removed by
///    `remove_dir_all` without ever being walked into.
/// 2. **Every path it touches is inside `root`.** Cheap, and it turns any
///    future escape into a panic in the test that caused it rather than into
///    silent damage somewhere else on the machine.
/// 3. **It is iterative and bounded.** The recursive version had no depth
///    limit, so a directory cycle ran until something killed it.
#[cfg(unix)]
fn restore_perms_within(root: &Path) -> usize {
    use std::os::unix::fs::PermissionsExt;

    let mut visited = 0usize;
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        assert!(
            dir.starts_with(root),
            "fixture teardown tried to leave its own tree: {} is not inside {}",
            dir.display(),
            root.display()
        );
        visited += 1;
        assert!(
            visited <= MAX_TEARDOWN_ENTRIES,
            "fixture teardown visited more than {MAX_TEARDOWN_ENTRIES} entries under {}; \
             it is walking something it did not create",
            root.display()
        );

        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));

        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            // `file_type` reads the directory entry itself and does not follow
            // a symlink, unlike `Path::is_dir`.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                // Never chmod through a link, and never walk one. Whatever it
                // points at is not ours, and `remove_dir_all` unlinks it.
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else {
                visited += 1;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
            }
        }
    }

    visited
}

impl Drop for DisposableFixtureTree {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            restore_perms_within(&self.root);
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    }

    /// Teardown used to follow symlinks, so a fixture containing a link to any
    /// directory turned into a recursive `chmod` of that directory. The
    /// classification tests build exactly such a fixture — a path replaced by a
    /// symlink to a system location — so this was reachable from a normal
    /// `cargo test` run, not a contrived one.
    #[test]
    fn teardown_does_not_follow_a_symlink_out_of_the_fixture() {
        // A directory we own, standing in for anywhere the link might point.
        let outside = DisposableFixtureTree::new("teardown-outside");
        let outside_file = outside.create_file("private.key", b"secret");
        std::fs::set_permissions(&outside_file, std::fs::Permissions::from_mode(0o600))
            .expect("set 0600");
        let outside_dir = outside.create_dir("locked");
        std::fs::set_permissions(&outside_dir, std::fs::Permissions::from_mode(0o700))
            .expect("set 0700");

        let fixture = DisposableFixtureTree::new("teardown-follows-link");
        fixture.create_file("real.txt", b"in the fixture");
        std::os::unix::fs::symlink(&outside.root, fixture.root.join("escape"))
            .expect("create escaping symlink");

        let visited = restore_perms_within(&fixture.root);

        assert_eq!(
            mode_of(&outside_file),
            0o600,
            "teardown reached through the symlink and changed a file outside the fixture"
        );
        assert_eq!(
            mode_of(&outside_dir),
            0o700,
            "teardown reached through the symlink and changed a directory outside the fixture"
        );
        assert!(
            visited < 20,
            "teardown visited {visited} entries for a two-entry fixture; it walked the link"
        );
    }

    /// A link pointing back at the fixture is a cycle. The recursive version had
    /// no depth limit and ran until it was killed.
    #[test]
    fn teardown_terminates_on_a_symlink_cycle() {
        let fixture = DisposableFixtureTree::new("teardown-cycle");
        let inner = fixture.create_dir("a/b");
        std::os::unix::fs::symlink(&fixture.root, inner.join("loop"))
            .expect("create cycle symlink");

        let visited = restore_perms_within(&fixture.root);
        assert!(
            visited < 20,
            "teardown visited {visited} entries walking a cycle it should have skipped"
        );
    }

    /// Permission restoration still has to do its actual job: a 0o000 directory
    /// must be made traversable again so the tree can be removed.
    #[test]
    fn teardown_still_restores_permissions_it_is_meant_to() {
        let fixture = DisposableFixtureTree::new("teardown-restores");
        let locked = fixture.create_permission_denied_dir("locked");
        assert_eq!(mode_of(&locked), 0o000, "fixture should start unreadable");

        restore_perms_within(&fixture.root);
        assert_eq!(mode_of(&locked), 0o755, "teardown must reopen the directory");
    }
}
