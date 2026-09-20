use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::boundary::destructive::{ActionMode, ItemOutcomeStatus};
use crate::boundary::error::CommandError;
use crate::platform::PlatformAdapter;
use crate::storage::AppDatabase;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchHistoryOperationsArgs {
    pub outcome_filter: Option<String>,
    pub date_from_ms: Option<u64>,
    pub date_to_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryOperationSummary {
    pub id: String,
    pub plan_id: String,
    pub action_mode: ActionMode,
    pub created_at_ms: u64,
    pub completed_at_ms: u64,
    pub succeeded_items: u64,
    pub failed_items: u64,
    pub skipped_items: u64,
    pub blocked_items: u64,
    pub vanished_items: u64,
    pub permission_denied_items: u64,
    pub bytes_pending_trash: u64,
    pub bytes_permanently_reclaimed: u64,
    pub total_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchOperationDetailArgs {
    pub operation_id: String,
    pub outcome_filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RestoreEligibility {
    Eligible {
        trashed_path: String,
        original_destination: String,
        destination_occupied: bool,
        suggested_alternate_destination: Option<String>,
    },
    Ineligible {
        reason: String,
    },
    Restored {
        restored_to_path: String,
        restored_at_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TrashHistoryItemDetail {
    pub operation_item_id: String,
    pub item_id: String,
    pub original_path: String,
    pub trashed_path: Option<String>,
    pub size_bytes: u64,
    pub status: ItemOutcomeStatus,
    pub bytes_pending_trash: u64,
    pub error_message: Option<String>,
    pub created_at_ms: u64,
    pub restore_eligibility: RestoreEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PermanentDeleteHistoryItemDetail {
    pub operation_item_id: String,
    pub item_id: String,
    pub original_path: String,
    pub size_bytes: u64,
    pub status: ItemOutcomeStatus,
    pub bytes_permanently_reclaimed: u64,
    pub error_message: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "actionType", rename_all = "camelCase")]
pub enum HistoryItemDetail {
    Trash(TrashHistoryItemDetail),
    PermanentDelete(PermanentDeleteHistoryItemDetail),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryOperationDetail {
    pub operation: HistoryOperationSummary,
    pub items: Vec<HistoryItemDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RestoreItemArgs {
    pub operation_item_id: String,
    pub alternate_destination: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RestoreItemSummary {
    pub restore_id: String,
    pub operation_item_id: String,
    pub restored_to_path: String,
    pub size_bytes: u64,
    pub restored_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LifetimeReclamationTotals {
    pub pending_in_trash_bytes: u64,
    pub permanently_reclaimed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedOperationItem {
    pub id: String,
    pub operation_id: String,
    pub item_id: String,
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub trashed_path: Option<PathBuf>,
    pub device_id: u64,
    pub inode: u64,
    pub size_bytes: u64,
    pub status: ItemOutcomeStatus,
    pub bytes_pending_trash: u64,
    pub bytes_permanently_reclaimed: u64,
    pub error_message: Option<String>,
    pub created_at_ms: u64,
}

pub fn suggest_non_colliding_path(original_path: &Path) -> PathBuf {
    let parent = original_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = original_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "restored".to_string());
    let extension = original_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let candidate = parent.join(format!("{stem} (Restored){extension}"));
    if !candidate.exists() && !candidate.is_symlink() {
        return candidate;
    }

    let mut counter = 2;
    loop {
        let candidate = parent.join(format!("{stem} (Restored {counter}){extension}"));
        if !candidate.exists() && !candidate.is_symlink() {
            return candidate;
        }
        counter += 1;
    }
}

pub fn compute_restore_eligibility(
    item: &PersistedOperationItem,
    conn: &rusqlite::Connection,
    adapter: &dyn PlatformAdapter,
) -> RestoreEligibility {
    // 1. Check if item has already been successfully restored
    let already_restored: Option<(String, u64)> = conn
        .query_row(
            "SELECT restored_to_path, restored_at_ms FROM restore_outcomes WHERE operation_item_id = ?1 AND status = 'Succeeded' ORDER BY restored_at_ms DESC LIMIT 1",
            rusqlite::params![item.id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u64)),
        )
        .ok();

    if let Some((restored_to_path, restored_at_ms)) = already_restored {
        return RestoreEligibility::Restored {
            restored_to_path,
            restored_at_ms,
        };
    }

    // 2. If item did not succeed when moved to trash, it is ineligible
    if item.status != ItemOutcomeStatus::Succeeded {
        return RestoreEligibility::Ineligible {
            reason: format!(
                "Item was not successfully moved to Trash (status: {:?})",
                item.status
            ),
        };
    }

    // 3. Must have a recorded trash path
    let trashed_path = match &item.trashed_path {
        Some(p) => p,
        None => {
            return RestoreEligibility::Ineligible {
                reason: "No trash location recorded for this item".to_string(),
            };
        }
    };

    // 4. Trashed item must exist on the filesystem
    if !trashed_path.exists() && !trashed_path.is_symlink() {
        return RestoreEligibility::Ineligible {
            reason: "Item is no longer in Trash (emptied from Trash or moved)".to_string(),
        };
    }

    // 5. Must match exact filesystem identity (device_id and inode)
    let identity = match adapter.file_identity(trashed_path) {
        Ok(id) => id,
        Err(err) => {
            return RestoreEligibility::Ineligible {
                reason: format!("Cannot read file identity in Trash: {err}"),
            };
        }
    };

    if identity.device_id != item.device_id || identity.inode != item.inode {
        return RestoreEligibility::Ineligible {
            reason: format!(
                "File identity in Trash changed (expected device {} inode {}, got device {} inode {})",
                item.device_id, item.inode, identity.device_id, identity.inode
            ),
        };
    }

    // 6. Inspect original destination for collision
    let destination_occupied = item.original_path.exists() || item.original_path.is_symlink();
    let suggested_alternate_destination = if destination_occupied {
        Some(
            suggest_non_colliding_path(&item.original_path)
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };

    RestoreEligibility::Eligible {
        trashed_path: trashed_path.to_string_lossy().to_string(),
        original_destination: item.original_path.to_string_lossy().to_string(),
        destination_occupied,
        suggested_alternate_destination,
    }
}

pub struct OperationRepository;

impl OperationRepository {
    pub fn insert_operation(
        conn: &rusqlite::Connection,
        op: &HistoryOperationSummary,
        items: &[PersistedOperationItem],
    ) -> Result<(), rusqlite::Error> {
        let action_str = match op.action_mode {
            ActionMode::Trash => "Trash",
            ActionMode::PermanentDelete => "PermanentDelete",
        };

        conn.execute(
            "INSERT INTO operations (id, plan_id, action_mode, created_at_ms, completed_at_ms, succeeded_items, failed_items, skipped_items, blocked_items, vanished_items, permission_denied_items, bytes_pending_trash, bytes_permanently_reclaimed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                op.id,
                op.plan_id,
                action_str,
                op.created_at_ms as i64,
                op.completed_at_ms as i64,
                op.succeeded_items as i64,
                op.failed_items as i64,
                op.skipped_items as i64,
                op.blocked_items as i64,
                op.vanished_items as i64,
                op.permission_denied_items as i64,
                op.bytes_pending_trash as i64,
                op.bytes_permanently_reclaimed as i64,
            ],
        )?;

        let mut stmt = conn.prepare(
            "INSERT INTO operation_items (id, operation_id, item_id, original_path, canonical_path, trashed_path, device_id, inode, size_bytes, status, bytes_pending_trash, bytes_permanently_reclaimed, error_message, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )?;

        for item in items {
            let status_str = match item.status {
                ItemOutcomeStatus::Succeeded => "Succeeded",
                ItemOutcomeStatus::Failed => "Failed",
                ItemOutcomeStatus::SkippedProtected => "SkippedProtected",
                ItemOutcomeStatus::BlockedChanged => "BlockedChanged",
                ItemOutcomeStatus::Vanished => "Vanished",
                ItemOutcomeStatus::PermissionDenied => "PermissionDenied",
                ItemOutcomeStatus::Unattempted => "Unattempted",
            };

            let trashed_str = item
                .trashed_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            stmt.execute(rusqlite::params![
                item.id,
                item.operation_id,
                item.item_id,
                item.original_path.to_string_lossy().to_string(),
                item.canonical_path.to_string_lossy().to_string(),
                trashed_str,
                item.device_id as i64,
                item.inode as i64,
                item.size_bytes as i64,
                status_str,
                item.bytes_pending_trash as i64,
                item.bytes_permanently_reclaimed as i64,
                item.error_message,
                item.created_at_ms as i64,
            ])?;
        }

        Ok(())
    }

    pub fn fetch_operations(
        conn: &rusqlite::Connection,
        args: &FetchHistoryOperationsArgs,
    ) -> Result<Vec<HistoryOperationSummary>, rusqlite::Error> {
        let mut sql = "SELECT id, plan_id, action_mode, created_at_ms, completed_at_ms, succeeded_items, failed_items, skipped_items, blocked_items, vanished_items, permission_denied_items, bytes_pending_trash, bytes_permanently_reclaimed FROM operations WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(from) = args.date_from_ms {
            sql.push_str(" AND created_at_ms >= ?");
            params.push(Box::new(from as i64));
        }

        if let Some(to) = args.date_to_ms {
            sql.push_str(" AND created_at_ms <= ?");
            params.push(Box::new(to as i64));
        }

        if let Some(ref filter) = args.outcome_filter {
            match filter.to_lowercase().as_str() {
                "failed" => {
                    sql.push_str(" AND (failed_items > 0 OR permission_denied_items > 0)");
                }
                "succeeded" => {
                    sql.push_str(" AND succeeded_items > 0");
                }
                "skipped" => {
                    sql.push_str(" AND skipped_items > 0");
                }
                "blocked" => {
                    sql.push_str(" AND (blocked_items > 0 OR vanished_items > 0)");
                }
                _ => {}
            }
        }

        sql.push_str(" ORDER BY created_at_ms DESC");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let mode_str: String = row.get(2)?;
            let action_mode = if mode_str == "PermanentDelete" {
                ActionMode::PermanentDelete
            } else {
                ActionMode::Trash
            };
            let succ: u64 = row.get::<_, i64>(5)? as u64;
            let fail: u64 = row.get::<_, i64>(6)? as u64;
            let skip: u64 = row.get::<_, i64>(7)? as u64;
            let blk: u64 = row.get::<_, i64>(8)? as u64;
            let van: u64 = row.get::<_, i64>(9)? as u64;
            let perm: u64 = row.get::<_, i64>(10)? as u64;
            let total = succ + fail + skip + blk + van + perm;

            Ok(HistoryOperationSummary {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                action_mode,
                created_at_ms: row.get::<_, i64>(3)? as u64,
                completed_at_ms: row.get::<_, i64>(4)? as u64,
                succeeded_items: succ,
                failed_items: fail,
                skipped_items: skip,
                blocked_items: blk,
                vanished_items: van,
                permission_denied_items: perm,
                bytes_pending_trash: row.get::<_, i64>(11)? as u64,
                bytes_permanently_reclaimed: row.get::<_, i64>(12)? as u64,
                total_items: total,
            })
        })?;

        let mut ops = Vec::new();
        for r in rows {
            ops.push(r?);
        }
        Ok(ops)
    }

    pub fn fetch_operation_by_id(
        conn: &rusqlite::Connection,
        operation_id: &str,
    ) -> Result<Option<HistoryOperationSummary>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, plan_id, action_mode, created_at_ms, completed_at_ms, succeeded_items, failed_items, skipped_items, blocked_items, vanished_items, permission_denied_items, bytes_pending_trash, bytes_permanently_reclaimed FROM operations WHERE id = ?1",
        )?;

        let mut rows = stmt.query(rusqlite::params![operation_id])?;
        if let Some(row) = rows.next()? {
            let mode_str: String = row.get(2)?;
            let action_mode = if mode_str == "PermanentDelete" {
                ActionMode::PermanentDelete
            } else {
                ActionMode::Trash
            };
            let succ: u64 = row.get::<_, i64>(5)? as u64;
            let fail: u64 = row.get::<_, i64>(6)? as u64;
            let skip: u64 = row.get::<_, i64>(7)? as u64;
            let blk: u64 = row.get::<_, i64>(8)? as u64;
            let van: u64 = row.get::<_, i64>(9)? as u64;
            let perm: u64 = row.get::<_, i64>(10)? as u64;
            let total = succ + fail + skip + blk + van + perm;

            Ok(Some(HistoryOperationSummary {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                action_mode,
                created_at_ms: row.get::<_, i64>(3)? as u64,
                completed_at_ms: row.get::<_, i64>(4)? as u64,
                succeeded_items: succ,
                failed_items: fail,
                skipped_items: skip,
                blocked_items: blk,
                vanished_items: van,
                permission_denied_items: perm,
                bytes_pending_trash: row.get::<_, i64>(11)? as u64,
                bytes_permanently_reclaimed: row.get::<_, i64>(12)? as u64,
                total_items: total,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn fetch_operation_items(
        conn: &rusqlite::Connection,
        operation_id: &str,
    ) -> Result<Vec<PersistedOperationItem>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, operation_id, item_id, original_path, canonical_path, trashed_path, device_id, inode, size_bytes, status, bytes_pending_trash, bytes_permanently_reclaimed, error_message, created_at_ms FROM operation_items WHERE operation_id = ?1 ORDER BY original_path ASC",
        )?;

        let rows = stmt.query_map(rusqlite::params![operation_id], |row| {
            let status_str: String = row.get(9)?;
            let status = match status_str.as_str() {
                "Succeeded" => ItemOutcomeStatus::Succeeded,
                "Failed" => ItemOutcomeStatus::Failed,
                "SkippedProtected" => ItemOutcomeStatus::SkippedProtected,
                "BlockedChanged" => ItemOutcomeStatus::BlockedChanged,
                "Vanished" => ItemOutcomeStatus::Vanished,
                "PermissionDenied" => ItemOutcomeStatus::PermissionDenied,
                _ => ItemOutcomeStatus::Unattempted,
            };

            let trashed_str: Option<String> = row.get(5)?;

            Ok(PersistedOperationItem {
                id: row.get(0)?,
                operation_id: row.get(1)?,
                item_id: row.get(2)?,
                original_path: PathBuf::from(row.get::<_, String>(3)?),
                canonical_path: PathBuf::from(row.get::<_, String>(4)?),
                trashed_path: trashed_str.map(PathBuf::from),
                device_id: row.get::<_, i64>(6)? as u64,
                inode: row.get::<_, i64>(7)? as u64,
                size_bytes: row.get::<_, i64>(8)? as u64,
                status,
                bytes_pending_trash: row.get::<_, i64>(10)? as u64,
                bytes_permanently_reclaimed: row.get::<_, i64>(11)? as u64,
                error_message: row.get(12)?,
                created_at_ms: row.get::<_, i64>(13)? as u64,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }

    pub fn fetch_operation_item_by_id(
        conn: &rusqlite::Connection,
        item_id: &str,
    ) -> Result<Option<PersistedOperationItem>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, operation_id, item_id, original_path, canonical_path, trashed_path, device_id, inode, size_bytes, status, bytes_pending_trash, bytes_permanently_reclaimed, error_message, created_at_ms FROM operation_items WHERE id = ?1",
        )?;

        let mut rows = stmt.query(rusqlite::params![item_id])?;
        if let Some(row) = rows.next()? {
            let status_str: String = row.get(9)?;
            let status = match status_str.as_str() {
                "Succeeded" => ItemOutcomeStatus::Succeeded,
                "Failed" => ItemOutcomeStatus::Failed,
                "SkippedProtected" => ItemOutcomeStatus::SkippedProtected,
                "BlockedChanged" => ItemOutcomeStatus::BlockedChanged,
                "Vanished" => ItemOutcomeStatus::Vanished,
                "PermissionDenied" => ItemOutcomeStatus::PermissionDenied,
                _ => ItemOutcomeStatus::Unattempted,
            };

            let trashed_str: Option<String> = row.get(5)?;

            Ok(Some(PersistedOperationItem {
                id: row.get(0)?,
                operation_id: row.get(1)?,
                item_id: row.get(2)?,
                original_path: PathBuf::from(row.get::<_, String>(3)?),
                canonical_path: PathBuf::from(row.get::<_, String>(4)?),
                trashed_path: trashed_str.map(PathBuf::from),
                device_id: row.get::<_, i64>(6)? as u64,
                inode: row.get::<_, i64>(7)? as u64,
                size_bytes: row.get::<_, i64>(8)? as u64,
                status,
                bytes_pending_trash: row.get::<_, i64>(10)? as u64,
                bytes_permanently_reclaimed: row.get::<_, i64>(11)? as u64,
                error_message: row.get(12)?,
                created_at_ms: row.get::<_, i64>(13)? as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn fetch_all_trash_operation_items(
        conn: &rusqlite::Connection,
    ) -> Result<Vec<PersistedOperationItem>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT oi.id, oi.operation_id, oi.item_id, oi.original_path, oi.canonical_path, oi.trashed_path, oi.device_id, oi.inode, oi.size_bytes, oi.status, oi.bytes_pending_trash, oi.bytes_permanently_reclaimed, oi.error_message, oi.created_at_ms
             FROM operation_items oi
             JOIN operations o ON oi.operation_id = o.id
             WHERE o.action_mode = 'Trash' AND oi.status = 'Succeeded'",
        )?;

        let rows = stmt.query_map([], |row| {
            let trashed_str: Option<String> = row.get(5)?;
            Ok(PersistedOperationItem {
                id: row.get(0)?,
                operation_id: row.get(1)?,
                item_id: row.get(2)?,
                original_path: PathBuf::from(row.get::<_, String>(3)?),
                canonical_path: PathBuf::from(row.get::<_, String>(4)?),
                trashed_path: trashed_str.map(PathBuf::from),
                device_id: row.get::<_, i64>(6)? as u64,
                inode: row.get::<_, i64>(7)? as u64,
                size_bytes: row.get::<_, i64>(8)? as u64,
                status: ItemOutcomeStatus::Succeeded,
                bytes_pending_trash: row.get::<_, i64>(10)? as u64,
                bytes_permanently_reclaimed: row.get::<_, i64>(11)? as u64,
                error_message: row.get(12)?,
                created_at_ms: row.get::<_, i64>(13)? as u64,
            })
        })?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r?);
        }
        Ok(items)
    }
}

pub fn fetch_history_operations_core(
    args: FetchHistoryOperationsArgs,
    database: &AppDatabase,
) -> Result<Vec<HistoryOperationSummary>, CommandError> {
    let conn = database
        .connection()
        .lock()
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_history_operations".into(),
            reason: e.to_string(),
        })?;

    OperationRepository::fetch_operations(&conn, &args).map_err(|e| CommandError::Unsupported {
        feature: "fetch_history_operations".into(),
        reason: e.to_string(),
    })
}

pub fn fetch_operation_detail_core(
    args: FetchOperationDetailArgs,
    database: &AppDatabase,
    adapter: &dyn PlatformAdapter,
) -> Result<HistoryOperationDetail, CommandError> {
    let conn = database
        .connection()
        .lock()
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_operation_detail".into(),
            reason: e.to_string(),
        })?;

    let op = OperationRepository::fetch_operation_by_id(&conn, &args.operation_id)
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_operation_detail".into(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| CommandError::Unsupported {
            feature: "fetch_operation_detail".into(),
            reason: format!("Operation '{}' not found", args.operation_id),
        })?;

    let persisted_items = OperationRepository::fetch_operation_items(&conn, &args.operation_id)
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_operation_detail".into(),
            reason: e.to_string(),
        })?;

    let mut filtered_items = Vec::new();
    for item in persisted_items {
        if let Some(ref filter) = args.outcome_filter {
            let matches = match filter.to_lowercase().as_str() {
                "succeeded" => item.status == ItemOutcomeStatus::Succeeded,
                "failed" => {
                    item.status == ItemOutcomeStatus::Failed
                        || item.status == ItemOutcomeStatus::PermissionDenied
                }
                "skipped" => item.status == ItemOutcomeStatus::SkippedProtected,
                "blocked" => {
                    item.status == ItemOutcomeStatus::BlockedChanged
                        || item.status == ItemOutcomeStatus::Vanished
                }
                _ => true,
            };
            if !matches {
                continue;
            }
        }

        match op.action_mode {
            ActionMode::Trash => {
                let restore_eligibility = compute_restore_eligibility(&item, &conn, adapter);
                filtered_items.push(HistoryItemDetail::Trash(TrashHistoryItemDetail {
                    operation_item_id: item.id,
                    item_id: item.item_id,
                    original_path: item.original_path.to_string_lossy().to_string(),
                    trashed_path: item.trashed_path.map(|p| p.to_string_lossy().to_string()),
                    size_bytes: item.size_bytes,
                    status: item.status,
                    bytes_pending_trash: item.bytes_pending_trash,
                    error_message: item.error_message,
                    created_at_ms: item.created_at_ms,
                    restore_eligibility,
                }));
            }
            ActionMode::PermanentDelete => {
                filtered_items.push(HistoryItemDetail::PermanentDelete(
                    PermanentDeleteHistoryItemDetail {
                        operation_item_id: item.id,
                        item_id: item.item_id,
                        original_path: item.original_path.to_string_lossy().to_string(),
                        size_bytes: item.size_bytes,
                        status: item.status,
                        bytes_permanently_reclaimed: item.bytes_permanently_reclaimed,
                        error_message: item.error_message,
                        created_at_ms: item.created_at_ms,
                    },
                ));
            }
        }
    }

    Ok(HistoryOperationDetail {
        operation: op,
        items: filtered_items,
    })
}

pub fn restore_item_core(
    args: RestoreItemArgs,
    database: &AppDatabase,
    adapter: &dyn PlatformAdapter,
) -> Result<RestoreItemSummary, CommandError> {
    let conn = database
        .connection()
        .lock()
        .map_err(|e| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: e.to_string(),
        })?;

    // 1. Fetch operation item
    let item = OperationRepository::fetch_operation_item_by_id(&conn, &args.operation_item_id)
        .map_err(|e| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: e.to_string(),
        })?
        .ok_or_else(|| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: format!("Operation item '{}' not found", args.operation_item_id),
        })?;

    // 2. Structurally exclude permanent delete
    let op_action_mode: String = conn
        .query_row(
            "SELECT action_mode FROM operations WHERE id = ?1",
            rusqlite::params![item.operation_id],
            |row| row.get(0),
        )
        .map_err(|e| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: e.to_string(),
        })?;

    if op_action_mode != "Trash" {
        return Err(CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: "Permanently deleted items cannot be restored. History records permanent deletion as audit evidence only; destroyed bytes are unrecoverable.".into(),
        });
    }

    // 3. Check if already restored
    let already_restored: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM restore_outcomes WHERE operation_item_id = ?1 AND status = 'Succeeded')",
            rusqlite::params![item.id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if already_restored {
        return Err(CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: "Item has already been restored".into(),
        });
    }

    // 4. Validate trashed path and identity
    let trashed_path = item
        .trashed_path
        .as_ref()
        .ok_or_else(|| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: "No trash location recorded for this item".into(),
        })?;

    if !trashed_path.exists() && !trashed_path.is_symlink() {
        return Err(CommandError::PathVanished {
            path: trashed_path.to_string_lossy().to_string(),
        });
    }

    let id = adapter
        .file_identity(trashed_path)
        .map_err(|e| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: format!("Failed to read file identity in Trash: {e}"),
        })?;

    if id.device_id != item.device_id || id.inode != item.inode {
        return Err(CommandError::ChangedAfterReview {
            plan_id: item.operation_id.clone(),
            details: format!(
                "File identity in Trash changed: expected device {} inode {}, got device {} inode {}",
                item.device_id, item.inode, id.device_id, id.inode
            ),
        });
    }

    // 5. Determine target destination and verify NO OVERWRITE
    let destination_path = if let Some(alt) = &args.alternate_destination {
        PathBuf::from(alt)
    } else {
        item.original_path.clone()
    };

    if destination_path.exists() || destination_path.is_symlink() {
        return Err(CommandError::PermissionDenied {
            path: Some(destination_path.to_string_lossy().to_string()),
            reason: "Destination is occupied. Overwriting existing files during restore is strictly prohibited. Choose a non-colliding alternate destination or decline.".into(),
        });
    }

    // 6. Re-check protected roots for destination
    let resolved = adapter
        .canonicalize_and_normalize(&destination_path)
        .unwrap_or_else(|_| crate::platform::ResolvedPath {
            original: destination_path.clone(),
            canonical: destination_path.clone(),
            normalized: destination_path.clone(),
            is_firmlink_alias: false,
        });

    let home = adapter.home_directory().ok().map(|d| d.path);
    let app_support = adapter.application_support_directory().ok().map(|d| d.path);
    let caches = adapter.caches_directory().ok().map(|d| d.path);

    let match_ctx = crate::classify::matcher::MatchContext {
        original_path: &destination_path,
        canonical_path: &resolved.canonical,
        normalized_path: &resolved.normalized,
        entry_type: crate::platform::EntryType::File,
        identity: id,
        apparent_size: item.size_bytes,
        allocated_size: item.size_bytes,
        modified_ms: None,
        is_symlink: false,
        symlink_target_canonical: None,
        home_dir: home.as_deref(),
        app_support_dir: app_support.as_deref(),
        caches_dir: caches.as_deref(),
        crosses_mount_boundary: false,
        inside_git_repo: false,
        is_git_internal: false,
    };

    if let Some(reason) = crate::classify::protected::evaluate_protected_roots(&match_ctx) {
        return Err(CommandError::PermissionDenied {
            path: Some(destination_path.to_string_lossy().to_string()),
            reason: format!(
                "Refusing to restore into protected root '{}' ({})",
                reason.root_name, reason.description
            ),
        });
    }

    // 7. Perform mutation through the platform adapter
    adapter
        .restore_from_trash(trashed_path, &destination_path)
        .map_err(|e| CommandError::Unsupported {
            feature: "restore_item".into(),
            reason: e.to_string(),
        })?;

    // 8. Record outcome in restore_outcomes table
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let restore_id = format!("res-{}", crate::scan::session::generate_session_id());

    conn.execute(
        "INSERT INTO restore_outcomes (id, operation_item_id, source_trashed_path, restored_to_path, device_id, inode, size_bytes, status, error_message, restored_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Succeeded', NULL, ?8)",
        rusqlite::params![
            restore_id,
            item.id,
            trashed_path.to_string_lossy().to_string(),
            destination_path.to_string_lossy().to_string(),
            item.device_id as i64,
            item.inode as i64,
            item.size_bytes as i64,
            now_ms as i64,
        ],
    )
    .map_err(|e| CommandError::Unsupported {
        feature: "restore_item".into(),
        reason: e.to_string(),
    })?;

    Ok(RestoreItemSummary {
        restore_id,
        operation_item_id: item.id,
        restored_to_path: destination_path.to_string_lossy().to_string(),
        size_bytes: item.size_bytes,
        restored_at_ms: now_ms,
    })
}

pub fn fetch_lifetime_reclamation_totals_core(
    database: &AppDatabase,
    adapter: &dyn PlatformAdapter,
) -> Result<LifetimeReclamationTotals, CommandError> {
    let conn = database
        .connection()
        .lock()
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_lifetime_reclamation_totals".into(),
            reason: e.to_string(),
        })?;

    // Permanently reclaimed bytes: lifetime sum from operations table
    let permanently_reclaimed_bytes: u64 = conn
        .query_row(
            "SELECT COALESCE(SUM(bytes_permanently_reclaimed), 0) FROM operations",
            [],
            |row| row.get::<_, i64>(0).map(|v| v as u64),
        )
        .map_err(|e| CommandError::Unsupported {
            feature: "fetch_lifetime_reclamation_totals".into(),
            reason: e.to_string(),
        })?;

    // Pending in Trash bytes: computed live against filesystem for items currently in Trash
    let trash_items = OperationRepository::fetch_all_trash_operation_items(&conn).map_err(|e| {
        CommandError::Unsupported {
            feature: "fetch_lifetime_reclamation_totals".into(),
            reason: e.to_string(),
        }
    })?;

    let mut pending_in_trash_bytes = 0u64;
    for item in trash_items {
        let eligibility = compute_restore_eligibility(&item, &conn, adapter);
        if let RestoreEligibility::Eligible { .. } = eligibility {
            pending_in_trash_bytes += item.size_bytes;
        }
    }

    Ok(LifetimeReclamationTotals {
        pending_in_trash_bytes,
        permanently_reclaimed_bytes,
    })
}

#[tauri::command]
pub fn fetch_history_operations(
    args: FetchHistoryOperationsArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
) -> Result<Vec<HistoryOperationSummary>, CommandError> {
    fetch_history_operations_core(args, database.inner())
}

#[tauri::command]
pub fn fetch_operation_detail(
    args: FetchOperationDetailArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
) -> Result<HistoryOperationDetail, CommandError> {
    fetch_operation_detail_core(args, database.inner(), adapter.inner().as_ref())
}

#[tauri::command]
pub fn restore_item(
    args: RestoreItemArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
) -> Result<RestoreItemSummary, CommandError> {
    restore_item_core(args, database.inner(), adapter.inner().as_ref())
}

#[tauri::command]
pub fn fetch_lifetime_reclamation_totals(
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
) -> Result<LifetimeReclamationTotals, CommandError> {
    fetch_lifetime_reclamation_totals_core(database.inner(), adapter.inner().as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boundary::destructive::{ExecutePlanArgs, execute_plan_core};
    use crate::boundary::plan::{BuildPlanArgs, build_plan_core};
    use crate::platform::tests::TestAdapter;
    use crate::scan::fixtures::DisposableFixtureTree;
    use crate::scan::session::ScanSessionRepository;
    use crate::storage::open_database_at_path;

    // Test 1: Operations and per-item outcomes persist across an application restart.
    // Unix-only: executing Trash actions requires pre-execution revalidation of
    // filesystem identity (device id and inode) and platform Trash semantics, which
    // the Windows platform adapter declares unsupported.
    #[cfg(unix)]
    #[test]
    fn test_operations_and_item_outcomes_persist_across_application_restart() {
        let fixture = DisposableFixtureTree::new("restart-persist");
        let file1 = fixture.create_file("f1.log", b"LOG DATA 1");
        let file2 = fixture.create_file("f2.log", b"LOG DATA 2");

        let db_path = fixture.root.join("test_restart.db");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        // Scope 1: Create database, plan, and execute
        {
            let db = open_database_at_path(&db_path).expect("open initial db");
            let conn = db.connection().lock().unwrap();
            ScanSessionRepository::create_session(&conn, "sess-restart", "/").unwrap();
            ScanSessionRepository::insert_finding(
                &conn,
                "find-1",
                "sess-restart",
                &file1.to_string_lossy(),
                10,
                "Rebuildable",
                "Logs",
            )
            .unwrap();
            ScanSessionRepository::insert_finding(
                &conn,
                "find-2",
                "sess-restart",
                &file2.to_string_lossy(),
                10,
                "Rebuildable",
                "Logs",
            )
            .unwrap();
            drop(conn);

            let plan = build_plan_core(
                BuildPlanArgs {
                    session_id: "sess-restart".into(),
                    finding_ids: vec!["find-1".into(), "find-2".into()],
                },
                &db,
                &adapter,
            )
            .unwrap();

            let summary = execute_plan_core(
                ExecutePlanArgs {
                    plan_id: plan.plan_id.clone(),
                    action_mode: ActionMode::Trash,
                },
                &db,
                &adapter,
                None,
            )
            .unwrap();
            assert_eq!(summary.succeeded_items, 2);
            assert!(summary.operation_id.is_some());
        }
        // Simulated application restart: DB connection and structs dropped.

        // Scope 2: Reopen database from same file on disk
        {
            let db_reopened = open_database_at_path(&db_path).expect("reopen db after restart");
            let ops = fetch_history_operations_core(
                FetchHistoryOperationsArgs {
                    outcome_filter: None,
                    date_from_ms: None,
                    date_to_ms: None,
                },
                &db_reopened,
            )
            .unwrap();

            assert_eq!(ops.len(), 1, "Operation must persist across restart");
            let op = &ops[0];
            assert_eq!(op.action_mode, ActionMode::Trash);
            assert_eq!(op.succeeded_items, 2);
            assert_eq!(op.failed_items, 0);
            assert_eq!(op.bytes_pending_trash, 20);
            assert_eq!(op.bytes_permanently_reclaimed, 0);

            let detail = fetch_operation_detail_core(
                FetchOperationDetailArgs {
                    operation_id: op.id.clone(),
                    outcome_filter: None,
                },
                &db_reopened,
                &adapter,
            )
            .unwrap();

            assert_eq!(
                detail.items.len(),
                2,
                "Operation items must persist across restart"
            );
            for item in &detail.items {
                match item {
                    HistoryItemDetail::Trash(t) => {
                        assert_eq!(t.status, ItemOutcomeStatus::Succeeded);
                        assert_eq!(t.bytes_pending_trash, 10);
                        assert!(t.trashed_path.is_some());
                    }
                    HistoryItemDetail::PermanentDelete(_) => {
                        panic!("Expected trash item");
                    }
                }
            }
        }
    }

    // Test 2: A test empties the fixture Trash and asserts the affected items become ineligible with a stated reason rather than disappearing.
    // Unix-only: verifying restore eligibility requires filesystem identity (device id
    // and inode) and Trash lifecycle semantics, which have no equivalent available on
    // Windows. The Windows platform adapter already declares `file_identity` and
    // `restore_from_trash` unsupported.
    #[cfg(unix)]
    #[test]
    fn test_emptied_trash_renders_item_ineligible_with_stated_reason_without_disappearing() {
        let fixture = DisposableFixtureTree::new("empty-trash");
        let file = fixture.create_file("test_cache.bin", b"CACHE DATA");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();
        ScanSessionRepository::create_session(&conn, "sess-empty", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-empty",
            "sess-empty",
            &file.to_string_lossy(),
            10,
            "Rebuildable",
            "Caches",
        )
        .unwrap();
        drop(conn);

        let plan = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-empty".into(),
                finding_ids: vec!["find-empty".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();

        let summary = execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan.plan_id.clone(),
                action_mode: ActionMode::Trash,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();
        let op_id = summary.operation_id.unwrap();

        // 1. Initially, item in trash is restorable
        let detail_before = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id.clone(),
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();
        assert_eq!(detail_before.items.len(), 1);
        let trashed_file_path = match &detail_before.items[0] {
            HistoryItemDetail::Trash(t) => {
                assert!(matches!(
                    t.restore_eligibility,
                    RestoreEligibility::Eligible { .. }
                ));
                PathBuf::from(t.trashed_path.as_ref().unwrap())
            }
            _ => panic!("Expected trash item"),
        };
        assert!(trashed_file_path.exists());

        // 2. Empty the fixture Trash (remove the file from trash)
        std::fs::remove_file(&trashed_file_path).expect("empty trash by deleting trashed file");
        assert!(!trashed_file_path.exists());

        // 3. Query history detail again: item must NOT disappear, but become ineligible with stated reason
        let detail_after = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id,
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();

        assert_eq!(
            detail_after.items.len(),
            1,
            "Item must NOT disappear from history when Trash is emptied"
        );
        match &detail_after.items[0] {
            HistoryItemDetail::Trash(t) => match &t.restore_eligibility {
                RestoreEligibility::Ineligible { reason } => {
                    assert!(
                        reason.to_lowercase().contains("no longer in trash")
                            || reason.to_lowercase().contains("emptied from trash"),
                        "Expected reason to state item was emptied from trash, got: {reason}"
                    );
                }
                other => panic!("Expected Ineligible status after emptying trash, got: {other:?}"),
            },
            _ => panic!("Expected trash item"),
        }
    }

    // Test 3: A test renames a trashed item and asserts it is ineligible — identity, not name, decides.
    // Unix-only: filesystem identity (device id and inode) has no equivalent
    // available here, and the Windows platform adapter already declares
    // `file_identity` and `restore_from_trash` unsupported.
    #[cfg(unix)]
    #[test]
    fn test_renamed_or_swapped_trash_item_is_ineligible_identity_not_name_decides() {
        let fixture = DisposableFixtureTree::new("rename-trash");
        let file = fixture.create_file("target.txt", b"ORIGINAL CONTENT FOR INODE");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();
        ScanSessionRepository::create_session(&conn, "sess-rename", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-rename",
            "sess-rename",
            &file.to_string_lossy(),
            26,
            "Rebuildable",
            "Caches",
        )
        .unwrap();
        drop(conn);

        let plan = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-rename".into(),
                finding_ids: vec!["find-rename".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();

        let summary = execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan.plan_id.clone(),
                action_mode: ActionMode::Trash,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();
        let op_id = summary.operation_id.unwrap();

        let detail_before = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id.clone(),
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();
        let trashed_file_path = match &detail_before.items[0] {
            HistoryItemDetail::Trash(t) => PathBuf::from(t.trashed_path.as_ref().unwrap()),
            _ => panic!("Expected trash item"),
        };
        assert!(trashed_file_path.exists());

        // Rename the trashed item to another name
        let renamed_path = trashed_file_path.with_extension("renamed");
        std::fs::rename(&trashed_file_path, &renamed_path).unwrap();

        // Create a fake/impostor file at the original trashed path with identical name but NEW inode
        std::fs::write(&trashed_file_path, b"NEW FAKE FILE DIFFERENT INODE").unwrap();

        // Live check against filesystem: identity mismatch must render it ineligible despite matching name
        let detail_after = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id,
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();

        assert_eq!(detail_after.items.len(), 1);
        match &detail_after.items[0] {
            HistoryItemDetail::Trash(t) => match &t.restore_eligibility {
                RestoreEligibility::Ineligible { reason } => {
                    assert!(
                        reason.contains("identity in Trash changed")
                            || reason.contains("Identity mismatch"),
                        "Expected identity mismatch reason, got: {reason}"
                    );
                }
                other => panic!(
                    "Expected Ineligible status due to inode identity mismatch, got: {other:?}"
                ),
            },
            _ => panic!("Expected trash item"),
        }
    }

    // Test 4: A restore into an occupied destination never overwrites; a test asserts the original occupant is untouched, byte for byte.
    // Unix-only: restore operations require verifying filesystem identity in Trash and
    // invoking restore_from_trash, both of which the Windows platform adapter declares
    // unsupported.
    #[cfg(unix)]
    #[test]
    fn test_restore_into_occupied_destination_never_overwrites_original_occupant() {
        let fixture = DisposableFixtureTree::new("restore-overwrite-test");
        let original_file = fixture.create_file("document.txt", b"PAYLOAD BEFORE TRASH");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();
        ScanSessionRepository::create_session(&conn, "sess-overwrite", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-ow",
            "sess-overwrite",
            &original_file.to_string_lossy(),
            20,
            "Rebuildable",
            "Logs",
        )
        .unwrap();
        drop(conn);

        let plan = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-overwrite".into(),
                finding_ids: vec!["find-ow".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();

        let summary = execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan.plan_id.clone(),
                action_mode: ActionMode::Trash,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();
        let op_id = summary.operation_id.unwrap();

        let detail = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id,
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();
        let op_item_id = match &detail.items[0] {
            HistoryItemDetail::Trash(t) => t.operation_item_id.clone(),
            _ => panic!("Expected trash item"),
        };

        // Occupy original destination with different content
        let occupant_data = b"CRITICAL NEW CONTENT IN PLACE - MUST NEVER OVERWRITE";
        std::fs::write(&original_file, occupant_data).unwrap();

        // 1. Direct restore to occupied destination must return Err and decline
        let res = restore_item_core(
            RestoreItemArgs {
                operation_item_id: op_item_id.clone(),
                alternate_destination: None,
            },
            &db,
            &adapter,
        );
        assert!(
            res.is_err(),
            "Must decline to restore over occupied destination"
        );

        // Assert original occupant is untouched byte for byte
        assert_eq!(
            std::fs::read(&original_file).unwrap(),
            occupant_data,
            "Occupant must remain untouched byte for byte"
        );

        // 2. Offer safe non-colliding alternate destination
        let safe_alt = fixture.path("document (Restored).txt");
        let restore_alt = restore_item_core(
            RestoreItemArgs {
                operation_item_id: op_item_id,
                alternate_destination: Some(safe_alt.to_string_lossy().to_string()),
            },
            &db,
            &adapter,
        )
        .expect("restore to alternate destination must succeed");

        assert_eq!(
            restore_alt.restored_to_path,
            safe_alt.to_string_lossy().to_string()
        );
        assert_eq!(std::fs::read(&safe_alt).unwrap(), b"PAYLOAD BEFORE TRASH");
        assert_eq!(
            std::fs::read(&original_file).unwrap(),
            occupant_data,
            "Occupant must still remain untouched byte for byte"
        );
    }

    // Test 5: A restore is itself recorded as an outcome and the preceding operation's record is unchanged.
    // Unix-only: restore operations require verifying filesystem identity in Trash and
    // invoking restore_from_trash, both of which the Windows platform adapter declares
    // unsupported.
    #[cfg(unix)]
    #[test]
    fn test_restore_recorded_as_own_outcome_preceding_operation_unchanged() {
        let fixture = DisposableFixtureTree::new("restore-preceding-unchanged");
        let file = fixture.create_file("unchanged.txt", b"RESTORE RECORD TEST");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();
        ScanSessionRepository::create_session(&conn, "sess-rec", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-rec",
            "sess-rec",
            &file.to_string_lossy(),
            19,
            "Rebuildable",
            "Cache",
        )
        .unwrap();
        drop(conn);

        let plan = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-rec".into(),
                finding_ids: vec!["find-rec".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();

        let summary = execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan.plan_id.clone(),
                action_mode: ActionMode::Trash,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();
        let op_id = summary.operation_id.unwrap();

        // Capture preceding operation before restore
        let conn = db.connection().lock().unwrap();
        let op_before = OperationRepository::fetch_operation_by_id(&conn, &op_id)
            .unwrap()
            .unwrap();
        let items_before = OperationRepository::fetch_operation_items(&conn, &op_id).unwrap();
        drop(conn);

        assert_eq!(items_before.len(), 1);
        let op_item_id = items_before[0].id.clone();

        // Perform restore
        let restore_summary = restore_item_core(
            RestoreItemArgs {
                operation_item_id: op_item_id.clone(),
                alternate_destination: None,
            },
            &db,
            &adapter,
        )
        .unwrap();

        // Verify restore outcome was recorded in restore_outcomes table
        let conn = db.connection().lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM restore_outcomes WHERE operation_item_id = ?1 AND status = 'Succeeded'",
                rusqlite::params![op_item_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "Restore must be recorded in restore_outcomes");

        // Assert preceding operation's record is completely UNCHANGED
        let op_after = OperationRepository::fetch_operation_by_id(&conn, &op_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            op_before, op_after,
            "Preceding operation must be completely unchanged"
        );

        let items_after = OperationRepository::fetch_operation_items(&conn, &op_id).unwrap();
        assert_eq!(
            items_before, items_after,
            "Preceding operation items must be completely unchanged"
        );
        drop(conn);

        // Check live detail reports item as Restored
        let detail_after = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id,
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();

        match &detail_after.items[0] {
            HistoryItemDetail::Trash(t) => match &t.restore_eligibility {
                RestoreEligibility::Restored {
                    restored_to_path, ..
                } => {
                    assert_eq!(restored_to_path, &restore_summary.restored_to_path);
                }
                other => panic!("Expected Restored status after restore, got: {other:?}"),
            },
            _ => panic!("Expected trash item"),
        }
    }

    // Test 6: Pending and permanently-reclaimed totals are asserted separate all the way from schema to rendered figure.
    // Unix-only: executing plan actions in Trash and PermanentDelete modes requires
    // pre-execution revalidation of filesystem identity (device id and inode), which
    // the Windows platform adapter declares unsupported.
    #[cfg(unix)]
    #[test]
    fn test_pending_and_permanently_reclaimed_totals_asserted_separate_schema_and_api() {
        let fixture = DisposableFixtureTree::new("separate-totals");
        let trash_file = fixture.create_file("trash_file.bin", b"TRASH FILE 500 BYTES LONG");
        let delete_file = fixture.create_file("delete_file.bin", b"PERM DELETE FILE 1000 BYTES");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();

        // 1. Assert schema has separate columns
        let op_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(operations)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(
            op_columns.contains(&"bytes_pending_trash".to_string()),
            "Schema must have bytes_pending_trash"
        );
        assert!(
            op_columns.contains(&"bytes_permanently_reclaimed".to_string()),
            "Schema must have bytes_permanently_reclaimed"
        );
        assert!(
            !op_columns.contains(&"total_bytes_freed".to_string()),
            "Schema must NOT have conflated total column"
        );

        let item_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(operation_items)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(item_columns.contains(&"bytes_pending_trash".to_string()));
        assert!(item_columns.contains(&"bytes_permanently_reclaimed".to_string()));

        ScanSessionRepository::create_session(&conn, "sess-totals", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-trash",
            "sess-totals",
            &trash_file.to_string_lossy(),
            500,
            "Rebuildable",
            "Logs",
        )
        .unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-del",
            "sess-totals",
            &delete_file.to_string_lossy(),
            1000,
            "Rebuildable",
            "Cache",
        )
        .unwrap();
        drop(conn);

        // Execute trash operation
        let plan_trash = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-totals".into(),
                finding_ids: vec!["find-trash".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();
        execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan_trash.plan_id,
                action_mode: ActionMode::Trash,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();

        // Execute permanent delete operation
        let plan_del = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-totals".into(),
                finding_ids: vec!["find-del".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();
        execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan_del.plan_id,
                action_mode: ActionMode::PermanentDelete,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();

        // Query lifetime totals via API
        let totals = fetch_lifetime_reclamation_totals_core(&db, &adapter).unwrap();

        // Assert separate figures
        assert_eq!(
            totals.pending_in_trash_bytes, 500,
            "Pending in Trash must strictly be 500 bytes"
        );
        assert_eq!(
            totals.permanently_reclaimed_bytes, 1000,
            "Permanently reclaimed must strictly be 1000 bytes"
        );
        assert_ne!(
            totals.pending_in_trash_bytes, totals.permanently_reclaimed_bytes,
            "Totals must not be coalesced"
        );

        // Ensure serialized representation carries both as distinct fields
        let serialized = serde_json::to_string(&totals).unwrap();
        assert!(serialized.contains("\"pendingInTrashBytes\":500"));
        assert!(serialized.contains("\"permanentlyReclaimedBytes\":1000"));
        assert!(
            !serialized.contains("1500"),
            "Totals must never be summed together in payload"
        );
    }

    // Cross-platform schema verification: asserts operations and history tables carry distinct
    // pending-trash and permanent-reclamation columns without requiring platform execution adapters.
    #[test]
    fn test_operations_and_history_schema_columns() {
        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();

        let op_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(operations)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(
            op_columns.contains(&"bytes_pending_trash".to_string()),
            "Schema must have bytes_pending_trash"
        );
        assert!(
            op_columns.contains(&"bytes_permanently_reclaimed".to_string()),
            "Schema must have bytes_permanently_reclaimed"
        );
        assert!(
            !op_columns.contains(&"total_bytes_freed".to_string()),
            "Schema must NOT have conflated total column"
        );

        let item_columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(operation_items)").unwrap();
            stmt.query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert!(item_columns.contains(&"bytes_pending_trash".to_string()));
        assert!(item_columns.contains(&"bytes_permanently_reclaimed".to_string()));
    }

    // Test 7: A permanently deleted item is unrepresentable in the restore flow.
    // Unix-only: executing permanent deletion requires pre-execution revalidation of
    // filesystem identity (device id and inode), and restore capabilities are unsupported
    // on Windows.
    #[cfg(unix)]
    #[test]
    fn test_permanently_deleted_item_is_structurally_unrepresentable_in_restore_flow() {
        let fixture = DisposableFixtureTree::new("perm-delete-unrep");
        let file = fixture.create_file("to_destroy.log", b"DESTROY DATA FOREVER");

        let mut adapter = TestAdapter::default();
        let trash_dir = fixture.create_dir("mock_trash");
        adapter.trash = trash_dir;

        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();
        ScanSessionRepository::create_session(&conn, "sess-perm", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "find-perm",
            "sess-perm",
            &file.to_string_lossy(),
            20,
            "Rebuildable",
            "Logs",
        )
        .unwrap();
        drop(conn);

        let plan = build_plan_core(
            BuildPlanArgs {
                session_id: "sess-perm".into(),
                finding_ids: vec!["find-perm".into()],
            },
            &db,
            &adapter,
        )
        .unwrap();

        let summary = execute_plan_core(
            ExecutePlanArgs {
                plan_id: plan.plan_id.clone(),
                action_mode: ActionMode::PermanentDelete,
            },
            &db,
            &adapter,
            None,
        )
        .unwrap();
        let op_id = summary.operation_id.unwrap();

        let detail = fetch_operation_detail_core(
            FetchOperationDetailArgs {
                operation_id: op_id,
                outcome_filter: None,
            },
            &db,
            &adapter,
        )
        .unwrap();

        assert_eq!(detail.items.len(), 1);

        // Assert item is of type PermanentDelete, which has no restore eligibility representation
        let op_item_id = match &detail.items[0] {
            HistoryItemDetail::PermanentDelete(p) => {
                assert_eq!(p.status, ItemOutcomeStatus::Succeeded);
                assert_eq!(p.bytes_permanently_reclaimed, 20);
                p.operation_item_id.clone()
            }
            HistoryItemDetail::Trash(_) => {
                panic!("Permanently deleted item must NOT be represented as Trash item");
            }
        };

        // Assert invoking restore_item_core on permanently deleted item is completely rejected
        let restore_res = restore_item_core(
            RestoreItemArgs {
                operation_item_id: op_item_id,
                alternate_destination: None,
            },
            &db,
            &adapter,
        );

        match restore_res {
            Err(CommandError::Unsupported { reason, .. }) => {
                assert!(
                    reason.contains("Permanently deleted items cannot be restored"),
                    "Expected permanent deletion rejection reason, got: {reason}"
                );
            }
            other => {
                panic!("Expected rejection of restore for permanently deleted item, got: {other:?}")
            }
        }
    }
}
