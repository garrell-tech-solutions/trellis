//! Step handlers for `features/quota_migration.feature`: migration `0016`
//! converts a `kind='quota'` task, triaged before the quota screen existed,
//! into a `quotas` row (#138).
//!
//! "the migration command is run" and "the "quota" screen is viewed" are
//! already matched generically by [`super::migrations::dispatch`] and
//! [`super::pool_screen::dispatch`], tried before this module; the quota
//! screen's own `Then` steps are [`super::quota_screen::dispatch`]'s --
//! nothing here duplicates any of them.

use super::*;

static GIVEN_PRE_0016_DATABASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^a trellis database that still keeps quota tasks apart from quotas$").unwrap()
});
static GIVEN_QUOTA_TASK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^it holds a quota task "([^"]+)" targeting "([^"]+)" sessions of "([^"]+)" minutes per week$"#,
    )
    .unwrap()
});
/// Distinct wording from `quota_screen.rs`'s own `GIVEN_QUOTA` ("a quota
/// named ..."): this scenario's own quota already exists on the screen
/// before the migration runs, which "it holds" says and "a quota named"
/// does not.
static GIVEN_EXISTING_QUOTA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^it holds a quota named "([^"]+)" with a target of "([^"]+)" hours a week$"#)
        .unwrap()
});

pub async fn dispatch(
    world: &mut World,
    text: &str,
    example: &BTreeMap<String, String>,
) -> Option<Result<(), String>> {
    if GIVEN_PRE_0016_DATABASE.is_match(text) {
        return Some(given_pre_0016_database(world).await);
    }
    if let Some(caps) = GIVEN_QUOTA_TASK.captures(text) {
        return Some(dispatch_given_quota_task(world, example, &caps).await);
    }
    if let Some(caps) = GIVEN_EXISTING_QUOTA.captures(text) {
        return Some(dispatch_given_existing_quota(world, &caps).await);
    }
    None
}

/// A database migrated through `0015` only -- everything `quotas` and
/// `quota_sessions` need already exists, but `0016`'s own conversion has
/// not run yet. Loaded at runtime from the same `migrations/` directory
/// `sqlx::migrate!()` embeds at compile time, filtered to versions below
/// `16`, so a later migration renumbering is the only thing that could ever
/// drift this out of step with the real migrator.
async fn given_pre_0016_database(world: &mut World) -> Result<(), String> {
    let (dir, db_path) = new_temp_db_path("acceptance.db")?;
    let pool = trellis_server::platform::db::connect(&db_path)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    let migrations_dir = workspace_root().join("crates/trellis-server/migrations");
    let mut migrator = sqlx::migrate::Migrator::new(migrations_dir)
        .await
        .map_err(|e| format!("load migrations: {e}"))?;
    migrator.migrations.to_mut().retain(|m| m.version < 16);
    migrator
        .run(&pool)
        .await
        .map_err(|e| format!("run pre-0016 migrations: {e}"))?;
    world.db_path = Some(db_path);
    world.pool = Some(pool);
    world.tmp_dir = Some(dir);
    Ok(())
}

/// See `quota_screen.rs`'s own copy of this function for the full
/// reasoning; duplicated rather than shared per this project's established
/// convention.
fn resolve(example: &BTreeMap<String, String>, raw: &str) -> Result<String, String> {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<(\w+)>$").unwrap());
    match PLACEHOLDER.captures(raw) {
        Some(caps) => Ok(example_value(example, &caps[1])?.to_string()),
        None => Ok(raw.to_string()),
    }
}

/// Inserts a capture and its `kind='quota'` task directly -- the pre-#138
/// shape migration `0016` is meant to convert, written straight to SQL
/// since no live code path produces it any more.
async fn dispatch_given_quota_task(
    world: &mut World,
    example: &BTreeMap<String, String>,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let text = resolve(example, &caps[1])?;
    let sessions: i64 = resolve(example, &caps[2])?
        .parse()
        .map_err(|e| format!("bad sessions: {e}"))?;
    let minutes_each: i64 = resolve(example, &caps[3])?
        .parse()
        .map_err(|e| format!("bad minutes_each: {e}"))?;
    let pool = world.pool()?;
    let capture_id: i64 = sqlx::query_scalar(
        "INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, 'web', 0) RETURNING id",
    )
    .bind(text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("insert capture: {e}"))?;
    sqlx::query(
        "INSERT INTO tasks (capture_id, kind, target_count, target_minutes_each, period, \
         created_at_ms) VALUES (?, 'quota', ?, ?, 'week', 0)",
    )
    .bind(capture_id)
    .bind(sessions)
    .bind(minutes_each)
    .execute(pool)
    .await
    .map_err(|e| format!("insert quota task: {e}"))?;
    Ok(())
}

/// A quota already on the screen before the migration runs -- the fixture
/// migration `0016`'s own `INSERT OR IGNORE` must not double.
async fn dispatch_given_existing_quota(
    world: &mut World,
    caps: &regex::Captures<'_>,
) -> Result<(), String> {
    let name = &caps[1];
    let hours: f64 = caps[2]
        .parse()
        .map_err(|e| format!("bad fixture hours {:?}: {e}", &caps[2]))?;
    let minutes = (hours * 60.0).round() as i64;
    let pool = world.pool()?;
    sqlx::query("INSERT INTO quotas (name, weekly_target_minutes, created_at_ms) VALUES (?, ?, 0)")
        .bind(name)
        .bind(minutes)
        .execute(pool)
        .await
        .map_err(|e| format!("create fixture quota {name:?}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_a_literal_value_unchanged() {
        let example = BTreeMap::new();
        assert_eq!(resolve(&example, "workout"), Ok("workout".to_string()));
    }

    #[test]
    fn resolve_looks_up_a_placeholder_in_the_example_row() {
        let example = super::super::example(&[("sessions", "3")]);
        assert_eq!(resolve(&example, "<sessions>"), Ok("3".to_string()));
    }

    /// The harness's own fixture: a database migrated through `0015`
    /// carries no `0016` bookkeeping row yet, so the real migrator still
    /// has work to do against it.
    #[tokio::test]
    async fn given_pre_0016_database_has_not_applied_0016() {
        let mut world = World::new();
        given_pre_0016_database(&mut world).await.unwrap();

        let pool = world.pool().unwrap();
        let applied: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(pool)
                .await
                .unwrap();

        assert_eq!(applied, (1..=15).collect::<Vec<i64>>());
    }

    /// End to end: the fixture's own `kind='quota'` task survives the real
    /// `0016` migration and becomes a `quotas` row multiplying its target,
    /// exactly what `quota_migration.feature`'s own scenario asserts
    /// through the HTTP screen instead.
    #[tokio::test]
    async fn a_fixture_quota_task_converts_to_a_quota_row_once_0016_runs() {
        let mut world = World::new();
        given_pre_0016_database(&mut world).await.unwrap();
        let example = super::super::example(&[
            ("text", "workout"),
            ("sessions", "3"),
            ("minutes_each", "45"),
        ]);
        let caps = GIVEN_QUOTA_TASK
            .captures(
                r#"it holds a quota task "<text>" targeting "<sessions>" sessions of "<minutes_each>" minutes per week"#,
            )
            .unwrap();
        dispatch_given_quota_task(&mut world, &example, &caps)
            .await
            .unwrap();

        let pool = world.pool().unwrap().clone();
        super::super::migrations::when_migration_command_is_run(&mut world)
            .await
            .unwrap();
        super::super::migrations::then_migration_exits_successfully(&mut world).unwrap();

        let rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(rows, vec![("workout".to_string(), 135)]);
    }

    #[tokio::test]
    async fn dispatch_given_existing_quota_creates_it() {
        let mut world = World::new();
        given_pre_0016_database(&mut world).await.unwrap();
        let caps = GIVEN_EXISTING_QUOTA
            .captures(r#"it holds a quota named "Piano" with a target of "4" hours a week"#)
            .unwrap();

        dispatch_given_existing_quota(&mut world, &caps)
            .await
            .unwrap();

        let pool = world.pool().unwrap();
        let row: (String, i64) = sqlx::query_as("SELECT name, weekly_target_minutes FROM quotas")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(row, ("Piano".to_string(), 240));
    }
}
