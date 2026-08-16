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
}
