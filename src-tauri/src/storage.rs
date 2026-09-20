use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, Result as SqliteResult};

use crate::platform::{PlatformAdapter, PlatformError};

pub const APP_BUNDLE_ID: &str = "com.aniklavida.diskclearance";
pub const DATABASE_FILENAME: &str = "diskclearance.db";

// Migrations are forward-only and append-only.
// An existing migration is never edited once shipped; a mistake is corrected by a new higher-numbered migration.
pub const MIGRATIONS: &[(i64, &str)] = &[
    (1, MIGRATION_0001_FOUNDATION),
    (2, MIGRATION_0002_CORE_TABLES),
    (3, MIGRATION_0003_REVIEW_PLANS),
    (4, MIGRATION_0004_OPERATIONS_AND_HISTORY),
];

const MIGRATION_0001_FOUNDATION: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
";

const MIGRATION_0002_CORE_TABLES: &str = "
CREATE TABLE scan_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL,
    root_path TEXT NOT NULL,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    scanned_items INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE rule_versions (
    id TEXT PRIMARY KEY NOT NULL,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE findings (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    safety_class TEXT NOT NULL,
    category TEXT NOT NULL,
    rule_id TEXT REFERENCES rule_versions(id),
    detected_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_findings_session_id ON findings(session_id);
CREATE INDEX idx_findings_safety_class ON findings(safety_class);
CREATE INDEX idx_scan_sessions_status ON scan_sessions(status);
";

#[derive(Debug)]
pub enum StorageError {
    Platform(PlatformError),
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    DowngradeNotSupported {
        found_version: i64,
        max_known_version: i64,
    },
    CorruptDatabase {
        path: PathBuf,
        reason: String,
    },
    MigrationFailed {
        version: i64,
        source: rusqlite::Error,
    },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Platform(err) => write!(f, "platform error: {err}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Sqlite(err) => write!(f, "SQLite error: {err}"),
            Self::DowngradeNotSupported {
                found_version,
                max_known_version,
            } => write!(
                f,
                "database version {found_version} is higher than maximum supported version {max_known_version}"
            ),
            Self::CorruptDatabase { path, reason } => {
                write!(f, "database at {} is corrupt: {reason}", path.display())
            }
            Self::MigrationFailed { version, source } => {
                write!(f, "migration {version} failed: {source}")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Platform(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::Sqlite(err) => Some(err),
            Self::MigrationFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<PlatformError> for StorageError {
    fn from(err: PlatformError) -> Self {
        Self::Platform(err)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub applied_versions: Vec<i64>,
    pub current_version: i64,
}

pub struct AppDatabase {
    connection: Mutex<Connection>,
    recovered_backup: Option<PathBuf>,
}

impl AppDatabase {
    pub fn new(connection: Connection, recovered_backup: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            connection: Mutex::new(connection),
            recovered_backup,
        })
    }

    pub fn connection(&self) -> &Mutex<Connection> {
        &self.connection
    }

    pub fn recovered_backup_path(&self) -> Option<&Path> {
        self.recovered_backup.as_deref()
    }

    pub fn was_recovered(&self) -> bool {
        self.recovered_backup.is_some()
    }

    pub fn open_in_memory() -> Result<Arc<Self>, StorageError> {
        let mut connection = Connection::open_in_memory()?;
        initialize(&mut connection)?;
        Ok(Self::new(connection, None))
    }
}

pub fn configure_connection(connection: &Connection) -> Result<(), StorageError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )?;
    Ok(())
}

pub fn highest_applied_version(connection: &Connection) -> Result<Option<i64>, StorageError> {
    let schema_table_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |row| row.get(0),
    )?;

    if !schema_table_exists {
        return Ok(None);
    }

    let version: Option<i64> =
        connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    Ok(version)
}

pub fn run_migrations(connection: &mut Connection) -> Result<MigrationReport, StorageError> {
    run_migrations_with(connection, MIGRATIONS)
}

pub fn run_migrations_with(
    connection: &mut Connection,
    migrations: &[(i64, &str)],
) -> Result<MigrationReport, StorageError> {
    let max_known_version = migrations.last().map(|(v, _)| *v).unwrap_or(0);

    if let Some(current) = highest_applied_version(connection)? {
        if current > max_known_version {
            return Err(StorageError::DowngradeNotSupported {
                found_version: current,
                max_known_version,
            });
        }
    }

    let current = highest_applied_version(connection)?.unwrap_or(0);
    let mut applied_versions = Vec::new();

    for &(version, sql) in migrations {
        if version > current {
            let tx = connection.transaction()?;
            tx.execute_batch(sql)
                .map_err(|source| StorageError::MigrationFailed { version, source })?;
            tx.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [version],
            )
            .map_err(|source| StorageError::MigrationFailed { version, source })?;
            tx.commit()?;
            applied_versions.push(version);
        }
    }

    let final_version = highest_applied_version(connection)?.unwrap_or(0);

    Ok(MigrationReport {
        applied_versions,
        current_version: final_version,
    })
}

pub fn initialize(connection: &mut Connection) -> Result<MigrationReport, StorageError> {
    configure_connection(connection)?;
    run_migrations(connection)
}

pub fn resolve_database_path(adapter: &dyn PlatformAdapter) -> Result<PathBuf, StorageError> {
    let app_support = adapter.application_support_directory()?;
    Ok(app_support.path.join(APP_BUNDLE_ID).join(DATABASE_FILENAME))
}

fn is_rusqlite_corrupt(err: &rusqlite::Error) -> bool {
    match err {
        rusqlite::Error::SqliteFailure(ffi_err, msg) => {
            if ffi_err.code == rusqlite::ffi::ErrorCode::DatabaseCorrupt
                || ffi_err.code == rusqlite::ffi::ErrorCode::NotADatabase
            {
                return true;
            }
            if let Some(msg) = msg {
                let lower = msg.to_lowercase();
                if lower.contains("file is not a database")
                    || lower.contains("malformed")
                    || lower.contains("corrupt")
                {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

enum ValidationResult {
    Valid(Connection),
    Corrupt(String),
    NonCorruption(StorageError),
}

fn validate_existing_file(path: &Path) -> ValidationResult {
    let conn = match Connection::open(path) {
        Ok(c) => c,
        Err(e) => {
            if is_rusqlite_corrupt(&e) {
                return ValidationResult::Corrupt(e.to_string());
            }
            return ValidationResult::NonCorruption(StorageError::Sqlite(e));
        }
    };

    if let Err(e) = configure_connection(&conn) {
        if let StorageError::Sqlite(ref sql_err) = e {
            if is_rusqlite_corrupt(sql_err) {
                return ValidationResult::Corrupt(sql_err.to_string());
            }
        }
        return ValidationResult::NonCorruption(e);
    }

    let quick_check: SqliteResult<String> =
        conn.query_row("PRAGMA quick_check(1);", [], |row| row.get(0));

    match quick_check {
        Ok(status) if status == "ok" => ValidationResult::Valid(conn),
        Ok(status) => ValidationResult::Corrupt(status),
        Err(e) if is_rusqlite_corrupt(&e) => ValidationResult::Corrupt(e.to_string()),
        Err(e) => ValidationResult::NonCorruption(StorageError::Sqlite(e)),
    }
}

pub fn generate_corrupt_backup_path(original_path: &Path) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let timestamp = format!("{}.{}", now.as_secs(), now.subsec_nanos());
    let original_name = original_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(DATABASE_FILENAME);

    let mut candidate =
        original_path.with_file_name(format!("{original_name}.corrupt.{timestamp}"));
    let mut counter = 1;
    while candidate.exists() {
        candidate =
            original_path.with_file_name(format!("{original_name}.corrupt.{timestamp}.{counter}"));
        counter += 1;
    }
    candidate
}

fn preserve_corrupt_database(path: &Path) -> Result<PathBuf, StorageError> {
    let backup_path = generate_corrupt_backup_path(path);
    std::fs::rename(path, &backup_path)?;

    // Also preserve WAL and SHM sidecar files if present
    let wal_path = path.with_file_name(format!(
        "{}-wal",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("")
    ));
    if wal_path.exists() {
        let wal_backup = backup_path.with_file_name(format!(
            "{}-wal",
            backup_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
        ));
        let _ = std::fs::rename(&wal_path, &wal_backup);
    }

    let shm_path = path.with_file_name(format!(
        "{}-shm",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("")
    ));
    if shm_path.exists() {
        let shm_backup = backup_path.with_file_name(format!(
            "{}-shm",
            backup_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
        ));
        let _ = std::fs::rename(&shm_path, &shm_backup);
    }

    Ok(backup_path)
}

pub fn open_database_at_path(path: &Path) -> Result<Arc<AppDatabase>, StorageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut recovered_backup = None;

    let mut connection = if path.exists() {
        match validate_existing_file(path) {
            ValidationResult::Valid(conn) => conn,
            ValidationResult::Corrupt(_reason) => {
                let backup = preserve_corrupt_database(path)?;
                recovered_backup = Some(backup);
                let new_conn = Connection::open(path)?;
                configure_connection(&new_conn)?;
                new_conn
            }
            ValidationResult::NonCorruption(err) => return Err(err),
        }
    } else {
        let conn = Connection::open(path)?;
        configure_connection(&conn)?;
        conn
    };

    initialize(&mut connection)?;
    Ok(AppDatabase::new(connection, recovered_backup))
}

pub fn open_database_for_adapter(
    adapter: &dyn PlatformAdapter,
) -> Result<Arc<AppDatabase>, StorageError> {
    let db_path = resolve_database_path(adapter)?;
    open_database_at_path(&db_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::tests::TestAdapter;

    struct DisposableTempDir {
        path: PathBuf,
    }

    impl DisposableTempDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "test-dc-{prefix}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for DisposableTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn application_creates_database_in_application_support_directory_on_first_launch() {
        let temp_dir = DisposableTempDir::new("first-launch");
        let app_support = temp_dir.path.join("Library/Application Support");

        let mut adapter = TestAdapter::default();
        adapter.app_support = app_support.clone();

        let db = open_database_for_adapter(&adapter).expect("open database");
        assert!(!db.was_recovered());

        let expected_db_path = app_support.join(APP_BUNDLE_ID).join(DATABASE_FILENAME);
        assert!(expected_db_path.exists());

        let conn = db.connection().lock().expect("lock connection");
        let session_table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'scan_sessions')",
                [],
                |row| row.get(0),
            )
            .expect("query scan_sessions table");
        assert!(session_table_exists);
    }

    #[test]
    fn migrations_apply_once_and_record_in_schema_migrations() {
        let mut conn = Connection::open_in_memory().expect("in-memory database");

        let first_report = initialize(&mut conn).expect("initial migration");
        assert_eq!(first_report.applied_versions, vec![1, 2, 3, 4]);
        assert_eq!(first_report.current_version, 4);

        let row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations");
        assert_eq!(row_count, 4);

        let second_report = initialize(&mut conn).expect("second run should do nothing");
        assert!(second_report.applied_versions.is_empty());
        assert_eq!(second_report.current_version, 4);

        let row_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations after");
        assert_eq!(row_count_after, 4);
    }

    #[test]
    fn failed_migration_rolls_back_and_leaves_previous_version_intact() {
        let mut conn = Connection::open_in_memory().expect("in-memory database");

        let test_migrations: &[(i64, &str)] = &[
            (1, MIGRATION_0001_FOUNDATION),
            (
                2,
                "CREATE TABLE should_not_exist (id INTEGER PRIMARY KEY); INVALID SQL SYNTAX;",
            ),
        ];

        let result = run_migrations_with(&mut conn, test_migrations);
        assert!(result.is_err());

        let current_version = highest_applied_version(&conn)
            .expect("query version")
            .expect("version 1 should be recorded");
        assert_eq!(current_version, 1);

        let table_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'should_not_exist')",
                [],
                |row| row.get(0),
            )
            .expect("check table existence");
        assert!(!table_exists);

        let migration_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("count migrations");
        assert_eq!(migration_rows, 1);
    }

    #[test]
    fn downgrade_refuses_database_with_higher_version() {
        let mut conn = Connection::open_in_memory().expect("in-memory database");

        // Seed with a version beyond what the migrations slice knows
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT);
             INSERT INTO schema_migrations (version) VALUES (999);",
        )
        .expect("seed higher version");

        let result = run_migrations_with(&mut conn, MIGRATIONS);
        match result {
            Err(StorageError::DowngradeNotSupported {
                found_version,
                max_known_version,
            }) => {
                assert_eq!(found_version, 999);
                assert_eq!(max_known_version, 4);
            }
            other => panic!("expected DowngradeNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_database_is_preserved_under_new_name_and_fresh_database_created() {
        let temp_dir = DisposableTempDir::new("corrupt-recovery");
        let db_path = temp_dir.path.join(DATABASE_FILENAME);

        // Write corrupt garbage into the database file
        std::fs::write(&db_path, b"NOT A SQLITE DATABASE CORRUPTED DATA")
            .expect("write corrupt file");

        let db = open_database_at_path(&db_path).expect("open database with recovery");
        assert!(db.was_recovered());

        let backup_path = db
            .recovered_backup_path()
            .expect("must have backup path")
            .to_path_buf();
        assert!(backup_path.exists());
        assert_ne!(backup_path, db_path);

        let backup_content = std::fs::read(&backup_path).expect("read backup");
        assert_eq!(backup_content, b"NOT A SQLITE DATABASE CORRUPTED DATA");

        // Assert fresh database at original path is functional
        let conn = db.connection().lock().expect("lock connection");
        let check: String = conn
            .query_row("PRAGMA quick_check(1);", [], |row| row.get(0))
            .expect("quick check on fresh db");
        assert_eq!(check, "ok");

        let current_version = highest_applied_version(&conn)
            .expect("highest version")
            .expect("applied version");
        assert_eq!(current_version, 4);
    }

    #[test]
    fn foreign_keys_and_wal_pragmas_are_enabled() {
        let mut conn = Connection::open_in_memory().expect("in-memory database");
        initialize(&mut conn).expect("initialize");

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
            .expect("foreign_keys pragma");
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn foreign_key_enforcement_blocks_orphan_findings() {
        let mut conn = Connection::open_in_memory().expect("in-memory database");
        initialize(&mut conn).expect("initialize");

        // Attempting to insert a finding without a corresponding scan session should fail
        let result = conn.execute(
            "INSERT INTO findings (id, session_id, path, size_bytes, safety_class, category)
             VALUES ('f-1', 'nonexistent-session', '/var/log', 1024, 'Rebuildable', 'Logs');",
            [],
        );
        assert!(result.is_err());

        // Inserting after creating the session succeeds
        conn.execute(
            "INSERT INTO scan_sessions (id, started_at, status, root_path)
             VALUES ('session-1', '2026-09-15T00:00:00Z', 'completed', '/');",
            [],
        )
        .expect("insert session");

        let success = conn.execute(
            "INSERT INTO findings (id, session_id, path, size_bytes, safety_class, category)
             VALUES ('f-1', 'session-1', '/var/log', 1024, 'Rebuildable', 'Logs');",
            [],
        );
        assert!(success.is_ok());
    }
}

const MIGRATION_0003_REVIEW_PLANS: &str = "
CREATE TABLE review_plans (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES scan_sessions(id) ON DELETE CASCADE,
    default_action_mode TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE plan_items (
    item_id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES review_plans(id) ON DELETE CASCADE,
    original_path TEXT NOT NULL,
    canonical_path TEXT NOT NULL,
    device_id INTEGER NOT NULL,
    inode INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    class_name TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    rule_version INTEGER NOT NULL,
    action_name TEXT NOT NULL,
    recoverable INTEGER NOT NULL
);
";

const MIGRATION_0004_OPERATIONS_AND_HISTORY: &str = "
CREATE TABLE operations (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES review_plans(id),
    action_mode TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER NOT NULL,
    succeeded_items INTEGER NOT NULL DEFAULT 0,
    failed_items INTEGER NOT NULL DEFAULT 0,
    skipped_items INTEGER NOT NULL DEFAULT 0,
    blocked_items INTEGER NOT NULL DEFAULT 0,
    vanished_items INTEGER NOT NULL DEFAULT 0,
    permission_denied_items INTEGER NOT NULL DEFAULT 0,
    bytes_pending_trash INTEGER NOT NULL DEFAULT 0,
    bytes_permanently_reclaimed INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE operation_items (
    id TEXT PRIMARY KEY NOT NULL,
    operation_id TEXT NOT NULL REFERENCES operations(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    original_path TEXT NOT NULL,
    canonical_path TEXT NOT NULL,
    trashed_path TEXT,
    device_id INTEGER NOT NULL,
    inode INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    status TEXT NOT NULL,
    bytes_pending_trash INTEGER NOT NULL DEFAULT 0,
    bytes_permanently_reclaimed INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_operation_items_op_id ON operation_items(operation_id);
CREATE INDEX idx_operation_items_status ON operation_items(status);
CREATE INDEX idx_operations_created_at ON operations(created_at_ms);

CREATE TABLE restore_outcomes (
    id TEXT PRIMARY KEY NOT NULL,
    operation_item_id TEXT NOT NULL REFERENCES operation_items(id) ON DELETE CASCADE,
    source_trashed_path TEXT NOT NULL,
    restored_to_path TEXT NOT NULL,
    device_id INTEGER NOT NULL,
    inode INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    restored_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_restore_outcomes_item_id ON restore_outcomes(operation_item_id);
";
