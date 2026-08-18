//! This capability's own reads and writes for the `exceptions` table
//! (`T-capability-owns-its-queries`).
//!
//! Resolving a submitted scope's life-area *name* is deliberately not here.
//! It is one question -- does this name a life area work may be filed
//! under? -- and `life_areas::active_id_for_name` answers it for every
//! boundary that asks (`T-one-front-door-per-capability`). It arrived here
//! as a byte-identical copy of the one in `triage::store`.

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
/// caller's job (`scheduler_core::exception::well_formed_range` and
/// `life_areas::active_id_for_name`) -- this is the write, not the rule.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};

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
        let work = seeded_life_area_id(&pool, "Work").await;

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
}
