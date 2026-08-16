//! Life areas' own queries: listing what is still active, finding one by
//! name (to refuse a duplicate before insert), adding one, and archiving one.
//!
//! Everything here speaks `sqlx::Error` and knows nothing of HTTP
//! (`T-module-boundary`, enforced by `platform::boundary`). What triage does
//! with a life area at the triage boundary is *not* here -- validating a
//! submitted name against this table is triage's own query, in
//! `triage/store.rs` (`T-capability-owns-its-queries`: "if triage needs to
//! validate a life area id, that query is triage's ... not a function
//! borrowed from the life-areas module"). This module owns the SQL that
//! *is* the life-areas capability: what the management page lists, and what
//! it writes.

use sqlx::SqlitePool;

/// A life area as any reader of the list needs it: enough to display it and
/// to aim an archive control at it.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct LifeAreaRow {
    pub id: i64,
    pub name: String,
}

/// Every life area not yet archived, oldest first -- the seed order, then
/// the order each was added. `T-archived-at-only`: archived is the one
/// signal, so this is the one query every picker and the management page's
/// own list both need.
pub async fn list_active(pool: &SqlitePool) -> Result<Vec<LifeAreaRow>, sqlx::Error> {
    sqlx::query_as("SELECT id, name FROM life_areas WHERE archived_at IS NULL ORDER BY id ASC")
        .fetch_all(pool)
        .await
}

/// The row named `name`, whatever its archived state -- duplicate names are
/// refused globally, not only among active rows, so a re-add cannot collide
/// with an archived one either (the `UNIQUE COLLATE NOCASE` constraint the
/// migration puts on the column agrees, and this is the query that lets the
/// application refuse it with a message before the constraint ever fires).
/// Case-insensitive by the column's own collation.
pub async fn find_by_name(
    pool: &SqlitePool,
    name: &str,
) -> Result<Option<LifeAreaRow>, sqlx::Error> {
    sqlx::query_as("SELECT id, name FROM life_areas WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await
}

/// Inserts a new life area and returns its id. `name` is assumed already
/// well-formed (`scheduler_core::life_area::parse_name`) and not a duplicate
/// (`find_by_name`) -- both are the caller's job, in that order.
pub async fn insert(pool: &SqlitePool, name: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO life_areas (name) VALUES (?) RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await
}

/// Stamps `archived_at`, taking the row out of every active listing without
/// deleting it (`D-kill-means-archive`). A row already archived, or an id
/// naming no row, is a silent no-op -- neither is a case this slice
/// specifies a rejection for (there is no un-archive to guard against
/// double-firing, and archiving a nonexistent id has no test naming what it
/// should do).
pub async fn archive(pool: &SqlitePool, id: i64, archived_at_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE life_areas SET archived_at = ? WHERE id = ? AND archived_at IS NULL")
        .bind(archived_at_ms)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    #[tokio::test]
    async fn list_active_reports_the_five_seeded_life_areas_in_order() {
        let (_dir, pool) = test_pool().await;

        let areas = list_active(&pool).await.unwrap();

        assert_eq!(
            areas.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["Work", "Fitness", "Learning", "Family", "Home"]
        );
    }

    #[tokio::test]
    async fn list_active_excludes_an_archived_life_area() {
        let (_dir, pool) = test_pool().await;
        let learning = find_by_name(&pool, "Learning").await.unwrap().unwrap();

        archive(&pool, learning.id, 1_000).await.unwrap();

        let areas = list_active(&pool).await.unwrap();
        assert!(!areas.iter().any(|a| a.name == "Learning"));
    }

    #[tokio::test]
    async fn find_by_name_matches_case_insensitively() {
        let (_dir, pool) = test_pool().await;

        let found = find_by_name(&pool, "work").await.unwrap();

        assert_eq!(found.map(|a| a.name), Some("Work".to_string()));
    }

    #[tokio::test]
    async fn find_by_name_reports_none_for_a_name_that_does_not_exist() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(find_by_name(&pool, "Gardening").await.unwrap(), None);
    }

    #[tokio::test]
    async fn find_by_name_still_finds_an_archived_row() {
        let (_dir, pool) = test_pool().await;
        let learning = find_by_name(&pool, "Learning").await.unwrap().unwrap();
        archive(&pool, learning.id, 1_000).await.unwrap();

        let found = find_by_name(&pool, "Learning").await.unwrap();

        assert_eq!(found.map(|a| a.id), Some(learning.id));
    }

    #[tokio::test]
    async fn insert_adds_a_new_active_life_area() {
        let (_dir, pool) = test_pool().await;

        let id = insert(&pool, "Side project").await.unwrap();

        let areas = list_active(&pool).await.unwrap();
        assert!(areas.iter().any(|a| a.id == id && a.name == "Side project"));
    }

    #[tokio::test]
    async fn insert_appends_after_the_seed() {
        let (_dir, pool) = test_pool().await;

        insert(&pool, "Side project").await.unwrap();

        let areas = list_active(&pool).await.unwrap();
        assert_eq!(areas.last().map(|a| a.name.as_str()), Some("Side project"));
    }

    #[tokio::test]
    async fn insert_rejects_a_case_insensitive_duplicate_at_the_schema_level() {
        let (_dir, pool) = test_pool().await;

        assert!(insert(&pool, "work").await.is_err());
    }

    #[tokio::test]
    async fn archive_stamps_the_named_row_and_leaves_others_untouched() {
        let (_dir, pool) = test_pool().await;
        let learning = find_by_name(&pool, "Learning").await.unwrap().unwrap();

        archive(&pool, learning.id, 4242).await.unwrap();

        let archived_at: Option<i64> =
            sqlx::query_scalar("SELECT archived_at FROM life_areas WHERE id = ?")
                .bind(learning.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(archived_at, Some(4242));

        let work_archived_at: Option<i64> =
            sqlx::query_scalar("SELECT archived_at FROM life_areas WHERE name = 'Work'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(work_archived_at, None);
    }

    #[tokio::test]
    async fn archive_does_not_delete_the_row() {
        let (_dir, pool) = test_pool().await;
        let learning = find_by_name(&pool, "Learning").await.unwrap().unwrap();

        archive(&pool, learning.id, 1_000).await.unwrap();

        let still_present = find_by_name(&pool, "Learning").await.unwrap();
        assert!(still_present.is_some());
    }
}
