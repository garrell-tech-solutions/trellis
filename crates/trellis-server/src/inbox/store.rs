//! What the page reads: `Recent` — the untriaged queue, and the last few
//! captures triage has taken out of it (#140).
//!
//! Both are the inbox's queries even though neither table is the inbox's
//! alone — a capability owns the SQL it issues. Which columns these select
//! is this module's business; what a page does with them is not
//! (`T-templates-take-view-models`, and see [`super::view`]).

use super::shown_kind::ShownKind;
use sqlx::SqlitePool;

/// A row of [`list_recent`]: either a still-untriaged capture (`kind` is
/// `None`) or one of the three most recently triaged (`kind` names what it
/// became).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RecentCapture {
    pub id: i64,
    pub raw_text: String,
    pub context_tag: Option<String>,
    pub shown_kind: Option<String>,
    pub kind: Option<String>,
}

/// Every untriaged capture, plus the three most recently triaged, in one
/// strict-recency order (#140, `D-four-screens`: the flat `Tasks` list this
/// used to live in is gone, and a capture that has just been triaged has
/// nowhere else on this page to show it went somewhere).
///
/// The untriaged half never ages out
/// (`inbox-view-untriaged-never-drop-06`) — only the triaged half is
/// bounded, so confirming old work can never crowd out something still
/// waiting. The bound is this query's own `LIMIT`, not a slice a caller
/// takes afterward (`T-set-operations-execute-in-the-store`).
pub async fn list_recent(pool: &SqlitePool) -> Result<Vec<RecentCapture>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, raw_text, context_tag, shown_kind, NULL AS kind, created_at_ms AS sort_ms \
         FROM captures WHERE left_inbox_at IS NULL \
         UNION ALL \
         SELECT captures.id, captures.raw_text, captures.context_tag, NULL AS shown_kind, \
                recent.kind, recent.created_at_ms AS sort_ms \
         FROM (SELECT id, capture_id, kind, created_at_ms FROM tasks \
               ORDER BY created_at_ms DESC, id DESC LIMIT 3) AS recent \
         JOIN captures ON captures.id = recent.capture_id \
         ORDER BY sort_ms DESC, id DESC",
    )
    .fetch_all(pool)
    .await
}

/// Records which kind's panel `capture_id`'s row should show (#119) —
/// presentation only, never read by triage validation.
///
/// Takes a [`ShownKind`] rather than a `&str`: the column's own
/// `CHECK` enforces the same closed domain, and a signature that could
/// carry anything left the "caller has already checked" contract to a
/// comment. Now the only way to reach this function is to have parsed.
pub(super) async fn set_shown_kind(
    pool: &SqlitePool,
    capture_id: i64,
    kind: ShownKind,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE captures SET shown_kind = ? WHERE id = ? AND left_inbox_at IS NULL")
        .bind(kind.as_str())
        .bind(capture_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// [`list_recent`]'s untriaged `WHERE` clause asked about one row: is this capture
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{insert_capture, test_pool};
    use crate::triage::store::insert_task;
    use proptest::prelude::*;
    use scheduler_core::task::TaskKind;

    #[tokio::test]
    async fn a_freshly_inserted_capture_shows_no_kind() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk", None).await;

        let captures = list_recent(&pool).await.unwrap();

        assert_eq!(captures[0].shown_kind, None);
    }

    #[tokio::test]
    async fn set_shown_kind_records_which_kind_the_row_shows() {
        let (_dir, pool) = test_pool().await;
        let id = insert_capture(&pool, "buy milk", None).await;

        set_shown_kind(&pool, id, ShownKind::Committed)
            .await
            .unwrap();

        let captures = list_recent(&pool).await.unwrap();
        assert_eq!(captures[0].shown_kind.as_deref(), Some("committed"));
    }

    #[tokio::test]
    async fn set_shown_kind_replaces_a_prior_choice_rather_than_adding_to_it() {
        let (_dir, pool) = test_pool().await;
        let id = insert_capture(&pool, "buy milk", None).await;
        set_shown_kind(&pool, id, ShownKind::Committed)
            .await
            .unwrap();

        set_shown_kind(&pool, id, ShownKind::Quota).await.unwrap();

        let captures = list_recent(&pool).await.unwrap();
        assert_eq!(captures[0].shown_kind.as_deref(), Some("quota"));
    }

    #[tokio::test]
    async fn set_shown_kind_leaves_other_captures_alone() {
        let (_dir, pool) = test_pool().await;
        let chosen = insert_capture(&pool, "buy milk", None).await;
        let other = insert_capture(&pool, "call the dentist", None).await;

        set_shown_kind(&pool, chosen, ShownKind::Committed)
            .await
            .unwrap();

        let captures = list_recent(&pool).await.unwrap();
        let other_row = captures.iter().find(|c| c.id == other).unwrap();
        assert_eq!(other_row.shown_kind, None);
    }

    #[tokio::test]
    async fn list_recent_lists_untriaged_captures_newest_first() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "call the dentist", None).await;
        insert_capture(&pool, "buy milk", None).await;

        let captures = list_recent(&pool).await.unwrap();

        assert_eq!(
            captures
                .iter()
                .map(|c| c.raw_text.as_str())
                .collect::<Vec<_>>(),
            vec!["buy milk", "call the dentist"]
        );
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
    async fn list_recent_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_recent(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn list_recent_reports_an_untriaged_capture_with_no_kind() {
        let (_dir, pool) = test_pool().await;
        insert_capture(&pool, "buy milk", None).await;

        let recent = list_recent(&pool).await.unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].raw_text, "buy milk");
        assert_eq!(recent[0].kind, None);
    }

    /// A real triage is two writes: the task, then closing the capture
    /// (`inbox::close_capture`) — [`insert_task`] alone (as most of this
    /// file's other fixtures use it, to test the task-side columns in
    /// isolation) leaves `left_inbox_at` `NULL`, which would double-count
    /// the capture as both still-untriaged and triaged here.
    async fn given_triaged(pool: &SqlitePool, raw_text: &str, created_at_ms: i64) -> i64 {
        let capture_id = insert_capture(pool, raw_text, None).await;
        insert_task(pool, capture_id, &TaskKind::Pool, created_at_ms)
            .await
            .unwrap();
        close_capture(pool, capture_id, created_at_ms)
            .await
            .unwrap();
        capture_id
    }

    #[tokio::test]
    async fn list_recent_reports_a_triaged_captures_kind_and_context_tag() {
        let (_dir, pool) = test_pool().await;
        let (capture_id, _) =
            crate::capture::create(&pool, "buy screws", "web", Some("@homedepot"), 0)
                .await
                .unwrap();
        insert_task(&pool, capture_id, &TaskKind::Pool, 7)
            .await
            .unwrap();
        close_capture(&pool, capture_id, 7).await.unwrap();

        let recent = list_recent(&pool).await.unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].raw_text, "buy screws");
        assert_eq!(recent[0].kind.as_deref(), Some("pool"));
        assert_eq!(recent[0].context_tag.as_deref(), Some("@homedepot"));
    }

    /// [`list_recent`], reduced to the recency order itself -- what every
    /// test below this point checks, none of the other fields.
    async fn recent_texts(pool: &SqlitePool) -> Vec<String> {
        list_recent(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.raw_text)
            .collect()
    }

    #[tokio::test]
    async fn list_recent_orders_by_strict_recency_across_both_kinds() {
        let (_dir, pool) = test_pool().await;
        given_triaged(&pool, "call the dentist", 1).await;
        crate::capture::store::insert(&pool, "buy milk", "web", None, 2)
            .await
            .unwrap();

        assert_eq!(
            recent_texts(&pool).await,
            vec!["buy milk", "call the dentist"],
            "the untriaged capture happened most recently and lists first"
        );
    }

    #[tokio::test]
    async fn list_recent_keeps_only_the_three_most_recently_triaged() {
        let (_dir, pool) = test_pool().await;
        for (index, text) in ["one", "two", "three", "four"].into_iter().enumerate() {
            given_triaged(&pool, text, index as i64).await;
        }

        assert_eq!(
            recent_texts(&pool).await,
            vec!["four", "three", "two"],
            "the oldest triaged capture drops off"
        );
    }

    #[tokio::test]
    async fn list_recent_never_drops_an_untriaged_capture_for_the_triaged_cap() {
        let (_dir, pool) = test_pool().await;
        for (index, text) in ["one", "two", "three", "four"].into_iter().enumerate() {
            given_triaged(&pool, text, index as i64).await;
        }
        crate::capture::store::insert(&pool, "still waiting", "web", None, 10)
            .await
            .unwrap();

        assert_eq!(
            recent_texts(&pool).await,
            vec!["still waiting", "four", "three", "two"]
        );
    }

    #[tokio::test]
    async fn list_recent_excludes_a_dismissed_capture() {
        let (_dir, pool) = test_pool().await;
        let dismissed = insert_capture(&pool, "asdfgh", None).await;
        close_capture(&pool, dismissed, 9999).await.unwrap();

        assert_eq!(list_recent(&pool).await.unwrap(), Vec::new());
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        /// **The untriaged half of the listing is still a partition.** Exactly
        /// the untriaged captures, in exactly newest-first order, whichever of
        /// the inbox's two exits (triage or dismissal,
        /// `dismiss-capture-keeps-the-row-03`, #48) the others took. One
        /// property covers both exits because both close the capture the same
        /// way, and they are told apart here by what actually distinguishes
        /// them — a triage leaves a `tasks` row behind, a dismissal leaves
        /// none.
        ///
        /// **This used to be asserted about `list_untriaged`, which #140 left
        /// with no caller.** `list_recent` is what the page reads now, and it
        /// is the harder query to get right: a `UNION ALL` whose two halves
        /// have different rules. Restating the property against it is not a
        /// port — the claim gained a clause. *Untriaged* is no longer the
        /// whole listing, so the property has to say which rows it is talking
        /// about, and the way it says so is
        /// `inbox-view-untriaged-never-drop-06`: **the untriaged half is
        /// never bounded.** A `LIMIT` that leaked from the triaged half onto
        /// the whole query would still pass every example test with three or
        /// fewer of them.
        ///
        /// Also pins `#9` AC-4's row-count property in the same run: the total
        /// row count never moves, for any queue and any split of it across
        /// untriaged, triaged and dismissed — a capture row is never deleted
        /// by either exit.
        #[test]
        #[ignore]
        fn list_recent_carries_exactly_the_untriaged_captures_newest_first(
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

                // The untriaged half is the rows carrying no kind; the
                // triaged half is bounded and has its own property below.
                let listed: Vec<String> = list_recent(&pool)
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|capture| capture.kind.is_none())
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

        /// **The triaged half is bounded, and bounded at three, however many
        /// have been triaged.** The `LIMIT 3` is the only thing standing
        /// between this page and the wall of every task ever triaged that
        /// #140 removed, and an example test can only pin it at the sizes it
        /// happens to name. Off-by-one either way survives a fixture of
        /// exactly four.
        ///
        /// It also pins the half the bound must *not* reach: every untriaged
        /// capture is still listed, at any queue size
        /// (`inbox-view-untriaged-never-drop-06`).
        #[test]
        #[ignore]
        fn the_triaged_half_never_exceeds_three_however_many_have_been_triaged(
            untriaged in 0usize..6,
            triaged in 0usize..9,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (triaged_listed, untriaged_listed) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;

                for n in 0..triaged {
                    let id = insert_capture(&pool, &format!("triaged {n}"), None).await;
                    insert_task(&pool, id, &TaskKind::Pool, n as i64).await.unwrap();
                    close_capture(&pool, id, n as i64).await.unwrap();
                }
                for n in 0..untriaged {
                    insert_capture(&pool, &format!("waiting {n}"), None).await;
                }

                let recent = list_recent(&pool).await.unwrap();
                let triaged_listed = recent.iter().filter(|c| c.kind.is_some()).count();
                let untriaged_listed = recent.iter().filter(|c| c.kind.is_none()).count();
                (triaged_listed, untriaged_listed)
            });

            prop_assert_eq!(triaged_listed, triaged.min(3));
            prop_assert_eq!(untriaged_listed, untriaged);
        }
    }
}
