//! The `#lists` fragment: the inbox and task list as one swappable block.
//!
//! Two endpoints render it — `GET /` wraps it in the full page, and a
//! page-originated triage swaps it in on its own. It lives here, beside
//! [`super::http`] rather than inside it, for the reason it always has: the
//! triage endpoint needs the *fragment*, not the inbox page. What the
//! business-domain packaging settles is which capability the fragment belongs
//! to — it is two lists of the inbox's own rows, so the inbox owns it and
//! triage reaches in.

use crate::inbox::store;
use crate::inbox::view::{CaptureRow, TaskRow};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The `#lists` fragment on its own — what a page-originated triage or
/// dismissal swaps in. Kept separate from the full-page template (rather than
/// making that template the response) because such a response is not a page:
/// it has no `<head>`, no quick-add form, nothing but the two lists htmx is
/// replacing.
#[derive(Template)]
#[template(path = "lists.html")]
pub(super) struct ListsTemplate {
    pub(super) captures: Vec<CaptureRow>,
    pub(super) tasks: Vec<TaskRow>,
    /// Every tag in use, for the `<datalist>` every triage form and the
    /// quick-add box reference by `list=` (`context-tags-suggestions-05`).
    /// Rebuilt on every render from stored values, never from anything the
    /// process remembers — the same reason `captures` and `tasks` are.
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

/// Fetches the current inbox, task list and triage picker, attaching `error`
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
        tasks: build_task_rows(pool).await?,
        context_tag_suggestions: crate::capture::distinct_tags(pool).await?,
    })
}

async fn build_capture_rows(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<Vec<CaptureRow>, sqlx::Error> {
    Ok(store::list_untriaged(pool)
        .await?
        .into_iter()
        .map(|capture| CaptureRow {
            id: capture.id,
            context_tag: capture.context_tag,
            error: error
                .as_ref()
                .filter(|(id, _)| *id == capture.id)
                .map(|(_, message)| message.clone()),
            text: capture.raw_text,
        })
        .collect())
}

async fn build_task_rows(pool: &SqlitePool) -> Result<Vec<TaskRow>, sqlx::Error> {
    Ok(store::list_tasks(pool)
        .await?
        .into_iter()
        .map(|task| TaskRow {
            kind: task.kind,
            text: task.raw_text,
            context_tag: task.context_tag,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::store::insert as insert_capture;
    use crate::platform::test_support::test_pool;
    use scheduler_core::task::TaskKind;

    #[tokio::test]
    async fn an_error_attaches_only_to_the_capture_that_failed_triage() {
        let (_dir, pool) = test_pool().await;
        let failed_id = insert_capture(&pool, "call the dentist", "web", 0)
            .await
            .unwrap();
        let other_id = insert_capture(&pool, "buy milk", "web", 1).await.unwrap();

        let lists = build_lists(&pool, Some((failed_id, "deadline is required".to_string())))
            .await
            .unwrap();

        let failed_row = lists.captures.iter().find(|c| c.id == failed_id).unwrap();
        let other_row = lists.captures.iter().find(|c| c.id == other_id).unwrap();
        assert_eq!(failed_row.error.as_deref(), Some("deadline is required"));
        assert_eq!(other_row.error, None);
    }

    #[tokio::test]
    async fn the_task_list_carries_each_triaged_tasks_kind_and_capture_text() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", "web", 0).await.unwrap();
        crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();

        let tasks = build_lists(&pool, None).await.unwrap().tasks;

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].kind, "pool");
        assert_eq!(tasks[0].text, "buy milk");
    }

    #[tokio::test]
    async fn context_tag_suggestions_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        let lists = build_lists(&pool, None).await.unwrap();

        assert_eq!(lists.context_tag_suggestions, Vec::<String>::new());
    }

    #[tokio::test]
    async fn context_tag_suggestions_reflects_tags_in_use() {
        let (_dir, pool) = test_pool().await;
        crate::capture::create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        let lists = build_lists(&pool, None).await.unwrap();

        assert_eq!(
            lists.context_tag_suggestions,
            vec!["@homedepot".to_string()]
        );
    }

    #[tokio::test]
    async fn a_capture_row_carries_its_context_tag() {
        let (_dir, pool) = test_pool().await;
        crate::capture::create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        let lists = build_lists(&pool, None).await.unwrap();

        assert_eq!(lists.captures[0].context_tag.as_deref(), Some("@homedepot"));
    }
}
