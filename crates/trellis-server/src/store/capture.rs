//! The `captures` table.

use sqlx::SqlitePool;

/// A capture as the inbox view needs it: just enough to render a row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UntriagedCapture {
    pub raw_text: String,
}

/// Untriaged captures, newest first — the inbox's contents.
pub async fn list_untriaged(pool: &SqlitePool) -> Result<Vec<UntriagedCapture>, sqlx::Error> {
    sqlx::query_as("SELECT raw_text FROM captures WHERE triaged_at IS NULL ORDER BY id DESC")
        .fetch_all(pool)
        .await
}

pub async fn insert(
    pool: &SqlitePool,
    raw_text: &str,
    source: &str,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, ?, ?)")
        .bind(raw_text)
        .bind(source)
        .bind(created_at_ms)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_triaged(
    pool: &SqlitePool,
    capture_id: i64,
    triaged_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE captures SET triaged_at = ? WHERE id = ?")
        .bind(triaged_at_ms)
        .bind(capture_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;

    async fn insert_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
        )
        .bind(raw_text)
        .fetch_one(pool)
        .await
        .unwrap()
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
    async fn mark_triaged_stamps_the_named_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk").await;

        mark_triaged(&pool, capture_id, 9999).await.unwrap();

        let triaged_at: Option<i64> =
            sqlx::query_scalar("SELECT triaged_at FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(triaged_at, Some(9999));
    }

    #[tokio::test]
    async fn mark_triaged_leaves_other_captures_untouched() {
        let (_dir, pool) = test_pool().await;
        let triaged = insert_capture(&pool, "buy milk").await;
        let untouched = insert_capture(&pool, "call the dentist").await;

        mark_triaged(&pool, triaged, 9999).await.unwrap();

        let triaged_at: Option<i64> =
            sqlx::query_scalar("SELECT triaged_at FROM captures WHERE id = ?")
                .bind(untouched)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(triaged_at, None);
    }

    #[tokio::test]
    async fn insert_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(insert(&pool, "buy milk", "web", 0).await.is_err());
    }

    #[tokio::test]
    async fn list_untriaged_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_untriaged(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_untriaged_lists_captures_newest_first() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "call the dentist").await;
        insert_capture(&pool, "buy milk").await;

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(
            captures
                .iter()
                .map(|c| c.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["buy milk", "call the dentist"]
        );
    }

    #[tokio::test]
    async fn list_untriaged_excludes_a_triaged_capture() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk").await;
        let triaged = insert_capture(&pool, "call the dentist").await;
        mark_triaged(&pool, triaged, 9999).await.unwrap();

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].raw_text, "buy milk");
    }
}
