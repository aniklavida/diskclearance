use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use ts_rs::TS;

use crate::boundary::destructive::ActionMode;
use crate::boundary::error::CommandError;
use crate::platform::PlatformAdapter;
use crate::storage::AppDatabase;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlanArgs {
    pub session_id: String,
    pub finding_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlanHeader {
    pub plan_id: String,
    pub session_id: String,
    pub item_count: u64,
    pub total_bytes: u64,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FetchPlanArgs {
    pub plan_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemSummary {
    pub item_id: String,
    pub original_path: String,
    pub size_bytes: u64,
    pub class_name: String,
    pub action_name: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPlan {
    pub plan_id: String,
    pub session_id: String,
    pub items: Vec<PlanItemSummary>,
    pub default_action_mode: ActionMode,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevalidatePlanArgs {
    pub plan_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RevalidationResult {
    pub plan_id: String,
    pub is_valid: bool,
    pub stale_item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItemDetail {
    pub item_id: String,
    pub plan_id: String,
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub device_id: u64,
    pub inode: u64,
    pub size_bytes: u64,
    pub class_name: String,
    pub rule_id: String,
    pub rule_version: u32,
    pub action_name: String,
    pub recoverable: bool,
}

pub struct PlanRepository;

impl PlanRepository {
    pub fn insert_plan_with_items(
        conn: &rusqlite::Connection,
        plan: &ReviewPlan,
        details: &[PlanItemDetail],
    ) -> Result<(), rusqlite::Error> {
        let default_mode = match plan.default_action_mode {
            ActionMode::Trash => "Trash",
            ActionMode::PermanentDelete => "PermanentDelete",
        };
        conn.execute(
            "INSERT INTO review_plans (id, session_id, default_action_mode, created_at_ms) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![plan.plan_id, plan.session_id, default_mode, plan.created_at_ms],
        )?;

        let mut stmt = conn.prepare(
            "INSERT INTO plan_items (item_id, plan_id, original_path, canonical_path, device_id, inode, size_bytes, class_name, rule_id, rule_version, action_name, recoverable)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
        )?;
        for item in details {
            stmt.execute(rusqlite::params![
                item.item_id,
                item.plan_id,
                item.original_path.to_string_lossy().to_string(),
                item.canonical_path.to_string_lossy().to_string(),
                item.device_id as i64,
                item.inode as i64,
                item.size_bytes as i64,
                item.class_name,
                item.rule_id,
                item.rule_version as i64,
                item.action_name,
                item.recoverable as i32,
            ])?;
        }
        Ok(())
    }

    pub fn insert_plan(
        conn: &rusqlite::Connection,
        plan: &ReviewPlan,
    ) -> Result<(), rusqlite::Error> {
        let default_mode = match plan.default_action_mode {
            ActionMode::Trash => "Trash",
            ActionMode::PermanentDelete => "PermanentDelete",
        };
        conn.execute(
            "INSERT INTO review_plans (id, session_id, default_action_mode, created_at_ms) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![plan.plan_id, plan.session_id, default_mode, plan.created_at_ms],
        )?;

        let mut stmt = conn.prepare(
            "INSERT INTO plan_items (item_id, plan_id, original_path, canonical_path, device_id, inode, size_bytes, class_name, rule_id, rule_version, action_name, recoverable)
             VALUES (?1, ?2, ?3, ?3, 0, 0, ?4, ?5, '', 0, ?6, ?7)"
        )?;
        for item in &plan.items {
            stmt.execute(rusqlite::params![
                item.item_id,
                plan.plan_id,
                item.original_path,
                item.size_bytes,
                item.class_name,
                item.action_name,
                item.recoverable as i32
            ])?;
        }
        Ok(())
    }

    pub fn fetch_plan_items(
        conn: &rusqlite::Connection,
        plan_id: &str,
    ) -> Result<Vec<PlanItemDetail>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT item_id, plan_id, original_path, canonical_path, device_id, inode, size_bytes, class_name, rule_id, rule_version, action_name, recoverable FROM plan_items WHERE plan_id = ?1"
        )?;
        let items_iter = stmt.query_map(rusqlite::params![plan_id], |row| {
            let orig: String = row.get(2)?;
            let canon: String = row.get(3)?;
            Ok(PlanItemDetail {
                item_id: row.get(0)?,
                plan_id: row.get(1)?,
                original_path: PathBuf::from(orig),
                canonical_path: PathBuf::from(canon),
                device_id: row.get::<_, i64>(4)? as u64,
                inode: row.get::<_, i64>(5)? as u64,
                size_bytes: row.get::<_, i64>(6)? as u64,
                class_name: row.get(7)?,
                rule_id: row.get(8)?,
                rule_version: row.get::<_, i64>(9)? as u32,
                action_name: row.get(10)?,
                recoverable: row.get::<_, i32>(11)? != 0,
            })
        })?;

        let mut items = Vec::new();
        for item in items_iter {
            items.push(item?);
        }
        Ok(items)
    }

    pub fn fetch_plan(
        conn: &rusqlite::Connection,
        plan_id: &str,
    ) -> Result<Option<ReviewPlan>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT session_id, default_action_mode, created_at_ms FROM review_plans WHERE id = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![plan_id])?;
        let row = match rows.next()? {
            Some(r) => r,
            None => return Ok(None),
        };
        let session_id: String = row.get(0)?;
        let mode_str: String = row.get(1)?;
        let created_at_ms: u64 = row.get(2)?;
        let default_action_mode = if mode_str == "PermanentDelete" {
            ActionMode::PermanentDelete
        } else {
            ActionMode::Trash
        };

        let mut stmt_items = conn.prepare("SELECT item_id, original_path, size_bytes, class_name, action_name, recoverable FROM plan_items WHERE plan_id = ?1")?;
        let items_iter = stmt_items.query_map(rusqlite::params![plan_id], |row| {
            Ok(PlanItemSummary {
                item_id: row.get(0)?,
                original_path: row.get(1)?,
                size_bytes: row.get(2)?,
                class_name: row.get(3)?,
                action_name: row.get(4)?,
                recoverable: row.get::<_, i32>(5)? != 0,
            })
        })?;

        let mut items = Vec::new();
        for item in items_iter {
            items.push(item?);
        }

        Ok(Some(ReviewPlan {
            plan_id: plan_id.to_string(),
            session_id,
            items,
            default_action_mode,
            created_at_ms,
        }))
    }
}

pub fn build_plan_core(
    args: BuildPlanArgs,
    database: &Arc<AppDatabase>,
    adapter: &dyn PlatformAdapter,
) -> Result<ReviewPlanHeader, CommandError> {
    let conn = database.connection().lock().unwrap();

    let mut stmt = conn
        .prepare("SELECT id, path, size_bytes, safety_class, category, rule_id FROM findings WHERE session_id = ?1 AND id = ?2")
        .map_err(|e| CommandError::Unsupported { feature: "build_plan".into(), reason: e.to_string() })?;

    let plan_id = crate::scan::session::generate_session_id();
    let mut summaries = Vec::new();
    let mut details = Vec::new();
    let mut total_bytes = 0;

    for finding_id in &args.finding_ids {
        let mut rows = stmt
            .query(rusqlite::params![args.session_id, finding_id])
            .map_err(|e| CommandError::Unsupported {
                feature: "build_plan".into(),
                reason: e.to_string(),
            })?;

        if let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0).unwrap();
            let path_str: String = row.get(1).unwrap();
            let size_bytes: u64 = row.get(2).unwrap();
            let safety_class: String = row.get(3).unwrap();
            let _category: String = row.get(4).unwrap();
            let rule_id: String = row
                .get::<_, Option<String>>(5)
                .unwrap_or(None)
                .unwrap_or_default();

            // Direct check on safety class: Protected items MUST NOT enter a plan
            if safety_class.eq_ignore_ascii_case("Protected") {
                return Err(CommandError::PermissionDenied {
                    path: Some(path_str),
                    reason: "Protected items cannot be admitted into a deletion plan".to_string(),
                });
            }

            let path = PathBuf::from(&path_str);

            // Re-resolve canonical path and read identity at build time
            let resolved = adapter
                .canonicalize_and_normalize(&path)
                .unwrap_or_else(|_| crate::platform::ResolvedPath {
                    original: path.clone(),
                    canonical: path.clone(),
                    normalized: path.clone(),
                    is_firmlink_alias: false,
                });

            let identity = adapter
                .file_identity(&path)
                .unwrap_or(crate::platform::FileIdentity {
                    device_id: 0,
                    inode: 0,
                });

            // Pre-check against protected roots table
            let home = adapter.home_directory().ok().map(|d| d.path);
            let app_support = adapter.application_support_directory().ok().map(|d| d.path);
            let caches = adapter.caches_directory().ok().map(|d| d.path);
            let is_symlink = path.is_symlink();
            let symlink_target_canonical = if is_symlink {
                path.read_link().ok().and_then(|t| t.canonicalize().ok())
            } else {
                None
            };

            let match_ctx = crate::classify::matcher::MatchContext {
                original_path: &path,
                canonical_path: &resolved.canonical,
                normalized_path: &resolved.normalized,
                entry_type: if is_symlink {
                    crate::platform::EntryType::Symlink
                } else if path.is_dir() {
                    crate::platform::EntryType::Directory
                } else {
                    crate::platform::EntryType::File
                },
                identity,
                apparent_size: size_bytes,
                allocated_size: size_bytes,
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

            if let Some(reason) = crate::classify::protected::evaluate_protected_roots(&match_ctx) {
                return Err(CommandError::PermissionDenied {
                    path: Some(path_str),
                    reason: format!(
                        "Target matches protected root '{}' ({})",
                        reason.root_name, reason.description
                    ),
                });
            }

            total_bytes += size_bytes;
            let action_name = if safety_class.eq_ignore_ascii_case("Review") {
                "Review".to_string()
            } else {
                "Trash".to_string()
            };

            summaries.push(PlanItemSummary {
                item_id: id.clone(),
                original_path: path_str,
                size_bytes,
                class_name: safety_class.clone(),
                action_name: action_name.clone(),
                recoverable: true,
            });

            details.push(PlanItemDetail {
                item_id: id,
                plan_id: plan_id.clone(),
                original_path: path,
                canonical_path: resolved.canonical,
                device_id: identity.device_id,
                inode: identity.inode,
                size_bytes,
                class_name: safety_class,
                rule_id,
                rule_version: 1,
                action_name,
                recoverable: true,
            });
        }
    }

    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let item_count = summaries.len() as u64;

    let plan = ReviewPlan {
        plan_id: plan_id.clone(),
        session_id: args.session_id.clone(),
        items: summaries,
        default_action_mode: ActionMode::Trash,
        created_at_ms,
    };

    PlanRepository::insert_plan_with_items(&conn, &plan, &details).map_err(|e| {
        CommandError::Unsupported {
            feature: "build_plan".into(),
            reason: e.to_string(),
        }
    })?;

    Ok(ReviewPlanHeader {
        plan_id,
        session_id: args.session_id,
        item_count,
        total_bytes,
        created_at_ms,
    })
}

pub fn fetch_plan_core(
    args: FetchPlanArgs,
    database: &Arc<AppDatabase>,
) -> Result<ReviewPlan, CommandError> {
    let conn = database.connection().lock().unwrap();
    if let Some(plan) = PlanRepository::fetch_plan(&conn, &args.plan_id).unwrap() {
        Ok(plan)
    } else {
        Err(CommandError::Unsupported {
            feature: "fetch_plan".into(),
            reason: "Not found".into(),
        })
    }
}

pub fn revalidate_plan_core(
    args: RevalidatePlanArgs,
    database: &Arc<AppDatabase>,
    adapter: &dyn PlatformAdapter,
) -> Result<RevalidationResult, CommandError> {
    let conn = database.connection().lock().unwrap();
    let items = PlanRepository::fetch_plan_items(&conn, &args.plan_id).map_err(|e| {
        CommandError::Unsupported {
            feature: "revalidate_plan".into(),
            reason: e.to_string(),
        }
    })?;

    let home = adapter.home_directory().ok().map(|d| d.path);
    let app_support = adapter.application_support_directory().ok().map(|d| d.path);
    let caches = adapter.caches_directory().ok().map(|d| d.path);

    let mut stale_item_ids = Vec::new();
    for item in items {
        // 1. Existence and metadata
        let metadata = match adapter.read_entry_metadata(&item.original_path) {
            Ok(m) => m,
            Err(_) => {
                stale_item_ids.push(item.item_id);
                continue;
            }
        };

        // 2. Exact filesystem identity (device and inode)
        if metadata.identity.device_id != item.device_id || metadata.identity.inode != item.inode {
            stale_item_ids.push(item.item_id);
            continue;
        }

        // 3. Canonical path stability
        let resolved = match adapter.canonicalize_and_normalize(&item.original_path) {
            Ok(r) => r,
            Err(_) => {
                stale_item_ids.push(item.item_id);
                continue;
            }
        };
        if resolved.canonical != item.canonical_path {
            stale_item_ids.push(item.item_id);
            continue;
        }

        // 4. Target became a symlink after review
        if metadata.entry_type == crate::platform::EntryType::Symlink
            && item.class_name != "Symlink"
        {
            stale_item_ids.push(item.item_id);
            continue;
        }

        // 5. Protected check
        let is_symlink = metadata.entry_type == crate::platform::EntryType::Symlink;
        let symlink_target_canonical = if is_symlink {
            item.original_path
                .read_link()
                .ok()
                .and_then(|t| t.canonicalize().ok())
        } else {
            None
        };

        let match_ctx = crate::classify::matcher::MatchContext {
            original_path: &item.original_path,
            canonical_path: &resolved.canonical,
            normalized_path: &resolved.normalized,
            entry_type: metadata.entry_type,
            identity: metadata.identity,
            apparent_size: metadata.apparent_size,
            allocated_size: metadata.allocated_size,
            modified_ms: metadata.modified_ms,
            is_symlink,
            symlink_target_canonical,
            home_dir: home.as_deref(),
            app_support_dir: app_support.as_deref(),
            caches_dir: caches.as_deref(),
            crosses_mount_boundary: false,
            inside_git_repo: false,
            is_git_internal: false,
        };

        if crate::classify::protected::evaluate_protected_roots(&match_ctx).is_some() {
            stale_item_ids.push(item.item_id);
            continue;
        }
    }

    Ok(RevalidationResult {
        plan_id: args.plan_id,
        is_valid: stale_item_ids.is_empty(),
        stale_item_ids,
    })
}

#[tauri::command]
pub fn build_plan(
    args: BuildPlanArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
) -> Result<ReviewPlanHeader, CommandError> {
    build_plan_core(args, database.inner(), adapter.inner().as_ref())
}

#[tauri::command]
pub fn fetch_plan(
    args: FetchPlanArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
) -> Result<ReviewPlan, CommandError> {
    fetch_plan_core(args, database.inner())
}

#[tauri::command]
pub fn revalidate_plan(
    args: RevalidatePlanArgs,
    database: tauri::State<'_, Arc<AppDatabase>>,
    adapter: tauri::State<'_, Arc<dyn PlatformAdapter>>,
) -> Result<RevalidationResult, CommandError> {
    revalidate_plan_core(args, database.inner(), adapter.inner().as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tests::TestAdapter;
    use crate::scan::fixtures::DisposableFixtureTree;
    use crate::scan::session::ScanSessionRepository;
    use crate::storage::AppDatabase;
    use std::sync::Arc;

    #[test]
    fn test_plan_immutability_and_revalidation() {
        let fixture = DisposableFixtureTree::new("boundary-plan");
        let file = fixture.create_file("finding.txt", b"content");

        let adapter: Arc<dyn PlatformAdapter> = Arc::new(TestAdapter::default());
        let db = AppDatabase::open_in_memory().unwrap();
        let conn = db.connection().lock().unwrap();

        ScanSessionRepository::create_session(&conn, "sess-1", "/").unwrap();
        ScanSessionRepository::insert_finding(
            &conn,
            "f-1",
            "sess-1",
            &file.to_string_lossy(),
            7,
            "Review",
            "Scanned",
        )
        .unwrap();
        drop(conn);

        let build_args = BuildPlanArgs {
            session_id: "sess-1".to_string(),
            finding_ids: vec!["f-1".to_string()],
        };

        // Build plan
        let header = build_plan_core(build_args, &db, adapter.as_ref()).unwrap();

        // Verify immutability: modify finding in DB
        let conn = db.connection().lock().unwrap();
        conn.execute("UPDATE findings SET size_bytes = 100 WHERE id = 'f-1'", [])
            .unwrap();
        drop(conn);

        // Fetch plan, verify size is still 7
        let fetch_args = FetchPlanArgs {
            plan_id: header.plan_id.clone(),
        };
        let plan = fetch_plan_core(fetch_args, &db).unwrap();
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].size_bytes, 7, "Plan must be immutable");

        // Revalidate: valid
        let reval_args = RevalidatePlanArgs {
            plan_id: header.plan_id.clone(),
        };
        let reval_res = revalidate_plan_core(reval_args.clone(), &db, adapter.as_ref()).unwrap();
        assert!(reval_res.is_valid);
        assert!(reval_res.stale_item_ids.is_empty());

        // Delete file, verify revalidate spots it
        std::fs::remove_file(&file).unwrap();
        let reval_res = revalidate_plan_core(reval_args, &db, adapter.as_ref()).unwrap();
        assert!(!reval_res.is_valid);
        assert_eq!(reval_res.stale_item_ids, vec!["f-1"]);
    }
}
