//! `POST /life-areas/{id}/guardrail` and `POST /guardrail-bands/{id}/remove`
//! -- split out from [`super`]'s life-area listing/CRUD handlers, a
//! different concern that happens to render into the same
//! `#life-areas-list` fragment (via [`super::render_life_areas_list`]).
//!
//! `D-life-area-owns-its-time`: a life area's guardrail is a weekly mask of
//! civil bands, or the life area is marked pool-only; `scheduler_core::
//! guardrail` decides whether a submission is well-formed and whether it
//! overlaps what the life area already has, since both survive changing
//! HTTP for something else. This module composes that decision with the
//! database question neither can answer alone.

use crate::life_areas::store;
use crate::life_areas::view::GuardrailBandGroup;
use crate::platform::response::write_failed;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use scheduler_core::guardrail::{
    group, overlaps, Band, BandSpan, GuardrailFields, GuardrailRejection, Weekday,
    WellFormedGuardrailSubmission,
};
use serde::Deserialize;
use sqlx::SqlitePool;

/// One stored row as the core's own band. The `expect` cannot fire: the
/// `weekday` column carries a `CHECK` naming exactly the seven
/// `Weekday::as_str` spellings (migration `0006`), so a row that failed to
/// parse would mean the schema and the enum had already diverged.
fn band_of(row: &store::GuardrailBandRow) -> Band {
    Band {
        weekday: Weekday::parse(&row.weekday).expect("stored weekday is well-formed"),
        start_minutes: row.start_minutes,
        end_minutes: row.end_minutes,
    }
}

/// The rows a life area's page shows as bands. Which rows make up one band
/// is `scheduler_core::guardrail::group`'s call, not this module's --
/// `store::remove_guardrail_band` deletes by the same rule in SQL, and the
/// band a reader sees has to be the band the Remove button takes.
///
/// What is left here is the label, which is presentation and belongs
/// nowhere else.
pub(super) fn group_bands(rows: Vec<store::GuardrailBandRow>) -> Vec<GuardrailBandGroup> {
    group(rows.iter().map(|row| (row.id, band_of(row))))
        .into_iter()
        .map(|band| GuardrailBandGroup {
            id: band.tag,
            label: band_label(band.span, &band.weekdays),
        })
        .collect()
}

fn band_label(span: BandSpan, weekdays: &[Weekday]) -> String {
    let days: Vec<&str> = weekdays.iter().map(|day| day.as_str()).collect();
    format!(
        "{} {}-{}",
        days.join(", "),
        format_minutes(span.start_minutes),
        format_minutes(span.end_minutes)
    )
}

fn format_minutes(total_minutes: i64) -> String {
    format!("{:02}:{:02}", total_minutes / 60, total_minutes % 60)
}

/// The guardrail form's own fields: seven independent weekday checkboxes
/// rather than one repeated-key field, so extraction never depends on
/// `serde_urlencoded`'s handling of arrays (it has none). `start`/`end` are
/// `HH:MM` text; `pool_only` is a checkbox of its own -- one form, either
/// half of it filled in, per `WellFormedGuardrailSubmission::from_fields`'s
/// own contract.
#[derive(Deserialize, Default)]
pub struct GuardrailFormRequest {
    mon: Option<String>,
    tue: Option<String>,
    wed: Option<String>,
    thu: Option<String>,
    fri: Option<String>,
    sat: Option<String>,
    sun: Option<String>,
    start: Option<String>,
    end: Option<String>,
    pool_only: Option<String>,
}

fn weekdays_from_form(form: &GuardrailFormRequest) -> Vec<Weekday> {
    let boxes: [(&Option<String>, Weekday); 7] = [
        (&form.mon, Weekday::Mon),
        (&form.tue, Weekday::Tue),
        (&form.wed, Weekday::Wed),
        (&form.thu, Weekday::Thu),
        (&form.fri, Weekday::Fri),
        (&form.sat, Weekday::Sat),
        (&form.sun, Weekday::Sun),
    ];
    boxes
        .into_iter()
        .filter_map(|(checked, day)| checked.is_some().then_some(day))
        .collect()
}

/// `"09:00"` into minutes since midnight, or `None` for anything that is
/// not exactly that shape -- unparseable and absent are the same submitter
/// mistake to `WellFormedGuardrailSubmission::from_fields`, which is
/// `T-empty-equals-absent`'s reasoning applied to a time instead of a life
/// area name. (Kept on one line on purpose: `scripts/ci/decision_slugs.sh`
/// reads a citation broken across a line wrap as a slug that does not
/// exist, and did.)
fn parse_minutes(value: &str) -> Option<i64> {
    let (hours, minutes) = value.split_once(':')?;
    let hours: i64 = hours.parse().ok()?;
    let minutes: i64 = minutes.parse().ok()?;
    if !(0..24).contains(&hours) || !(0..60).contains(&minutes) {
        return None;
    }
    Some(hours * 60 + minutes)
}

fn guardrail_fields(form: &GuardrailFormRequest) -> GuardrailFields {
    GuardrailFields {
        weekdays: weekdays_from_form(form),
        start_minutes: form.start.as_deref().and_then(parse_minutes),
        end_minutes: form.end.as_deref().and_then(parse_minutes),
        pool_only: form.pool_only.is_some(),
    }
}

fn guardrail_rejection_message(rejection: &GuardrailRejection) -> String {
    match rejection {
        GuardrailRejection::ChooseOne => {
            "choose a guardrail band or mark never scheduled".to_string()
        }
        GuardrailRejection::NoWeekday => "a guardrail band needs at least one weekday".to_string(),
        GuardrailRejection::InvalidTimes => {
            "a guardrail band's end must follow its start".to_string()
        }
    }
}

/// Whether any of `bands` overlaps a band the life area already has, and
/// the write if not -- the one branch `save_guardrail` delegates so its own
/// match stays one outcome per arm.
async fn existing_bands(pool: &SqlitePool, life_area_id: i64) -> Result<Vec<Band>, StatusCode> {
    let rows = store::list_guardrail_bands(pool, life_area_id)
        .await
        .map_err(write_failed)?;
    Ok(rows.iter().map(band_of).collect())
}

async fn insert_bands(
    pool: &SqlitePool,
    life_area_id: i64,
    bands: &[Band],
) -> Result<(), StatusCode> {
    for band in bands {
        store::insert_guardrail_band(
            pool,
            life_area_id,
            band.weekday.as_str(),
            band.start_minutes,
            band.end_minutes,
        )
        .await
        .map_err(write_failed)?;
    }
    Ok(())
}

async fn try_add_bands(
    pool: &SqlitePool,
    life_area_id: i64,
    bands: Vec<Band>,
) -> Result<(StatusCode, Option<(i64, String)>), StatusCode> {
    let existing = existing_bands(pool, life_area_id).await?;
    if bands.iter().any(|band| overlaps(&existing, band)) {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((
                life_area_id,
                "the band overlaps one the life area already has".to_string(),
            )),
        ));
    }
    insert_bands(pool, life_area_id, &bands).await?;
    Ok((StatusCode::CREATED, None))
}

async fn decide_guardrail(
    pool: &SqlitePool,
    life_area_id: i64,
    fields: &GuardrailFields,
) -> Result<(StatusCode, Option<(i64, String)>), StatusCode> {
    match WellFormedGuardrailSubmission::from_fields(fields) {
        Err(rejection) => Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((life_area_id, guardrail_rejection_message(&rejection))),
        )),
        Ok(WellFormedGuardrailSubmission::PoolOnly) => {
            store::set_pool_only(pool, life_area_id, true)
                .await
                .map_err(write_failed)?;
            Ok((StatusCode::OK, None))
        }
        Ok(WellFormedGuardrailSubmission::Bands(bands)) => {
            try_add_bands(pool, life_area_id, bands).await
        }
    }
}

pub async fn save_guardrail(
    State(pool): State<SqlitePool>,
    Path(life_area_id): Path<i64>,
    Form(form): Form<GuardrailFormRequest>,
) -> Result<Response, StatusCode> {
    let fields = guardrail_fields(&form);
    let (status, row_error) = decide_guardrail(&pool, life_area_id, &fields).await?;
    super::render_life_areas_list(&pool, status, None, row_error).await
}

pub async fn remove_guardrail_band(
    State(pool): State<SqlitePool>,
    Path(band_id): Path<i64>,
) -> Result<Response, StatusCode> {
    store::remove_guardrail_band(&pool, band_id)
        .await
        .map_err(write_failed)?;
    super::render_life_areas_list(&pool, StatusCode::OK, None, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{seeded_life_area_id, test_pool};
    use proptest::prelude::*;
    use scheduler_core::guardrail::AuthoredBand;

    async fn authored_bands(pool: &SqlitePool, life_area_id: i64) -> Vec<AuthoredBand<i64>> {
        let rows = store::list_guardrail_bands(pool, life_area_id)
            .await
            .unwrap();
        group(rows.iter().map(|row| (row.id, band_of(row))))
    }

    async fn row_count(pool: &SqlitePool, life_area_id: i64) -> usize {
        store::list_guardrail_bands(pool, life_area_id)
            .await
            .unwrap()
            .len()
    }

    /// The weekday indices a generated submission checked, deduplicated the
    /// way seven distinct checkboxes already are.
    fn weekdays_of(indices: &[usize]) -> Vec<Weekday> {
        let all = [
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Fri,
            Weekday::Sat,
            Weekday::Sun,
        ];
        let mut days: Vec<Weekday> = indices.iter().map(|&i| all[i]).collect();
        days.sort();
        days.dedup();
        days
    }

    #[test]
    fn parse_minutes_adds_the_hour_and_minute_components() {
        assert_eq!(parse_minutes("09:30"), Some(570));
    }

    #[test]
    fn parse_minutes_rejects_an_hour_outside_0_24_even_with_valid_minutes() {
        assert_eq!(parse_minutes("24:00"), None);
    }

    #[test]
    fn parse_minutes_rejects_a_minute_outside_0_60_even_with_a_valid_hour() {
        assert_eq!(parse_minutes("09:60"), None);
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

        /// **The band the page shows and the band Remove takes are the same
        /// band** — for any set of authored submissions, and whichever of
        /// the resulting bands is removed.
        ///
        /// This is the one property in the slice that no type can carry.
        /// "Which rows are one band" is now stated once, in
        /// `scheduler_core::guardrail::group` — but only for the half that
        /// runs in Rust. `store::remove_guardrail_band` states it a second
        /// time in SQL (`WHERE life_area_id = ? AND start_minutes = ? AND
        /// end_minutes = ?`), and no signature makes those two agree. Give
        /// a band a third distinguishing field and the Rust half splits a
        /// group the SQL half still deletes whole; the page would show two
        /// bands whose Remove buttons each take both, and every example
        /// test would still pass.
        ///
        /// Three invariants in one run: the removed band is gone, every
        /// other band survives *untouched* — same handle, same label, same
        /// order — and exactly the weekday rows that band displayed left
        /// the table, no more and no fewer.
        #[test]
        #[ignore]
        fn removing_a_band_removes_exactly_the_rows_that_band_displayed(
            submissions in prop::collection::vec(
                (prop::collection::vec(0..7usize, 1..8), 0..4i64, 1..4i64),
                1..5,
            ),
            victim in 0..8usize,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let life_area_id = seeded_life_area_id(&pool, "Work").await;

                // Starts and lengths are drawn from a handful of values, not
                // spread over the day. Two bands that share a start and differ
                // only in their end are the case that separates "same band" from
                // "same start", and a generator ranging over 1380 minutes
                // essentially never produces one -- it passes while covering
                // nothing, which is this repo's own recorded trap
                // (life-areas-duplicate-03).
                for (indices, start, length) in &submissions {
                    for weekday in weekdays_of(indices) {
                        store::insert_guardrail_band(
                            &pool,
                            life_area_id,
                            weekday.as_str(),
                            start * 60,
                            start * 60 + length * 30,
                        )
                        .await
                        .unwrap();
                    }
                }

                let before = group_bands(
                    store::list_guardrail_bands(&pool, life_area_id).await.unwrap(),
                );
                let rows_before = row_count(&pool, life_area_id).await;
                prop_assume!(!before.is_empty());
                let index = victim % before.len();
                let removed_id = before[index].id;
                let removed_weekdays = authored_bands(&pool, life_area_id).await[index]
                    .weekdays
                    .len();

                store::remove_guardrail_band(&pool, removed_id).await.unwrap();

                let after = group_bands(
                    store::list_guardrail_bands(&pool, life_area_id).await.unwrap(),
                );
                let rows_after = row_count(&pool, life_area_id).await;

                prop_assert!(
                    !after.iter().any(|band| band.id == removed_id),
                    "the removed band is still on the page"
                );
                let survivors: Vec<(i64, String)> = before
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .map(|(_, band)| (band.id, band.label.clone()))
                    .collect();
                let actual: Vec<(i64, String)> = after
                    .iter()
                    .map(|band| (band.id, band.label.clone()))
                    .collect();
                prop_assert_eq!(
                    survivors,
                    actual,
                    "removing one band disturbed the others"
                );
                prop_assert_eq!(
                    rows_before - rows_after,
                    removed_weekdays,
                    "the rows deleted are not the rows the band displayed"
                );
                Ok(())
            })?;
        }
    }
}
