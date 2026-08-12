use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

fn missing_committed_field(payload: &Value) -> Option<&'static str> {
    ["deadline", "deadline_type", "priority"]
        .into_iter()
        .find(|field| payload.get(field).is_none())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
}

pub async fn create_triage(
    State(pool): State<SqlitePool>,
    Path(capture_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let kind = payload.get("kind").and_then(Value::as_str).unwrap_or("");

    if kind == "committed" {
        if let Some(field) = missing_committed_field(&payload) {
            return Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "missing_field": field })),
            ));
        }
    }

    let deadline = payload.get("deadline").and_then(Value::as_str);
    let deadline_type = payload.get("deadline_type").and_then(Value::as_str);
    let priority = payload.get("priority").and_then(Value::as_str);
    let target_count = payload.get("target_count").and_then(Value::as_i64);
    let target_minutes_each = payload.get("target_minutes_each").and_then(Value::as_i64);
    let period = payload.get("period").and_then(Value::as_str);

    let created_at_ms = now_ms();
    sqlx::query(
        "INSERT INTO tasks (capture_id, kind, deadline, deadline_type, priority, \
         target_count, target_minutes_each, period, created_at_ms) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(capture_id)
    .bind(kind)
    .bind(deadline)
    .bind(deadline_type)
    .bind(priority)
    .bind(target_count)
    .bind(target_minutes_each)
    .bind(period)
    .bind(created_at_ms)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE captures SET triaged_at = ? WHERE id = ?")
        .bind(created_at_ms)
        .bind(capture_id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    async fn test_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = crate::db::connect(&db_path).await.unwrap();
        crate::db::run_migrations(&pool).await.unwrap();
        (dir, pool)
    }

    async fn insert_untriaged_capture(pool: &sqlx::SqlitePool, raw_text: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
        )
        .bind(raw_text)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn triage_response(
        pool: &sqlx::SqlitePool,
        capture_id: i64,
        body: serde_json::Value,
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

    #[tokio::test]
    async fn triaging_as_pool_creates_a_pool_task_with_no_deadline_or_quota_target() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_untriaged_capture(&pool, "buy milk").await;

        let response = triage_response(&pool, capture_id, json!({ "kind": "pool" })).await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let row: (String, Option<String>, Option<i64>, Option<i64>, Option<String>) =
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

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
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
        let row: (String, Option<String>, Option<String>, Option<String>, Option<i64>) =
            sqlx::query_as(
                "SELECT kind, deadline, deadline_type, priority, target_count FROM tasks WHERE capture_id = ?",
            )
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "committed");
        assert_eq!(row.1.as_deref(), Some("2026-08-20T17:00:00Z"));
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

    async fn committed_payload_missing(field: &str) -> serde_json::Value {
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
    async fn triaging_as_committed_without_a_required_field_is_rejected_and_creates_nothing() {
        for field in ["deadline", "deadline_type", "priority"] {
            let (_dir, pool) = test_pool().await;
            let capture_id = insert_untriaged_capture(&pool, "call the dentist").await;

            let response =
                triage_response(&pool, capture_id, committed_payload_missing(field).await).await;

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

    #[test]
    fn missing_committed_field_is_none_when_all_three_are_present() {
        let payload = json!({
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard",
            "priority": "P1"
        });
        assert_eq!(missing_committed_field(&payload), None);
    }

    #[test]
    fn missing_committed_field_reports_a_missing_deadline() {
        let payload = json!({
            "deadline_type": "hard",
            "priority": "P1"
        });
        assert_eq!(missing_committed_field(&payload), Some("deadline"));
    }

    #[test]
    fn missing_committed_field_reports_a_missing_deadline_type() {
        let payload = json!({
            "deadline": "2026-08-20T17:00:00Z",
            "priority": "P1"
        });
        assert_eq!(missing_committed_field(&payload), Some("deadline_type"));
    }

    #[test]
    fn missing_committed_field_reports_a_missing_priority() {
        let payload = json!({
            "deadline": "2026-08-20T17:00:00Z",
            "deadline_type": "hard"
        });
        assert_eq!(missing_committed_field(&payload), Some("priority"));
    }
}
