//! Platform capabilities stay behind this boundary.

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Diagnostic and execution errors emitted by platform adapters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "details")]
pub enum PlatformError {
    Unsupported(&'static str),
    Io(String),
    NotFound(PathBuf),
    PermissionDenied {
        scope: PathBuf,
        reason: String,
        settings_target: Option<SettingsDestination>,
    },
    ResolutionError(String),
    MetadataError(String),
    TrashError(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(cap) => write!(f, "capability unsupported on this platform: {cap}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::NotFound(path) => write!(f, "path not found: {}", path.display()),
            Self::PermissionDenied { scope, reason, .. } => {
                write!(f, "permission denied for {}: {reason}", scope.display())
            }
            Self::ResolutionError(msg) => write!(f, "path resolution error: {msg}"),
            Self::MetadataError(msg) => write!(f, "metadata error: {msg}"),
            Self::TrashError(msg) => write!(f, "trash error: {msg}"),
        }
    }
}

impl std::error::Error for PlatformError {}

/// Typed kind of discovered standard path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PathKind {
    Home,
    ApplicationSupport,
    Caches,
    Trash,
    Applications,
}

/// A discovered standard platform location.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPath {
    pub path: PathBuf,
    pub kind: PathKind,
}

/// Role or scope of a scan root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanRootScope {
    UserHome,
    SystemApplications,
    UserApplications,
    Custom,
}

/// A default root directory where storage scans can begin.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRoot {
    pub path: PathBuf,
    pub label: String,
    pub scope: ScanRootScope,
}

/// System Settings destination to guide user permission grants.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsDestination {
    FullDiskAccess,
    FilesAndFolders,
    SystemSettings,
}

/// Permission query result for a given filesystem scope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopePermission {
    pub scope: PathBuf,
    pub is_readable: bool,
    pub required_settings: Option<SettingsDestination>,
}

/// A trashed filesystem item preserving its origin for Put Back recovery.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashedItem {
    pub trashed_path: PathBuf,
    pub original_path: PathBuf,
    pub display_name: String,
}

/// Existence status of a previously trashed item.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrashedItemStatus {
    Present(TrashedItem),
    Missing(PathBuf),
}

/// Metadata describing an installed application bundle.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationMetadata {
    pub bundle_identifier: Option<String>,
    pub version: Option<String>,
    pub install_location: PathBuf,
    pub measured_footprint_bytes: u64,
}

/// Filesystem device and inode identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIdentity {
    pub device_id: u64,
    pub inode: u64,
}

/// Canonical path information with firmlink normalisation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPath {
    pub original: PathBuf,
    pub canonical: PathBuf,
    pub normalized: PathBuf,
    pub is_firmlink_alias: bool,
}

/// Boundary check when traversing across filesystems or mounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountBoundary {
    pub root_device_id: u64,
    pub path_device_id: u64,
    pub crosses_boundary: bool,
}

/// Category of filesystem entry visited during traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryType {
    File,
    Directory,
    Symlink,
    Other,
}

/// Rich metadata for a single filesystem entry, preserving allocated vs apparent size.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryMetadata {
    pub identity: FileIdentity,
    pub entry_type: EntryType,
    pub apparent_size: u64,
    pub allocated_size: u64,
    pub modified_ms: Option<u64>,
    pub nlink: u64,
}

/// Platform capabilities contract.
pub trait PlatformAdapter: Send + Sync {
    /// Canonical name of the platform.
    fn platform_name(&self) -> &'static str;

    // --- Path discovery ---
    fn home_directory(&self) -> Result<DiscoveredPath, PlatformError>;
    fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError>;
    fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError>;
    fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError>;
    fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError>;
    fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError>;

    // --- Permissions ---
    fn check_read_permission(&self, scope: &Path) -> Result<ScopePermission, PlatformError>;
    fn settings_destination_for_scope(
        &self,
        scope: &Path,
    ) -> Result<SettingsDestination, PlatformError>;

    // --- Trash ---
    fn move_to_trash(&self, path: &Path) -> Result<TrashedItem, PlatformError>;
    fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError>;
    fn check_trashed_item_exists(
        &self,
        item: &TrashedItem,
    ) -> Result<TrashedItemStatus, PlatformError>;
    fn restore_from_trash(
        &self,
        trashed_path: &Path,
        destination_path: &Path,
    ) -> Result<(), PlatformError>;

    // --- Application metadata ---
    fn application_metadata(&self, path: &Path) -> Result<ApplicationMetadata, PlatformError>;

    // --- Filesystem identity ---
    fn file_identity(&self, path: &Path) -> Result<FileIdentity, PlatformError>;
    fn canonicalize_and_normalize(&self, path: &Path) -> Result<ResolvedPath, PlatformError>;
    fn check_mount_boundary(
        &self,
        base: &Path,
        target: &Path,
    ) -> Result<MountBoundary, PlatformError>;
    fn read_entry_metadata(&self, path: &Path) -> Result<EntryMetadata, PlatformError>;
}

/// Fallback adapter implementing the full trait with typed unsupported errors.
pub struct UnsupportedAdapter {
    name: &'static str,
}

impl UnsupportedAdapter {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl PlatformAdapter for UnsupportedAdapter {
    fn platform_name(&self) -> &'static str {
        self.name
    }

    fn home_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("home_directory"))
    }

    fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("application_support_directory"))
    }

    fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("caches_directory"))
    }

    fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("trash_directory"))
    }

    fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError> {
        Err(PlatformError::Unsupported("application_directories"))
    }

    fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError> {
        Err(PlatformError::Unsupported("default_scan_roots"))
    }

    fn check_read_permission(&self, _scope: &Path) -> Result<ScopePermission, PlatformError> {
        Err(PlatformError::Unsupported("check_read_permission"))
    }

    fn settings_destination_for_scope(
        &self,
        _scope: &Path,
    ) -> Result<SettingsDestination, PlatformError> {
        Err(PlatformError::Unsupported("settings_destination_for_scope"))
    }

    fn move_to_trash(&self, _path: &Path) -> Result<TrashedItem, PlatformError> {
        Err(PlatformError::Unsupported("move_to_trash"))
    }

    fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError> {
        Err(PlatformError::Unsupported("enumerate_trash"))
    }

    fn check_trashed_item_exists(
        &self,
        _item: &TrashedItem,
    ) -> Result<TrashedItemStatus, PlatformError> {
        Err(PlatformError::Unsupported("check_trashed_item_exists"))
    }

    fn restore_from_trash(
        &self,
        _trashed_path: &Path,
        _destination_path: &Path,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("restore_from_trash"))
    }

    fn application_metadata(&self, _path: &Path) -> Result<ApplicationMetadata, PlatformError> {
        Err(PlatformError::Unsupported("application_metadata"))
    }

    fn file_identity(&self, _path: &Path) -> Result<FileIdentity, PlatformError> {
        Err(PlatformError::Unsupported("file_identity"))
    }

    fn canonicalize_and_normalize(&self, _path: &Path) -> Result<ResolvedPath, PlatformError> {
        Err(PlatformError::Unsupported("canonicalize_and_normalize"))
    }

    fn check_mount_boundary(
        &self,
        _base: &Path,
        _target: &Path,
    ) -> Result<MountBoundary, PlatformError> {
        Err(PlatformError::Unsupported("check_mount_boundary"))
    }

    fn read_entry_metadata(&self, _path: &Path) -> Result<EntryMetadata, PlatformError> {
        Err(PlatformError::Unsupported("read_entry_metadata"))
    }
}

#[cfg(target_os = "macos")]
pub struct MacOsAdapter;

#[cfg(target_os = "macos")]
impl PlatformAdapter for MacOsAdapter {
    fn platform_name(&self) -> &'static str {
        "macOS"
    }

    fn home_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        match std::env::var_os("HOME") {
            Some(path) if !path.is_empty() => Ok(DiscoveredPath {
                path: PathBuf::from(path),
                kind: PathKind::Home,
            }),
            _ => Err(PlatformError::NotFound(PathBuf::from("$HOME"))),
        }
    }

    fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        let home = self.home_directory()?;
        Ok(DiscoveredPath {
            path: home.path.join("Library").join("Application Support"),
            kind: PathKind::ApplicationSupport,
        })
    }

    fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        let home = self.home_directory()?;
        Ok(DiscoveredPath {
            path: home.path.join("Library").join("Caches"),
            kind: PathKind::Caches,
        })
    }

    fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        let home = self.home_directory()?;
        Ok(DiscoveredPath {
            path: home.path.join(".Trash"),
            kind: PathKind::Trash,
        })
    }

    fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError> {
        let mut dirs = vec![
            DiscoveredPath {
                path: PathBuf::from("/Applications"),
                kind: PathKind::Applications,
            },
            DiscoveredPath {
                path: PathBuf::from("/System/Applications"),
                kind: PathKind::Applications,
            },
        ];
        if let Ok(home) = self.home_directory() {
            dirs.push(DiscoveredPath {
                path: home.path.join("Applications"),
                kind: PathKind::Applications,
            });
        }
        Ok(dirs)
    }

    fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError> {
        let mut roots = Vec::new();
        if let Ok(home) = self.home_directory() {
            roots.push(ScanRoot {
                path: home.path.clone(),
                label: "User Home".to_string(),
                scope: ScanRootScope::UserHome,
            });
            roots.push(ScanRoot {
                path: home.path.join("Applications"),
                label: "User Applications".to_string(),
                scope: ScanRootScope::UserApplications,
            });
        }
        roots.push(ScanRoot {
            path: PathBuf::from("/Applications"),
            label: "System Applications".to_string(),
            scope: ScanRootScope::SystemApplications,
        });
        Ok(roots)
    }

    fn check_read_permission(&self, scope: &Path) -> Result<ScopePermission, PlatformError> {
        if !scope.exists() {
            return Err(PlatformError::NotFound(scope.to_path_buf()));
        }

        let is_readable = if scope.is_dir() {
            match std::fs::read_dir(scope) {
                Ok(mut entries) => {
                    let _ = entries.next();
                    true
                }
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => false,
                Err(err) => return Err(PlatformError::Io(err.to_string())),
            }
        } else {
            match std::fs::File::open(scope) {
                Ok(_) => true,
                Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => false,
                Err(err) => return Err(PlatformError::Io(err.to_string())),
            }
        };

        let required_settings = if is_readable {
            None
        } else {
            Some(self.settings_destination_for_scope(scope)?)
        };

        Ok(ScopePermission {
            scope: scope.to_path_buf(),
            is_readable,
            required_settings,
        })
    }

    fn settings_destination_for_scope(
        &self,
        scope: &Path,
    ) -> Result<SettingsDestination, PlatformError> {
        let scope_str = scope.to_string_lossy();
        if scope_str.contains("/Library/Mail")
            || scope_str.contains("/Library/Messages")
            || scope_str.contains("/Library/Safari")
            || scope_str.contains("/Library/HomeKit")
            || scope_str.contains("/Library/IdentityServices")
            || scope_str.contains("/Library/PersonalizationPortrait")
            || scope_str.contains("/Library/Application Support/com.apple.TCC")
        {
            Ok(SettingsDestination::FullDiskAccess)
        } else if scope_str.contains("/Desktop")
            || scope_str.contains("/Documents")
            || scope_str.contains("/Downloads")
        {
            Ok(SettingsDestination::FilesAndFolders)
        } else if scope_str.contains("/Library/") {
            Ok(SettingsDestination::FullDiskAccess)
        } else {
            Ok(SettingsDestination::SystemSettings)
        }
    }

    fn move_to_trash(&self, path: &Path) -> Result<TrashedItem, PlatformError> {
        if !path.exists() && !path.is_symlink() {
            return Err(PlatformError::NotFound(path.to_path_buf()));
        }

        let path_str = path.to_str().ok_or_else(|| {
            PlatformError::ResolutionError("Path contains non-UTF-8 characters".into())
        })?;

        // Invoke native macOS Cocoa NSFileManager.trashItemAtURL via osascript JXA.
        // This ensures the item lands in the native macOS Trash with Put Back metadata,
        // without prompting for Apple Events TCC permissions.
        // The path argument is passed via argv array, eliminating command injection risks.
        let script = r#"ObjC.import("Foundation");
function run(argv) {
    var path = argv[0];
    var url = $.NSURL.fileURLWithPath(path);
    var resultingURL = $();
    var err = $();
    var ok = $.NSFileManager.defaultManager.trashItemAtURLResultingItemURLError(url, resultingURL, err);
    if (!ok) {
        var desc = (err && err.localizedDescription) ? err.localizedDescription.js : "Failed to move to trash";
        return JSON.stringify({ success: false, error: desc });
    }
    var resPath = (resultingURL && resultingURL.path) ? resultingURL.path.js : "";
    return JSON.stringify({ success: true, resultingPath: resPath });
}"#;

        let output = std::process::Command::new("/usr/bin/osascript")
            .args(["-l", "JavaScript", "-e", script])
            .arg(path_str)
            .output()
            .map_err(|e| PlatformError::Io(format!("Failed to spawn osascript: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::Io(format!("osascript failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        #[derive(serde::Deserialize)]
        struct JxaTrashOutput {
            success: bool,
            #[serde(rename = "resultingPath")]
            resulting_path: Option<String>,
            error: Option<String>,
        }

        let parsed: JxaTrashOutput = serde_json::from_str(stdout.trim()).map_err(|e| {
            PlatformError::Io(format!("Failed to parse trash output: {e} ({stdout})"))
        })?;

        if !parsed.success {
            let msg = parsed
                .error
                .unwrap_or_else(|| "Unknown trash failure".into());
            if msg.to_lowercase().contains("permission") {
                return Err(PlatformError::PermissionDenied {
                    scope: path.to_path_buf(),
                    reason: msg,
                    settings_target: None,
                });
            }
            return Err(PlatformError::TrashError(msg));
        }

        let trashed_path = parsed
            .resulting_path
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());

        Ok(TrashedItem {
            trashed_path,
            original_path: path.to_path_buf(),
            display_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        })
    }

    fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError> {
        Err(PlatformError::Unsupported(
            "enumerate_trash is not yet implemented (planned for a future history/restore milestone)",
        ))
    }

    fn check_trashed_item_exists(
        &self,
        item: &TrashedItem,
    ) -> Result<TrashedItemStatus, PlatformError> {
        if item.trashed_path.exists() || item.trashed_path.is_symlink() {
            Ok(TrashedItemStatus::Present(item.clone()))
        } else {
            Ok(TrashedItemStatus::Missing(item.trashed_path.clone()))
        }
    }

    fn restore_from_trash(
        &self,
        trashed_path: &Path,
        destination_path: &Path,
    ) -> Result<(), PlatformError> {
        if destination_path.exists() {
            return Err(PlatformError::Io(format!(
                "Destination '{}' already exists; refusing to overwrite",
                destination_path.display()
            )));
        }
        if let Some(parent) = destination_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| PlatformError::Io(e.to_string()))?;
            }
        }
        std::fs::rename(trashed_path, destination_path)
            .map_err(|e| PlatformError::Io(e.to_string()))
    }

    fn application_metadata(&self, _path: &Path) -> Result<ApplicationMetadata, PlatformError> {
        Err(PlatformError::Unsupported(
            "application_metadata is scheduled for M3 and unsupported in M0/M1",
        ))
    }

    fn file_identity(&self, path: &Path) -> Result<FileIdentity, PlatformError> {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::symlink_metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound(path.to_path_buf()),
            _ => PlatformError::Io(e.to_string()),
        })?;
        Ok(FileIdentity {
            device_id: meta.dev(),
            inode: meta.ino(),
        })
    }

    fn canonicalize_and_normalize(&self, path: &Path) -> Result<ResolvedPath, PlatformError> {
        let is_symlink = path.is_symlink();
        let canonical = if is_symlink {
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let parent_canonical = if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.canonicalize().map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => PlatformError::NotFound(path.to_path_buf()),
                    _ => PlatformError::ResolutionError(e.to_string()),
                })?
            };
            if let Some(name) = path.file_name() {
                parent_canonical.join(name)
            } else {
                parent_canonical
            }
        } else {
            path.canonicalize().map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => PlatformError::NotFound(path.to_path_buf()),
                _ => PlatformError::ResolutionError(e.to_string()),
            })?
        };

        let (normalized, is_firmlink_alias) =
            if let Ok(stripped) = canonical.strip_prefix("/System/Volumes/Data") {
                (Path::new("/").join(stripped), true)
            } else {
                (canonical.clone(), false)
            };
        Ok(ResolvedPath {
            original: path.to_path_buf(),
            canonical,
            normalized,
            is_firmlink_alias,
        })
    }

    fn check_mount_boundary(
        &self,
        base: &Path,
        target: &Path,
    ) -> Result<MountBoundary, PlatformError> {
        let base_identity = self.file_identity(base)?;
        let target_identity = self.file_identity(target)?;
        Ok(MountBoundary {
            root_device_id: base_identity.device_id,
            path_device_id: target_identity.device_id,
            crosses_boundary: base_identity.device_id != target_identity.device_id,
        })
    }

    fn read_entry_metadata(&self, path: &Path) -> Result<EntryMetadata, PlatformError> {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::symlink_metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => PlatformError::NotFound(path.to_path_buf()),
            std::io::ErrorKind::PermissionDenied => PlatformError::PermissionDenied {
                scope: path.to_path_buf(),
                reason: e.to_string(),
                settings_target: None,
            },
            _ => PlatformError::Io(e.to_string()),
        })?;

        let entry_type = if meta.is_symlink() {
            EntryType::Symlink
        } else if meta.is_dir() {
            EntryType::Directory
        } else if meta.is_file() {
            EntryType::File
        } else {
            EntryType::Other
        };

        let apparent_size = meta.len();
        let allocated_size = meta.blocks() * 512;
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);

        Ok(EntryMetadata {
            identity: FileIdentity {
                device_id: meta.dev(),
                inode: meta.ino(),
            },
            entry_type,
            apparent_size,
            allocated_size,
            modified_ms,
            nlink: meta.nlink(),
        })
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsAdapter;

#[cfg(target_os = "windows")]
impl PlatformAdapter for WindowsAdapter {
    fn platform_name(&self) -> &'static str {
        "Windows"
    }

    fn home_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("home_directory on Windows"))
    }

    fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported(
            "application_support_directory on Windows",
        ))
    }

    fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("caches_directory on Windows"))
    }

    fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError> {
        Err(PlatformError::Unsupported("trash_directory on Windows"))
    }

    fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError> {
        Err(PlatformError::Unsupported(
            "application_directories on Windows",
        ))
    }

    fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError> {
        Err(PlatformError::Unsupported("default_scan_roots on Windows"))
    }

    fn check_read_permission(&self, _scope: &Path) -> Result<ScopePermission, PlatformError> {
        Err(PlatformError::Unsupported(
            "check_read_permission on Windows",
        ))
    }

    fn settings_destination_for_scope(
        &self,
        _scope: &Path,
    ) -> Result<SettingsDestination, PlatformError> {
        Err(PlatformError::Unsupported(
            "settings_destination_for_scope on Windows",
        ))
    }

    fn move_to_trash(&self, _path: &Path) -> Result<TrashedItem, PlatformError> {
        Err(PlatformError::Unsupported("move_to_trash on Windows"))
    }

    fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError> {
        Err(PlatformError::Unsupported("enumerate_trash on Windows"))
    }

    fn check_trashed_item_exists(
        &self,
        _item: &TrashedItem,
    ) -> Result<TrashedItemStatus, PlatformError> {
        Err(PlatformError::Unsupported(
            "check_trashed_item_exists on Windows",
        ))
    }

    fn restore_from_trash(
        &self,
        _trashed_path: &Path,
        _destination_path: &Path,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("restore_from_trash on Windows"))
    }

    fn application_metadata(&self, _path: &Path) -> Result<ApplicationMetadata, PlatformError> {
        Err(PlatformError::Unsupported(
            "application_metadata on Windows",
        ))
    }

    fn file_identity(&self, _path: &Path) -> Result<FileIdentity, PlatformError> {
        Err(PlatformError::Unsupported("file_identity on Windows"))
    }

    fn canonicalize_and_normalize(&self, _path: &Path) -> Result<ResolvedPath, PlatformError> {
        Err(PlatformError::Unsupported(
            "canonicalize_and_normalize on Windows",
        ))
    }

    fn check_mount_boundary(
        &self,
        _base: &Path,
        _target: &Path,
    ) -> Result<MountBoundary, PlatformError> {
        Err(PlatformError::Unsupported(
            "check_mount_boundary on Windows",
        ))
    }

    fn read_entry_metadata(&self, _path: &Path) -> Result<EntryMetadata, PlatformError> {
        Err(PlatformError::Unsupported("read_entry_metadata on Windows"))
    }
}

/// Create the platform adapter instance for the current operating system.
pub fn create_platform_adapter() -> Arc<dyn PlatformAdapter> {
    #[cfg(target_os = "macos")]
    {
        Arc::new(MacOsAdapter)
    }
    #[cfg(target_os = "windows")]
    {
        Arc::new(WindowsAdapter)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Arc::new(UnsupportedAdapter::new("unsupported"))
    }
}

static CURRENT_ADAPTER: std::sync::LazyLock<Arc<dyn PlatformAdapter>> =
    std::sync::LazyLock::new(create_platform_adapter);

/// Return a reference-counted handle to the resolved platform adapter.
pub fn current_platform_adapter() -> Arc<dyn PlatformAdapter> {
    CURRENT_ADAPTER.clone()
}

/// Return the canonical display name of the current platform via the resolved adapter.
pub fn current_platform_name() -> &'static str {
    CURRENT_ADAPTER.platform_name()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    pub struct TestAdapter {
        pub name: &'static str,
        pub home: PathBuf,
        pub app_support: PathBuf,
        pub caches: PathBuf,
        pub trash: PathBuf,
        pub application_dirs: Vec<DiscoveredPath>,
        pub scan_roots: Vec<ScanRoot>,
        pub permissions: Arc<Mutex<HashMap<PathBuf, ScopePermission>>>,
        pub trashed_items: Arc<Mutex<Vec<TrashedItem>>>,
        pub application_metadata_map: Arc<Mutex<HashMap<PathBuf, ApplicationMetadata>>>,
        pub identities: Arc<Mutex<HashMap<PathBuf, FileIdentity>>>,
        pub resolved_paths: Arc<Mutex<HashMap<PathBuf, ResolvedPath>>>,
        pub mount_boundaries: Arc<Mutex<HashMap<(PathBuf, PathBuf), MountBoundary>>>,
        pub entry_metadata_map: Arc<Mutex<HashMap<PathBuf, EntryMetadata>>>,
    }

    impl Default for TestAdapter {
        fn default() -> Self {
            let home = PathBuf::from("/mock/test/home");
            let app_support = home.join("Library/Application Support");
            let caches = home.join("Library/Caches");
            let trash = home.join(".Trash");
            let application_dirs = vec![
                DiscoveredPath {
                    path: PathBuf::from("/mock/Applications"),
                    kind: PathKind::Applications,
                },
                DiscoveredPath {
                    path: home.join("Applications"),
                    kind: PathKind::Applications,
                },
            ];
            let scan_roots = vec![
                ScanRoot {
                    path: home.clone(),
                    label: "Mock Home".to_string(),
                    scope: ScanRootScope::UserHome,
                },
                ScanRoot {
                    path: PathBuf::from("/mock/Applications"),
                    label: "Mock Applications".to_string(),
                    scope: ScanRootScope::SystemApplications,
                },
            ];

            Self {
                name: "test",
                home,
                app_support,
                caches,
                trash,
                application_dirs,
                scan_roots,
                permissions: Arc::new(Mutex::new(HashMap::new())),
                trashed_items: Arc::new(Mutex::new(Vec::new())),
                application_metadata_map: Arc::new(Mutex::new(HashMap::new())),
                identities: Arc::new(Mutex::new(HashMap::new())),
                resolved_paths: Arc::new(Mutex::new(HashMap::new())),
                mount_boundaries: Arc::new(Mutex::new(HashMap::new())),
                entry_metadata_map: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    impl TestAdapter {
        pub fn with_permission(
            self,
            scope: PathBuf,
            is_readable: bool,
            required_settings: Option<SettingsDestination>,
        ) -> Self {
            self.permissions.lock().unwrap().insert(
                scope.clone(),
                ScopePermission {
                    scope,
                    is_readable,
                    required_settings,
                },
            );
            self
        }

        pub fn with_file_identity(self, path: PathBuf, device_id: u64, inode: u64) -> Self {
            self.identities
                .lock()
                .unwrap()
                .insert(path, FileIdentity { device_id, inode });
            self
        }

        pub fn with_resolved_path(self, path: PathBuf, resolved: ResolvedPath) -> Self {
            self.resolved_paths.lock().unwrap().insert(path, resolved);
            self
        }

        pub fn with_application_metadata(
            self,
            path: PathBuf,
            metadata: ApplicationMetadata,
        ) -> Self {
            self.application_metadata_map
                .lock()
                .unwrap()
                .insert(path, metadata);
            self
        }

        pub fn with_mount_boundary(
            self,
            base: PathBuf,
            target: PathBuf,
            crosses_boundary: bool,
        ) -> Self {
            let boundary = MountBoundary {
                root_device_id: 100,
                path_device_id: if crosses_boundary { 200 } else { 100 },
                crosses_boundary,
            };
            self.mount_boundaries
                .lock()
                .unwrap()
                .insert((base, target), boundary);
            self
        }

        pub fn with_entry_metadata(self, path: PathBuf, meta: EntryMetadata) -> Self {
            self.entry_metadata_map.lock().unwrap().insert(path, meta);
            self
        }
    }

    impl PlatformAdapter for TestAdapter {
        fn platform_name(&self) -> &'static str {
            self.name
        }

        fn home_directory(&self) -> Result<DiscoveredPath, PlatformError> {
            Ok(DiscoveredPath {
                path: self.home.clone(),
                kind: PathKind::Home,
            })
        }

        fn application_support_directory(&self) -> Result<DiscoveredPath, PlatformError> {
            Ok(DiscoveredPath {
                path: self.app_support.clone(),
                kind: PathKind::ApplicationSupport,
            })
        }

        fn caches_directory(&self) -> Result<DiscoveredPath, PlatformError> {
            Ok(DiscoveredPath {
                path: self.caches.clone(),
                kind: PathKind::Caches,
            })
        }

        fn trash_directory(&self) -> Result<DiscoveredPath, PlatformError> {
            Ok(DiscoveredPath {
                path: self.trash.clone(),
                kind: PathKind::Trash,
            })
        }

        fn application_directories(&self) -> Result<Vec<DiscoveredPath>, PlatformError> {
            Ok(self.application_dirs.clone())
        }

        fn default_scan_roots(&self) -> Result<Vec<ScanRoot>, PlatformError> {
            Ok(self.scan_roots.clone())
        }

        fn check_read_permission(&self, scope: &Path) -> Result<ScopePermission, PlatformError> {
            if let Some(perm) = self.permissions.lock().unwrap().get(scope) {
                return Ok(perm.clone());
            }
            Ok(ScopePermission {
                scope: scope.to_path_buf(),
                is_readable: true,
                required_settings: None,
            })
        }

        fn settings_destination_for_scope(
            &self,
            _scope: &Path,
        ) -> Result<SettingsDestination, PlatformError> {
            Ok(SettingsDestination::SystemSettings)
        }

        fn move_to_trash(&self, path: &Path) -> Result<TrashedItem, PlatformError> {
            let filename = path.file_name().unwrap_or_default();
            let trashed_path = self.trash.join(filename);
            if path.exists() {
                if let Some(parent) = trashed_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::rename(path, &trashed_path);
            }
            let item = TrashedItem {
                trashed_path,
                original_path: path.to_path_buf(),
                display_name: filename.to_string_lossy().to_string(),
            };
            self.trashed_items.lock().unwrap().push(item.clone());
            Ok(item)
        }

        fn enumerate_trash(&self) -> Result<Vec<TrashedItem>, PlatformError> {
            Ok(self.trashed_items.lock().unwrap().clone())
        }

        fn check_trashed_item_exists(
            &self,
            item: &TrashedItem,
        ) -> Result<TrashedItemStatus, PlatformError> {
            let items = self.trashed_items.lock().unwrap();
            if items.iter().any(|i| i == item) {
                Ok(TrashedItemStatus::Present(item.clone()))
            } else {
                Ok(TrashedItemStatus::Missing(item.trashed_path.clone()))
            }
        }

        fn restore_from_trash(
            &self,
            trashed_path: &Path,
            destination_path: &Path,
        ) -> Result<(), PlatformError> {
            if destination_path.exists() {
                return Err(PlatformError::Io(format!(
                    "Destination '{}' already exists; refusing to overwrite",
                    destination_path.display()
                )));
            }
            if let Some(parent) = destination_path.parent() {
                if !parent.exists() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            if trashed_path.exists() {
                std::fs::rename(trashed_path, destination_path)
                    .map_err(|e| PlatformError::Io(e.to_string()))?;
            }
            let mut items = self.trashed_items.lock().unwrap();
            items.retain(|i| i.trashed_path != trashed_path);
            Ok(())
        }

        fn application_metadata(&self, path: &Path) -> Result<ApplicationMetadata, PlatformError> {
            if let Some(meta) = self.application_metadata_map.lock().unwrap().get(path) {
                Ok(meta.clone())
            } else {
                Err(PlatformError::NotFound(path.to_path_buf()))
            }
        }

        fn file_identity(&self, path: &Path) -> Result<FileIdentity, PlatformError> {
            if let Some(id) = self.identities.lock().unwrap().get(path) {
                Ok(*id)
            } else if let Some(meta) = self.entry_metadata_map.lock().unwrap().get(path) {
                Ok(meta.identity)
            } else if let Ok(meta) = std::fs::symlink_metadata(path) {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    Ok(FileIdentity {
                        device_id: meta.dev(),
                        inode: meta.ino(),
                    })
                }
                #[cfg(not(unix))]
                {
                    Ok(FileIdentity {
                        device_id: 1,
                        inode: 100,
                    })
                }
            } else {
                Ok(FileIdentity {
                    device_id: 1,
                    inode: 100,
                })
            }
        }

        fn canonicalize_and_normalize(&self, path: &Path) -> Result<ResolvedPath, PlatformError> {
            if let Some(res) = self.resolved_paths.lock().unwrap().get(path) {
                Ok(res.clone())
            } else if path.exists() || path.is_symlink() {
                let is_symlink = path.is_symlink();
                let canonical = if is_symlink {
                    let parent = path.parent().unwrap_or_else(|| Path::new(""));
                    let parent_canonical = if parent.as_os_str().is_empty() {
                        PathBuf::from(".")
                    } else {
                        parent
                            .canonicalize()
                            .unwrap_or_else(|_| parent.to_path_buf())
                    };
                    if let Some(name) = path.file_name() {
                        parent_canonical.join(name)
                    } else {
                        parent_canonical
                    }
                } else {
                    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
                };
                let (normalized, is_firmlink_alias) =
                    if let Ok(stripped) = canonical.strip_prefix("/System/Volumes/Data") {
                        (Path::new("/").join(stripped), true)
                    } else {
                        (canonical.clone(), false)
                    };
                Ok(ResolvedPath {
                    original: path.to_path_buf(),
                    canonical,
                    normalized,
                    is_firmlink_alias,
                })
            } else {
                Ok(ResolvedPath {
                    original: path.to_path_buf(),
                    canonical: path.to_path_buf(),
                    normalized: path.to_path_buf(),
                    is_firmlink_alias: false,
                })
            }
        }

        fn check_mount_boundary(
            &self,
            base: &Path,
            target: &Path,
        ) -> Result<MountBoundary, PlatformError> {
            if let Some(boundary) = self
                .mount_boundaries
                .lock()
                .unwrap()
                .get(&(base.to_path_buf(), target.to_path_buf()))
            {
                return Ok(*boundary);
            }
            let b = self.file_identity(base)?;
            let t = self.file_identity(target)?;
            Ok(MountBoundary {
                root_device_id: b.device_id,
                path_device_id: t.device_id,
                crosses_boundary: b.device_id != t.device_id,
            })
        }

        fn read_entry_metadata(&self, path: &Path) -> Result<EntryMetadata, PlatformError> {
            if let Some(meta) = self.entry_metadata_map.lock().unwrap().get(path) {
                return Ok(meta.clone());
            }
            if let Some(id) = self.identities.lock().unwrap().get(path) {
                return Ok(EntryMetadata {
                    identity: *id,
                    entry_type: EntryType::File,
                    apparent_size: 1024,
                    allocated_size: 1024,
                    modified_ms: Some(0),
                    nlink: 1,
                });
            }
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                let entry_type = if meta.is_symlink() {
                    EntryType::Symlink
                } else if meta.is_dir() {
                    EntryType::Directory
                } else if meta.is_file() {
                    EntryType::File
                } else {
                    EntryType::Other
                };
                let apparent_size = meta.len();
                #[cfg(unix)]
                let (dev, ino, allocated_size, nlink) = {
                    use std::os::unix::fs::MetadataExt;
                    (meta.dev(), meta.ino(), meta.blocks() * 512, meta.nlink())
                };
                #[cfg(not(unix))]
                let (dev, ino, allocated_size, nlink) = (1, 100, meta.len(), 1);

                let modified_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64);

                return Ok(EntryMetadata {
                    identity: FileIdentity {
                        device_id: dev,
                        inode: ino,
                    },
                    entry_type,
                    apparent_size,
                    allocated_size,
                    modified_ms,
                    nlink,
                });
            }
            Err(PlatformError::NotFound(path.to_path_buf()))
        }
    }

    #[test]
    fn platform_contract_is_independent_of_the_os() {
        let adapter = TestAdapter::default();
        assert_eq!(adapter.platform_name(), "test");
    }

    #[test]
    fn test_adapter_reports_scripted_paths() {
        let adapter = TestAdapter::default();
        let home = adapter.home_directory().unwrap();
        assert_eq!(home.kind, PathKind::Home);
        assert_eq!(home.path, PathBuf::from("/mock/test/home"));

        let app_support = adapter.application_support_directory().unwrap();
        assert_eq!(app_support.kind, PathKind::ApplicationSupport);

        let caches = adapter.caches_directory().unwrap();
        assert_eq!(caches.kind, PathKind::Caches);

        let trash = adapter.trash_directory().unwrap();
        assert_eq!(trash.kind, PathKind::Trash);

        let app_dirs = adapter.application_directories().unwrap();
        assert_eq!(app_dirs.len(), 2);

        let roots = adapter.default_scan_roots().unwrap();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_adapter_reports_scripted_permissions() {
        let protected_path = PathBuf::from("/mock/test/home/Library/Mail");
        let adapter = TestAdapter::default().with_permission(
            protected_path.clone(),
            false,
            Some(SettingsDestination::FullDiskAccess),
        );

        let perm = adapter.check_read_permission(&protected_path).unwrap();
        assert!(!perm.is_readable);
        assert_eq!(
            perm.required_settings,
            Some(SettingsDestination::FullDiskAccess)
        );

        let open_path = PathBuf::from("/mock/test/home/Downloads");
        let open_perm = adapter.check_read_permission(&open_path).unwrap();
        assert!(open_perm.is_readable);
        assert_eq!(open_perm.required_settings, None);
    }

    #[test]
    fn test_adapter_simulates_trash_lifecycle() {
        let adapter = TestAdapter::default();
        let target = PathBuf::from("/mock/test/home/file.txt");

        let trashed = adapter.move_to_trash(&target).unwrap();
        assert_eq!(trashed.original_path, target);

        let in_trash = adapter.enumerate_trash().unwrap();
        assert_eq!(in_trash.len(), 1);
        assert_eq!(in_trash[0], trashed);

        let status = adapter.check_trashed_item_exists(&trashed).unwrap();
        assert_eq!(status, TrashedItemStatus::Present(trashed.clone()));

        let missing_item = TrashedItem {
            trashed_path: PathBuf::from("/mock/test/home/.Trash/other.txt"),
            original_path: PathBuf::from("/mock/test/home/other.txt"),
            display_name: "other.txt".to_string(),
        };
        let missing_status = adapter.check_trashed_item_exists(&missing_item).unwrap();
        assert_eq!(
            missing_status,
            TrashedItemStatus::Missing(missing_item.trashed_path)
        );
    }

    #[test]
    fn test_adapter_reports_scripted_file_identity_and_boundaries() {
        let path_a = PathBuf::from("/mock/test/volume_a/file");
        let path_b = PathBuf::from("/mock/test/volume_b/file");

        let adapter = TestAdapter::default()
            .with_file_identity(path_a.clone(), 1, 101)
            .with_file_identity(path_b.clone(), 2, 202);

        let id_a = adapter.file_identity(&path_a).unwrap();
        assert_eq!(id_a.device_id, 1);
        assert_eq!(id_a.inode, 101);

        let boundary = adapter.check_mount_boundary(&path_a, &path_b).unwrap();
        assert!(boundary.crosses_boundary);
        assert_eq!(boundary.root_device_id, 1);
        assert_eq!(boundary.path_device_id, 2);
    }

    #[test]
    fn unsupported_adapter_returns_typed_unsupported_for_all_capabilities() {
        let adapter = UnsupportedAdapter::new("generic-unsupported");
        assert_eq!(adapter.platform_name(), "generic-unsupported");

        assert_eq!(
            adapter.home_directory(),
            Err(PlatformError::Unsupported("home_directory"))
        );
        assert_eq!(
            adapter.application_support_directory(),
            Err(PlatformError::Unsupported("application_support_directory"))
        );
        assert_eq!(
            adapter.caches_directory(),
            Err(PlatformError::Unsupported("caches_directory"))
        );
        assert_eq!(
            adapter.trash_directory(),
            Err(PlatformError::Unsupported("trash_directory"))
        );
        assert_eq!(
            adapter.application_directories(),
            Err(PlatformError::Unsupported("application_directories"))
        );
        assert_eq!(
            adapter.default_scan_roots(),
            Err(PlatformError::Unsupported("default_scan_roots"))
        );
        assert_eq!(
            adapter.check_read_permission(Path::new("/dummy")),
            Err(PlatformError::Unsupported("check_read_permission"))
        );
        assert_eq!(
            adapter.settings_destination_for_scope(Path::new("/dummy")),
            Err(PlatformError::Unsupported("settings_destination_for_scope"))
        );
        assert_eq!(
            adapter.move_to_trash(Path::new("/dummy")),
            Err(PlatformError::Unsupported("move_to_trash"))
        );
        assert_eq!(
            adapter.enumerate_trash(),
            Err(PlatformError::Unsupported("enumerate_trash"))
        );
        let dummy_trashed = TrashedItem {
            trashed_path: PathBuf::from("/dummy/.Trash/item"),
            original_path: PathBuf::from("/dummy/item"),
            display_name: "item".to_string(),
        };
        assert_eq!(
            adapter.check_trashed_item_exists(&dummy_trashed),
            Err(PlatformError::Unsupported("check_trashed_item_exists"))
        );
        assert_eq!(
            adapter.restore_from_trash(Path::new("/dummy/trash"), Path::new("/dummy/dest")),
            Err(PlatformError::Unsupported("restore_from_trash"))
        );
        assert_eq!(
            adapter.application_metadata(Path::new("/dummy.app")),
            Err(PlatformError::Unsupported("application_metadata"))
        );
        assert_eq!(
            adapter.file_identity(Path::new("/dummy")),
            Err(PlatformError::Unsupported("file_identity"))
        );
        assert_eq!(
            adapter.canonicalize_and_normalize(Path::new("/dummy")),
            Err(PlatformError::Unsupported("canonicalize_and_normalize"))
        );
        assert_eq!(
            adapter.check_mount_boundary(Path::new("/dummy_a"), Path::new("/dummy_b")),
            Err(PlatformError::Unsupported("check_mount_boundary"))
        );
        assert_eq!(
            adapter.read_entry_metadata(Path::new("/dummy")),
            Err(PlatformError::Unsupported("read_entry_metadata"))
        );
    }

    #[test]
    fn current_platform_name_dispatches_through_adapter() {
        assert_eq!(
            current_platform_name(),
            current_platform_adapter().platform_name()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_adapter_filesystem_identity_on_disposable_fixture() {
        let temp_dir = std::env::temp_dir().join(format!("dc_test_id_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("test_file.txt");
        std::fs::write(&file_path, b"test").unwrap();

        let adapter = MacOsAdapter;
        let id = adapter.file_identity(&file_path).unwrap();
        assert!(id.device_id > 0);
        assert!(id.inode > 0);

        let resolved = adapter.canonicalize_and_normalize(&file_path).unwrap();
        assert_eq!(resolved.original, file_path);
        assert!(!resolved.normalized.as_os_str().is_empty());

        let boundary = adapter.check_mount_boundary(&temp_dir, &file_path).unwrap();
        assert!(!boundary.crosses_boundary);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
