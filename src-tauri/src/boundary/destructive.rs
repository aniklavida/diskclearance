use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::boundary::cancellation::CancellationRegistry;
use crate::boundary::error::CommandError;
use crate::boundary::plan::PlanRepository;
use crate::platform::{EntryType, PlatformAdapter, PlatformError};
use crate::storage::AppDatabase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ActionMode {
    Trash,
    PermanentDelete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutePlanArgs {
    pub plan_id: String,
    pub action_mode: ActionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ItemOutcomeStatus {
    Succeeded,
    Failed,
    SkippedProtected,
    BlockedChanged,
    Vanished,
    PermissionDenied,
    Unattempted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ItemOutcomeRecord {
    pub item_id: String,
    pub original_path: String,
    pub status: ItemOutcomeStatus,
    pub bytes_reclaimed: u64,
    pub bytes_pending_trash: u64,
    pub error_message: Option<String>,
    #[serde(default)]
    pub trashed_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSummary {
    pub plan_id: String,
    pub action_mode: ActionMode,
    pub succeeded_items: u64,
    pub failed_items: u64,
    pub bytes_freed: u64,
    pub bytes_pending_trash: u64,
    pub skipped_protected: u64,
    pub blocked_changed: u64,
    pub vanished_items: u64,
    pub permission_denied_items: u64,
    pub item_outcomes: Vec<ItemOutcomeRecord>,
    #[serde(default)]
    pub operation_id: Option<String>,
}

fn is_item_protected(
    adapter: &dyn PlatformAdapter,
    original_path: &Path,
    canonical_path: &Path,
) -> bool {
    let home = adapter.home_directory().ok().map(|d| d.path);
    let app_support = adapter.application_support_directory().ok().map(|d| d.path);
    let caches = adapter.caches_directory().ok().map(|d| d.path);
    let is_symlink = original_path.is_symlink();
    let symlink_target_canonical = if is_symlink {
        original_path
            .read_link()
            .ok()
            .and_then(|t| t.canonicalize().ok())
    } else {
        None
    };

    let resolved = adapter
        .canonicalize_and_normalize(original_path)
        .unwrap_or_else(|_| crate::platform::ResolvedPath {
            original: original_path.to_path_buf(),
            canonical: canonical_path.to_path_buf(),
            normalized: canonical_path.to_path_buf(),
            is_firmlink_alias: false,
        });

    let identity = adapter
        .file_identity(original_path)
        .unwrap_or(crate::platform::FileIdentity {
            device_id: 0,
            inode: 0,
        });

    let match_ctx = crate::classify::matcher::MatchContext {
        original_path,
        canonical_path: &resolved.canonical,
        normalized_path: &resolved.normalized,
        entry_type: if is_symlink {
            EntryType::Symlink
        } else if original_path.is_dir() {
            EntryType::Directory
        } else {
            EntryType::File
        },
        identity,
        apparent_size: 0,
        allocated_size: 0,
        modified_ms: None,
        is_symlink,
        symlink_target_canonical,
        home_dir: home.as_deref(),
        app_support_dir: app_support.as_deref(),
        caches_dir: caches.as_deref(),
        crosses_mount_boundary: false,
        inside_git_repo: false,
        is_git_internal: false,
    };

    crate::classify::protected::evaluate_protected_roots(&match_ctx).is_some()
}

fn delete_directory_recursively(
    dir_path: &Path,
    root_device_id: u64,
    current_depth: usize,
    max_depth: usize,
) -> Result<(), std::io::Error> {
    if current_depth > max_depth {
        return Err(std::io::Error::other(format!(
            "Maximum directory recursion depth of {max_depth} exceeded at '{}'",
            dir_path.display()
        )));
    }

    for entry_res in std::fs::read_dir(dir_path)? {
        let entry = entry_res?;
        let entry_path = entry.path();
        let symlink_meta = std::fs::symlink_metadata(&entry_path)?;

        // Mount boundary guard: prevent crossing onto a different volume/device
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if symlink_meta.dev() != root_device_id {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!(
                        "Refusing to cross mount boundary into device {} at '{}'",
                        symlink_meta.dev(),
                        entry_path.display()
                    ),
                ));
            }
        }

        // Symlink guard: remove symlink node ONLY; NEVER traverse into target
        if symlink_meta.file_type().is_symlink() {
            std::fs::remove_file(&entry_path)?;
        } else if symlink_meta.is_dir() {
            delete_directory_recursively(
                &entry_path,
                root_device_id,
                current_depth + 1,
                max_depth,
            )?;
            std::fs::remove_dir(&entry_path)?;
        } else {
            // Direct syscall removal without shell
            std::fs::remove_file(&entry_path)?;
        }
    }
    Ok(())
}

pub fn execute_plan_core(
    args: ExecutePlanArgs,
    database: &AppDatabase,
    adapter: &dyn PlatformAdapter,
    cancellations: Option<&CancellationRegistry>,
) -> Result<ExecutionSummary, CommandError> {
    let conn = database.connection().lock().unwrap();

    // 1. Verify plan exists
    let plan = PlanRepository::fetch_plan(&conn, &args.plan_id)
        .map_err(|e| CommandError::Unsupported {
            feature: "execute_plan".into(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| CommandError::Unsupported {
            feature: "execute_plan".into(),
            reason: format!("Plan '{}' not found", args.plan_id),
        })?;

    let items = PlanRepository::fetch_plan_items(&conn, &args.plan_id).map_err(|e| {
        CommandError::Unsupported {
            feature: "execute_plan".into(),
            reason: e.to_string(),
        }
    })?;
    drop(conn);

    // 2. Plan-level safety check: if any item in the plan is protected, abort before ANY execution
    for item in &items {
        if is_item_protected(adapter, &item.original_path, &item.canonical_path)
            || item.class_name.eq_ignore_ascii_case("Protected")
        {
            return Err(CommandError::PermissionDenied {
                path: Some(item.original_path.to_string_lossy().to_string()),
                reason: format!(
                    "Plan contains protected root '{}'; execution aborted before any modification",
                    item.original_path.display()
                ),
            });
        }
    }

    let cancel_token = cancellations.map(|reg| {
        reg.get(&args.plan_id)
            .unwrap_or_else(|| reg.register(&args.plan_id))
    });

    let started_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut item_outcomes = Vec::new();
    let mut succeeded_items = 0u64;
    let mut failed_items = 0u64;
    let mut bytes_freed = 0u64;
    let mut bytes_pending_trash = 0u64;
    let mut skipped_protected = 0u64;
    let mut blocked_changed = 0u64;
    let mut vanished_items = 0u64;
    let mut permission_denied_items = 0u64;

    for (idx, item) in items.iter().enumerate() {
        // Inspect cancellation token before each item
        if let Some(token) = &cancel_token
            && token.is_cancelled()
        {
            for remaining in &items[idx..] {
                item_outcomes.push(ItemOutcomeRecord {
                    item_id: remaining.item_id.clone(),
                    original_path: remaining.original_path.to_string_lossy().to_string(),
                    status: ItemOutcomeStatus::Unattempted,
                    bytes_reclaimed: 0,
                    bytes_pending_trash: 0,
                    error_message: Some("Operation cancelled before item execution".to_string()),
                    trashed_path: None,
                });
            }
            break;
        }

        let orig_path_str = item.original_path.to_string_lossy().to_string();

        // 3. Immediate last-moment pre-execution revalidation of this specific item
        let metadata = match adapter.read_entry_metadata(&item.original_path) {
            Ok(m) => m,
            Err(PlatformError::NotFound(_)) => {
                vanished_items += 1;
                item_outcomes.push(ItemOutcomeRecord {
                    item_id: item.item_id.clone(),
                    original_path: orig_path_str,
                    status: ItemOutcomeStatus::Vanished,
                    bytes_reclaimed: 0,
                    bytes_pending_trash: 0,
                    error_message: Some("Path vanished before execution".to_string()),
                    trashed_path: None,
                });
                continue;
            }
            Err(PlatformError::PermissionDenied { reason, .. }) => {
                failed_items += 1;
                permission_denied_items += 1;
                item_outcomes.push(ItemOutcomeRecord {
                    item_id: item.item_id.clone(),
                    original_path: orig_path_str,
                    status: ItemOutcomeStatus::PermissionDenied,
                    bytes_reclaimed: 0,
                    bytes_pending_trash: 0,
                    error_message: Some(reason),
                    trashed_path: None,
                });
                continue;
            }
            Err(err) => {
                failed_items += 1;
                item_outcomes.push(ItemOutcomeRecord {
                    item_id: item.item_id.clone(),
                    original_path: orig_path_str,
                    status: ItemOutcomeStatus::Failed,
                    bytes_reclaimed: 0,
                    bytes_pending_trash: 0,
                    error_message: Some(err.to_string()),
                    trashed_path: None,
                });
                continue;
            }
        };

        // Inode and device check (mandatory identity verification)
        if metadata.identity.device_id != item.device_id || metadata.identity.inode != item.inode {
            blocked_changed += 1;
            item_outcomes.push(ItemOutcomeRecord {
                item_id: item.item_id.clone(),
                original_path: orig_path_str,
                status: ItemOutcomeStatus::BlockedChanged,
                bytes_reclaimed: 0,
                bytes_pending_trash: 0,
                error_message: Some(format!(
                    "Identity changed: expected device {} inode {}, got device {} inode {}",
                    item.device_id,
                    item.inode,
                    metadata.identity.device_id,
                    metadata.identity.inode
                )),
                trashed_path: None,
            });
            continue;
        }

        // Canonical path check
        let resolved = match adapter.canonicalize_and_normalize(&item.original_path) {
            Ok(r) => r,
            Err(err) => {
                failed_items += 1;
                item_outcomes.push(ItemOutcomeRecord {
                    item_id: item.item_id.clone(),
                    original_path: orig_path_str,
                    status: ItemOutcomeStatus::Failed,
                    bytes_reclaimed: 0,
                    bytes_pending_trash: 0,
                    error_message: Some(err.to_string()),
                    trashed_path: None,
                });
                continue;
            }
        };

        if resolved.canonical != item.canonical_path {
            blocked_changed += 1;
            item_outcomes.push(ItemOutcomeRecord {
                item_id: item.item_id.clone(),
                original_path: orig_path_str,
                status: ItemOutcomeStatus::BlockedChanged,
                bytes_reclaimed: 0,
                bytes_pending_trash: 0,
                error_message: Some(format!(
                    "Canonical path shifted from '{}' to '{}'",
                    item.canonical_path.display(),
                    resolved.canonical.display()
                )),
                trashed_path: None,
            });
            continue;
        }

        // Target swapped with a symlink check
        if metadata.entry_type == EntryType::Symlink && item.class_name != "Symlink" {
            blocked_changed += 1;
            item_outcomes.push(ItemOutcomeRecord {
                item_id: item.item_id.clone(),
                original_path: orig_path_str,
                status: ItemOutcomeStatus::BlockedChanged,
                bytes_reclaimed: 0,
                bytes_pending_trash: 0,
                error_message: Some("Target was replaced by a symlink after review".to_string()),
                trashed_path: None,
            });
            continue;
        }

        // Re-check protected roots
        if is_item_protected(adapter, &item.original_path, &resolved.canonical) {
            skipped_protected += 1;
            item_outcomes.push(ItemOutcomeRecord {
                item_id: item.item_id.clone(),
                original_path: orig_path_str,
                status: ItemOutcomeStatus::SkippedProtected,
                bytes_reclaimed: 0,
                bytes_pending_trash: 0,
                error_message: Some("Target matched a protected root descriptor".to_string()),
                trashed_path: None,
            });
            continue;
        }

        // 4. Perform vetted execution action
        match args.action_mode {
            ActionMode::Trash => match adapter.move_to_trash(&item.original_path) {
                Ok(trashed) => {
                    succeeded_items += 1;
                    bytes_pending_trash += item.size_bytes;
                    item_outcomes.push(ItemOutcomeRecord {
                        item_id: item.item_id.clone(),
                        original_path: orig_path_str,
                        status: ItemOutcomeStatus::Succeeded,
                        bytes_reclaimed: 0,
                        bytes_pending_trash: item.size_bytes,
                        error_message: None,
                        trashed_path: Some(trashed.trashed_path.to_string_lossy().to_string()),
                    });
                }
                Err(PlatformError::PermissionDenied { reason, .. }) => {
                    failed_items += 1;
                    permission_denied_items += 1;
                    item_outcomes.push(ItemOutcomeRecord {
                        item_id: item.item_id.clone(),
                        original_path: orig_path_str,
                        status: ItemOutcomeStatus::PermissionDenied,
                        bytes_reclaimed: 0,
                        bytes_pending_trash: 0,
                        error_message: Some(reason),
                        trashed_path: None,
                    });
                }
                Err(err) => {
                    failed_items += 1;
                    item_outcomes.push(ItemOutcomeRecord {
                        item_id: item.item_id.clone(),
                        original_path: orig_path_str,
                        status: ItemOutcomeStatus::Failed,
                        bytes_reclaimed: 0,
                        bytes_pending_trash: 0,
                        error_message: Some(err.to_string()),
                        trashed_path: None,
                    });
                }
            },
            ActionMode::PermanentDelete => {
                let delete_result = if metadata.entry_type == EntryType::Symlink {
                    // Symlink node removal: direct unlink without following
                    std::fs::remove_file(&item.original_path)
                } else if item.original_path.is_dir() {
                    delete_directory_recursively(
                        &item.original_path,
                        metadata.identity.device_id,
                        0,
                        64,
                    )
                    .and_then(|_| std::fs::remove_dir(&item.original_path))
                } else {
                    std::fs::remove_file(&item.original_path)
                };

                match delete_result {
                    Ok(()) => {
                        succeeded_items += 1;
                        bytes_freed += item.size_bytes;
                        item_outcomes.push(ItemOutcomeRecord {
                            item_id: item.item_id.clone(),
                            original_path: orig_path_str,
                            status: ItemOutcomeStatus::Succeeded,
                            bytes_reclaimed: item.size_bytes,
                            bytes_pending_trash: 0,
                            error_message: None,
                            trashed_path: None,
                        });
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                        failed_items += 1;
                        permission_denied_items += 1;
                        item_outcomes.push(ItemOutcomeRecord {
                            item_id: item.item_id.clone(),
                            original_path: orig_path_str,
                            status: ItemOutcomeStatus::PermissionDenied,
                            bytes_reclaimed: 0,
                            bytes_pending_trash: 0,
                            error_message: Some(err.to_string()),
                            trashed_path: None,
                        });
                    }
                    Err(err) => {
                        failed_items += 1;
                        item_outcomes.push(ItemOutcomeRecord {
                            item_id: item.item_id.clone(),
                            original_path: orig_path_str,
                            status: ItemOutcomeStatus::Failed,
                            bytes_reclaimed: 0,
                            bytes_pending_trash: 0,
                            error_message: Some(err.to_string()),
                            trashed_path: None,
                        });
                    }
                }
            }
        }
    }

    let _ = plan;

    let completed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let op_id = format!("op-{}", crate::scan::session::generate_session_id());
    let op_summary = crate::boundary::history::HistoryOperationSummary {
        id: op_id.clone(),
        plan_id: args.plan_id.clone(),
        action_mode: args.action_mode,
        created_at_ms: started_at_ms,
        completed_at_ms,
        succeeded_items,
        failed_items,
        skipped_items: skipped_protected,
        blocked_items: blocked_changed,
        vanished_items,
        permission_denied_items,
        bytes_pending_trash: if args.action_mode == ActionMode::Trash {
            bytes_pending_trash
        } else {
            0
        },
        bytes_permanently_reclaimed: if args.action_mode == ActionMode::PermanentDelete {
            bytes_freed
        } else {
            0
        },
        total_items: items.len() as u64,
    };

    let mut persisted_items = Vec::new();
    for outcome in &item_outcomes {
        if let Some(matching_item) = items.iter().find(|i| i.item_id == outcome.item_id) {
            persisted_items.push(crate::boundary::history::PersistedOperationItem {
                id: format!("op-item-{}", crate::scan::session::generate_session_id()),
                operation_id: op_id.clone(),
                item_id: matching_item.item_id.clone(),
                original_path: matching_item.original_path.clone(),
                canonical_path: matching_item.canonical_path.clone(),
                trashed_path: outcome.trashed_path.as_ref().map(std::path::PathBuf::from),
                device_id: matching_item.device_id,
                inode: matching_item.inode,
                size_bytes: matching_item.size_bytes,
                status: outcome.status,
                bytes_pending_trash: outcome.bytes_pending_trash,
                bytes_permanently_reclaimed: outcome.bytes_reclaimed,
                error_message: outcome.error_message.clone(),
                created_at_ms: completed_at_ms,
            });
        }
    }

    if let Ok(conn) = database.connection().lock() {
        let _ = crate::boundary::history::OperationRepository::insert_operation(
            &conn,
            &op_summary,
            &persisted_items,
        );
    }

    Ok(ExecutionSummary {
        plan_id: args.plan_id,
        action_mode: args.action_mode,
        succeeded_items,
        failed_items,
        bytes_freed: if args.action_mode == ActionMode::Trash {
            0
        } else {
            bytes_freed
        },
        bytes_pending_trash: if args.action_mode == ActionMode::Trash {
            bytes_pending_trash
        } else {
            0
        },
        skipped_protected,
        blocked_changed,
        vanished_items,
        permission_denied_items,
        item_outcomes,
        operation_id: Some(op_id),
    })
}

#[tauri::command]
pub fn execute_plan(
    args: ExecutePlanArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
    cancellations: tauri::State<'_, CancellationRegistry>,
) -> Result<ExecutionSummary, CommandError> {
    execute_plan_core(
        args,
        database.inner(),
        adapter.inner().as_ref(),
        Some(cancellations.inner()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructive_execute_plan_signature_does_not_accept_path() {
        let args = ExecutePlanArgs {
            plan_id: "plan-xyz-789".into(),
            action_mode: ActionMode::Trash,
        };

        // Assert serialized representation only contains planId and actionMode
        let value = serde_json::to_value(&args).expect("to_value");
        let map = value.as_object().expect("object");
        assert!(map.contains_key("planId"));
        assert!(map.contains_key("actionMode"));
        assert_eq!(
            map.len(),
            2,
            "ExecutePlanArgs must strictly only contain planId and actionMode; paths are strictly prohibited"
        );

        // Verify TypeScript declaration does not contain 'path'
        let cfg = ts_rs::Config::default();
        let decl = ExecutePlanArgs::decl(&cfg);
        assert!(
            !decl.to_lowercase().contains("path"),
            "ExecutePlanArgs TypeScript signature must not contain path fields"
        );
        assert!(decl.contains("planId: string"));
        assert!(decl.contains("actionMode: ActionMode"));
    }
}
