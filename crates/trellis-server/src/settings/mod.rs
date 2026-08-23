//! **Settings** -- the owner's timezone, one value for the whole product
//! (`D-single-user`: one zone, not one per guardrail, #59).
//!
//! [`store`] holds the one row. [`http`] serves `POST /timezone`. Kept
//! deliberately by #88 even though its former reader (`free_time`) and its
//! former page (life areas) are both gone: `D-menu-is-a-worklist` gives #85
//! inline controls that read the timezone through this module, and the
//! value itself is the owner's data, not scaffolding for a page that no
//! longer exists. Until #85 lands there is no way to change it from the
//! running app -- a stated one-way door, not an oversight.

pub mod http;
mod store;

use sqlx::SqlitePool;

/// The owner's configured zone -- the front door #110 needed the moment a
/// second capability first had a reason to read it (`committed`, for
/// rendering a date cell in the owner's timezone rather than UTC; `triage`,
/// for converting a page-submitted local date into an instant). Reaching
/// `settings::store::get_timezone` directly would be exactly the sideways
/// dependency `T-one-front-door-per-capability` exists to catch.
pub(crate) async fn current_timezone(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    store::get_timezone(pool).await
}
