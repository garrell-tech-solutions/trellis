//! `GET /quota`: the fourth screen, quotas grouped by nothing but the order
//! they were triaged in (#138). Logging, correcting and deleting a session
//! against one are this module's other routes; defining one is not --
//! triage is the one door (#138, `crate::triage::http`).

use super::body;
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use crate::quota::view::QuotaRowView;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashSet;

/// Which quota rows render already expanded (#93, quota-sessions): the
/// same shape `pool::http::ExpandedQuery` takes for `#120` -- client state
/// the browser echoes back on every request from inside `#quota-body`, so
/// a row you opened stays open across the `outerHTML` swap logging a
/// session causes. Comma-separated quota ids; absent or empty means
/// nothing is expanded, which is every request `GET /quota` itself sends.
#[derive(Deserialize, Default)]
pub struct ExpandedQuery {
    #[serde(default)]
    expanded: String,
}

fn expanded_ids(query: &ExpandedQuery) -> HashSet<i64> {
    query
        .expanded
        .split(',')
        .filter_map(|id| id.parse().ok())
        .collect()
}

/// The full page is the only thing that is not the `#quota-body` fragment,
/// so it is the only caller that takes `body::build`'s fields apart instead
/// of going through `body::respond` -- the same shape `pool::http::show_pool`
/// takes.
#[derive(Template)]
#[template(path = "quota.html")]
struct QuotaTemplate {
    meta: String,
    empty: bool,
    quotas: Vec<QuotaRowView>,
    day_options: Vec<String>,
    today: String,
    nav: Vec<NavLink>,
}

pub async fn show_quota(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let built = body::build(&pool, clock, &HashSet::new())
        .await
        .map_err(write_failed)?;
    Ok(render_template(
        StatusCode::OK,
        &QuotaTemplate {
            meta: built.meta,
            empty: built.empty,
            quotas: built.quotas,
            day_options: built.day_options,
            today: built.today,
            nav: nav::links(Page::Quota),
        },
    ))
}

/// A session write's own submission -- the quick-log buttons' hidden
/// fields and the `Other…`/correction forms' visible ones all land here the
/// same way (`T-one-front-door-per-capability`).
#[derive(Deserialize, Default)]
pub struct SessionForm {
    day: Option<String>,
    minutes: Option<String>,
}

/// Validates a session submission against the current week, or refuses --
/// shared by [`log_session`] and [`correct_session`], which differ only in
/// which store write and which success status follow
/// (`quota-sessions-a-session-must-be-positive-09`,
/// `-only-days-that-have-happened-03`'s server-side half: a guard that only
/// lives in the day picker's own options is not a guard).
async fn validated_session(
    pool: &SqlitePool,
    clock: Clock,
    form: &SessionForm,
    expanded_ids: &HashSet<i64>,
) -> Result<Result<(i64, i64), Response>, StatusCode> {
    let (week, zone) = body::current_week(pool, clock)
        .await
        .map_err(write_failed)?;
    match scheduler_core::quota::validate_session(
        form.day.as_deref(),
        form.minutes.as_deref(),
        &week,
    ) {
        Ok(session) => Ok(Ok((week.day_ms(session.day, &zone), session.minutes))),
        Err(_) => {
            let rejection =
                body::respond(pool, clock, StatusCode::UNPROCESSABLE_ENTITY, expanded_ids).await?;
            Ok(Err(rejection))
        }
    }
}

/// `POST /quota/{quota_id}/sessions` -- logs a session against `quota_id`.
/// The quick-log buttons submit this with a hidden `day` already set to
/// today and `minutes` fixed at `30`/`60`; `Other…` submits whatever the
/// picker and the minutes field held.
pub async fn log_session(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(quota_id): Path<i64>,
    Query(expanded): Query<ExpandedQuery>,
    Form(form): Form<SessionForm>,
) -> Result<Response, StatusCode> {
    let expanded_ids = expanded_ids(&expanded);
    let (day_ms, minutes) = match validated_session(&pool, clock, &form, &expanded_ids).await? {
        Ok(session) => session,
        Err(rejection) => return Ok(rejection),
    };
    super::store::log_session(&pool, quota_id, day_ms, minutes, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::CREATED, &expanded_ids).await
}

/// `POST /quota/sessions/{session_id}` -- corrects a logged session's day
/// and minutes in place (`quota-sessions-correcting-a-session-06`).
pub async fn correct_session(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(session_id): Path<i64>,
    Query(expanded): Query<ExpandedQuery>,
    Form(form): Form<SessionForm>,
) -> Result<Response, StatusCode> {
    let expanded_ids = expanded_ids(&expanded);
    let (day_ms, minutes) = match validated_session(&pool, clock, &form, &expanded_ids).await? {
        Ok(session) => session,
        Err(rejection) => return Ok(rejection),
    };
    super::store::update_session(&pool, session_id, day_ms, minutes)
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::OK, &expanded_ids).await
}

/// `POST /quota/sessions/{session_id}/delete` -- deletes a logged session
/// outright, taking its minutes with it
/// (`quota-sessions-deleting-a-session-07`).
pub async fn delete_session(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(session_id): Path<i64>,
    Query(expanded): Query<ExpandedQuery>,
) -> Result<Response, StatusCode> {
    super::store::delete_session(&pool, session_id)
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::OK, &expanded_ids(&expanded)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get_quota(pool: &SqlitePool) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/quota")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    /// Percent-encoding for the handful of characters this module's own
    /// fixtures submit -- the same minimal encoder `inbox_view.rs`'s own
    /// acceptance-side helper is, kept local since a unit test's fixtures
    /// are simpler still (no curly quotes or non-ASCII to carry).
    fn urlencode(value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char);
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }

    #[tokio::test]
    async fn the_quota_screen_is_reachable_and_shows_the_empty_state_when_nothing_is_triaged() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = get_quota(&pool).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains(
                "A quota is a weekly hour target you keep — practice, study, running. Capture \
                 one and triage it."
            ),
            "got:\n{body}"
        );
        assert!(
            body.contains(r#"<a href="/" class="quota-go-capture">Go to Capture &rarr;</a>"#),
            "got:\n{body}"
        );
        assert!(
            !body.contains("quota-define-form"),
            "the quota screen must offer no way to define a quota, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_quota_screen_marks_quota_as_the_current_tab() {
        let (_dir, pool) = test_pool().await;

        let (_, body) = get_quota(&pool).await;

        assert!(
            body.contains(r#"aria-current="page">Quota</a>"#),
            "got:\n{body}"
        );
    }

    #[tokio::test]
    async fn a_quota_created_by_triage_shows_its_target() {
        let (_dir, pool) = test_pool().await;
        given_a_quota(&pool, "Piano", "4").await;

        let (_, body) = get_quota(&pool).await;

        assert!(body.contains("Piano"), "got:\n{body}");
        assert!(body.contains("0m / 4h"), "got:\n{body}");
        assert!(body.contains("4h left this week"), "got:\n{body}");
    }

    // --- sessions (#93, quota-sessions) -----------------------------------

    /// A Tuesday, both in UTC and in every zone a fresh database's own
    /// default settings row resolves to -- these tests do not touch
    /// `settings`, so `UTC` (`0006_guardrails.sql`) is what `Week::of`
    /// actually sees.
    fn tuesday_clock() -> Clock {
        let ms = "2026-08-25T14:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .as_millisecond();
        Clock::pinned_at(ms)
    }

    async fn post_path(
        pool: &SqlitePool,
        clock: Clock,
        path: &str,
        fields: &[(&str, &str)],
    ) -> (StatusCode, String) {
        let body = fields
            .iter()
            .map(|(name, value)| format!("{name}={}", urlencode(value)))
            .collect::<Vec<_>>()
            .join("&");
        let app = crate::platform::app::build_app(pool.clone(), clock);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    /// A quota is created by triage now (#138), not by a route this module
    /// owns -- so its own tests build one directly through the store, the
    /// same way `triage::http`'s tests build the `TaskKind` it decides on.
    async fn given_a_quota(pool: &SqlitePool, name: &str, hours: &str) -> i64 {
        let definition =
            scheduler_core::quota::QuotaDefinition::from_fields(Some(name), Some(hours))
                .expect("a valid fixture name and hours");
        crate::quota::store::create(pool, &definition, 0)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT id FROM quotas WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn logging_a_session_creates_it_and_updates_the_readout() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Tue"), ("minutes", "20")],
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("20m / 4h"), "got:\n{body}");
        assert!(body.contains("Tue 20m"), "got:\n{body}");
    }

    #[tokio::test]
    async fn logging_a_session_for_an_earlier_day_this_week_counts_the_same() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Mon"), ("minutes", "20")],
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("20m / 4h"), "got:\n{body}");
        assert!(body.contains("Mon 20m"), "got:\n{body}");
    }

    #[tokio::test]
    async fn logging_a_session_for_a_day_that_has_not_happened_yet_is_refused() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, _) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Wed"), ("minutes", "20")],
        )
        .await;

        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "not by the picker, and not by posting one directly"
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quota_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn logging_a_session_of_zero_minutes_is_refused() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, _) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Tue"), ("minutes", "0")],
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn correcting_a_session_changes_its_day_and_minutes() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;
        post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Mon"), ("minutes", "25")],
        )
        .await;
        let session_id: i64 = sqlx::query_scalar("SELECT id FROM quota_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/sessions/{session_id}"),
            &[("day", "Tue"), ("minutes", "45")],
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("45m / 4h"), "got:\n{body}");
        assert!(body.contains("Tue 45m"), "got:\n{body}");
        assert!(!body.contains("Mon 25m"), "got:\n{body}");
    }

    #[tokio::test]
    async fn deleting_a_session_removes_it_and_leaves_the_quota() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;
        post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Mon"), ("minutes", "25")],
        )
        .await;
        let session_id: i64 = sqlx::query_scalar("SELECT id FROM quota_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/sessions/{session_id}/delete"),
            &[],
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Piano"), "the quota itself must survive");
        assert!(body.contains("0m / 4h"), "got:\n{body}");
        assert!(body.contains("No sessions yet this week"), "got:\n{body}");
    }

    // --- #93 (quota-sessions): expand state rides along the request,
    // never stored -- the same shape pool::http's own `?expanded=` tests
    // take for #120. ---

    fn expanded_row(body: &str, quota_id: i64) -> bool {
        body.contains("<details class=\"quota-expand\" open>")
            && body.contains(&format!("id=\"quota-row-{quota_id}\""))
    }

    #[tokio::test]
    async fn a_quota_named_in_the_expanded_query_renders_expanded_after_logging_a_session() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions?expanded={quota_id}"),
            &[("day", "Tue"), ("minutes", "20")],
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(expanded_row(&body, quota_id), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_plain_log_with_no_expanded_query_renders_collapsed() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (_, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Tue"), ("minutes", "20")],
        )
        .await;

        assert!(
            !body.contains("<details class=\"quota-expand\" open>"),
            "expected no quota rendered expanded, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn correcting_a_session_preserves_the_expanded_query() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;
        post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Mon"), ("minutes", "25")],
        )
        .await;
        let session_id: i64 = sqlx::query_scalar("SELECT id FROM quota_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/sessions/{session_id}?expanded={quota_id}"),
            &[("day", "Mon"), ("minutes", "45")],
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(expanded_row(&body, quota_id), "got:\n{body}");
    }

    #[tokio::test]
    async fn deleting_a_session_preserves_the_expanded_query() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;
        post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions"),
            &[("day", "Mon"), ("minutes", "25")],
        )
        .await;
        let session_id: i64 = sqlx::query_scalar("SELECT id FROM quota_sessions")
            .fetch_one(&pool)
            .await
            .unwrap();

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/sessions/{session_id}/delete?expanded={quota_id}"),
            &[],
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(expanded_row(&body, quota_id), "got:\n{body}");
    }

    #[tokio::test]
    async fn a_rejected_session_still_preserves_the_expanded_query() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;

        let (status, body) = post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions?expanded={quota_id}"),
            &[("day", "Tue"), ("minutes", "0")],
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(expanded_row(&body, quota_id), "got:\n{body}");
    }

    #[tokio::test]
    async fn the_quota_screen_never_renders_expanded_on_a_fresh_get() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", "4").await;
        post_path(
            &pool,
            tuesday_clock(),
            &format!("/quota/{quota_id}/sessions?expanded={quota_id}"),
            &[("day", "Tue"), ("minutes", "20")],
        )
        .await;

        let (_, body) = get_quota(&pool).await;

        assert!(
            !body.contains("<details class=\"quota-expand\" open>"),
            "a fresh GET must always start collapsed, got:\n{body}"
        );
    }
}
