//! **Quota** — `GET /quota`, the fourth Menu tab: a named container with a
//! weekly hour target. Triaging a capture as `quota` is what *creates* one
//! (#138, `crate::triage::http`); this screen only displays and logs
//! sessions against what triage produced -- it defines nothing itself.
//!
//! [`store`] owns the tables this screen needs; [`view`] turns its rows
//! into the view the template renders; [`body`] is the `#quota-body`
//! fragment `GET /quota` and every session write swap in, the same shape
//! `pool::body` and `committed::body` take.
//!
//! **All three are private, so this file is the whole of what another
//! capability can reach.** `create`, `name_standing`, `change`, `remove`
//! and the two message helpers below are the front door
//! (`T-one-front-door-per-capability`); `http` is public only because
//! `platform::app` mounts its handlers on routes. #148 grew that door by
//! four functions, which is exactly when it is worth having the compiler
//! hold the line rather than a comment and a CI gate --
//! `scripts/ci/capability_front_doors.sh` still checks the tree, but for
//! this capability it can no longer be the thing that catches a reach: a
//! reach does not compile. `inbox` says the same of itself and got there
//! the same way.

mod body;
pub mod http;
mod store;
mod view;

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
    let existing = store::existing(pool, exclude_id).await?;
    let candidates: Vec<(String, i64)> = existing
        .iter()
        .map(|q| (q.name.clone(), q.weekly_target_minutes))
        .collect();
    let matched = scheduler_core::quota::check_name(candidate, &candidates);
    Ok(standing_from_match(matched, &existing))
}

/// [`NameStanding`] for what [`scheduler_core::quota::check_name`] found --
/// the domain translation half of [`name_standing`], with no database of
/// its own. Takes `existing` rather than re-querying: an exact match still
/// needs it, to say whether the row it found is currently archived.
fn standing_from_match(
    matched: Option<scheduler_core::quota::NameMatch>,
    existing: &[store::ExistingQuota],
) -> NameStanding {
    use scheduler_core::quota::NameMatch;

    match matched {
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
    }
}

/// [`change`]'s own name guard: `Some` message when `name` collides with a
/// live quota other than `id` itself, `None` when nothing is in the way.
/// Unlike triage, `Taken` is always refused here, archived or not
/// (`quota-screen-rename-refused-07`): only triage resolves that case by
/// reviving, because only triage is naming a quota that does not otherwise
/// exist on screen. `Resembles` is not checked -- there is no scenario
/// asking this door to warn-and-confirm the way triage's does, and adding
/// that flow here would be inventing behavior nobody asked for.
async fn change_name_rejection(
    pool: &SqlitePool,
    id: i64,
    name: &str,
) -> Result<Option<String>, sqlx::Error> {
    match name_standing(pool, name, Some(id)).await? {
        NameStanding::Taken {
            name,
            weekly_target_minutes,
            ..
        } => Ok(Some(name_exists_message(&name, weekly_target_minutes))),
        _ => Ok(None),
    }
}

/// Renames and retargets `id` in one gesture (#148,
/// `T-set-operations-execute-in-the-store`), or refuses -- the same
/// `NameStanding` guard triage's own creation door reuses
/// (`T-one-front-door-per-capability`: a rename that skipped it could
/// produce the exact duplicate the create path refuses).
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
    if let Some(message) = change_name_rejection(pool, id, &definition.name).await? {
        return Ok(Err(message));
    }
    store::update(
        pool,
        id,
        &definition.name,
        definition.weekly_target.minutes(),
    )
    .await?;
    Ok(Ok(()))
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
/// Read by [`name_exists_message`] just above, and by `triage::rejection`
/// for the resemblance warning triage alone offers. #138 retired the quota
/// screen's define form and with it the collision message this was first
/// written for; #148 gave it a second refusal to word, on this screen's own
/// rename door. It stays here rather than moving to either caller because a
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
    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq, proptest, test_runner::Config as ProptestConfig};
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

    // --- The two halves of #148, as properties ---------------------------

    /// Characters `T-collation-enforces-name-identity` folds away entirely.
    const SEPARATORS: [&str; 5] = ["-", " ", "_", ".", "'"];

    /// One name, spelled two ways: same alphanumeric runs in the same order,
    /// different punctuation and different case. ASCII-only on purpose --
    /// `to_uppercase` expands some characters into several, which would
    /// change the letters rather than only their case.
    fn any_respelling() -> impl Strategy<Value = (String, String)> {
        (
            prop::collection::vec("[a-zA-Z0-9]{1,6}", 1..4),
            prop::collection::vec(prop::sample::select(SEPARATORS.as_slice()), 1..3),
            prop::collection::vec(prop::sample::select(SEPARATORS.as_slice()), 1..3),
            any::<bool>(),
        )
            .prop_map(|(runs, first, second, shout)| {
                let spell = |seps: &Vec<&str>| {
                    let mut out = runs[0].clone();
                    for (i, run) in runs.iter().enumerate().skip(1) {
                        out.push_str(seps[(i - 1) % seps.len()]);
                        out.push_str(run);
                    }
                    out
                };
                let other = spell(&second);
                let other = if shout {
                    other.to_uppercase()
                } else {
                    other.to_lowercase()
                };
                (spell(&first), other)
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

        /// **A quota never collides with itself.** However the owner
        /// respells a name they already have, on the quota they already have
        /// it on, [`change`] accepts it -- `workout` becomes `Workout`
        /// (#148). This is the guarantee `store::existing`'s `exclude`
        /// exists for, and it is exactly the one an example test cannot
        /// pin: `check_name` folds case, spaces and punctuation, so the set
        /// of spellings that must be accepted here is unbounded.
        ///
        /// Its mirror is in the same run: the *other* quota's name,
        /// respelled, is still refused. An `exclude` that excluded too much
        /// would pass the first half and fail this one.
        #[test]
        #[ignore]
        fn a_quota_can_be_respelled_but_not_renamed_onto_another(
            (name, respelling) in any_respelling(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (own, onto_other) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                given_quota(&pool, &name, 240).await;
                given_quota(&pool, "a quota with some other name", 120).await;
                let id = quota_id(&pool, &name).await;

                let own = change(&pool, id, Some(&respelling), Some("4")).await.unwrap();
                let onto_other = change(
                    &pool, id, Some("A Quota, With Some Other Name"), Some("4"),
                ).await.unwrap();
                (own, onto_other)
            });

            prop_assert!(own.is_ok(), "{:?} refused on its own quota: {:?}", respelling, own);
            prop_assert!(onto_other.is_err(), "renaming onto another quota was allowed");
        }

        /// **Removing a quota takes it off the screen and leaves its name
        /// taken.** Both halves of #148's bargain, asserted together over any
        /// arrangement of live and removed quotas, because they are only
        /// coherent together: `list_quotas` hiding an archived row is what
        /// makes removal mean anything, and `name_standing` still reporting
        /// it `Taken` is what stops the `UNIQUE COLLATE NOCASE` column
        /// refusing a write the screen had already promised
        /// (`T-collation-enforces-name-identity`).
        #[test]
        #[ignore]
        fn a_removed_quota_leaves_the_screen_and_keeps_its_name(
            removed in prop::collection::vec(any::<bool>(), 0..6),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (listed, standings) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let names: Vec<String> =
                    (0..removed.len()).map(|n| format!("quota {n}")).collect();
                for name in &names {
                    given_quota(&pool, name, 60).await;
                }
                for (name, gone) in names.iter().zip(&removed) {
                    if *gone {
                        remove(&pool, quota_id(&pool, name).await, 4242).await.unwrap();
                    }
                }

                let listed: Vec<String> = store::list_quotas(&pool)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|q| q.name)
                    .collect();
                let mut standings = Vec::new();
                for name in &names {
                    standings.push(name_standing(&pool, name, None).await.unwrap());
                }
                (listed, standings)
            });

            let expected: Vec<String> = (0..removed.len())
                .filter(|n| !removed[*n])
                .map(|n| format!("quota {n}"))
                .collect();
            prop_assert_eq!(listed, expected);

            for (standing, gone) in standings.iter().zip(&removed) {
                match standing {
                    NameStanding::Taken { archived, .. } => {
                        prop_assert_eq!(archived, gone, "archived flag disagreed with the removal")
                    }
                    other => prop_assert!(
                        false, "a quota that exists read as {:?} rather than Taken", other
                    ),
                }
            }
        }
    }
}
