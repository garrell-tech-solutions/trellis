//! Step handlers for `features/schedule.feature` (#75, M3 slice 1 of 5).
//!
//! The Background, "the server believes it is ...", "the trellis server is
//! running with an empty task list", "the life area ... is saved with a
//! guardrail band ..." and "the life area ... is saved as never scheduled"
//! steps this feature also uses are already matched generically by
//! [`super::triage::dispatch`], [`super::guardrails::dispatch`] and
//! [`super::free_time::dispatch`], tried before this module.
//!
//! Fixture triage reuses [`super::capacity::insert_capture`] and
//! [`super::capacity::triage_fixture`] rather than a second copy of
//! "insert a throwaway capture, triage it, fail loud if refused".

use super::capacity::triage_fixture;
use super::html;
use super::inbox_view::html_response;
use super::life_areas::resolve;
use super::payloads;
use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::Value;

static GIVEN_COMMITTED_LONG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a committed task "([^"]+)" in life area "([^"]+)" estimated "([^"]+)" minutes due "([^"]+)" as "([^"]+)" with priority "([^"]+)"$"#,
    )
    .unwrap()
});
static GIVEN_COMMITTED_SHORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a committed task "([^"]+)" in life area "([^"]+)" needs "([^"]+)" minutes by "([^"]+)"(?: is triaged)?$"#,
    )
    .unwrap()
});
static GIVEN_POOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a pool task "([^"]+)" in life area "([^"]+)" is triaged$"#).unwrap()
});
static GIVEN_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^a quota task "([^"]+)" in life area "([^"]+)" targeting "([^"]+)" sessions of "([^"]+)" minutes per "([^"]+)" is triaged$"#,
    )
    .unwrap()
});
static WHEN_GENERATED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the schedule is generated$").unwrap());
static WHEN_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the schedule page is viewed$").unwrap());
static THEN_PLACES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the schedule places "([^"]+)" starting "([^"]+)" and ending "([^"]+)"$"#)
        .unwrap()
});
static THEN_REPORTS_PLACED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the schedule reports "([^"]+)" placed blocks$"#).unwrap());
static THEN_FINISHES_AFTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the schedule reports "([^"]+)" as finishing after its deadline$"#).unwrap()
});
static THEN_UNPLACEABLE_BECAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the schedule reports "([^"]+)" as unplaceable because "([^"]+)"$"#).unwrap()
});
static THEN_DOES_NOT_MENTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the schedule does not mention "([^"]+)"$"#).unwrap());
static THEN_EMPTY_STATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the schedule shows an empty-state message$").unwrap());
static THEN_NO_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the schedule page does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_CONTAINS_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the schedule page contains the word "([^"]+)"$"#).unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_COMMITTED_LONG.captures(text) {
        return Some(dispatch_committed_long(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_COMMITTED_SHORT.captures(text) {
        return Some(dispatch_committed_short(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_POOL.captures(text) {
        return Some(dispatch_pool(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_QUOTA.captures(text) {
        return Some(dispatch_quota(world, example, &caps).await);
    }
    if WHEN_GENERATED.is_match(text) {
        return Some(when_schedule_generated(world).await);
    }
    if WHEN_VIEWED.is_match(text) {
        return Some(when_schedule_viewed(world).await);
    }
    if let Some(caps) = THEN_PLACES.captures(text) {
        return Some(dispatch_places(world, example, &caps));
    }
    if let Some(caps) = THEN_REPORTS_PLACED.captures(text) {
        return Some(dispatch_reports_placed(world, example, &caps));
    }
    if let Some(caps) = THEN_FINISHES_AFTER.captures(text) {
        return Some(dispatch_finishes_after(world, example, &caps));
    }
    if let Some(caps) = THEN_UNPLACEABLE_BECAUSE.captures(text) {
        return Some(dispatch_unplaceable_because(world, example, &caps));
    }
    if let Some(caps) = THEN_DOES_NOT_MENTION.captures(text) {
        return Some(dispatch_does_not_mention(world, example, &caps));
    }
    if THEN_EMPTY_STATE.is_match(text) {
        return Some(then_html_body_contains(world, "Nothing to schedule"));
    }
    if THEN_NO_SCRIPT.is_match(text) {
        return Some(then_no_unescaped_script(world));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(dispatch_contains_word(world, example, &caps));
    }
    None
}

async fn committed_fixture(
    world: &mut World,
    text: &str,
    life_area: &str,
    estimated_minutes: &str,
    deadline: &str,
    deadline_type: &str,
    priority: &str,
) -> Result<(), String> {
    let body = payloads::committed();
    let body = payloads::with_field(body, "life_area", Value::from(life_area));
    let body = payloads::with_field(
        body,
        "estimated_minutes",
        Value::from(
            estimated_minutes
                .parse::<i64>()
                .map_err(|e| format!("bad estimated minutes {estimated_minutes:?}: {e}"))?,
        ),
    );
    let body = payloads::with_field(body, "deadline", Value::from(deadline));
    let body = payloads::with_field(body, "deadline_type", Value::from(deadline_type));
    let body = payloads::with_field(body, "priority", Value::from(priority));
    triage_fixture(world, text, body).await
}

/// The task text and life area every `Given a ... task` regex captures in
/// its first two groups, resolved together -- the two-line prologue
/// [`dispatch_committed_long`], [`dispatch_committed_short`], [`dispatch_pool`]
/// and [`dispatch_quota`] all otherwise repeated identically.
fn resolve_text_and_life_area(
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(String, String), String> {
    Ok((resolve(example, &caps[1])?, resolve(example, &caps[2])?))
}

async fn dispatch_committed_long(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (text, life_area) = resolve_text_and_life_area(example, caps)?;
    let estimated_minutes = resolve(example, &caps[3])?;
    let deadline = resolve(example, &caps[4])?;
    let deadline_type = resolve(example, &caps[5])?;
    let priority = resolve(example, &caps[6])?;
    committed_fixture(
        world,
        &text,
        &life_area,
        &estimated_minutes,
        &deadline,
        &deadline_type,
        &priority,
    )
    .await
}

/// The short form implies a soft deadline at P2 (the feature file's own
/// header: "the uninteresting case").
async fn dispatch_committed_short(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (text, life_area) = resolve_text_and_life_area(example, caps)?;
    let estimated_minutes = resolve(example, &caps[3])?;
    let deadline = resolve(example, &caps[4])?;
    committed_fixture(
        world,
        &text,
        &life_area,
        &estimated_minutes,
        &deadline,
        "soft",
        "P2",
    )
    .await
}

async fn dispatch_pool(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (text, life_area) = resolve_text_and_life_area(example, caps)?;
    let body = payloads::with_field(payloads::pool(), "life_area", Value::from(life_area));
    triage_fixture(world, &text, body).await
}

async fn dispatch_quota(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let (text, life_area) = resolve_text_and_life_area(example, caps)?;
    let target_count = super::capacity::resolved_i64(example, &caps[3], "target_count")?;
    let target_minutes_each =
        super::capacity::resolved_i64(example, &caps[4], "target_minutes_each")?;
    let period = resolve(example, &caps[5])?;
    let body = payloads::quota();
    let body = payloads::with_field(body, "life_area", Value::from(life_area));
    let body = payloads::with_field(body, "target_count", Value::from(target_count));
    let body = payloads::with_field(
        body,
        "target_minutes_each",
        Value::from(target_minutes_each),
    );
    let body = payloads::with_field(body, "period", Value::from(period));
    triage_fixture(world, &text, body).await
}

async fn when_schedule_generated(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .method("POST")
        .uri("/schedule/generate")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

async fn when_schedule_viewed(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .uri("/schedule")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no schedule response recorded")
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no schedule response recorded")
}

fn placed_section(world: &World) -> Result<&str, String> {
    html::between(html_body(world)?, r#"<ul id="schedule-placed">"#, "</ul>")
}

fn unplaceable_section(world: &World) -> Result<&str, String> {
    html::between(
        html_body(world)?,
        r#"<ul id="schedule-unplaceable">"#,
        "</ul>",
    )
}

/// The `<li>...</li>` markup for the row naming `needle`, found by content
/// rather than position -- the same shape `app_shell`'s own
/// `header_link_html` uses to find one link among several.
/// The `<li>` markup's own start and end offsets around whichever
/// occurrence of `needle` `at` names -- [`row_containing`]'s bounds-finding
/// half, split out from locating `needle` itself.
fn li_bounds(section: &str, at: usize, needle: &str) -> Result<(usize, usize), String> {
    let start = section[..at]
        .rfind("<li>")
        .ok_or_else(|| format!("malformed row markup near {needle:?}"))?;
    let end = section[start..]
        .find("</li>")
        .ok_or_else(|| format!("unterminated row near {needle:?}"))?;
    Ok((start, end))
}

fn row_containing<'a>(section: &'a str, needle: &str) -> Result<&'a str, String> {
    let at = section
        .find(needle)
        .ok_or_else(|| format!("expected {needle:?} in:\n{section}"))?;
    let (start, end) = li_bounds(section, at, needle)?;
    Ok(&section[start..start + end + "</li>".len()])
}

fn placed_row<'a>(world: &'a World, text: &str) -> Result<&'a str, String> {
    row_containing(placed_section(world)?, text)
}

fn unplaceable_row<'a>(world: &'a World, text: &str) -> Result<&'a str, String> {
    row_containing(unplaceable_section(world)?, text)
}

/// The literal every `Then` regex in this module resolves its first
/// capture group as -- the row-naming task text, whichever section it is
/// later looked up in.
fn resolve_text(
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<String, String> {
    resolve(example, &caps[1])
}

fn dispatch_places(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve_text(example, caps)?;
    let start = resolve(example, &caps[2])?;
    let end = resolve(example, &caps[3])?;
    let row = placed_row(world, &text)?;
    super::then_section_contains(row, &text, "start at", &start)?;
    super::then_section_contains(row, &text, "end at", &end)
}

fn dispatch_reports_placed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let count = resolve(example, &caps[1])?;
    then_html_body_contains(world, &format!("{count} placed"))
}

fn dispatch_finishes_after(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve_text(example, caps)?;
    let row = placed_row(world, &text)?;
    super::then_section_contains(row, &text, "report", "finishes after its deadline")
}

fn dispatch_unplaceable_because(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve_text(example, caps)?;
    let reason = resolve(example, &caps[2])?;
    let row = unplaceable_row(world, &text)?;
    super::then_section_contains(row, &text, "report", &reason)
}

fn dispatch_does_not_mention(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve_text(example, caps)?;
    super::then_html_body_excludes(world, &text, "no schedule response recorded")
}

fn then_no_unescaped_script(world: &mut World) -> Result<(), String> {
    let placed = placed_section(world)?;
    let unplaceable = unplaceable_section(world)?;
    if placed.contains("<script>") || unplaceable.contains("<script>") {
        Err(format!(
            "expected no <script> tag in the schedule, got:\n{placed}\n{unplaceable}"
        ))
    } else {
        Ok(())
    }
}

fn dispatch_contains_word(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let word = resolve(example, &caps[1])?;
    let section = placed_section(world)?;
    if section.contains(&word) {
        Ok(())
    } else {
        Err(format!(
            "expected {word:?} in the schedule, got:\n{section}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_containing_finds_the_row_naming_the_needle() {
        let section =
            "<li>write the Q3 deck (Work) — 2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z</li><li>other</li>";
        let row = row_containing(section, "write the Q3 deck").unwrap();
        assert!(row.starts_with("<li>write the Q3 deck"));
        assert!(row.ends_with("</li>"));
        assert!(!row.contains("other"));
    }

    #[test]
    fn row_containing_errors_when_the_needle_is_absent() {
        let section = "<li>something else</li>";
        assert!(row_containing(section, "missing").is_err());
    }

    /// The placed row `dispatch_places`'s own tests share: "write the Q3
    /// deck", 09:00-11:00Z -- only the example each test resolves against
    /// it differs.
    fn world_with_the_q3_deck_placed_row() -> World {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-placed"><li>write the Q3 deck — 2026-08-17T09:00:00Z to 2026-08-17T11:00:00Z</li></ul>"#
                .to_string(),
        );
        world
    }

    #[test]
    fn dispatch_places_passes_when_the_row_names_both_instants() {
        let mut world = world_with_the_q3_deck_placed_row();
        let ex = example(&[
            ("start", "2026-08-17T09:00:00Z"),
            ("end", "2026-08-17T11:00:00Z"),
        ]);
        let caps = THEN_PLACES
            .captures(
                r#"the schedule places "write the Q3 deck" starting "<start>" and ending "<end>""#,
            )
            .unwrap();

        assert_eq!(dispatch_places(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_places_errors_when_the_row_is_missing_the_end_instant() {
        let mut world = world_with_the_q3_deck_placed_row();
        let ex = example(&[
            ("start", "2026-08-17T09:00:00Z"),
            ("end", "2026-08-17T12:00:00Z"),
        ]);
        let caps = THEN_PLACES
            .captures(
                r#"the schedule places "write the Q3 deck" starting "<start>" and ending "<end>""#,
            )
            .unwrap();

        assert!(dispatch_places(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn dispatch_reports_placed_reads_the_summary_line() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<p id="schedule-summary">2 placed</p>"#.to_string());
        let ex = example(&[("placed", "2")]);
        let caps = THEN_REPORTS_PLACED
            .captures(r#"the schedule reports "<placed>" placed blocks"#)
            .unwrap();

        assert_eq!(dispatch_reports_placed(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_reports_placed_errors_when_the_summary_names_a_different_count() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<p id="schedule-summary">1 placed</p>"#.to_string());
        let ex = example(&[("placed", "2")]);
        let caps = THEN_REPORTS_PLACED
            .captures(r#"the schedule reports "<placed>" placed blocks"#)
            .unwrap();

        assert!(dispatch_reports_placed(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn dispatch_finishes_after_finds_the_overrun_report_in_the_placed_row() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-placed"><li>renew the passport — report: finishes after its deadline</li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_FINISHES_AFTER
            .captures(
                r#"the schedule reports "renew the passport" as finishing after its deadline"#,
            )
            .unwrap();

        assert_eq!(dispatch_finishes_after(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_finishes_after_errors_when_the_row_carries_no_overrun_report() {
        let mut world = World::new();
        world.last_html_body =
            Some(r#"<ul id="schedule-placed"><li>renew the passport</li></ul>"#.to_string());
        let ex = example(&[]);
        let caps = THEN_FINISHES_AFTER
            .captures(
                r#"the schedule reports "renew the passport" as finishing after its deadline"#,
            )
            .unwrap();

        assert!(dispatch_finishes_after(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn dispatch_unplaceable_because_finds_the_reason_in_the_wont_fit_row() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-unplaceable"><li>rebuild the deck — unplaceable: chunk_policy_unsatisfiable</li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_UNPLACEABLE_BECAUSE
            .captures(
                r#"the schedule reports "rebuild the deck" as unplaceable because "chunk_policy_unsatisfiable""#,
            )
            .unwrap();

        assert_eq!(dispatch_unplaceable_because(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_unplaceable_because_errors_when_the_row_names_a_different_reason() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-unplaceable"><li>rebuild the deck — unplaceable: no_window</li></ul>"#
                .to_string(),
        );
        let ex = example(&[]);
        let caps = THEN_UNPLACEABLE_BECAUSE
            .captures(
                r#"the schedule reports "rebuild the deck" as unplaceable because "chunk_policy_unsatisfiable""#,
            )
            .unwrap();

        assert!(dispatch_unplaceable_because(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn dispatch_does_not_mention_passes_when_the_text_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some("<p>Nothing to schedule.</p>".to_string());
        let ex = example(&[]);
        let caps = THEN_DOES_NOT_MENTION
            .captures(r#"the schedule does not mention "read the spec""#)
            .unwrap();

        assert_eq!(dispatch_does_not_mention(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_does_not_mention_errors_when_the_text_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("<p>read the spec</p>".to_string());
        let ex = example(&[]);
        let caps = THEN_DOES_NOT_MENTION
            .captures(r#"the schedule does not mention "read the spec""#)
            .unwrap();

        assert!(dispatch_does_not_mention(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn then_no_unescaped_script_passes_when_neither_section_carries_one() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-placed"><li>boom</li></ul><ul id="schedule-unplaceable"></ul>"#
                .to_string(),
        );
        assert_eq!(then_no_unescaped_script(&mut world), Ok(()));
    }

    #[test]
    fn then_no_unescaped_script_errors_when_the_placed_section_carries_one() {
        let mut world = World::new();
        world.last_html_body = Some(
            r#"<ul id="schedule-placed"><li>&lt;script&gt;alert('boom')&lt;/script&gt;<script>alert('boom')</script></li></ul><ul id="schedule-unplaceable"></ul>"#
                .to_string(),
        );
        assert!(then_no_unescaped_script(&mut world).is_err());
    }

    #[test]
    fn dispatch_contains_word_finds_the_word_in_the_placed_section() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="schedule-placed"><li>boom</li></ul>"#.to_string());
        let ex = example(&[]);
        let caps = THEN_CONTAINS_WORD
            .captures(r#"the schedule page contains the word "boom""#)
            .unwrap();

        assert_eq!(dispatch_contains_word(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn dispatch_contains_word_errors_when_the_word_is_absent() {
        let mut world = World::new();
        world.last_html_body = Some(r#"<ul id="schedule-placed"><li>bust</li></ul>"#.to_string());
        let ex = example(&[]);
        let caps = THEN_CONTAINS_WORD
            .captures(r#"the schedule page contains the word "boom""#)
            .unwrap();

        assert!(dispatch_contains_word(&mut world, &ex, &caps).is_err());
    }

    #[test]
    fn then_html_body_contains_passes_when_the_body_carries_the_expected_text() {
        let mut world = World::new();
        world.last_html_body = Some("<p>Nothing to schedule.</p>".to_string());

        assert_eq!(
            then_html_body_contains(&mut world, "Nothing to schedule"),
            Ok(())
        );
    }

    #[test]
    fn then_html_body_contains_errors_when_the_body_lacks_the_expected_text() {
        let mut world = World::new();
        world.last_html_body = Some("<p>2 placed</p>".to_string());

        assert!(then_html_body_contains(&mut world, "Nothing to schedule").is_err());
    }

    #[tokio::test]
    async fn dispatch_pool_creates_a_pool_task_in_the_named_life_area() {
        let mut world = migrated_world().await;
        let ex = example(&[]);
        let caps = GIVEN_POOL
            .captures(r#"a pool task "read the spec" in life area "Work" is triaged"#)
            .unwrap();

        dispatch_pool(&mut world, &ex, &caps).await.unwrap();

        let task: (String,) = sqlx::query_as("SELECT kind FROM tasks LIMIT 1")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(task.0, "pool");
    }

    #[tokio::test]
    async fn dispatch_quota_records_the_recurring_target() {
        let mut world = migrated_world().await;
        let ex = example(&[]);
        let caps = GIVEN_QUOTA
            .captures(
                r#"a quota task "swim laps" in life area "Work" targeting "3" sessions of "45" minutes per "week" is triaged"#,
            )
            .unwrap();

        dispatch_quota(&mut world, &ex, &caps).await.unwrap();

        let task: (String, i64, i64, String) = sqlx::query_as(
            "SELECT kind, target_count, target_minutes_each, period FROM tasks LIMIT 1",
        )
        .fetch_one(world.pool().unwrap())
        .await
        .unwrap();
        assert_eq!(task, ("quota".to_string(), 3, 45, "week".to_string()));
    }

    #[tokio::test]
    async fn dispatch_committed_long_places_a_task_through_the_long_form() {
        let mut world = migrated_world().await;
        world.pinned_now_ms = Some(1_755_421_200_000);
        let ex = example(&[
            ("deadline", "2026-08-21T17:00:00Z"),
            ("deadline_type", "hard"),
        ]);
        let caps = GIVEN_COMMITTED_LONG
            .captures(
                r#"a committed task "write the Q3 deck" in life area "Work" estimated "120" minutes due "<deadline>" as "<deadline_type>" with priority "P2""#,
            )
            .unwrap();

        dispatch_committed_long(&mut world, &ex, &caps)
            .await
            .unwrap();

        let task: (String,) = sqlx::query_as("SELECT deadline_type FROM tasks LIMIT 1")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(task.0, "hard");
    }

    #[tokio::test]
    async fn dispatch_committed_short_implies_a_soft_p2_deadline() {
        let mut world = migrated_world().await;
        let ex = example(&[]);
        let caps = GIVEN_COMMITTED_SHORT
            .captures(
                r#"a committed task "renew the passport" in life area "Work" needs "120" minutes by "2026-08-21T17:00:00Z""#,
            )
            .unwrap();

        dispatch_committed_short(&mut world, &ex, &caps)
            .await
            .unwrap();

        let task: (String, String) =
            sqlx::query_as("SELECT deadline_type, priority FROM tasks LIMIT 1")
                .fetch_one(world.pool().unwrap())
                .await
                .unwrap();
        assert_eq!(task, ("soft".to_string(), "P2".to_string()));
    }
}
