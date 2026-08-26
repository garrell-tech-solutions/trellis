//! What the quota screen reads and writes: the `quotas` table (#93,
//! `D-quotas-are-selected-not-typed`) -- a different table from `tasks`,
//! which `TaskKind::Quota` keeps using untouched.

use scheduler_core::quota::{QuotaDefinition, WeeklyTarget};
use sqlx::SqlitePool;

/// A row of [`list_quotas`], in the order they were defined
/// (`quota-screen-defined-order-08`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaRow {
    pub id: i64,
    pub name: String,
    /// Already a [`WeeklyTarget`] rather than a bare count of minutes: the
    /// column's `CHECK (weekly_target_minutes > 0)` says the same thing the
    /// type does, and this is where the two meet, so nothing downstream has
    /// to re-establish it.
    pub weekly_target: WeeklyTarget,
}

/// The row exactly as the column set reads. Private: what leaves this
/// module is [`QuotaRow`], whose target has been through the domain type.
#[derive(sqlx::FromRow)]
struct StoredQuotaRow {
    id: i64,
    name: String,
    weekly_target_minutes: i64,
}

/// Every defined quota, oldest first -- the store's own `ORDER BY`
/// (`T-set-operations-execute-in-the-store`), not a sort a caller performs
/// after fetching.
///
/// A row whose target is not positive has broken its own `CHECK`, so this
/// reports it as what it is -- a value that cannot be decoded into the type
/// the row declares -- rather than passing a zero on for
/// `scheduler_core::quota::progress` to divide by. The handler already
/// turns a store error into a 500, which is the honest answer to a corrupt
/// row; nothing here can render it.
pub async fn list_quotas(pool: &SqlitePool) -> Result<Vec<QuotaRow>, sqlx::Error> {
    let stored: Vec<StoredQuotaRow> =
        sqlx::query_as("SELECT id, name, weekly_target_minutes FROM quotas ORDER BY id ASC")
            .fetch_all(pool)
            .await?;
    stored.into_iter().map(quota_row).collect()
}

fn quota_row(stored: StoredQuotaRow) -> Result<QuotaRow, sqlx::Error> {
    let weekly_target =
        WeeklyTarget::from_minutes(stored.weekly_target_minutes).ok_or_else(|| {
            sqlx::Error::Decode(
                format!(
                    "quota {} has a weekly target of {} minutes, which its own CHECK forbids",
                    stored.id, stored.weekly_target_minutes
                )
                .into(),
            )
        })?;
    Ok(QuotaRow {
        id: stored.id,
        name: stored.name,
        weekly_target,
    })
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
        .bind(definition.weekly_target.minutes())
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
            weekly_target: WeeklyTarget::from_minutes(weekly_target_minutes)
                .expect("a positive target"),
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
        assert_eq!(rows[0].weekly_target.minutes(), 240);
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

    fn stored(weekly_target_minutes: i64) -> StoredQuotaRow {
        StoredQuotaRow {
            id: 1,
            name: "Piano".to_string(),
            weekly_target_minutes,
        }
    }

    /// A row that has broken its own `CHECK` is refused here rather than
    /// handed on as a zero for `quota::progress` to divide by. The
    /// constraint makes this unreachable through the write path -- an
    /// `UPDATE` to zero is refused by SQLite itself -- so the conversion is
    /// exercised where it lives rather than through a database rigged to
    /// permit what it forbids.
    #[test]
    fn a_stored_target_that_is_not_positive_is_refused() {
        assert!(
            quota_row(stored(0)).is_err(),
            "expected a zero stored target to be refused"
        );
        assert!(
            quota_row(stored(-30)).is_err(),
            "expected a negative stored target to be refused"
        );
    }

    #[test]
    fn a_positive_stored_target_converts_to_the_domains_own_type() {
        let row = quota_row(stored(240)).expect("a positive target converts");

        assert_eq!(row.id, 1);
        assert_eq!(row.name, "Piano");
        assert_eq!(row.weekly_target.minutes(), 240);
    }

    /// The `CHECK` is the reason the conversion above is unreachable in
    /// practice, so it is worth one assertion of its own.
    #[tokio::test]
    async fn the_column_refuses_a_target_of_zero() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let result = sqlx::query("UPDATE quotas SET weekly_target_minutes = 0")
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "expected the CHECK to refuse a zero target"
        );
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
