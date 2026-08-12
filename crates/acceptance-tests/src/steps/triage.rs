use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::{json, Value};
use tower::ServiceExt;

static GIVEN_EMPTY_TASK_LIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the trellis server is running with an empty task list$").unwrap()
});
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
            r#"^the capture is triaged as a quota task targeting "<(\w+)>" sessions of "<(\w+)>" minutes per "week"$"#,
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
static THEN_NO_QUOTA_TARGET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the resulting task has no quota target$").unwrap());
static THEN_HAS_DEADLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the resulting task has a "<(\w+)>" deadline of "<(\w+)>"$"#).unwrap()
});
static THEN_HAS_PRIORITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the resulting task has priority "<(\w+)>"$"#).unwrap());
static THEN_HAS_QUOTA_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"^the resulting task has a quota target of "<(\w+)>" sessions of "<(\w+)>" minutes per "week"$"#,
        )
        .unwrap()
});
static THEN_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the triage is rejected$").unwrap());
static THEN_REJECTION_NAMES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the rejection names "<(\w+)>"$"#).unwrap());
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
    if let Some(caps) = GIVEN_CAPTURE_WAITING.captures(text) {
        return Some(given_capture_waiting(world, &caps[1]).await);
    }
    if WHEN_TRIAGED_AS_POOL.is_match(text) {
        return Some(when_triaged(world, json!({ "kind": "pool" })).await);
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
    if THEN_NO_QUOTA_TARGET.is_match(text) {
        return Some(then_task_has_no_quota_target(world).await);
    }
    if let Some(caps) = THEN_HAS_DEADLINE.captures(text) {
        return Some(dispatch_has_deadline(world, example, &caps).await);
    }
    if let Some(caps) = THEN_HAS_PRIORITY.captures(text) {
        return Some(dispatch_has_priority(world, example, &caps).await);
    }
    if let Some(caps) = THEN_HAS_QUOTA_TARGET.captures(text) {
        return Some(dispatch_has_quota_target(world, example, &caps).await);
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

/// Parses a Gherkin example value expected to hold an integer, naming
/// `field` in the error so a bad fixture points back at its source.
fn parse_i64(field: &str, value: &str) -> Result<i64, String> {
    value.parse().map_err(|e| format!("bad {field}: {e}"))
}

async fn dispatch_triaged_as_committed(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let deadline_type = example_value(example, &caps[1])?;
    let deadline = example_value(example, &caps[2])?;
    let priority = example_value(example, &caps[3])?;
    let body = json!({
        "kind": "committed",
        "deadline": deadline,
        "deadline_type": deadline_type,
        "priority": priority,
    });
    when_triaged(world, body).await
}

async fn dispatch_triaged_as_quota(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let target_count = parse_i64("target_count", example_value(example, &caps[1])?)?;
    let target_minutes_each = parse_i64("target_minutes_each", example_value(example, &caps[2])?)?;
    let body = json!({
        "kind": "quota",
        "target_count": target_count,
        "target_minutes_each": target_minutes_each,
        "period": "week",
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
    let deadline_type = example_value(example, &caps[1])?;
    let deadline = example_value(example, &caps[2])?;
    then_task_has_deadline(world, deadline_type, deadline).await
}

async fn dispatch_has_priority(
    world: &World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let priority = example_value(example, &caps[1])?;
    then_task_has_priority(world, priority).await
}

async fn dispatch_has_quota_target(
    world: &World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let target_count = example_value(example, &caps[1])?;
    let target_minutes_each = example_value(example, &caps[2])?;
    then_task_has_quota_target(world, target_count, target_minutes_each).await
}

fn dispatch_rejection_names(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let missing_field = example_value(example, &caps[1])?;
    then_rejection_names(world, missing_field)
}

pub async fn given_empty_task_list(world: &mut World) -> Result<(), String> {
    empty_migrated_database_world(world).await
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
    let pool = world.pool()?.clone();
    let capture_id = capture_id(world)?;
    let app = trellis_server::app::build_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/captures/{capture_id}/triage"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .map_err(|e| format!("build request: {e}"))?,
        )
        .await
        .map_err(|e| format!("send request: {e}"))?;

    world.last_status = Some(response.status().as_u16());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| format!("read response body: {e}"))?;
    world.last_response_body = serde_json::from_slice(&bytes).ok();
    Ok(())
}

pub async fn when_triaged_committed_missing(
    world: &mut World,
    missing_field: &str,
) -> Result<(), String> {
    let mut body = json!({
        "kind": "committed",
        "deadline": "2026-08-20T17:00:00Z",
        "deadline_type": "hard",
        "priority": "P1",
    });
    body.as_object_mut()
        .expect("committed payload is an object")
        .remove(missing_field);
    when_triaged(world, body).await
}

/// A triaged task's persisted fields, as read back for assertions.
/// A named struct (rather than the query's positional tuple) so each
/// assertion names the field it cares about instead of counting
/// underscores.
pub(super) struct TaskRow {
    pub(super) kind: String,
    pub(super) deadline: Option<i64>,
    pub(super) deadline_type: Option<String>,
    pub(super) priority: Option<String>,
    pub(super) target_count: Option<i64>,
    pub(super) target_minutes_each: Option<i64>,
    pub(super) period: Option<String>,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TaskRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(TaskRow {
            kind: row.try_get("kind")?,
            deadline: row.try_get("deadline")?,
            deadline_type: row.try_get("deadline_type")?,
            priority: row.try_get("priority")?,
            target_count: row.try_get("target_count")?,
            target_minutes_each: row.try_get("target_minutes_each")?,
            period: row.try_get("period")?,
        })
    }
}

pub(super) async fn task_row(world: &World) -> Result<TaskRow, String> {
    let pool = world.pool()?;
    let capture_id = capture_id(world)?;
    sqlx::query_as(
            "SELECT kind, deadline, deadline_type, priority, target_count, target_minutes_each, period \
             FROM tasks WHERE capture_id = ?",
        )
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

pub async fn then_task_has_no_quota_target(world: &World) -> Result<(), String> {
    let row = task_row(world).await?;
    if row.target_count.is_none() && row.target_minutes_each.is_none() && row.period.is_none() {
        Ok(())
    } else {
        Err(format!(
            "expected no quota target, got target_count={:?} \
                 target_minutes_each={:?} period={:?}",
            row.target_count, row.target_minutes_each, row.period
        ))
    }
}

/// `deadline` is stored as epoch milliseconds (T3); the Gherkin example gives
/// the deadline as text, so the expectation is parsed the same way the
/// triage boundary parses a submission, and compared as the instant it
/// names rather than as text.
pub async fn then_task_has_deadline(
    world: &World,
    expected_type: &str,
    expected_deadline: &str,
) -> Result<(), String> {
    let expected_ms = expected_deadline
        .parse::<jiff::Timestamp>()
        .map_err(|e| format!("bad expected deadline {expected_deadline:?}: {e}"))?
        .as_millisecond();
    let row = task_row(world).await?;
    if row.deadline == Some(expected_ms) && row.deadline_type.as_deref() == Some(expected_type) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected_type} deadline {expected_deadline} ({expected_ms}ms), \
                 got deadline_type={:?} deadline={:?}",
            row.deadline_type, row.deadline
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

pub async fn then_task_has_quota_target(
    world: &World,
    expected_count: &str,
    expected_minutes_each: &str,
) -> Result<(), String> {
    let row = task_row(world).await?;
    let expected_count = parse_i64("expected target_count", expected_count)?;
    let expected_minutes_each = parse_i64("expected target_minutes_each", expected_minutes_each)?;
    if row.target_count == Some(expected_count)
        && row.target_minutes_each == Some(expected_minutes_each)
        && row.period.as_deref() == Some("week")
    {
        Ok(())
    } else {
        Err(format!(
            "expected quota target {expected_count}x{expected_minutes_each}min per week, \
                 got target_count={:?} target_minutes_each={:?} period={:?}",
            row.target_count, row.target_minutes_each, row.period
        ))
    }
}

pub fn then_triage_is_rejected(world: &mut World) -> Result<(), String> {
    match world.last_status {
        Some(status) if (400..500).contains(&status) => Ok(()),
        Some(status) => Err(format!("expected a client error, got status {status}")),
        None => Err("no triage response recorded".to_string()),
    }
}

pub fn then_rejection_names(world: &mut World, expected_field: &str) -> Result<(), String> {
    let body = world
        .last_response_body
        .as_ref()
        .ok_or_else(|| "no rejection body recorded".to_string())?;
    match body.get("missing_field").and_then(Value::as_str) {
        Some(field) if field == expected_field => Ok(()),
        other => Err(format!(
            "expected rejection to name {expected_field}, body reported {other:?}"
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
    let triaged_at: Option<i64> =
        sqlx::query_scalar("SELECT triaged_at FROM captures WHERE id = ?")
            .bind(capture_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("query capture: {e}"))?;
    if triaged_at.is_none() {
        Ok(())
    } else {
        Err("expected the capture to still be untriaged".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        when_triaged(&mut world, json!({ "kind": "pool" }))
            .await
            .unwrap();

        then_task_has_kind(&world, "pool").await.unwrap();
        then_task_has_no_deadline(&world).await.unwrap();
        then_task_has_no_quota_target(&world).await.unwrap();
    }

    #[tokio::test]
    async fn committed_triage_round_trips_through_the_task_assertions() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(
            &mut world,
            json!({
                "kind": "committed",
                "deadline": "2026-08-20T17:00:00Z",
                "deadline_type": "hard",
                "priority": "P1",
            }),
        )
        .await
        .unwrap();

        then_task_has_kind(&world, "committed").await.unwrap();
        then_task_has_deadline(&world, "hard", "2026-08-20T17:00:00Z")
            .await
            .unwrap();
        then_task_has_priority(&world, "P1").await.unwrap();
        then_task_has_no_quota_target(&world).await.unwrap();
    }

    #[tokio::test]
    async fn quota_triage_round_trips_through_the_task_assertions() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "buy milk").await.unwrap();
        when_triaged(
            &mut world,
            json!({
                "kind": "quota",
                "target_count": 3,
                "target_minutes_each": 45,
                "period": "week",
            }),
        )
        .await
        .unwrap();

        then_task_has_kind(&world, "quota").await.unwrap();
        then_task_has_quota_target(&world, "3", "45").await.unwrap();
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
}
