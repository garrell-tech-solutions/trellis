//! `POST /captures/{id}/triage`.
//!
//! One code path for both callers, the same shape as `http::capture`: the
//! JSON API and the page's own triage controls (triage-from-page, issue #33)
//! both end at this handler and the same `scheduler_core`/`store` calls, so a
//! task created either way is the same row. Content type is the only thing
//! that differs — a form post (the page) gets back the `#lists` fragment to
//! swap in, carrying a rejection message on the failing row when triage was
//! refused; a JSON request (the existing API) gets back exactly what it
//! always has, unchanged.

use crate::clock::now_ms;
use crate::http::lists::{build_lists, ListsTemplate};
use crate::http::{render_template, write_failed};
use crate::store;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use scheduler_core::task::{TaskKind, TriageFields, TriageRejection};
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
        target_count: payload.get("target_count").and_then(Value::as_i64),
        target_minutes_each: payload.get("target_minutes_each").and_then(Value::as_i64),
        period: string_field(payload, "period"),
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
    target_count: Option<i64>,
    target_minutes_each: Option<i64>,
    period: Option<String>,
}

impl From<TriageFormRequest> for TriageFields {
    fn from(form: TriageFormRequest) -> Self {
        TriageFields {
            kind: form.kind,
            deadline: form.deadline,
            deadline_type: form.deadline_type,
            priority: form.priority,
            target_count: form.target_count,
            target_minutes_each: form.target_minutes_each,
            period: form.period,
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

fn content_type_is_json(req: &Request) -> bool {
    req.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"))
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
fn rejected(rejection: TriageRejection, kind_submitted: &Value) -> (StatusCode, Json<Value>) {
    let body = match rejection {
        TriageRejection::MissingField(field) => json!({ "missing_field": field.name() }),
        TriageRejection::InvalidField(field) => json!({ "invalid_field": field.name() }),
        TriageRejection::UnknownKind => json!({ "unknown_kind": kind_submitted }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
}

/// A one-line summary of a rejection for the page's per-row error slot. Not
/// the API's rejection contract (that stays `rejected`'s job) — this is
/// prose for a human reading the form they just submitted.
fn rejection_message(rejection: &TriageRejection, kind_submitted: &Value) -> String {
    match rejection {
        TriageRejection::MissingField(field) => format!("{} is required", field.name()),
        TriageRejection::InvalidField(field) => format!("{} is invalid", field.name()),
        TriageRejection::UnknownKind => format!("unrecognised kind: {kind_submitted}"),
    }
}

async fn write_task(pool: &SqlitePool, capture_id: i64, kind: &TaskKind) -> Result<(), StatusCode> {
    let created_at_ms = now_ms();
    store::task::insert(pool, capture_id, kind, created_at_ms)
        .await
        .map_err(write_failed)?;
    store::capture::mark_triaged(pool, capture_id, created_at_ms)
        .await
        .map_err(write_failed)?;
    Ok(())
}

/// The page-originated response: whatever happened, re-render `#lists` from
/// current state. On success that reflects the write; on rejection nothing
/// changed, but the failing capture's row carries the rejection message.
async fn page_response(
    pool: &SqlitePool,
    capture_id: i64,
    outcome: Result<TaskKind, TriageRejection>,
    kind_submitted: &Value,
) -> Result<Response, StatusCode> {
    let (status, error) = match &outcome {
        Ok(_) => (StatusCode::CREATED, None),
        Err(rejection) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some((capture_id, rejection_message(rejection, kind_submitted))),
        ),
    };
    if let Ok(kind) = &outcome {
        write_task(pool, capture_id, kind).await?;
    }
    let (captures, tasks) = build_lists(pool, error).await.map_err(write_failed)?;
    Ok(render_template(status, &ListsTemplate { captures, tasks }))
}

pub async fn create_triage(
    State(pool): State<SqlitePool>,
    Path(capture_id): Path<i64>,
    input: TriageInput,
) -> Result<Response, StatusCode> {
    let from_page = matches!(input, TriageInput::Form(_));
    let (fields, kind_submitted): (TriageFields, Value) = match input {
        TriageInput::Json(payload) => {
            let kind_submitted = payload.get("kind").cloned().unwrap_or(Value::Null);
            (triage_fields(&payload), kind_submitted)
        }
        TriageInput::Form(form) => {
            let kind_submitted = json!(form.kind);
            (form.into(), kind_submitted)
        }
    };

    let outcome = TaskKind::from_fields(&fields);

    if from_page {
        return page_response(&pool, capture_id, outcome, &kind_submitted).await;
    }

    match outcome {
        Ok(kind) => {
            write_task(&pool, capture_id, &kind).await?;
            Ok((StatusCode::CREATED, Json(json!({}))).into_response())
        }
        Err(rejection) => Ok(rejected(rejection, &kind_submitted).into_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
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
        let app = crate::app::build_app(pool.clone());
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

    fn committed_payload_missing(field: &str) -> Value {
        let mut payload = json!({
            "kind": "committed",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": "P1"
        });
        payload.as_object_mut().unwrap().remove(field);
        payload
    }

    #[tokio::test]
    async fn triaging_as_pool_creates_a_pool_task_with_no_deadline_or_quota_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(&pool, capture_id, json!({ "kind": "pool" })).await;

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

    #[tokio::test]
    async fn triaging_as_committed_records_deadline_deadline_type_and_priority() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let row: (String, Option<i64>, Option<String>, Option<String>, Option<i64>) =
            sqlx::query_as(
                "SELECT kind, deadline, deadline_type, priority, target_count FROM tasks WHERE capture_id = ?",
            )
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "committed");
        assert_eq!(row.1, Some(1787245200000));
        assert_eq!(row.2.as_deref(), Some("hard"));
        assert_eq!(row.3.as_deref(), Some("P1"));
        assert_eq!(row.4, None, "committed task must have no quota target");
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
                "period": "week"
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
    async fn an_accepted_triage_stamps_the_capture_as_triaged() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        triage_response(&pool, capture_id, json!({ "kind": "pool" })).await;

        let triaged_at: Option<i64> =
            sqlx::query_scalar("SELECT triaged_at FROM captures WHERE id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            triaged_at.is_some(),
            "an accepted triage consumes the capture"
        );
    }

    #[tokio::test]
    async fn triaging_as_committed_without_a_required_field_is_rejected_and_creates_nothing() {
        for field in ["deadline", "deadline_type", "priority"] {
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

            let triaged_at: Option<i64> =
                sqlx::query_scalar("SELECT triaged_at FROM captures WHERE id = ?")
                    .bind(capture_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(
                triaged_at, None,
                "a rejected triage must not consume the capture"
            );
        }
    }

    #[tokio::test]
    async fn triaging_as_committed_with_a_required_field_left_empty_is_rejected_the_same_as_absent()
    {
        for field in ["deadline", "deadline_type", "priority"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            let mut payload = json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
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

    #[test]
    fn triage_fields_reads_every_field_the_core_asks_for() {
        let payload = json!({
            "kind": "quota",
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": "P1",
            "target_count": 3,
            "target_minutes_each": 45,
            "period": "week"
        });

        assert_eq!(
            triage_fields(&payload),
            TriageFields {
                kind: Some("quota".to_string()),
                deadline: Some("2026-08-20T17:00:00Z".to_string()),
                deadline_type: Some("hard".to_string()),
                priority: Some("P1".to_string()),
                target_count: Some(3),
                target_minutes_each: Some(45),
                period: Some("week".to_string()),
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
        let (status, Json(body)) =
            rejected(TriageRejection::MissingField(Field::Priority), &Value::Null);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "missing_field": "priority" }));
    }

    #[test]
    fn an_invalid_field_rejection_names_the_field() {
        let (status, Json(body)) =
            rejected(TriageRejection::InvalidField(Field::Deadline), &Value::Null);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "invalid_field": "deadline" }));
    }

    #[test]
    fn an_unknown_kind_rejection_echoes_the_value_it_is_given() {
        let (status, Json(body)) =
            rejected(TriageRejection::UnknownKind, &json!({ "not": "a string" }));
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "unknown_kind": { "not": "a string" } }));
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
        put("target_count", fields.target_count.map(Value::from));
        put(
            "target_minutes_each",
            fields.target_minutes_each.map(Value::from),
        );
        put("period", fields.period.clone().map(Value::from));
        Value::Object(body)
    }

    fn form_request(fields: &TriageFields) -> TriageFormRequest {
        TriageFormRequest {
            kind: fields.kind.clone(),
            deadline: fields.deadline.clone(),
            deadline_type: fields.deadline_type.clone(),
            priority: fields.priority.clone(),
            target_count: fields.target_count,
            target_minutes_each: fields.target_minutes_each,
            period: fields.period.clone(),
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
            target_count in proptest::option::of(any::<i64>()),
            target_minutes_each in proptest::option::of(any::<i64>()),
            period in proptest::option::of(".{0,10}"),
        ) {
            let submission = TriageFields {
                kind,
                deadline,
                deadline_type,
                priority,
                target_count,
                target_minutes_each,
                period,
            };

            let from_json = triage_fields(&json_body(&submission));
            let from_form: TriageFields = form_request(&submission).into();

            prop_assert_eq!(&from_json, &submission);
            prop_assert_eq!(from_json, from_form);
        }
    }
}
