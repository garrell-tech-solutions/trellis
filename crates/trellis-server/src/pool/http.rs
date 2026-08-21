//! `GET /pool`: pool work grouped by where it can be done (#92).

use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use crate::pool::view::{self, LooseItemView, TripView};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "pool.html")]
struct PoolTemplate {
    meta: String,
    empty: bool,
    trips: Vec<TripView>,
    loose: Vec<LooseItemView>,
    nav: Vec<NavLink>,
}

pub async fn show_pool(State(pool): State<SqlitePool>) -> Result<Response, StatusCode> {
    let rows = super::store::list_pool_tasks(&pool)
        .await
        .map_err(write_failed)?;
    let built = view::build(rows);
    Ok(render_template(
        StatusCode::OK,
        &PoolTemplate {
            meta: built.meta,
            empty: built.empty,
            trips: built.trips,
            loose: built.loose,
            nav: nav::links(Page::Pool),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::clock::Clock;
    use crate::platform::test_support::test_pool;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use scheduler_core::task::TaskKind;
    use tower::ServiceExt;

    async fn get_pool(pool: &SqlitePool) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::system());
        let response = app
            .oneshot(Request::builder().uri("/pool").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    async fn given_a_pool_task(pool: &SqlitePool, raw_text: &str, tag: Option<&str>) -> i64 {
        let capture_id = crate::capture::store::insert(pool, raw_text, "web", tag, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        capture_id
    }

    #[tokio::test]
    async fn the_pool_screen_is_reachable_and_shows_the_empty_state_when_nothing_is_pooled() {
        let (_dir, pool) = test_pool().await;

        let (status, body) = get_pool(&pool).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("Nothing in the pool"),
            "expected the empty-state message, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn the_pool_screen_shows_a_trip_once_three_tasks_share_a_tag() {
        let (_dir, pool) = test_pool().await;
        given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        given_a_pool_task(&pool, "return the drill", Some("@homedepot")).await;
        given_a_pool_task(&pool, "pick up trim", Some("@homedepot")).await;

        let (_, body) = get_pool(&pool).await;

        assert!(body.contains("@homedepot"), "got:\n{body}");
        assert!(body.contains("buy screws"), "got:\n{body}");
    }

    /// Three spellings of one tag are one trip, because the *write* path
    /// made them one string before this screen ever saw them.
    ///
    /// **This is the only test on this path that goes through
    /// `capture::create`.** Every other fixture here calls
    /// `capture::store::insert` with a tag already in its final spelling,
    /// which is convenient and skips the very step the screen depends on:
    /// `scheduler_core::pool::group` buckets by plain string equality, and
    /// that is correct *only* because `capture::resolve_tag` canonicalized
    /// first. Those are two capabilities holding one invariant between
    /// them, and until now a doc comment was the only thing tying them.
    ///
    /// The failure it guards against is not subtle-but-harmless. Three
    /// items under one tag is exactly `TRIP_THRESHOLD`; split into two
    /// spellings they are groups of 2 and 1, both under it, so **both fall
    /// to loose ends and the trip vanishes from the screen entirely.**
    #[tokio::test]
    async fn case_variant_spellings_of_one_tag_make_one_trip_not_none() {
        let (_dir, pool) = test_pool().await;
        for (text, tag) in [
            ("buy screws", "@homedepot"),
            ("return the drill", "@HomeDepot"),
            ("pick up trim", "@HOMEDEPOT"),
        ] {
            let (capture_id, _) = crate::capture::create(&pool, text, "web", Some(tag), 0)
                .await
                .unwrap();
            crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, 0)
                .await
                .unwrap();
        }

        let (_, body) = get_pool(&pool).await;

        assert!(
            body.contains("3 things"),
            "the three spellings should be one trip of three, got:\n{body}"
        );
        assert_eq!(
            body.matches("@homedepot").count(),
            1,
            "the trip should be listed once, under the first spelling, got:\n{body}"
        );
        for text in ["buy screws", "return the drill", "pick up trim"] {
            assert!(body.contains(text), "missing {text}, got:\n{body}");
        }
    }

    #[tokio::test]
    async fn the_pool_screen_marks_pool_as_the_current_tab() {
        let (_dir, pool) = test_pool().await;

        let (_, body) = get_pool(&pool).await;

        assert!(body.contains(r#"aria-current="page""#), "got:\n{body}");
    }
}
