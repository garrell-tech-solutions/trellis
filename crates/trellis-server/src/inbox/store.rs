//! What the page reads: the untriaged queue, and the tasks triage has
//! produced.
//!
//! Both are the inbox's queries even though neither table is the inbox's
//! alone — a capability owns the SQL it issues. Which columns these select
//! is this module's business; what a page does with them is not
//! (`T-templates-take-view-models`, and see [`super::view`]).

use sqlx::SqlitePool;

/// A row of [`list_untriaged`].
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UntriagedCapture {
    pub id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
}

/// Untriaged captures, newest first — the inbox's contents.
pub async fn list_untriaged(pool: &SqlitePool) -> Result<Vec<UntriagedCapture>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, raw_text, context_tag FROM captures WHERE left_inbox_at IS NULL ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await
}

/// [`list_untriaged`]'s `WHERE` clause asked about one row: is this capture
/// still in the inbox? An id naming no capture answers no, which is the same
/// answer a caller wants for it.
///
/// `pub(super)` on purpose. Both this and [`close_capture`] are reached
/// through [`super::capture_is_open`] and [`super::close_capture`], and the
/// visibility is what makes that a rule rather than a request
/// (`T-one-front-door-per-capability`).
pub(super) async fn capture_is_open(
    pool: &SqlitePool,
    capture_id: i64,
) -> Result<bool, sqlx::Error> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ? AND left_inbox_at IS NULL")
            .bind(capture_id)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

/// Takes `capture_id` out of the inbox, recording when it left. The row is
/// never deleted (`D-kill-means-archive`); this is the whole of what leaving
/// means.
///
/// Guarded by `left_inbox_at IS NULL` so that two exits racing cannot
/// overwrite each other's stamp — whichever arrives first is the one that
/// sticks, the same defence-in-depth `life_areas::store::archive` gives
/// archiving. The caller's eligibility check is what produces a rejection
/// message; this guard is what keeps the write honest without one.
pub(super) async fn close_capture(
    pool: &SqlitePool,
    capture_id: i64,
    left_inbox_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE captures SET left_inbox_at = ? WHERE id = ? AND left_inbox_at IS NULL")
        .bind(left_inbox_at_ms)
        .bind(capture_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// A row of [`list_tasks`]: a task, alongside the text and context tag of
/// the capture it was triaged from — the task list's own rows have neither
/// of their own to show, so both are this query's business, not the page's.
/// `context_tag` comes through the join rather than a column of its own
/// (`context-tags-survives-triage-07`'s "one fact, one row": the tag lives
/// on `captures`, and a task reads it, never copies it).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct TaskWithCaptureText {
    pub kind: String,
    pub raw_text: String,
    pub context_tag: Option<String>,
}

/// Every task, newest first.
pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskWithCaptureText>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.kind, captures.raw_text, captures.context_tag FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         ORDER BY tasks.id DESC",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use crate::triage::store::insert_task;
    use proptest::prelude::*;
    use scheduler_core::task::TaskKind;

    #[tokio::test]
    async fn list_untriaged_reports_each_captures_id() {
        let (_dir, pool) = test_pool().await;
        let id = insert_capture(&pool, "buy milk", None).await;

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].id, id);
    }

    #[tokio::test]
    async fn list_untriaged_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_untriaged(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_untriaged_lists_captures_newest_first() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "call the dentist", None).await;
        insert_capture(&pool, "buy milk", None).await;

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(
            captures
                .iter()
                .map(|c| c.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["buy milk", "call the dentist"]
        );
    }

    #[tokio::test]
    async fn list_untriaged_excludes_a_capture_that_has_left_the_inbox() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk", None).await;
        let gone = insert_capture(&pool, "call the dentist", None).await;
        close_capture(&pool, gone, 9999).await.unwrap();

        let captures = list_untriaged(&pool).await.unwrap();

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].raw_text, "buy milk");
    }

    #[tokio::test]
    async fn capture_is_open_is_true_for_a_freshly_inserted_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        assert!(capture_is_open(&pool, capture_id).await.unwrap());
    }

    #[tokio::test]
    async fn capture_is_open_is_false_once_it_has_left_the_inbox() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;
        close_capture(&pool, capture_id, 9999).await.unwrap();

        assert!(!capture_is_open(&pool, capture_id).await.unwrap());
    }

    #[tokio::test]
    async fn capture_is_open_is_false_for_an_id_naming_no_capture() {
        let (_dir, pool) = test_pool().await;

        assert!(!capture_is_open(&pool, 999).await.unwrap());
    }

    #[tokio::test]
    async fn close_capture_stamps_the_named_capture() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        close_capture(&pool, capture_id, 4242).await.unwrap();

        assert_eq!(left_inbox_at(&pool, capture_id).await, Some(4242));
    }

    #[tokio::test]
    async fn close_capture_leaves_other_captures_in_the_inbox() {
        let (_dir, pool) = test_pool().await;
        let closed = insert_capture(&pool, "buy milk", None).await;
        let untouched = insert_capture(&pool, "call the dentist", None).await;

        close_capture(&pool, closed, 9999).await.unwrap();

        assert_eq!(left_inbox_at(&pool, untouched).await, None);
    }

    #[tokio::test]
    async fn close_capture_does_not_delete_the_row() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        close_capture(&pool, capture_id, 9999).await.unwrap();

        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_count, 1);
    }

    /// The guard, stated from the side that matters: a second exit arriving
    /// after the first does not move the stamp the first one wrote.
    #[tokio::test]
    async fn close_capture_is_a_noop_once_the_capture_has_already_left() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;
        close_capture(&pool, capture_id, 1111).await.unwrap();

        close_capture(&pool, capture_id, 2222).await.unwrap();

        assert_eq!(left_inbox_at(&pool, capture_id).await, Some(1111));
    }

    async fn left_inbox_at(pool: &SqlitePool, capture_id: i64) -> Option<i64> {
        sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn list_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_tasks_reports_each_tasks_kind_and_the_text_of_the_capture_it_came_from() {
        let (_dir, pool) = test_pool().await;
        let capture_id = insert_capture(&pool, "buy milk", None).await;

        insert_task(&pool, capture_id, &TaskKind::Pool, 7)
            .await
            .unwrap();

        assert_eq!(
            list_tasks(&pool).await.unwrap(),
            vec![TaskWithCaptureText {
                kind: "pool".to_string(),
                raw_text: "buy milk".to_string(),
                context_tag: None,
            }]
        );
    }

    #[tokio::test]
    async fn list_tasks_reports_the_context_tag_of_the_capture_it_came_from() {
        let (_dir, pool) = test_pool().await;
        let (capture_id, _) =
            crate::capture::create(&pool, "buy screws", "web", Some("@homedepot"), 0)
                .await
                .unwrap();

        insert_task(&pool, capture_id, &TaskKind::Pool, 7)
            .await
            .unwrap();

        let tasks = list_tasks(&pool).await.unwrap();
        assert_eq!(tasks[0].context_tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn list_tasks_lists_tasks_newest_first() {
        let (_dir, pool) = test_pool().await;
        let first_capture = insert_capture(&pool, "buy milk", None).await;
        let second_capture = insert_capture(&pool, "call the dentist", None).await;

        insert_task(&pool, first_capture, &TaskKind::Pool, 1)
            .await
            .unwrap();
        insert_task(&pool, second_capture, &TaskKind::Pool, 2)
            .await
            .unwrap();

        let tasks = list_tasks(&pool).await.unwrap();
        assert_eq!(
            tasks
                .iter()
                .map(|t| t.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["call the dentist", "buy milk"]
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// The listing is a *partition*: exactly the untriaged captures, in
        /// exactly newest-first order, whichever of the inbox's two exits
        /// (triage or dismissal, `dismiss-capture-keeps-the-row-03`, #48) a
        /// capture took. Extended to cover dismissal here rather than as a
        /// second property (the brief's open question 2): both exits close
        /// the capture the same way, so one property already exercises both
        /// `WHERE` clauses this query could get wrong.
        ///
        /// The two exits are told apart by what actually distinguishes them —
        /// a triage leaves a `tasks` row behind, a dismissal leaves none —
        /// rather than by which module wrote the stamp, which is now one
        /// function for both. That is the stronger statement anyway: a
        /// capture that has become a task and one that was thrown away are
        /// equally gone from this listing, and neither disturbs the row
        /// count.
        ///
        /// Also pins `#9` AC-4's row-count property in the same run: the
        /// total row count never moves, for any queue and any split of it
        /// across untriaged, triaged and dismissed — a capture row is never
        /// deleted by either exit.
        #[test]
        #[ignore]
        fn list_untriaged_returns_exactly_the_untriaged_captures_newest_first(
            queue in prop::collection::vec((".{0,40}", 0..3u8), 0..12),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (expected, listed, row_count) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;

                let mut expected: Vec<String> = Vec::new();
                for (raw_text, exit) in &queue {
                    let id = insert_capture(&pool, raw_text, None).await;
                    match exit {
                        1 => {
                            insert_task(&pool, id, &TaskKind::Pool, 9999).await.unwrap();
                            close_capture(&pool, id, 9999).await.unwrap();
                        }
                        2 => close_capture(&pool, id, 9999).await.unwrap(),
                        _ => expected.push(raw_text.clone()),
                    }
                }
                expected.reverse();

                let listed: Vec<String> = list_untriaged(&pool)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|capture| capture.raw_text)
                    .collect();
                let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM captures")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                (expected, listed, row_count)
            });

            prop_assert_eq!(expected, listed);
            prop_assert_eq!(row_count as usize, queue.len());
        }
    }
}
