//! This capability's own reads and writes for the `exceptions` table
//! (`T-capability-owns-its-queries`), plus its own life-area name lookup --
//! validating a submitted scope at this boundary is this capability's
//! concern, the same call `triage::store::find_active_life_area_id` already
//! made for a different boundary, not a function borrowed from
//! `life_areas`.

use sqlx::SqlitePool;

/// One stored exception, exactly as the row holds it -- dates as text,
/// scope as a nullable id. Parsing either into something a reader can use
/// is the caller's job.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ExceptionRow {
    pub id: i64,
    pub life_area_id: Option<i64>,
    pub start_date: String,
    pub end_date: String,
    pub label: String,
}

/// Every stored exception, oldest first -- the order each was added, and
/// the only order anything reads them in.
pub async fn list_all(pool: &SqlitePool) -> Result<Vec<ExceptionRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, life_area_id, start_date, end_date, label FROM exceptions ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
}

/// Inserts one exception. Well-formed dates and a resolved scope are the
/// caller's job (`scheduler_core::exception::well_formed_range` and this
/// module's own `find_active_life_area_id`) -- this is the write, not the
/// rule.
pub async fn insert(
    pool: &SqlitePool,
    life_area_id: Option<i64>,
    start_date: &str,
    end_date: &str,
    label: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO exceptions (life_area_id, start_date, end_date, label) \
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(life_area_id)
    .bind(start_date)
    .bind(end_date)
    .bind(label)
    .fetch_one(pool)
    .await
}

/// Deletes an exception outright (`D-kill-means-archive`: configuration is
/// removed, not archived -- the same call `#59` made for a guardrail band).
/// An id naming no row is a silent no-op, the same choice
/// `life_areas::store::archive` already made for the same reason: nothing
/// specifies a rejection for it.
pub async fn remove(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM exceptions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// The active life area named `name`, if any -- this capability's own
/// validation query (`T-capability-owns-its-queries`), case-insensitive by
/// the `life_areas.name` column's own collation.
pub async fn find_active_life_area_id(
    pool: &SqlitePool,
    name: &str,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM life_areas WHERE name = ? AND archived_at IS NULL")
        .bind(name)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    async fn work_id(pool: &SqlitePool) -> i64 {
        find_active_life_area_id(pool, "Work")
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn list_all_is_empty_on_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_all(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn insert_stores_a_global_exception() {
        let (_dir, pool) = test_pool().await;

        let id = insert(&pool, None, "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();

        let rows = list_all(&pool).await.unwrap();
        assert_eq!(
            rows,
            vec![ExceptionRow {
                id,
                life_area_id: None,
                start_date: "2026-08-24".to_string(),
                end_date: "2026-08-28".to_string(),
                label: "".to_string(),
            }]
        );
    }

    #[tokio::test]
    async fn insert_stores_a_life_area_scoped_exception_with_its_label() {
        let (_dir, pool) = test_pool().await;
        let work = work_id(&pool).await;

        let id = insert(&pool, Some(work), "2026-08-24", "2026-08-24", "vacation")
            .await
            .unwrap();

        let rows = list_all(&pool).await.unwrap();
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].life_area_id, Some(work));
        assert_eq!(rows[0].label, "vacation");
    }

    #[tokio::test]
    async fn insert_rejects_a_backwards_range_at_the_schema_level() {
        let (_dir, pool) = test_pool().await;

        assert!(insert(&pool, None, "2026-08-28", "2026-08-24", "")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn remove_deletes_the_named_exception_and_leaves_others() {
        let (_dir, pool) = test_pool().await;
        let kept = insert(&pool, None, "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();
        let removed = insert(&pool, None, "2026-09-01", "2026-09-02", "")
            .await
            .unwrap();

        remove(&pool, removed).await.unwrap();

        let rows = list_all(&pool).await.unwrap();
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![kept]);
    }

    #[tokio::test]
    async fn remove_of_an_unknown_id_is_a_silent_no_op() {
        let (_dir, pool) = test_pool().await;

        remove(&pool, 999).await.unwrap();

        assert_eq!(list_all(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn find_active_life_area_id_matches_case_insensitively() {
        let (_dir, pool) = test_pool().await;

        let found = find_active_life_area_id(&pool, "work").await.unwrap();

        assert!(found.is_some());
    }

    #[tokio::test]
    async fn find_active_life_area_id_reports_none_for_an_unknown_name() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(
            find_active_life_area_id(&pool, "Gardening").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn find_active_life_area_id_excludes_an_archived_life_area() {
        let (_dir, pool) = test_pool().await;
        let learning = find_active_life_area_id(&pool, "Learning")
            .await
            .unwrap()
            .unwrap();
        sqlx::query("UPDATE life_areas SET archived_at = 1000 WHERE id = ?")
            .bind(learning)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            find_active_life_area_id(&pool, "Learning").await.unwrap(),
            None
        );
    }
}
