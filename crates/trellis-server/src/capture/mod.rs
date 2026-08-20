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

/// Writes a new capture carrying whatever tag it was given -- the quick-add
/// box's whole write. Returns the new capture's id and the tag it now
/// carries (already resolved, for the row the caller renders back).
///
/// **Resolve first, then one `INSERT`.** The order matters only for
/// honesty, not for the answer: [`resolve_tag`] looks for a tag already
/// *stored*, and the row being written does not carry one yet either way.
/// What it buys is that the capture is never briefly on disk without the
/// tag this function has already promised its caller, and that the write
/// stays the single statement this module's header budgets for.
pub(crate) async fn create(
    pool: &SqlitePool,
    raw_text: &str,
    source: &str,
    raw_context_tag: Option<&str>,
    created_at_ms: i64,
) -> Result<(i64, Option<String>), sqlx::Error> {
    let resolved = resolve_tag(pool, raw_context_tag).await?;
    let id = store::insert(pool, raw_text, source, resolved.as_deref(), created_at_ms).await?;
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
    use proptest::prelude::*;

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

    /// Spellings that collide case-insensitively, padded with whitespace --
    /// so "same tag, typed differently" is the common case rather than a
    /// rare one, which is the whole thing this property is about.
    ///
    /// **ASCII on purpose.** SQLite's `NOCASE` folds `A-Z` and nothing else,
    /// so `@CAFÉ` and `@café` are two tags, not one. That is the same
    /// limitation `life_areas.name UNIQUE COLLATE NOCASE` already carries
    /// and `life_area.rs` already documents ("if the rule ever outgrows what
    /// a collation can express -- Unicode folding, say -- it comes back
    /// here"). Generating non-ASCII here would assert a claim the schema
    /// does not make.
    fn any_submission() -> impl Strategy<Value = (String, String)> {
        let tag = prop_oneof![
            Just("@homedepot"),
            Just("@HomeDepot"),
            Just("@HOMEDEPOT"),
            Just("@supermarket"),
            Just("@SuperMarket"),
        ];
        let pad = prop_oneof![Just(""), Just(" "), Just("\t"), Just("  ")];
        (tag, pad.clone(), pad, "[a-z ]{1,12}")
            .prop_map(|(tag, before, after, text)| (text, format!("{before}{tag}{after}")))
    }

    fn class_of(raw: &str) -> String {
        raw.trim().to_lowercase()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

        /// **One tag, spelled the way it was first typed** -- the claim
        /// `context-tags-case-is-one-tag-06` makes about one pair of
        /// spellings, over arbitrary sequences of them.
        ///
        /// Three things at once, because they are the same fact seen from
        /// three sides: every capture stores the *first* spelling of its
        /// case-insensitive class rather than the one just submitted;
        /// [`distinct_tags`] reports exactly one entry per class, in
        /// first-use order; and nothing a caller can type is refused by
        /// migration `0010`'s `CHECK`, which is what ties
        /// `context_tag::normalize`'s postcondition to the column that
        /// depends on it -- the pure crate has no database to assert that
        /// against itself.
        #[test]
        #[ignore]
        fn a_tag_is_stored_as_the_spelling_it_was_first_given(
            submissions in prop::collection::vec(any_submission(), 1..6),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (stored, listed, expected_first, expected_distinct) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;

                let mut expected_first: Vec<String> = Vec::new();
                let mut first_by_class: Vec<(String, String)> = Vec::new();
                let mut stored: Vec<Option<String>> = Vec::new();

                for (text, raw_tag) in &submissions {
                    let class = class_of(raw_tag);
                    let first = match first_by_class.iter().find(|(c, _)| *c == class) {
                        Some((_, spelling)) => spelling.clone(),
                        None => {
                            let spelling = raw_tag.trim().to_string();
                            first_by_class.push((class, spelling.clone()));
                            spelling
                        }
                    };
                    expected_first.push(first);

                    let (id, _) = create(&pool, text, "web", Some(raw_tag), 0)
                        .await
                        .expect("the column CHECK must accept every resolved tag");
                    stored.push(stored_tag(&pool, id).await);
                }

                let listed = distinct_tags(&pool).await.unwrap();
                let expected_distinct: Vec<String> =
                    first_by_class.into_iter().map(|(_, s)| s).collect();
                (stored, listed, expected_first, expected_distinct)
            });

            let stored: Vec<String> = stored
                .into_iter()
                .map(|tag| tag.expect("every submission named a tag"))
                .collect();
            prop_assert_eq!(stored, expected_first);
            prop_assert_eq!(listed, expected_distinct);
        }
    }
}
