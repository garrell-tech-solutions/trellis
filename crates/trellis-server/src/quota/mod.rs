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
///
/// **Revives rather than inserts when the name belongs to an archived
/// quota** (#148, `quota-screen-removed-name-returns-09`). Freeing the name
/// on removal instead would mean rebuilding `quotas` against live data for
/// a case `D-quota-no-rollover`'s Monday reset already hides, so triage is
/// the one path that resolves `NameStanding::Taken { archived: true, .. }`
/// by reviving rather than refusing -- `triage::http::check_quota_name` is
/// what lets a submission reach this function at all in that case.
pub async fn create(
    pool: &SqlitePool,
    definition: &scheduler_core::quota::QuotaDefinition,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    match name_standing(pool, &definition.name, None).await? {
        NameStanding::Taken {
            name,
            archived: true,
            ..
        } => {
            store::revive(
                pool,
                &name,
                &definition.name,
                definition.weekly_target.minutes(),
            )
            .await
        }
        _ => store::create(pool, definition, created_at_ms).await,
    }
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
        /// #148: an archived quota is still `Taken` -- its name still sits
        /// in the `UNIQUE COLLATE NOCASE` column -- but triage resolves
        /// that case by reviving it, while a rename still refuses onto it
        /// (`quota-screen-rename-refused-07` weighs every quota, archived
        /// or not).
        archived: bool,
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
/// `exclude_id` leaves one quota out of the comparison entirely -- a rename
/// or retarget (#148) must not refuse a submission for colliding with the
/// very row it is changing, which a plain "does this name exist" check
/// cannot tell from a genuine duplicate. `None` for every other caller:
/// triage is naming a quota that does not exist yet, so nothing is ever
/// excluded on its behalf.
///
/// **This is where the tracked Gap now lives, whole.** `store::existing`
/// has no similarity `WHERE`: the similar tier (Levenshtein, containment)
/// cannot execute in SQLite at all, and the exact tier could only against a
/// stored normalized name, which still waits on the quota identity ruling.
/// What changed is the blast radius -- one function, not two modules.
pub async fn name_standing(
    pool: &SqlitePool,
    candidate: &str,
    exclude_id: Option<i64>,
) -> Result<NameStanding, sqlx::Error> {
    use scheduler_core::quota::NameMatch;

    let existing = store::existing(pool).await?;
    let candidates: Vec<(String, i64)> = existing
        .iter()
        .filter(|q| Some(q.id) != exclude_id)
        .map(|q| (q.name.clone(), q.weekly_target_minutes))
        .collect();
    let standing = match scheduler_core::quota::check_name(candidate, &candidates) {
        Some(NameMatch::Exact(name, weekly_target_minutes)) => {
            let archived = existing
                .iter()
                .find(|q| q.name == name)
                .is_some_and(|q| q.archived);
            NameStanding::Taken {
                name: name.to_string(),
                weekly_target_minutes,
                archived,
            }
        }
        Some(NameMatch::Similar(name, weekly_target_minutes)) => NameStanding::Resembles {
            name: name.to_string(),
            weekly_target_minutes,
        },
        None => NameStanding::Free,
    };
    Ok(standing)
}

/// Renames and retargets `id` in one gesture (#148,
/// `T-set-operations-execute-in-the-store`), or refuses -- the same
/// `NameStanding` guard triage's own creation door reuses
/// (`T-one-front-door-per-capability`: a rename that skipped it could
/// produce the exact duplicate the create path refuses).
///
/// Unlike triage, `Taken` is always refused here, archived or not
/// (`quota-screen-rename-refused-07`): only triage resolves that case by
/// reviving, because only triage is naming a quota that does not otherwise
/// exist on screen. `Resembles` is not checked -- there is no scenario
/// asking this door to warn-and-confirm the way triage's does, and adding
/// that flow here would be inventing behavior nobody asked for.
pub async fn change(
    pool: &SqlitePool,
    id: i64,
    name: Option<&str>,
    hours: Option<&str>,
) -> Result<Result<(), String>, sqlx::Error> {
    let definition = match scheduler_core::quota::QuotaDefinition::from_fields(name, hours) {
        Ok(definition) => definition,
        Err(rejection) => return Ok(Err(definition_rejection_message(rejection))),
    };
    match name_standing(pool, &definition.name, Some(id)).await? {
        NameStanding::Taken {
            name,
            weekly_target_minutes,
            ..
        } => Ok(Err(name_exists_message(&name, weekly_target_minutes))),
        _ => {
            store::update(
                pool,
                id,
                &definition.name,
                definition.weekly_target.minutes(),
            )
            .await?;
            Ok(Ok(()))
        }
    }
}

/// Removes `id` -- the write half of #148's other new door
/// (`T-one-front-door-per-capability`): `remove means archive`, so this is
/// `store::archive` and nothing more. No name to check, no rejection to
/// report -- a removal cannot collide with anything.
pub async fn remove(pool: &SqlitePool, id: i64, archived_at_ms: i64) -> Result<(), sqlx::Error> {
    store::archive(pool, id, archived_at_ms).await
}

/// [`scheduler_core::quota::DefinitionRejection`] as one line of prose for
/// [`change`]'s own row -- the same two fields
/// [`scheduler_core::task::TriageRejection`]'s core rejections read, in
/// this capability's own words rather than triage's, since this door is
/// reached without ever going through triage at all.
fn definition_rejection_message(rejection: scheduler_core::quota::DefinitionRejection) -> String {
    use scheduler_core::quota::{DefinitionRejection, Field};
    match rejection {
        DefinitionRejection::MissingField(Field::Name) => "name is required".to_string(),
        DefinitionRejection::MissingField(Field::Hours) => "hours is required".to_string(),
        DefinitionRejection::InvalidField(Field::Name) => "name is invalid".to_string(),
        DefinitionRejection::InvalidField(Field::Hours) => "hours is invalid".to_string(),
    }
}

/// The message a taken name produces wherever that refusal happens --
/// triage's own creation door (`triage::rejection::Rejection::
/// QuotaNameExists`) and [`change`]'s rename door alike, so a rename that
/// collides reads exactly like triage's own refusal (`T-422-is-product-
/// wide`): the same fact refused the same way, not two independently
/// maintained sentences that could drift apart.
pub(crate) fn name_exists_message(existing_name: &str, existing_minutes: i64) -> String {
    format!(
        "\u{201c}{existing_name}\u{201d} already exists at {}. Log your time against that \
         one, or give this a different name.",
        hours_a_week(existing_minutes)
    )
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

    async fn quota_id(pool: &SqlitePool, name: &str) -> i64 {
        sqlx::query_scalar("SELECT id FROM quotas WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_name_nothing_resembles_stands_free() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 120).await;

        assert_eq!(
            name_standing(&pool, "Running", None).await.unwrap(),
            NameStanding::Free
        );
    }

    #[tokio::test]
    async fn a_name_stands_free_against_no_quotas_at_all() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(
            name_standing(&pool, "Piano", None).await.unwrap(),
            NameStanding::Free
        );
    }

    #[tokio::test]
    async fn a_name_that_folds_onto_an_existing_one_is_taken_and_quotes_the_stored_spelling() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Pi-ano", 120).await;

        assert_eq!(
            name_standing(&pool, "pi ano", None).await.unwrap(),
            NameStanding::Taken {
                name: "Pi-ano".to_string(),
                weekly_target_minutes: 120,
                archived: false,
            }
        );
    }

    #[tokio::test]
    async fn a_name_that_merely_reads_like_an_existing_one_resembles_it() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 90).await;

        assert_eq!(
            name_standing(&pool, "Pianoo", None).await.unwrap(),
            NameStanding::Resembles {
                name: "Piano".to_string(),
                weekly_target_minutes: 90,
            }
        );
    }

    #[tokio::test]
    async fn a_name_taken_by_an_archived_quota_is_taken_and_says_so() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 120).await;
        let id = quota_id(&pool, "Piano").await;
        remove(&pool, id, 4242).await.unwrap();

        assert_eq!(
            name_standing(&pool, "Piano", None).await.unwrap(),
            NameStanding::Taken {
                name: "Piano".to_string(),
                weekly_target_minutes: 120,
                archived: true,
            }
        );
    }

    #[tokio::test]
    async fn excluding_a_quotas_own_id_frees_its_own_name_for_it() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 120).await;
        let id = quota_id(&pool, "Piano").await;

        assert_eq!(
            name_standing(&pool, "Piano", Some(id)).await.unwrap(),
            NameStanding::Free,
            "a quota renaming itself to its own name must not collide with itself"
        );
    }

    #[tokio::test]
    async fn excluding_a_different_id_still_catches_the_collision() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 120).await;
        given_quota(&pool, "Running", 90).await;
        let running_id = quota_id(&pool, "Running").await;

        assert_eq!(
            name_standing(&pool, "Piano", Some(running_id))
                .await
                .unwrap(),
            NameStanding::Taken {
                name: "Piano".to_string(),
                weekly_target_minutes: 120,
                archived: false,
            }
        );
    }

    // --- create revives an archived name rather than duplicating it (#148) --

    #[tokio::test]
    async fn create_revives_an_archived_quota_of_the_same_name() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;
        remove(&pool, id, 4242).await.unwrap();

        given_quota(&pool, "Piano", 120).await;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "one quota, not two");
        assert_eq!(quota_id(&pool, "Piano").await, id, "the same identity");
    }

    #[tokio::test]
    async fn create_still_inserts_a_fresh_row_for_a_free_name() {
        let (_dir, pool) = test_pool().await;

        given_quota(&pool, "Piano", 240).await;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    // --- change: rename and retarget, or refuse (#148) ----------------------

    #[tokio::test]
    async fn change_renames_and_retargets() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;

        let outcome = change(&pool, id, Some("Piano theory"), Some("2"))
            .await
            .unwrap();

        assert_eq!(outcome, Ok(()));
        let (name, minutes): (String, i64) =
            sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "Piano theory");
        assert_eq!(minutes, 120);
    }

    #[tokio::test]
    async fn change_to_its_own_unchanged_name_is_not_a_collision() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;

        let outcome = change(&pool, id, Some("Piano"), Some("2")).await.unwrap();

        assert_eq!(outcome, Ok(()));
    }

    #[tokio::test]
    async fn change_onto_a_name_another_live_quota_holds_is_refused() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        given_quota(&pool, "Running", 180).await;
        let running_id = quota_id(&pool, "Running").await;

        let outcome = change(&pool, running_id, Some("piano"), Some("3"))
            .await
            .unwrap();

        assert_eq!(
            outcome,
            Err(name_exists_message("Piano", 240)),
            "the same message triage's own refusal reads"
        );
        let (name, minutes): (String, i64) =
            sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas WHERE id = ?")
                .bind(running_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(name, "Running", "the refused row must be unchanged");
        assert_eq!(minutes, 180);
    }

    #[tokio::test]
    async fn change_onto_an_archived_name_is_still_refused() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let piano_id = quota_id(&pool, "Piano").await;
        remove(&pool, piano_id, 4242).await.unwrap();
        given_quota(&pool, "Running", 180).await;
        let running_id = quota_id(&pool, "Running").await;

        let outcome = change(&pool, running_id, Some("Piano"), Some("3"))
            .await
            .unwrap();

        assert!(outcome.is_err(), "only triage revives an archived name");
    }

    #[tokio::test]
    async fn change_rejects_a_missing_name() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;

        let outcome = change(&pool, id, None, Some("2")).await.unwrap();

        assert_eq!(outcome, Err("name is required".to_string()));
    }

    #[tokio::test]
    async fn change_rejects_invalid_hours() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;

        let outcome = change(&pool, id, Some("Piano"), Some("0")).await.unwrap();

        assert_eq!(outcome, Err("hours is invalid".to_string()));
    }

    // --- remove: archive, not delete (#148) ----------------------------------

    #[tokio::test]
    async fn remove_archives_rather_than_deletes() {
        let (_dir, pool) = test_pool().await;
        given_quota(&pool, "Piano", 240).await;
        let id = quota_id(&pool, "Piano").await;

        remove(&pool, id, 4242).await.unwrap();

        let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(still_there, 1, "nothing was deleted");
    }

    #[test]
    fn hours_a_week_reads_a_whole_number_of_hours() {
        assert_eq!(hours_a_week(240), "4 h a week");
    }

    #[test]
    fn hours_a_week_reads_a_fractional_number_of_hours() {
        assert_eq!(hours_a_week(90), "1.5 h a week");
    }

    #[test]
    fn name_exists_message_matches_triages_own_refusal_wording() {
        assert_eq!(
            name_exists_message("Piano", 240),
            "\u{201c}Piano\u{201d} already exists at 4 h a week. Log your time against that \
             one, or give this a different name."
        );
    }
}
