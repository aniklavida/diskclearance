use serde::{Deserialize, Serialize};
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

pub struct PlanRepository;

impl PlanRepository {
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
) -> Result<ReviewPlanHeader, CommandError> {
    let conn = database.connection().lock().unwrap();

    let mut stmt = conn.prepare("SELECT id, path, size_bytes, safety_class, category FROM findings WHERE session_id = ?1 AND id = ?2")
        .map_err(|e| CommandError::Unsupported { feature: "build_plan".into(), reason: e.to_string() })?;

    let mut items = Vec::new();
    let mut total_bytes = 0;
    for finding_id in &args.finding_ids {
        let mut rows = stmt
            .query(rusqlite::params![args.session_id, finding_id])
            .unwrap();
        if let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0).unwrap();
            let path: String = row.get(1).unwrap();
            let size_bytes: u64 = row.get(2).unwrap();
            let safety_class: String = row.get(3).unwrap();
            let _category: String = row.get(4).unwrap();

            total_bytes += size_bytes;
            items.push(PlanItemSummary {
                item_id: id,
                original_path: path,
                size_bytes,
                class_name: safety_class.clone(),
                action_name: if safety_class == "Review" {
                    "Review".to_string()
                } else {
                    "Trash".to_string()
                },
                recoverable: true,
            });
        }
    }

    let plan_id = crate::scan::session::generate_session_id();
    let created_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let item_count = items.len() as u64;

    let plan = ReviewPlan {
        plan_id: plan_id.clone(),
        session_id: args.session_id.clone(),
        items,
        default_action_mode: ActionMode::Trash,
        created_at_ms,
    };

    PlanRepository::insert_plan(&conn, &plan).map_err(|e| CommandError::Unsupported {
        feature: "build_plan".into(),
        reason: e.to_string(),
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
    _adapter: &Arc<dyn PlatformAdapter>,
) -> Result<RevalidationResult, CommandError> {
    let conn = database.connection().lock().unwrap();
    let plan = PlanRepository::fetch_plan(&conn, &args.plan_id)
        .unwrap()
        .unwrap();

    let mut stale_item_ids = Vec::new();
    for item in plan.items {
        if std::fs::metadata(&item.original_path).is_err() {
            stale_item_ids.push(item.item_id);
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
) -> Result<ReviewPlanHeader, CommandError> {
    build_plan_core(args, database.inner())
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
    revalidate_plan_core(args, database.inner(), adapter.inner())
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
        let header = build_plan_core(build_args, &db).unwrap();

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
        let reval_res = revalidate_plan_core(reval_args.clone(), &db, &adapter).unwrap();
        assert!(reval_res.is_valid);
        assert!(reval_res.stale_item_ids.is_empty());

        // Delete file, verify revalidate spots it
        std::fs::remove_file(&file).unwrap();
        let reval_res = revalidate_plan_core(reval_args, &db, &adapter).unwrap();
        assert!(!reval_res.is_valid);
        assert_eq!(reval_res.stale_item_ids, vec!["f-1"]);
    }
}
