use crate::platform::clock::Clock;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use sqlx::SqlitePool;
use tower::ServiceExt;

pub(crate) async fn test_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let pool = crate::platform::db::connect(&db_path).await.unwrap();
    crate::platform::db::run_migrations(&pool).await.unwrap();
    (dir, pool)
}

/// A capture's stored context tag, read straight from the table -- both
/// `capture::mod` and `triage::http`'s own tests assert on it, since a
/// retag at triage writes the same column [`crate::capture::create`] does.
pub(crate) async fn stored_context_tag(pool: &SqlitePool, capture_id: i64) -> Option<String> {
    sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
        .bind(capture_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Setting up "a capture exists" by calling the capture domain's own writer
/// rather than retyping its `INSERT` here: a fixture that spells out
/// another module's SQL is a second copy of that schema. Shared by every
/// store module's own tests (`committed`, `pool`, `triage`, `inbox`) that
/// need one to triage or list against.
pub(crate) async fn insert_capture(pool: &SqlitePool, raw_text: &str, tag: Option<&str>) -> i64 {
    crate::capture::store::insert(pool, raw_text, "web", tag, 0)
        .await
        .unwrap()
}

/// A committed task, given only its capture text -- everything else pinned
/// to fixed, arbitrary values neither `committed::body`'s nor
/// `committed::http`'s own tests care about. Shared by both.
pub(crate) async fn given_a_committed_task(pool: &SqlitePool, raw_text: &str) -> i64 {
    let capture_id = insert_capture(pool, raw_text, None).await;
    crate::triage::store::insert_task(
        pool,
        capture_id,
        &scheduler_core::task::TaskKind::Committed {
            deadline: 1787646600000,
            commitment: scheduler_core::task::Commitment::At,
            priority: scheduler_core::task::Priority::P1,
            estimated_minutes: 30,
        },
        0,
    )
    .await
    .unwrap();
    sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
        .bind(capture_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A task's stored `archived_at` -- both `committed::http` and `pool::http`
/// assert on it after marking a task done (#97), since both go through the
/// same [`crate::mark_done::mark_task_done`] write.
pub(crate) async fn archived_at(pool: &SqlitePool, task_id: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT archived_at FROM tasks WHERE id = ?")
        .bind(task_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Sends `method uri` against a fresh app built on `pool` and `clock`, with
/// an empty body, and returns the status and the response body as text.
/// Shared by every capability's own `http` tests that only need a bare
/// request/response round trip -- `triage::http`'s own transports carry a
/// real body and stay local, since a shared helper for those would need as
/// many parameters as the thing it replaced.
pub(crate) async fn http_request(
    pool: &SqlitePool,
    clock: Clock,
    method: &str,
    uri: &str,
) -> (StatusCode, String) {
    let app = crate::platform::app::build_app(pool.clone(), clock);
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}
