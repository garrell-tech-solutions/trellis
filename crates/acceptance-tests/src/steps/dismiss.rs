//! Step handlers for `features/dismiss_capture.feature`: a capture can be
//! dismissed from the inbox, and its row is kept (#48).
//!
//! The Background and "a capture ... is waiting" steps this feature uses are
//! already matched generically by [`super::triage::dispatch`], tried before
//! this module. "The capture is triaged as a pool task in life area ..." is
//! [`super::life_areas`]'s own step (`life_area_triage.feature`'s), reused
//! here rather than duplicated -- dismissal's "cannot then be triaged" and
//! "cannot be triaged again" scenarios are triage assertions in every way but
//! which feature file wrote them down.

use super::html;
use super::inbox_view::html_response;
use super::life_areas::capture_id_by_text;
use super::*;
use axum::body::Body;
use axum::http::Request;

static WHEN_DISMISSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the capture is dismissed from the inbox$").unwrap());
static WHEN_NAMED_DISMISSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"([^"]+)" is dismissed from the inbox$"#).unwrap());
static THEN_OFFERS_TO_DISMISS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the inbox offers to dismiss "([^"]+)"$"#).unwrap());
static THEN_NOT_REDIRECT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the dismissal response does not redirect the browser$").unwrap()
});
static THEN_REJECTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the dismissal is rejected$").unwrap());
static THEN_ROW_COUNT_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the capture row count is "<(\w+)>"$"#).unwrap());
static THEN_TASK_COUNT_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the task list has "<(\w+)>" tasks$"#).unwrap());
static THEN_REJECTION_SAYS_NOT_IN_INBOX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the rejection says the capture is no longer in the inbox$").unwrap()
});
static THEN_NO_UNESCAPED_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the dismissal response does not contain an unescaped "<script>" tag$"#).unwrap()
});
static THEN_CONTAINS_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^the dismissal response contains the word "([^"]+)"$"#).unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if WHEN_DISMISSED.is_match(text) {
        return Some(when_dismissed(world).await);
    }
    if let Some(caps) = WHEN_NAMED_DISMISSED.captures(text) {
        return Some(when_named_dismissed(world, &caps[1]).await);
    }
    if let Some(caps) = THEN_OFFERS_TO_DISMISS.captures(text) {
        return Some(then_offers_to_dismiss(world, &caps[1]).await);
    }
    if THEN_NOT_REDIRECT.is_match(text) {
        return Some(then_not_redirect(world));
    }
    if THEN_REJECTED.is_match(text) {
        return Some(then_status_is(world, 422));
    }
    if let Some(caps) = THEN_ROW_COUNT_IS.captures(text) {
        return Some(dispatch_row_count_is(world, example, &caps).await);
    }
    if let Some(caps) = THEN_TASK_COUNT_IS.captures(text) {
        return Some(dispatch_task_count_is(world, example, &caps).await);
    }
    if THEN_REJECTION_SAYS_NOT_IN_INBOX.is_match(text) {
        return Some(then_rejection_says_not_in_inbox(world));
    }
    if THEN_NO_UNESCAPED_SCRIPT.is_match(text) {
        return Some(then_html_body_excludes(world, "<script>"));
    }
    if let Some(caps) = THEN_CONTAINS_WORD.captures(text) {
        return Some(then_html_body_contains(world, &caps[1]));
    }
    None
}

fn capture_id(world: &World) -> Result<i64, String> {
    world
        .last_capture_id
        .ok_or_else(|| "no capture set up for this scenario".to_string())
}

async fn dismiss_request(world: &mut World, capture_id: i64) -> Result<(), String> {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/captures/{capture_id}/dismiss"))
        .body(Body::empty())
        .map_err(|e| format!("build request: {e}"))?;
    html_response(world, request).await
}

async fn when_dismissed(world: &mut World) -> Result<(), String> {
    let id = capture_id(world)?;
    dismiss_request(world, id).await
}

async fn when_named_dismissed(world: &mut World, raw_text: &str) -> Result<(), String> {
    let id = capture_id_by_text(world, raw_text).await?;
    dismiss_request(world, id).await
}

async fn then_offers_to_dismiss(world: &mut World, raw_text: &str) -> Result<(), String> {
    let id = capture_id_by_text(world, raw_text).await?;
    let body = html_body(world)?;
    let section = html::captures_section(body)?;
    let needle = format!("/captures/{id}/dismiss");
    if section.contains(&needle) {
        Ok(())
    } else {
        Err(format!(
            "expected a dismiss control for {raw_text:?} ({needle:?}), got:\n{section}"
        ))
    }
}

fn then_not_redirect(world: &mut World) -> Result<(), String> {
    super::then_not_redirect(world, "no dismissal response recorded")
}

fn then_status_is(world: &mut World, expected: u16) -> Result<(), String> {
    super::then_status_is(world, expected, "no dismissal response recorded")
}

fn parse_expected_count(
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
    parse_noun: &str,
) -> Result<i64, String> {
    example_value(example, &caps[1])?
        .parse()
        .map_err(|e| format!("bad {parse_noun}: {e}"))
}

/// `table` is always one of this module's own two literals, never submitted
/// text, so building the query with it carries no injection risk.
async fn table_count(world: &mut World, table: &str) -> Result<i64, String> {
    let pool = world.pool()?;
    let query = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar(&query)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("count {table}: {e}"))
}

/// Shared by [`dispatch_row_count_is`] and [`dispatch_task_count_is`]: parse
/// the expected count, query the named table, and compare.
async fn assert_table_count(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
    table: &str,
    parse_noun: &str,
    count_noun: &str,
) -> Result<(), String> {
    let expected = parse_expected_count(example, caps, parse_noun)?;
    let actual = table_count(world, table).await?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected} {count_noun}, got {actual}"))
    }
}

async fn dispatch_row_count_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    assert_table_count(
        world,
        example,
        caps,
        "captures",
        "row count",
        "capture rows",
    )
    .await
}

async fn dispatch_task_count_is(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    assert_table_count(world, example, caps, "tasks", "task count", "tasks").await
}

/// Checks the reason for a dismissal-adjacent rejection wherever it landed:
/// a JSON body (the triage endpoint the "cannot then be triaged" scenario
/// drives, via `life_areas::triage_pool_in_life_area`'s API transport), or an
/// HTML fragment. Serialising the JSON body to text and substring-matching it
/// is what lets this check the message without depending on which key it
/// rides in under (`triage::http::Rejection::CaptureNotOpen`'s own body
/// shape is that module's business, not this step's).
fn then_rejection_says_not_in_inbox(world: &mut World) -> Result<(), String> {
    const EXPECTED: &str = "the capture is no longer in the inbox";
    if let Some(body) = &world.last_response_body {
        let text = body.to_string();
        return if text.contains(EXPECTED) {
            Ok(())
        } else {
            Err(format!(
                "expected {EXPECTED:?} in the rejection, got:\n{text}"
            ))
        };
    }
    if let Some(html) = &world.last_html_body {
        return if html.contains(EXPECTED) {
            Ok(())
        } else {
            Err(format!(
                "expected {EXPECTED:?} in the rejection, got:\n{html}"
            ))
        };
    }
    Err("no rejection response recorded".to_string())
}

fn html_body(world: &World) -> Result<&str, String> {
    super::html_body(world, "no HTML response recorded")
}

fn then_html_body_contains(world: &mut World, expected: &str) -> Result<(), String> {
    super::then_html_body_contains(world, expected, "no HTML response recorded")
}

fn then_html_body_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
    let body = html_body(world)?;
    if body.contains(forbidden) {
        Err(format!(
            "expected no {forbidden:?} in the response, got:\n{body}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::triage::given_capture_waiting;
    use super::*;

    /// The captures section as it reads immediately after a `GET /` reload,
    /// the assertion setup shared by every test that dismisses a capture and
    /// then checks who is left in the inbox.
    async fn captures_section_after_reload(world: &mut World) -> String {
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        html_response(world, request).await.unwrap();
        let body = html_body(&*world).unwrap();
        html::captures_section(body).unwrap().to_string()
    }

    #[tokio::test]
    async fn when_dismissed_removes_the_capture_from_the_inbox() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "asdfgh").await.unwrap();

        when_dismissed(&mut world).await.unwrap();

        then_not_redirect(&mut world).unwrap();
        let section = captures_section_after_reload(&mut world).await;
        assert!(!section.contains("asdfgh"));
    }

    #[tokio::test]
    async fn when_named_dismissed_dismisses_the_capture_with_that_text_not_the_last_one_set_up() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "asdfgh").await.unwrap();
        given_capture_waiting(&mut world, "buy milk").await.unwrap();

        when_named_dismissed(&mut world, "asdfgh").await.unwrap();

        let section = captures_section_after_reload(&mut world).await;
        assert!(!section.contains("asdfgh"));
        assert!(section.contains("buy milk"));
    }

    #[tokio::test]
    async fn then_offers_to_dismiss_finds_the_named_captures_own_control() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "asdfgh").await.unwrap();
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        html_response(&mut world, request).await.unwrap();

        assert_eq!(then_offers_to_dismiss(&mut world, "asdfgh").await, Ok(()));
    }

    #[tokio::test]
    async fn a_second_dismissal_is_rejected() {
        let mut world = migrated_world().await;
        given_capture_waiting(&mut world, "asdfgh").await.unwrap();
        when_dismissed(&mut world).await.unwrap();

        when_dismissed(&mut world).await.unwrap();

        then_status_is(&mut world, 422).unwrap();
    }

    #[test]
    fn then_not_redirect_errors_for_a_redirect_status() {
        let mut world = World::new();
        world.last_status = Some(302);
        assert!(then_not_redirect(&mut world).is_err());
    }

    #[test]
    fn then_rejection_says_not_in_inbox_checks_a_json_body() {
        let mut world = World::new();
        world.last_response_body = Some(
            serde_json::json!({ "capture_not_open": "the capture is no longer in the inbox" }),
        );
        assert_eq!(then_rejection_says_not_in_inbox(&mut world), Ok(()));
    }

    #[test]
    fn then_rejection_says_not_in_inbox_checks_an_html_body() {
        let mut world = World::new();
        world.last_html_body =
            Some("<p class=\"triage-error\">the capture is no longer in the inbox</p>".to_string());
        assert_eq!(then_rejection_says_not_in_inbox(&mut world), Ok(()));
    }

    #[test]
    fn then_rejection_says_not_in_inbox_errors_when_neither_response_carries_it() {
        let mut world = World::new();
        world.last_response_body = Some(serde_json::json!({ "missing_field": "life_area" }));
        assert!(then_rejection_says_not_in_inbox(&mut world).is_err());
    }
}
