use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::classify::matcher::MatchContext;

/// Structured explanation of why an item is protected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedReason {
    pub root_id: String,
    pub root_name: String,
    pub description: String,
}

/// Category of protected path in the macOS operating environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ProtectedCategory {
    SystemVolume,
    SystemDirectory,
    LibrarySystemComponent,
    CredentialStore,
    CryptoMaterial,
    SourceControl,
    CloudSync,
    TimeMachineSnapshot,
    DurableAppState,
}

/// Declarative description of a protected root path or scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedRootDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub category: ProtectedCategory,
    pub description: &'static str,
    /// Exact canonical path if fixed on system.
    pub exact_system_path: Option<&'static str>,
    /// System path prefix (matches all subpaths).
    pub system_prefix: Option<&'static str>,
    /// Path relative to user home directory.
    pub home_relative_prefix: Option<&'static str>,
    /// Exact path relative to user home directory.
    pub home_relative_exact: Option<&'static str>,
}

impl ProtectedRootDescriptor {
    pub fn matches(&self, ctx: &MatchContext<'_>) -> bool {
        // 1. Check exact system path
        if let Some(exact) = self.exact_system_path {
            let path = Path::new(exact);
            if ctx.canonical_path == path || ctx.normalized_path == path {
                return true;
            }
        }

        // 2. Check system prefix
        if let Some(prefix) = self.system_prefix {
            let prefix_path = Path::new(prefix);
            if ctx.canonical_path.starts_with(prefix_path)
                || ctx.normalized_path.starts_with(prefix_path)
            {
                return true;
            }
        }

        // 3. Check user home relative prefix
        if let Some(home) = ctx.home_dir {
            let home_canon = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
            if let Some(rel) = self.home_relative_prefix {
                let full = home.join(rel);
                let full_canon = home_canon.join(rel);
                if ctx.canonical_path.starts_with(&full)
                    || ctx.canonical_path.starts_with(&full_canon)
                    || ctx.normalized_path.starts_with(&full)
                    || ctx.normalized_path.starts_with(&full_canon)
                {
                    return true;
                }
            }

            // 4. Check user home relative exact
            if let Some(rel) = self.home_relative_exact {
                let full = home.join(rel);
                let full_canon = home_canon.join(rel);
                if ctx.canonical_path == full
                    || ctx.canonical_path == full_canon
                    || ctx.normalized_path == full
                    || ctx.normalized_path == full_canon
                {
                    return true;
                }
            }
        }

        // 5. Check special categories (e.g. source control)
        if self.category == ProtectedCategory::SourceControl && ctx.is_git_internal {
            return true;
        }

        // 6. Check symlink targets: if a symlink resolves to a protected location
        if ctx.is_symlink
            && let Some(target) = &ctx.symlink_target_canonical
        {
            if let Some(exact) = self.exact_system_path {
                let path = Path::new(exact);
                if target == path {
                    return true;
                }
            }
            if let Some(prefix) = self.system_prefix {
                let prefix_path = Path::new(prefix);
                if target.starts_with(prefix_path) {
                    return true;
                }
            }
            if let Some(home) = ctx.home_dir {
                let home_canon = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
                if let Some(rel) = self.home_relative_prefix {
                    let full = home.join(rel);
                    let full_canon = home_canon.join(rel);
                    if target.starts_with(&full) || target.starts_with(&full_canon) {
                        return true;
                    }
                }
                if let Some(rel) = self.home_relative_exact {
                    let full = home.join(rel);
                    let full_canon = home_canon.join(rel);
                    if target == &full || target == &full_canon {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Declarative collection of all standard protected roots.
pub const PROTECTED_ROOTS: &[ProtectedRootDescriptor] = &[
    // 1. System and firmware volumes
    ProtectedRootDescriptor {
        id: "protected.system.root",
        name: "macOS System Root Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Read-only root filesystem sealed by APFS snapshot integrity verification.",
        exact_system_path: Some("/"),
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.preboot",
        name: "System Preboot Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Firmware boot assets and bootloader payloads.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/Preboot"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.update",
        name: "System Update Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Operating system staging volume for system updates.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/Update"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.vm",
        name: "Virtual Memory Swap Volume",
        category: ProtectedCategory::SystemVolume,
        description: "macOS system swap and encrypted paging files.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/VM"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.hardware",
        name: "Hardware and Secure Enclave Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Platform secure enclave and hardware support data.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/Hardware"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.recovery",
        name: "macOS Recovery Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Recovery operating system and diagnostic tools.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/Recovery"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.xarts",
        name: "Apple Silicon xarts Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Secure Enclave cross-architecture runtime storage.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/xarts"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.system.iscpreboot",
        name: "iSC Preboot Volume",
        category: ProtectedCategory::SystemVolume,
        description: "Apple Silicon secondary boot and recovery descriptors.",
        exact_system_path: None,
        system_prefix: Some("/System/Volumes/iSCPreboot"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    // 2. /System
    ProtectedRootDescriptor {
        id: "protected.system.directory",
        name: "/System Directory",
        category: ProtectedCategory::SystemDirectory,
        description: "Sealed core macOS operating system components and binaries.",
        exact_system_path: None,
        system_prefix: Some("/System"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    // 3. /Library system components
    ProtectedRootDescriptor {
        id: "protected.library.system_extensions",
        name: "System Extensions",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "Installed macOS driver and network system extensions.",
        exact_system_path: None,
        system_prefix: Some("/Library/SystemExtensions"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.kernel_extensions",
        name: "Kernel Extensions",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "System kernel extensions and low-level drivers.",
        exact_system_path: None,
        system_prefix: Some("/Library/KernelExtensions"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.preferences",
        name: "System Preferences",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "Machine-wide system configuration property lists.",
        exact_system_path: None,
        system_prefix: Some("/Library/Preferences"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.security",
        name: "System Security Components",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "Security authorization plugins, trust evaluation, and certificates.",
        exact_system_path: None,
        system_prefix: Some("/Library/Security"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.keychains",
        name: "System Keychains",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "System-wide cryptographic trust store and root certificates.",
        exact_system_path: None,
        system_prefix: Some("/Library/Keychains"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.apple",
        name: "Apple Platform Support",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "Critical vendor system support files.",
        exact_system_path: None,
        system_prefix: Some("/Library/Apple"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.library.directory_services",
        name: "Directory Services",
        category: ProtectedCategory::LibrarySystemComponent,
        description: "Local directory node and user account database components.",
        exact_system_path: None,
        system_prefix: Some("/Library/DirectoryServices"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    // 4. User Keychain and credential stores
    ProtectedRootDescriptor {
        id: "protected.user.keychains",
        name: "User Keychain Store",
        category: ProtectedCategory::CredentialStore,
        description: "Encrypted passwords, secrets, private keys, and application credentials.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Library/Keychains"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.user.aws_credentials",
        name: "AWS Credentials",
        category: ProtectedCategory::CredentialStore,
        description: "AWS cloud access keys, session tokens, and account configuration.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".aws"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.user.azure_credentials",
        name: "Azure Credentials",
        category: ProtectedCategory::CredentialStore,
        description: "Microsoft Azure authentication tokens and service principal profiles.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".azure"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.user.gcloud_credentials",
        name: "Google Cloud Credentials",
        category: ProtectedCategory::CredentialStore,
        description: "Google Cloud SDK application default credentials and authentication tokens.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".config/gcloud"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.user.docker_credentials",
        name: "Docker Authentication Store",
        category: ProtectedCategory::CredentialStore,
        description: "Container registry authentication tokens and credentials.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: Some(".docker/config.json"),
    },
    ProtectedRootDescriptor {
        id: "protected.user.kube_credentials",
        name: "Kubernetes Credentials",
        category: ProtectedCategory::CredentialStore,
        description: "Cluster configuration, client certificates, and bearer tokens.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".kube"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.user.npmrc",
        name: "npm Configuration and Auth Tokens",
        category: ProtectedCategory::CredentialStore,
        description: "Package registry access tokens and private registry credentials.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: Some(".npmrc"),
    },
    ProtectedRootDescriptor {
        id: "protected.user.netrc",
        name: "Network Credentials (.netrc)",
        category: ProtectedCategory::CredentialStore,
        description: "Plaintext network login credentials and machine authentication records.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: Some(".netrc"),
    },
    ProtectedRootDescriptor {
        id: "protected.user.git_credentials",
        name: "Git Credential Store",
        category: ProtectedCategory::CredentialStore,
        description: "Cached passwords and personal access tokens for remote repositories.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: Some(".git-credentials"),
    },
    ProtectedRootDescriptor {
        id: "protected.user.cargo_credentials",
        name: "Cargo Registry Tokens",
        category: ProtectedCategory::CredentialStore,
        description: "Crates.io and private registry publishing tokens.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: Some(".cargo/credentials.toml"),
    },
    // 5. SSH and GPG material
    ProtectedRootDescriptor {
        id: "protected.crypto.ssh",
        name: "SSH Key Material",
        category: ProtectedCategory::CryptoMaterial,
        description: "Secure Shell private keys, identity files, and known host signatures.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".ssh"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.crypto.gnupg",
        name: "GnuPG Keyrings and Trust Database",
        category: ProtectedCategory::CryptoMaterial,
        description: "PGP secret keys, public keyrings, and trust database.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some(".gnupg"),
        home_relative_exact: None,
    },
    // 6. Source-control working trees and their .git directories
    ProtectedRootDescriptor {
        id: "protected.source_control.git_internal",
        name: "Git Repository Internal State",
        category: ProtectedCategory::SourceControl,
        description: "Git object database, commit history, branch refs, and HEAD pointer.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    // 7. Cloud-sync provider roots
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.icloud_drive",
        name: "iCloud Drive Document Root",
        category: ProtectedCategory::CloudSync,
        description: "Local cache and synchronization container for Apple iCloud Drive.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Library/Mobile Documents/com~apple~CloudDocs"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.dropbox",
        name: "Dropbox Sync Root",
        category: ProtectedCategory::CloudSync,
        description: "User synchronized cloud storage managed by Dropbox.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Dropbox"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.onedrive",
        name: "OneDrive Sync Root",
        category: ProtectedCategory::CloudSync,
        description: "User synchronized cloud storage managed by Microsoft OneDrive.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("OneDrive"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.google_drive",
        name: "Google Drive Sync Root",
        category: ProtectedCategory::CloudSync,
        description: "User synchronized cloud storage managed by Google Drive.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Google Drive"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.nextcloud",
        name: "Nextcloud Sync Root",
        category: ProtectedCategory::CloudSync,
        description: "User synchronized cloud storage managed by Nextcloud.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Nextcloud"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.cloud_sync.box",
        name: "Box Sync Root",
        category: ProtectedCategory::CloudSync,
        description: "User synchronized cloud storage managed by Box.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Box"),
        home_relative_exact: None,
    },
    // 8. Time Machine local snapshots
    ProtectedRootDescriptor {
        id: "protected.time_machine.snapshots",
        name: "Time Machine Local Snapshots",
        category: ProtectedCategory::TimeMachineSnapshot,
        description: "APFS local snapshots created for Time Machine incremental recovery.",
        exact_system_path: None,
        system_prefix: Some("/.MobileBackups"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.time_machine.volumes",
        name: "Time Machine Volume Snapshots",
        category: ProtectedCategory::TimeMachineSnapshot,
        description: "Mounted Time Machine volume backup chains and catalogs.",
        exact_system_path: None,
        system_prefix: Some("/Volumes/.timemachine"),
        home_relative_prefix: None,
        home_relative_exact: None,
    },
    // 9. Durable application state a tool cannot regenerate
    ProtectedRootDescriptor {
        id: "protected.durable.apple_notes",
        name: "Apple Notes Database",
        category: ProtectedCategory::DurableAppState,
        description: "User notes, attachments, and non-rebuildable local SQLite databases.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Library/Group Containers/group.com.apple.notes"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.durable.mail",
        name: "Apple Mail Local Store",
        category: ProtectedCategory::DurableAppState,
        description: "Local mailboxes, cached drafts, and email messages.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Library/Mail"),
        home_relative_exact: None,
    },
    ProtectedRootDescriptor {
        id: "protected.durable.messages",
        name: "Apple Messages Database",
        category: ProtectedCategory::DurableAppState,
        description: "iMessage chat history, attachments, and conversation threads.",
        exact_system_path: None,
        system_prefix: None,
        home_relative_prefix: Some("Library/Messages"),
        home_relative_exact: None,
    },
];

/// Evaluates if a given item matches any protected root.
///
/// Returns `Some(ProtectedReason)` if protected, or `None` if not protected by this set.
pub fn evaluate_protected_roots(ctx: &MatchContext<'_>) -> Option<ProtectedReason> {
    for root in PROTECTED_ROOTS {
        if root.matches(ctx) {
            return Some(ProtectedReason {
                root_id: root.id.to_string(),
                root_name: root.name.to_string(),
                description: root.description.to_string(),
            });
        }
    }
    None
}
