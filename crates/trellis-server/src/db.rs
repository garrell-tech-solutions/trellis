use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    SqlitePoolOptions::new().connect_with(options).await
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn connected_test_db() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = connect(&db_path).await.unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn migrations_apply_to_empty_database_and_enable_wal() {
        let (_dir, pool) = connected_test_db().await;

        run_migrations(&pool).await.unwrap();

        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal_mode, "wal");
    }

    #[tokio::test]
    async fn rerunning_migrations_against_already_migrated_database_is_a_noop() {
        let (_dir, pool) = connected_test_db().await;
        run_migrations(&pool).await.unwrap();
        let schema_before = table_names(&pool).await;

        run_migrations(&pool).await.unwrap();

        let schema_after = table_names(&pool).await;
        assert_eq!(schema_before, schema_after);
    }

    async fn table_names(pool: &sqlx::SqlitePool) -> Vec<String> {
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(pool)
            .await
            .unwrap()
    }

    /// Folded in from the PR #31 review: SQLite's INTEGER *affinity* does not
    /// reject text, so before 0003 added this `CHECK`, `deadline='banana'`
    /// succeeded through direct SQL even though the triage boundary already
    /// rejected it — the column had no defence of its own.
    #[tokio::test]
    async fn a_non_integer_deadline_is_rejected_by_the_schema_even_via_direct_sql() {
        let (_dir, pool) = connected_test_db().await;
        run_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES ('buy milk', 'web', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = sqlx::query(
            "INSERT INTO tasks (capture_id, kind, deadline, created_at_ms) \
             VALUES (1, 'committed', 'banana', 0)",
        )
        .execute(&pool)
        .await;

        assert!(
            result.is_err(),
            "a non-integer deadline should be rejected by the schema"
        );
    }
}
