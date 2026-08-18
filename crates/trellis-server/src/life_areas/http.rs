//! `GET /life-areas`, `POST /life-areas`, `POST /life-areas/{id}/archive`,
//! `POST /life-areas/{id}/guardrail`, `POST /guardrail-bands/{id}/remove`.
//!
//! Management lives on its own route (`#45` already established that a
//! further page-level concern earns one, once `/` had both the inbox and the
//! task list). The triage picker stays on `/`, where triage happens; this
//! module never renders it -- [`crate::inbox`] reaches in here for the
//! options, the same direction `capture` and `triage` already reach into
//! `inbox::view`.
//!
//! Every write follows `T-forms-swap-one-fragment`: a rejection re-renders
//! the same `#life-areas-list` fragment carrying the error, as a `422`.
//! Adding a life area carries its message at the top of the fragment (no
//! row exists yet to attach it to); a guardrail save or a band removal
//! carries it on the life area's own row, the same shape `inbox::view::
//! CaptureRow` uses for a failed triage.
//!
//! `D-life-area-owns-its-time`: a life area's guardrail is a weekly mask of
//! civil bands, or the life area is marked pool-only; `scheduler_core::
//! guardrail` decides whether a submission is well-formed and whether it
//! overlaps what the life area already has, since both survive changing
//! HTTP for something else. This module composes that decision with the
//! database question neither can answer alone.

use crate::life_areas::store;
use crate::life_areas::view::{GuardrailBandGroup, LifeAreaListItem};
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use scheduler_core::guardrail::{
    overlaps, Band, GuardrailFields, GuardrailRejection, Weekday, WellFormedGuardrailSubmission,
};
use scheduler_core::life_area::{parse_name, NameRejection};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "life_areas.html")]
struct LifeAreasTemplate {
    life_areas: Vec<LifeAreaListItem>,
    add_error: Option<String>,
    timezone: String,
    timezone_error: Option<String>,
    nav: Vec<NavLink>,
}

/// The `#life-areas-list` fragment on its own -- what an add, an archive, a
/// guardrail save and a band removal all swap in.
#[derive(Template)]
#[template(path = "life_areas_list.html")]
struct LifeAreasListTemplate {
    life_areas: Vec<LifeAreaListItem>,
    add_error: Option<String>,
}

pub async fn show_life_areas(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let life_areas = build_life_area_rows(&pool, None)
        .await
        .map_err(write_failed)?;
    let timezone = crate::settings::current_timezone(&pool)
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &LifeAreasTemplate {
            life_areas,
            add_error: None,
            timezone,
            timezone_error: None,
            nav: nav::links(Page::LifeAreas),
        },
    ))
}

/// Every active life area as the management page shows it, its guardrail
/// bands grouped back into what the owner authored, with `row_error`
/// attached to whichever life area's guardrail action just failed (if any)
/// -- the same shape `inbox::lists::build_capture_rows` gives a failed
/// triage.
async fn build_life_area_rows(
    pool: &SqlitePool,
    row_error: Option<(i64, String)>,
) -> Result<Vec<LifeAreaListItem>, sqlx::Error> {
    let rows = store::list_active(pool).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let bands = store::list_guardrail_bands(pool, row.id).await?;
        items.push(LifeAreaListItem {
            id: row.id,
            name: row.name,
            pool_only: row.pool_only,
            bands: group_bands(bands),
            error: row_error
                .as_ref()
                .filter(|(id, _)| *id == row.id)
                .map(|(_, message)| message.clone()),
        });
    }
    Ok(items)
}

/// Rows sharing the same start and end are one band the owner authored
/// together, whichever submission wrote them (`guardrails-two-bands-03`:
/// "two bands, one life area" counts groups, not weekday rows). `id` names
/// the group's own representative row -- the one `remove_guardrail_band`'s
/// URL carries, and `store::remove_guardrail_band` takes the whole group
/// with it regardless of which member's id arrives.
fn group_bands(rows: Vec<store::GuardrailBandRow>) -> Vec<GuardrailBandGroup> {
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
        .into_iter()
        .map(|(id, start, end, mut weekdays)| {
            weekdays.sort();
            let days: Vec<&str> = weekdays.iter().map(|day| day.as_str()).collect();
            GuardrailBandGroup {
                id,
                label: format!(
                    "{} {}-{}",
                    days.join(", "),
                    format_minutes(start),
                    format_minutes(end)
                ),
            }
        })
        .collect()
}

fn format_minutes(total_minutes: i64) -> String {
    format!("{:02}:{:02}", total_minutes / 60, total_minutes % 60)
}

async fn render_life_areas_list(
    pool: &SqlitePool,
    status: StatusCode,
    add_error: Option<String>,
    row_error: Option<(i64, String)>,
) -> Result<Response, StatusCode> {
    let life_areas = build_life_area_rows(pool, row_error)
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        status,
        &LifeAreasListTemplate {
            life_areas,
            add_error,
        },
    ))
}

#[derive(Deserialize)]
pub struct AddLifeAreaRequest {
    name: String,
}

/// Well-formedness first (`scheduler_core::life_area::parse_name`), then
/// the duplicate check -- trim-before-blank is what makes whitespace-only
/// input rejected rather than silently creating a life area that renders as
/// nothing, and neither question needs the database until the first is
/// already answered.
async fn try_add_life_area(
    pool: &SqlitePool,
    raw_name: &str,
) -> Result<Result<(), String>, StatusCode> {
    let name = match parse_name(raw_name) {
        Ok(name) => name,
        Err(NameRejection::Blank) => return Ok(Err("name is required".to_string())),
    };
    if store::find_by_name(pool, &name)
        .await
        .map_err(write_failed)?
        .is_some()
    {
        return Ok(Err(format!("{name} is already a life area")));
    }
    store::insert(pool, &name).await.map_err(write_failed)?;
    Ok(Ok(()))
}

pub async fn create_life_area(
    State(pool): State<SqlitePool>,
    Form(payload): Form<AddLifeAreaRequest>,
) -> Result<Response, StatusCode> {
    let (status, add_error) = match try_add_life_area(&pool, &payload.name).await? {
        Ok(()) => (StatusCode::CREATED, None),
        Err(message) => (StatusCode::UNPROCESSABLE_ENTITY, Some(message)),
    };
    render_life_areas_list(&pool, status, add_error, None).await
}

pub async fn archive_life_area(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(id): Path<i64>,
) -> Result<Response, StatusCode> {
    store::archive(&pool, id, clock.now_ms())
        .await
        .map_err(write_failed)?;
    render_life_areas_list(&pool, StatusCode::OK, None, None).await
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
async fn try_add_bands(
    pool: &SqlitePool,
    life_area_id: i64,
    bands: Vec<Band>,
) -> Result<(StatusCode, Option<(i64, String)>), StatusCode> {
    let existing_rows = store::list_guardrail_bands(pool, life_area_id)
        .await
        .map_err(write_failed)?;
    let existing: Vec<Band> = existing_rows
        .iter()
        .map(|row| Band {
            weekday: Weekday::parse(&row.weekday).expect("stored weekday is well-formed"),
            start_minutes: row.start_minutes,
            end_minutes: row.end_minutes,
        })
        .collect();
    if bands.iter().any(|band| overlaps(&existing, band)) {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((
                life_area_id,
                "the band overlaps one the life area already has".to_string(),
            )),
        ));
    }
    for band in &bands {
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
    render_life_areas_list(&pool, status, None, row_error).await
}

pub async fn remove_guardrail_band(
    State(pool): State<SqlitePool>,
    Path(band_id): Path<i64>,
) -> Result<Response, StatusCode> {
    store::remove_guardrail_band(&pool, band_id)
        .await
        .map_err(write_failed)?;
    render_life_areas_list(&pool, StatusCode::OK, None, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn app(pool: &SqlitePool) -> axum::Router {
        crate::platform::app::build_app(pool.clone(), Clock::system())
    }

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn get_life_areas(pool: &SqlitePool) -> String {
        let response = app(pool)
            .await
            .oneshot(
                Request::builder()
                    .uri("/life-areas")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        body_string(response).await
    }

    async fn post_life_area(pool: &SqlitePool, name: &str) -> Response {
        app(pool)
            .await
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/life-areas")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("name={}", urlencoding_for_tests(name))))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Minimal percent-encoding for the handful of characters these tests
    /// submit -- not a general-purpose form encoder.
    fn urlencoding_for_tests(value: &str) -> String {
        let mut out = String::new();
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char)
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }

    async fn archive_by_name(pool: &SqlitePool, name: &str) -> Response {
        let id = store::find_by_name(pool, name).await.unwrap().unwrap().id;
        app(pool)
            .await
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/life-areas/{id}/archive"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn life_area_id(pool: &SqlitePool, name: &str) -> i64 {
        store::find_by_name(pool, name).await.unwrap().unwrap().id
    }

    async fn post_guardrail(
        pool: &SqlitePool,
        life_area_id: i64,
        fields: &[(&str, &str)],
    ) -> Response {
        let body = fields
            .iter()
            .map(|(name, value)| format!("{name}={}", urlencoding_for_tests(value)))
            .collect::<Vec<_>>()
            .join("&");
        app(pool)
            .await
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/life-areas/{life_area_id}/guardrail"))
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn remove_band(pool: &SqlitePool, band_id: i64) -> Response {
        app(pool)
            .await
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/guardrail-bands/{band_id}/remove"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_fresh_database_lists_the_five_seeded_life_areas() {
        let (_dir, pool) = test_pool().await;

        let body = get_life_areas(&pool).await;

        for expected in ["Work", "Fitness", "Learning", "Family", "Home"] {
            assert!(body.contains(expected), "expected {expected} in:\n{body}");
        }
    }

    #[tokio::test]
    async fn a_fresh_database_shows_no_guardrail_for_every_life_area() {
        let (_dir, pool) = test_pool().await;

        let body = get_life_areas(&pool).await;

        assert_eq!(
            body.matches("no guardrail").count(),
            5,
            "expected all five seeded life areas to show no guardrail, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn adding_a_life_area_is_not_a_redirect_and_lists_it_immediately() {
        let (_dir, pool) = test_pool().await;

        let response = post_life_area(&pool, "Side project").await;

        assert!(!response.status().is_redirection());
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_string(response).await;
        assert!(body.contains("Side project"));
    }

    #[tokio::test]
    async fn adding_a_duplicate_name_is_rejected_whatever_the_case() {
        for variant in ["Work", "work", "WORK"] {
            let (_dir, pool) = test_pool().await;

            let response = post_life_area(&pool, variant).await;

            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = body_string(response).await;
            assert!(
                body.contains("already a life area"),
                "expected a duplicate rejection for {variant:?}, got:\n{body}"
            );
        }
    }

    #[tokio::test]
    async fn a_duplicate_add_does_not_change_the_listed_set() {
        let (_dir, pool) = test_pool().await;

        post_life_area(&pool, "work").await;
        let body = get_life_areas(&pool).await;

        let work_count = body.matches("Work").count();
        assert_eq!(work_count, 1, "expected exactly one Work, got:\n{body}");
    }

    #[tokio::test]
    async fn a_name_is_trimmed_before_storage() {
        let (_dir, pool) = test_pool().await;

        post_life_area(&pool, "  Side project  ").await;

        let stored = store::find_by_name(&pool, "Side project").await.unwrap();
        assert_eq!(stored.map(|a| a.name), Some("Side project".to_string()));
    }

    #[tokio::test]
    async fn a_whitespace_only_name_is_rejected_naming_the_field() {
        let (_dir, pool) = test_pool().await;

        let response = post_life_area(&pool, "   ").await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(
            body.contains("name"),
            "expected the rejection to name the field, got:\n{body}"
        );

        let after = get_life_areas(&pool).await;
        for expected in ["Work", "Fitness", "Learning", "Family", "Home"] {
            assert!(after.contains(expected));
        }
    }

    #[tokio::test]
    async fn archiving_removes_a_life_area_from_the_list_but_not_the_table() {
        let (_dir, pool) = test_pool().await;

        let response = archive_by_name(&pool, "Learning").await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = get_life_areas(&pool).await;
        assert!(!body.contains("Learning"), "got:\n{body}");
        assert!(store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn hostile_text_in_a_name_is_escaped_where_the_list_renders_it() {
        let (_dir, pool) = test_pool().await;

        post_life_area(&pool, "<script>alert('boom')</script>").await;
        let body = get_life_areas(&pool).await;

        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }

    #[tokio::test]
    async fn saving_a_band_is_accepted_and_listed_as_authored() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(
            &pool,
            id,
            &[
                ("mon", "on"),
                ("tue", "on"),
                ("wed", "on"),
                ("thu", "on"),
                ("fri", "on"),
                ("start", "09:00"),
                ("end", "17:00"),
            ],
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_string(response).await;
        assert!(
            body.contains("Mon, Tue, Wed, Thu, Fri 09:00-17:00"),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn two_separate_bands_both_list_and_count_as_two() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Fitness").await;

        post_guardrail(
            &pool,
            id,
            &[
                ("mon", "on"),
                ("wed", "on"),
                ("fri", "on"),
                ("start", "06:00"),
                ("end", "07:00"),
            ],
        )
        .await;
        let response = post_guardrail(
            &pool,
            id,
            &[("sat", "on"), ("start", "09:00"), ("end", "11:00")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_string(response).await;
        assert!(body.contains("Mon, Wed, Fri 06:00-07:00"), "got:\n{body}");
        assert!(body.contains("Sat 09:00-11:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn saving_pool_only_is_accepted_and_shown() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(&pool, id, &[("pool_only", "on")]).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("never scheduled"), "got:\n{body}");
    }

    #[tokio::test]
    async fn saving_neither_a_band_nor_pool_only_is_rejected_naming_both_options() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(&pool, id, &[]).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(body.contains("guardrail band"), "got:\n{body}");
        assert!(body.contains("never scheduled"), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_band_naming_no_weekday_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(&pool, id, &[("start", "09:00"), ("end", "17:00")]).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(!body.contains("09:00-17:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_band_whose_end_does_not_follow_its_start_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "17:00"), ("end", "09:00")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn an_overlapping_band_on_the_same_life_area_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;
        post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "09:00"), ("end", "12:00")],
        )
        .await;

        let response = post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "11:00"), ("end", "17:00")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(body.contains("overlaps"), "got:\n{body}");
    }

    #[tokio::test]
    async fn touching_bands_are_both_kept() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;
        post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "09:00"), ("end", "12:00")],
        )
        .await;

        let response = post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "12:00"), ("end", "17:00")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_string(response).await;
        assert!(body.contains("Mon 09:00-12:00"), "got:\n{body}");
        assert!(body.contains("Mon 12:00-17:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn two_life_areas_may_claim_the_same_hours() {
        let (_dir, pool) = test_pool().await;
        let work = life_area_id(&pool, "Work").await;
        let learning = life_area_id(&pool, "Learning").await;
        post_guardrail(
            &pool,
            work,
            &[("mon", "on"), ("start", "09:00"), ("end", "17:00")],
        )
        .await;

        let response = post_guardrail(
            &pool,
            learning,
            &[("mon", "on"), ("start", "09:00"), ("end", "17:00")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn removing_a_band_leaves_the_life_area_with_no_guardrail() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;
        post_guardrail(
            &pool,
            id,
            &[("mon", "on"), ("start", "09:00"), ("end", "17:00")],
        )
        .await;
        let bands = store::list_guardrail_bands(&pool, id).await.unwrap();
        let band_id = bands[0].id;

        let response = remove_band(&pool, band_id).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(!body.contains("09:00-17:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn hostile_text_as_a_band_start_stays_escaped() {
        let (_dir, pool) = test_pool().await;
        let id = life_area_id(&pool, "Work").await;

        let response = post_guardrail(
            &pool,
            id,
            &[
                ("mon", "on"),
                ("start", "<script>alert('boom')</script>"),
                ("end", "17:00"),
            ],
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(!body.contains("<script>"), "got:\n{body}");
    }

    #[tokio::test]
    async fn the_page_reports_the_owners_timezone() {
        let (_dir, pool) = test_pool().await;

        let body = get_life_areas(&pool).await;

        assert!(body.contains("UTC"), "got:\n{body}");
    }
}
