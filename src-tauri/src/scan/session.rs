use rusqlite::{Connection, params};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::boundary::read::FindingItem;
use crate::scan::coverage::CoverageSummary;
use crate::storage::StorageError;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique scan session identifier generator.
pub fn generate_session_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!(
        "scan-{}-{}-{}",
        now.as_millis(),
        std::process::id(),
        counter
    )
}

/// Helper for storing and querying scan sessions and findings in SQLite.
pub struct ScanSessionRepository;

impl ScanSessionRepository {
    pub fn create_session(
        conn: &Connection,
        session_id: &str,
        root_path: &str,
    ) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO scan_sessions (id, started_at, status, root_path, total_bytes, scanned_items)
             VALUES (?1, CURRENT_TIMESTAMP, 'scanning', ?2, 0, 0)",
            params![session_id, root_path],
        )?;
        Ok(())
    }

    pub fn complete_session(
        conn: &Connection,
        session_id: &str,
        status: &str,
        coverage: &CoverageSummary,
    ) -> Result<(), StorageError> {
        conn.execute(
            "UPDATE scan_sessions
             SET completed_at = CURRENT_TIMESTAMP,
                 status = ?1,
                 total_bytes = ?2,
                 scanned_items = ?3
             WHERE id = ?4",
            params![
                status,
                coverage.total_allocated_bytes as i64,
                coverage.items_scanned as i64,
                session_id
            ],
        )?;
        Ok(())
    }

    pub fn insert_finding(
        conn: &Connection,
        id: &str,
        session_id: &str,
        path: &str,
        size_bytes: u64,
        safety_class: &str,
        category: &str,
    ) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO findings (id, session_id, path, size_bytes, safety_class, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                session_id,
                path,
                size_bytes as i64,
                safety_class,
                category
            ],
        )?;
        Ok(())
    }

    pub fn fetch_findings_page(
        conn: &Connection,
        session_id: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<FindingItem>, Option<String>, u64), StorageError> {
        let total_items: u64 = conn
            .query_row(
                "SELECT scanned_items FROM scan_sessions WHERE id = ?1",
                params![session_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v.max(0) as u64)
            .unwrap_or(0);

        let query_limit = limit.clamp(1, 1000) as i64;
        let mut items = Vec::new();

        if let Some(c) = cursor {
            let mut stmt = conn.prepare(
                "SELECT id, path, size_bytes, category, safety_class
                 FROM findings
                 WHERE session_id = ?1 AND id > ?2
                 ORDER BY id ASC
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![session_id, c, query_limit], |row| {
                Ok(FindingItem {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    size_bytes: row.get::<_, i64>(2)?.max(0) as u64,
                    category: row.get(3)?,
                    action_kind: row.get(4)?,
                })
            })?;
            for item in rows {
                items.push(item?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, path, size_bytes, category, safety_class
                 FROM findings
                 WHERE session_id = ?1
                 ORDER BY id ASC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![session_id, query_limit], |row| {
                Ok(FindingItem {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    size_bytes: row.get::<_, i64>(2)?.max(0) as u64,
                    category: row.get(3)?,
                    action_kind: row.get(4)?,
                })
            })?;
            for item in rows {
                items.push(item?);
            }
        }

        let next_cursor = if items.len() == query_limit as usize {
            items.last().map(|it| it.id.clone())
        } else {
            None
        };

        Ok((items, next_cursor, total_items))
    }
}
