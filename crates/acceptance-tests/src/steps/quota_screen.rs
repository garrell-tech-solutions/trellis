//! Step handlers for `features/quota_screen.feature`: the fourth screen,
//! and defining a quota (#93).
//!
//! "the "quota" screen is viewed" and "the tab bar marks ... as the current
//! tab" are already matched generically by [`super::pool_screen::dispatch`],
//! tried before this module — nothing here duplicates them.

use super::html;
use super::inbox_view::{html_response, urlencode};
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
static THEN_DEFINE_CONTROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the quota screen offers a define control named "([^"]+)"$"#).unwrap()
});
static WHEN_DEFINED_OMITTING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^a quota is defined with "([^"]+)" omitted$"#).unwrap());
static THEN_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the definition is rejected$").unwrap());
/// The `Given`/`And` fixture form -- "a quota named X exists", with no
/// trailing "is defined" -- distinct from [`WHEN_DEFINED`]'s action form,
/// which the feature file spells with "is defined" precisely because it is
/// the step under test.
static GIVEN_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a quota named "([^"]+)" with a target of "([^"]+)" hours a week$"#).unwrap()
});
static WHEN_DEFINED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a quota named "([^"]+)" with a target of "([^"]+)" hours a week is defined$"#)
        .unwrap()
});
static WHEN_DEFINED_AGAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a quota named "([^"]+)" with a target of "([^"]+)" hours a week is defined again$"#,
    )
    .unwrap()
});
static THEN_QUOTA_READS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota "([^"]+)" reads "([^"]+)"$"#).unwrap());
static THEN_QUOTA_NOTES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the quota "([^"]+)" notes "([^"]+)"$"#).unwrap());
static THEN_FORM_WARNS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the new-quota form warns "([^"]+)"$"#).unwrap());
static THEN_FORM_CREATE_CONTROL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the new-quota form offers a create control named "([^"]+)"$"#).unwrap()
});
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
        return Some(dispatch_meta(world, example, &caps));
    }
    if THEN_OFFERS_NO_QUOTAS.is_match(text) {
        return Some(then_offers_no_quotas(world));
    }
    if let Some(caps) = THEN_SCREEN_NOTES.captures(text) {
        return Some(dispatch_screen_notes(world, example, &caps));
    }
    if let Some(caps) = THEN_DEFINE_CONTROL.captures(text) {
        return Some(dispatch_define_control(world, example, &caps));
    }
    if let Some(caps) = WHEN_DEFINED_OMITTING.captures(text) {
        return Some(dispatch_defined_omitting(world, example, &caps).await);
    }
    if THEN_REJECTED.is_match(text) {
        return Some(super::then_status_is(
            world,
            422,
            "no quota definition response recorded",
        ));
    }
    if let Some(caps) = WHEN_DEFINED_AGAIN.captures(text) {
        return Some(dispatch_defined_again(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_DEFINED.captures(text) {
        return Some(dispatch_defined(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_QUOTA.captures(text) {
        return Some(dispatch_defined(world, example, &caps).await);
    }
    if let Some(caps) = THEN_QUOTA_READS.captures(text) {
        return Some(dispatch_quota_reads(world, example, &caps));
    }
    if let Some(caps) = THEN_QUOTA_NOTES.captures(text) {
        return Some(dispatch_quota_notes(world, example, &caps));
    }
    if let Some(caps) = THEN_FORM_WARNS.captures(text) {
        return Some(dispatch_form_warns(world, example, &caps));
    }
    if let Some(caps) = THEN_FORM_CREATE_CONTROL.captures(text) {
        return Some(dispatch_form_create_control(world, example, &caps));
    }
    if let Some(caps) = THEN_OFFERS_QUOTAS.captures(text) {
        return Some(dispatch_offers_quotas(world, example, &caps));
    }
    if let Some(caps) = THEN_DOES_NOT_MENTION.captures(text) {
        return Some(
            html_body(world).and_then(|body| super::then_does_not_mention(body, &caps[1])),
        );
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

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no quota screen response recorded")
}

/// The shared "does the extracted text match" verdict every comparison
/// dispatcher in this module ends on -- same condition, differing only in
/// `mismatch`, each call site's own description of what it was comparing.
/// Built eagerly rather than lazily (a plain `String`, not a closure): a
/// closure defined inline counts as a nested space whose own complexity
/// rolls back into the caller that defines it, so it would not have moved
/// the branch out of the dispatcher at all.
fn expect_eq(actual: &str, expected: &str, mismatch: String) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(mismatch)
    }
}

fn dispatch_meta(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
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
fn then_offers_no_quotas(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(r#"<div class="quota-row""#) {
        Err(format!("expected no quotas, got:\n{body}"))
    } else {
        Ok(())
    }
}

fn dispatch_screen_notes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let note = html::between(body, r#"<div class="quota-empty">"#, "</div>")?;
    let note = html::between(note, "<p>", "</p>")?.trim();
    expect_eq(
        note,
        &expected,
        format!("expected the quota screen to note {expected:?}, got {note:?}"),
    )
}

fn dispatch_define_control(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let (_, label) = html::button(body, r#"class="quota-define-submit""#)?;
    expect_eq(
        label,
        &expected,
        format!("expected the define control named {expected:?}, got {label:?}"),
    )
}

/// Posts a `application/x-www-form-urlencoded` body to `path` -- the same
/// transport `triage_from_page.rs`'s page forms use, since the quota define
/// form is a plain HTML form rather than JSON.
async fn post_form(world: &mut World, path: &str, fields: &[(&str, &str)]) -> Result<(), String> {
    let body = fields
        .iter()
        .map(|(name, value)| format!("{name}={}", urlencode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

async fn dispatch_defined_omitting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing = resolve(example, &caps[1])?;
    let mut fields: Vec<(&str, &str)> = vec![("name", "Piano"), ("hours", "4")];
    fields.retain(|(name, _)| *name != missing);
    post_form(world, "/quota", &fields).await
}

async fn dispatch_defined(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let hours = resolve(example, &caps[2])?;
    post_form(world, "/quota", &[("name", &name), ("hours", &hours)]).await
}

/// The "Create anyway" resubmission: the same name and hours, plus the
/// `confirmed` field the button's own hidden input carries
/// (`quota-screen-similar-name-warns-07`).
async fn dispatch_defined_again(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let hours = resolve(example, &caps[2])?;
    post_form(
        world,
        "/quota",
        &[("name", &name), ("hours", &hours), ("confirmed", &name)],
    )
    .await
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

fn dispatch_quota_reads(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    let readout = html::between(row, r#"<div class="quota-readout">"#, "</div>")?;
    expect_eq(
        readout,
        &expected,
        format!("expected the quota {name:?} to read {expected:?}, got {readout:?}"),
    )
}

fn dispatch_quota_notes(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (name, expected) = resolve_pair(example, caps)?;
    let body = html_body(world)?;
    let row = html::quota_row(body, &name)?;
    let note = html::between(row, r#"<div class="quota-note">"#, "</div>")?;
    expect_eq(
        note,
        &expected,
        format!("expected the quota {name:?} to note {expected:?}, got {note:?}"),
    )
}

fn dispatch_form_warns(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let warning = html::between(body, r#"<p class="quota-warning">"#, "</p>")?;
    expect_eq(
        warning,
        &expected,
        format!("expected the new-quota form to warn {expected:?}, got {warning:?}"),
    )
}

fn dispatch_form_create_control(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
    let (_, label) = html::button(body, r#"class="quota-define-submit""#)?;
    expect_eq(
        label,
        &expected,
        format!("expected a create control named {expected:?}, got {label:?}"),
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

fn dispatch_offers_quotas(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let expected = resolve(example, &caps[1])?;
    let body = html_body(world)?;
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

    #[test]
    fn then_offers_no_quotas_passes_when_the_list_is_absent_entirely() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<div class="quota-empty"></div>"#.to_string());
        then_offers_no_quotas(&mut world).unwrap();
    }

    #[test]
    fn then_offers_no_quotas_errors_when_a_row_exists() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<div class="quota-rows"><div class="quota-row"><div class="quota-name">Piano</div></div></div>"#
                .to_string(),
        );
        assert!(then_offers_no_quotas(&mut world).is_err());
    }

    #[test]
    fn quota_row_finds_the_named_row_and_not_another_one() {
        let body = r#"<div class="quota-rows"><div class="quota-row"><div class="quota-name">Piano</div><div class="quota-readout">0m / 4h</div></div><div class="quota-row"><div class="quota-name">Running</div><div class="quota-readout">0m / 3h</div></div></div>"#;
        let row = html::quota_row(body, "Piano").unwrap();
        assert!(row.contains("0m / 4h"));
        assert!(!row.contains("0m / 3h"));
    }
}
