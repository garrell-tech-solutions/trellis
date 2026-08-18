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

/// Issues a `GET` against a router built on `pool` and `clock`, and returns
/// the response body as text -- the "build the app, send one GET, read the
/// body" shape most handlers' own tests otherwise hand-roll per module.
/// Asserts `200 OK`, since every current caller wants exactly that; a caller
/// needing another status is not yet a caller of this.
pub(crate) async fn get_ok(pool: &SqlitePool, clock: Clock, uri: &str) -> String {
    let app = crate::platform::app::build_app(pool.clone(), clock);
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The id of a seeded life area, by name -- "give me Work" in one line.
///
/// Four modules had hand-rolled this as a local `work_id`/`fitness_id`,
/// which is the same reason [`get_ok`] is here: a fixture repeated per
/// module is one the fifth module writes slightly differently. It goes
/// through `life_areas::active_id_for_name`, so a test setting one up sees
/// exactly what production sees.
pub(crate) async fn seeded_life_area_id(pool: &SqlitePool, name: &str) -> i64 {
    crate::life_areas::active_id_for_name(pool, name)
        .await
        .expect("the life areas table is readable")
        .unwrap_or_else(|| panic!("{name} is not a seeded life area"))
}
