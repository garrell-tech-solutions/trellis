//! `POST /captures/{id}/triage`.
//!
//! One code path for both callers, the same shape as `capture::http`: the
//! JSON API and the page's own triage controls (triage-from-page, issue #33)
//! both end at this handler and the same `scheduler_core`/`store` calls, so a
//! task created either way is the same row. Content type is the only thing
//! that differs — a form post (the page) gets back the `#lists` fragment to
//! swap in, carrying a rejection message on the failing row when triage was
//! refused; a JSON request (the existing API) gets back exactly what it
//! always has, unchanged.

use crate::inbox;
use crate::inbox::CAPTURE_NOT_OPEN_MESSAGE;
use crate::platform::clock::Clock;
use crate::platform::request::content_type_is_json;
use crate::platform::response::write_failed;
use crate::triage::store;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use scheduler_core::task::{TaskKind, TriageFields, TriageRejection, WellFormedTriage};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn string_field(payload: &Value, name: &str) -> Option<String> {
    payload
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Translates the JSON body into the core's triage input. This function is
/// the whole of the JSON transport's involvement: nothing it calls sees
/// `serde_json`.
fn triage_fields(payload: &Value) -> TriageFields {
    TriageFields {
        kind: string_field(payload, "kind"),
        deadline: string_field(payload, "deadline"),
        deadline_type: string_field(payload, "deadline_type"),
        priority: string_field(payload, "priority"),
        estimated_minutes: payload.get("estimated_minutes").and_then(Value::as_i64),
        target_count: payload.get("target_count").and_then(Value::as_i64),
        target_minutes_each: payload.get("target_minutes_each").and_then(Value::as_i64),
        period: string_field(payload, "period"),
        life_area: string_field(payload, "life_area"),
        context_tag: string_field(payload, "context_tag"),
    }
}

/// The page's triage controls submit as a plain HTML form: url-encoded,
/// every field optional, since which fields matter depends on `kind`. Mirrors
/// `scheduler_core::task::TriageFields` rather than being that type directly
/// — the core must not depend on `serde` (T-module-boundary).
#[derive(Deserialize, Default)]
pub struct TriageFormRequest {
    kind: Option<String>,
    deadline: Option<String>,
    deadline_type: Option<String>,
    priority: Option<String>,
    estimated_minutes: Option<i64>,
    target_count: Option<i64>,
    target_minutes_each: Option<i64>,
    period: Option<String>,
    life_area: Option<String>,
    context_tag: Option<String>,
}

impl From<TriageFormRequest> for TriageFields {
    fn from(form: TriageFormRequest) -> Self {
        TriageFields {
            kind: form.kind,
            deadline: form.deadline,
            deadline_type: form.deadline_type,
            priority: form.priority,
            estimated_minutes: form.estimated_minutes,
            target_count: form.target_count,
            target_minutes_each: form.target_minutes_each,
            period: form.period,
            life_area: form.life_area,
            context_tag: form.context_tag,
        }
    }
}

/// Either transport this endpoint accepts. JSON keeps the raw `Value`: a
/// wrong-typed `kind` (e.g. `7`) must still be echoed back by the rejection
/// (T-unknown-kind-rejected), which a strongly-typed `Option<String>` field
/// cannot represent — it would fail to deserialize at all. A form field has
/// no such case; every value a browser form submits is already a string.
pub enum TriageInput {
    Json(Value),
    Form(TriageFormRequest),
}

impl<S: Send + Sync> FromRequest<S> for TriageInput {
    type Rejection = StatusCode;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if content_type_is_json(&req) {
            let Json(payload) = Json::<Value>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(Self::Json(payload))
        } else {
            let Form(form) = Form::<TriageFormRequest>::from_request(req, state)
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok(Self::Form(form))
        }
    }
}

/// Every way triage can be refused. `Core` wraps `scheduler_core`'s own
/// rejection — well-formedness, decidable without a database. `UnknownLifeArea`
/// is the one reason that needs the database to detect: it asks whether a
/// submitted name currently resolves to a real, active row, which is an
/// adapter question, not `scheduler_core`'s to answer
/// (`T-capability-owns-its-queries`).
enum Rejection {
    Core(TriageRejection),
    UnknownLifeArea(String),
    /// The capture named by the URL has already left the inbox -- by triage
    /// or by dismissal, either of which needs the database to know
    /// (`dismiss-capture-no-triage-after-dismissal-05`). Which of the two it
    /// was is not reported: the rejection is the same either way, "not open
    /// for business", and nothing downstream needs to tell them apart.
    CaptureNotOpen,
}

/// The rejection contract: a client error whose body names what was wrong.
/// A rejection that does not say what was wrong is a failure even with the
/// right status code.
///
/// `kind_submitted` comes from the request rather than from the rejection:
/// the core sees `kind` only after `string_field` has turned a wrong-typed
/// value into `None`, so this module holds the only surviving copy of what
/// actually arrived. That is what lets `{"kind": 7}` echo back `7` instead
/// of `null` (T-unknown-kind-rejected: report what was submitted).
fn rejected(rejection: &Rejection, kind_submitted: &Value) -> (StatusCode, Json<Value>) {
    let body = match rejection {
        Rejection::Core(TriageRejection::MissingField(field)) => {
            json!({ "missing_field": field.name() })
        }
        Rejection::Core(TriageRejection::InvalidField(field)) => {
            json!({ "invalid_field": field.name() })
        }
        Rejection::Core(TriageRejection::UnknownKind) => json!({ "unknown_kind": kind_submitted }),
        Rejection::UnknownLifeArea(name) => json!({ "unknown_life_area": name }),
        Rejection::CaptureNotOpen => {
            json!({ "capture_not_open": CAPTURE_NOT_OPEN_MESSAGE })
        }
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
}

/// A one-line summary of a rejection for the page's per-row error slot. Not
/// the API's rejection contract (that stays `rejected`'s job) — this is
/// prose for a human reading the form they just submitted.
fn rejection_message(rejection: &Rejection, kind_submitted: &Value) -> String {
    match rejection {
        Rejection::Core(TriageRejection::MissingField(field)) => {
            format!("{} is required", field.name())
        }
        Rejection::Core(TriageRejection::InvalidField(field)) => {
            format!("{} is invalid", field.name())
        }
        Rejection::Core(TriageRejection::UnknownKind) => {
            format!("unrecognised kind: {kind_submitted}")
        }
        Rejection::UnknownLifeArea(name) => format!("{name} is not a life area"),
        Rejection::CaptureNotOpen => CAPTURE_NOT_OPEN_MESSAGE.to_string(),
    }
}

/// The instant is passed in rather than read here: reading the clock is the
/// handler's business, and a triage stamps the task and the capture it
/// consumed with the same one.
///
/// Three writes, two of them not triage's own: the task itself, then the
/// inbox is asked to close the capture it consumed
/// (`T-one-front-door-per-capability` — triage does not know that leaving
/// the inbox is a `left_inbox_at` stamp), then — only when a tag was
/// submitted — capture is asked to set or change it
/// (`context-tags-taggable-at-triage-07`), through
/// the same [`crate::capture::retag`] a fresh capture's own tag goes
/// through, so the two paths cannot canonicalize case identity differently.
async fn write_task(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    life_area_id: Option<i64>,
    context_tag: Option<&str>,
    created_at_ms: i64,
) -> Result<(), StatusCode> {
    store::insert_task(pool, capture_id, kind, life_area_id, created_at_ms)
        .await
        .map_err(write_failed)?;
    inbox::close_capture(pool, capture_id, created_at_ms)
        .await
        .map_err(write_failed)?;
    crate::capture::retag(pool, capture_id, context_tag)
        .await
        .map_err(write_failed)?;
    Ok(())
}

/// What a validated submission is ready to write, or why it is not.
enum TriageOutcome {
    Accepted {
        kind: TaskKind,
        life_area_id: Option<i64>,
        context_tag: Option<String>,
    },
    Rejected(Rejection),
}

/// Only once the core calls a submission well-formed does the database enter
/// it: first, whether the capture named by the URL is still open
/// (`dismiss-capture-no-second-triage-07`, `-no-triage-after-dismissal-05`) —
/// cheaper than resolving a life area, and logically prior, since a
/// submission naming a life area that is fine in the abstract still cannot
/// land on a capture that has already left. Then, only when a life area was
/// actually named (`T-life-area-required-at-triage` superseded by
/// `D-context-tags-are-the-taxonomy`), whether it resolves to a real, active
/// row — a submission naming none skips this lookup outright and is accepted
/// with no life area.
async fn decide_triage(
    pool: &SqlitePool,
    capture_id: i64,
    fields: &TriageFields,
) -> Result<TriageOutcome, StatusCode> {
    let submission = match WellFormedTriage::from_fields(fields) {
        Ok(submission) => submission,
        Err(rejection) => return Ok(TriageOutcome::Rejected(Rejection::Core(rejection))),
    };
    if !inbox::capture_is_open(pool, capture_id)
        .await
        .map_err(write_failed)?
    {
        return Ok(TriageOutcome::Rejected(Rejection::CaptureNotOpen));
    }
    let Some(name) = submission.life_area_name.clone() else {
        return Ok(TriageOutcome::Accepted {
            kind: submission.kind,
            life_area_id: None,
            context_tag: submission.context_tag,
        });
    };
    let life_area_id = crate::life_areas::active_id_for_name(pool, &name)
        .await
        .map_err(write_failed)?;
    Ok(outcome_for_resolved_life_area(
        submission,
        name,
        life_area_id,
    ))
}

/// Once a named life area has been looked up, whether it resolved decides
/// the rest of the outcome: an id accepts the submission, its absence
/// rejects it by the name that failed to resolve.
fn outcome_for_resolved_life_area(
    submission: WellFormedTriage,
    name: String,
    life_area_id: Option<i64>,
) -> TriageOutcome {
    match life_area_id {
        Some(life_area_id) => TriageOutcome::Accepted {
            kind: submission.kind,
            life_area_id: Some(life_area_id),
            context_tag: submission.context_tag,
        },
        None => TriageOutcome::Rejected(Rejection::UnknownLifeArea(name)),
    }
}

/// The page-originated response: whatever happened, re-render `#lists` from
/// current state. On success that reflects the write; on rejection nothing
/// changed, but the failing capture's row carries the rejection message.
async fn page_response(
    pool: &SqlitePool,
    capture_id: i64,
    outcome: &TriageOutcome,
    kind_submitted: &Value,
    created_at_ms: i64,
) -> Result<Response, StatusCode> {
    let (status, error) = match outcome {
        TriageOutcome::Accepted { .. } => (StatusCode::CREATED, None),
        TriageOutcome::Rejected(rejection) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((capture_id, rejection_message(rejection, kind_submitted))),
        ),
    };
    if let TriageOutcome::Accepted {
        kind,
        life_area_id,
        context_tag,
    } = outcome
    {
        write_task(
            pool,
            capture_id,
            kind,
            *life_area_id,
            context_tag.as_deref(),
            created_at_ms,
        )
        .await?;
    }
    inbox::render_lists(pool, status, error).await
}

/// The two things every branch of [`TriageInput`] must produce: the fields
/// the core decides on, and the raw `kind` a rejection echoes back
/// (T-unknown-kind-rejected).
fn fields_from_input(input: TriageInput) -> (TriageFields, Value) {
    match input {
        TriageInput::Json(payload) => {
            let kind_submitted = payload.get("kind").cloned().unwrap_or(Value::Null);
            (triage_fields(&payload), kind_submitted)
        }
        TriageInput::Form(form) => {
            let kind_submitted = json!(form.kind);
            (form.into(), kind_submitted)
        }
    }
}

pub async fn create_triage(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(capture_id): Path<i64>,
    input: TriageInput,
) -> Result<Response, StatusCode> {
    let from_page = matches!(input, TriageInput::Form(_));
    let (fields, kind_submitted) = fields_from_input(input);
    let created_at_ms = clock.now_ms();

    let outcome = decide_triage(&pool, capture_id, &fields).await?;

    if from_page {
        return page_response(&pool, capture_id, &outcome, &kind_submitted, created_at_ms).await;
    }

    match outcome {
        TriageOutcome::Accepted {
            kind,
            life_area_id,
            context_tag,
        } => {
            write_task(
                &pool,
                capture_id,
                &kind,
                life_area_id,
                context_tag.as_deref(),
                created_at_ms,
            )
            .await?;
            Ok((StatusCode::CREATED, Json(json!({}))).into_response())
        }
        TriageOutcome::Rejected(rejection) => {
            Ok(rejected(&rejection, &kind_submitted).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;
    use axum::body::Body;
    use axum::http::Request;
    use proptest::prelude::*;
    use scheduler_core::task::Field;
    use tower::ServiceExt;

    async fn insert_untriaged_capture(pool: &SqlitePool, raw_text: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
        )
        .bind(raw_text)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn triage_response(
        pool: &SqlitePool,
        capture_id: i64,
        body: Value,
    ) -> axum::response::Response {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/triage"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// The page-originated (form) transport, as opposed to `triage_response`'s
    /// JSON. Exercises `content_type_is_json`, `TriageFormRequest::into`,
    /// `page_response` and `rejection_message` together, none of which any
    /// JSON-only test can reach.
    async fn page_triage_response(
        pool: &SqlitePool,
        capture_id: i64,
        fields: &[(&str, &str)],
    ) -> axum::response::Response {
        let body = fields
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("&");
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/triage"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn response_html(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn committed_payload_missing(field: &str) -> Value {
        let mut payload = json!({
            "kind": "committed",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": "P1",
            "estimated_minutes": 180
        });
        payload.as_object_mut().unwrap().remove(field);
        payload
    }

    #[tokio::test]
    async fn triaging_as_pool_creates_a_pool_task_with_no_deadline_or_quota_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Work" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let row: (String, Option<i64>, Option<i64>, Option<i64>, Option<String>) =
            sqlx::query_as(
                "SELECT kind, deadline, target_count, target_minutes_each, period FROM tasks WHERE capture_id = ?",
            )
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "pool");
        assert_eq!(row.1, None, "pool task must have no deadline");
        assert_eq!(row.2, None, "pool task must have no quota target");
        assert_eq!(row.3, None, "pool task must have no quota target");
        assert_eq!(row.4, None, "pool task must have no quota target");
    }

    /// `T-life-area-required-at-triage` is superseded by
    /// `D-context-tags-are-the-taxonomy` (#82): all three kinds succeed with
    /// no life area, over the JSON transport.
    #[tokio::test]
    async fn triaging_with_no_life_area_succeeds_for_every_kind() {
        for body in [
            json!({ "kind": "pool" }),
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
            json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "week",
            }),
        ] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

            let response = triage_response(&pool, capture_id, body.clone()).await;

            assert_eq!(
                response.status(),
                StatusCode::CREATED,
                "expected {body} to succeed with no life area"
            );
            let life_area_id: Option<i64> =
                sqlx::query_scalar("SELECT life_area_id FROM tasks WHERE capture_id = ?")
                    .bind(capture_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(life_area_id, None, "expected no life area for {body}");
        }
    }

    /// An empty or whitespace-only `life_area` is the same as omitting it
    /// (`T-empty-equals-absent`), not a rejection.
    #[tokio::test]
    async fn triaging_with_a_blank_life_area_succeeds_with_no_life_area() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "   " }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let life_area_id: Option<i64> =
            sqlx::query_scalar("SELECT life_area_id FROM tasks WHERE capture_id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(life_area_id, None);
    }

    /// Supplying a life area still works -- only the requirement dropped.
    #[tokio::test]
    async fn triaging_with_no_life_area_through_the_page_succeeds() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = page_triage_response(&pool, capture_id, &[("kind", "pool")]).await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    /// Triage's own tag field is a second chance to tag, alongside the
    /// capture (`context-tags-taggable-at-triage-07`) -- both write the
    /// same field.
    #[tokio::test]
    async fn triaging_with_a_context_tag_stores_it_on_the_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy screws").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "context_tag": "@homedepot" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    /// A triage submitting no tag must not erase one the capture already
    /// carries -- triage's own field is optional, not a reset.
    #[tokio::test]
    async fn triaging_with_no_context_tag_leaves_an_existing_one_untouched() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy screws").await;
        crate::capture::store::set_context_tag(&pool, capture_id, "@homedepot")
            .await
            .unwrap();

        triage_response(&pool, capture_id, json!({ "kind": "pool" })).await;

        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn triaging_as_committed_records_deadline_deadline_type_priority_and_estimate() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 180,
                "life_area": "Work"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        type CommittedRow = (
            String,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
        );
        let row: CommittedRow = sqlx::query_as(
            "SELECT kind, deadline, deadline_type, priority, estimated_minutes, target_count \
             FROM tasks WHERE capture_id = ?",
        )
        .bind(capture_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "committed");
        assert_eq!(row.1, Some(1787245200000));
        assert_eq!(row.2.as_deref(), Some("hard"));
        assert_eq!(row.3.as_deref(), Some("P1"));
        assert_eq!(row.4, Some(180));
        assert_eq!(row.5, None, "committed task must have no quota target");
    }

    #[tokio::test]
    async fn triaging_as_quota_records_the_recurring_target_and_leaves_deadline_empty() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "week",
                "life_area": "Work"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let row: (String, Option<i64>, Option<i64>, Option<String>, Option<String>) =
            sqlx::query_as(
                "SELECT kind, target_count, target_minutes_each, period, deadline FROM tasks WHERE capture_id = ?",
            )
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "quota");
        assert_eq!(row.1, Some(3));
        assert_eq!(row.2, Some(45));
        assert_eq!(row.3.as_deref(), Some("week"));
        assert_eq!(row.4, None, "quota task must have no deadline");
    }

    #[tokio::test]
    async fn triaging_an_already_triaged_capture_is_rejected_and_leaves_one_task() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;
        triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Work" }),
        )
        .await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Home" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "capture_not_open": "the capture is no longer in the inbox" })
        );
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 1, "the second triage must not add a task");
    }

    #[tokio::test]
    async fn triaging_a_capture_that_does_not_exist_is_rejected_as_not_open() {
        let (_dir, pool) = test_pool().await;

        let response =
            triage_response(&pool, 999, json!({ "kind": "pool", "life_area": "Work" })).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "capture_not_open": "the capture is no longer in the inbox" })
        );
    }

    #[tokio::test]
    async fn an_accepted_triage_stamps_the_capture_as_triaged() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Work" }),
        )
        .await;

        let left_inbox_at: Option<i64> =
            sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            left_inbox_at.is_some(),
            "an accepted triage consumes the capture"
        );
    }

    #[tokio::test]
    async fn triaging_as_committed_without_a_required_field_is_rejected_and_creates_nothing() {
        for field in ["deadline", "deadline_type", "priority", "estimated_minutes"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            let response =
                triage_response(&pool, capture_id, committed_payload_missing(field)).await;

            assert_eq!(
                response.status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "missing {field} should be rejected"
            );
            let body = response_json(response).await;
            assert_eq!(
                body.get("missing_field").and_then(Value::as_str),
                Some(field)
            );

            let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(
                task_count, 0,
                "a rejected triage must not leave a task behind"
            );

            let left_inbox_at: Option<i64> =
                sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
                    .bind(capture_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(
                left_inbox_at, None,
                "a rejected triage must not consume the capture"
            );
        }
    }

    #[tokio::test]
    async fn triaging_as_committed_with_a_required_field_left_empty_is_rejected_the_same_as_absent()
    {
        for field in ["deadline", "deadline_type", "priority", "estimated_minutes"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            let mut payload = json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 180,
            });
            payload[field] = json!("");

            let response = triage_response(&pool, capture_id, payload).await;

            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = response_json(response).await;
            assert_eq!(
                body.get("missing_field").and_then(Value::as_str),
                Some(field),
                "an empty {field} should be reported the same as an absent one"
            );
        }
    }

    #[tokio::test]
    async fn triaging_as_committed_with_an_unparseable_deadline_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "banana",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "invalid_field": "deadline" })
        );
    }

    #[tokio::test]
    async fn triaging_as_committed_with_an_invalid_deadline_type_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "squishy",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "invalid_field": "deadline_type" })
        );
    }

    #[tokio::test]
    async fn triaging_as_committed_with_an_invalid_priority_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P9",
                "estimated_minutes": 180,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "invalid_field": "priority" })
        );
    }

    #[tokio::test]
    async fn triaging_as_committed_with_a_zero_estimate_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 0,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "invalid_field": "estimated_minutes" })
        );
    }

    #[tokio::test]
    async fn triaging_as_pool_does_not_require_an_estimate() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Work" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn triaging_as_quota_without_a_required_target_field_is_rejected() {
        for field in ["target_count", "target_minutes_each", "period"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

            let mut payload = json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "week",
            });
            payload.as_object_mut().unwrap().remove(field);

            let response = triage_response(&pool, capture_id, payload).await;

            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let body = response_json(response).await;
            assert_eq!(
                body.get("missing_field").and_then(Value::as_str),
                Some(field)
            );

            let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(task_count, 0);
        }
    }

    #[tokio::test]
    async fn triaging_as_quota_with_period_left_empty_is_rejected_the_same_as_absent() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "",
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "missing_field": "period" })
        );
    }

    #[tokio::test]
    async fn triaging_as_quota_with_an_invalid_period_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "fortnight",
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "invalid_field": "period" })
        );
    }

    #[tokio::test]
    async fn triaging_as_a_kind_that_is_not_one_of_the_three_is_rejected_naming_it() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(&pool, capture_id, json!({ "kind": "someday" })).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "unknown_kind": "someday" })
        );

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 0);
    }

    #[tokio::test]
    async fn triaging_without_naming_a_kind_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(&pool, capture_id, json!({})).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(response).await,
            json!({ "unknown_kind": Value::Null })
        );
    }

    /// Folded in from the PR #31 review: `string_field` drops a wrong-typed
    /// `kind` before the core ever sees it, so the rejection used to report
    /// `{"unknown_kind": null}` for `{"kind": 7}` — losing exactly the value
    /// T-unknown-kind-rejected says the caller should see echoed back.
    #[tokio::test]
    async fn triaging_with_kind_submitted_as_the_wrong_json_type_echoes_what_was_submitted() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(&pool, capture_id, json!({ "kind": 7 })).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response_json(response).await, json!({ "unknown_kind": 7 }));
    }

    #[tokio::test]
    async fn triaging_as_pool_through_the_page_creates_the_task_and_carries_no_error() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = page_triage_response(
            &pool,
            capture_id,
            &[("kind", "pool"), ("life_area", "Work")],
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let html = response_html(response).await;
        assert!(!html.contains("is required"));

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 1);
    }

    #[tokio::test]
    async fn triaging_as_committed_through_the_page_without_a_required_field_shows_the_rejection_on_the_row(
    ) {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = page_triage_response(
            &pool,
            capture_id,
            &[
                ("kind", "committed"),
                ("deadline_type", "hard"),
                ("priority", "P1"),
            ],
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = response_html(response).await;
        assert!(html.contains("deadline is required"));

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 0);
    }

    #[test]
    fn triage_fields_reads_every_field_the_core_asks_for() {
        let payload = json!({
            "kind": "quota",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": "P1",
            "estimated_minutes": 180,
            "target_count": 3,
            "target_minutes_each": 45,
            "period": "week",
            "life_area": "Work"
        });

        assert_eq!(
            triage_fields(&payload),
            TriageFields {
                kind: Some("quota".to_string()),
                deadline: Some("2026-08-20T17:00:00Z".to_string()),
                deadline_type: Some("hard".to_string()),
                priority: Some("P1".to_string()),
                estimated_minutes: Some(180),
                target_count: Some(3),
                target_minutes_each: Some(45),
                period: Some("week".to_string()),
                life_area: Some("Work".to_string()),
                context_tag: None,
            }
        );
    }

    #[test]
    fn triage_fields_treats_an_empty_body_as_nothing_submitted() {
        assert_eq!(triage_fields(&json!({})), TriageFields::default());
    }

    #[test]
    fn triage_fields_ignores_a_field_submitted_with_the_wrong_json_type() {
        let payload = json!({ "kind": 7, "target_count": "three" });
        assert_eq!(triage_fields(&payload), TriageFields::default());
    }

    #[test]
    fn a_missing_field_rejection_names_the_field() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::MissingField(Field::Priority)),
            &Value::Null,
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "missing_field": "priority" }));
    }

    #[test]
    fn an_invalid_field_rejection_names_the_field() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::InvalidField(Field::Deadline)),
            &Value::Null,
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "invalid_field": "deadline" }));
    }

    #[test]
    fn an_unknown_kind_rejection_echoes_the_value_it_is_given() {
        let (status, Json(body)) = rejected(
            &Rejection::Core(TriageRejection::UnknownKind),
            &json!({ "not": "a string" }),
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "unknown_kind": { "not": "a string" } }));
    }

    #[test]
    fn an_unknown_life_area_rejection_echoes_the_submitted_name() {
        let (status, Json(body)) = rejected(
            &Rejection::UnknownLifeArea("Gardening".to_string()),
            &Value::Null,
        );
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "unknown_life_area": "Gardening" }));
    }

    #[test]
    fn an_unknown_life_area_rejection_message_names_it() {
        let message = rejection_message(
            &Rejection::UnknownLifeArea("Gardening".to_string()),
            &Value::Null,
        );
        assert_eq!(message, "Gardening is not a life area");
    }

    #[test]
    fn an_invalid_field_rejection_message_names_the_field() {
        let message = rejection_message(
            &Rejection::Core(TriageRejection::InvalidField(Field::Deadline)),
            &Value::Null,
        );
        assert_eq!(message, "deadline is invalid");
    }

    #[test]
    fn an_unknown_kind_rejection_message_echoes_the_value_it_is_given() {
        let message = rejection_message(
            &Rejection::Core(TriageRejection::UnknownKind),
            &json!({ "not": "a string" }),
        );
        assert_eq!(message, r#"unrecognised kind: {"not":"a string"}"#);
    }

    #[test]
    fn outcome_for_resolved_life_area_rejects_a_name_that_did_not_resolve() {
        let submission = WellFormedTriage {
            kind: TaskKind::Pool,
            life_area_name: Some("Gardening".to_string()),
            context_tag: None,
        };
        let outcome = outcome_for_resolved_life_area(submission, "Gardening".to_string(), None);
        match outcome {
            TriageOutcome::Rejected(Rejection::UnknownLifeArea(name)) => {
                assert_eq!(name, "Gardening")
            }
            _ => panic!("expected an UnknownLifeArea rejection"),
        }
    }

    /// A submission expressed as JSON and as a form, field for field.
    fn json_body(fields: &TriageFields) -> Value {
        let mut body = serde_json::Map::new();
        let mut put = |name: &str, value: Option<Value>| {
            if let Some(value) = value {
                body.insert(name.to_string(), value);
            }
        };
        put("kind", fields.kind.clone().map(Value::from));
        put("deadline", fields.deadline.clone().map(Value::from));
        put(
            "deadline_type",
            fields.deadline_type.clone().map(Value::from),
        );
        put("priority", fields.priority.clone().map(Value::from));
        put(
            "estimated_minutes",
            fields.estimated_minutes.map(Value::from),
        );
        put("target_count", fields.target_count.map(Value::from));
        put(
            "target_minutes_each",
            fields.target_minutes_each.map(Value::from),
        );
        put("period", fields.period.clone().map(Value::from));
        put("life_area", fields.life_area.clone().map(Value::from));
        put("context_tag", fields.context_tag.clone().map(Value::from));
        Value::Object(body)
    }

    fn form_request(fields: &TriageFields) -> TriageFormRequest {
        TriageFormRequest {
            kind: fields.kind.clone(),
            deadline: fields.deadline.clone(),
            deadline_type: fields.deadline_type.clone(),
            priority: fields.priority.clone(),
            estimated_minutes: fields.estimated_minutes,
            target_count: fields.target_count,
            target_minutes_each: fields.target_minutes_each,
            period: fields.period.clone(),
            life_area: fields.life_area.clone(),
            context_tag: fields.context_tag.clone(),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

        /// "The page and `POST /captures/{id}/triage` are one code path, not
        /// two" is this module's stated design, and this is the seam where it
        /// could quietly stop being true: two hand-written, field-by-field
        /// translations into the same core input. Adding a field to
        /// `TriageFields` and wiring it into only one of them still compiles,
        /// still passes every example test that does not happen to use it,
        /// and silently makes the page and the API disagree. It fails here.
        #[test]
        #[ignore]
        fn both_transports_translate_one_submission_into_the_same_core_input(
            kind in proptest::option::of(".{0,12}"),
            deadline in proptest::option::of(".{0,30}"),
            deadline_type in proptest::option::of(".{0,10}"),
            priority in proptest::option::of(".{0,6}"),
            estimated_minutes in proptest::option::of(any::<i64>()),
            target_count in proptest::option::of(any::<i64>()),
            target_minutes_each in proptest::option::of(any::<i64>()),
            period in proptest::option::of(".{0,10}"),
            life_area in proptest::option::of(".{0,20}"),
            context_tag in proptest::option::of(".{0,20}"),
        ) {
            let submission = TriageFields {
                kind,
                deadline,
                deadline_type,
                priority,
                estimated_minutes,
                target_count,
                target_minutes_each,
                period,
                life_area,
                context_tag,
            };

            let from_json = triage_fields(&json_body(&submission));
            let from_form: TriageFields = form_request(&submission).into();

            prop_assert_eq!(&from_json, &submission);
            prop_assert_eq!(from_json, from_form);
        }
    }
}
