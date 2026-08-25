//! What the quota screen reads and writes: the `quotas` table (#93,
//! `D-quotas-are-selected-not-typed`) -- a different table from `tasks`,
//! which `TaskKind::Quota` keeps using untouched.

use scheduler_core::quota::QuotaDefinition;
use sqlx::SqlitePool;

/// A row of [`list_quotas`], in the order they were defined
/// (`quota-screen-defined-order-08`).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct QuotaRow {
    pub id: i64,
    pub name: String,
    pub weekly_target_minutes: i64,
}

/// Every defined quota, oldest first -- the store's own `ORDER BY`
/// (`T-set-operations-execute-in-the-store`), not a sort a caller performs
/// after fetching.
pub async fn list_quotas(pool: &SqlitePool) -> Result<Vec<QuotaRow>, sqlx::Error> {
    sqlx::query_as("SELECT id, name, weekly_target_minutes FROM quotas ORDER BY id ASC")
        .fetch_all(pool)
        .await
}

/// Every existing quota's own spelling and target, for
/// `scheduler_core::quota::check_name` to compare a candidate name against
/// before writing it.
pub async fn existing_names(pool: &SqlitePool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas ORDER BY id ASC")
        .fetch_all(pool)
        .await
}

/// Writes a new quota row. Callers must have already checked
/// `scheduler_core::quota::check_name` against [`existing_names`] -- the
/// `UNIQUE COLLATE NOCASE` constraint on `name` is the backstop for a write
/// path that forgot to (`T-collation-enforces-name-identity`), not the
/// primary guard, so a caller that skipped the check sees a constraint
/// violation here rather than a silent duplicate.
pub async fn create(
    pool: &SqlitePool,
    definition: &QuotaDefinition,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES (?, ?, ?)")
        .bind(&definition.name)
        .bind(definition.weekly_target_minutes)
        .bind(created_at_ms)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    fn definition(name: &str, weekly_target_minutes: i64) -> QuotaDefinition {
        QuotaDefinition {
            name: name.to_string(),
            weekly_target_minutes,
        }
    }

    #[tokio::test]
    async fn list_quotas_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_quotas(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn create_writes_a_quota_list_quotas_then_reports() {
        let (_dir, pool) = test_pool().await;

        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Piano");
        assert_eq!(rows[0].weekly_target_minutes, 240);
    }

    #[tokio::test]
    async fn list_quotas_orders_oldest_first() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();
        create(&pool, &definition("Running", 180), 1).await.unwrap();
        create(&pool, &definition("Rust", 300), 2).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();

        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Piano", "Running", "Rust"]);
    }

    #[tokio::test]
    async fn existing_names_reports_every_quotas_name_and_target() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let names = existing_names(&pool).await.unwrap();

        assert_eq!(names, vec![("Piano".to_string(), 240)]);
    }

    #[tokio::test]
    async fn create_refuses_a_second_row_with_the_same_name_once_case_is_folded() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let result = create(&pool, &definition("PIANO", 120), 1).await;

        assert!(
            result.is_err(),
            "expected the UNIQUE COLLATE NOCASE backstop to refuse a case-only duplicate"
        );
    }
}
