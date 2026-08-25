//! What the pool screen reads: every pool task, with the text and context
//! tag of the capture it came from.
//!
//! `D-no-pool-on-calendar`: this is the *only* place pool work is offered,
//! so this query's `WHERE tasks.kind = 'pool'` is the whole of what keeps
//! committed and quota work off this screen (`pool-screen-only-pool-04`).
//!
//! `AND tasks.cleared_at IS NULL` (#122) is the only exclusion left: a
//! task marked done stays in this list -- struck through, not removed --
//! until its trip's "Clear done" control sweeps it out. `archived_at` alone
//! no longer keeps a row off this screen; see [`PoolTaskRow::done`].

use sqlx::SqlitePool;

/// A row of [`list_pool_tasks`].
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PoolTaskRow {
    pub task_id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
    /// Struck-through (#122): `archived_at IS NOT NULL`. A row this query
    /// returns is never *cleared* -- that is what excludes it -- so `done`
    /// alone is enough for the view to decide open-vs-struck.
    pub done: bool,
}

/// Every pool task not yet cleared, alongside the text and context tag of
/// the capture it was triaged from. Unordered on purpose:
/// `scheduler_core::pool::group` establishes newest-first itself rather
/// than trusting a caller to have sorted already.
///
/// Run sizes are [`run_member_counts`]'s answer, not this one's: a run
/// belongs to a *tag*, and there is no honest way to hang one off a row
/// without inviting a caller to believe two rows could disagree about their
/// shared tag.
pub async fn list_pool_tasks(pool: &SqlitePool) -> Result<Vec<PoolTaskRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT tasks.id AS task_id, captures.raw_text, captures.context_tag, \
         (tasks.archived_at IS NOT NULL) AS done FROM tasks \
         JOIN captures ON captures.id = tasks.capture_id \
         WHERE tasks.kind = 'pool' AND tasks.cleared_at IS NULL",
    )
    .fetch_all(pool)
    .await
}

/// The largest task `id` a run-ending clear has swept away, per tag, or `0`
/// for a tag whose run has never ended (#129). Every member with a larger
/// `id` belongs to the run still open now. Correlated on `c.context_tag`, so
/// it answers for whichever tag the enclosing query is grouping by.
///
/// A clear batch — every task sharing one `cleared_at` — ended the run at
/// its own largest `id` if nothing else at the same tag was still open
/// *at that instant*: created no later than the clear (`created_at_ms <=
/// cleared_at`) and not yet cleared by it (`cleared_at IS NULL` or a *later*
/// clear). Such a survivor is exactly an open item the clear left behind,
/// and [`clear_done`] never clears one, so its presence is what tells a
/// partial clear from a run-ending one. This is the one comparison in this
/// module between a creation instant and a clear instant, and it is a real
/// (if narrow) risk: `platform::clock::Clock` is a plain wall clock with no
/// de-duplication, so a creation landing in the exact same millisecond as a
/// PRIOR clear on the same tag would misread as "already existed then" and
/// the run would fail to reset. Once a batch is judged full, membership
/// itself is counted on `id` -- strictly increasing and tie-free -- rather
/// than repeating the timestamp comparison.
const RUN_BOUNDARY_ID: &str = "COALESCE(( \
     SELECT MAX(b.batch_max_id) FROM ( \
       SELECT MAX(t3.id) AS batch_max_id, t3.cleared_at AS cleared_at \
       FROM tasks t3 JOIN captures c3 ON c3.id = t3.capture_id \
       WHERE c3.context_tag = c.context_tag AND t3.kind = 'pool' \
         AND t3.cleared_at IS NOT NULL \
       GROUP BY t3.cleared_at \
     ) b \
     WHERE NOT EXISTS ( \
       SELECT 1 FROM tasks t4 JOIN captures c4 ON c4.id = t4.capture_id \
       WHERE c4.context_tag = c.context_tag AND t4.kind = 'pool' \
         AND t4.created_at_ms <= b.cleared_at \
         AND (t4.cleared_at IS NULL OR t4.cleared_at > b.cleared_at) \
     ) \
   ), 0)";

/// How many pool tasks belong to each tag's current *run* -- everything
/// created since that tag's run last ended, cleared or not (#129,
/// `D-a-trip-survives-being-tidied`). Equal to the tag's concurrently-
/// uncleared count (what [`list_pool_tasks`] alone would show) until a clear
/// removes a member without ending the run; larger than it from that point
/// on, since the removed member still belonged to the run.
///
/// One query for every tag at once, and the only one: a tag whose run is
/// empty is simply absent from the result, which is the same answer as zero
/// to `scheduler_core::pool::RunSizes`'s only question. A second,
/// single-tag variant of this rule would be a second place it could drift.
pub async fn run_member_counts(pool: &SqlitePool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let sql = format!(
        "SELECT c.context_tag AS tag, COUNT(*) AS member_count \
         FROM tasks t JOIN captures c ON c.id = t.capture_id \
         WHERE t.kind = 'pool' AND c.context_tag IS NOT NULL \
           AND t.id > {RUN_BOUNDARY_ID} \
         GROUP BY c.context_tag"
    );
    sqlx::query_as(&sql).fetch_all(pool).await
}

/// Sweeps every struck-through (`archived_at IS NOT NULL`), not-yet-cleared
/// pool task at `tag` off the screen (#122's "Clear done") -- scoped to the
/// tag rather than a caller-supplied id list, since the control acts on a
/// whole trip at once and the row never carries its own "still open" flag
/// for a caller to check first. Never touches an open task: the `WHERE`
/// makes clearing a no-op on anything not already done, the same shape the
/// mark-done write's own guard takes.
pub async fn clear_done(
    pool: &SqlitePool,
    tag: &str,
    cleared_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE tasks SET cleared_at = ? \
         WHERE kind = 'pool' AND archived_at IS NOT NULL AND cleared_at IS NULL \
         AND capture_id IN (SELECT id FROM captures WHERE context_tag = ?)",
    )
    .bind(cleared_at_ms)
    .bind(tag)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use scheduler_core::task::TaskKind;

    fn committed_task_kind() -> TaskKind {
        TaskKind::Committed {
            deadline: 1787245200000,
            commitment: scheduler_core::task::Commitment::At,
            priority: scheduler_core::task::Priority::P1,
            estimated_minutes: 30,
        }
    }

    fn quota_task_kind() -> TaskKind {
        TaskKind::Quota {
            target_count: 3,
            target_minutes_each: 20,
            period: scheduler_core::task::Period::Week,
        }
    }

    async fn given_a_pool_task(
        pool: &SqlitePool,
        raw_text: &str,
        tag: Option<&str>,
    ) -> Vec<PoolTaskRow> {
        let capture_id = insert_capture(pool, raw_text, tag).await;
        crate::triage::store::insert_task(pool, capture_id, &TaskKind::Pool, 0)
            .await
            .unwrap();
        list_pool_tasks(pool).await.unwrap()
    }

    async fn mark_archived(pool: &SqlitePool, task_id: i64) {
        sqlx::query("UPDATE tasks SET archived_at = 1 WHERE id = ?")
            .bind(task_id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_pool_tasks_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_a_pool_tasks_text_and_tag() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "buy screws");
        assert_eq!(tasks[0].context_tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_an_untagged_pool_task_with_no_tag() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "fix the door latch", None).await;

        assert_eq!(tasks[0].context_tag, None);
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_committed_and_quota_tasks() {
        let (_dir, pool) = test_pool().await;
        let pool_capture = insert_capture(&pool, "buy screws", Some("@homedepot")).await;
        let committed_capture = insert_capture(&pool, "file the return", Some("@homedepot")).await;
        let quota_capture = insert_capture(&pool, "practise piano", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, pool_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, committed_capture, &committed_task_kind(), 0)
            .await
            .unwrap();
        crate::triage::store::insert_task(&pool, quota_capture, &quota_task_kind(), 0)
            .await
            .unwrap();

        let tasks = list_pool_tasks(&pool).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].raw_text, "buy screws");
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_an_untriaged_capture() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy screws", Some("@homedepot")).await;

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_pool_tasks_still_reports_a_task_marked_done_but_not_cleared() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        mark_archived(&pool, task_id).await;

        let tasks = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].done);
    }

    #[tokio::test]
    async fn list_pool_tasks_reports_an_open_task_as_not_done() {
        let (_dir, pool) = test_pool().await;

        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;

        assert!(!tasks[0].done);
    }

    #[tokio::test]
    async fn list_pool_tasks_excludes_a_cleared_task() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        sqlx::query("UPDATE tasks SET archived_at = 1, cleared_at = 2 WHERE id = ?")
            .bind(task_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn clear_done_stamps_cleared_at_on_a_done_task_at_the_named_tag() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        mark_archived(&pool, task_id).await;

        clear_done(&pool, "@homedepot", 42).await.unwrap();

        assert_eq!(list_pool_tasks(&pool).await.unwrap(), Vec::new());
        let cleared_at: Option<i64> =
            sqlx::query_scalar("SELECT cleared_at FROM tasks WHERE id = ?")
                .bind(task_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cleared_at, Some(42));
    }

    #[tokio::test]
    async fn clear_done_leaves_an_open_task_at_the_same_tag_alone() {
        let (_dir, pool) = test_pool().await;
        let done_capture = insert_capture(&pool, "buy screws", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, done_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        let open_capture = insert_capture(&pool, "return the drill", Some("@homedepot")).await;
        crate::triage::store::insert_task(&pool, open_capture, &TaskKind::Pool, 0)
            .await
            .unwrap();
        let done_task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
            .bind(done_capture)
            .fetch_one(&pool)
            .await
            .unwrap();
        mark_archived(&pool, done_task_id).await;

        clear_done(&pool, "@homedepot", 42).await.unwrap();

        let remaining = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].raw_text, "return the drill");
        assert!(!remaining[0].done);
    }

    #[tokio::test]
    async fn clear_done_leaves_a_done_task_at_a_different_tag_alone() {
        let (_dir, pool) = test_pool().await;
        let tasks = given_a_pool_task(&pool, "buy screws", Some("@homedepot")).await;
        let task_id = tasks[0].task_id;
        mark_archived(&pool, task_id).await;

        clear_done(&pool, "@supermarket", 42).await.unwrap();

        let remaining = list_pool_tasks(&pool).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].done);
    }

    // --- #129: a trip survives being tidied -----------------------------

    /// `n` pool tasks tagged `tag`, created at consecutive milliseconds from
    /// `start_ms` -- distinct, ordered timestamps, since [`run_member_counts`]
    /// compares creation instants to clear instants and the real `Clock`
    /// this stands in for would never hand out the same millisecond twice
    /// for tasks created this deliberately apart.
    async fn given_pool_tasks_at(
        pool: &SqlitePool,
        tag: &str,
        start_ms: i64,
        n: usize,
    ) -> Vec<i64> {
        let mut ids = Vec::new();
        for i in 0..n {
            let capture_id = insert_capture(pool, &format!("{tag} errand {i}"), Some(tag)).await;
            crate::triage::store::insert_task(
                pool,
                capture_id,
                &TaskKind::Pool,
                start_ms + i as i64,
            )
            .await
            .unwrap();
            let task_id: i64 = sqlx::query_scalar("SELECT id FROM tasks WHERE capture_id = ?")
                .bind(capture_id)
                .fetch_one(pool)
                .await
                .unwrap();
            ids.push(task_id);
        }
        ids
    }

    async fn given_pool_tasks(pool: &SqlitePool, tag: &str, n: usize) -> Vec<i64> {
        given_pool_tasks_at(pool, tag, 1, n).await
    }

    async fn mark_archived_ids(pool: &SqlitePool, ids: &[i64]) {
        for id in ids {
            mark_archived(pool, *id).await;
        }
    }

    /// One tag's run size, read off the production query rather than a
    /// second one written for tests: absent means a run of zero, which is
    /// exactly what `RunSizes` makes of it.
    async fn run_members(pool: &SqlitePool, tag: &str) -> i64 {
        run_member_counts(pool)
            .await
            .unwrap()
            .into_iter()
            .find(|(t, _)| t == tag)
            .map_or(0, |(_, members)| members)
    }

    #[tokio::test]
    async fn run_members_equals_the_uncleared_count_when_the_run_has_never_been_cleared() {
        let (_dir, pool) = test_pool().await;
        given_pool_tasks(&pool, "@homedepot", 3).await;

        assert_eq!(run_members(&pool, "@homedepot").await, 3);
    }

    #[tokio::test]
    async fn run_members_is_zero_for_a_tag_with_no_pool_tasks() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(run_members(&pool, "@homedepot").await, 0);
    }

    /// A partial clear -- one that leaves an open item behind -- does not
    /// end the run (#129): the two swept members still belong to it.
    #[tokio::test]
    async fn run_members_includes_members_a_partial_clear_swept_away() {
        let (_dir, pool) = test_pool().await;
        let ids = given_pool_tasks(&pool, "@homedepot", 5).await;
        mark_archived_ids(&pool, &ids[0..3]).await;

        clear_done(&pool, "@homedepot", 100).await.unwrap();

        assert_eq!(run_members(&pool, "@homedepot").await, 5);
    }

    /// A clear that leaves nothing open behind ends the run: a fresh task
    /// afterward starts a new one and does not inherit the old members
    /// (`trip-persistence-re-earns-its-trip-03`).
    #[tokio::test]
    async fn run_members_resets_once_a_clear_leaves_nothing_open() {
        let (_dir, pool) = test_pool().await;
        let ids = given_pool_tasks(&pool, "@homedepot", 3).await;
        mark_archived_ids(&pool, &ids).await;
        clear_done(&pool, "@homedepot", 100).await.unwrap();

        given_pool_tasks_at(&pool, "@homedepot", 200, 2).await;

        assert_eq!(run_members(&pool, "@homedepot").await, 2);
    }

    /// Two separate partial clears in the same still-open run both count
    /// toward it -- the derivation must not stop at the first clear batch it
    /// finds.
    #[tokio::test]
    async fn run_members_accumulates_across_multiple_partial_clears() {
        let (_dir, pool) = test_pool().await;
        let ids = given_pool_tasks(&pool, "@homedepot", 6).await;
        mark_archived_ids(&pool, &ids[0..2]).await;
        clear_done(&pool, "@homedepot", 100).await.unwrap();
        mark_archived_ids(&pool, &ids[2..4]).await;
        clear_done(&pool, "@homedepot", 200).await.unwrap();

        assert_eq!(run_members(&pool, "@homedepot").await, 6);
    }

    #[tokio::test]
    async fn run_members_is_scoped_to_its_own_tag() {
        let (_dir, pool) = test_pool().await;
        given_pool_tasks(&pool, "@homedepot", 5).await;
        given_pool_tasks(&pool, "@supermarket", 2).await;

        assert_eq!(run_members(&pool, "@supermarket").await, 2);
    }

    /// The two facts the pool screen reads are read separately and stay
    /// consistent: a partial clear leaves two rows and a run of five.
    #[tokio::test]
    async fn a_partial_clear_leaves_fewer_rows_than_its_tags_run_has_members() {
        let (_dir, pool) = test_pool().await;
        let ids = given_pool_tasks(&pool, "@homedepot", 5).await;
        mark_archived_ids(&pool, &ids[0..3]).await;
        clear_done(&pool, "@homedepot", 100).await.unwrap();

        assert_eq!(list_pool_tasks(&pool).await.unwrap().len(), 2);
        assert_eq!(run_members(&pool, "@homedepot").await, 5);
    }

    /// An untagged task has no tag to have a run, so it contributes to no
    /// entry at all rather than to an empty-string one.
    #[tokio::test]
    async fn run_member_counts_reports_nothing_for_an_untagged_task() {
        let (_dir, pool) = test_pool().await;
        given_a_pool_task(&pool, "fix the door latch", None).await;

        assert_eq!(run_member_counts(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn run_member_counts_reports_every_tag_with_a_live_run_at_once() {
        let (_dir, pool) = test_pool().await;
        given_pool_tasks(&pool, "@homedepot", 5).await;
        given_pool_tasks_at(&pool, "@supermarket", 100, 2).await;

        let mut counts = run_member_counts(&pool).await.unwrap();
        counts.sort();

        assert_eq!(
            counts,
            vec![
                ("@homedepot".to_string(), 5),
                ("@supermarket".to_string(), 2)
            ]
        );
    }
}
