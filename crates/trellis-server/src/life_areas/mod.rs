//! **Life areas** -- user-managed rows a task is tagged with at triage
//! (`T-life-areas-are-data`, #47). Not a Rust enum and not a config file: a
//! fresh database seeds five (Work, Fitness, Learning, Family, Home), and
//! from then on the set belongs to the user, editable from the running app
//! with no rebuild.
//!
//! [`store`] holds the capability's own reads and writes: listing what is
//! still active, finding a name to refuse a duplicate, adding, and
//! archiving. [`view`] is what the management page and the triage picker
//! both render. [`http`] serves `/life-areas` and its archive control.
//!
//! What is deliberately *not* here: resolving a triage submission's life
//! area name to an id is triage's own query, in `triage::store`
//! (`T-capability-owns-its-queries`), and whether a submitted name is
//! well-formed is `scheduler_core::life_area`'s rule, not this module's --
//! both survive changing HTTP for something else.

pub mod http;
pub mod store;
pub mod view;

use crate::life_areas::view::LifeAreaOption;
use scheduler_core::guardrail::{Band, Weekday};
use sqlx::SqlitePool;

/// What every other capability asks this one for: the life areas a user may
/// currently pick.
///
/// Three readers need it -- the management page lists them, the inbox
/// fragment offers them in every capture's triage forms, and a quick-added
/// capture renders its own row with the same forms. Each of those knowing
/// *which* query and *which* mapping to compose would be three copies of
/// this capability's internals living in other capabilities; they ask for
/// the choices instead, and how a choice is stored and shaped stays in here.
pub(crate) async fn active_options(pool: &SqlitePool) -> Result<Vec<LifeAreaOption>, sqlx::Error> {
    Ok(store::list_active(pool)
        .await?
        .into_iter()
        .map(LifeAreaOption::from)
        .collect())
}

/// Resolve a submitted name to the life area it names, if that life area is
/// one work may currently be filed under.
///
/// **Two capabilities validate a submitted life-area name at their own
/// boundary** -- triage, before writing a task, and exceptions, before
/// writing a dated exception scoped to one life area. They arrived as two
/// byte-identical `find_active_life_area_id` functions in two `store.rs`
/// files, each citing `T-capability-owns-its-queries`.
///
/// That decision licenses separate copies for *two facts*: "two queries
/// against one table for two different reasons". This is one fact asked
/// twice for the same reason -- does this name resolve to a life area work
/// may be filed under? -- and the answer is `life_areas`' to give, since it
/// owns the table, the name-identity rule and the archived/active
/// distinction. `T-one-front-door-per-capability` is the complement that
/// says so, and names this exact failure: without it, "own your own
/// queries" degenerates into every capability hand-assembling another's
/// internals.
///
/// The cost of the copies was about to be real rather than theoretical.
/// "Active" means `archived_at IS NULL` today, and `pool_only` is a second
/// dimension of the same question sitting one slice away; a third caller
/// (M3's borrowing, `#62`'s capacity) would have made three places to
/// remember it in.
pub(crate) async fn active_id_for_name(
    pool: &SqlitePool,
    name: &str,
) -> Result<Option<i64>, sqlx::Error> {
    store::find_active_id_by_name(pool, name).await
}

/// One life area's guardrail, as a reader outside this capability needs it:
/// enough to compute free time from, nothing about how it is stored.
///
/// `pool_only` has no field of its own because it is **resolved before a
/// reader sees it**: a pool-only life area reports no bands, which projects
/// to zero free time -- the same answer a life area with no guardrail yet
/// gives. `D-life-area-owns-its-time` settles that pool-only means never
/// placed, so no reader should have to remember to check a second field to
/// honour it.
///
/// It says "resolved" rather than "always empty" on purpose. Empty is what
/// the stored rows *ought* to be and currently need not be: nothing forbids
/// a life area from being marked pool-only while it still holds bands (the
/// GAP in `docs/design/architecture.md`), and before [`guardrails`] began
/// applying the rule, `/free-time` reported 32h for a life area the owner
/// had marked never scheduled.
pub(crate) struct LifeAreaGuardrail {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) bands: Vec<Band>,
}

fn to_band(row: store::GuardrailBandRow) -> Band {
    Band {
        weekday: Weekday::parse(&row.weekday).expect("stored weekday is well-formed"),
        start_minutes: row.start_minutes,
        end_minutes: row.end_minutes,
    }
}

/// Every active life area's guardrail -- `#60`'s free-time page is the
/// first reader outside this capability, asking a different question of the
/// same rows the management page already lists (`store::list_active`,
/// `store::list_guardrail_bands`), so it gets a front door of its own rather
/// than reaching for those queries itself (`T-one-front-door-per-capability`).
pub(crate) async fn guardrails(pool: &SqlitePool) -> Result<Vec<LifeAreaGuardrail>, sqlx::Error> {
    let rows = store::list_active(pool).await?;
    let mut guardrails = Vec::with_capacity(rows.len());
    for row in rows {
        // Pool-only wins over whatever bands the row happens to carry. One
        // place applies it, so no reader can forget to.
        let bands = if row.pool_only {
            Vec::new()
        } else {
            store::list_guardrail_bands(pool, row.id)
                .await?
                .into_iter()
                .map(to_band)
                .collect()
        };
        guardrails.push(LifeAreaGuardrail {
            id: row.id,
            name: row.name,
            bands,
        });
    }
    Ok(guardrails)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    /// The reader-side half of `D-life-area-owns-its-time`'s "no guardrail
    /// means pool-only": a life area the owner marked never scheduled
    /// projects nothing, whatever bands its row still holds.
    ///
    /// The setup reaches a state the page cannot show and the model does
    /// not want to exist -- marked pool-only *and* carrying bands -- which
    /// is exactly why the assertion is worth having. Nothing forbids that
    /// state yet (the GAP in `docs/design/architecture.md`); until
    /// something does, this is what keeps it from reaching `#60`'s page as
    /// free hours.
    /// "Work, holding one band", as the front door reports it -- the setup
    /// both assertions below start from, differing only in whether the
    /// pool-only column is set.
    async fn work_as_reported(pool: &SqlitePool, pool_only: bool) -> LifeAreaGuardrail {
        let work = store::find_by_name(pool, "Work").await.unwrap().unwrap();
        store::insert_guardrail_band(pool, work.id, "Mon", 540, 1020)
            .await
            .unwrap();
        if pool_only {
            store::set_pool_only(pool, work.id, true).await.unwrap();
        }
        guardrails(pool)
            .await
            .unwrap()
            .into_iter()
            .find(|area| area.id == work.id)
            .expect("an active life area is reported")
    }

    #[tokio::test]
    async fn a_pool_only_life_area_offers_no_guardrail_even_holding_bands() {
        let (_dir, pool) = test_pool().await;

        let work = work_as_reported(&pool, true).await;

        assert!(
            work.bands.is_empty(),
            "a never-scheduled life area offered {} band(s) to project free time from",
            work.bands.len()
        );
    }

    #[tokio::test]
    async fn a_walled_life_area_still_offers_its_bands() {
        let (_dir, pool) = test_pool().await;

        let work = work_as_reported(&pool, false).await;

        assert_eq!(work.bands.len(), 1);
    }

    /// The three cases both former copies tested, kept once. Case
    /// insensitivity is the column's collation doing its job
    /// (`T-collation-enforces-name-identity`), not this function's.
    #[tokio::test]
    async fn active_id_for_name_resolves_a_seeded_name_case_insensitively() {
        let (_dir, pool) = test_pool().await;

        assert!(active_id_for_name(&pool, "work").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn active_id_for_name_is_none_for_a_name_that_names_nothing() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(active_id_for_name(&pool, "Gardening").await.unwrap(), None);
    }

    /// Archived and never-existed are one answer on purpose: every caller
    /// refuses both identically, so nothing downstream needs to tell them
    /// apart.
    #[tokio::test]
    async fn active_id_for_name_is_none_once_the_life_area_is_archived() {
        let (_dir, pool) = test_pool().await;
        let learning = active_id_for_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap();
        store::archive(&pool, learning, 1_000).await.unwrap();

        assert_eq!(active_id_for_name(&pool, "Learning").await.unwrap(), None);
    }

    #[tokio::test]
    async fn the_choices_are_the_active_life_areas_in_listing_order() {
        let (_dir, pool) = test_pool().await;

        let options = active_options(&pool).await.unwrap();

        assert_eq!(
            options.iter().map(|o| o.name.as_str()).collect::<Vec<_>>(),
            vec!["Work", "Fitness", "Learning", "Family", "Home"]
        );
    }

    #[tokio::test]
    async fn an_archived_life_area_is_not_a_choice() {
        let (_dir, pool) = test_pool().await;
        let learning = store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap();
        store::archive(&pool, learning.id, 1_000).await.unwrap();

        let options = active_options(&pool).await.unwrap();

        assert!(!options.iter().any(|o| o.name == "Learning"));
    }

    #[tokio::test]
    async fn guardrails_reports_every_active_life_areas_bands() {
        let (_dir, pool) = test_pool().await;
        let work = store::find_by_name(&pool, "Work").await.unwrap().unwrap();
        store::insert_guardrail_band(&pool, work.id, "Mon", 540, 1020)
            .await
            .unwrap();

        let guardrails = guardrails(&pool).await.unwrap();

        let work = guardrails.iter().find(|g| g.name == "Work").unwrap();
        assert_eq!(work.bands.len(), 1);
        assert_eq!(work.bands[0].weekday, Weekday::Mon);
        assert_eq!(work.bands[0].start_minutes, 540);
        assert_eq!(work.bands[0].end_minutes, 1020);

        let fitness = guardrails.iter().find(|g| g.name == "Fitness").unwrap();
        assert!(fitness.bands.is_empty());
    }

    #[tokio::test]
    async fn guardrails_excludes_an_archived_life_area() {
        let (_dir, pool) = test_pool().await;
        let learning = store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap();
        store::archive(&pool, learning.id, 1_000).await.unwrap();

        let guardrails = guardrails(&pool).await.unwrap();

        assert!(!guardrails.iter().any(|g| g.name == "Learning"));
    }
}
