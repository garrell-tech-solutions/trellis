//! **Exceptions** -- how the owner narrows a life area's guardrail, or
//! every life area's, for specific civil dates (#61). Only ever *removes*
//! hours: `D-guardrails-never-yield` says a guardrail is never breached, so
//! an exception is the owner redrawing the wall down for a day, never up.
//!
//! [`store`] holds the capability's own reads, writes, and its own
//! life-area-name validation query. [`view`] is what the free time page's
//! exceptions list renders. [`http`] serves `POST /exceptions` and
//! `POST /exceptions/{id}/remove`, rendering into the same `#exceptions-
//! list` fragment `free_time::http` also includes on `GET /free-time`.
//!
//! What is deliberately *not* here: `free_time` is the only reader outside
//! this capability, and it asks [`for_life_area`] rather than reaching into
//! [`store`] itself (`T-one-front-door-per-capability`).

pub mod http;
pub mod store;
pub mod view;

use scheduler_core::exception::DateRange;
use sqlx::SqlitePool;
use std::str::FromStr;
use view::ExceptionListItem;

fn parse_date(value: &str) -> jiff::civil::Date {
    jiff::civil::Date::from_str(value)
        .expect("a stored exception date was well-formed before it was written")
}

/// Every exception whose scope reaches `life_area_id` -- global ones and
/// any scoped to it -- the shape `scheduler_core::free_time::Guardrail`
/// wants as `excluded`. `free_time::http` calls this once per life area
/// rather than filtering `store::list_all` itself, since which rows apply
/// to a given life area is this capability's own rule.
pub(crate) async fn for_life_area(
    pool: &SqlitePool,
    life_area_id: i64,
) -> Result<Vec<DateRange>, sqlx::Error> {
    let rows = store::list_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter(|row| row.life_area_id.is_none() || row.life_area_id == Some(life_area_id))
        .map(|row| DateRange {
            start: parse_date(&row.start_date),
            end: parse_date(&row.end_date),
        })
        .collect())
}

/// A stored row's scope, resolved to the words the page shows -- "All life
/// areas" for a global exception, or the named life area's own name.
/// Resolving against `options` rather than querying per row avoids an
/// exceptions list of N rows issuing N life-area lookups.
fn scope_of(
    life_area_id: Option<i64>,
    options: &[crate::life_areas::view::LifeAreaOption],
) -> String {
    match life_area_id {
        None => "All life areas".to_string(),
        Some(id) => options
            .iter()
            .find(|option| option.id == id)
            .map(|option| option.name.clone())
            .unwrap_or_default(),
    }
}

/// Every stored exception as the free time page's own list shows it.
pub(crate) async fn list(pool: &SqlitePool) -> Result<Vec<ExceptionListItem>, sqlx::Error> {
    let rows = store::list_all(pool).await?;
    let options = crate::life_areas::active_options(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| ExceptionListItem {
            id: row.id,
            scope: scope_of(row.life_area_id, &options),
            start: row.start_date,
            end: row.end_date,
            label: row.label,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};

    #[tokio::test]
    async fn for_life_area_includes_a_global_exception() {
        let (_dir, pool) = test_pool().await;
        store::insert(&pool, None, "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();
        let work = seeded_life_area_id(&pool, "Work").await;

        let excluded = for_life_area(&pool, work).await.unwrap();

        assert_eq!(
            excluded,
            vec![DateRange {
                start: "2026-08-24".parse().unwrap(),
                end: "2026-08-28".parse().unwrap(),
            }]
        );
    }

    #[tokio::test]
    async fn for_life_area_includes_its_own_scoped_exception() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        store::insert(&pool, Some(work), "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();

        let excluded = for_life_area(&pool, work).await.unwrap();

        assert_eq!(excluded.len(), 1);
    }

    #[tokio::test]
    async fn for_life_area_excludes_another_life_areas_scoped_exception() {
        let (_dir, pool) = test_pool().await;
        let fitness = seeded_life_area_id(&pool, "Fitness").await;
        store::insert(&pool, Some(fitness), "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();
        let work = seeded_life_area_id(&pool, "Work").await;

        let excluded = for_life_area(&pool, work).await.unwrap();

        assert_eq!(excluded, Vec::new());
    }

    #[tokio::test]
    async fn list_reports_a_global_exceptions_scope_as_all_life_areas() {
        let (_dir, pool) = test_pool().await;
        store::insert(&pool, None, "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();

        let items = list(&pool).await.unwrap();

        assert_eq!(items[0].scope, "All life areas");
    }

    #[tokio::test]
    async fn list_reports_a_scoped_exceptions_scope_as_its_life_areas_name() {
        let (_dir, pool) = test_pool().await;
        let work = seeded_life_area_id(&pool, "Work").await;
        store::insert(&pool, Some(work), "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();

        let items = list(&pool).await.unwrap();

        assert_eq!(items[0].scope, "Work");
    }

    #[tokio::test]
    async fn list_carries_each_exceptions_dates_and_label() {
        let (_dir, pool) = test_pool().await;
        store::insert(&pool, None, "2026-08-24", "2026-08-28", "vacation")
            .await
            .unwrap();

        let items = list(&pool).await.unwrap();

        assert_eq!(items[0].start, "2026-08-24");
        assert_eq!(items[0].end, "2026-08-28");
        assert_eq!(items[0].label, "vacation");
    }
}
