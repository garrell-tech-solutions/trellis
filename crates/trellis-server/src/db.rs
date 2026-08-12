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
}
