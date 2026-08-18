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

/// One life area's guardrail, as a reader outside this capability needs it:
/// enough to compute free time from, nothing about how it is stored.
/// `pool_only` needs no field of its own here -- a pool-only life area's
/// `bands` is always empty, which already projects to zero free time, the
/// same answer a life area that simply has no guardrail yet reports.
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
        let bands = store::list_guardrail_bands(pool, row.id)
            .await?
            .into_iter()
            .map(to_band)
            .collect();
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
