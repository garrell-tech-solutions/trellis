use sqlx::SqlitePool;

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
