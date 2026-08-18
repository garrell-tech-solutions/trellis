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
//! HTTP for something else. [`guardrail`] composes that decision with the
//! database question neither can answer alone -- split into its own module
//! from the life-area listing/CRUD handlers here, which is a different
//! concern that happens to render into the same fragment.

mod guardrail;

pub use guardrail::{remove_guardrail_band, save_guardrail};

use crate::life_areas::store;
use crate::life_areas::view::LifeAreaListItem;
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
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
            bands: guardrail::group_bands(bands),
            error: row_error
                .as_ref()
                .filter(|(id, _)| *id == row.id)
                .map(|(_, message)| message.clone()),
        });
    }
    Ok(items)
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

    /// The markup for one life area's own `<li>`, scoped by its row id so a
    /// message attached to one row can't be mistaken for a message on
    /// another -- `row_error`'s whole job is telling those apart.
    fn row_of(body: &str, life_area_id: i64) -> &str {
        let marker = format!(r#"id="life-area-row-{life_area_id}""#);
        let start = body
            .find(&marker)
            .unwrap_or_else(|| panic!("no row for life area {life_area_id} in:\n{body}"));
        let end = body[start..]
            .find("</li>")
            .unwrap_or_else(|| panic!("row {life_area_id} is not closed in:\n{body}"));
        &body[start..start + end]
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
    async fn a_rejected_guardrail_names_only_the_life_area_it_was_submitted_for() {
        let (_dir, pool) = test_pool().await;
        let work_id = life_area_id(&pool, "Work").await;
        let fitness_id = life_area_id(&pool, "Fitness").await;

        let response = post_guardrail(&pool, work_id, &[]).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_string(response).await;
        assert!(
            row_of(&body, work_id).contains("guardrail-error"),
            "got:\n{body}"
        );
        assert!(
            !row_of(&body, fitness_id).contains("guardrail-error"),
            "Fitness carried Work's rejection:\n{body}"
        );
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
        assert!(body.contains("Work"), "not a re-rendered list:\n{body}");
        assert!(!body.contains("09:00-17:00"), "got:\n{body}");
        assert!(
            store::list_guardrail_bands(&pool, id)
                .await
                .unwrap()
                .is_empty(),
            "the band was not removed from storage"
        );
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
