//! `GET /pool`: pool work grouped by where it can be done (#92).
//! `POST /pool/tasks/{id}/done`: marks a pool task done (#97) and swaps in
//! the `#pool-body` fragment.

use super::body;
use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::render_template;
use crate::platform::response::write_failed;
use crate::pool::view::{LooseItemView, TripView};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The full page is the only thing that is not the `#pool-body` fragment,
/// so it is the only caller that takes `body::build`'s fields apart instead
/// of going through `body::respond`. `pool.html` `{% include %}`s
/// `pool_body.html`, and an Askama include renders in its parent's context,
/// so the page template has to carry the same fields by the same names.
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
    let built = body::build(&pool).await.map_err(write_failed)?;
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

/// Marks `task_id` done and swaps in the current `#pool-body` fragment,
/// regardless of whether it had already been marked -- there is no failure
/// state worth reporting back on this row (`T-forms-swap-one-fragment`'s
/// contract still holds: whatever happened, the fragment reflects current
/// state). `D-inaction-archives` still leaves nothing to un-do *silently*;
/// #122 gave the row its own explicit undo in [`unmark_pool_task_done`],
/// which is a deliberate tap, not an inaction.
pub async fn mark_pool_task_done(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(task_id): Path<i64>,
) -> Result<Response, StatusCode> {
    crate::mark_done::mark_task_done(&pool, task_id, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool).await
}

/// Unchecks `task_id` (#122: the direct inverse of the tap that struck it)
/// and swaps in the current `#pool-body` fragment. Same no-failure-state
/// shape as [`mark_pool_task_done`]: unchecking a task that is already
/// open, already cleared, or does not exist is a no-op the fragment already
/// reflects correctly either way.
pub async fn unmark_pool_task_done(
    State(pool): State<SqlitePool>,
    Path(task_id): Path<i64>,
) -> Result<Response, StatusCode> {
    crate::mark_done::unmark_task_done(&pool, task_id)
        .await
        .map_err(write_failed)?;
    body::respond(&pool).await
}

/// Clears every struck-through, not-yet-cleared task at `tag` (#122's
/// "Clear done") and swaps in the current `#pool-body` fragment. `tag`
/// arrives percent-decoded by the `Path` extractor; the template
/// percent-encodes it going out (`urlencode_strict`, since a tag may
/// contain `/`).
pub async fn clear_pool_trip_done(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
    Path(tag): Path<String>,
) -> Result<Response, StatusCode> {
    crate::pool::store::clear_done(&pool, &tag, clock.now_ms())
        .await
        .map_err(write_failed)?;
    body::respond(&pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::clock::Clock;
    use crate::platform::test_support::{archived_at, test_pool};
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

    async fn post_mark_done(pool: &SqlitePool, task_id: i64) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::pinned_at(4242));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/pool/tasks/{task_id}/done"))
                    .body(Body::empty())
                    .unwrap(),
            )
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

    async fn task_id_for_capture(pool: &SqlitePool, capture_id: i64) -> i64 {
        sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
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

    #[tokio::test]
    async fn marking_a_task_done_removes_it_from_the_next_render() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = task_id_for_capture(&pool, capture_id).await;

        let (status, body) = post_mark_done(&pool, task_id).await;

        assert_eq!(status, StatusCode::OK);
        assert!(!body.contains("buy screws"), "got:\n{body}");
    }

    #[tokio::test]
    async fn marking_a_task_done_still_renders_a_different_remaining_task() {
        let (_dir, pool) = test_pool().await;
        let done_capture_id = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let done_task_id = task_id_for_capture(&pool, done_capture_id).await;
        given_a_pool_task(&pool, "call the dentist", None).await;

        let (status, body) = post_mark_done(&pool, done_task_id).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("call the dentist"), "got:\n{body}");
    }

    #[tokio::test]
    async fn marking_a_task_done_stamps_the_row_rather_than_deleting_it() {
        let (_dir, pool) = test_pool().await;
        let capture_id = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = task_id_for_capture(&pool, capture_id).await;

        post_mark_done(&pool, task_id).await;

        assert!(
            archived_at(&pool, task_id).await.is_some(),
            "expected archived_at to be stamped, got None"
        );
    }

    // --- #122: a trip survives being worked -----------------------------

    async fn post_undone(pool: &SqlitePool, task_id: i64) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::pinned_at(4242));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/pool/tasks/{task_id}/undone"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    async fn post_clear(pool: &SqlitePool, tag: &str) -> (StatusCode, String) {
        let app = crate::platform::app::build_app(pool.clone(), Clock::pinned_at(4242));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/pool/trips/{}/clear",
                        urlencoding_placeholder(tag)
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    /// Minimal percent-encoding for the one character (`@`) every fixture
    /// tag in this file's tests carries -- not a general-purpose encoder,
    /// just enough for these tests to name a real URL.
    fn urlencoding_placeholder(tag: &str) -> String {
        tag.replace('@', "%40")
    }

    async fn given_three_pool_tasks(pool: &SqlitePool, tag: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for text in ["buy screws", "return the drill", "pick up trim"] {
            let capture_id = given_a_pool_task(pool, text, Some(tag)).await;
            ids.push(task_id_for_capture(pool, capture_id).await);
        }
        ids
    }

    async fn given_five_pool_tasks(pool: &SqlitePool, tag: &str) -> Vec<i64> {
        let mut ids = Vec::new();
        for text in [
            "buy screws",
            "return the drill",
            "pick up trim",
            "grab a tarp",
            "buy screws again",
        ] {
            let capture_id = given_a_pool_task(pool, text, Some(tag)).await;
            ids.push(task_id_for_capture(pool, capture_id).await);
        }
        ids
    }

    #[tokio::test]
    async fn a_trip_holds_after_marking_one_of_three_done() {
        let (_dir, pool) = test_pool().await;
        let ids = given_three_pool_tasks(&pool, "@homedepot").await;

        let (status, body) = post_mark_done(&pool, ids[0]).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("@homedepot"), "got:\n{body}");
        assert!(body.contains("1 of 3 done"), "got:\n{body}");
        assert!(
            body.contains("buy screws"),
            "the struck item stays in place, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn unchecking_a_struck_item_puts_it_back() {
        let (_dir, pool) = test_pool().await;
        let ids = given_three_pool_tasks(&pool, "@homedepot").await;
        post_mark_done(&pool, ids[0]).await;

        let (status, body) = post_undone(&pool, ids[0]).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("3 things"), "got:\n{body}");
        assert!(archived_at(&pool, ids[0]).await.is_none());
    }

    #[tokio::test]
    async fn clearing_done_removes_only_the_struck_items() {
        let (_dir, pool) = test_pool().await;
        let ids = given_five_pool_tasks(&pool, "@homedepot").await;
        post_mark_done(&pool, ids[0]).await;
        post_mark_done(&pool, ids[1]).await;
        post_mark_done(&pool, ids[2]).await;

        let (status, body) = post_clear(&pool, "@homedepot").await;

        assert_eq!(status, StatusCode::OK);
        assert!(!body.contains(">buy screws<"), "got:\n{body}");
        assert!(!body.contains("return the drill"), "got:\n{body}");
        assert!(!body.contains("pick up trim"), "got:\n{body}");
        // Two survive (grab a tarp, buy screws again), which is itself
        // below TRIP_THRESHOLD -- qa/trip_progress.md's own "second case"
        // (`trip-progress-clear-done-04`'s Examples: 0 struck, 2 loose):
        // clearing can take a group below the threshold same as scenario
        // 05 does, this is simply the five-item fixture reaching it too.
        assert!(
            body.contains("grab a tarp") && body.contains("buy screws again"),
            "got:\n{body}"
        );
        assert!(
            body.contains("@homedepot"),
            "the survivors keep their tag, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn clearing_can_drop_a_group_below_the_threshold_to_loose_ends() {
        let (_dir, pool) = test_pool().await;
        let ids = given_three_pool_tasks(&pool, "@homedepot").await;
        post_mark_done(&pool, ids[0]).await;

        let (_, body) = post_clear(&pool, "@homedepot").await;

        assert!(
            !body.contains("Show"),
            "sanity: no truncation control expected"
        );
        assert!(
            body.contains("return the drill") && body.contains("pick up trim"),
            "got:\n{body}"
        );
        // Two items remain at the tag -- below TRIP_THRESHOLD -- so the tag
        // no longer heads a trip panel; the loose-ends section carries them.
        let trip_header_count = body.matches("trip-tag").count();
        assert_eq!(trip_header_count, 0, "got:\n{body}");
    }

    #[tokio::test]
    async fn a_fully_done_group_vanishes_once_cleared() {
        let (_dir, pool) = test_pool().await;
        let ids = given_three_pool_tasks(&pool, "@homedepot").await;
        for id in &ids {
            post_mark_done(&pool, *id).await;
        }

        let (_, body) = post_clear(&pool, "@homedepot").await;

        assert!(!body.contains("@homedepot"), "got:\n{body}");
    }
}
