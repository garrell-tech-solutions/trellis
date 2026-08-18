//! The owner's timezone: one row, one column, always present (migration
//! `0006` inserts it). Everything here speaks `sqlx::Error` and knows
//! nothing of HTTP (`T-module-boundary`, enforced by `platform::boundary`).

use sqlx::SqlitePool;

pub async fn get_timezone(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT timezone FROM settings WHERE id = 1")
        .fetch_one(pool)
        .await
}

pub async fn set_timezone(pool: &SqlitePool, timezone: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE settings SET timezone = ? WHERE id = 1")
        .bind(timezone)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    #[tokio::test]
    async fn a_fresh_database_reports_utc() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(get_timezone(&pool).await.unwrap(), "UTC");
    }

    #[tokio::test]
    async fn set_timezone_changes_what_get_timezone_reports() {
        let (_dir, pool) = test_pool().await;

        set_timezone(&pool, "Europe/London").await.unwrap();

        assert_eq!(get_timezone(&pool).await.unwrap(), "Europe/London");
    }
}
