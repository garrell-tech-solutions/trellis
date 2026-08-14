//! Project step handlers (APS acceptance-generator.md "Step Handler
//! Contract"). Regex-based parameter extraction is the default: a `<name>`
//! marker in step text captures the *placeholder name*, which is then looked
//! up in the current example object; a literal value in step text (e.g. a
//! status code) is captured and compared directly.
//!
//! Each Gherkin feature area gets its own step-handler submodule (`capture`,
//! `triage`, `migrations`, `build`); [`dispatch`] tries each in turn.

use crate::ir::Step;
use crate::world::World;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

mod app_client;
mod build;
mod capture;
mod committed_empty_fields;
mod committed_field_domains;
mod html;
mod inbox_view;
mod migrations;
mod payloads;
mod quota_triage_validation;
mod stats_ratio;
mod triage;
mod triage_from_page;
mod unknown_kind_rejection;

fn example_value<'a>(example: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, String> {
    example
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("example is missing placeholder \"{name}\""))
}

/// Looks up the two `<name>` placeholders captured by a two-group regex
/// (raw text and source appear together in several step shapes).
fn example_pair<'a>(
    example: &'a BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(&'a str, &'a str), String> {
    Ok((
        example_value(example, &caps[1])?,
        example_value(example, &caps[2])?,
    ))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("acceptance-tests crate is two levels under the workspace root")
        .to_path_buf()
}

/// Creates a fresh temp-directory-backed database path for a scenario's
/// background step. The directory is returned alongside the path so the
/// caller can keep it alive in `World` for the rest of the scenario.
fn new_temp_db_path(file_name: &str) -> Result<(tempfile::TempDir, PathBuf), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e}"))?;
    let db_path = dir.path().join(file_name);
    Ok((dir, db_path))
}

/// Opens a connection at `db_path` and runs migrations against it, the
/// shared setup for any background step that needs a ready-to-use database.
async fn connected_and_migrated(db_path: &Path) -> Result<sqlx::SqlitePool, String> {
    let pool = trellis_server::platform::db::connect(db_path)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    trellis_server::platform::db::run_migrations(&pool)
        .await
        .map_err(|e| format!("run_migrations: {e}"))?;
    Ok(pool)
}

/// Shared background-step body for scenarios that start from a fresh,
/// migrated database: create the temp-backed pool and record it on `World`.
async fn empty_migrated_database_world(world: &mut World) -> Result<(), String> {
    let (dir, db_path) = new_temp_db_path("acceptance.db")?;
    let pool = connected_and_migrated(&db_path).await?;
    world.db_path = Some(db_path);
    world.pool = Some(pool);
    world.tmp_dir = Some(dir);
    Ok(())
}

/// Shared unit-test fixture mirroring [`empty_migrated_database_world`]: a
/// `World` already wired to a fresh, migrated temp database.
#[cfg(test)]
async fn migrated_world() -> World {
    let (dir, db_path) = new_temp_db_path("acceptance.db").unwrap();
    let pool = connected_and_migrated(&db_path).await.unwrap();
    let mut world = World::new();
    world.tmp_dir = Some(dir);
    world.db_path = Some(db_path);
    world.pool = Some(pool);
    world
}

pub async fn dispatch(
    world: &mut World,
    step: &Step,
    example: &BTreeMap<String, String>,
) -> Result<(), String> {
    let text = step.text.as_str();

    if let Some(outcome) = capture::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = triage::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = committed_empty_fields::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = committed_field_domains::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = quota_triage_validation::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = unknown_kind_rejection::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = inbox_view::dispatch(world, text).await {
        return outcome;
    }
    if let Some(outcome) = triage_from_page::dispatch(world, text).await {
        return outcome;
    }
    if let Some(outcome) = stats_ratio::dispatch(world, text, example).await {
        return outcome;
    }
    if let Some(outcome) = migrations::dispatch(world, text).await {
        return outcome;
    }
    if let Some(outcome) = build::dispatch(world, text, example) {
        return outcome;
    }

    Err(format!("unsupported step: {} {}", step.keyword, step.text))
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    fn example(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn example_value_returns_the_named_placeholder() {
        let ex = example(&[("raw_text", "buy milk")]);
        assert_eq!(example_value(&ex, "raw_text"), Ok("buy milk"));
    }

    #[test]
    fn example_value_errors_when_the_placeholder_is_missing() {
        let ex = example(&[("raw_text", "buy milk")]);
        assert!(example_value(&ex, "source").is_err());
    }

    #[test]
    fn example_pair_returns_both_named_placeholders_in_order() {
        let ex = example(&[("raw_text", "buy milk"), ("source", "web")]);
        let re = Regex::new(r#""<(\w+)>" and "<(\w+)>""#).unwrap();
        let caps = re.captures(r#""<raw_text>" and "<source>""#).unwrap();
        assert_eq!(example_pair(&ex, &caps), Ok(("buy milk", "web")));
    }

    #[test]
    fn example_pair_errors_when_a_placeholder_is_missing() {
        let ex = example(&[("raw_text", "buy milk")]);
        let re = Regex::new(r#""<(\w+)>" and "<(\w+)>""#).unwrap();
        let caps = re.captures(r#""<raw_text>" and "<source>""#).unwrap();
        assert!(example_pair(&ex, &caps).is_err());
    }
}
