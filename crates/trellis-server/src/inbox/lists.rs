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
use crate::life_areas::store as life_areas_store;
use crate::life_areas::view::LifeAreaOption;
use askama::Template;
use sqlx::SqlitePool;

/// The `#lists` fragment on its own — what a page-originated triage response
/// swaps in. Kept separate from the full-page template (rather than making
/// that template the triage response) because a triage response is not a
/// page: it has no `<head>`, no quick-add form, nothing but the two lists
/// htmx is replacing.
///
/// `life_areas` rides along beside `captures` and `tasks` because the
/// triage forms inside `capture_row.html` are part of this same fragment —
/// the picker they offer must reflect the current, active set on every
/// render, the same "no restart" requirement the management page itself
/// has.
#[derive(Template)]
#[template(path = "lists.html")]
pub(crate) struct ListsTemplate {
    pub(crate) captures: Vec<CaptureRow>,
    pub(crate) tasks: Vec<TaskRow>,
    pub(crate) life_areas: Vec<LifeAreaOption>,
}

/// Fetches the current inbox, task list and triage picker, attaching `error`
/// to whichever capture's triage attempt just failed (if any). Shared by the
/// inbox page and the triage handler's page-originated response: "the page
/// and `POST /captures/{id}/triage` are one code path" extends to what gets
/// rendered afterward, not just to how the write itself happens.
pub(crate) async fn build_lists(
    pool: &SqlitePool,
    error: Option<(i64, String)>,
) -> Result<(Vec<CaptureRow>, Vec<TaskRow>, Vec<LifeAreaOption>), sqlx::Error> {
    let captures = build_capture_rows(pool, error).await?;
    let tasks = build_task_rows(pool).await?;
    let life_areas = build_life_area_options(pool).await?;
    Ok((captures, tasks, life_areas))
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
        })
        .collect())
}

/// The triage picker's choices: every life area not yet archived
/// (`life_areas::store::list_active` -- the same query the management page's
/// own list uses, reused here rather than re-issued, since both need exactly
/// "what is currently choosable").
async fn build_life_area_options(pool: &SqlitePool) -> Result<Vec<LifeAreaOption>, sqlx::Error> {
    Ok(life_areas_store::list_active(pool)
        .await?
        .into_iter()
        .map(LifeAreaOption::from)
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

        let (captures, _tasks, _life_areas) =
            build_lists(&pool, Some((failed_id, "deadline is required".to_string())))
                .await
                .unwrap();

        let failed_row = captures.iter().find(|c| c.id == failed_id).unwrap();
        let other_row = captures.iter().find(|c| c.id == other_id).unwrap();
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

        let (_captures, tasks, _life_areas) = build_lists(&pool, None).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].kind, "pool");
        assert_eq!(tasks[0].text, "buy milk");
    }

    #[tokio::test]
    async fn the_life_area_options_are_the_active_seed() {
        let (_dir, pool) = test_pool().await;

        let (_captures, _tasks, life_areas) = build_lists(&pool, None).await.unwrap();

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

        let (_captures, _tasks, life_areas) = build_lists(&pool, None).await.unwrap();

        assert!(!life_areas.iter().any(|a| a.name == "Learning"));
    }
}
