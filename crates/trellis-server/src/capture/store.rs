//! Writing a capture down.
//!
//! Only the write lives here. Listing untriaged captures is the inbox's
//! query (`inbox::store`) and stamping one as consumed is triage's write
//! (`triage::store`), because a capability owns the SQL it issues rather
//! than the table it happens to touch — three domains write the `captures`
//! table and each does so from its own module. Everything here speaks
//! `sqlx::Error`; naming a status code is the delivery side's job
//! (`T-module-boundary`, enforced by `platform::boundary`).

use sqlx::SqlitePool;

/// Returns the new capture's id — the inbox row this request renders needs
/// it to aim a later triage action at.
///
/// `context_tag` is written by this same statement rather than by a
/// follow-up `UPDATE`. It arrived as insert-then-update, which cost the
/// quick-add box a third round trip on the path this module's own header
/// budgets at 50ms, and left a window in which the capture existed without
/// the tag the caller had already been told it carried. It must be
/// **already resolved** — normalized and canonicalized to whichever
/// spelling was first used — which is [`super::resolve_tag`]'s job, not
/// this one's; the column's `CHECK` refuses an untrimmed or empty string
/// either way (`T-empty-equals-absent`).
pub async fn insert(
    pool: &SqlitePool,
    raw_text: &str,
    source: &str,
    context_tag: Option<&str>,
    created_at_ms: i64,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, context_tag, created_at_ms) \
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(raw_text)
    .bind(source)
    .bind(context_tag)
    .bind(created_at_ms)
    .fetch_one(pool)
    .await
}

/// Sets `capture_id`'s context tag outright -- the tag's own store, on the
/// capture (`features/context_tags.feature`'s own header: "one fact, one
/// row"). `tag` is assumed already resolved (normalized and canonicalized
/// to whichever spelling was first used, `crate::capture::resolve_tag`'s
/// job) -- this is the write, not the rule.
///
/// `pub(super)`, so the rule cannot be bypassed by anything but this
/// capability: retagging an existing capture goes through
/// [`super::retag`], which resolves first. Same move
/// `T-inbox-owns-membership` made for `inbox::store::close_capture` --
/// the front door is enforced by the compiler here, not only by
/// `platform::boundary`'s substring lint.
pub(super) async fn set_context_tag(
    pool: &SqlitePool,
    capture_id: i64,
    tag: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE captures SET context_tag = ? WHERE id = ?")
        .bind(tag)
        .bind(capture_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// The existing spelling of a case-insensitively matching tag, if any --
/// the first one ever stored. `None` when `normalized` names no tag stored
/// before. Relies on `captures.context_tag`'s own `COLLATE NOCASE` (migration
/// `0010`) for the comparison, the same way `life_areas.name`'s collation
/// backs `life_areas::store::find_by_name`.
pub async fn canonical_tag(
    pool: &SqlitePool,
    normalized: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT context_tag FROM captures WHERE context_tag = ? ORDER BY id ASC LIMIT 1",
    )
    .bind(normalized)
    .fetch_optional(pool)
    .await
}

/// Every distinct tag stored on any capture, earliest use first -- one
/// case-insensitive group per tag (the column's own `COLLATE NOCASE`),
/// naming the spelling its earliest row used.
pub async fn distinct_tags(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT context_tag FROM captures WHERE id IN ( \
             SELECT MIN(id) FROM captures WHERE context_tag IS NOT NULL GROUP BY context_tag \
         ) ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    #[tokio::test]
    async fn insert_returns_the_new_captures_id() {
        let (_dir, pool) = test_pool().await;

        let id = insert(&pool, "buy milk", "web", None, 1234).await.unwrap();

        let row_id: i64 = sqlx::query_scalar("SELECT id FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id, row_id);
    }

    #[tokio::test]
    async fn insert_stores_the_submitted_text_source_and_timestamp() {
        let (_dir, pool) = test_pool().await;

        insert(&pool, "buy milk", "web", None, 1234).await.unwrap();

        let row: (String, String, i64) =
            sqlx::query_as("SELECT raw_text, source, created_at_ms FROM captures")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row, ("buy milk".to_string(), "web".to_string(), 1234));
    }

    #[tokio::test]
    async fn a_freshly_inserted_capture_is_untriaged() {
        let (_dir, pool) = test_pool().await;

        insert(&pool, "buy milk", "web", None, 1234).await.unwrap();

        let left_inbox_at: Option<i64> = sqlx::query_scalar("SELECT left_inbox_at FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left_inbox_at, None);
    }

    #[tokio::test]
    async fn insert_reports_the_database_error_when_the_table_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::platform::db::connect(&dir.path().join("unmigrated.db"))
            .await
            .unwrap();

        assert!(insert(&pool, "buy milk", "web", None, 0).await.is_err());
    }

    #[tokio::test]
    async fn a_freshly_inserted_capture_carries_no_tag() {
        let (_dir, pool) = test_pool().await;

        let id = insert(&pool, "buy milk", "web", None, 0).await.unwrap();

        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn set_context_tag_stamps_the_named_capture() {
        let (_dir, pool) = test_pool().await;
        let id = insert(&pool, "buy screws", "web", None, 0).await.unwrap();

        set_context_tag(&pool, id, "@homedepot").await.unwrap();

        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn set_context_tag_leaves_other_captures_untouched() {
        let (_dir, pool) = test_pool().await;
        let tagged = insert(&pool, "buy screws", "web", None, 0).await.unwrap();
        let untouched = insert(&pool, "buy milk", "web", None, 1).await.unwrap();

        set_context_tag(&pool, tagged, "@homedepot").await.unwrap();

        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(untouched)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn canonical_tag_is_none_when_nothing_matches() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(canonical_tag(&pool, "@homedepot").await.unwrap(), None);
    }

    #[tokio::test]
    async fn canonical_tag_finds_a_case_insensitive_match() {
        let (_dir, pool) = test_pool().await;
        let id = insert(&pool, "buy screws", "web", None, 0).await.unwrap();
        set_context_tag(&pool, id, "@HomeDepot").await.unwrap();

        assert_eq!(
            canonical_tag(&pool, "@homedepot").await.unwrap().as_deref(),
            Some("@HomeDepot")
        );
    }

    /// Two captures carrying the same tag in two spellings, earliest first.
    ///
    /// Writes through [`set_context_tag`] rather than
    /// `super::resolve_tag`, on purpose and in one place: these tests prove
    /// that the *queries* pick the earliest row, so the fixture must be
    /// able to create the state a canonicalizing caller never would.
    async fn given_two_spellings_of_one_tag(pool: &SqlitePool) {
        let first = insert(pool, "buy screws", "web", None, 0).await.unwrap();
        set_context_tag(pool, first, "@HomeDepot").await.unwrap();
        let second = insert(pool, "return the drill", "web", None, 1)
            .await
            .unwrap();
        set_context_tag(pool, second, "@homedepot").await.unwrap();
    }

    #[tokio::test]
    async fn canonical_tag_returns_the_earliest_spelling_when_several_exist() {
        let (_dir, pool) = test_pool().await;
        given_two_spellings_of_one_tag(&pool).await;

        assert_eq!(
            canonical_tag(&pool, "@HOMEDEPOT").await.unwrap().as_deref(),
            Some("@HomeDepot")
        );
    }

    #[tokio::test]
    async fn distinct_tags_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(distinct_tags(&pool).await.unwrap(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn distinct_tags_lists_each_tag_once_earliest_use_first() {
        let (_dir, pool) = test_pool().await;
        let first = insert(&pool, "buy screws", "web", None, 0).await.unwrap();
        set_context_tag(&pool, first, "@homedepot").await.unwrap();
        let second = insert(&pool, "pick up milk", "web", None, 1).await.unwrap();
        set_context_tag(&pool, second, "@supermarket")
            .await
            .unwrap();
        let third = insert(&pool, "return the drill", "web", None, 2)
            .await
            .unwrap();
        set_context_tag(&pool, third, "@homedepot").await.unwrap();

        assert_eq!(
            distinct_tags(&pool).await.unwrap(),
            vec!["@homedepot".to_string(), "@supermarket".to_string()]
        );
    }

    #[tokio::test]
    async fn distinct_tags_reports_the_first_spelling_of_a_case_insensitive_group() {
        let (_dir, pool) = test_pool().await;
        given_two_spellings_of_one_tag(&pool).await;

        assert_eq!(
            distinct_tags(&pool).await.unwrap(),
            vec!["@HomeDepot".to_string()]
        );
    }

    #[tokio::test]
    async fn distinct_tags_excludes_untagged_captures() {
        let (_dir, pool) = test_pool().await;
        insert(&pool, "buy milk", "web", None, 0).await.unwrap();

        assert_eq!(distinct_tags(&pool).await.unwrap(), Vec::<String>::new());
    }
}
