//! **Settings** -- the owner's timezone, one value for the whole product
//! (`D-single-user`: one zone, not one per guardrail, #59). Not a life
//! area's concern and not `platform` machinery: it is data the owner
//! changes, the same way a life area's name is.
//!
//! [`store`] holds the one row. [`http`] serves `POST /timezone`; there is
//! no `GET` of its own and no page of its own -- the value renders on the
//! life areas page (`T-nav-is-the-site-map` would put a whole route in the
//! header for one field), which reaches in here through this front door
//! (`T-one-front-door-per-capability`).

pub mod http;
pub mod store;

use sqlx::SqlitePool;

/// What every other capability asks this one for: the owner's current
/// timezone.
pub(crate) async fn current_timezone(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    store::get_timezone(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    #[tokio::test]
    async fn a_fresh_database_reports_utc() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(current_timezone(&pool).await.unwrap(), "UTC");
    }
}
