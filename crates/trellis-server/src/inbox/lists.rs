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
use crate::life_areas::view::LifeAreaOption;
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
///
/// `life_areas` rides along beside `captures` and `tasks` because the
/// triage forms inside `capture_row.html` are part of this same fragment —
/// the picker they offer must reflect the current, active set on every
/// render, the same "no restart" requirement the management page itself
/// has.
///
/// **`pub(super)`, which is the point of [`respond`].** Two other
/// capabilities used to name these three fields to build one of these
/// themselves; what the fragment is made of is the inbox's business, and a
/// fourth list joining it should not be a four-file change. It stays visible
/// inside `inbox` because [`super::http`] wraps the same three lists in the
/// full page.
#[derive(Template)]
#[template(path = "lists.html")]
pub(super) struct ListsTemplate {
    pub(super) captures: Vec<CaptureRow>,
    pub(super) tasks: Vec<TaskRow>,
    pub(super) life_areas: Vec<LifeAreaOption>,
    /// Every distinct context tag used before, earliest first -- what the
    /// tag control's `<datalist>` offers (`context-tags-suggestions-05`).
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
        life_areas: crate::life_areas::active_options(pool).await?,
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
            error: error
                .as_ref()
                .filter(|(id, _)| *id == capture.id)
                .map(|(_, message)| message.clone()),
            text: capture.raw_text,
            context_tag: capture.context_tag,
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
            life_area: task.life_area_name,
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
        crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, None, 0)
            .await
            .unwrap();

        let tasks = build_lists(&pool, None).await.unwrap().tasks;

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].kind, "pool");
        assert_eq!(tasks[0].text, "buy milk");
    }

    #[tokio::test]
    async fn the_life_area_options_are_the_active_seed() {
        let (_dir, pool) = test_pool().await;

        let life_areas = build_lists(&pool, None).await.unwrap().life_areas;

        assert_eq!(
            life_areas
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Work", "Fitness", "Learning", "Family", "Home"]
        );
    }

    #[tokio::test]
    async fn an_archived_life_area_is_excluded_from_the_picker_options() {
        let (_dir, pool) = test_pool().await;
        let learning = crate::life_areas::store::find_by_name(&pool, "Learning")
            .await
            .unwrap()
            .unwrap();
        crate::life_areas::store::archive(&pool, learning.id, 1_000)
            .await
            .unwrap();

        let life_areas = build_lists(&pool, None).await.unwrap().life_areas;

        assert!(!life_areas.iter().any(|a| a.name == "Learning"));
    }
}
