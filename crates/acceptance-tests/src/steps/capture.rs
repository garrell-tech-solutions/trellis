use super::*;
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

static GIVEN_EMPTY_TABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the trellis server is running with an empty captures table$").unwrap()
});
static WHEN_CAPTURE_SENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"^the QA agent sends a capture request with raw text "<([A-Za-z0-9_]+)>" and source "<([A-Za-z0-9_]+)>"$"#,
        )
        .unwrap()
});
static THEN_STATUS_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the response status is (\d+)$").unwrap());
static THEN_WITHIN_BUDGET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the response is received within (\d+) milliseconds$").unwrap());
static THEN_ROW_EXISTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
            r#"^a row exists in the captures table with raw text "<([A-Za-z0-9_]+)>" and source "<([A-Za-z0-9_]+)>"$"#,
        )
        .unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if GIVEN_EMPTY_TABLE.is_match(text) {
        return Some(given_empty_captures_table(world).await);
    }
    if let Some(caps) = WHEN_CAPTURE_SENT.captures(text) {
        let (raw_text, source) = match example_pair(example, &caps) {
            Ok(pair) => pair,
            Err(e) => return Some(Err(e)),
        };
        return Some(when_capture_request_sent(world, raw_text, source).await);
    }
    if let Some(caps) = THEN_STATUS_IS.captures(text) {
        let expected: u16 = match caps[1].parse() {
            Ok(v) => v,
            Err(e) => return Some(Err(format!("bad status code: {e}"))),
        };
        return Some(then_response_status_is(world, expected));
    }
    if let Some(caps) = THEN_WITHIN_BUDGET.captures(text) {
        let budget_ms: u128 = match caps[1].parse() {
            Ok(v) => v,
            Err(e) => return Some(Err(format!("bad millisecond budget: {e}"))),
        };
        return Some(then_response_within_budget(world, budget_ms));
    }
    if let Some(caps) = THEN_ROW_EXISTS.captures(text) {
        let (raw_text, source) = match example_pair(example, &caps) {
            Ok(pair) => pair,
            Err(e) => return Some(Err(e)),
        };
        return Some(then_row_exists(world, raw_text, source).await);
    }
    None
}

pub async fn given_empty_captures_table(world: &mut World) -> Result<(), String> {
    empty_migrated_database_world(world).await
}

pub async fn when_capture_request_sent(
    world: &mut World,
    raw_text: &str,
    source: &str,
) -> Result<(), String> {
    let pool = world.pool()?.clone();
    let app = trellis_server::app::build_app(pool);
    let body = serde_json::json!({ "raw_text": raw_text, "source": source }).to_string();

    let start = std::time::Instant::now();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/captures")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .map_err(|e| format!("build request: {e}"))?,
        )
        .await
        .map_err(|e| format!("send request: {e}"))?;

    world.last_elapsed = Some(start.elapsed());
    world.last_status = Some(response.status().as_u16());
    Ok(())
}

pub fn then_response_status_is(world: &mut World, expected: u16) -> Result<(), String> {
    match world.last_status {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("expected status {expected}, got {actual}")),
        None => Err("no response recorded".to_string()),
    }
}

pub fn then_response_within_budget(world: &mut World, budget_ms: u128) -> Result<(), String> {
    match world.last_elapsed {
        Some(elapsed) if elapsed.as_millis() < budget_ms => Ok(()),
        Some(elapsed) => Err(format!(
            "expected response within {budget_ms}ms, took {}ms",
            elapsed.as_millis()
        )),
        None => Err("no response timing recorded".to_string()),
    }
}

pub async fn then_row_exists(
    world: &mut World,
    raw_text: &str,
    source: &str,
) -> Result<(), String> {
    let pool = world.pool()?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM captures WHERE raw_text = ? AND source = ?")
            .bind(raw_text)
            .bind(source)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("query captures: {e}"))?;
    if count > 0 {
        Ok(())
    } else {
        Err(format!(
            "no row with raw_text={raw_text:?} source={source:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn then_response_status_is_matches_the_expected_code() {
        let mut world = World::new();
        world.last_status = Some(201);
        assert_eq!(then_response_status_is(&mut world, 201), Ok(()));
    }

    #[test]
    fn then_response_status_is_errors_on_a_mismatched_code() {
        let mut world = World::new();
        world.last_status = Some(500);
        assert!(then_response_status_is(&mut world, 201).is_err());
    }

    #[test]
    fn then_response_status_is_errors_when_no_response_was_recorded() {
        let mut world = World::new();
        assert!(then_response_status_is(&mut world, 201).is_err());
    }

    #[test]
    fn then_response_within_budget_accepts_a_faster_response() {
        let mut world = World::new();
        world.last_elapsed = Some(std::time::Duration::from_millis(10));
        assert_eq!(then_response_within_budget(&mut world, 50), Ok(()));
    }

    #[test]
    fn then_response_within_budget_rejects_a_response_at_exactly_the_budget() {
        let mut world = World::new();
        world.last_elapsed = Some(std::time::Duration::from_millis(50));
        assert!(
            then_response_within_budget(&mut world, 50).is_err(),
            "the budget is an exclusive upper bound"
        );
    }

    #[test]
    fn then_response_within_budget_rejects_a_slower_response() {
        let mut world = World::new();
        world.last_elapsed = Some(std::time::Duration::from_millis(100));
        assert!(then_response_within_budget(&mut world, 50).is_err());
    }

    #[tokio::test]
    async fn then_row_exists_errors_when_no_matching_row_is_present() {
        let mut world = migrated_world().await;
        assert!(then_row_exists(&mut world, "buy milk", "web")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn then_row_exists_succeeds_once_the_row_is_inserted() {
        let mut world = migrated_world().await;
        when_capture_request_sent(&mut world, "buy milk", "web")
            .await
            .unwrap();
        assert_eq!(then_row_exists(&mut world, "buy milk", "web").await, Ok(()));
        assert!(
            then_row_exists(&mut world, "call the dentist", "telegram")
                .await
                .is_err(),
            "should not match an unrelated raw_text/source pair"
        );
    }
}
