//! `GET /quota`: the fourth screen, quotas grouped by nothing but the order
//! they were defined in (#93). `POST /quota`: defines a new one, guarded
//! against a mistyped name (`D-quotas-are-selected-not-typed`).

use super::body::{self, DefineFormView, DEFAULT_BUTTON_LABEL};
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use crate::quota::view::QuotaRowView;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Form;
use scheduler_core::quota::{NameMatch, QuotaDefinition};
use serde::Deserialize;
use sqlx::SqlitePool;

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
    pending_name: String,
    pending_hours: String,
    warning: Option<String>,
    button_label: &'static str,
    confirm_name: Option<String>,
    nav: Vec<NavLink>,
}

pub async fn show_quota(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let built = body::build(&pool, clock, DefineFormView::default())
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
            pending_name: built.pending_name,
            pending_hours: built.pending_hours,
            warning: built.warning,
            button_label: built.button_label,
            confirm_name: built.confirm_name,
            nav: nav::links(Page::Quota),
        },
    ))
}

/// The define form's own submission -- a plain HTML form, mirroring
/// `triage::http::TriageFormRequest`'s reasoning: every field optional here
/// too, since "missing" and "present but invalid" are different rejections
/// (`quota-screen-both-fields-required-03` vs `-target-must-be-positive-04`)
/// that only `scheduler_core::quota::QuotaDefinition::from_fields` can tell
/// apart.
#[derive(Deserialize, Default)]
pub struct DefineQuotaForm {
    name: Option<String>,
    hours: Option<String>,
    /// The name a *similar*-name warning was shown for, carried back by the
    /// button's own resubmission -- present only when this exact name has
    /// already been confirmed once (`quota-screen-similar-name-warns-07`).
    /// An *exact* match ignores this field entirely: it is never
    /// bypassable, confirmed or not.
    #[serde(default)]
    confirmed: Option<String>,
}

/// `minutes` the way the name-guard's own messages read it: `"4 h a week"`,
/// never `"4h"` -- the row readout's compact form is a different context
/// with its own established spelling, and this project does not invent a
/// third.
fn hours_a_week(minutes: i64) -> String {
    if minutes % 60 == 0 {
        format!("{} h a week", minutes / 60)
    } else {
        format!("{:.1} h a week", minutes as f64 / 60.0)
    }
}

/// The re-rendered fragment for a rejected submission, echoing back what
/// was typed (`T-forms-swap-one-fragment`, `T-422-is-product-wide`).
async fn rejected(
    pool: &SqlitePool,
    clock: Clock,
    pending_name: String,
    pending_hours: String,
    warning: Option<String>,
    button_label: &'static str,
    confirm_name: Option<String>,
) -> Result<Response, StatusCode> {
    body::respond(
        pool,
        clock,
        StatusCode::UNPROCESSABLE_ENTITY,
        DefineFormView {
            pending_name,
            pending_hours,
            warning,
            button_label,
            confirm_name,
        },
    )
    .await
}

/// Whether `confirmed` already carries this exact candidate name -- the
/// resubmission a "Create anyway" tap sends, distinguished from a first
/// attempt that has not been warned about yet.
fn already_confirmed(confirmed: Option<&str>, candidate_name: &str) -> bool {
    confirmed == Some(candidate_name)
}

/// Defines a new quota, or refuses and re-renders the same form carrying
/// why (`T-one-front-door-per-capability`: one write path, `store::create`,
/// reached only from here).
pub async fn define_quota(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Form(form): Form<DefineQuotaForm>,
) -> Result<Response, StatusCode> {
    let pending_name = form.name.clone().unwrap_or_default();
    let pending_hours = form.hours.clone().unwrap_or_default();

    let definition = match QuotaDefinition::from_fields(form.name.as_deref(), form.hours.as_deref())
    {
        Ok(definition) => definition,
        // A missing name/hours and an out-of-domain hours value both
        // re-render the same reset form (`quota-screen-both-fields-
        // required-03`, `-target-must-be-positive-04`): neither scenario
        // asserts a message here, only that nothing was created.
        Err(_) => {
            return rejected(
                &pool,
                clock,
                pending_name,
                pending_hours,
                None,
                DEFAULT_BUTTON_LABEL,
                None,
            )
            .await;
        }
    };

    let existing = super::store::existing_names(&pool)
        .await
        .map_err(write_failed)?;
    let name_match = scheduler_core::quota::check_name(&definition.name, &existing);

    respond_to_name_match(
        &pool,
        clock,
        definition,
        name_match,
        form.confirmed.as_deref(),
        pending_name,
        pending_hours,
    )
    .await
}

/// The three ways a checked name can go: an exact duplicate, never
/// bypassable; a similar one, warned once and created on confirmation
/// (`quota-screen-similar-name-warns-07`); or nothing in its way, created
/// outright. Split out of [`define_quota`] so each function's own branching
/// stays under `T-complexity-8`.
async fn respond_to_name_match(
    pool: &SqlitePool,
    clock: Clock,
    definition: QuotaDefinition,
    name_match: Option<NameMatch<'_>>,
    confirmed: Option<&str>,
    pending_name: String,
    pending_hours: String,
) -> Result<Response, StatusCode> {
    match name_match {
        Some(NameMatch::Exact(existing_name, existing_minutes)) => {
            reject_exact_match(
                pool,
                clock,
                pending_name,
                pending_hours,
                existing_name,
                existing_minutes,
            )
            .await
        }
        Some(NameMatch::Similar(existing_name, existing_minutes))
            if !already_confirmed(confirmed, &definition.name) =>
        {
            warn_similar_match(
                pool,
                clock,
                pending_name,
                pending_hours,
                existing_name,
                existing_minutes,
                definition.name.clone(),
            )
            .await
        }
        _ => create_quota(pool, clock, definition).await,
    }
}

/// An exact-match candidate is refused outright, never bypassable
/// (`D-quotas-are-selected-not-typed`): the reset form carries only the
/// existing quota's own name and target, nothing to confirm past.
async fn reject_exact_match(
    pool: &SqlitePool,
    clock: Clock,
    pending_name: String,
    pending_hours: String,
    existing_name: &str,
    existing_minutes: i64,
) -> Result<Response, StatusCode> {
    let warning = format!(
        "\u{201c}{existing_name}\u{201d} already exists at {}. File it there instead of making a second one.",
        hours_a_week(existing_minutes)
    );
    rejected(
        pool,
        clock,
        pending_name,
        pending_hours,
        Some(warning),
        DEFAULT_BUTTON_LABEL,
        None,
    )
    .await
}

/// A similar-match candidate is warned once; `candidate_name` rides back on
/// the reset form's hidden `confirmed` field so a "Create anyway" resubmit
/// can be told apart from a first attempt (`quota-screen-similar-name-
/// warns-07`, [`already_confirmed`]).
async fn warn_similar_match(
    pool: &SqlitePool,
    clock: Clock,
    pending_name: String,
    pending_hours: String,
    existing_name: &str,
    existing_minutes: i64,
    candidate_name: String,
) -> Result<Response, StatusCode> {
    let warning = format!(
        "That reads a lot like \u{201c}{existing_name}\u{201d} ({}). Same thing?",
        hours_a_week(existing_minutes)
    );
    rejected(
        pool,
        clock,
        pending_name,
        pending_hours,
        Some(warning),
        "Create anyway",
        Some(candidate_name),
    )
    .await
}

async fn create_quota(
    pool: &SqlitePool,
    clock: Clock,
    definition: QuotaDefinition,
) -> Result<Response, StatusCode> {
    super::store::create(pool, &definition, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(pool, clock, StatusCode::CREATED, DefineFormView::default()).await
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
            let rejection = body::respond(
                pool,
                clock,
                StatusCode::UNPROCESSABLE_ENTITY,
                DefineFormView::default(),
            )
            .await?;
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
    Form(form): Form<SessionForm>,
) -> Result<Response, StatusCode> {
    let (day_ms, minutes) = match validated_session(&pool, clock, &form).await? {
        Ok(session) => session,
        Err(rejection) => return Ok(rejection),
    };
    super::store::log_session(&pool, quota_id, day_ms, minutes, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::CREATED, DefineFormView::default()).await
}

/// `POST /quota/sessions/{session_id}` -- corrects a logged session's day
/// and minutes in place (`quota-sessions-correcting-a-session-06`).
pub async fn correct_session(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(session_id): Path<i64>,
    Form(form): Form<SessionForm>,
) -> Result<Response, StatusCode> {
    let (day_ms, minutes) = match validated_session(&pool, clock, &form).await? {
        Ok(session) => session,
        Err(rejection) => return Ok(rejection),
    };
    super::store::update_session(&pool, session_id, day_ms, minutes)
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::OK, DefineFormView::default()).await
}

/// `POST /quota/sessions/{session_id}/delete` -- deletes a logged session
/// outright, taking its minutes with it
/// (`quota-sessions-deleting-a-session-07`).
pub async fn delete_session(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(session_id): Path<i64>,
) -> Result<Response, StatusCode> {
    super::store::delete_session(&pool, session_id)
        .await
        .map_err(write_failed)?;
    body::respond(&pool, clock, StatusCode::OK, DefineFormView::default()).await
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

    async fn post_define(pool: &SqlitePool, fields: &[(&str, &str)]) -> (StatusCode, String) {
        let body = fields
            .iter()
            .map(|(name, value)| format!("{name}={}", urlencode(value)))
            .collect::<Vec<_>>()
            .join("&");
        let app = crate::platform::app::build_app(pool.clone(), Clock::pinned_at(1_000));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quota")
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

    #[tokio::test]
    async fn the_quota_screen_is_reachable_and_shows_the_empty_state_when_nothing_is_defined() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = get_quota(&pool).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("A quota is a weekly hour target you keep"),
            "got:\n{body}"
        );
        assert!(body.contains("+ Define a new quota"), "got:\n{body}");
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
    async fn defining_a_quota_creates_it_and_shows_its_target() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = post_define(&pool, &[("name", "Piano"), ("hours", "4")]).await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("Piano"), "got:\n{body}");
        assert!(body.contains("0m / 4h"), "got:\n{body}");
        assert!(body.contains("4h left this week"), "got:\n{body}");
    }

    #[tokio::test]
    async fn defining_a_quota_without_a_name_is_rejected_and_creates_nothing() {
        let (_dir, pool) = test_pool().await;

        let (status, _) = post_define(&pool, &[("hours", "4")]).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn defining_a_quota_without_hours_is_rejected_and_creates_nothing() {
        let (_dir, pool) = test_pool().await;

        let (status, _) = post_define(&pool, &[("name", "Piano")]).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn defining_a_quota_with_zero_hours_is_rejected() {
        let (_dir, pool) = test_pool().await;

        let (status, _) = post_define(&pool, &[("name", "Piano"), ("hours", "0")]).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn a_repeated_name_is_refused_with_a_message_naming_the_existing_quota() {
        let (_dir, pool) = test_pool().await;
        post_define(&pool, &[("name", "Piano"), ("hours", "4")]).await;

        let (status, body) = post_define(&pool, &[("name", "piano"), ("hours", "2")]).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body.contains(
                "\u{201c}Piano\u{201d} already exists at 4 h a week. File it there instead of making a second one."
            ),
            "got:\n{body}"
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "the exact-match duplicate must not be created");
    }

    #[tokio::test]
    async fn an_exact_match_cannot_be_bypassed_by_confirming() {
        let (_dir, pool) = test_pool().await;
        post_define(&pool, &[("name", "Piano"), ("hours", "4")]).await;

        let (status, _) = post_define(
            &pool,
            &[("name", "piano"), ("hours", "2"), ("confirmed", "piano")],
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn a_similar_name_warns_but_offers_to_create_anyway() {
        let (_dir, pool) = test_pool().await;
        post_define(&pool, &[("name", "Piano"), ("hours", "4")]).await;

        let (status, body) = post_define(&pool, &[("name", "Pianoo"), ("hours", "2")]).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body.contains("That reads a lot like \u{201c}Piano\u{201d} (4 h a week). Same thing?"),
            "got:\n{body}"
        );
        assert!(body.contains("Create anyway"), "got:\n{body}");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "the warned name must not be created yet");
    }

    #[tokio::test]
    async fn confirming_a_similar_name_creates_it() {
        let (_dir, pool) = test_pool().await;
        post_define(&pool, &[("name", "Piano"), ("hours", "4")]).await;

        let (status, body) = post_define(
            &pool,
            &[("name", "Pianoo"), ("hours", "2"), ("confirmed", "Pianoo")],
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert!(body.contains("Pianoo"), "got:\n{body}");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn a_triaged_quota_task_does_not_appear_on_the_quota_screen() {
        let (_dir, pool) = test_pool().await;
        let capture_id = crate::capture::store::insert(&pool, "practise piano", "web", None, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &scheduler_core::task::TaskKind::Quota {
                target_count: 3,
                target_minutes_each: 45,
                period: scheduler_core::task::Period::Week,
            },
            0,
        )
        .await
        .unwrap();

        let (_, body) = get_quota(&pool).await;

        assert!(!body.contains("practise piano"), "got:\n{body}");
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

    async fn given_a_quota(pool: &SqlitePool, name: &str, hours: &str) -> i64 {
        post_define(pool, &[("name", name), ("hours", hours)]).await;
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
}
