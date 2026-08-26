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

/// Where a candidate name stands against the quotas that already exist --
/// the read half of the same front door, in this capability's own terms.
///
/// Carries the matched quota's *stored* spelling and target, not the
/// candidate's: a message that says what already exists has to quote it as
/// it is written, not as it was typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameStanding {
    /// Nothing in the way.
    Free,
    /// The same quota, once case, spaces and punctuation are folded away
    /// (`T-collation-enforces-name-identity`'s rule, which has outgrown the
    /// collation and lives in `scheduler_core::quota::check_name`).
    Taken {
        name: String,
        weekly_target_minutes: i64,
    },
    /// Close enough to warn about, and no more than that.
    Resembles {
        name: String,
        weekly_target_minutes: i64,
    },
}

/// Answers the question rather than handing back the set it was answered
/// from (#138). `triage::http` used to fetch every quota and run
/// `check_name` itself; asking here keeps the *rule* in `scheduler_core`,
/// where mutation testing reaches it, while the *retrieval* stays behind
/// this capability's door -- the company standard's split of intention from
/// execution, and `T-one-front-door-per-capability`.
///
/// **This is where the tracked Gap now lives, whole.** `existing_names` has
/// no `WHERE`: the similar tier (Levenshtein, containment) cannot execute
/// in SQLite at all, and the exact tier could only against a stored
/// normalized name, which still waits on the quota identity ruling. What
/// changed is the blast radius -- one function, not two modules.
pub async fn name_standing(
    pool: &SqlitePool,
    candidate: &str,
) -> Result<NameStanding, sqlx::Error> {
    use scheduler_core::quota::NameMatch;

    let existing = store::existing_names(pool).await?;
    let standing = match scheduler_core::quota::check_name(candidate, &existing) {
        Some(NameMatch::Exact(name, weekly_target_minutes)) => NameStanding::Taken {
            name: name.to_string(),
            weekly_target_minutes,
        },
        Some(NameMatch::Similar(name, weekly_target_minutes)) => NameStanding::Resembles {
            name: name.to_string(),
            weekly_target_minutes,
        },
        None => NameStanding::Free,
    };
    Ok(standing)
}

/// `minutes` the way a quota name-collision message reads it: `"4 h a
/// week"`, never `"4h"` -- the row readout's compact form is a different
/// context with its own established spelling, and this project does not
/// invent a third.
///
/// Its only caller is now `triage::rejection`: #138 retired the quota
/// screen's define form, and with it the collision message this was first
/// written for. It stays here rather than moving with its caller because a
/// quota's target is spelled the way this capability spells it -- a refusal
/// quoting one is borrowing quota's words, not coining its own.
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
    use crate::platform::test_support::test_pool;
    use scheduler_core::quota::WeeklyTarget;

    async fn given_quota(pool: &SqlitePool, name: &str, weekly_target_minutes: i64) {
        let definition = scheduler_core::quota::QuotaDefinition {
            name: name.to_string(),
            weekly_target: WeeklyTarget::from_minutes(weekly_target_minutes)
                .expect("a positive target"),
        };
        create(pool, &definition, 0)
            .await
            .expect("the quota created");
    }

    #[tokio::test]
    async fn a_name_nothing_resembles_stands_free() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 120).await;

        assert_eq!(
            name_standing(&pool, "Running").await.unwrap(),
            NameStanding::Free
        );
    }

    #[tokio::test]
    async fn a_name_stands_free_against_no_quotas_at_all() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(
            name_standing(&pool, "Piano").await.unwrap(),
            NameStanding::Free
        );
    }

    #[tokio::test]
    async fn a_name_that_folds_onto_an_existing_one_is_taken_and_quotes_the_stored_spelling() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Pi-ano", 120).await;

        assert_eq!(
            name_standing(&pool, "pi ano").await.unwrap(),
            NameStanding::Taken {
                name: "Pi-ano".to_string(),
                weekly_target_minutes: 120,
            }
        );
    }

    #[tokio::test]
    async fn a_name_that_merely_reads_like_an_existing_one_resembles_it() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 90).await;

        assert_eq!(
            name_standing(&pool, "Pianoo").await.unwrap(),
            NameStanding::Resembles {
                name: "Piano".to_string(),
                weekly_target_minutes: 90,
            }
        );
    }

    #[test]
    fn hours_a_week_reads_a_whole_number_of_hours() {
        assert_eq!(hours_a_week(240), "4 h a week");
    }

    #[test]
    fn hours_a_week_reads_a_fractional_number_of_hours() {
        assert_eq!(hours_a_week(90), "1.5 h a week");
    }
}
