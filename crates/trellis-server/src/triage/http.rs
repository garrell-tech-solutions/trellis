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
        commitment: string_field(payload, "commitment"),
        priority: string_field(payload, "priority"),
        estimated_minutes: payload.get("estimated_minutes").and_then(Value::as_i64),
        quota_name: string_field(payload, "name"),
        quota_hours: string_field(payload, "hours"),
        // #110's date/time/zone trio is the page form's alternative to
        // `deadline`, never JSON's -- the API keeps sending one complete
        // instant, per `committed-date-json-still-takes-an-instant-05`.
        deadline_date: None,
        deadline_time: None,
        timezone: None,
    }
}

/// The page's triage controls submit as a plain HTML form: url-encoded,
/// every field optional, since which fields matter depends on `kind`. Mirrors
/// `scheduler_core::task::TriageFields` rather than being that type directly
/// — the core must not depend on `serde` (T-module-boundary).
///
/// `context_tag` rides along but is not part of `TriageFields`: it never
/// decides a kind or fails a submission (`context-tags-taggable-at-triage-08`
/// is a second chance to supply a fact the core does not adjudicate), so it
/// is extracted separately rather than smuggled into the type that exists to
/// answer "what kind of task is this".
#[derive(Deserialize, Default)]
pub struct TriageFormRequest {
    kind: Option<String>,
    deadline: Option<String>,
    /// `"2026-08-25"` -- the committed date-picker's value (#110). Absent
    /// for every other kind, and for a committed submission that still
    /// sends a complete `deadline` (an older client, or a test fixture).
    #[serde(default)]
    deadline_date: Option<String>,
    /// `"08:30"` -- the committed time-picker's value, present only for an
    /// *at*. A *by* never asks the page for one.
    #[serde(default)]
    deadline_time: Option<String>,
    commitment: Option<String>,
    priority: Option<String>,
    estimated_minutes: Option<i64>,
    /// A quota's name (#138) -- prefilled by the page with the capture's
    /// own words, but always required and editable.
    name: Option<String>,
    /// A quota's weekly hour target, as free text -- the canvas's own
    /// `step="0.5"` input.
    hours: Option<String>,
    #[serde(default)]
    context_tag: Option<String>,
    /// The candidate name a *similar*-name warning was already shown for,
    /// carried back by a "Create anyway" resubmission
    /// (`quota-triage-validation-similar-name-warns-04`) -- rides outside
    /// `TriageFields` for the same reason `context_tag` does: it decides no
    /// kind and fails no submission on its own, and the core has no
    /// database to check it against.
    #[serde(default)]
    confirmed: Option<String>,
}

impl From<&TriageFormRequest> for TriageFields {
    fn from(form: &TriageFormRequest) -> Self {
        TriageFields {
            kind: form.kind.clone(),
            deadline: form.deadline.clone(),
            deadline_date: form.deadline_date.clone(),
            deadline_time: form.deadline_time.clone(),
            // Set by `create_triage`, which has the database this
            // conversion does not (`T-core-owns-validation-order`: the
            // adapter keeps only what the core genuinely cannot have).
            timezone: None,
            commitment: form.commitment.clone(),
            priority: form.priority.clone(),
            estimated_minutes: form.estimated_minutes,
            quota_name: form.name.clone(),
            quota_hours: form.hours.clone(),
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
    Form(Box<TriageFormRequest>),
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
            Ok(Self::Form(Box::new(form)))
        }
    }
}

/// Every way triage can be refused. `Core` wraps `scheduler_core`'s own
/// rejection — well-formedness, decidable without a database.
enum Rejection {
    Core(TriageRejection),
    /// The capture named by the URL has already left the inbox -- by triage
    /// or by dismissal, either of which needs the database to know
    /// (`dismiss-capture-no-triage-after-dismissal-05`). Which of the two it
    /// was is not reported: the rejection is the same either way, "not open
    /// for business", and nothing downstream needs to tell them apart.
    CaptureNotOpen,
    /// A well-formed quota name collides with an existing quota once case,
    /// spaces and punctuation are folded away
    /// (`D-quotas-are-selected-not-typed`'s guard, moved from the retired
    /// define form to triage's own front door, #138). Never bypassable --
    /// filing into an existing quota is deferred, so the only way out is a
    /// different name.
    QuotaNameExists {
        existing_name: String,
        existing_minutes: i64,
    },
    /// A well-formed quota name merely resembles an existing one -- warned
    /// once, and created on confirmation
    /// (`quota-triage-validation-similar-name-warns-04`). The candidate
    /// name itself already decided [`check_quota_name`]'s outcome, so
    /// nothing further downstream needs to carry it.
    QuotaNameSimilar {
        existing_name: String,
        existing_minutes: i64,
    },
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
        Rejection::CaptureNotOpen => {
            json!({ "capture_not_open": CAPTURE_NOT_OPEN_MESSAGE })
        }
        Rejection::QuotaNameExists { .. } => {
            json!({ "quota_conflict": "exact", "message": rejection_message(rejection, kind_submitted) })
        }
        Rejection::QuotaNameSimilar { .. } => json!({
            "quota_conflict": "similar",
            "message": rejection_message(rejection, kind_submitted),
            "confirm_control": QUOTA_CONFIRM_CONTROL,
        }),
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
        Rejection::CaptureNotOpen => CAPTURE_NOT_OPEN_MESSAGE.to_string(),
        Rejection::QuotaNameExists {
            existing_name,
            existing_minutes,
        } => format!(
            "\u{201c}{existing_name}\u{201d} already exists at {}. Log your time against that \
             one, or give this a different name.",
            crate::quota::hours_a_week(*existing_minutes)
        ),
        Rejection::QuotaNameSimilar {
            existing_name,
            existing_minutes,
            ..
        } => format!(
            "That reads a lot like \u{201c}{existing_name}\u{201d} ({}). Same thing?",
            crate::quota::hours_a_week(*existing_minutes)
        ),
    }
}

/// The "Create anyway" control a similar-name warning offers, on both
/// transports -- the JSON contract's own `confirm_control` and the page's
/// button label alike (`quota-triage-validation-similar-name-warns-04`).
const QUOTA_CONFIRM_CONTROL: &str = "Create anyway";

/// The instant is passed in rather than read here: reading the clock is the
/// handler's business, and a triage stamps the task and the capture it
/// consumed with the same one.
///
/// Up to three writes, two of them the same for every kind: triage creates
/// the task, then asks the inbox to close the capture it consumed. Triage
/// does not know that leaving the inbox is a `left_inbox_at` stamp, which is
/// what lets dismissal reach the same state without a second copy of the
/// write (`T-one-front-door-per-capability`). A quota triage (#138) also
/// creates the `quotas` row itself -- the `tasks` row it writes carries
/// neither name nor target, so this is the one write that actually makes
/// the quota exist.
async fn write_task(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    created_at_ms: i64,
) -> Result<(), StatusCode> {
    store::insert_task(pool, capture_id, kind, created_at_ms)
        .await
        .map_err(write_failed)?;
    if let TaskKind::Quota {
        name,
        weekly_target,
    } = kind
    {
        let definition = scheduler_core::quota::QuotaDefinition {
            name: name.clone(),
            weekly_target: *weekly_target,
        };
        crate::quota::create(pool, &definition, created_at_ms)
            .await
            .map_err(write_failed)?;
    }
    inbox::close_capture(pool, capture_id, created_at_ms)
        .await
        .map_err(write_failed)?;
    Ok(())
}

/// What a validated submission is ready to write, or why it is not.
enum TriageOutcome {
    Accepted { kind: TaskKind },
    Rejected(Rejection),
}

/// Whether `confirmed` already carries this exact candidate name -- the
/// resubmission a "Create anyway" tap sends, distinguished from a first
/// attempt that has not been warned about yet
/// (`quota-triage-validation-similar-name-warns-04`).
fn already_confirmed(confirmed: Option<&str>, candidate_name: &str) -> bool {
    confirmed == Some(candidate_name)
}

/// A quota's name checked against every existing quota (#138, moved from
/// the retired quota-screen define form to triage's own front door): `None`
/// when nothing is in the way, or the write-blocking rejection otherwise.
async fn check_quota_name(
    pool: &SqlitePool,
    candidate_name: &str,
    confirmed: Option<&str>,
) -> Result<Option<Rejection>, sqlx::Error> {
    let existing = crate::quota::existing_names(pool).await?;
    let rejection = match scheduler_core::quota::check_name(candidate_name, &existing) {
        Some(scheduler_core::quota::NameMatch::Exact(existing_name, existing_minutes)) => {
            Some(Rejection::QuotaNameExists {
                existing_name: existing_name.to_string(),
                existing_minutes,
            })
        }
        Some(scheduler_core::quota::NameMatch::Similar(existing_name, existing_minutes))
            if !already_confirmed(confirmed, candidate_name) =>
        {
            Some(Rejection::QuotaNameSimilar {
                existing_name: existing_name.to_string(),
                existing_minutes,
            })
        }
        _ => None,
    };
    Ok(rejection)
}

/// Only once the core calls a submission well-formed does the database enter
/// it: whether the capture named by the URL is still open
/// (`dismiss-capture-no-second-triage-07`, `-no-triage-after-dismissal-05`),
/// and, for a quota, whether its name collides with one that already exists
/// (#138).
async fn decide_triage(
    pool: &SqlitePool,
    capture_id: i64,
    fields: &TriageFields,
    confirmed: Option<&str>,
) -> Result<TriageOutcome, StatusCode> {
    let kind = match TaskKind::from_fields(fields) {
        Ok(kind) => kind,
        Err(rejection) => return Ok(TriageOutcome::Rejected(Rejection::Core(rejection))),
    };
    if !inbox::capture_is_open(pool, capture_id)
        .await
        .map_err(write_failed)?
    {
        return Ok(TriageOutcome::Rejected(Rejection::CaptureNotOpen));
    }
    if let TaskKind::Quota { name, .. } = &kind {
        if let Some(rejection) = check_quota_name(pool, name, confirmed)
            .await
            .map_err(write_failed)?
        {
            return Ok(TriageOutcome::Rejected(rejection));
        }
    }
    Ok(TriageOutcome::Accepted { kind })
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
    if let TriageOutcome::Accepted { kind } = outcome {
        write_task(pool, capture_id, kind, created_at_ms).await?;
    }
    inbox::render_lists(pool, status, error).await
}

/// Fields both transports carry alongside [`TriageFields`] without the core
/// deciding either: `context_tag` decides no kind and fails no submission
/// (`context-tags-taggable-at-triage-08`), and `confirmed` needs a database
/// the core does not have (#138). Bundled rather than a longer tuple, since
/// [`fields_from_input`] now has three things to return besides the fields
/// themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SideFields {
    context_tag: Option<String>,
    confirmed: Option<String>,
}

/// The three things every branch of [`TriageInput`] must produce: the fields
/// the core decides on, the raw `kind` a rejection echoes back
/// (T-unknown-kind-rejected), and the side fields decided by neither
/// transport nor core.
fn fields_from_input(input: TriageInput) -> (TriageFields, Value, SideFields) {
    match input {
        TriageInput::Json(payload) => {
            let kind_submitted = payload.get("kind").cloned().unwrap_or(Value::Null);
            let side = SideFields {
                context_tag: string_field(&payload, "context_tag"),
                confirmed: string_field(&payload, "confirmed"),
            };
            (triage_fields(&payload), kind_submitted, side)
        }
        TriageInput::Form(form) => {
            let kind_submitted = json!(form.kind);
            let side = SideFields {
                context_tag: form.context_tag.clone(),
                confirmed: form.confirmed.clone(),
            };
            (TriageFields::from(form.as_ref()), kind_submitted, side)
        }
    }
}

/// The JSON-API-originated response: the existing contract, unchanged --
/// `{}` on success, `rejected`'s body on refusal.
async fn json_response(
    pool: &SqlitePool,
    capture_id: i64,
    outcome: TriageOutcome,
    kind_submitted: &Value,
    created_at_ms: i64,
) -> Result<Response, StatusCode> {
    match outcome {
        TriageOutcome::Accepted { kind } => {
            write_task(pool, capture_id, &kind, created_at_ms).await?;
            Ok((StatusCode::CREATED, Json(json!({}))).into_response())
        }
        TriageOutcome::Rejected(rejection) => {
            Ok(rejected(&rejection, kind_submitted).into_response())
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
    let (mut fields, kind_submitted, side) = fields_from_input(input);
    let created_at_ms = clock.now_ms();

    // #110: the one piece of a `deadline_date` conversion the core cannot
    // supply itself. Fetched unconditionally rather than only when
    // `deadline_date` is present -- `TaskKind::from_fields` already ignores
    // `timezone` whenever `deadline` or no date is given, so branching here
    // would just be a second copy of that same decision.
    fields.timezone = Some(
        crate::settings::current_timezone(&pool)
            .await
            .map_err(write_failed)?,
    );

    let outcome = decide_triage(&pool, capture_id, &fields, side.confirmed.as_deref()).await?;

    if let TriageOutcome::Accepted { .. } = &outcome {
        crate::capture::retag(&pool, capture_id, side.context_tag.as_deref())
            .await
            .map_err(write_failed)?;
    }

    if from_page {
        page_response(&pool, capture_id, &outcome, &kind_submitted, created_at_ms).await
    } else {
        json_response(&pool, capture_id, outcome, &kind_submitted, created_at_ms).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{stored_context_tag, test_pool};
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

    /// Submits `payload` and asserts the JSON rejection contract: 422, body
    /// exactly `expected_body`. Most of this file's rejection tests differ
    /// only in what they submit and what they expect back.
    async fn assert_json_rejected(
        pool: &SqlitePool,
        capture_id: i64,
        payload: Value,
        expected_body: Value,
    ) {
        let response = triage_response(pool, capture_id, payload).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response_json(response).await, expected_body);
    }

    /// Asserts the standard "accepted, tag written" contract: 201, and the
    /// capture's stored tag is exactly `tag`.
    async fn assert_created_with_tag(
        response: axum::response::Response,
        pool: &SqlitePool,
        capture_id: i64,
        tag: &str,
    ) {
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(
            stored_context_tag(pool, capture_id).await.as_deref(),
            Some(tag)
        );
    }

    /// Submits `payload` (missing `field`) and asserts the standard
    /// "missing required field" contract: 422, `missing_field` names it,
    /// and no task is left behind.
    async fn assert_missing_field_rejected(
        pool: &SqlitePool,
        capture_id: i64,
        payload: Value,
        field: &str,
    ) {
        let response = triage_response(pool, capture_id, payload).await;
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
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(
            task_count, 0,
            "a rejected triage must not leave a task behind"
        );
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

    /// Submits `fields` through the page transport and asserts the standard
    /// "accepted, deadline resolved" contract: 201, and the stored
    /// `tasks.deadline` is exactly `expected_ms`.
    async fn assert_stored_deadline(
        pool: &SqlitePool,
        capture_id: i64,
        fields: &[(&str, &str)],
        expected_ms: i64,
    ) {
        let response = page_triage_response(pool, capture_id, fields).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let deadline: Option<i64> = sqlx::query_scalar("SELECT deadline FROM tasks")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(deadline, Some(expected_ms));
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
            "commitment": "at",
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

    #[tokio::test]
    async fn triaging_with_a_context_tag_writes_it_onto_the_capture_not_the_task() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy screws").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "pool", "context_tag": "@homedepot" }),
        )
        .await;

        assert_created_with_tag(response, &pool, capture_id, "@homedepot").await;
    }

    #[tokio::test]
    async fn triaging_a_capture_that_already_carries_a_tag_with_no_tag_submitted_leaves_it() {
        let (_dir, pool) = test_pool().await;
        let (capture_id, _) =
            crate::capture::create(&pool, "buy screws", "web", Some("@homedepot"), 0)
                .await
                .unwrap();

        let response = triage_response(&pool, capture_id, json!({ "kind": "pool" })).await;

        assert_created_with_tag(response, &pool, capture_id, "@homedepot").await;
    }

    #[tokio::test]
    async fn a_rejected_triage_does_not_write_the_submitted_tag() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy screws").await;

        let response =
            triage_response(&pool, capture_id, json!({ "context_tag": "@homedepot" })).await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(stored_context_tag(&pool, capture_id).await, None);
    }

    #[tokio::test]
    async fn triaging_through_the_page_with_a_context_tag_writes_it() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy screws").await;

        let response = page_triage_response(
            &pool,
            capture_id,
            &[("kind", "pool"), ("context_tag", "@homedepot")],
        )
        .await;

        assert_created_with_tag(response, &pool, capture_id, "@homedepot").await;
    }

    #[tokio::test]
    async fn triaging_as_committed_records_deadline_commitment_priority_and_estimate() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "commitment": "at",
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
            "SELECT kind, deadline, commitment, priority, estimated_minutes, target_count \
             FROM tasks WHERE capture_id = ?",
        )
        .bind(capture_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "committed");
        assert_eq!(row.1, Some(1787245200000));
        assert_eq!(row.2.as_deref(), Some("at"));
        assert_eq!(row.3.as_deref(), Some("P1"));
        assert_eq!(row.4, Some(180));
        assert_eq!(row.5, None, "committed task must have no quota target");
    }

    /// #110: the page's own committed form sends `deadline_date` and
    /// `deadline_time` instead of a pre-resolved `deadline`, and the
    /// resulting instant must land in the *owner's* configured zone, not
    /// UTC -- the whole reason the near-midnight scenarios exist.
    #[tokio::test]
    async fn triaging_as_committed_through_the_page_with_a_local_date_and_time_uses_the_owner_zone()
    {
        let (_dir, pool) = test_pool().await;
        crate::settings::current_timezone(&pool).await.unwrap(); // sanity: row exists
        sqlx::query("UPDATE settings SET timezone = 'America/New_York' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        let capture_id = insert_untriaged_capture(&pool, "book the dentist").await;

        assert_stored_deadline(
            &pool,
            capture_id,
            &[
                ("kind", "committed"),
                ("deadline_date", "2026-08-25"),
                ("deadline_time", "08:30"),
                ("commitment", "at"),
                ("priority", "P1"),
                ("estimated_minutes", "30"),
            ],
            // 2026-08-25T08:30 America/New_York (EDT, UTC-4).
            1787661000000,
        )
        .await;
    }

    /// A `by` submitted through the page sends only `deadline_date`, and
    /// the resulting instant is the end of that day in the owner's zone --
    /// `committed-date-by-takes-a-day-02`.
    #[tokio::test]
    async fn triaging_as_committed_by_through_the_page_with_only_a_date_is_the_end_of_that_day() {
        let (_dir, pool) = test_pool().await;
        sqlx::query("UPDATE settings SET timezone = 'America/New_York' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();
        let capture_id = insert_untriaged_capture(&pool, "file the tax return").await;

        assert_stored_deadline(
            &pool,
            capture_id,
            &[
                ("kind", "committed"),
                ("deadline_date", "2026-08-27"),
                ("commitment", "by"),
                ("priority", "P1"),
                ("estimated_minutes", "30"),
            ],
            // End of 2026-08-27 in America/New_York.
            1787889599999,
        )
        .await;
    }

    /// #138: triaging as quota is what creates the quota. The `tasks` row
    /// it writes carries no target of its own -- the name and weekly
    /// target land in `quotas` instead, which is the fact this test pins.
    #[tokio::test]
    async fn triaging_as_quota_creates_a_quota_and_leaves_the_task_with_no_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "practise piano").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "quota", "name": "Piano", "hours": "4" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let row: (String, Option<i64>, Option<i64>) =
            sqlx::query_as("SELECT kind, target_count, deadline FROM tasks WHERE capture_id = ?")
                .bind(capture_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "quota");
        assert_eq!(row.1, None, "quota task must carry no legacy target");
        assert_eq!(row.2, None, "quota task must have no deadline");

        let quota: (String, i64) = sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(quota, ("Piano".to_string(), 240));
    }

    #[tokio::test]
    async fn triaging_as_quota_without_a_name_is_rejected_and_creates_neither_row() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

        assert_missing_field_rejected(
            &pool,
            capture_id,
            json!({ "kind": "quota", "hours": "4" }),
            "name",
        )
        .await;
        let quota_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(quota_count, 0);
    }

    #[tokio::test]
    async fn triaging_as_quota_without_hours_is_rejected() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

        assert_missing_field_rejected(
            &pool,
            capture_id,
            json!({ "kind": "quota", "name": "Gym" }),
            "hours",
        )
        .await;
    }

    #[tokio::test]
    async fn triaging_as_quota_with_zero_hours_is_rejected_as_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "go to the gym").await;

        assert_json_rejected(
            &pool,
            capture_id,
            json!({ "kind": "quota", "name": "Gym", "hours": "0" }),
            json!({ "invalid_field": "hours" }),
        )
        .await;
    }

    /// #138's own front door: a name that already exists (case, spaces and
    /// punctuation folded) is refused outright, and nothing new is
    /// written -- the existing quota is left exactly as it was.
    #[tokio::test]
    async fn triaging_as_quota_with_a_name_that_already_exists_is_refused() {
        let (_dir, pool) = test_pool().await;
        let existing =
            scheduler_core::quota::QuotaDefinition::from_fields(Some("Piano"), Some("4")).unwrap();
        crate::quota::store::create(&pool, &existing, 0)
            .await
            .unwrap();
        let capture_id = insert_untriaged_capture(&pool, "practise piano more").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "quota", "name": "piano", "hours": "2" }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response_json(response).await;
        assert_eq!(
            body.get("message").and_then(Value::as_str),
            Some(
                "\u{201c}Piano\u{201d} already exists at 4 h a week. Log your time against that \
                 one, or give this a different name."
            )
        );
        assert_eq!(
            body.get("quota_conflict").and_then(Value::as_str),
            Some("exact")
        );
        let quota_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            quota_count, 1,
            "the exact-match duplicate must not be created"
        );
    }

    /// A merely similar name warns rather than refusing outright, and
    /// offers a "Create anyway" control the resubmission's `confirmed`
    /// field can use to get past it
    /// (`quota-triage-validation-similar-name-warns-04`).
    #[tokio::test]
    async fn triaging_as_quota_with_a_similar_name_warns_and_can_be_confirmed() {
        let (_dir, pool) = test_pool().await;
        let existing =
            scheduler_core::quota::QuotaDefinition::from_fields(Some("Piano"), Some("4")).unwrap();
        crate::quota::store::create(&pool, &existing, 0)
            .await
            .unwrap();
        let capture_id = insert_untriaged_capture(&pool, "learn piano theory").await;

        let warned = triage_response(
            &pool,
            capture_id,
            json!({ "kind": "quota", "name": "Piano theory", "hours": "2" }),
        )
        .await;

        assert_eq!(warned.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response_json(warned).await;
        assert_eq!(
            body.get("message").and_then(Value::as_str),
            Some("That reads a lot like \u{201c}Piano\u{201d} (4 h a week). Same thing?")
        );
        assert_eq!(
            body.get("confirm_control").and_then(Value::as_str),
            Some("Create anyway")
        );

        let confirmed = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "quota",
                "name": "Piano theory",
                "hours": "2",
                "confirmed": "Piano theory"
            }),
        )
        .await;

        assert_eq!(confirmed.status(), StatusCode::CREATED);
        let quota_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(quota_count, 2);
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

        assert_json_rejected(
            &pool,
            capture_id,
            json!({ "kind": "pool", "life_area": "Home" }),
            json!({ "capture_not_open": "the capture is no longer in the inbox" }),
        )
        .await;
        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(task_count, 1, "the second triage must not add a task");
    }

    #[tokio::test]
    async fn triaging_a_capture_that_does_not_exist_is_rejected_as_not_open() {
        let (_dir, pool) = test_pool().await;

        assert_json_rejected(
            &pool,
            999,
            json!({ "kind": "pool", "life_area": "Work" }),
            json!({ "capture_not_open": "the capture is no longer in the inbox" }),
        )
        .await;
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
        for field in ["deadline", "commitment", "priority", "estimated_minutes"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            assert_missing_field_rejected(
                &pool,
                capture_id,
                committed_payload_missing(field),
                field,
            )
            .await;

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
        for field in ["deadline", "commitment", "priority", "estimated_minutes"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            let mut payload = json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "commitment": "at",
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

        assert_json_rejected(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "banana",
                "commitment": "at",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
            json!({ "invalid_field": "deadline" }),
        )
        .await;
    }

    /// A well-formed submission of `kind` with one field replaced by
    /// `value`, and the rejection that must draw.
    ///
    /// Four rejection tests had each written out a whole valid payload to
    /// change one cell of it, which is the shape `payloads::with_field`
    /// already exists for on the acceptance side. Each caller still names
    /// its own field and its own expected rejection, so a failure still
    /// reads as that field's test.
    async fn one_bad_field_is_rejected(
        kind: &str,
        field: &str,
        value: serde_json::Value,
        expected: serde_json::Value,
    ) {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;
        let mut payload = match kind {
            "committed" => json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "commitment": "at",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
            _ => json!({
                "kind": "quota",
                "name": "Piano",
                "hours": "4",
            }),
        };
        payload[field] = value;

        assert_json_rejected(&pool, capture_id, payload, expected).await;
    }

    #[tokio::test]
    async fn triaging_as_committed_with_an_invalid_commitment_is_rejected_naming_it_invalid() {
        one_bad_field_is_rejected(
            "committed",
            "commitment",
            json!("hard"),
            json!({ "invalid_field": "commitment" }),
        )
        .await;
    }

    /// `commitment` replaced `deadline_type` on the form (#94); the column
    /// stays in the schema, unread. A submission that still carries
    /// `deadline_type` alongside a valid `commitment` is not rejected for
    /// it -- presence, not the column's mere existence, is what a rejection
    /// could ever be about, and nothing reads this one any more.
    #[tokio::test]
    async fn triaging_as_committed_with_a_deadline_type_present_is_not_rejected_for_it() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        let response = triage_response(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "commitment": "at",
                "deadline_type": "hard",
                "priority": "P1",
                "estimated_minutes": 180,
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn triaging_as_committed_with_an_invalid_priority_is_rejected_naming_it_invalid() {
        one_bad_field_is_rejected(
            "committed",
            "priority",
            json!("P9"),
            json!({ "invalid_field": "priority" }),
        )
        .await;
    }

    #[tokio::test]
    async fn triaging_as_committed_with_a_zero_estimate_is_rejected_naming_it_invalid() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

        assert_json_rejected(
            &pool,
            capture_id,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "commitment": "at",
                "priority": "P1",
                "estimated_minutes": 0,
            }),
            json!({ "invalid_field": "estimated_minutes" }),
        )
        .await;
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
    async fn triaging_as_quota_with_a_blank_name_is_rejected_the_same_as_absent() {
        one_bad_field_is_rejected(
            "quota",
            "name",
            json!("   "),
            json!({ "missing_field": "name" }),
        )
        .await;
    }

    #[tokio::test]
    async fn triaging_as_quota_with_unparseable_hours_is_rejected_naming_it_invalid() {
        one_bad_field_is_rejected(
            "quota",
            "hours",
            json!("four"),
            json!({ "invalid_field": "hours" }),
        )
        .await;
    }

    #[tokio::test]
    async fn triaging_as_a_kind_that_is_not_one_of_the_three_is_rejected_naming_it() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        assert_json_rejected(
            &pool,
            capture_id,
            json!({ "kind": "someday" }),
            json!({ "unknown_kind": "someday" }),
        )
        .await;

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

        assert_json_rejected(
            &pool,
            capture_id,
            json!({}),
            json!({ "unknown_kind": Value::Null }),
        )
        .await;
    }

    /// Folded in from the PR #31 review: `string_field` drops a wrong-typed
    /// `kind` before the core ever sees it, so the rejection used to report
    /// `{"unknown_kind": null}` for `{"kind": 7}` — losing exactly the value
    /// T-unknown-kind-rejected says the caller should see echoed back.
    #[tokio::test]
    async fn triaging_with_kind_submitted_as_the_wrong_json_type_echoes_what_was_submitted() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        assert_json_rejected(
            &pool,
            capture_id,
            json!({ "kind": 7 }),
            json!({ "unknown_kind": 7 }),
        )
        .await;
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
                ("commitment", "at"),
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
            "commitment": "at",
            "priority": "P1",
            "estimated_minutes": 180,
            "name": "Piano",
            "hours": "4"
        });

        assert_eq!(
            triage_fields(&payload),
            TriageFields {
                kind: Some("quota".to_string()),
                deadline: Some("2026-08-20T17:00:00Z".to_string()),
                commitment: Some("at".to_string()),
                priority: Some("P1".to_string()),
                estimated_minutes: Some(180),
                quota_name: Some("Piano".to_string()),
                quota_hours: Some("4".to_string()),
                ..TriageFields::default()
            }
        );
    }

    #[test]
    fn triage_fields_treats_an_empty_body_as_nothing_submitted() {
        assert_eq!(triage_fields(&json!({})), TriageFields::default());
    }

    #[test]
    fn triage_fields_ignores_a_field_submitted_with_the_wrong_json_type() {
        let payload = json!({ "kind": 7, "name": 3 });
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
        put("commitment", fields.commitment.clone().map(Value::from));
        put("priority", fields.priority.clone().map(Value::from));
        put(
            "estimated_minutes",
            fields.estimated_minutes.map(Value::from),
        );
        put("name", fields.quota_name.clone().map(Value::from));
        put("hours", fields.quota_hours.clone().map(Value::from));
        Value::Object(body)
    }

    fn form_request(fields: &TriageFields) -> TriageFormRequest {
        TriageFormRequest {
            kind: fields.kind.clone(),
            deadline: fields.deadline.clone(),
            deadline_date: fields.deadline_date.clone(),
            deadline_time: fields.deadline_time.clone(),
            commitment: fields.commitment.clone(),
            priority: fields.priority.clone(),
            estimated_minutes: fields.estimated_minutes,
            name: fields.quota_name.clone(),
            hours: fields.quota_hours.clone(),
            context_tag: None,
            confirmed: None,
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
            commitment in proptest::option::of(".{0,10}"),
            priority in proptest::option::of(".{0,6}"),
            estimated_minutes in proptest::option::of(any::<i64>()),
            quota_name in proptest::option::of(".{0,20}"),
            quota_hours in proptest::option::of(".{0,10}"),
        ) {
            let submission = TriageFields {
                kind,
                deadline,
                commitment,
                priority,
                estimated_minutes,
                quota_name,
                quota_hours,
                // #110's deadline_date/deadline_time/timezone are the page
                // form's own fields, with no JSON equivalent to round-trip
                // against -- left at their default `None` here and covered
                // by their own example tests instead.
                ..TriageFields::default()
            };

            let from_json = triage_fields(&json_body(&submission));
            let from_form: TriageFields = TriageFields::from(&form_request(&submission));

            prop_assert_eq!(&from_json, &submission);
            prop_assert_eq!(from_json, from_form);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The context-tag counterpart of the property above: not part of
        /// `TriageFields` (it decides no kind and fails no submission), but
        /// still a field both transports must agree on
        /// (`context-tags-taggable-at-triage-08`).
        #[test]
        #[ignore]
        fn both_transports_extract_the_same_context_tag(
            context_tag in proptest::option::of(".{0,20}"),
        ) {
            let mut json_payload = json_body(&TriageFields::default());
            if let Some(tag) = &context_tag {
                json_payload["context_tag"] = Value::from(tag.clone());
            }
            let (_, _, from_json) = fields_from_input(TriageInput::Json(json_payload));

            let mut form = form_request(&TriageFields::default());
            form.context_tag = context_tag.clone();
            let (_, _, from_form) = fields_from_input(TriageInput::Form(Box::new(form)));

            prop_assert_eq!(from_json.context_tag, context_tag.clone());
            prop_assert_eq!(from_form.context_tag, context_tag);
        }
    }
}
