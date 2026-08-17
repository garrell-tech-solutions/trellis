//! Dismissal's own queries against `captures`: whether one is still open,
//! and the write that takes it out of the inbox without creating a task.
//!
//! `captures` is already written by three domains from their own modules
//! (`capture::store`, `triage::store`, and this one) — a capability owns the
//! SQL it issues against a table it touches, not the table itself
//! (`T-capability-owns-its-queries`). Everything here speaks `sqlx::Error`
//! and knows nothing of HTTP (`T-module-boundary`, enforced by
//! `platform::boundary`).

use sqlx::SqlitePool;

/// Whether `capture_id` is still eligible to be dismissed: it exists and has
/// not already left the inbox, by either exit. Dismissal's own copy of the
/// same question `triage::store::capture_is_open` asks — each capability
/// owns the check against the table it touches rather than one lending it to
/// the other.
pub async fn capture_is_open(pool: &SqlitePool, capture_id: i64) -> Result<bool, sqlx::Error> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ? AND left_inbox_at IS NULL")
            .bind(capture_id)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

/// Guarded by `left_inbox_at IS NULL`, the same defence-in-depth
/// `triage::store::mark_triaged` gives its own write: whichever of triage or
/// dismissal gets there first is the one that sticks.
pub async fn mark_dismissed(
    pool: &SqlitePool,
    capture_id: i64,
    left_inbox_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE captures SET left_inbox_at = ? WHERE id = ? AND left_inbox_at IS NULL")
        .bind(left_inbox_at_ms)
        .bind(capture_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    async fn given_a_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        crate::capture::store::insert(pool, raw_text, "web", 0)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn capture_is_open_is_true_for_a_freshly_inserted_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        assert!(capture_is_open(&pool, capture_id).await.unwrap());
    }

    #[tokio::test]
    async fn capture_is_open_is_false_once_dismissed() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;
        mark_dismissed(&pool, capture_id, 9999).await.unwrap();

        assert!(!capture_is_open(&pool, capture_id).await.unwrap());
    }

    #[tokio::test]
    async fn capture_is_open_is_false_once_triaged() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;
        crate::triage::store::mark_triaged(&pool, capture_id, 9999)
            .await
            .unwrap();

        assert!(!capture_is_open(&pool, capture_id).await.unwrap());
    }

    #[tokio::test]
    async fn capture_is_open_is_false_for_an_id_naming_no_capture() {
        let (_dir, pool) = test_pool().await;

        assert!(!capture_is_open(&pool, 999).await.unwrap());
    }

    #[tokio::test]
    async fn mark_dismissed_stamps_the_named_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        mark_dismissed(&pool, capture_id, 4242).await.unwrap();

        let left_inbox_at: Option<i64> =
            sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(left_inbox_at, Some(4242));
    }

    #[tokio::test]
    async fn mark_dismissed_leaves_other_captures_untouched() {
        let (_dir, pool) = test_pool().await;
        let dismissed = given_a_capture(&pool, "buy milk").await;
        let untouched = given_a_capture(&pool, "call the dentist").await;

        mark_dismissed(&pool, dismissed, 9999).await.unwrap();

        let left_inbox_at: Option<i64> =
            sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
                .bind(untouched)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(left_inbox_at, None);
    }

    #[tokio::test]
    async fn mark_dismissed_does_not_delete_the_row() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;

        mark_dismissed(&pool, capture_id, 9999).await.unwrap();

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn mark_dismissed_is_a_noop_once_already_triaged() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_capture(&pool, "buy milk").await;
        crate::triage::store::mark_triaged(&pool, capture_id, 1111)
            .await
            .unwrap();

        mark_dismissed(&pool, capture_id, 2222).await.unwrap();

        let left_inbox_at: Option<i64> =
            sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(left_inbox_at, Some(1111));
    }
}
