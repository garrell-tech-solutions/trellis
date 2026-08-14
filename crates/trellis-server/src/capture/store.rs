//! Writing a capture down.
//!
//! Only the write lives here. Listing untriaged captures is the inbox's
//! query (`inbox::store`) and stamping one as consumed is triage's write
//! (`triage::store`), because a capability owns the SQL it issues rather
//! than the table it happens to touch — three domains write the `captures`
//! table and each does so from its own module. Everything here speaks
//! `sqlx::Error`; naming a status code is the delivery side's job
//! (`T-module-boundary`, enforced by `platform::boundary`).

use sqlx::SqlitePool;

/// Returns the new capture's id — the inbox row this request renders needs
/// it to aim a later triage action at.
pub async fn insert(
    pool: &SqlitePool,
    raw_text: &str,
    source: &str,
    created_at_ms: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(raw_text)
    .bind(source)
    .bind(created_at_ms)
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    #[tokio::test]
    async fn insert_returns_the_new_captures_id() {
        let (_dir, pool) = test_pool().await;

        let id = insert(&pool, "buy milk", "web", 1234).await.unwrap();

        let row_id: i64 = sqlx::query_scalar("SELECT id FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id, row_id);
    }

    #[tokio::test]
    async fn insert_stores_the_submitted_text_source_and_timestamp() {
        let (_dir, pool) = test_pool().await;

        insert(&pool, "buy milk", "web", 1234).await.unwrap();

        let row: (String, String, i64) =
            sqlx::query_as("SELECT raw_text, source, created_at_ms FROM captures")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string(), 1234));
    }

    #[tokio::test]
    async fn a_freshly_inserted_capture_is_untriaged() {
        let (_dir, pool) = test_pool().await;

        insert(&pool, "buy milk", "web", 1234).await.unwrap();

        let triaged_at: Option<i64> = sqlx::query_scalar("SELECT triaged_at FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(triaged_at, None);
    }

    #[tokio::test]
    async fn insert_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::platform::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(insert(&pool, "buy milk", "web", 0).await.is_err());
    }
}
