//! Step handlers for `features/guardrails.feature` (a life area's own
//! weekly guardrail) and `features/timezone_setting.feature` (the owner's
//! one timezone) -- the same slice, #59, and the timezone setting only
//! exists to make guardrails meaningful, so one module covers both.
//!
//! The Background and "the life area(s) page is viewed" steps both features
//! use are already matched generically by [`super::triage::dispatch`] and
//! [`super::life_areas::dispatch`], tried before this module.

use super::html;
use super::inbox_view::html_response;
use super::life_areas::{life_area_id_by_name, resolve};
use super::*;
use axum::body::Body;
use axum::http::Request;

static THEN_EVERY_SHOWS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^every life area shows "([^"]+)"$"#).unwrap());
static WHEN_SAVED_WITH_BAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the life area "([^"]+)" is saved with a guardrail band on "([^"]+)" from "([^"]+)" to "([^"]+)"$"#,
    )
    .unwrap()
});
static WHEN_SAVED_NO_WEEKDAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the life area "([^"]+)" is saved with a guardrail band on no weekday from "([^"]+)" to "([^"]+)"$"#,
    )
    .unwrap()
});
static WHEN_SAVED_STARTING_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life area "([^"]+)" is saved with a guardrail band starting at "([^"]+)"$"#)
        .unwrap()
});
static WHEN_SAVED_POOL_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life area "([^"]+)" is saved as pool-only$"#).unwrap());
static WHEN_SAVED_NEITHER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^the life area "([^"]+)" is saved with neither a guardrail band nor the pool-only mark$"#,
    )
    .unwrap()
});
static THEN_SAVE_ACCEPTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the save is accepted$").unwrap());
static THEN_SAVE_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the save is rejected$").unwrap());
static THEN_SHOWS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life area "([^"]+)" shows "([^"]+)"$"#).unwrap());
static THEN_SHOWS_BAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life area "([^"]+)" shows the guardrail band "([^"]+)"$"#).unwrap()
});
static THEN_SHOWS_BAND_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life area "([^"]+)" shows "([^"]+)" guardrail bands$"#).unwrap()
});
static THEN_ROW_MENTIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the rejection message on the life area's row mentions "([^"]+)"$"#).unwrap()
});
static THEN_SAYS_OVERLAPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the rejection says the band overlaps one the life area already has$").unwrap()
});
static WHEN_BAND_REMOVED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the guardrail band "([^"]+)" is removed from the life area "([^"]+)"$"#).unwrap()
});

static THEN_OWNER_TIMEZONE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the page reports the owner's timezone as "([^"]+)"$"#).unwrap()
});
static WHEN_TIMEZONE_SET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the owner's timezone is set to "([^"]+)"$"#).unwrap());
static THEN_CHANGE_ACCEPTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the change is accepted$").unwrap());
static THEN_CHANGE_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the change is rejected$").unwrap());
static THEN_REJECTION_SAYS_NOT_A_TIMEZONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection says "([^"]+)" is not a timezone$"#).unwrap());
static THEN_PAGE_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the life areas page does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_PAGE_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the life areas page contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = THEN_EVERY_SHOWS.captures(text) {
        return Some(dispatch_every_shows(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_SAVED_WITH_BAND.captures(text) {
        return Some(dispatch_saved_with_band(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_SAVED_NO_WEEKDAY.captures(text) {
        return Some(dispatch_saved_no_weekday(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_SAVED_STARTING_AT.captures(text) {
        return Some(dispatch_saved_starting_at(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_SAVED_POOL_ONLY.captures(text) {
        return Some(dispatch_saved_pool_only(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_SAVED_NEITHER.captures(text) {
        return Some(dispatch_saved_neither(world, example, &caps).await);
    }
    if THEN_SAVE_ACCEPTED.is_match(text) {
        return Some(then_status_is_in(world, 200..300));
    }
    if THEN_SAVE_REJECTED.is_match(text) {
        return Some(super::then_status_is(
            world,
            422,
            "no save response recorded",
        ));
    }
    if let Some(caps) = THEN_SHOWS_BAND.captures(text) {
        return Some(dispatch_shows_band(world, example, &caps).await);
    }
    if let Some(caps) = THEN_SHOWS_BAND_COUNT.captures(text) {
        return Some(dispatch_shows_band_count(world, example, &caps).await);
    }
    if let Some(caps) = THEN_SHOWS.captures(text) {
        return Some(dispatch_shows(world, example, &caps).await);
    }
    if let Some(caps) = THEN_ROW_MENTIONS.captures(text) {
        return Some(dispatch_row_mentions(world, example, &caps));
    }
    if THEN_SAYS_OVERLAPS.is_match(text) {
        return Some(then_body_contains(
            world,
            "the band overlaps one the life area already has",
        ));
    }
    if let Some(caps) = WHEN_BAND_REMOVED.captures(text) {
        return Some(dispatch_band_removed(world, example, &caps).await);
    }
    if let Some(caps) = THEN_OWNER_TIMEZONE.captures(text) {
        return Some(dispatch_owner_timezone(world, example, &caps));
    }
    if let Some(caps) = WHEN_TIMEZONE_SET.captures(text) {
        return Some(dispatch_timezone_set(world, example, &caps).await);
    }
    if THEN_CHANGE_ACCEPTED.is_match(text) {
        return Some(then_status_is_in(world, 200..300));
    }
    if THEN_CHANGE_REJECTED.is_match(text) {
        return Some(super::then_status_is(
            world,
            422,
            "no timezone response recorded",
        ));
    }
    if let Some(caps) = THEN_REJECTION_SAYS_NOT_A_TIMEZONE.captures(text) {
        return Some(dispatch_rejection_says_not_a_timezone(
            world, example, &caps,
        ));
    }
    if THEN_PAGE_NO_SCRIPT.is_match(text) {
        return Some(then_body_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_PAGE_CONTAINS_WORD.captures(text) {
        return Some(then_body_contains(world, &caps[1]));
    }
    None
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no page response recorded")
}

fn then_status_is_in(world: &mut World, range: std::ops::Range<u16>) -> Result<(), String> {
    match world.last_status {
        Some(status) if range.contains(&status) => Ok(()),
        Some(status) => Err(format!("expected a status in {range:?}, got {status}")),
        None => Err("no response recorded".to_string()),
    }
}

fn then_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} in the response, got:\n{body}"
        ))
    }
}

fn then_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the response, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

fn life_area_row_section(body: &str, id: i64) -> Result<&str, String> {
    let start_tag = format!(r#"<li id="life-area-row-{id}">"#);
    html::between(body, &start_tag, "</li>")
}

async fn life_area_row<'a>(world: &'a World, name: &str) -> Result<(i64, &'a str), String> {
    let id = life_area_id_by_name(world, name).await?;
    let body = html_body(world)?;
    let section = life_area_row_section(body, id)?;
    Ok((id, section))
}

/// `"mon"`/`"tue"`/... -- the checkbox field name `save_guardrail`'s form
/// accepts, or `None` for anything that does not name a weekday.
/// The step's day names paired with `save_guardrail`'s own checkbox field
/// names -- a table, not a match, so a seventh day's-worth of branches does
/// not count against `T-complexity-8` for carrying no logic beyond the
/// lookup itself.
const DAY_FIELDS: [(&str, &str); 7] = [
    ("Mon", "mon"),
    ("Tue", "tue"),
    ("Wed", "wed"),
    ("Thu", "thu"),
    ("Fri", "fri"),
    ("Sat", "sat"),
    ("Sun", "sun"),
];

fn day_field_name(day: &str) -> Option<&'static str> {
    let day = day.trim();
    DAY_FIELDS
        .iter()
        .find(|(name, _)| *name == day)
        .map(|(_, field)| *field)
}

fn day_fields(days: &str) -> Vec<(&'static str, &'static str)> {
    days.split(',')
        .filter_map(day_field_name)
        .map(|field| (field, "on"))
        .collect()
}

async fn post_guardrail(
    world: &mut World,
    life_area_id: i64,
    fields: &[(&str, &str)],
) -> Result<(), String> {
    let body = fields
        .iter()
        .map(|(name, value)| format!("{name}={}", super::inbox_view::urlencode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let request = Request::builder()
        .method("POST")
        .uri(format!("/life-areas/{life_area_id}/guardrail"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

/// One row's own contribution to [`dispatch_every_shows`]'s check, split
/// out so the loop it runs in carries only the looping.
fn row_shows(body: &str, id: i64, name: &str, state: &str) -> Result<(), String> {
    let section = life_area_row_section(body, id)?;
    if section.contains(state) {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to show {state:?}, got:\n{section}"
        ))
    }
}

async fn dispatch_every_shows(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let state = resolve(example, &caps[1])?;
    let pool = world.pool()?.clone();
    let rows = trellis_server::life_areas::store::list_active(&pool)
        .await
        .map_err(|e| format!("list life areas: {e}"))?;
    let body = html_body(world)?.to_string();
    for row in rows {
        row_shows(&body, row.id, &row.name, &state)?;
    }
    Ok(())
}

async fn dispatch_saved_with_band(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let days = resolve(example, &caps[2])?;
    let start = resolve(example, &caps[3])?;
    let end = resolve(example, &caps[4])?;
    let id = life_area_id_by_name(world, &name).await?;
    let mut fields = day_fields(&days);
    fields.push(("start", &start));
    fields.push(("end", &end));
    post_guardrail(world, id, &fields).await
}

async fn dispatch_saved_no_weekday(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let start = resolve(example, &caps[2])?;
    let end = resolve(example, &caps[3])?;
    let id = life_area_id_by_name(world, &name).await?;
    post_guardrail(world, id, &[("start", &start), ("end", &end)]).await
}

/// The hostile-text scenario names only a start value -- Monday and a
/// well-formed end carry the submission far enough for the times check to
/// be the one that fires and refuses it.
async fn dispatch_saved_starting_at(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let start = resolve(example, &caps[2])?;
    let id = life_area_id_by_name(world, &name).await?;
    post_guardrail(
        world,
        id,
        &[("mon", "on"), ("start", &start), ("end", "17:00")],
    )
    .await
}

async fn dispatch_saved_pool_only(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let id = life_area_id_by_name(world, &name).await?;
    post_guardrail(world, id, &[("pool_only", "on")]).await
}

async fn dispatch_saved_neither(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let id = life_area_id_by_name(world, &name).await?;
    post_guardrail(world, id, &[]).await
}

async fn dispatch_shows(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let state = resolve(example, &caps[2])?;
    let (_, section) = life_area_row(world, &name).await?;
    if section.contains(&state) {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to show {state:?}, got:\n{section}"
        ))
    }
}

async fn dispatch_shows_band(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let listed = resolve(example, &caps[2])?;
    let (_, section) = life_area_row(world, &name).await?;
    if section.contains(&listed) {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to show the guardrail band {listed:?}, got:\n{section}"
        ))
    }
}

async fn dispatch_shows_band_count(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = resolve(example, &caps[1])?;
    let expected: usize = resolve(example, &caps[2])?
        .parse()
        .map_err(|e| format!("bad band count: {e}"))?;
    let (_, section) = life_area_row(world, &name).await?;
    let actual = section.matches("/guardrail-bands/").count();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "expected {name:?} to show {expected} guardrail bands, got {actual} in:\n{section}"
        ))
    }
}

fn dispatch_row_mentions(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let option = resolve(example, &caps[1])?;
    then_body_contains(world, &option)
}

/// The id `store::remove_guardrail_band` takes -- found by matching the
/// band's own rendered label rather than assuming which member of the group
/// the template happened to pick as representative.
fn band_id_by_label(section: &str, label: &str) -> Result<i64, String> {
    for chunk in section.split("<div>").skip(1) {
        if chunk.contains(label) {
            if let Some(id) = extract_band_id(chunk) {
                return Ok(id);
            }
        }
    }
    Err(format!(
        "no guardrail band labelled {label:?} in:\n{section}"
    ))
}

fn extract_band_id(chunk: &str) -> Option<i64> {
    let after = chunk.split_once("/guardrail-bands/")?.1;
    let (id_str, _) = after.split_once('/')?;
    id_str.parse().ok()
}

async fn dispatch_band_removed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let listed = resolve(example, &caps[1])?;
    let name = resolve(example, &caps[2])?;
    let (_, section) = life_area_row(world, &name).await?;
    let band_id = band_id_by_label(section, &listed)?;
    let request = Request::builder()
        .method("POST")
        .uri(format!("/guardrail-bands/{band_id}/remove"))
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn dispatch_owner_timezone(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let zone = resolve(example, &caps[1])?;
    then_body_contains(world, &zone)
}

async fn dispatch_timezone_set(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let zone = resolve(example, &caps[1])?;
    let body = format!("zone={}", super::inbox_view::urlencode(&zone));
    let request = Request::builder()
        .method("POST")
        .uri("/timezone")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn dispatch_rejection_says_not_a_timezone(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let zone = resolve(example, &caps[1])?;
    then_body_contains(world, &format!("{zone} is not a timezone"))
}

#[cfg(test)]
mod tests {
    use super::super::life_areas::when_life_areas_page_viewed;
    use super::*;

    fn example(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn day_fields_reads_a_comma_separated_list() {
        assert_eq!(
            day_fields("Mon, Wed, Fri"),
            vec![("mon", "on"), ("wed", "on"), ("fri", "on")]
        );
    }

    #[test]
    fn day_fields_is_empty_for_text_naming_no_weekday() {
        assert_eq!(day_fields("no weekday"), Vec::<(&str, &str)>::new());
    }

    #[test]
    fn band_id_by_label_finds_the_matching_bands_id() {
        let section = concat!(
            r#"<div class="guardrail-bands">"#,
            r#"<div>Mon 09:00-12:00<form hx-post="/guardrail-bands/7/remove"></form></div>"#,
            r#"<div>Mon 12:00-17:00<form hx-post="/guardrail-bands/9/remove"></form></div>"#,
            r#"</div>"#,
        );
        assert_eq!(band_id_by_label(section, "Mon 12:00-17:00"), Ok(9));
    }

    #[test]
    fn band_id_by_label_errors_when_no_band_matches() {
        let section =
            r#"<div>Mon 09:00-12:00<form hx-post="/guardrail-bands/7/remove"></form></div>"#;
        assert!(band_id_by_label(section, "Tue 09:00-12:00").is_err());
    }

    #[test]
    fn then_status_is_in_passes_for_a_status_inside_the_range() {
        let mut world = World::new();
        world.last_status = Some(201);
        assert_eq!(then_status_is_in(&mut world, 200..300), Ok(()));
    }

    #[test]
    fn then_status_is_in_errors_for_a_status_outside_the_range() {
        let mut world = World::new();
        world.last_status = Some(422);
        assert!(then_status_is_in(&mut world, 200..300).is_err());
    }

    #[tokio::test]
    async fn saving_a_band_through_the_step_lists_it_on_the_life_areas_page() {
        let mut world = migrated_world().await;
        when_life_areas_page_viewed(&mut world).await.unwrap();

        dispatch_saved_with_band(
            &mut world,
            &example(&[("days", "Mon, Wed"), ("start", "09:00"), ("end", "10:00")]),
            &WHEN_SAVED_WITH_BAND
                .captures(
                    r#"the life area "Work" is saved with a guardrail band on "<days>" from "<start>" to "<end>""#,
                )
                .unwrap(),
        )
        .await
        .unwrap();

        then_status_is_in(&mut world, 200..300).unwrap();
        let body = html_body(&world).unwrap();
        assert!(body.contains("Mon, Wed 09:00-10:00"), "got:\n{body}");
    }

    #[tokio::test]
    async fn dispatch_shows_finds_the_named_life_areas_own_state() {
        let mut world = migrated_world().await;
        when_life_areas_page_viewed(&mut world).await.unwrap();

        let result = dispatch_shows(
            &mut world,
            &example(&[("name", "Work"), ("state", "no guardrail")]),
            &THEN_SHOWS
                .captures(r#"the life area "Work" shows "no guardrail""#)
                .unwrap(),
        )
        .await;

        assert_eq!(result, Ok(()));
    }

    #[tokio::test]
    async fn timezone_round_trips_through_the_dispatch_helpers() {
        let mut world = migrated_world().await;

        dispatch_timezone_set(
            &mut world,
            &BTreeMap::new(),
            &WHEN_TIMEZONE_SET
                .captures(r#"the owner's timezone is set to "Europe/London""#)
                .unwrap(),
        )
        .await
        .unwrap();

        then_status_is_in(&mut world, 200..300).unwrap();
        dispatch_owner_timezone(
            &mut world,
            &BTreeMap::new(),
            &THEN_OWNER_TIMEZONE
                .captures(r#"the page reports the owner's timezone as "Europe/London""#)
                .unwrap(),
        )
        .unwrap();
    }
}
