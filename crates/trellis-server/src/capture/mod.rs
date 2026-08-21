//! **Capture** — getting a thought out of a head and into the system before
//! it evaporates. One route, `POST /captures`, content-negotiated so the JSON
//! API and the inbox's quick-add box are one code path; a 50ms budget, which
//! is why a tagged capture costs one lookup and one insert, and an untagged
//! one costs the insert alone.
//!
//! A capture is raw text and an optional context tag. It is not yet a task,
//! carries no kind, no deadline and no priority, and is never deleted —
//! [`crate::triage`] is what turns one into a task, and [`crate::inbox`] is
//! what shows the ones that have not been triaged yet.
//!
//! **The tag lives here, on the capture, and nowhere else**
//! (`context-tags-survives-triage-07`'s "one fact, one row"): a task reads
//! it through the capture it came from rather than carrying a copy, which is
//! two places to edit and two chances to disagree. [`resolve_tag`] is the one
//! front door both ways a tag is written go through — a fresh capture
//! ([`create`]) and a triage-time retag ([`retag`]) — so case identity
//! (`context-tags-case-is-one-tag-06`) is decided in exactly one place.

pub mod http;
pub mod store;

use sqlx::SqlitePool;

/// Normalizes `raw` (trim, blank-collapse-to-`None`) and, if it names a tag,
/// resolves it to the spelling already in use for it — the earliest one, per
/// [`store::canonical_tag`]'s own `COLLATE NOCASE` lookup — or to the
/// normalized text itself if this is the first capture to use it.
///
/// `pub(crate)` rather than `pub`: `triage::http` calls this for a
/// triage-time retag, so it is a front door between two capabilities, not a
/// private helper — but nothing outside this crate has any business writing
/// a capture's tag.
pub(crate) async fn resolve_tag(
    pool: &SqlitePool,
    raw: Option<&str>,
) -> Result<Option<String>, sqlx::Error> {
    let Some(normalized) = scheduler_core::context_tag::normalize(raw) else {
        return Ok(None);
    };
    Ok(Some(
        match store::canonical_tag(pool, &normalized).await? {
            Some(existing) => existing,
            None => normalized,
        },
    ))
}

/// Writes a new capture, tagging it if `raw_context_tag` names one. Returns
/// the new row's id and the tag as resolved (the spelling that will actually
/// render), since a caller building the freshly-created row's markup needs
/// it and would otherwise have no way to learn it.
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

/// Tags an existing capture at triage time (`context-tags-taggable-at-triage-08`).
/// An absent or blank `raw_context_tag` is a no-op, not a reset — a tag given
/// at capture and left blank on the triage form must survive, since the two
/// are two chances to supply the same fact, not two independent writes of it.
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

/// Every tag in use, for the suggestion control — [`store::distinct_tags`]
/// through this capability's own front door, the way every other capability
/// reaches capture (`T-one-front-door-per-capability`).
pub(crate) async fn distinct_tags(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    store::distinct_tags(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{stored_context_tag, test_pool};
    use proptest::prelude::*;

    #[tokio::test]
    async fn resolve_tag_reports_none_for_an_absent_tag() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(resolve_tag(&pool, None).await.unwrap(), None);
    }

    #[tokio::test]
    async fn resolve_tag_reports_none_for_a_whitespace_only_tag() {
        let (_dir, pool) = test_pool().await;
        assert_eq!(resolve_tag(&pool, Some("   ")).await.unwrap(), None);
    }

    #[tokio::test]
    async fn resolve_tag_reports_the_trimmed_tag_when_it_is_new() {
        let (_dir, pool) = test_pool().await;

        let resolved = resolve_tag(&pool, Some("  @homedepot  ")).await.unwrap();

        assert_eq!(resolved.as_deref(), Some("@homedepot"));
    }

    #[tokio::test]
    async fn resolve_tag_reports_the_earlier_spelling_for_a_case_variant() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", Some("@HomeDepot"), 0)
            .await
            .unwrap();
        assert!(id > 0);

        let resolved = resolve_tag(&pool, Some("@homedepot")).await.unwrap();

        assert_eq!(resolved.as_deref(), Some("@HomeDepot"));
    }

    #[tokio::test]
    async fn create_stores_the_row_and_the_resolved_tag() {
        let (_dir, pool) = test_pool().await;

        let (id, resolved) = create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        assert_eq!(resolved.as_deref(), Some("@homedepot"));
        assert_eq!(
            stored_context_tag(&pool, id).await.as_deref(),
            Some("@homedepot")
        );
    }

    #[tokio::test]
    async fn create_with_no_tag_stores_none_and_reports_none() {
        let (_dir, pool) = test_pool().await;

        let (id, resolved) = create(&pool, "buy screws", "web", None, 0).await.unwrap();

        assert_eq!(resolved, None);
        assert_eq!(stored_context_tag(&pool, id).await, None);
    }

    #[tokio::test]
    async fn retag_writes_a_tag_onto_an_untagged_capture() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", None, 0).await.unwrap();

        retag(&pool, id, Some("@homedepot")).await.unwrap();

        assert_eq!(
            stored_context_tag(&pool, id).await.as_deref(),
            Some("@homedepot")
        );
    }

    #[tokio::test]
    async fn retag_with_no_tag_leaves_an_existing_tag_untouched() {
        let (_dir, pool) = test_pool().await;
        let (id, _) = create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();

        retag(&pool, id, None).await.unwrap();

        assert_eq!(
            stored_context_tag(&pool, id).await.as_deref(),
            Some("@homedepot")
        );
    }

    #[tokio::test]
    async fn distinct_tags_reflects_captures_created_through_this_module() {
        let (_dir, pool) = test_pool().await;
        create(&pool, "buy screws", "web", Some("@homedepot"), 0)
            .await
            .unwrap();
        create(&pool, "call the dentist", "web", None, 1)
            .await
            .unwrap();

        assert_eq!(
            distinct_tags(&pool).await.unwrap(),
            vec!["@homedepot".to_string()]
        );
    }

    /// Spellings that collide case-insensitively, padded with whitespace, so
    /// "same tag, typed differently" is the common case rather than a rare
    /// one — which is the whole thing this property is about.
    ///
    /// **ASCII on purpose.** SQLite's `NOCASE` folds `A-Z` and nothing else,
    /// so `@CAFÉ` and `@café` are two tags, not one. Generating non-ASCII
    /// here would assert a claim the schema does not make.
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

    proptest! {
        #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

        /// **One tag, spelled the way it was first typed** —
        /// `context-tags-case-is-one-tag-06`'s claim about one pair of
        /// spellings, over arbitrary sequences of them.
        ///
        /// Two things at once, because they are one fact seen from two
        /// sides: every capture stores the *first* spelling of its
        /// case-insensitive class rather than the one just submitted, and
        /// nothing a caller can type is refused by migration `0010`'s
        /// `CHECK`. That second half is what ties
        /// `context_tag::normalize`'s postcondition to the column that
        /// mirrors it — a pure crate has no database to assert it against.
        #[test]
        #[ignore]
        fn a_tag_is_stored_as_the_spelling_it_was_first_given(
            submissions in prop::collection::vec(any_submission(), 1..6),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (stored, expected) = rt.block_on(async {
                let (_dir, pool) = test_pool().await;
                let mut first_by_class: Vec<(String, String)> = Vec::new();
                let mut expected: Vec<String> = Vec::new();
                let mut stored: Vec<Option<String>> = Vec::new();

                for (text, raw_tag) in &submissions {
                    let class = raw_tag.trim().to_lowercase();
                    let first = match first_by_class.iter().find(|(c, _)| *c == class) {
                        Some((_, spelling)) => spelling.clone(),
                        None => {
                            let spelling = raw_tag.trim().to_string();
                            first_by_class.push((class, spelling.clone()));
                            spelling
                        }
                    };
                    expected.push(first);
                    let (id, _) = create(&pool, text, "web", Some(raw_tag), 0)
                        .await
                        .expect("the column CHECK must accept every resolved tag");
                    stored.push(stored_context_tag(&pool, id).await);
                }
                (stored, expected)
            });

            let stored: Vec<String> = stored
                .into_iter()
                .map(|t| t.expect("every submission named a tag"))
                .collect();
            prop_assert_eq!(stored, expected);
        }
    }
}
