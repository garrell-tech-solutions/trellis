//! The `#lists` fragment: the inbox and task list as one swappable block.
//!
//! Two endpoints render it — `GET /` wraps it in the full page, and a
//! page-originated triage swaps it in on its own — so it lives beside them
//! rather than inside either. `triage` previously reached into `inbox` for
//! it, which made the triage endpoint depend on the inbox *page* when what it
//! actually needs is the fragment they share.

use crate::http::view::{CaptureRow, TaskRow};
use crate::store;
use askama::Template;
use sqlx::SqlitePool;

/// The `#lists` fragment on its own — what a page-originated triage response
/// swaps in. Kept separate from the full-page template (rather than making
/// that template the triage response) because a triage response is not a
/// page: it has no `<head>`, no quick-add form, nothing but the two lists
/// htmx is replacing.
#[derive(Template)]
#[template(path = "lists.html")]
pub(crate) struct ListsTemplate {
    pub(crate) captures: Vec<CaptureRow>,
    pub(crate) tasks: Vec<TaskRow>,
}

/// Fetches the current inbox and task list, attaching `error` to whichever
/// capture's triage attempt just failed (if any). Shared by the inbox page
/// and the triage handler's page-originated response: "the page and
/// `POST /captures/{id}/triage` are one code path" extends to what gets
/// rendered afterward, not just to how the write itself happens.
pub(crate) async fn build_lists(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<(Vec<CaptureRow>, Vec<TaskRow>), sqlx::Error> {
    let captures = store::capture::list_untriaged(pool)
        .await?
        .into_iter()
        .map(|capture| CaptureRow {
            id: capture.id,
            error: error
                .as_ref()
                .filter(|(id, _)| *id == capture.id)
                .map(|(_, message)| message.clone()),
            text: capture.raw_text,
        })
        .collect();
    let tasks = store::task::list_all(pool)
        .await?
        .into_iter()
        .map(|task| TaskRow {
            kind: task.kind,
            text: task.raw_text,
        })
        .collect();
    Ok((captures, tasks))
}
