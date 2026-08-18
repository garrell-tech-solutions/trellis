//! `GET /life-areas`, `POST /life-areas`, `POST /life-areas/{id}/archive`.
//!
//! Management lives on its own route (`#45` already established that a
//! further page-level concern earns one, once `/` had both the inbox and the
//! task list). The triage picker stays on `/`, where triage happens; this
//! module never renders it -- [`crate::inbox`] reaches in here for the
//! options, the same direction `capture` and `triage` already reach into
//! `inbox::view`.
//!
//! Both writes follow `T-forms-swap-one-fragment`: a rejection re-renders
//! the same `#life-areas-list` fragment carrying the error, as a `422`, and
//! the archive control lives inline in the row it retires.

use crate::life_areas::store;
use crate::life_areas::view::LifeAreaOption;
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
    life_areas: Vec<LifeAreaOption>,
    error: Option<String>,
    nav: Vec<NavLink>,
}

/// The `#life-areas-list` fragment on its own -- what both a successful add
/// and a rejected one swap in, and what an archive swaps in too.
#[derive(Template)]
#[template(path = "life_areas_list.html")]
struct LifeAreasListTemplate {
    life_areas: Vec<LifeAreaOption>,
    error: Option<String>,
}

pub async fn show_life_areas(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let life_areas = crate::life_areas::active_options(&pool)
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &LifeAreasTemplate {
            life_areas,
            error: None,
            nav: nav::links(Page::LifeAreas),
        },
    ))
}

async fn render_life_areas_list(
    pool: &SqlitePool,
    status: StatusCode,
    error: Option<String>,
) -> Result<Response, StatusCode> {
    let life_areas = crate::life_areas::active_options(pool)
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        status,
        &LifeAreasListTemplate { life_areas, error },
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
    let (status, error) = match try_add_life_area(&pool, &payload.name).await? {
        Ok(()) => (StatusCode::CREATED, None),
        Err(message) => (StatusCode::UNPROCESSABLE_ENTITY, Some(message)),
    };
    render_life_areas_list(&pool, status, error).await
}

pub async fn archive_life_area(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(id): Path<i64>,
) -> Result<Response, StatusCode> {
    store::archive(&pool, id, clock.now_ms())
        .await
        .map_err(write_failed)?;
    render_life_areas_list(&pool, StatusCode::OK, None).await
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

    #[tokio::test]
    async fn a_fresh_database_lists_the_five_seeded_life_areas() {
        let (_dir, pool) = test_pool().await;

        let body = get_life_areas(&pool).await;

        for expected in ["Work", "Fitness", "Learning", "Family", "Home"] {
            assert!(body.contains(expected), "expected {expected} in:\n{body}");
        }
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
}
