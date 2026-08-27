//! The `#lists` fragment: `Recent`, as one swappable block.
//!
//! Two endpoints render it — `GET /` wraps it in the full page, and a
//! page-originated triage swaps it in on its own. It lives here, beside
//! [`super::http`] rather than inside it, for the reason it always has: the
//! triage endpoint needs the *fragment*, not the inbox page. What the
//! business-domain packaging settles is which capability the fragment belongs
//! to — it is a list of the inbox's own rows, so the inbox owns it and
//! triage reaches in.
//!
//! The plural name outlived the second list (#140) and is kept: it is the
//! `id` htmx swaps against, spelled in `inbox.html`, in every form's
//! `hx-target`, and in the acceptance suite. Renaming it is a rename of the
//! contract, not of this file.

use crate::inbox::shown_kind::ShownKind;
use crate::inbox::store;
use crate::inbox::view::CaptureRow;
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The `#lists` fragment on its own — what a page-originated triage or
/// dismissal swaps in. Kept separate from the full-page template (rather than
/// making that template the response) because such a response is not a page:
/// it has no `<head>`, no quick-add form, nothing but the list htmx is
/// replacing.
#[derive(Template)]
#[template(path = "lists.html")]
pub(super) struct ListsTemplate {
    pub(super) captures: Vec<CaptureRow>,
    /// Every tag in use, for the `<datalist>` every triage form and the
    /// quick-add box reference by `list=` (`context-tags-suggestions-05`).
    /// Rebuilt on every render from stored values, never from anything the
    /// process remembers — the same reason `captures` is.
    pub(super) context_tag_suggestions: Vec<String>,
}

/// `T-forms-swap-one-fragment`'s response contract, implemented once:
/// whatever happened, re-render `#lists` from current state at `status`,
/// carrying `error` on the row that caused it.
///
/// This is what the inbox's two exits ask for. Neither builds the fragment:
/// triage decided the outcome and dismissal decided the outcome, and the
/// shape of the answer is the inbox's (`T-one-front-door-per-capability`).
/// The contract is easy to half-implement — a 422 whose body is *not* the
/// re-rendered fragment breaks the whole page, since the 422 swap is
/// configured globally in `inbox.html` — so it is worth having exactly one
/// implementation of it.
pub(super) async fn respond(
    pool: &SqlitePool,
    status: StatusCode,
    error: Option<(i64, String)>,
) -> Result<Response, StatusCode> {
    let lists = build_lists(pool, error).await.map_err(write_failed)?;
    Ok(render_template(status, &lists))
}

/// Fetches the current `Recent` rows and triage picker, attaching `error`
/// to whichever capture's triage attempt just failed (if any). Shared by the
/// inbox page and [`respond`]: "the page and `POST /captures/{id}/triage` are
/// one code path" extends to what gets rendered afterward, not just to how
/// the write itself happens.
///
/// `pub(super)` because `inbox::http` wraps the same three lists in the full
/// page; everyone else goes through [`respond`].
pub(super) async fn build_lists(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<ListsTemplate, sqlx::Error> {
    Ok(ListsTemplate {
        captures: build_capture_rows(pool, error).await?,
        context_tag_suggestions: crate::capture::distinct_tags(pool).await?,
    })
}

/// `"pool"` reads `"Pool"` (#140): the row's own meta line spells the kind
/// the way the Menu tabs do, not the lowercase discriminant `tasks.kind`
/// stores. A table rather than a `match`, for the reason a fixed lookup
/// already is one -- three arms of no logic beyond the lookup.
const KIND_LABELS: [(&str, &str); 3] = [
    (scheduler_core::task::POOL, "Pool"),
    (scheduler_core::task::COMMITTED, "Committed"),
    (scheduler_core::task::QUOTA, "Quota"),
];

fn kind_label(kind: &str) -> &str {
    KIND_LABELS
        .iter()
        .find(|(stored, _)| *stored == kind)
        .map(|(_, label)| *label)
        .unwrap_or(kind)
}

/// `"Pool · @homedepot"`, or `"Pool · no context"` untagged -- what a
/// triaged row reads in place of its kind buttons
/// (`inbox-view-triaged-row-stays-04`).
fn triaged_meta(kind: &str, context_tag: Option<&str>) -> String {
    format!(
        "{} \u{b7} {}",
        kind_label(kind),
        context_tag.unwrap_or("no context")
    )
}

async fn build_capture_rows(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<Vec<CaptureRow>, sqlx::Error> {
    Ok(store::list_recent(pool)
        .await?
        .into_iter()
        .map(|capture| {
            // Whatever the column holds, read through the domain that owns
            // it -- a value outside it opens no panel rather than matching
            // a literal spelled here for the third time.
            let shown = capture.shown_kind.as_deref().and_then(ShownKind::parse);
            CaptureRow {
                id: capture.id,
                meta: capture
                    .kind
                    .as_deref()
                    .map(|kind| triaged_meta(kind, capture.context_tag.as_deref())),
                context_tag: capture.context_tag,
                error: error
                    .as_ref()
                    .filter(|(id, _)| *id == capture.id)
                    .map(|(_, message)| message.clone()),
                committed_open: shown == Some(ShownKind::Committed),
                quota_open: shown == Some(ShownKind::Quota),
                text: capture.raw_text,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::store::insert as insert_capture;
    use crate::platform::test_support::test_pool;
    use scheduler_core::task::TaskKind;

    #[test]
    fn kind_label_titlecases_each_of_the_three_kinds() {
        assert_eq!(kind_label("pool"), "Pool");
        assert_eq!(kind_label("committed"), "Committed");
        assert_eq!(kind_label("quota"), "Quota");
    }

    #[test]
    fn triaged_meta_names_the_context_tag_when_present() {
        assert_eq!(
            triaged_meta("pool", Some("@homedepot")),
            "Pool \u{b7} @homedepot"
        );
    }

    #[test]
    fn triaged_meta_reads_no_context_when_untagged() {
        assert_eq!(
            triaged_meta("committed", None),
            "Committed \u{b7} no context"
        );
    }

    #[tokio::test]
    async fn an_error_attaches_only_to_the_capture_that_failed_triage() {
        let (_dir, pool) = test_pool().await;
        let failed_id = insert_capture(&pool, "call the dentist", "web", None, 0)
            .await
            .unwrap();
        let other_id = insert_capture(&pool, "buy milk", "web", None, 1)
            .await
            .unwrap();

        let lists = build_lists(&pool, Some((failed_id, "deadline is required".to_string())))
            .await
            .unwrap();

        let failed_row = lists.captures.iter().find(|c| c.id == failed_id).unwrap();
        let other_row = lists.captures.iter().find(|c| c.id == other_id).unwrap();
        assert_eq!(failed_row.error.as_deref(), Some("deadline is required"));
        assert_eq!(other_row.error, None);
    }

    /// A capture created, triaged as `Pool`, and closed -- the shared setup
    /// for this module's own "what a triaged row reads" tests, with or
    /// without a context tag.
    async fn given_a_triaged_pool_capture(
        pool: &SqlitePool,
        text: &str,
        context_tag: Option<&str>,
    ) -> i64 {
        let capture_id = match context_tag {
            Some(tag) => {
                crate::capture::create(pool, text, "web", Some(tag), 0)
                    .await
                    .unwrap()
                    .0
            }
            None => insert_capture(pool, text, "web", None, 0).await.unwrap(),
        };
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        crate::inbox::close_capture(pool, capture_id, 0)
            .await
            .unwrap();
        capture_id
    }

    #[tokio::test]
    async fn a_triaged_capture_stays_and_reads_what_it_became() {
        let (_dir, pool) = test_pool().await;
        given_a_triaged_pool_capture(&pool, "buy milk", None).await;

        let captures = build_lists(&pool, None).await.unwrap().captures;

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].text, "buy milk");
        assert_eq!(captures[0].meta.as_deref(), Some("Pool \u{b7} no context"));
    }

    #[tokio::test]
    async fn a_triaged_captures_meta_names_its_context_tag() {
        let (_dir, pool) = test_pool().await;
        given_a_triaged_pool_capture(&pool, "buy screws", Some("@homedepot")).await;

        let captures = build_lists(&pool, None).await.unwrap().captures;

        assert_eq!(captures[0].meta.as_deref(), Some("Pool \u{b7} @homedepot"));
    }

    #[tokio::test]
    async fn an_untriaged_captures_meta_is_none() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk", "web", None, 0)
            .await
            .unwrap();

        let captures = build_lists(&pool, None).await.unwrap().captures;

        assert_eq!(captures[0].meta, None);
    }

    #[tokio::test]
    async fn context_tag_suggestions_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        let lists = build_lists(&pool, None).await.unwrap();

        assert_eq!(lists.context_tag_suggestions, Vec::<String>::new());
    }

    async fn lists_after_a_tagged_capture(tag: &str) -> (tempfile::TempDir, ListsTemplate) {
        let (dir, pool) = test_pool().await;
        crate::capture::create(&pool, "buy screws", "web", Some(tag), 0)
            .await
            .unwrap();
        (dir, build_lists(&pool, None).await.unwrap())
    }

    #[tokio::test]
    async fn context_tag_suggestions_reflects_tags_in_use() {
        let (_dir, lists) = lists_after_a_tagged_capture("@homedepot").await;

        assert_eq!(
            lists.context_tag_suggestions,
            vec!["@homedepot".to_string()]
        );
    }

    #[tokio::test]
    async fn a_capture_row_carries_its_context_tag() {
        let (_dir, lists) = lists_after_a_tagged_capture("@homedepot").await;

        assert_eq!(lists.captures[0].context_tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn a_freshly_captured_row_shows_no_kinds_panel() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk", "web", None, 0)
            .await
            .unwrap();

        let lists = build_lists(&pool, None).await.unwrap();

        assert!(!lists.captures[0].committed_open);
        assert!(!lists.captures[0].quota_open);
    }

    #[tokio::test]
    async fn a_row_with_committed_chosen_shows_the_committed_panel_and_not_quotas() {
        let (_dir, pool) = test_pool().await;
        let id = insert_capture(&pool, "buy milk", "web", None, 0)
            .await
            .unwrap();
        store::set_shown_kind(&pool, id, ShownKind::Committed)
            .await
            .unwrap();

        let lists = build_lists(&pool, None).await.unwrap();

        assert!(lists.captures[0].committed_open);
        assert!(!lists.captures[0].quota_open);
    }
}
