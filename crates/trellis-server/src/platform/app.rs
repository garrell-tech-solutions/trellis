//! Composition root: wires each capability's handlers onto routes and hands
//! them the pool they persist through and the clock they stamp rows with.
//!
//! The one place in the crate that names every business domain at once —
//! which is what a route table is, and why it sits in `platform` rather than
//! inside any capability. Read top to bottom it is also the shortest
//! statement of what this server does.
//!
//! It is also the one place that decides what a handler may reach for. Both
//! of [`AppState`]'s fields are things the outside world supplies — a
//! database and a clock — so composing them here is what keeps a handler from
//! going and finding either for itself.

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use sqlx::SqlitePool;

use crate::capture::http::create_capture;
use crate::dismiss::http::dismiss_capture;
use crate::inbox::http::show_inbox;
use crate::life_areas::http::{
    archive_life_area, create_life_area, remove_guardrail_band, save_guardrail, show_life_areas,
};
use crate::platform::assets::htmx_js;
use crate::platform::clock::Clock;
use crate::settings::http::set_timezone;
use crate::stats::http::show_stats;
use crate::triage::http::create_triage;

/// What a handler is given: somewhere to persist, and an answer to "what time
/// is it". The [`FromRef`] impls below let each handler extract only the half
/// it uses, so `show_inbox` still names nothing but a pool.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub clock: Clock,
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Clock {
    fn from_ref(state: &AppState) -> Self {
        state.clock
    }
}

pub fn build_app(pool: SqlitePool, clock: Clock) -> Router {
    Router::new()
        .route("/", get(show_inbox))
        .route("/static/htmx.min.js", get(htmx_js))
        .route("/captures", post(create_capture))
        .route("/captures/{id}/triage", post(create_triage))
        .route("/captures/{id}/dismiss", post(dismiss_capture))
        .route("/stats", get(show_stats))
        .route("/life-areas", get(show_life_areas).post(create_life_area))
        .route("/life-areas/{id}/archive", post(archive_life_area))
        .route("/life-areas/{id}/guardrail", post(save_guardrail))
        .route("/guardrail-bands/{id}/remove", post(remove_guardrail_band))
        .route("/timezone", post(set_timezone))
        .with_state(AppState { pool, clock })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::nav;
    use crate::platform::test_support::test_pool;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use proptest::prelude::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn capture_request_persists_a_row_and_responds_within_50ms() {
        let (_dir, pool) = test_pool().await;
        let app = build_app(pool.clone(), Clock::system());

        let start = std::time::Instant::now();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/captures")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"raw_text":"buy milk","source":"web"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(elapsed.as_millis() < 50, "took {elapsed:?}");

        let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string()));
    }

    /// **Every header link is a real route, and the page it reaches agrees
    /// about which one it is.**
    ///
    /// `build_app` above and `nav::Page::path` are two statements of the same
    /// three paths, and nothing in the type system makes them agree: rename a
    /// route here and the header keeps offering the old one, which 404s. The
    /// acceptance suite asserts the same thing, but from an Examples table
    /// listing today's three pages by hand — so it covers what someone
    /// remembered to add, while this walks `nav::ALL` and covers whatever is
    /// in it.
    ///
    /// That is the difference the brief asked for. Its open question 2 named
    /// the hazard as *"the part most likely to be got wrong in a way that
    /// only shows up on page five"*, and page five is exactly the case a
    /// hand-maintained table misses: a new variant whose `path` has a typo,
    /// or whose handler was copied from another page and still declares that
    /// page current, fails here the moment it joins `ALL`.
    #[tokio::test]
    async fn every_header_link_reaches_the_page_it_names() {
        let (_dir, pool) = test_pool().await;

        for page in nav::ALL {
            let link = nav::links(page)
                .into_iter()
                .find(|link| link.current)
                .expect("a page's own link is in the header it renders");

            let response = build_app(pool.clone(), Clock::system())
                .oneshot(
                    Request::builder()
                        .uri(link.path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "the header offers {} but GET {} is not a page",
                link.label,
                link.path
            );
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body = String::from_utf8(body.to_vec()).unwrap();
            let header = header_of(&body);

            assert!(
                header.contains(&format!(r#"aria-current="page">{}</a>"#, link.label)),
                "GET {} does not mark {} as the current page",
                link.path,
                link.label
            );
            assert_eq!(
                header.matches("aria-current").count(),
                1,
                "GET {} marks more than one link current",
                link.path
            );
        }
    }

    /// The header's own markup. Scoped rather than searching the whole page,
    /// so "exactly one link is current" stays a claim about the nav even
    /// after some page's content grows an `aria-current` of its own.
    fn header_of(body: &str) -> &str {
        let start = body.find("<header>").expect("every page carries a header");
        let end = body.find("</header>").expect("the header is closed");
        &body[start..end]
    }

    /// One exit attempt against a capture, through the real router.
    /// `TRIAGE` and `DISMISS` are the two the inbox has.
    async fn attempt_exit(pool: &SqlitePool, capture_id: i64, triage: bool) -> StatusCode {
        let app = build_app(pool.clone(), Clock::system());
        let request = if triage {
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/triage"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"kind":"pool","life_area":"Work"}"#))
        } else {
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/dismiss"))
                .body(Body::empty())
        };
        app.oneshot(request.unwrap()).await.unwrap().status()
    }

    async fn left_inbox_at(pool: &SqlitePool, capture_id: i64) -> Option<i64> {
        sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

        /// **A capture leaves the inbox exactly once, and the first attempt
        /// is the one that counts** — for any sequence of triage and
        /// dismissal attempts, in any order, of any length.
        ///
        /// This property belongs here rather than in either capability
        /// because neither can state it alone: it is about two handlers
        /// racing for one row, and this is the module where they meet. Both
        /// exits now write through `inbox::close_capture`, and the thing
        /// worth pinning is that consolidating them did not make a capture
        /// reachable by both — the failure mode `T-archived-at-only` names,
        /// arriving through the handlers instead of through the schema.
        ///
        /// Four invariants in one run: the first attempt succeeds with its
        /// own success code and every later one is refused; a `tasks` row
        /// exists if and only if the winner was a triage; the capture row
        /// survives all of it (`#9` AC-4); and the stamp never moves once
        /// written.
        #[test]
        #[ignore]
        fn a_capture_leaves_the_inbox_at_most_once(
            attempts in prop::collection::vec(any::<bool>(), 1..6),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (statuses, first_stamp, final_stamp, tasks, rows) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let capture_id = crate::capture::store::insert(&pool, "buy milk", "web", 0)
                    .await
                    .unwrap();

                let mut statuses = Vec::new();
                let mut first_stamp = None;
                for (index, triage) in attempts.iter().enumerate() {
                    statuses.push(attempt_exit(&pool, capture_id, *triage).await);
                    if index == 0 {
                        first_stamp = left_inbox_at(&pool, capture_id).await;
                    }
                }

                let final_stamp = left_inbox_at(&pool, capture_id).await;
                let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                (statuses, first_stamp, final_stamp, tasks, rows)
            });

            let winner_was_triage = attempts[0];
            let expected_first = if winner_was_triage {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            prop_assert_eq!(statuses[0], expected_first);
            for status in &statuses[1..] {
                prop_assert_eq!(*status, StatusCode::UNPROCESSABLE_ENTITY);
            }
            prop_assert_eq!(tasks, i64::from(winner_was_triage));
            prop_assert_eq!(rows, 1);
            prop_assert!(first_stamp.is_some());
            prop_assert_eq!(final_stamp, first_stamp);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]
        #[test]
        #[ignore]
        fn capture_round_trips_arbitrary_text_and_source(raw_text in ".{0,200}", source in ".{0,50}") {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (status, stored) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let app = build_app(pool.clone(), Clock::system());

                let body = serde_json::json!({"raw_text": raw_text, "source": source}).to_string();
                let response = app
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/captures")
                            .header("content-type", "application/json")
                            .body(Body::from(body))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let status = response.status();

                let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                (status, row)
            });

            prop_assert_eq!(status, StatusCode::CREATED);
            prop_assert_eq!(stored, (raw_text, source));
        }
    }
}
