//! What the committed screen reads: every committed task, with the text and
//! context tag of the capture it came from, and its deadline/commitment.
//!
//! `committed-screen-only-committed-04`: this query's `WHERE tasks.kind =
//! 'committed'` is the whole of what keeps pool and quota work off this
//! screen, the same shape `pool::store`'s own `WHERE` clause takes.
//!
//! `AND tasks.archived_at IS NULL` keeps a task you marked done off this
//! screen and out of its count (`mark-done-counts-exclude-04`).

use sqlx::SqlitePool;

/// A row of [`list_committed_tasks`]. `commitment` is not optional here:
/// every committed row this query can return was created after #94, through
/// this app's own triage boundary, which requires it
/// (`T-cross-capability-invariants-need-an-owner` applies the same way #92's
/// context-tag canonicalisation did -- a row this query cannot explain is a
/// bug to surface as a decode error, not a `None` to paper over).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct CommittedTaskRow {
    pub task_id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
    pub deadline: i64,
    pub commitment: String,
}

/// Every committed task, alongside the text and context tag of the capture
/// it was triaged from. Unordered on purpose:
/// `scheduler_core::committed_screen::order` establishes chronological order
/// itself rather than trusting a caller to have sorted already.
pub async fn list_committed_tasks(pool: &SqlitePool) -> Result<Vec<CommittedTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.id AS task_id, captures.raw_text, captures.context_tag, tasks.deadline, \
         tasks.commitment \
         FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.kind = 'committed' AND tasks.archived_at IS NULL",
    )
    .fetch_all(pool)
    .await
}

/// The raw text `task_id` was triaged from, for the way back a "done" POST
/// names it in (#111) -- [`list_committed_tasks`]'s own `WHERE` excludes an
/// archived row, so the task this is asked about the instant after marking
/// it done needs its own lookup rather than a scan of that list. Scoped to
/// `kind = 'committed'` the same way that query is, so an id from another
/// screen cannot borrow this one's way back.
pub async fn task_text(pool: &SqlitePool, task_id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT captures.raw_text FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.id = ? AND tasks.kind = 'committed'",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use scheduler_core::task::{Commitment, Priority, TaskKind};

    fn committed(deadline: i64, commitment: Commitment) -> TaskKind {
        TaskKind::Committed {
            deadline,
            commitment,
            priority: Priority::P1,
            estimated_minutes: 30,
        }
    }

    fn quota() -> TaskKind {
        TaskKind::Quota {
            name: "Piano".to_string(),
            weekly_target: scheduler_core::quota::WeeklyTarget::from_minutes(20).unwrap(),
        }
    }

    async fn given_a_committed_task(pool: &SqlitePool, raw_text: &str, tag: Option<&str>) {
        let capture_id = insert_capture(pool, raw_text, tag).await;
        crate::triage::store::insert_task(
            pool,
            capture_id,
            &committed(1787646600000, Commitment::At),
            0,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_committed_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_committed_tasks_reports_a_committed_tasks_text_tag_deadline_and_commitment() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist", Some("@phone")).await;

        let tasks = list_committed_tasks(&pool).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "book the dentist");
        assert_eq!(tasks[0].context_tag.as_deref(), Some("@phone"));
        assert_eq!(tasks[0].deadline, 1787646600000);
        assert_eq!(tasks[0].commitment, "at");
    }

    #[tokio::test]
    async fn list_committed_tasks_reports_an_untagged_committed_task_with_no_tag() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "furnace service window", None).await;
        crate::triage::store::insert_task(
            &pool,
            capture_id,
            &committed(1787922000000, Commitment::At),
            0,
        )
        .await
        .unwrap();

        let tasks = list_committed_tasks(&pool).await.unwrap();

        assert_eq!(tasks[0].context_tag, None);
    }

    #[tokio::test]
    async fn list_committed_tasks_excludes_pool_and_quota_tasks() {
        let (_dir, pool) = test_pool().await;
        let committed_capture = insert_capture(&pool, "book the dentist", Some("@phone")).await;
        let pool_capture = insert_capture(&pool, "buy screws", Some("@homedepot")).await;
        let quota_capture = insert_capture(&pool, "practise piano", Some("@desk")).await;
        crate::triage::store::insert_task(
            &pool,
            committed_capture,
            &committed(1787646600000, Commitment::At),
            0,
        )
        .await
        .unwrap();
        crate::triage::store::insert_task(&pool, pool_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, quota_capture, &quota(), 0)
            .await
            .unwrap();

        let tasks = list_committed_tasks(&pool).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "book the dentist");
    }

    #[tokio::test]
    async fn list_committed_tasks_excludes_an_untriaged_capture() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "book the dentist", Some("@phone")).await;

        assert_eq!(list_committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_committed_tasks_reports_the_tasks_own_id() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist", Some("@phone")).await;

        let tasks = list_committed_tasks(&pool).await.unwrap();

        let task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tasks[0].task_id, task_id);
    }

    #[tokio::test]
    async fn list_committed_tasks_excludes_a_task_marked_done() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist", Some("@phone")).await;
        sqlx::query("UPDATE tasks SET archived_at = 1")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(list_committed_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn task_text_reads_the_capture_it_was_triaged_from() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist", Some("@phone")).await;
        let task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(
            task_text(&pool, task_id).await.unwrap().as_deref(),
            Some("book the dentist")
        );
    }

    #[tokio::test]
    async fn task_text_still_answers_once_the_task_is_archived() {
        let (_dir, pool) = test_pool().await;
        given_a_committed_task(&pool, "book the dentist", None).await;
        let task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE tasks SET archived_at = 1")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            task_text(&pool, task_id).await.unwrap().as_deref(),
            Some("book the dentist")
        );
    }

    #[tokio::test]
    async fn task_text_is_none_for_an_unknown_id() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(task_text(&pool, 999).await.unwrap(), None);
    }

    #[tokio::test]
    async fn task_text_is_none_for_a_task_of_another_kind() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy screws", None).await;
        crate::triage::store::insert_task(&pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        let task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(task_text(&pool, task_id).await.unwrap(), None);
    }
}
