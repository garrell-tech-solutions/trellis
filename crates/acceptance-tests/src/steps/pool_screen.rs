//! Step handlers for `features/pool_screen.feature`: pool work grouped by
//! where it can be done (#92).
//!
//! The Background ("the trellis server is running with an empty task list")
//! is already matched generically by [`super::triage::dispatch`], tried
//! before this module.
//!
//! Several scenarios name a value via a literal `Examples` placeholder
//! (`"<trips>"`, `"<count>"`, ...) that this Gherkin runner does not
//! pre-substitute — a step's own regex sees the literal text and must
//! resolve it against the current example row itself. [`resolve`] does
//! that, the same helper `context_tags.rs` already established, careful to
//! require the *whole* captured value look like `<name>` before treating it
//! as a placeholder rather than a literal:
//! `pool-screen-escapes-hostile-text-09`'s own tag,
//! `<script>alert('boom')</script>`, both starts and ends with an angle
//! bracket without being one.

use super::html;
use super::payloads;
use super::triage::{given_capture_waiting, when_triaged};
use super::*;
use serde_json::Value;

static GIVEN_POOL_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a pool task "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static GIVEN_POOL_NO_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a pool task "([^"]+)" with no context tag$"#).unwrap());
static GIVEN_N_POOL_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"([^"]+)" pool tasks tagged "([^"]+)"$"#).unwrap());
static GIVEN_COMMITTED_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a committed task "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static GIVEN_QUOTA_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a quota task "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static WHEN_POOL_SCREEN_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen is viewed$").unwrap());
static WHEN_SCREEN_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the "([^"]+)" screen is viewed$"#).unwrap());
static THEN_OFFERS_TRIPS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the pool screen offers the trips "([^"]+)"$"#).unwrap());
static THEN_TRIP_READS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" reads "([^"]+)"$"#).unwrap());
static THEN_LOOSE_SHOWS_TAGGED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the loose ends list shows "([^"]+)" tagged "([^"]+)"$"#).unwrap()
});
static THEN_LOOSE_SHOWS_NO_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the loose ends list shows "([^"]+)" with no context tag$"#).unwrap()
});
static THEN_META: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the pool screen reports "([^"]+)" beside its title$"#).unwrap()
});
static THEN_DOES_NOT_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the pool screen does not mention "([^"]+)"$"#).unwrap());
static THEN_TRIP_LISTS_EXACTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" lists "([^"]+)"$"#).unwrap());
static THEN_TRIP_LISTS_N_ITEMS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" lists "([^"]+)" items$"#).unwrap());
static THEN_TRIP_OFFERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the trip "([^"]+)" offers "([^"]+)"$"#).unwrap());
static THEN_LOOSE_ORDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the loose ends are in the order "([^"]+)"$"#).unwrap());
static THEN_NO_REORDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen offers no reorder control$").unwrap());
static THEN_SHOWS_EMPTY_STATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen shows an empty-state message$").unwrap());
static THEN_WAY_BACK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen offers a way back to Capture$").unwrap());
static THEN_TABS_EXACTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the tab bar offers exactly "([^"]+)"$"#).unwrap());
static THEN_TAB_MARKS_CURRENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the tab bar marks "([^"]+)" as the current tab$"#).unwrap());
static THEN_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the pool screen does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the pool screen contains the word "([^"]+)"$"#).unwrap());
static THEN_POOL_LISTS_NOTHING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the pool screen lists nothing$").unwrap());
static THEN_POOL_LISTS_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the pool screen lists "([^"]+)" tagged "([^"]+)"$"#).unwrap());
static THEN_POOL_LISTS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the pool screen lists "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_POOL_TAGGED.captures(text) {
        return Some(dispatch_given_task(world, example, &caps, payloads::pool()).await);
    }
    if let Some(caps) = GIVEN_POOL_NO_TAG.captures(text) {
        let raw_text = match resolve(example, &caps[1]) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(given_task(world, &raw_text, payloads::pool(), None).await);
    }
    if let Some(caps) = GIVEN_N_POOL_TAGGED.captures(text) {
        return Some(dispatch_given_n_pool_tasks(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_COMMITTED_TAGGED.captures(text) {
        return Some(dispatch_given_task(world, example, &caps, payloads::committed()).await);
    }
    if let Some(caps) = GIVEN_QUOTA_TAGGED.captures(text) {
        return Some(dispatch_given_task(world, example, &caps, payloads::quota()).await);
    }
    if WHEN_POOL_SCREEN_VIEWED.is_match(text) {
        return Some(view_screen(world, "/pool").await);
    }
    if let Some(caps) = WHEN_SCREEN_VIEWED.captures(text) {
        return Some(dispatch_screen_viewed(world, example, &caps).await);
    }
    if let Some(caps) = THEN_OFFERS_TRIPS.captures(text) {
        return Some(dispatch_offers_trips(world, example, &caps));
    }
    if let Some(caps) = THEN_TRIP_READS.captures(text) {
        return Some(dispatch_trip_reads(world, example, &caps));
    }
    if let Some(caps) = THEN_LOOSE_SHOWS_TAGGED.captures(text) {
        return Some(dispatch_loose_shows_tagged(world, example, &caps));
    }
    if let Some(caps) = THEN_LOOSE_SHOWS_NO_TAG.captures(text) {
        return Some(then_loose_shows_no_tag(world, &caps[1]));
    }
    if let Some(caps) = THEN_META.captures(text) {
        return Some(dispatch_meta(world, example, &caps));
    }
    if let Some(caps) = THEN_DOES_NOT_MENTION.captures(text) {
        return Some(
            html_body(world).and_then(|body| super::then_does_not_mention(body, &caps[1])),
        );
    }
    if let Some(caps) = THEN_TRIP_LISTS_EXACTLY.captures(text) {
        return Some(dispatch_trip_lists_exactly(world, example, &caps));
    }
    if let Some(caps) = THEN_TRIP_LISTS_N_ITEMS.captures(text) {
        return Some(dispatch_trip_lists_n_items(world, example, &caps));
    }
    if let Some(caps) = THEN_TRIP_OFFERS.captures(text) {
        return Some(dispatch_trip_offers(world, example, &caps));
    }
    if let Some(caps) = THEN_LOOSE_ORDER.captures(text) {
        return Some(dispatch_loose_order(world, example, &caps));
    }
    if THEN_NO_REORDER.is_match(text) {
        return Some(then_no_reorder(world));
    }
    if THEN_SHOWS_EMPTY_STATE.is_match(text) {
        return Some(then_html_body_contains(world, "Nothing in the pool"));
    }
    if THEN_WAY_BACK.is_match(text) {
        return Some(then_way_back(world));
    }
    if let Some(caps) = THEN_TABS_EXACTLY.captures(text) {
        return Some(dispatch_tabs_exactly(world, example, &caps));
    }
    if let Some(caps) = THEN_TAB_MARKS_CURRENT.captures(text) {
        return Some(dispatch_tab_marks_current(world, example, &caps));
    }
    if THEN_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_html_body_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(then_html_body_contains(world, &caps[1]));
    }
    if THEN_POOL_LISTS_NOTHING.is_match(text) {
        return Some(then_pool_lists_nothing(world).await);
    }
    if let Some(caps) = THEN_POOL_LISTS_TAGGED.captures(text) {
        return Some(dispatch_pool_lists_tagged(world, example, &caps).await);
    }
    if let Some(caps) = THEN_POOL_LISTS.captures(text) {
        return Some(dispatch_pool_lists(world, example, &caps).await);
    }
    None
}

/// See `context_tags.rs`'s own copy of this function for the full
/// reasoning; duplicated rather than shared because the two modules'
/// captures come from different `World` state and sharing would cost more
/// than the four lines it would save.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// [`resolve`], then parsed as a count -- shared by every step that names a
/// quantity (`"3" pool tasks ...`, `the trip "X" lists "3" items`), so
/// neither caller carries its own parse-and-explain branch.
fn resolved_count(example: &BTreeMap<String, String>, raw: &str) -> Result<usize, String> {
    resolve(example, raw)?
        .parse()
        .map_err(|e| format!("bad count {raw:?}: {e}"))
}

async fn dispatch_given_task(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
    kind: Value,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    given_task(world, &raw_text, kind, Some(&tag)).await
}

/// Captures `raw_text`, then triages it as `kind` (already carrying its own
/// required fields, per `payloads`), tagged with `tag` if one is given.
async fn given_task(
    world: &mut World,
    raw_text: &str,
    kind: Value,
    tag: Option<&str>,
) -> Result<(), String> {
    given_capture_waiting(world, raw_text).await?;
    let body = match tag {
        Some(tag) => payloads::with_field(kind, "context_tag", Value::from(tag)),
        None => kind,
    };
    when_triaged(world, body).await
}

async fn dispatch_given_n_pool_tasks(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let count = resolved_count(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    for n in 0..count {
        let raw_text = format!("{tag} errand {n}");
        given_task(world, &raw_text, payloads::pool(), Some(&tag)).await?;
    }
    Ok(())
}

async fn view_screen(world: &mut World, path: &str) -> Result<(), String> {
    let request = axum::http::Request::builder()
        .uri(path)
        .body(axum::body::Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

async fn dispatch_screen_viewed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let screen = resolve(example, &caps[1])?;
    let path = match screen.as_str() {
        "capture" => "/",
        "pool" => "/pool",
        "quota" => "/quota",
        "committed" => "/committed",
        other => return Err(format!("unknown screen {other:?}")),
    };
    view_screen(world, path).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no pool screen response recorded")
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no pool screen response recorded")
}

fn then_html_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    super::then_html_body_excludes(world, forbidden, "no pool screen response recorded")
}

fn dispatch_offers_trips(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    html::listed_in_order(&expected, trip_tags_in_order(body), "trips")
}

/// Every `.trip-tag` label's text, in document order — what "the pool
/// screen offers the trips ..." means.
fn trip_tags_in_order(body: &str) -> Vec<String> {
    body.split(r#"<div class="trip-tag">"#)
        .skip(1)
        .filter_map(|chunk| chunk.split_once("</div>").map(|(tag, _)| tag.to_string()))
        .collect()
}

fn dispatch_trip_reads(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let section = html::trip_section(body, &tag)?;
    let count = html::between(section, r#"<div class="trip-count">"#, "</div>")?;
    if count == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the trip {tag:?} to read {expected:?}, got {count:?}"
        ))
    }
}

/// Scoped to `loose-text`, not the whole row -- see `context_tags.rs`'s
/// identical concern about a triage form's own `placeholder="@homedepot"`
/// producing a false positive. This screen carries no such form, but a
/// second loose row can still share the tag being searched for
/// (`pool-screen-case-folded-grouping-03`'s reasoning applies here too).
fn dispatch_loose_shows_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let row = html::row_containing(loose_scope(body), &raw_text)?;
    if row.contains(&tag) {
        Ok(())
    } else {
        Err(format!(
            "expected the loose row for {raw_text:?} to carry the tag {tag:?}, got:\n{row}"
        ))
    }
}

fn then_loose_shows_no_tag(world: &mut World, raw_text: &str) -> Result<(), String> {
    let body = html_body(world)?;
    let row = html::row_containing(loose_scope(body), raw_text)?;
    let text = html::between(row, r#"<div class="loose-text">"#, "</div>")?;
    if text.trim() == raw_text {
        Ok(())
    } else {
        Err(format!(
            "expected {raw_text:?} to carry no context tag, got row text {text:?}"
        ))
    }
}

fn loose_scope(body: &str) -> &str {
    html::between(body, r#"<ul class="loose">"#, "</ul>").unwrap_or(body)
}

fn dispatch_meta(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let meta = html::between(body, r#"<div class="pool-meta">"#, "</div>")?;
    if meta.trim() == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the pool screen to report {expected:?}, got {:?}",
            meta.trim()
        ))
    }
}

/// Every item the trip labelled `tag` holds, in document order -- the whole
/// `<ul class="trip-items">`, not just what a phone screen paints (#120:
/// one list throughout, CSS decides what is visually hidden past the
/// third; `pool-screen-truncation-06` moved "only three are on screen" to
/// `qa/trip_controls.md`, a fact about a rendered page rather than about
/// this document). Shared by both "lists exactly" and "lists N items",
/// which differ only in what they do with the list once they have it.
fn trip_held_items(world: &World, tag: &str) -> Result<Vec<String>, String> {
    let body = html_body(world)?;
    let section = html::trip_section(body, tag)?;
    let items = html::between(section, r#"<ul class="trip-items">"#, "</ul>")?;
    Ok(list_items(items))
}

fn dispatch_trip_lists_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let actual = trip_held_items(world, &tag)?;
    html::listed_in_order(&expected, actual, &format!("the trip {tag:?} to list"))
}

fn dispatch_trip_lists_n_items(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolved_count(example, &caps[2])?;
    let actual = trip_held_items(world, &tag)?.len();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the trip {tag:?} to list {expected} items, got {actual}"
        ))
    }
}

fn dispatch_trip_offers(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let tag = resolve(example, &caps[1])?;
    let expected = resolve(example, &caps[2])?;
    let body = html_body(world)?;
    let section = html::trip_section(body, &tag)?;
    let (_, label) = html::button(section, r#"class="trip-more-toggle""#)?;
    if label == expected {
        Ok(())
    } else {
        Err(format!(
            "expected the trip {tag:?} to offer {expected:?}, got {label:?}"
        ))
    }
}

/// Every `.trip-item-text` span's content in `section`, trimmed, in
/// document order — scoped past the row's own leading mark-done checkbox
/// (#97), which a bare `<li>...</li>` extraction would otherwise swallow
/// whole.
fn list_items(section: &str) -> Vec<String> {
    section
        .split(r#"<span class="trip-item-text">"#)
        .skip(1)
        .filter_map(|chunk| {
            chunk
                .split_once("</span>")
                .map(|(text, _)| text.trim().to_string())
        })
        .collect()
}

fn dispatch_loose_order(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let section = html::between(body, r#"<ul class="loose">"#, "</ul>")?;
    let actual: Vec<String> = section
        .split(r#"<div class="loose-text">"#)
        .skip(1)
        .filter_map(|chunk| {
            chunk
                .split_once("</div>")
                .map(|(text, _)| text.trim().to_string())
        })
        .collect();
    html::listed_in_order(&expected, actual, "the loose ends in the order")
}

/// **Absent, not disabled and not hidden** — the whole page, not a scoped
/// section, since a reorder control could in principle appear anywhere.
fn then_no_reorder(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    let forbidden = ["Raise priority", "Lower priority", "&#9650;", "&#9660;"];
    for needle in forbidden {
        if body.contains(needle) {
            return Err(format!(
                "expected no reorder control ({needle:?} found), got:\n{body}"
            ));
        }
    }
    Ok(())
}

fn then_way_back(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(r#"href="/""#) && body.contains("Go to Capture") {
        Ok(())
    } else {
        Err(format!("expected a link back to Capture, got:\n{body}"))
    }
}

/// "The pool screen lists ..." (#140, reused from `pool_screen.feature`'s
/// own "trip"/"loose ends" vocabulary by other features that only care
/// whether *something* landed here, not which group it landed in) always
/// fetches `/pool` itself rather than trusting `world.last_html_body` --
/// several call sites check the pool screen and the committed screen back
/// to back in one scenario, and a cached body from whichever screen was
/// fetched last would silently answer for the wrong one.
async fn fetch_pool_body(world: &mut World) -> Result<(), String> {
    view_screen(world, "/pool").await
}

async fn then_pool_lists_nothing(world: &mut World) -> Result<(), String> {
    fetch_pool_body(world).await?;
    let body = html_body(world)?;
    if body.contains("trip-item-text") || body.contains("loose-text") {
        Err(format!(
            "expected the pool screen to list nothing, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

async fn dispatch_pool_lists(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    fetch_pool_body(world).await?;
    let body = html_body(world)?;
    if body.contains(raw_text.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "expected the pool screen to list {raw_text:?}, got:\n{body}"
        ))
    }
}

async fn dispatch_pool_lists_tagged(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    let tag = resolve(example, &caps[2])?;
    fetch_pool_body(world).await?;
    let body = html_body(world)?;
    let row = html::row_containing(body, raw_text.as_str())?;
    if row.contains(tag.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "expected the row for {raw_text:?} to carry the tag {tag:?}, got:\n{row}"
        ))
    }
}

fn header_labels(body: &str) -> Result<Vec<String>, String> {
    let section = html::header_section(body)?;
    Ok(section
        .split("<a href=\"")
        .skip(1)
        .filter_map(|chunk| {
            let after_tag = chunk.split_once('>')?.1;
            Some(after_tag.split("</a>").next()?.trim().to_string())
        })
        .collect())
}

fn dispatch_tabs_exactly(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected: Vec<String> = resolve(example, &caps[1])?
        .split(", ")
        .map(str::to_string)
        .collect();
    let body = html_body(world)?;
    let actual = header_labels(body)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected tabs {expected:?}, got {actual:?}"))
    }
}

fn dispatch_tab_marks_current(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let label = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let section = html::header_section(body)?;
    let needle = format!(r#"aria-current="page">{label}</a>"#);
    if !section.contains(&needle) {
        return Err(format!(
            "expected {label:?} to be marked the current tab, got:\n{section}"
        ));
    }
    let current_count = section.matches(r#"aria-current="page""#).count();
    if current_count != 1 {
        return Err(format!(
            "expected exactly one current tab, found {current_count} in:\n{section}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(
            resolve(&example, "@homedepot"),
            Ok("@homedepot".to_string())
        );
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("trips", "@homedepot")]);
        assert_eq!(resolve(&example, "<trips>"), Ok("@homedepot".to_string()));
    }

    #[test]
    fn resolve_does_not_mistake_hostile_text_for_a_placeholder() {
        let example = BTreeMap::new();
        let hostile = "<script>alert('boom')</script>";
        assert_eq!(resolve(&example, hostile), Ok(hostile.to_string()));
    }

    /// The lone capture's own tag -- every test below that reads it back has
    /// exactly one row to read.
    async fn only_capture_tag(world: &World) -> Option<String> {
        sqlx::query_scalar("SELECT context_tag FROM captures")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn given_task_creates_a_triaged_pool_task_with_a_tag() {
        let mut world = migrated_world().await;

        given_task(
            &mut world,
            "buy screws",
            payloads::pool(),
            Some("@homedepot"),
        )
        .await
        .unwrap();

        assert_eq!(
            only_capture_tag(&world).await.as_deref(),
            Some("@homedepot")
        );
        let kind: String = sqlx::query_scalar("SELECT kind FROM tasks")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(kind, "pool");
    }

    #[tokio::test]
    async fn given_task_with_no_tag_stores_none() {
        let mut world = migrated_world().await;

        given_task(&mut world, "fix the door latch", payloads::pool(), None)
            .await
            .unwrap();

        assert_eq!(only_capture_tag(&world).await, None);
    }

    #[tokio::test]
    async fn dispatch_given_n_pool_tasks_creates_exactly_that_many() {
        let mut world = migrated_world().await;
        let example = BTreeMap::new();
        let re = GIVEN_N_POOL_TAGGED
            .captures(r#""3" pool tasks tagged "@bakery""#)
            .unwrap();

        dispatch_given_n_pool_tasks(&mut world, &example, &re)
            .await
            .unwrap();

        let pool = world.pool().unwrap().clone();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE kind = 'pool'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn trip_tags_in_order_reads_every_trip_label() {
        let body = r#"<div class="trip panel"><div class="trip-header"><div class="trip-tag">@bakery</div></div></div><div class="trip panel"><div class="trip-header"><div class="trip-tag">@attic</div></div></div>"#;
        assert_eq!(
            trip_tags_in_order(body),
            vec!["@bakery".to_string(), "@attic".to_string()]
        );
    }

    fn item_li(text: &str) -> String {
        format!(
            r##"<li><label class="done-check"><input type="checkbox" aria-label="Mark done" hx-post="/pool/tasks/1/done" hx-target="#pool-body" hx-swap="outerHTML"></label><span class="trip-item-text">{text}</span></li>"##
        )
    }

    fn two_trip_body() -> String {
        format!(
            r#"<div class="trip panel"><div class="trip-header"><div class="trip-tag">@attic</div><div class="trip-count">3 things</div></div><ul class="trip-items">{}{}{}</ul></div><div class="trip panel"><div class="trip-header"><div class="trip-tag">@bakery</div><div class="trip-count">4 things</div></div><ul class="trip-items">{}{}{}{}</ul><button type="button" class="trip-more-toggle" data-collapsed-label="Show 1 more">Show 1 more</button></div>"#,
            item_li("b3"),
            item_li("b2"),
            item_li("b1"),
            item_li("a3"),
            item_li("a2"),
            item_li("a1"),
            item_li("a0"),
        )
    }

    #[test]
    fn trip_section_finds_the_named_panel_and_not_another_one() {
        let body = two_trip_body();
        let section = html::trip_section(&body, "@bakery").unwrap();
        assert!(section.contains("4 things"));
        assert!(!section.contains("3 things"));
    }

    #[test]
    fn trip_section_errors_when_no_panel_matches() {
        let body = two_trip_body();
        assert!(html::trip_section(&body, "@cellar").is_err());
    }

    #[test]
    fn dispatch_trip_reads_finds_the_named_trips_own_count() {
        let mut world = World::new();
        world.last_html_body = Some(two_trip_body());
        let example = BTreeMap::new();
        let re = THEN_TRIP_READS
            .captures(r#"the trip "@bakery" reads "4 things""#)
            .unwrap();

        assert_eq!(dispatch_trip_reads(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn list_items_reads_every_item_text_span_in_order() {
        let section = r#"<li><span class="trip-item-text">c</span></li><li><span class="trip-item-text">b</span></li><li><span class="trip-item-text">a</span></li>"#;
        assert_eq!(
            list_items(section),
            vec!["c".to_string(), "b".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn list_items_ignores_the_leading_mark_done_checkbox() {
        let section = r#"<li><label class="done-check"><input type="checkbox" hx-post="/pool/tasks/1/done"></label><span class="trip-item-text">buy screws</span></li>"#;
        assert_eq!(list_items(section), vec!["buy screws".to_string()]);
    }

    #[test]
    fn dispatch_trip_lists_exactly_checks_everything_the_trip_holds() {
        let mut world = World::new();
        world.last_html_body = Some(two_trip_body());
        let example = BTreeMap::new();
        let re = THEN_TRIP_LISTS_EXACTLY
            .captures(r#"the trip "@bakery" lists "a3, a2, a1, a0""#)
            .unwrap();

        assert_eq!(
            dispatch_trip_lists_exactly(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn dispatch_trip_lists_n_items_counts_everything_the_trip_holds() {
        let mut world = World::new();
        world.last_html_body = Some(two_trip_body());
        let example = BTreeMap::new();
        let re = THEN_TRIP_LISTS_N_ITEMS
            .captures(r#"the trip "@bakery" lists "4" items"#)
            .unwrap();

        assert_eq!(
            dispatch_trip_lists_n_items(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn dispatch_trip_offers_reads_the_more_labels_text() {
        let mut world = World::new();
        world.last_html_body = Some(two_trip_body());
        let example = BTreeMap::new();
        let re = THEN_TRIP_OFFERS
            .captures(r#"the trip "@bakery" offers "Show 1 more""#)
            .unwrap();

        assert_eq!(dispatch_trip_offers(&mut world, &example, &re), Ok(()));
    }

    #[test]
    fn then_no_reorder_passes_when_nothing_matches() {
        let mut world = World::new();
        world.last_html_body = Some("<main><h1>Pool</h1></main>".to_string());
        assert_eq!(then_no_reorder(&mut world), Ok(()));
    }

    #[test]
    fn then_no_reorder_errors_when_a_raise_priority_control_exists() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<button aria-label="Raise priority">^</button>"#.to_string());
        assert!(then_no_reorder(&mut world).is_err());
    }

    #[test]
    fn then_way_back_passes_when_the_capture_link_and_its_words_are_present() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<a href="/" class="pool-go-capture">Go to Capture &rarr;</a>"#.to_string());
        assert_eq!(then_way_back(&mut world), Ok(()));
    }

    #[test]
    fn then_way_back_errors_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<main></main>".to_string());
        assert!(then_way_back(&mut world).is_err());
    }

    fn header_body(links: &[(&str, &str, bool)]) -> String {
        let items: String = links
            .iter()
            .map(|(label, path, current)| {
                let attr = if *current {
                    " aria-current=\"page\""
                } else {
                    ""
                };
                format!(r#"<li><a href="{path}"{attr}>{label}</a></li>"#)
            })
            .collect();
        format!(r#"<header><nav><ul>{items}</ul></nav></header><main></main>"#)
    }

    #[test]
    fn header_labels_reads_labels_in_document_order() {
        let body = header_body(&[("Capture", "/", true), ("Pool", "/pool", false)]);
        assert_eq!(
            header_labels(&body).unwrap(),
            vec!["Capture".to_string(), "Pool".to_string()]
        );
    }

    #[test]
    fn dispatch_tabs_exactly_matches_the_resolved_list() {
        let mut world = World::new();
        world.last_html_body = Some(header_body(&[
            ("Capture", "/", true),
            ("Pool", "/pool", false),
        ]));
        let example = BTreeMap::new();
        let re = THEN_TABS_EXACTLY
            .captures(r#"the tab bar offers exactly "Capture, Pool""#)
            .unwrap();

        assert_eq!(dispatch_tabs_exactly(&mut world, &example, &re), Ok(()));
    }

    fn world_with_pool_current() -> World {
        let mut world = World::new();
        world.last_html_body = Some(header_body(&[
            ("Capture", "/", false),
            ("Pool", "/pool", true),
        ]));
        world
    }

    #[test]
    fn dispatch_tab_marks_current_passes_for_the_marked_tab() {
        let mut world = world_with_pool_current();
        let example = BTreeMap::new();
        let re = THEN_TAB_MARKS_CURRENT
            .captures(r#"the tab bar marks "Pool" as the current tab"#)
            .unwrap();

        assert_eq!(
            dispatch_tab_marks_current(&mut world, &example, &re),
            Ok(())
        );
    }

    #[test]
    fn dispatch_tab_marks_current_errors_for_the_wrong_tab() {
        let mut world = world_with_pool_current();
        let example = BTreeMap::new();
        let re = THEN_TAB_MARKS_CURRENT
            .captures(r#"the tab bar marks "Capture" as the current tab"#)
            .unwrap();

        assert!(dispatch_tab_marks_current(&mut world, &example, &re).is_err());
    }
}
