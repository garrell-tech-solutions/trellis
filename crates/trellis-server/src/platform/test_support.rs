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
