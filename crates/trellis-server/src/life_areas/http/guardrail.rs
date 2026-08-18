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
    overlaps, Band, GuardrailFields, GuardrailRejection, Weekday, WellFormedGuardrailSubmission,
};
use serde::Deserialize;
use sqlx::SqlitePool;

/// Rows sharing the same start and end are one band the owner authored
/// together, whichever submission wrote them (`guardrails-two-bands-03`:
/// "two bands, one life area" counts groups, not weekday rows). `id` names
/// the group's own representative row -- the one `remove_guardrail_band`'s
/// URL carries, and `store::remove_guardrail_band` takes the whole group
/// with it regardless of which member's id arrives.
/// Folds `rows` down to one `(id, start, end, weekdays)` tuple per distinct
/// start/end pair -- [`group_bands`]'s grouping half, split out from its
/// labelling half.
fn group_band_rows(rows: Vec<store::GuardrailBandRow>) -> Vec<(i64, i64, i64, Vec<Weekday>)> {
    let mut groups: Vec<(i64, i64, i64, Vec<Weekday>)> = Vec::new();
    for row in rows {
        let weekday = Weekday::parse(&row.weekday).expect("stored weekday is well-formed");
        match groups
            .iter_mut()
            .find(|(_, start, end, _)| *start == row.start_minutes && *end == row.end_minutes)
        {
            Some(group) => group.3.push(weekday),
            None => groups.push((row.id, row.start_minutes, row.end_minutes, vec![weekday])),
        }
    }
    groups
}

fn band_group_label(start: i64, end: i64, mut weekdays: Vec<Weekday>) -> String {
    weekdays.sort();
    let days: Vec<&str> = weekdays.iter().map(|day| day.as_str()).collect();
    format!(
        "{} {}-{}",
        days.join(", "),
        format_minutes(start),
        format_minutes(end)
    )
}

pub(super) fn group_bands(rows: Vec<store::GuardrailBandRow>) -> Vec<GuardrailBandGroup> {
    group_band_rows(rows)
        .into_iter()
        .map(|(id, start, end, weekdays)| GuardrailBandGroup {
            id,
            label: band_group_label(start, end, weekdays),
        })
        .collect()
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
/// mistake to `WellFormedGuardrailSubmission::from_fields` (`T-empty-equals-
/// absent`'s reasoning, applied to a time instead of a life area name).
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
    Ok(rows
        .iter()
        .map(|row| Band {
            weekday: Weekday::parse(&row.weekday).expect("stored weekday is well-formed"),
            start_minutes: row.start_minutes,
            end_minutes: row.end_minutes,
        })
        .collect())
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
