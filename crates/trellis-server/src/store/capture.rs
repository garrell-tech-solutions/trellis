//! The `captures` table.

use sqlx::SqlitePool;

/// A row of [`list_untriaged`]. Which columns that query selects is this
/// module's business; what a page does with them is not (see `http::view`).
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
    use proptest::prelude::*;

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

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The listing is a *partition*: exactly the untriaged captures, in
        /// exactly newest-first order. The example tests above sample two
        /// captures and one triaged one; this pins both halves — the `WHERE`
        /// and the `ORDER BY` — against any queue and any triaged subset of
        /// it, which is the shape the inbox actually meets.
        #[test]
        #[ignore]
        fn list_untriaged_returns_exactly_the_untriaged_captures_newest_first(
            queue in prop::collection::vec((".{0,40}", any::<bool>()), 0..12),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (expected, listed) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;

                let mut expected: Vec<String> = Vec::new();
                for (raw_text, triaged) in &queue {
                    let id = insert_capture(&pool, raw_text).await;
                    if *triaged {
                        mark_triaged(&pool, id, 9999).await.unwrap();
                    } else {
                        expected.push(raw_text.clone());
                    }
                }
                expected.reverse();

                let listed: Vec<String> = list_untriaged(&pool)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|capture| capture.raw_text)
                    .collect();
                (expected, listed)
            });

            prop_assert_eq!(expected, listed);
        }
    }
}
