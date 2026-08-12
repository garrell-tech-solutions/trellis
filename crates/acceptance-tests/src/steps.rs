//! Project step handlers (APS acceptance-generator.md "Step Handler
//! Contract"). Regex-based parameter extraction is the default: a `<name>`
//! marker in step text captures the *placeholder name*, which is then looked
//! up in the current example object; a literal value in step text (e.g. a
//! status code) is captured and compared directly.

use crate::ir::Step;
use crate::world::World;
use regex::Regex;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn example_value<'a>(example: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, String> {
    example
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("example is missing placeholder \"{name}\""))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("acceptance-tests crate is two levels under the workspace root")
        .to_path_buf()
}

pub async fn dispatch(
    world: &mut World,
    step: &Step,
    example: &BTreeMap<String, String>,
) -> Result<(), String> {
    let text = step.text.as_str();

    // --- capture endpoint -------------------------------------------------

    if Regex::new(r"^the trellis server is running with an empty captures table$")
        .unwrap()
        .is_match(text)
    {
        return capture::given_empty_captures_table(world).await;
    }

    if let Some(caps) = Regex::new(
        r#"^the QA agent sends a capture request with raw text "<([A-Za-z0-9_]+)>" and source "<([A-Za-z0-9_]+)>"$"#,
    )
    .unwrap()
    .captures(text)
    {
        let raw_text = example_value(example, &caps[1])?;
        let source = example_value(example, &caps[2])?;
        return capture::when_capture_request_sent(world, raw_text, source).await;
    }

    if let Some(caps) = Regex::new(r"^the response status is (\d+)$")
        .unwrap()
        .captures(text)
    {
        let expected: u16 = caps[1]
            .parse()
            .map_err(|e| format!("bad status code: {e}"))?;
        return capture::then_response_status_is(world, expected);
    }

    if let Some(caps) = Regex::new(r"^the response is received within (\d+) milliseconds$")
        .unwrap()
        .captures(text)
    {
        let budget_ms: u128 = caps[1]
            .parse()
            .map_err(|e| format!("bad millisecond budget: {e}"))?;
        return capture::then_response_within_budget(world, budget_ms);
    }

    if let Some(caps) = Regex::new(
        r#"^a row exists in the captures table with raw text "<([A-Za-z0-9_]+)>" and source "<([A-Za-z0-9_]+)>"$"#,
    )
    .unwrap()
    .captures(text)
    {
        let raw_text = example_value(example, &caps[1])?;
        let source = example_value(example, &caps[2])?;
        return capture::then_row_exists(world, raw_text, source).await;
    }

    // --- migrations ---------------------------------------------------------

    if Regex::new(r"^an empty trellis database file$")
        .unwrap()
        .is_match(text)
    {
        return migrations::given_empty_database_file(world);
    }

    if Regex::new(r"^a trellis database that has already been migrated once$")
        .unwrap()
        .is_match(text)
    {
        return migrations::given_already_migrated_database(world).await;
    }

    if Regex::new(r"^the migration command is run$")
        .unwrap()
        .is_match(text)
    {
        return migrations::when_migration_command_is_run(world).await;
    }

    if Regex::new(r"^the migration command exits successfully$")
        .unwrap()
        .is_match(text)
    {
        return migrations::then_migration_exits_successfully(world);
    }

    if let Some(caps) = Regex::new(r#"^the database journal mode is "(\w+)"$"#)
        .unwrap()
        .captures(text)
    {
        return migrations::then_journal_mode_is(world, &caps[1]).await;
    }

    if Regex::new(r"^the database schema is unchanged$")
        .unwrap()
        .is_match(text)
    {
        return migrations::then_schema_is_unchanged(world).await;
    }

    // --- release binary / scheduler-core purity (shared background step) ---

    if Regex::new(r"^the workspace is checked out$")
        .unwrap()
        .is_match(text)
    {
        return build::given_workspace_checked_out();
    }

    if Regex::new(r"^the release binary is built for the musl target$")
        .unwrap()
        .is_match(text)
    {
        return build::when_release_binary_built_for_musl(world);
    }

    if Regex::new(r"^exactly one release binary is produced$")
        .unwrap()
        .is_match(text)
    {
        return build::then_exactly_one_release_binary(world);
    }

    if Regex::new(r"^the binary reports no dynamic executable dependencies$")
        .unwrap()
        .is_match(text)
    {
        return build::then_binary_has_no_dynamic_dependencies(world);
    }

    if Regex::new(r"^the dependency tree for the scheduler-core crate is listed$")
        .unwrap()
        .is_match(text)
    {
        return build::when_scheduler_core_dependency_tree_listed(world);
    }

    if let Some(caps) =
        Regex::new(r#"^the dependency tree contains zero occurrences of "<([A-Za-z0-9_]+)>"$"#)
            .unwrap()
            .captures(text)
    {
        let forbidden = example_value(example, &caps[1])?;
        return build::then_dependency_tree_excludes(world, forbidden);
    }

    Err(format!("unsupported step: {} {}", step.keyword, step.text))
}

mod capture {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    pub async fn given_empty_captures_table(world: &mut World) -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e}"))?;
        let db_path = dir.path().join("acceptance.db");
        let pool = trellis_server::db::connect(&db_path)
            .await
            .map_err(|e| format!("connect: {e}"))?;
        trellis_server::db::run_migrations(&pool)
            .await
            .map_err(|e| format!("run_migrations: {e}"))?;
        world.db_path = Some(db_path);
        world.pool = Some(pool);
        world.tmp_dir = Some(dir);
        Ok(())
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
}

mod migrations {
    use super::*;

    async fn table_names(pool: &sqlx::SqlitePool) -> Result<Vec<String>, String> {
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("list tables: {e}"))
    }

    pub fn given_empty_database_file(world: &mut World) -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e}"))?;
        let db_path = dir.path().join("acceptance.db");
        world.db_path = Some(db_path);
        world.tmp_dir = Some(dir);
        Ok(())
    }

    pub async fn given_already_migrated_database(world: &mut World) -> Result<(), String> {
        given_empty_database_file(world)?;
        let db_path = world.db_path()?.clone();
        let pool = trellis_server::db::connect(&db_path)
            .await
            .map_err(|e| format!("connect: {e}"))?;
        trellis_server::db::run_migrations(&pool)
            .await
            .map_err(|e| format!("run_migrations: {e}"))?;
        world.schema_snapshot = Some(table_names(&pool).await?);
        world.pool = Some(pool);
        Ok(())
    }

    pub async fn when_migration_command_is_run(world: &mut World) -> Result<(), String> {
        let db_path = world.db_path()?.clone();
        let pool = if let Some(pool) = &world.pool {
            pool.clone()
        } else {
            let pool = trellis_server::db::connect(&db_path)
                .await
                .map_err(|e| format!("connect: {e}"))?;
            world.pool = Some(pool.clone());
            pool
        };
        let result = trellis_server::db::run_migrations(&pool)
            .await
            .map_err(|e| e.to_string());
        world.migration_result = Some(result);
        Ok(())
    }

    pub fn then_migration_exits_successfully(world: &mut World) -> Result<(), String> {
        match &world.migration_result {
            Some(Ok(())) => Ok(()),
            Some(Err(e)) => Err(format!("migration command failed: {e}")),
            None => Err("migration command was never run".to_string()),
        }
    }

    pub async fn then_journal_mode_is(world: &mut World, expected: &str) -> Result<(), String> {
        let pool = world.pool()?;
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("read journal_mode: {e}"))?;
        if mode.eq_ignore_ascii_case(expected) {
            Ok(())
        } else {
            Err(format!("expected journal mode {expected}, got {mode}"))
        }
    }

    pub async fn then_schema_is_unchanged(world: &mut World) -> Result<(), String> {
        let pool = world.pool()?;
        let after = table_names(pool).await?;
        let before = world
            .schema_snapshot
            .clone()
            .ok_or_else(|| "no schema snapshot recorded before migration rerun".to_string())?;
        if before == after {
            Ok(())
        } else {
            Err(format!("schema changed: before={before:?} after={after:?}"))
        }
    }
}

mod build {
    use super::*;

    const MUSL_TARGET: &str = "x86_64-unknown-linux-musl";

    pub fn given_workspace_checked_out() -> Result<(), String> {
        let root = workspace_root();
        if root.join("Cargo.toml").is_file() {
            Ok(())
        } else {
            Err(format!("no workspace Cargo.toml under {}", root.display()))
        }
    }

    pub fn when_release_binary_built_for_musl(world: &mut World) -> Result<(), String> {
        let root = workspace_root();
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--target",
                MUSL_TARGET,
                "-p",
                "trellis-server",
                "--bin",
                "trellis",
            ])
            .current_dir(&root)
            .status()
            .map_err(|e| format!("spawn cargo build: {e}"))?;
        world.release_build_ok = Some(status.success());
        if !status.success() {
            return Err(format!(
                "cargo build --release --target {MUSL_TARGET} exited with {status}"
            ));
        }

        let release_dir = root.join("target").join(MUSL_TARGET).join("release");
        let mut binaries = Vec::new();
        for entry in std::fs::read_dir(&release_dir)
            .map_err(|e| format!("read {}: {e}", release_dir.display()))?
        {
            let entry = entry.map_err(|e| format!("read dir entry: {e}"))?;
            let path = entry.path();
            let is_executable_file = path.is_file()
                && path
                    .metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
            if is_executable_file {
                binaries.push(path);
            }
        }
        world.release_binaries = Some(binaries);
        Ok(())
    }

    pub fn then_exactly_one_release_binary(world: &mut World) -> Result<(), String> {
        let binaries = world
            .release_binaries
            .as_ref()
            .ok_or_else(|| "release binary was never built".to_string())?;
        if binaries.len() == 1 {
            Ok(())
        } else {
            Err(format!(
                "expected exactly one release binary, found {}: {binaries:?}",
                binaries.len()
            ))
        }
    }

    pub fn then_binary_has_no_dynamic_dependencies(world: &mut World) -> Result<(), String> {
        let binaries = world
            .release_binaries
            .as_ref()
            .ok_or_else(|| "release binary was never built".to_string())?;
        let binary = binaries
            .first()
            .ok_or_else(|| "no release binary to inspect".to_string())?;
        let output = Command::new("file")
            .arg(binary)
            .output()
            .map_err(|e| format!("spawn file: {e}"))?;
        let description = String::from_utf8_lossy(&output.stdout);
        if description.contains("statically linked") || description.contains("static-pie linked") {
            Ok(())
        } else {
            Err(format!(
                "expected a statically linked binary, `file` reported: {description}"
            ))
        }
    }

    pub fn when_scheduler_core_dependency_tree_listed(world: &mut World) -> Result<(), String> {
        let root = workspace_root();
        let output = Command::new("cargo")
            .args(["tree", "-p", "scheduler-core"])
            .current_dir(&root)
            .output()
            .map_err(|e| format!("spawn cargo tree: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "cargo tree -p scheduler-core exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        world.cargo_tree_output = Some(String::from_utf8_lossy(&output.stdout).into_owned());
        Ok(())
    }

    pub fn then_dependency_tree_excludes(world: &mut World, forbidden: &str) -> Result<(), String> {
        let tree = world
            .cargo_tree_output
            .as_ref()
            .ok_or_else(|| "dependency tree was never listed".to_string())?;
        let found = tree.lines().any(|line| {
            let name = line.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let name = name.split_whitespace().next().unwrap_or("");
            name == forbidden
        });
        if found {
            Err(format!(
                "dependency tree for scheduler-core contains forbidden dependency \"{forbidden}\""
            ))
        } else {
            Ok(())
        }
    }
}
