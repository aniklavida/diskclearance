use rusqlite::{Connection, Result};

const FOUNDATION_MIGRATION: &str = "
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
";

#[allow(dead_code)]
pub fn initialize(connection: &Connection) -> Result<()> {
    connection.execute_batch(FOUNDATION_MIGRATION)
}

#[cfg(test)]
mod tests {
    use super::initialize;
    use rusqlite::Connection;

    #[test]
    fn foundation_migration_is_repeatable() {
        let database = Connection::open_in_memory().expect("in-memory database");
        initialize(&database).expect("first migration");
        initialize(&database).expect("repeat migration");

        let count: i64 = database
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
                [],
                |row| row.get(0),
            )
            .expect("schema table query");

        assert_eq!(count, 1);
    }
}
