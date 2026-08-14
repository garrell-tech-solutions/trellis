//! Step handlers for `features/stats_ratio.feature`: `GET /stats`, the
//! rolling committed:pool ratio (issue #45, R2's instrumentation).
//!
//! Fixture tasks are inserted directly against the schema, the same way
//! [`super::triage::given_capture_waiting`] does -- a `store.rs` write would
//! need a full, valid triage submission per kind, when all these scenarios
//! need is a `kind` and a `created_at_ms` to land in or out of the window.
//! Kept as its own module rather than folded into [`super::triage::dispatch`],
//! which the complexity gate is already red on (handoff brief gotcha #5).

use super::*;
use axum::body::Body;
use axum::http::Request;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

static GIVEN_TASKS_TRIAGED_TODAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^<(\w+)> (committed|pool|quota) tasks were triaged today$").unwrap()
});
static GIVEN_TASKS_TRIAGED_DAYS_AGO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^<(\w+)> (committed|pool|quota) tasks were triaged <(\w+)> days ago$").unwrap()
});
static WHEN_STATS_VIEWED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the stats page is viewed$").unwrap());
static THEN_REPORTS_SHARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the stats page reports a committed share of "<(\w+)>"$"#).unwrap()
});
static THEN_REPORTS_COUNTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the stats page reports <(\w+)> committed, <(\w+)> pool and <(\w+)> quota tasks$")
        .unwrap()
});
static THEN_REPORTS_IN_WINDOW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the stats page reports <(\w+)> tasks in the window$").unwrap());
static THEN_MARKS_SHARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the stats page marks the committed share "<(\w+)>"$"#).unwrap()
});
static THEN_REPORTS_NO_SHARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the stats page reports no committed share$").unwrap());
static THEN_DOES_NOT_MARK_SHARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the stats page does not mark the committed share$").unwrap());
static THEN_NOT_MEASURED_ENOUGH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the stats page says it has not measured enough yet$").unwrap());

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if let Some(caps) = GIVEN_TASKS_TRIAGED_TODAY.captures(text) {
        return Some(given_tasks_triaged_today(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_TASKS_TRIAGED_DAYS_AGO.captures(text) {
        return Some(given_tasks_triaged_days_ago(world, example, &caps).await);
    }
    if WHEN_STATS_VIEWED.is_match(text) {
        return Some(when_stats_viewed(world).await);
    }
    if let Some(caps) = THEN_REPORTS_SHARE.captures(text) {
        let expected = match example_value(example, &caps[1]) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(then_body_contains(world, expected));
    }
    if let Some(caps) = THEN_REPORTS_COUNTS.captures(text) {
        return Some(then_reports_counts(world, example, &caps));
    }
    if let Some(caps) = THEN_REPORTS_IN_WINDOW.captures(text) {
        let count = match example_value(example, &caps[1]) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(then_body_contains(
            world,
            &format!("{count} tasks in the window"),
        ));
    }
    if let Some(caps) = THEN_MARKS_SHARE.captures(text) {
        let expected = match example_value(example, &caps[1]) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        return Some(then_body_contains(world, expected));
    }
    if THEN_REPORTS_NO_SHARE.is_match(text) {
        return Some(then_body_excludes(world, "Committed share:"));
    }
    if THEN_DOES_NOT_MARK_SHARE.is_match(text) {
        return Some(then_does_not_mark_share(world));
    }
    if THEN_NOT_MEASURED_ENOUGH.is_match(text) {
        return Some(then_body_contains(world, "not measured enough"));
    }
    None
}

fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

async fn insert_fixture_tasks(
    world: &World,
    count: i64,
    kind: &str,
    created_at_ms: i64,
) -> Result<(), String> {
    let pool = world.pool()?;
    for _ in 0..count {
        let capture_id: i64 = sqlx::query_scalar(
            "INSERT INTO captures (raw_text, source, created_at_ms) VALUES ('fixture', 'test', ?) \
             RETURNING id",
        )
        .bind(created_at_ms)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("insert fixture capture: {e}"))?;
        sqlx::query("INSERT INTO tasks (capture_id, kind, created_at_ms) VALUES (?, ?, ?)")
            .bind(capture_id)
            .bind(kind)
            .bind(created_at_ms)
            .execute(pool)
            .await
            .map_err(|e| format!("insert fixture task: {e}"))?;
    }
    Ok(())
}

async fn given_tasks_triaged_today(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let count: i64 = example_value(example, &caps[1])?
        .parse()
        .map_err(|e| format!("bad task count: {e}"))?;
    let kind = &caps[2];
    insert_fixture_tasks(world, count, kind, now_ms()).await
}

async fn given_tasks_triaged_days_ago(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let count: i64 = example_value(example, &caps[1])?
        .parse()
        .map_err(|e| format!("bad task count: {e}"))?;
    let kind = &caps[2];
    let days_ago: i64 = example_value(example, &caps[3])?
        .parse()
        .map_err(|e| format!("bad days_ago: {e}"))?;
    let created_at_ms = now_ms() - days_ago * DAY_MS;
    insert_fixture_tasks(world, count, kind, created_at_ms).await
}

/// Delegates to [`super::inbox_view::html_response`], which already does
/// "send a GET, record status and body" -- the only thing this step adds is
/// the route.
async fn when_stats_viewed(world: &mut World) -> Result<(), String> {
    let request = Request::builder()
        .uri("/stats")
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    super::inbox_view::html_response(world, request).await
}

fn html_body(world: &World) -> Result<&str, String> {
    world
        .last_html_body
        .as_deref()
        .ok_or_else(|| "no stats page response recorded".to_string())
}

fn then_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(expected) {
        Ok(())
    } else {
        Err(format!(
            "expected {expected:?} on the stats page, got:\n{body}"
        ))
    }
}

fn then_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} on the stats page, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

fn then_reports_counts(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let committed = example_value(example, &caps[1])?;
    let pool = example_value(example, &caps[2])?;
    let quota = example_value(example, &caps[3])?;
    let expected = format!("{committed} committed, {pool} pool and {quota} quota tasks");
    then_body_contains(world, &expected)
}

fn then_does_not_mark_share(world: &mut World) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains("over the line") || body.contains("under the line") {
        Err(format!(
            "expected no committed-share standing on the stats page, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[tokio::test]
    async fn given_tasks_triaged_today_inserts_the_requested_count_of_the_named_kind() {
        let mut world = migrated_world().await;
        let ex = example(&[("pool", "3")]);
        let caps = GIVEN_TASKS_TRIAGED_TODAY
            .captures("<pool> pool tasks were triaged today")
            .unwrap();

        given_tasks_triaged_today(&mut world, &ex, &caps)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE kind = 'pool'")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn given_tasks_triaged_days_ago_stamps_the_task_that_many_days_in_the_past() {
        let mut world = migrated_world().await;
        let ex = example(&[("committed", "1"), ("days_ago", "15")]);
        let caps = GIVEN_TASKS_TRIAGED_DAYS_AGO
            .captures("<committed> committed tasks were triaged <days_ago> days ago")
            .unwrap();

        given_tasks_triaged_days_ago(&mut world, &ex, &caps)
            .await
            .unwrap();

        let created_at_ms: i64 = sqlx::query_scalar("SELECT created_at_ms FROM tasks")
            .fetch_one(world.pool().unwrap())
            .await
            .unwrap();
        let expected = now_ms() - 15 * DAY_MS;
        assert!(
            (created_at_ms - expected).abs() < 5_000,
            "expected roughly {expected}, got {created_at_ms}"
        );
    }

    #[tokio::test]
    async fn when_stats_viewed_records_the_response_body() {
        let mut world = migrated_world().await;

        when_stats_viewed(&mut world).await.unwrap();

        assert_eq!(world.last_status, Some(200));
        assert!(html_body(&world).unwrap().contains("tasks in the window"));
    }

    #[test]
    fn then_reports_counts_builds_the_exact_page_phrasing() {
        let mut world = World::new();
        world.last_html_body = Some("5 committed, 4 pool and 3 quota tasks".to_string());
        let ex = example(&[("committed", "5"), ("pool", "4"), ("quota", "3")]);
        let caps = THEN_REPORTS_COUNTS
            .captures(
                "the stats page reports <committed> committed, <pool> pool and <quota> quota tasks",
            )
            .unwrap();

        assert_eq!(then_reports_counts(&mut world, &ex, &caps), Ok(()));
    }

    #[test]
    fn then_does_not_mark_share_fails_when_a_standing_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("over the line".to_string());
        assert!(then_does_not_mark_share(&mut world).is_err());
    }

    #[test]
    fn then_does_not_mark_share_passes_when_no_standing_is_present() {
        let mut world = World::new();
        world.last_html_body = Some("not measured enough".to_string());
        assert_eq!(then_does_not_mark_share(&mut world), Ok(()));
    }
}
