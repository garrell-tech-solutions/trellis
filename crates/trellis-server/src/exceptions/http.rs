//! `POST /exceptions`, `POST /exceptions/{id}/remove` -- the free time
//! page's own exception controls (#61). Both render into the
//! `#exceptions-list` fragment `free_time.html` also includes on
//! `GET /free-time` (`T-forms-swap-one-fragment`); a rejection re-renders
//! the same fragment carrying the reason, as a `422` (`T-422-is-product-
//! wide`).

use super::store;
use super::view::ExceptionListItem;
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use jiff::civil::Date;
use scheduler_core::exception::{well_formed_range, DateRange, ExceptionRejection};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::str::FromStr;

/// The `#exceptions-list` fragment on its own -- what an add and a removal
/// both swap in.
#[derive(Template)]
#[template(path = "exceptions_list.html")]
struct ExceptionsListTemplate {
    exceptions: Vec<ExceptionListItem>,
    exception_error: Option<String>,
}

async fn render_exceptions_list(
    pool: &SqlitePool,
    status: StatusCode,
    exception_error: Option<String>,
) -> Result<Response, StatusCode> {
    let exceptions = super::list(pool).await.map_err(write_failed)?;
    Ok(render_template(
        status,
        &ExceptionsListTemplate {
            exceptions,
            exception_error,
        },
    ))
}

/// The exception form's own fields. `life_area` blank or absent means
/// global; a non-empty value must resolve through
/// `life_areas::active_id_for_name`, the one answer to "does this name a
/// life area work may be filed under" (`T-one-front-door-per-capability`).
/// `label` is the only free text an exception carries.
#[derive(Deserialize, Default)]
pub struct ExceptionFormRequest {
    start: String,
    end: String,
    life_area: Option<String>,
    label: Option<String>,
}

/// Resolves the submitted `life_area` field: `Ok(None)` for global,
/// `Ok(Some(id))` for a name that names an active life area, `Err(message)`
/// for one that does not. Split from [`decide_exception`] so that
/// function's own match stays one outcome per arm.
async fn resolve_scope(pool: &SqlitePool, life_area: Option<&str>) -> Result<Option<i64>, String> {
    let Some(name) = life_area.filter(|name| !name.is_empty()) else {
        return Ok(None);
    };
    crate::life_areas::active_id_for_name(pool, name)
        .await
        .map_err(|_| "the exception's life area could not be looked up".to_string())?
        .map(Some)
        .ok_or_else(|| format!("{name:?} is not a life area"))
}

fn parse_date(value: &str) -> Option<Date> {
    Date::from_str(value).ok()
}

fn rejection_message(rejection: ExceptionRejection) -> &'static str {
    match rejection {
        ExceptionRejection::Backwards => "the last day precedes the first",
    }
}

/// The submitted dates, well-formed -- both present, parseable, and the
/// last not preceding the first.
fn well_formed_dates(form: &ExceptionFormRequest) -> Result<DateRange, String> {
    let start =
        parse_date(&form.start).ok_or_else(|| "the start date is not a date".to_string())?;
    let end = parse_date(&form.end).ok_or_else(|| "the end date is not a date".to_string())?;
    well_formed_range(start, end).map_err(|rejection| rejection_message(rejection).to_string())
}

async fn decide_exception(
    pool: &SqlitePool,
    form: &ExceptionFormRequest,
) -> Result<(StatusCode, Option<String>), StatusCode> {
    let range = match well_formed_dates(form) {
        Err(message) => return Ok((StatusCode::UNPROCESSABLE_ENTITY, Some(message))),
        Ok(range) => range,
    };
    let life_area_id = match resolve_scope(pool, form.life_area.as_deref()).await {
        Err(message) => return Ok((StatusCode::UNPROCESSABLE_ENTITY, Some(message))),
        Ok(life_area_id) => life_area_id,
    };
    store::insert(
        pool,
        life_area_id,
        &range.start.to_string(),
        &range.end.to_string(),
        form.label.as_deref().unwrap_or(""),
    )
    .await
    .map_err(write_failed)?;
    Ok((StatusCode::CREATED, None))
}

pub async fn add_exception(
    State(pool): State<SqlitePool>,
    Form(form): Form<ExceptionFormRequest>,
) -> Result<Response, StatusCode> {
    let (status, exception_error) = decide_exception(&pool, &form).await?;
    render_exceptions_list(&pool, status, exception_error).await
}

pub async fn remove_exception(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Response, StatusCode> {
    store::remove(&pool, id).await.map_err(write_failed)?;
    render_exceptions_list(&pool, StatusCode::OK, None).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::app::build_app;
    use crate::platform::clock::Clock;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn post_form(pool: &SqlitePool, uri: &str, body: &str) -> (StatusCode, String) {
        let app = build_app(pool.clone(), Clock::system());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn adding_a_global_exception_is_created_and_listed() {
        let (_dir, pool) = test_pool().await;

        let (status, body) =
            post_form(&pool, "/exceptions", "start=2026-08-24&end=2026-08-28").await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("All life areas"), "got:\n{body}");
        assert!(body.contains("2026-08-24"), "got:\n{body}");
    }

    #[tokio::test]
    async fn adding_a_life_area_scoped_exception_names_that_life_area() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = post_form(
            &pool,
            "/exceptions",
            "start=2026-08-24&end=2026-08-28&life_area=Work",
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("Work"), "got:\n{body}");
        assert!(!body.contains("All life areas"), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_backwards_range_is_rejected_naming_the_reason_and_stores_nothing() {
        let (_dir, pool) = test_pool().await;

        let (status, body) =
            post_form(&pool, "/exceptions", "start=2026-08-28&end=2026-08-24").await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body.contains("the last day precedes the first"),
            "got:\n{body}"
        );
        assert!(!body.contains("2026-08-24"), "got:\n{body}");
    }

    #[tokio::test]
    async fn an_unknown_life_area_name_is_rejected() {
        let (_dir, pool) = test_pool().await;

        let (status, _) = post_form(
            &pool,
            "/exceptions",
            "start=2026-08-24&end=2026-08-28&life_area=Gardening",
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn removing_an_exception_takes_it_off_the_list() {
        let (_dir, pool) = test_pool().await;
        store::insert(&pool, None, "2026-08-24", "2026-08-28", "")
            .await
            .unwrap();
        let id = store::list_all(&pool).await.unwrap()[0].id;

        let (status, body) = post_form(&pool, &format!("/exceptions/{id}/remove"), "").await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("exceptions-list"), "not a re-render:\n{body}");
        assert!(!body.contains("2026-08-24"), "got:\n{body}");
        assert!(
            store::list_all(&pool).await.unwrap().is_empty(),
            "the exception was not removed from storage"
        );
    }

    #[test]
    fn well_formed_dates_reports_the_backwards_range_message() {
        let form = ExceptionFormRequest {
            start: "2026-08-28".to_string(),
            end: "2026-08-24".to_string(),
            ..ExceptionFormRequest::default()
        };
        assert_eq!(
            well_formed_dates(&form),
            Err("the last day precedes the first".to_string())
        );
    }

    #[test]
    fn well_formed_dates_accepts_a_single_day() {
        let form = ExceptionFormRequest {
            start: "2026-08-24".to_string(),
            end: "2026-08-24".to_string(),
            ..ExceptionFormRequest::default()
        };
        assert!(well_formed_dates(&form).is_ok());
    }
}
