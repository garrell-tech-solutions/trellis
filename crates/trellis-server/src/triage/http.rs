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
//!
//! Which transport arrived is [`super::input`]'s business and stops being
//! visible here after `fields_from_input`; what a refusal says is
//! [`super::rejection`]'s. What is left in this module is the order things
//! happen in: well-formedness, then the questions that need a database,
//! then the writes.

use crate::inbox;
use crate::platform::clock::Clock;
use crate::platform::response::write_failed;
use crate::quota::NameStanding;
use crate::triage::input::{fields_from_input, TriageInput};
use crate::triage::rejection::{rejected, rejection_message, Rejection};
use crate::triage::store;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use scheduler_core::task::{TaskKind, TriageFields};
use serde_json::{json, Value};
use sqlx::SqlitePool;

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

/// A quota's name put to the quota capability (#138, moved from the retired
/// quota-screen define form to triage's own front door): `None` when
/// nothing is in the way, or the write-blocking rejection otherwise.
///
/// Triage asks where the name *stands* rather than reading the quotas
/// itself. Which quotas exist, and how the comparison is reached, are
/// `crate::quota`'s business; what triage adds is the half the quota
/// capability has no way to know -- that this submission already carried a
/// "Create anyway", so a resemblance has been warned about once already.
async fn check_quota_name(
    pool: &SqlitePool,
    candidate_name: &str,
    confirmed: Option<&str>,
) -> Result<Option<Rejection>, sqlx::Error> {
    let rejection = match crate::quota::name_standing(pool, candidate_name).await? {
        NameStanding::Taken {
            name,
            weekly_target_minutes,
        } => Some(Rejection::QuotaNameExists {
            existing_name: name,
            existing_minutes: weekly_target_minutes,
        }),
        NameStanding::Resembles {
            name,
            weekly_target_minutes,
        } if !already_confirmed(confirmed, candidate_name) => Some(Rejection::QuotaNameSimilar {
            existing_name: name,
            existing_minutes: weekly_target_minutes,
        }),
        _ => None,
    };
    Ok(rejection)
}

/// [`check_quota_name`] for a `kind` that may or may not be a quota --
/// `None` for the other two kinds, no database touched.
async fn quota_name_rejection(
    pool: &SqlitePool,
    kind: &TaskKind,
    confirmed: Option<&str>,
) -> Result<Option<Rejection>, StatusCode> {
    let TaskKind::Quota { name, .. } = kind else {
        return Ok(None);
    };
    check_quota_name(pool, name, confirmed)
        .await
        .map_err(write_failed)
}

/// The two rejections that need a database, once the core has already
/// called `kind` well-formed: the capture named by the URL no longer open
/// (`dismiss-capture-no-second-triage-07`, `-no-triage-after-dismissal-05`),
/// or, for a quota, its name colliding with one that already exists (#138).
async fn database_rejection(
    pool: &SqlitePool,
    capture_id: i64,
    kind: &TaskKind,
    confirmed: Option<&str>,
) -> Result<Option<Rejection>, StatusCode> {
    if !inbox::capture_is_open(pool, capture_id)
        .await
        .map_err(write_failed)?
    {
        return Ok(Some(Rejection::CaptureNotOpen));
    }
    quota_name_rejection(pool, kind, confirmed).await
}

/// Well-formedness first (no database), then whatever the database has to
/// say about it.
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
    if let Some(rejection) = database_rejection(pool, capture_id, &kind, confirmed).await? {
        return Ok(TriageOutcome::Rejected(rejection));
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
}
