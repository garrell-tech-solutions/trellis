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
/// `context_tag` is written by this statement rather than by a follow-up
/// `UPDATE`. It must arrive **already resolved** — normalized and settled to
/// whichever spelling was first used — which is [`super::resolve_tag`]'s
/// job, not this one's; the column's `CHECK` refuses a padded or empty
/// string either way.
///
/// Two reasons it belongs here. The write path this module's own header
/// budgets at 50ms was three round trips (insert, look the tag up, update)
/// where two will do. And between the second and the third, the row existed
/// **untagged** while `create` had already handed its caller the tag it was
/// supposed to carry — a window with no reader today, and one that a
/// re-render or a retry would eventually find.
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

/// Writes `tag` (already normalized — trimmed, non-empty) onto `capture_id`
/// — the retag path, for a capture that already exists. A capture created
/// *with* a tag gets it from [`insert`] instead, in one statement.
///
/// `pub(super)`, so the front door this module's header describes is
/// enforced by the compiler and not only by `platform::boundary`'s substring
/// lint. `resolve_tag` decides case identity "in exactly one place" only if
/// nothing can write a tag without going through it; while this was `pub`,
/// any capability could store an unresolved spelling and split one tag in
/// two. Same move `T-inbox-owns-membership` made for
/// `inbox::store::close_capture`.
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

/// The stored spelling of `normalized`, if some earlier capture already used
/// a tag that is the same one under the column's `COLLATE NOCASE` comparison
/// — what `context-tags-case-is-one-tag-06` means by "shown as first typed".
/// `ORDER BY id ASC LIMIT 1` picks the earliest row that used it, so a tag's
/// canonical spelling is whichever came first, never whichever query ran
/// last.
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

/// Every tag in use, once each, in the spelling `canonical_tag` would report
/// for it — what the suggestion control offers
/// (`context-tags-suggestions-05`). `MIN(id)` per `context_tag` picks each
/// tag's earliest row under the column's own collation, so a tag used with
/// two case spellings is grouped and reported once, in whichever spelling
/// was first.
pub async fn distinct_tags(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT context_tag FROM captures \
         WHERE id IN (SELECT MIN(id) FROM captures WHERE context_tag IS NOT NULL GROUP BY context_tag) \
         ORDER BY id ASC",
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
    async fn a_freshly_inserted_capture_has_no_context_tag() {
        let (_dir, pool) = test_pool().await;
        insert(&pool, "buy milk", "web", None, 0).await.unwrap();

        let tag: Option<String> = sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn set_context_tag_writes_the_named_captures_tag() {
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
        let other = insert(&pool, "call the dentist", "web", None, 1)
            .await
            .unwrap();

        set_context_tag(&pool, tagged, "@homedepot").await.unwrap();

        let tag: Option<String> =
            sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
                .bind(other)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn canonical_tag_reports_none_when_no_capture_has_used_it() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(canonical_tag(&pool, "@homedepot").await.unwrap(), None);
    }

    #[tokio::test]
    async fn canonical_tag_reports_the_earliest_spelling_used() {
        let (_dir, pool) = test_pool().await;
        let first = insert(&pool, "buy screws", "web", None, 0).await.unwrap();
        let second = insert(&pool, "return the drill", "web", None, 1)
            .await
            .unwrap();
        set_context_tag(&pool, first, "@HomeDepot").await.unwrap();
        set_context_tag(&pool, second, "@homedepot").await.unwrap();

        let canonical = canonical_tag(&pool, "@homedepot").await.unwrap();

        assert_eq!(canonical.as_deref(), Some("@HomeDepot"));
    }

    #[tokio::test]
    async fn distinct_tags_is_empty_against_a_fresh_database() {
        let (_dir, pool) = test_pool().await;

        assert_eq!(distinct_tags(&pool).await.unwrap(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn distinct_tags_reports_each_tag_once_in_its_first_spelling() {
        let (_dir, pool) = test_pool().await;
        let first = insert(&pool, "buy screws", "web", None, 0).await.unwrap();
        let second = insert(&pool, "return the drill", "web", None, 1)
            .await
            .unwrap();
        let third = insert(&pool, "pick up milk", "web", None, 2).await.unwrap();
        let untagged = insert(&pool, "renew the passport", "web", None, 3)
            .await
            .unwrap();
        set_context_tag(&pool, first, "@HomeDepot").await.unwrap();
        set_context_tag(&pool, second, "@homedepot").await.unwrap();
        set_context_tag(&pool, third, "@supermarket").await.unwrap();
        let _ = untagged;

        let tags = distinct_tags(&pool).await.unwrap();

        assert_eq!(
            tags,
            vec!["@HomeDepot".to_string(), "@supermarket".to_string()]
        );
    }
}
