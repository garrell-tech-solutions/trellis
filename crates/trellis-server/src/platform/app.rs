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
use crate::committed::http::{
    mark_committed_task_done, show_committed, unmark_committed_task_done,
};
use crate::dismiss::http::dismiss_capture;
use crate::inbox::http::{set_shown_kind, show_inbox};
use crate::platform::assets::{
    htmx_js, ibm_plex_sans_woff2, icon_192, icon_512, icon_maskable_512, manifest, trellis_css,
};
use crate::platform::clock::Clock;
use crate::pool::http::{
    clear_pool_trip_done, complete_pool_trip, mark_pool_task_done, show_pool, unmark_pool_task_done,
};
use crate::quota::http::{correct_session, delete_session, log_session, show_quota};
use crate::settings::http::set_timezone;
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
        .route("/static/trellis.css", get(trellis_css))
        .route(
            "/static/fonts/ibm-plex-sans-variable.woff2",
            get(ibm_plex_sans_woff2),
        )
        .route("/manifest.webmanifest", get(manifest))
        .route("/static/icons/icon-192.png", get(icon_192))
        .route("/static/icons/icon-512.png", get(icon_512))
        .route(
            "/static/icons/icon-maskable-512.png",
            get(icon_maskable_512),
        )
        .route("/captures", post(create_capture))
        .route("/captures/{id}/triage", post(create_triage))
        .route("/captures/{id}/dismiss", post(dismiss_capture))
        .route("/captures/{id}/kind", post(set_shown_kind))
        .route("/timezone", post(set_timezone))
        .route("/pool", get(show_pool))
        .route("/pool/tasks/{id}/done", post(mark_pool_task_done))
        .route("/pool/tasks/{id}/undone", post(unmark_pool_task_done))
        .route("/pool/trips/{tag}/clear", post(clear_pool_trip_done))
        .route("/pool/trips/{tag}/complete", post(complete_pool_trip))
        .route("/committed", get(show_committed))
        .route("/committed/tasks/{id}/done", post(mark_committed_task_done))
        .route(
            "/committed/tasks/{id}/undone",
            post(unmark_committed_task_done),
        )
        .route("/quota", get(show_quota))
        .route("/quota/{id}/sessions", post(log_session))
        .route("/quota/sessions/{id}", post(correct_session))
        .route("/quota/sessions/{id}/delete", post(delete_session))
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

    /// **Every URL the manifest names is a real route.**
    ///
    /// The manifest is the *third* independent statement of this route
    /// table, after `build_app` above and `nav::Page::path`, and it is the
    /// one whose disagreement is hardest to see: a wrong `start_url` or icon
    /// `src` costs nothing until somebody installs Trellis on a phone and
    /// taps the home-screen icon. That is the failure mode this project
    /// keeps shipping -- `#101` went out three times unseen, and `#83`
    /// closed "Capture works from a phone" on a stylesheet.
    ///
    /// What existed already covers what somebody remembered: `assets.rs`
    /// compares `start_url` to the literal `"/"`, which agrees with itself
    /// however wrong both are, and `installable.feature` fetches the icon
    /// sizes its own Examples table lists. This **walks** the manifest, so a
    /// fourth icon is covered by existing -- the same reason
    /// [`every_header_link_reaches_the_page_it_names`] walks `nav::ALL`
    /// rather than restating it.
    #[tokio::test]
    async fn every_url_the_manifest_names_is_a_real_route() {
        let (_dir, pool) = test_pool().await;

        let manifest: serde_json::Value =
            serde_json::from_str(crate::platform::assets::manifest_source())
                .expect("the embedded manifest is JSON");

        let mut urls = vec![manifest["start_url"]
            .as_str()
            .expect("a manifest declares a start_url")
            .to_string()];
        urls.extend(
            manifest["icons"]
                .as_array()
                .expect("a manifest declares an icons array")
                .iter()
                .map(|icon| {
                    icon["src"]
                        .as_str()
                        .expect("every declared icon names a src")
                        .to_string()
                }),
        );
        assert!(
            urls.len() >= 2,
            "the walk found {} url(s); a manifest with no icons would pass vacuously",
            urls.len()
        );

        for url in urls {
            let response = build_app(pool.clone(), Clock::system())
                .oneshot(Request::builder().uri(&url).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "the manifest names {url}, which this server does not serve"
            );
        }
    }

    /// **Every header link is a real route, and the page it reaches agrees
    /// about which one it is.**
    ///
    /// `build_app` above and `nav::Page::path` are two statements of the same
    /// two paths, and nothing in the type system makes them agree: rename a
    /// route here and the header keeps offering the old one, which 404s.
    /// Walking `nav::ALL` covers whatever pages exist rather than a
    /// hand-maintained Examples table listing today's two by hand.
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

    /// The 50 ms budget itself moved to `qa/capture_endpoint.md` and
    /// `qa/one_screen.md` in #88 (`T-latency-is-a-qa-assertion`): a reading
    /// taken on a machine fighting a mutation run is not evidence, and it
    /// failed `cargo-mutants`' own unmutated baseline at 1.885s, costing the
    /// whole crate its mutation coverage. This keeps the part of the test
    /// that is still a unit-test question -- 201, and the row lands.
    #[tokio::test]
    async fn capture_request_persists_a_row_and_responds_201() {
        let (_dir, pool) = test_pool().await;
        let app = build_app(pool.clone(), Clock::system());

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

        assert_eq!(response.status(), StatusCode::CREATED);

        let row: (String, String) = sqlx::query_as("SELECT raw_text, source FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string()));
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
                let capture_id = crate::capture::store::insert(&pool, "buy milk", "web", None, 0)
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
