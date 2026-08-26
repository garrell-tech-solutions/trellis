use super::app_client;
use super::payloads;
use super::*;
use serde_json::{json, Value};

static GIVEN_EMPTY_TASK_LIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the trellis server is running with an empty task list$").unwrap()
});
static GIVEN_SERVER_BELIEVES_IT_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the server believes it is "([^"]+)"$"#).unwrap());
static GIVEN_CAPTURE_WAITING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^a capture with raw text "([^"]+)" is waiting in the untriaged queue$"#).unwrap()
});
static WHEN_TRIAGED_AS_POOL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capture is triaged as a pool task$").unwrap());
static WHEN_TRIAGED_AS_COMMITTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"^the capture is triaged as a committed task with a "<(\w+)>" deadline of "<(\w+)>" and priority "<(\w+)>"$"#,
        )
        .unwrap()
});
static WHEN_TRIAGED_AS_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"^the capture is triaged as a quota named "<(\w+)>" with a target of "<(\w+)>" hours a week$"#,
        )
        .unwrap()
});
static WHEN_TRIAGED_AS_COMMITTED_MISSING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the capture is triaged as a committed task with "<(\w+)>" omitted$"#).unwrap()
});
static THEN_KIND_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the resulting task has kind "(\w+)"$"#).unwrap());
static THEN_NO_DEADLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the resulting task has no deadline$").unwrap());
static THEN_HAS_DEADLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the resulting task has a "<(\w+)>" deadline of "<(\w+)>"$"#).unwrap()
});
static THEN_HAS_PRIORITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the resulting task has priority "<(\w+)>"$"#).unwrap());
static THEN_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the triage is rejected$").unwrap());
static THEN_REJECTION_NAMES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection names "([^"]+)"$"#).unwrap());
static THEN_TASK_LIST_EMPTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the task list is still empty$").unwrap());
static THEN_CAPTURE_STILL_WAITING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capture is still waiting in the untriaged queue$").unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if GIVEN_EMPTY_TASK_LIST.is_match(text) {
        return Some(given_empty_task_list(world).await);
    }
    if let Some(caps) = GIVEN_SERVER_BELIEVES_IT_IS.captures(text) {
        return Some(dispatch_server_believes_it_is(world, example, &caps));
    }
    if let Some(caps) = GIVEN_CAPTURE_WAITING.captures(text) {
        return Some(dispatch_capture_waiting(world, example, &caps).await);
    }
    if WHEN_TRIAGED_AS_POOL.is_match(text) {
        return Some(when_triaged(world, payloads::pool()).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AS_COMMITTED.captures(text) {
        return Some(dispatch_triaged_as_committed(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AS_QUOTA.captures(text) {
        return Some(dispatch_triaged_as_quota(world, example, &caps).await);
    }
    if let Some(caps) = WHEN_TRIAGED_AS_COMMITTED_MISSING.captures(text) {
        return Some(dispatch_triaged_committed_missing(world, example, &caps).await);
    }
    if let Some(caps) = THEN_KIND_IS.captures(text) {
        return Some(then_task_has_kind(world, &caps[1]).await);
    }
    if THEN_NO_DEADLINE.is_match(text) {
        return Some(then_task_has_no_deadline(world).await);
    }
    if let Some(caps) = THEN_HAS_DEADLINE.captures(text) {
        return Some(dispatch_has_deadline(world, example, &caps).await);
    }
    if let Some(caps) = THEN_HAS_PRIORITY.captures(text) {
        return Some(dispatch_has_priority(world, example, &caps).await);
    }
    if THEN_REJECTED.is_match(text) {
        return Some(then_triage_is_rejected(world));
    }
    if let Some(caps) = THEN_REJECTION_NAMES.captures(text) {
        return Some(dispatch_rejection_names(world, example, &caps));
    }
    if THEN_TASK_LIST_EMPTY.is_match(text) {
        return Some(then_task_list_is_empty(world).await);
    }
    if THEN_CAPTURE_STILL_WAITING.is_match(text) {
        return Some(then_capture_still_waiting(world).await);
    }
    None
}

async fn dispatch_triaged_as_committed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = example_value(example, &caps[1])?;
    let deadline = example_value(example, &caps[2])?;
    let priority = example_value(example, &caps[3])?;
    let body = json!({
        "kind": "committed",
        "deadline": deadline,
        "commitment": commitment,
        "priority": priority,
        "estimated_minutes": payloads::VALID_ESTIMATED_MINUTES,
        "life_area": payloads::VALID_LIFE_AREA,
    });
    when_triaged(world, body).await
}

async fn dispatch_triaged_as_quota(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = example_value(example, &caps[1])?;
    let hours = example_value(example, &caps[2])?;
    let body = json!({
        "kind": "quota",
        "name": name,
        "hours": hours,
        "life_area": payloads::VALID_LIFE_AREA,
    });
    when_triaged(world, body).await
}

async fn dispatch_triaged_committed_missing(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = example_value(example, &caps[1])?;
    when_triaged_committed_missing(world, missing_field).await
}

async fn dispatch_has_deadline(
    world: &World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let commitment = example_value(example, &caps[1])?;
    let deadline = example_value(example, &caps[2])?;
    then_task_has_deadline(world, commitment, deadline).await
}

async fn dispatch_has_priority(
    world: &World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let priority = example_value(example, &caps[1])?;
    then_task_has_priority(world, priority).await
}

/// See `pool_screen.rs`'s own copy of this function for the full reasoning;
/// duplicated rather than shared per this project's established convention.
/// `given_server_believes_it_is`'s own step is the first caller in this
/// module that names a placeholder (`quota_sessions.feature`'s `"<now>"`)
/// rather than always a literal timestamp -- every step above it happens to
/// have been written with literals only, which is what let the gap stand.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

fn dispatch_rejection_names(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = resolve(example, &caps[1])?;
    then_rejection_names(world, &missing_field)
}

async fn dispatch_capture_waiting(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let raw_text = resolve(example, &caps[1])?;
    given_capture_waiting(world, &raw_text).await
}

fn dispatch_server_believes_it_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let timestamp = resolve(example, &caps[1])?;
    given_server_believes_it_is(world, &timestamp)
}

pub async fn given_empty_task_list(world: &mut World) -> Result<(), String> {
    empty_migrated_database_world(world).await
}

/// Pins `world.pinned_now_ms`, which `world.clock()` -- and so every step
/// module's request-building helper -- reads instead of the real clock
/// (`committed-screen-past-still-shows-03`: "past" and "today" both need a
/// fixed instant to mean anything, not whatever day the suite happens to
/// run on).
pub fn given_server_believes_it_is(world: &mut World, timestamp: &str) -> Result<(), String> {
    let ms = timestamp
        .parse::<jiff::Timestamp>()
        .map_err(|e| format!("bad pinned instant {timestamp:?}: {e}"))?
        .as_millisecond();
    world.pinned_now_ms = Some(ms);
    Ok(())
}

pub async fn given_capture_waiting(world: &mut World, raw_text: &str) -> Result<(), String> {
    let pool = world.pool()?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
    )
    .bind(raw_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("insert capture: {e}"))?;
    world.last_capture_id = Some(id);
    Ok(())
}

fn capture_id(world: &World) -> Result<i64, String> {
    world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())
}

pub async fn when_triaged(world: &mut World, body: Value) -> Result<(), String> {
    let capture_id = capture_id(world)?;
    let uri = format!("/captures/{capture_id}/triage");
    let response = app_client::post_json(world, &uri, &body).await?;

    world.last_status = Some(response.status);
    world.last_response_body = response.body;
    Ok(())
}

pub async fn when_triaged_committed_missing(
    world: &mut World,
    missing_field: &str,
) -> Result<(), String> {
    let body = payloads::without_field(payloads::committed(), missing_field);
    when_triaged(world, body).await
}

/// A triaged task's persisted fields, as read back for assertions.
/// A named struct (rather than the query's positional tuple) so each
/// assertion names the field it cares about instead of counting
/// underscores.
pub(super) struct TaskRow {
    pub(super) kind: String,
    pub(super) deadline: Option<i64>,
    pub(super) commitment: Option<String>,
    pub(super) priority: Option<String>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TaskRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(TaskRow {
            kind: row.try_get("kind")?,
            deadline: row.try_get("deadline")?,
            commitment: row.try_get("commitment")?,
            priority: row.try_get("priority")?,
        })
    }
}

pub(super) async fn task_row(world: &World) -> Result<TaskRow, String> {
    let pool = world.pool()?;
    let capture_id = capture_id(world)?;
    sqlx::query_as("SELECT kind, deadline, commitment, priority FROM tasks WHERE capture_id = ?")
        .bind(capture_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("query resulting task: {e}"))
}

pub async fn then_task_has_kind(world: &World, expected: &str) -> Result<(), String> {
    let row = task_row(world).await?;
    if row.kind == expected {
        Ok(())
    } else {
        Err(format!("expected task kind {expected}, got {}", row.kind))
    }
}

pub async fn then_task_has_no_deadline(world: &World) -> Result<(), String> {
    let row = task_row(world).await?;
    if row.deadline.is_none() {
        Ok(())
    } else {
        Err(format!("expected no deadline, got {:?}", row.deadline))
    }
}

/// `deadline` is stored as epoch milliseconds (T-jiff-epoch-millis); the
/// Gherkin example gives the deadline as text, so the expectation is parsed
/// the same way the triage boundary parses a submission, and compared as the
/// instant it names rather than as text.
pub async fn then_task_has_deadline(
    world: &World,
    expected_commitment: &str,
    expected_deadline: &str,
) -> Result<(), String> {
    let expected_ms = expected_deadline
        .parse::<jiff::Timestamp>()
        .map_err(|e| format!("bad expected deadline {expected_deadline:?}: {e}"))?
        .as_millisecond();
    let row = task_row(world).await?;
    if row.deadline == Some(expected_ms) && row.commitment.as_deref() == Some(expected_commitment) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected_commitment} deadline {expected_deadline} ({expected_ms}ms), \
                 got commitment={:?} deadline={:?}",
            row.commitment, row.deadline
        ))
    }
}

pub async fn then_task_has_priority(world: &World, expected: &str) -> Result<(), String> {
    let row = task_row(world).await?;
    if row.priority.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected priority {expected}, got {:?}",
            row.priority
        ))
    }
}

pub fn then_triage_is_rejected(world: &mut World) -> Result<(), String> {
    match world.last_status {
        Some(422) => Ok(()),
        Some(status) => Err(format!(
            "expected the validation rejection status 422, got {status}"
        )),
        None => Err("no triage response recorded".to_string()),
    }
}

/// Checks the JSON rejection body (the API path) when there is one; a
/// page-originated triage has no JSON body at all — its response is the
/// re-rendered `#lists` fragment, so the fallback checks that HTML for the
/// same rejection prose `triage::http::rejection_message` writes into the
/// failing row (`"{field} is required"`). Both paths report the same fact
/// ("triage-from-page brief: the page and the API share one validation
/// contract"), just through the shape each transport actually returns.
fn rejection_names_in_json(body: &Value, expected_field: &str) -> Result<(), String> {
    match body.get("missing_field").and_then(Value::as_str) {
        Some(field) if field == expected_field => Ok(()),
        other => Err(format!(
            "expected rejection to name {expected_field}, body reported {other:?}"
        )),
    }
}

fn rejection_names_in_html(html: &str, expected_field: &str) -> Result<(), String> {
    let needle = format!("{expected_field} is required");
    if html.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected the page's rejection to say {needle:?}, got:\n{html}"
        ))
    }
}

pub fn then_rejection_names(world: &mut World, expected_field: &str) -> Result<(), String> {
    if let Some(body) = world.last_response_body.clone() {
        return rejection_names_in_json(&body, expected_field);
    }
    if let Some(html) = world.last_html_body.clone() {
        return rejection_names_in_html(&html, expected_field);
    }
    Err("no rejection response recorded".to_string())
}

/// [`then_rejection_names`]'s counterpart for a field that was present but
/// outside its domain. Shared by `committed_field_domains` and
/// `quota_triage_validation`, which both report this way.
pub fn then_rejection_reports_invalid(
    world: &mut World,
    expected_field: &str,
) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("invalid_field").and_then(Value::as_str) {
        Some(field) if field == expected_field => Ok(()),
        other => Err(format!(
            "expected rejection to report {expected_field} as invalid, body reported {other:?}"
        )),
    }
}

pub async fn then_task_list_is_empty(world: &World) -> Result<(), String> {
    let pool = world.pool()?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("count tasks: {e}"))?;
    if count == 0 {
        Ok(())
    } else {
        Err(format!("expected an empty task list, found {count} row(s)"))
    }
}

pub async fn then_capture_still_waiting(world: &World) -> Result<(), String> {
    let pool = world.pool()?;
    let capture_id = capture_id(world)?;
    let left_inbox_at: Option<i64> =
        sqlx::query_scalar("SELECT left_inbox_at FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("query capture: {e}"))?;
    if left_inbox_at.is_none() {
        Ok(())
    } else {
        Err("expected the capture to still be untriaged".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Folded in from the PR #31 review: this used to accept anything in
    /// `400..500`, so a malformed-JSON 400 (an axum extractor failure, never
    /// reaching the domain validation) would satisfy a scenario written to
    /// prove a 422 validation rejection.
    #[test]
    fn then_triage_is_rejected_errors_on_a_400_that_is_not_the_validation_status() {
        let mut world = World::new();
        world.last_status = Some(400);
        assert!(then_triage_is_rejected(&mut world).is_err());
    }

    #[test]
    fn then_triage_is_rejected_passes_on_the_validation_status() {
        let mut world = World::new();
        world.last_status = Some(422);
        assert_eq!(then_triage_is_rejected(&mut world), Ok(()));
    }

    #[test]
    fn resolve_returns_a_literal_timestamp_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(
            resolve(&example, "2026-08-25T14:00:00Z"),
            Ok("2026-08-25T14:00:00Z".to_string())
        );
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("now", "2026-08-25T14:00:00Z")]);
        assert_eq!(
            resolve(&example, "<now>"),
            Ok("2026-08-25T14:00:00Z".to_string())
        );
    }

    /// The bug the quota-sessions brief named: `given_server_believes_it_is`
    /// used to receive the captured text raw, so a step written as `the
    /// server believes it is "<now>"` pinned the literal string `"<now>"`
    /// rather than looking it up -- failing with "bad pinned instant" rather
    /// than the missing-step error a reader would expect.
    #[tokio::test]
    async fn dispatch_resolves_the_now_placeholder_before_pinning_the_clock() {
        let mut world = migrated_world().await;
        let example = super::super::example(&[("now", "2026-08-25T14:00:00Z")]);

        let outcome = dispatch(&mut world, r#"the server believes it is "<now>""#, &example).await;

        assert_eq!(outcome, Some(Ok(())));
        assert_eq!(world.pinned_now_ms, Some(1787666400000));
    }

    #[tokio::test]
    async fn given_capture_waiting_records_the_capture_id() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        assert!(world.last_capture_id.is_some());
    }

    #[tokio::test]
    async fn pool_triage_round_trips_through_the_task_assertions() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(&mut world, payloads::pool()).await.unwrap();

        then_task_has_kind(&world, "pool").await.unwrap();
        then_task_has_no_deadline(&world).await.unwrap();
    }

    #[tokio::test]
    async fn committed_triage_round_trips_through_the_task_assertions() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(&mut world, payloads::committed())
            .await
            .unwrap();

        then_task_has_kind(&world, "committed").await.unwrap();
        then_task_has_deadline(&world, "at", payloads::VALID_DEADLINE)
            .await
            .unwrap();
        then_task_has_priority(&world, "P1").await.unwrap();
    }

    #[tokio::test]
    async fn quota_triage_round_trips_through_the_task_assertions() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(&mut world, payloads::quota()).await.unwrap();

        then_task_has_kind(&world, "quota").await.unwrap();
        then_task_has_no_deadline(&world).await.unwrap();
    }

    #[tokio::test]
    async fn missing_field_triage_is_rejected_and_leaves_no_trace() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "call the dentist")
            .await
            .unwrap();
        when_triaged_committed_missing(&mut world, "deadline")
            .await
            .unwrap();

        then_triage_is_rejected(&mut world).unwrap();
        then_rejection_names(&mut world, "deadline").unwrap();
        then_task_list_is_empty(&world).await.unwrap();
        then_capture_still_waiting(&world).await.unwrap();
    }

    #[tokio::test]
    async fn invalid_field_triage_is_rejected_and_leaves_no_trace() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "call the dentist")
            .await
            .unwrap();
        let payload = payloads::with_field(payloads::committed(), "commitment", json!("squishy"));
        when_triaged(&mut world, payload).await.unwrap();

        then_triage_is_rejected(&mut world).unwrap();
        then_rejection_reports_invalid(&mut world, "commitment").unwrap();
        then_task_list_is_empty(&world).await.unwrap();
        then_capture_still_waiting(&world).await.unwrap();
    }
}
