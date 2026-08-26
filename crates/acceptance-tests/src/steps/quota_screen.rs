//! Step handlers for `features/quota_screen.feature`: the fourth screen,
//! showing what was triaged as a quota (#138).
//!
//! "the "quota" screen is viewed" and "the tab bar marks ... as the current
//! tab" are already matched generically by [`super::pool_screen::dispatch`],
//! tried before this module — nothing here duplicates them.

use super::html;
use super::inbox_view::html_response;
use super::*;
use axum::body::Body;
use axum::http::Request;

static THEN_META: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the quota screen reports "([^"]+)" beside its title$"#).unwrap()
});
static THEN_OFFERS_NO_QUOTAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the quota screen offers no quotas$").unwrap());
static THEN_SCREEN_NOTES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota screen notes "([^"]+)"$"#).unwrap());
static THEN_WAY_BACK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the quota screen offers a way back to Capture$").unwrap());
static THEN_NO_DEFINE_CONTROL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the quota screen offers no way to define a quota$").unwrap());
/// The fixture form -- "a quota named X with a target of Y hours a week" --
/// creates the quota directly through the store rather than through the
/// page: #138 retired the one route (`POST /quota`) that could have done
/// this by request, since triage is now the only door.
static GIVEN_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a quota named "([^"]+)" with a target of "([^"]+)" hours a week$"#).unwrap()
});
static THEN_QUOTA_READS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota "([^"]+)" reads "([^"]+)"$"#).unwrap());
static THEN_QUOTA_NOTES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota "([^"]+)" notes "([^"]+)"$"#).unwrap());
static THEN_OFFERS_QUOTAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota screen offers the quotas "([^"]+)"$"#).unwrap());
static THEN_DOES_NOT_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota screen does not mention "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_META.captures(text) {
        return Some(dispatch_meta(world, example, &caps).await);
    }
    if THEN_OFFERS_NO_QUOTAS.is_match(text) {
        return Some(then_offers_no_quotas(world).await);
    }
    if let Some(caps) = THEN_SCREEN_NOTES.captures(text) {
        return Some(dispatch_screen_notes(world, example, &caps).await);
    }
    if THEN_WAY_BACK.is_match(text) {
        return Some(then_way_back(world).await);
    }
    if THEN_NO_DEFINE_CONTROL.is_match(text) {
        return Some(then_no_define_control(world).await);
    }
    if let Some(caps) = GIVEN_QUOTA.captures(text) {
        return Some(dispatch_given_quota(world, example, &caps).await);
    }
    if let Some(caps) = THEN_QUOTA_READS.captures(text) {
        return Some(dispatch_quota_reads(world, example, &caps).await);
    }
    if let Some(caps) = THEN_QUOTA_NOTES.captures(text) {
        return Some(dispatch_quota_notes(world, example, &caps).await);
    }
    if let Some(caps) = THEN_OFFERS_QUOTAS.captures(text) {
        return Some(dispatch_offers_quotas(world, example, &caps).await);
    }
    if let Some(caps) = THEN_DOES_NOT_MENTION.captures(text) {
        let expected = caps[1].to_string();
        return Some(match html_body(world).await {
            Ok(body) => super::then_does_not_mention(body, &expected),
            Err(e) => Err(e),
        });
    }
    None
}

/// See `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// The last recorded quota screen response, fetching `GET /quota` first if
/// nothing has viewed it yet this scenario. Several scenarios assert this
/// screen's state as a *consequence* of a triage or migration they just ran
/// through a different endpoint entirely, without an explicit "the "quota"
/// screen is viewed" step of their own -- fetching on demand is what lets
/// the same `Then` wording work whether or not one preceded it.
async fn html_body(world: &mut World) -> Result<&str, String> {
    if world.last_html_body.is_none() {
        let request = Request::builder()
            .uri("/quota")
            .body(Body::empty())
            .map_err(|e| format!("build request: {e}"))?;
        html_response(world, request).await?;
    }
    super::html_body(world, "no quota screen response recorded")
}

/// The shared "does the extracted text match" verdict every comparison
/// dispatcher in this module ends on -- same condition, differing only in
/// `mismatch`, each call site's own description of what it was comparing.
fn expect_eq(actual: &str, expected: &str, mismatch: String) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(mismatch)
    }
}

async fn dispatch_meta(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world).await?;
    let meta = html::between(body, r#"<div class="quota-meta">"#, "</div>")?.trim();
    expect_eq(
        meta,
        &expected,
        format!("expected the quota screen to report {expected:?}, got {meta:?}"),
    )
}

/// Absence of the row marker itself, not a delimited "quota-rows" section:
/// a row nests its own `<ul>`/`<li>` session list
/// (`html::quota_row`'s own reasoning), so scanning for *a* closing tag to
/// bound an empty check the same way risks stopping inside the first row
/// found rather than answering "is there one at all".
async fn then_offers_no_quotas(world: &mut World) -> Result<(), String> {
    let body = html_body(world).await?;
    if body.contains(r#"<div class="quota-row""#) {
        Err(format!("expected no quotas, got:\n{body}"))
    } else {
        Ok(())
    }
}

async fn dispatch_screen_notes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world).await?;
    let note = html::between(body, r#"<div class="quota-empty">"#, "</div>")?;
    let note = html::between(note, "<p>", "</p>")?.trim();
    expect_eq(
        note,
        &expected,
        format!("expected the quota screen to note {expected:?}, got {note:?}"),
    )
}

/// `pool-screen-empty-07`'s own shape: an empty quota screen is a dead end
/// with no define control here, so it points at Capture the same way.
async fn then_way_back(world: &mut World) -> Result<(), String> {
    let body = html_body(world).await?;
    if body.contains(r#"<a href="/" class="quota-go-capture">Go to Capture &rarr;</a>"#) {
        Ok(())
    } else {
        Err(format!("expected a way back to Capture, got:\n{body}"))
    }
}

/// #138: the quota screen defines nothing -- the absence is asserted
/// because the owner settled it, unlike the reorder arrows' absence
/// elsewhere on this screen, which remains undecided.
async fn then_no_define_control(world: &mut World) -> Result<(), String> {
    let body = html_body(world).await?;
    if body.contains("quota-define-form") {
        Err(format!("expected no way to define a quota, got:\n{body}"))
    } else {
        Ok(())
    }
}

/// Creates a quota directly through `quotas`, the fixture's own door since
/// `POST /quota` no longer exists (#138: triage is the one door). Hours to
/// minutes by the same rounding `scheduler_core::quota::to_minutes` uses --
/// this crate has no dependency on `scheduler-core` to call it directly, and
/// every fixture hours value in this feature is already a whole or
/// half-hour, so the rounding never has anything to do.
async fn dispatch_given_quota(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let hours = resolve(example, &caps[2])?;
    let hours: f64 = hours
        .parse()
        .map_err(|e| format!("bad fixture hours {hours:?}: {e}"))?;
    let minutes = (hours * 60.0).round() as i64;
    let pool = world.pool()?;
    sqlx::query("INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES (?, ?, 0)")
        .bind(&name)
        .bind(minutes)
        .execute(pool)
        .await
        .map_err(|e| format!("create fixture quota {name:?}: {e}"))?;
    Ok(())
}

/// `caps[1]` and `caps[2]` resolved together -- the "which quota, what do we
/// expect of it" pair [`dispatch_quota_reads`] and [`dispatch_quota_notes`]
/// both start with, one `?` in the caller rather than two.
fn resolve_pair(
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(String, String), String> {
    Ok((resolve(example, &caps[1])?, resolve(example, &caps[2])?))
}

async fn dispatch_quota_reads(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world).await?;
    let row = html::quota_row(body, &name)?;
    let readout = html::between(row, r#"<div class="quota-readout">"#, "</div>")?;
    expect_eq(
        readout,
        &expected,
        format!("expected the quota {name:?} to read {expected:?}, got {readout:?}"),
    )
}

async fn dispatch_quota_notes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world).await?;
    let row = html::quota_row(body, &name)?;
    let note = html::between(row, r#"<div class="quota-note">"#, "</div>")?;
    expect_eq(
        note,
        &expected,
        format!("expected the quota {name:?} to note {expected:?}, got {note:?}"),
    )
}

/// Every `.quota-name` label's text, in document order — what "the quota
/// screen offers the quotas ..." means.
fn quota_names_in_order(body: &str) -> Vec<String> {
    body.split(r#"<div class="quota-name">"#)
        .skip(1)
        .filter_map(|chunk| chunk.split_once("</div>").map(|(name, _)| name.to_string()))
        .collect()
}

async fn dispatch_offers_quotas(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world).await?;
    html::listed_in_order(&expected, quota_names_in_order(body), "quotas")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "Piano"), Ok("Piano".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("hours", "4")]);
        assert_eq!(resolve(&example, "<hours>"), Ok("4".to_string()));
    }

    #[test]
    fn quota_names_in_order_reads_every_quota_label() {
        let body = r#"<div class="quota-row"><div class="quota-name">Piano</div></div><div class="quota-row"><div class="quota-name">Running</div></div>"#;
        assert_eq!(
            quota_names_in_order(body),
            vec!["Piano".to_string(), "Running".to_string()]
        );
    }

    #[tokio::test]
    async fn then_offers_no_quotas_passes_when_the_list_is_absent_entirely() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div class="quota-empty"></div>"#.to_string());
        then_offers_no_quotas(&mut world).await.unwrap();
    }

    #[tokio::test]
    async fn then_offers_no_quotas_errors_when_a_row_exists() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<div class="quota-rows"><div class="quota-row"><div class="quota-name">Piano</div></div></div>"#
                .to_string(),
        );
        assert!(then_offers_no_quotas(&mut world).await.is_err());
    }

    #[test]
    fn quota_row_finds_the_named_row_and_not_another_one() {
        let body = r#"<div class="quota-rows"><div class="quota-row"><div class="quota-name">Piano</div><div class="quota-readout">0m / 4h</div></div><div class="quota-row"><div class="quota-name">Running</div><div class="quota-readout">0m / 3h</div></div></div>"#;
        let row = html::quota_row(body, "Piano").unwrap();
        assert!(row.contains("0m / 4h"));
        assert!(!row.contains("0m / 3h"));
    }

    #[tokio::test]
    async fn then_way_back_passes_when_the_capture_link_and_its_words_are_present() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<a href="/" class="quota-go-capture">Go to Capture &rarr;</a>"#.to_string());
        assert_eq!(then_way_back(&mut world).await, Ok(()));
    }

    #[tokio::test]
    async fn then_way_back_errors_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<main></main>".to_string());
        assert!(then_way_back(&mut world).await.is_err());
    }

    #[tokio::test]
    async fn then_no_define_control_passes_when_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<main></main>".to_string());
        assert_eq!(then_no_define_control(&mut world).await, Ok(()));
    }

    #[tokio::test]
    async fn then_no_define_control_errors_when_present() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<form class="quota-define-form">"#.to_string());
        assert!(then_no_define_control(&mut world).await.is_err());
    }
}
