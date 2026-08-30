//! What the quota screen reads and writes: the `quotas` table (#93,
//! `D-quotas-are-selected-not-typed`) -- a different table from `tasks`,
//! which `TaskKind::Quota` keeps using untouched.

use scheduler_core::quota::{QuotaDefinition, WeeklyTarget};
use sqlx::SqlitePool;

/// A row of [`list_quotas`], in the order they were defined
/// (`quota-screen-defined-order-08`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaRow {
    pub id: i64,
    pub name: String,
    /// Already a [`WeeklyTarget`] rather than a bare count of minutes: the
    /// column's `CHECK (weekly_target_minutes > 0)` says the same thing the
    /// type does, and this is where the two meet, so nothing downstream has
    /// to re-establish it.
    pub weekly_target: WeeklyTarget,
}

/// The row exactly as the column set reads. Private: what leaves this
/// module is [`QuotaRow`], whose target has been through the domain type.
#[derive(sqlx::FromRow)]
struct StoredQuotaRow {
    id: i64,
    name: String,
    weekly_target_minutes: i64,
}

/// Every quota not yet removed, oldest first -- the store's own `ORDER BY`
/// (`T-set-operations-execute-in-the-store`), not a sort a caller performs
/// after fetching. `archived_at IS NULL` is the whole of what "removed"
/// means (#148, `D-kill-means-archive`'s bargain applied to a quota): the
/// row and its sessions stay, this screen just stops reading it.
///
/// A row whose target is not positive has broken its own `CHECK`, so this
/// reports it as what it is -- a value that cannot be decoded into the type
/// the row declares -- rather than passing a zero on for
/// `scheduler_core::quota::progress` to divide by. The handler already
/// turns a store error into a 500, which is the honest answer to a corrupt
/// row; nothing here can render it.
pub async fn list_quotas(pool: &SqlitePool) -> Result<Vec<QuotaRow>, sqlx::Error> {
    let stored: Vec<StoredQuotaRow> = sqlx::query_as(
        "SELECT id, name, weekly_target_minutes FROM quotas \
         WHERE archived_at IS NULL ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;
    stored.into_iter().map(quota_row).collect()
}

fn quota_row(stored: StoredQuotaRow) -> Result<QuotaRow, sqlx::Error> {
    let weekly_target =
        WeeklyTarget::from_minutes(stored.weekly_target_minutes).ok_or_else(|| {
            sqlx::Error::Decode(
                format!(
                    "quota {} has a weekly target of {} minutes, which its own CHECK forbids",
                    stored.id, stored.weekly_target_minutes
                )
                .into(),
            )
        })?;
    Ok(QuotaRow {
        id: stored.id,
        name: stored.name,
        weekly_target,
    })
}

/// One row of [`existing`]: a quota's own spelling, target and whether it
/// has been removed.
#[derive(sqlx::FromRow)]
pub(super) struct ExistingQuota {
    pub name: String,
    pub weekly_target_minutes: i64,
    /// #148: an archived quota still counts as "taken" -- its name is
    /// still sitting in the `UNIQUE COLLATE NOCASE` column -- so
    /// [`super::name_standing`] weighs it exactly like a live one. Whether
    /// *that* is a refusal or a revival is the caller's decision, not this
    /// query's.
    pub archived: bool,
}

/// Every quota that has ever existed, live or removed, for
/// `scheduler_core::quota::check_name` to compare a candidate name against
/// before writing it -- `T-collation-enforces-name-identity`'s rule must
/// see the same rows the `UNIQUE` column itself would refuse a duplicate
/// against, and that column does not stop enforcing itself once a quota is
/// archived.
///
/// Reachable only from this capability: the question "where does this name
/// stand?" leaves through [`super::name_standing`], which is the whole of
/// what any other capability may ask (`T-one-front-door-per-capability`,
/// and the company standard's *opaque retrieval* -- how the comparison is
/// reached is nobody else's business). That visibility is also what keeps
/// the Gaps entry under this one function: the day the exact tier moves
/// into the query, no caller changes.
///
/// `exclude` is the one row a rename must not measure itself against -- its
/// own (#148: renaming `workout` to `Workout` is a respelling, not a
/// collision). It is a `WHERE`, not a filter the caller applies afterwards
/// (`T-set-operations-execute-in-the-store`); the tier that cannot execute
/// here is the similarity comparison, and that is no reason to hand back a
/// row the question has already excluded.
pub(super) async fn existing(
    pool: &SqlitePool,
    exclude: Option<i64>,
) -> Result<Vec<ExistingQuota>, sqlx::Error> {
    sqlx::query_as(
        "SELECT name, weekly_target_minutes, (archived_at IS NOT NULL) AS archived \
         FROM quotas WHERE (? IS NULL OR id <> ?) ORDER BY id ASC",
    )
    .bind(exclude)
    .bind(exclude)
    .fetch_all(pool)
    .await
}

/// Writes a new quota row. Callers must have already checked
/// `scheduler_core::quota::check_name` against [`existing`] -- the
/// `UNIQUE COLLATE NOCASE` constraint on `name` is the backstop for a write
/// path that forgot to (`T-collation-enforces-name-identity`), not the
/// primary guard, so a caller that skipped the check sees a constraint
/// violation here rather than a silent duplicate.
pub async fn create(
    pool: &SqlitePool,
    definition: &QuotaDefinition,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES (?, ?, ?)")
        .bind(&definition.name)
        .bind(definition.weekly_target.minutes())
        .bind(created_at_ms)
        .execute(pool)
        .await?;
    Ok(())
}

/// Renames and retargets `id` in one statement
/// (`T-set-operations-execute-in-the-store`) -- a rename and a retarget are
/// one gesture on this screen (#148), never two round trips one of which
/// could apply without the other. Leaves every logged session untouched:
/// they hang off `quota_id`, not the name, so what was logged comes with
/// whichever spelling and target the quota now carries.
pub(super) async fn update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    weekly_target_minutes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE quotas SET name = ?, weekly_target_minutes = ? WHERE id = ?")
        .bind(name)
        .bind(weekly_target_minutes)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Removes `id` (#148, `D-kill-means-archive`'s bargain: stamped, not
/// deleted). Guarded by `archived_at IS NULL` the same way
/// `mark_done::store::mark_task_done` guards its own stamp -- idempotent
/// rather than merely harmless, since a second removal has nothing left to
/// change.
pub(super) async fn archive(
    pool: &SqlitePool,
    id: i64,
    archived_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE quotas SET archived_at = ? WHERE id = ? AND archived_at IS NULL")
        .bind(archived_at_ms)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Brings an archived quota back at its newly typed name and target,
/// keeping its `id` -- and with it every session already logged against
/// that `id`, and its original position in `list_quotas`' `ORDER BY id ASC`
/// (#148, `quota-screen-removed-name-returns-09`: the identity that logged
/// those sessions is what returns, not a fresh one that happens to read the
/// same). `archived_name` is [`super::NameStanding::Taken`]'s own exact
/// stored spelling, not the newly typed candidate -- the two need not
/// match byte-for-byte (`T-collation-enforces-name-identity` folds case,
/// spaces and punctuation), and this `WHERE` must find the row the
/// candidate matched, not merely a name that looks like it.
pub(super) async fn revive(
    pool: &SqlitePool,
    archived_name: &str,
    new_name: &str,
    weekly_target_minutes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE quotas SET name = ?, weekly_target_minutes = ?, archived_at = NULL \
         WHERE name = ? AND archived_at IS NOT NULL",
    )
    .bind(new_name)
    .bind(weekly_target_minutes)
    .bind(archived_name)
    .execute(pool)
    .await?;
    Ok(())
}

/// A row of [`week_sessions`]: one logged session, wherever it belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct SessionRow {
    pub id: i64,
    pub quota_id: i64,
    pub day_ms: i64,
    pub minutes: i64,
}

/// Every session logged for *any* quota whose day falls within
/// `[week_start_ms, week_end_ms)` -- the week boundary is the query's own
/// `WHERE`, not a filter a caller applies afterwards
/// (`T-set-operations-execute-in-the-store`), and one query for every quota
/// is what keeps a screen with several quotas at one round trip rather than
/// one per row. Oldest day first, and within a day, logged-first
/// (`quota-sessions-this-week-lists-what-was-logged-04`'s "ordered Monday
/// first"), which grouping the flat result by `quota_id` afterwards
/// preserves without a second sort.
pub async fn week_sessions(
    pool: &SqlitePool,
    week_start_ms: i64,
    week_end_ms: i64,
) -> Result<Vec<SessionRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, quota_id, day_ms, minutes FROM quota_sessions \
         WHERE day_ms >= ? AND day_ms < ? ORDER BY day_ms ASC, id ASC",
    )
    .bind(week_start_ms)
    .bind(week_end_ms)
    .fetch_all(pool)
    .await
}

/// Logs one session against `quota_id` -- the one write path
/// [`crate::quota::http`]'s quick-log buttons and its `Other…` form both
/// reach (`T-one-front-door-per-capability`).
pub async fn log_session(
    pool: &SqlitePool,
    quota_id: i64,
    day_ms: i64,
    minutes: i64,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO quota_sessions (quota_id, day_ms, minutes, created_at_ms) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(quota_id)
    .bind(day_ms)
    .bind(minutes)
    .bind(created_at_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// Corrects a logged session's day and minutes in place -- its `id` never
/// changes, so "This week" keeps listing the same row rather than a
/// delete-and-recreate a reader could mistake for two events.
pub async fn update_session(
    pool: &SqlitePool,
    session_id: i64,
    day_ms: i64,
    minutes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE quota_sessions SET day_ms = ?, minutes = ? WHERE id = ?")
        .bind(day_ms)
        .bind(minutes)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes a logged session outright -- `D-kill-means-archive` governs a
/// *task*'s own lifecycle, not a session, which has nothing to browse once
/// gone (`quota-sessions-deleting-a-session-07`: deleting one takes its
/// minutes with it, not merely hides them).
pub async fn delete_session(pool: &SqlitePool, session_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM quota_sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    fn definition(name: &str, weekly_target_minutes: i64) -> QuotaDefinition {
        QuotaDefinition {
            name: name.to_string(),
            weekly_target: WeeklyTarget::from_minutes(weekly_target_minutes)
                .expect("a positive target"),
        }
    }

    #[tokio::test]
    async fn list_quotas_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(list_quotas(&pool).await.unwrap(), Vec::new());
    }

    #[tokio::test]
    async fn create_writes_a_quota_list_quotas_then_reports() {
        let (_dir, pool) = test_pool().await;

        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Piano");
        assert_eq!(rows[0].weekly_target.minutes(), 240);
    }

    #[tokio::test]
    async fn list_quotas_orders_oldest_first() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();
        create(&pool, &definition("Running", 180), 1).await.unwrap();
        create(&pool, &definition("Rust", 300), 2).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();

        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Piano", "Running", "Rust"]);
    }

    #[tokio::test]
    async fn existing_reports_every_quotas_name_target_and_archived_flag() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let rows = existing(&pool, None).await.unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Piano");
        assert_eq!(rows[0].weekly_target_minutes, 240);
        assert!(!rows[0].archived);
    }

    #[tokio::test]
    async fn existing_still_reports_an_archived_quota() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_quota(&pool, "Piano", 240).await;
        archive(&pool, id, 4242).await.unwrap();

        let rows = existing(&pool, None).await.unwrap();

        assert_eq!(rows.len(), 1, "an archived quota still holds its name");
        assert!(rows[0].archived);
    }

    /// The rename door's own exclusion, and the reason it is a `WHERE`: a
    /// quota measured against itself collides with itself, so `workout`
    /// could never be respelled `Workout`.
    #[tokio::test]
    async fn existing_omits_the_row_the_caller_excluded() {
        let (_dir, pool) = test_pool().await;
        let piano = given_a_quota(&pool, "Piano", 240).await;
        given_a_quota(&pool, "Running", 120).await;

        let rows = existing(&pool, Some(piano)).await.unwrap();

        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Running"]);
    }

    #[tokio::test]
    async fn update_renames_and_retargets_in_place() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_quota(&pool, "Piano", 240).await;

        update(&pool, id, "Piano theory", 120).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();
        assert_eq!(rows.len(), 1, "the same row, not a second one");
        assert_eq!(rows[0].name, "Piano theory");
        assert_eq!(rows[0].weekly_target.minutes(), 120);
    }

    #[tokio::test]
    async fn archive_removes_a_quota_from_list_quotas_without_deleting_it() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_quota(&pool, "Piano", 240).await;

        archive(&pool, id, 4242).await.unwrap();

        assert!(list_quotas(&pool).await.unwrap().is_empty());
        let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM quotas WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(still_there, 1, "nothing was deleted");
    }

    #[tokio::test]
    async fn archive_is_a_no_op_the_second_time() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_quota(&pool, "Piano", 240).await;
        archive(&pool, id, 1).await.unwrap();

        archive(&pool, id, 2).await.unwrap();

        let archived_at: i64 = sqlx::query_scalar("SELECT archived_at FROM quotas WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(archived_at, 1, "the first stamp must not be overwritten");
    }

    #[tokio::test]
    async fn revive_clears_archived_at_and_applies_the_new_name_and_target() {
        let (_dir, pool) = test_pool().await;
        let id = given_a_quota(&pool, "Piano", 240).await;
        archive(&pool, id, 4242).await.unwrap();

        revive(&pool, "Piano", "Piano", 120).await.unwrap();

        let rows = list_quotas(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id, "the same identity, not a fresh row");
        assert_eq!(rows[0].weekly_target.minutes(), 120);
    }

    #[tokio::test]
    async fn revive_leaves_a_live_quota_of_the_same_name_alone() {
        let (_dir, pool) = test_pool().await;
        given_a_quota(&pool, "Piano", 240).await;

        revive(&pool, "Piano", "Piano", 120).await.unwrap();

        assert_eq!(
            list_quotas(&pool).await.unwrap()[0].weekly_target.minutes(),
            240,
            "a live quota is not archived, so revive must not touch it"
        );
    }

    fn stored(weekly_target_minutes: i64) -> StoredQuotaRow {
        StoredQuotaRow {
            id: 1,
            name: "Piano".to_string(),
            weekly_target_minutes,
        }
    }

    /// A row that has broken its own `CHECK` is refused here rather than
    /// handed on as a zero for `quota::progress` to divide by. The
    /// constraint makes this unreachable through the write path -- an
    /// `UPDATE` to zero is refused by SQLite itself -- so the conversion is
    /// exercised where it lives rather than through a database rigged to
    /// permit what it forbids.
    #[test]
    fn a_stored_target_that_is_not_positive_is_refused() {
        assert!(
            quota_row(stored(0)).is_err(),
            "expected a zero stored target to be refused"
        );
        assert!(
            quota_row(stored(-30)).is_err(),
            "expected a negative stored target to be refused"
        );
    }

    #[test]
    fn a_positive_stored_target_converts_to_the_domains_own_type() {
        let row = quota_row(stored(240)).expect("a positive target converts");

        assert_eq!(row.id, 1);
        assert_eq!(row.name, "Piano");
        assert_eq!(row.weekly_target.minutes(), 240);
    }

    /// The `CHECK` is the reason the conversion above is unreachable in
    /// practice, so it is worth one assertion of its own.
    #[tokio::test]
    async fn the_column_refuses_a_target_of_zero() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let result = sqlx::query("UPDATE quotas SET weekly_target_minutes = 0")
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "expected the CHECK to refuse a zero target"
        );
    }

    #[tokio::test]
    async fn create_refuses_a_second_row_with_the_same_name_once_case_is_folded() {
        let (_dir, pool) = test_pool().await;
        create(&pool, &definition("Piano", 240), 0).await.unwrap();

        let result = create(&pool, &definition("PIANO", 120), 1).await;

        assert!(
            result.is_err(),
            "expected the UNIQUE COLLATE NOCASE backstop to refuse a case-only duplicate"
        );
    }

    // --- sessions (#93, quota-sessions) ----------------------------------

    async fn given_a_quota(pool: &SqlitePool, name: &str, weekly_target_minutes: i64) -> i64 {
        create(pool, &definition(name, weekly_target_minutes), 0)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT id FROM quotas WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    const MONDAY_MS: i64 = 1_000_000_000_000;
    const WEEK_END_MS: i64 = MONDAY_MS + 7 * 86_400_000;

    #[tokio::test]
    async fn week_sessions_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn log_session_then_week_sessions_reports_it() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", 240).await;

        log_session(&pool, quota_id, MONDAY_MS, 20, 0)
            .await
            .unwrap();

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].quota_id, quota_id);
        assert_eq!(sessions[0].day_ms, MONDAY_MS);
        assert_eq!(sessions[0].minutes, 20);
    }

    #[tokio::test]
    async fn week_sessions_excludes_a_session_outside_the_bounds() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", 240).await;
        log_session(&pool, quota_id, WEEK_END_MS, 20, 0)
            .await
            .unwrap();

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();

        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn week_sessions_orders_by_day_then_by_when_it_was_logged() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", 240).await;
        let tuesday_ms = MONDAY_MS + 86_400_000;
        log_session(&pool, quota_id, tuesday_ms, 60, 1)
            .await
            .unwrap();
        log_session(&pool, quota_id, MONDAY_MS, 20, 2)
            .await
            .unwrap();
        log_session(&pool, quota_id, tuesday_ms, 30, 3)
            .await
            .unwrap();

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();

        let minutes: Vec<i64> = sessions.iter().map(|s| s.minutes).collect();
        assert_eq!(
            minutes,
            vec![20, 60, 30],
            "Monday first, then Tuesday in logged order"
        );
    }

    #[tokio::test]
    async fn week_sessions_reports_every_quotas_own_sessions_in_one_query() {
        let (_dir, pool) = test_pool().await;
        let piano = given_a_quota(&pool, "Piano", 240).await;
        let running = given_a_quota(&pool, "Running", 180).await;
        log_session(&pool, piano, MONDAY_MS, 20, 0).await.unwrap();
        log_session(&pool, running, MONDAY_MS, 30, 0).await.unwrap();

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();

        let quota_ids: Vec<i64> = sessions.iter().map(|s| s.quota_id).collect();
        assert_eq!(quota_ids, vec![piano, running]);
    }

    #[tokio::test]
    async fn update_session_changes_its_day_and_minutes() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", 240).await;
        log_session(&pool, quota_id, MONDAY_MS, 25, 0)
            .await
            .unwrap();
        let session_id = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap()[0].id;
        let tuesday_ms = MONDAY_MS + 86_400_000;

        update_session(&pool, session_id, tuesday_ms, 45)
            .await
            .unwrap();

        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].day_ms, tuesday_ms);
        assert_eq!(sessions[0].minutes, 45);
    }

    #[tokio::test]
    async fn delete_session_removes_it_and_leaves_others_alone() {
        let (_dir, pool) = test_pool().await;
        let quota_id = given_a_quota(&pool, "Piano", 240).await;
        let tuesday_ms = MONDAY_MS + 86_400_000;
        log_session(&pool, quota_id, MONDAY_MS, 25, 0)
            .await
            .unwrap();
        log_session(&pool, quota_id, tuesday_ms, 35, 1)
            .await
            .unwrap();
        let sessions = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();
        let monday_session_id = sessions.iter().find(|s| s.day_ms == MONDAY_MS).unwrap().id;

        delete_session(&pool, monday_session_id).await.unwrap();

        let remaining = week_sessions(&pool, MONDAY_MS, WEEK_END_MS).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].day_ms, tuesday_ms);
    }
}
