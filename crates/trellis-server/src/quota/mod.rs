//! **Quota** — `GET /quota`, the fourth Menu tab: a named container with a
//! weekly hour target. Triaging a capture as `quota` is what *creates* one
//! (#138, `crate::triage::http`); this screen only displays and logs
//! sessions against what triage produced -- it defines nothing itself.
//!
//! [`store`] owns the tables this screen needs; [`view`] turns its rows
//! into the view the template renders; [`body`] is the `#quota-body`
//! fragment `GET /quota` and every session write swap in, the same shape
//! `pool::body` and `committed::body` take.

mod body;
pub mod http;
pub mod store;
pub mod view;

use sqlx::SqlitePool;

/// Creates a quota -- the write half of #138's front door for `triage::http`
/// (`T-one-front-door-per-capability`: a capability that another capability
/// writes into exposes one function for it here, not its `store` directly).
pub async fn create(
    pool: &SqlitePool,
    definition: &scheduler_core::quota::QuotaDefinition,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    store::create(pool, definition, created_at_ms).await
}

/// Every existing quota's own spelling and target -- the read half of the
/// same front door, for `triage::http`'s own name-collision guard.
pub async fn existing_names(pool: &SqlitePool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    store::existing_names(pool).await
}

/// `minutes` the way a quota name-collision message reads it: `"4 h a
/// week"`, never `"4h"` -- the row readout's compact form is a different
/// context with its own established spelling, and this project does not
/// invent a third. Shared by `quota::http`'s own messages and
/// `triage::http`'s (#138, moved from the retired quota-screen define form
/// to triage's own front door).
pub(crate) fn hours_a_week(minutes: i64) -> String {
    if minutes % 60 == 0 {
        format!("{} h a week", minutes / 60)
    } else {
        format!("{:.1} h a week", minutes as f64 / 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_a_week_reads_a_whole_number_of_hours() {
        assert_eq!(hours_a_week(240), "4 h a week");
    }

    #[test]
    fn hours_a_week_reads_a_fractional_number_of_hours() {
        assert_eq!(hours_a_week(90), "1.5 h a week");
    }
}
