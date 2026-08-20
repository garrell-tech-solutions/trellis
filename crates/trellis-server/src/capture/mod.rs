//! **Capture** — getting a thought out of a head and into the system before
//! it evaporates. One route, `POST /captures`, content-negotiated so the JSON
//! API and the inbox's quick-add box are one code path; a 50ms budget, which
//! is why nothing slower than an insert happens here.
//!
//! A capture is raw text. It is not yet a task, carries no kind, no deadline
//! and no priority, and is never deleted — [`crate::triage`] is what turns
//! one into a task, and [`crate::inbox`] is what shows the ones that have not
//! been triaged yet.
//!
//! A capture may also carry a context tag (#82,
//! `D-context-tags-are-the-taxonomy`) — free text, optional, the product's
//! only taxonomy now. [`resolve_tag`] is the one front door both a fresh
//! capture ([`create`]) and a triage-time retag ([`retag`]) go through
//! (`T-one-front-door-per-capability`), so the two paths cannot canonicalize
//! case identity differently.

pub mod http;
pub mod store;

use scheduler_core::context_tag;
use sqlx::SqlitePool;

/// Normalizes a submitted tag and, if it names one, canonicalizes it to
/// whatever spelling was first used for a case-insensitive match already
/// stored (`context-tags-case-is-one-tag-06`: shown as first typed, the
/// same argument `T-collation-enforces-name-identity` made for life-area
/// names, applied to something typed rather than picked). `None` in, `None`
/// out; a genuinely new tag round-trips as the (trimmed) spelling it was
/// first given.
pub(crate) async fn resolve_tag(
    pool: &SqlitePool,
    raw: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let Some(normalized) = context_tag::normalize(raw) else {
        return Ok(None);
    };
    Ok(Some(
        match store::canonical_tag(pool, &normalized).await? {
            Some(existing) => existing,
            None => normalized,
        },
    ))
}

/// Writes a new capture and, if a tag was given, resolves and stores it --
/// the quick-add box's whole write. Returns the new capture's id and the
/// tag it now carries (already resolved, for the row the caller renders
/// back).
pub(crate) async fn create(
    pool: &SqlitePool,
    raw_text: &str,
    source: &str,
    raw_context_tag: Option<&str>,
    created_at_ms: i64,
) -> Result<(i64, Option<String>), sqlx::Error> {
    let id = store::insert(pool, raw_text, source, created_at_ms).await?;
    let resolved = resolve_tag(pool, raw_context_tag).await?;
    if let Some(tag) = &resolved {
        store::set_context_tag(pool, id, tag).await?;
    }
    Ok((id, resolved))
}

/// Sets or changes the tag on an existing capture — triage's own second
/// chance to tag (`context-tags-taggable-at-triage-07`), routed through the
/// same resolution [`create`] gives a fresh capture. A submission naming no
/// tag is a no-op: triage's own fields are all optional, and an absent
/// `context_tag` must not erase one already there.
pub(crate) async fn retag(
    pool: &SqlitePool,
    capture_id: i64,
    raw_context_tag: Option<&str>,
) -> Result<(), sqlx::Error> {
    if let Some(tag) = resolve_tag(pool, raw_context_tag).await? {
        store::set_context_tag(pool, capture_id, &tag).await?;
    }
    Ok(())
}

/// Every distinct tag stored on any capture, earliest use first — what the
/// tag control's suggestions are built from wherever it renders
/// (`context-tags-suggestions-05`).
pub(crate) async fn distinct_tags(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    store::distinct_tags(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::test_pool;

    async fn stored_tag(pool: &SqlitePool, id: i64) -> Option<String> {
        sqlx::query_scalar("SELECT context_tag FROM captures WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn resolve_tag_is_none_for_an_absent_tag() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(resolve_tag(&pool, None).await.unwrap(), None);
    }

    #[tokio::test]
    async fn resolve_tag_is_none_for_a_whitespace_only_tag() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(resolve_tag(&pool, Some("   ")).await.unwrap(), None);
    }

    #[tokio::test]
    async fn resolve_tag_trims_a_genuinely_new_tag() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(
            resolve_tag(&pool, Some("  @homedepot  ")).await.unwrap(),
            Some("@homedepot".to_string())
        );
    }

    #[tokio::test]
    async fn resolve_tag_canonicalizes_to_a_prior_spelling() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", Some("@HomeDepot"), 0)
            .await
            .unwrap();
        assert!(id > 0);

        let resolved = resolve_tag(&pool, Some("@homedepot")).await.unwrap();

        assert_eq!(resolved.as_deref(), Some("@HomeDepot"));
    }

    #[tokio::test]
    async fn create_stores_the_resolved_tag_on_the_new_capture() {
        let (_dir, pool) = test_pool().await;

        let (id, tag) = create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        assert_eq!(tag.as_deref(), Some("@homedepot"));
        assert_eq!(stored_tag(&pool, id).await.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn create_with_no_tag_stores_none() {
        let (_dir, pool) = test_pool().await;

        let (_id, tag) = create(&pool, "buy milk", "web", None, 0).await.unwrap();

        assert_eq!(tag, None);
    }

    #[tokio::test]
    async fn retag_sets_the_tag_on_an_existing_capture() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", None, 0).await.unwrap();

        retag(&pool, id, Some("@homedepot")).await.unwrap();

        assert_eq!(stored_tag(&pool, id).await.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn retag_with_no_tag_leaves_the_existing_tag_untouched() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        retag(&pool, id, None).await.unwrap();

        assert_eq!(stored_tag(&pool, id).await.as_deref(), Some("@homedepot"));
    }
}
