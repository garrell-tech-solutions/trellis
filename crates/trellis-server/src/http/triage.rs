//! `POST /captures/{id}/triage`.

use crate::clock::now_ms;
use crate::http::write_failed;
use crate::store;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use scheduler_core::task::{TaskKind, TriageFields, TriageRejection};
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn string_field(payload: &Value, name: &str) -> Option<String> {
    payload
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Translates the JSON body into the core's triage input. This function is
/// the whole of the transport's involvement: nothing it calls sees
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

/// The rejection contract: a client error whose body names what was wrong.
/// A rejection that does not say what was wrong is a failure even with the
/// right status code.
fn rejected(rejection: TriageRejection) -> (StatusCode, Json<Value>) {
    let body = match rejection {
        TriageRejection::MissingField(field) => json!({ "missing_field": field.name() }),
        TriageRejection::InvalidField(field) => json!({ "invalid_field": field.name() }),
        TriageRejection::UnknownKind(submitted) => json!({ "unknown_kind": submitted }),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
}

pub async fn create_triage(
    State(pool): State<SqlitePool>,
    Path(capture_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let kind = match TaskKind::from_fields(&triage_fields(&payload)) {
        Ok(kind) => kind,
        Err(rejection) => return Ok(rejected(rejection)),
    };

    let created_at_ms = now_ms();
    store::task::insert(&pool, capture_id, &kind, created_at_ms)
        .await
        .map_err(write_failed)?;
    store::capture::mark_triaged(&pool, capture_id, created_at_ms)
        .await
        .map_err(write_failed)?;

    Ok((StatusCode::CREATED, Json(json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_pool;
    use axum::body::Body;
    use axum::http::Request;
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
        let (status, Json(body)) = rejected(TriageRejection::MissingField(Field::Priority));
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "missing_field": "priority" }));
    }

    #[test]
    fn an_invalid_field_rejection_names_the_field() {
        let (status, Json(body)) = rejected(TriageRejection::InvalidField(Field::Deadline));
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, json!({ "invalid_field": "deadline" }));
    }
}
