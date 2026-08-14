use super::*;

static GIVEN_EMPTY_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^an empty trellis database file$").unwrap());
static GIVEN_ALREADY_MIGRATED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^a trellis database that has already been migrated once$").unwrap()
});
static WHEN_MIGRATION_RUN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the migration command is run$").unwrap());
static THEN_MIGRATION_EXITS_OK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the migration command exits successfully$").unwrap());
static THEN_JOURNAL_MODE_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^the database journal mode is "(\w+)"$"#).unwrap());
static THEN_SCHEMA_UNCHANGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^the database schema is unchanged$").unwrap());

pub async fn dispatch(world: &mut World, text: &str) -> Option<Result<(), String>> {
    if GIVEN_EMPTY_FILE.is_match(text) {
        return Some(given_empty_database_file(world));
    }
    if GIVEN_ALREADY_MIGRATED.is_match(text) {
        return Some(given_already_migrated_database(world).await);
    }
    if WHEN_MIGRATION_RUN.is_match(text) {
        return Some(when_migration_command_is_run(world).await);
    }
    if THEN_MIGRATION_EXITS_OK.is_match(text) {
        return Some(then_migration_exits_successfully(world));
    }
    if let Some(caps) = THEN_JOURNAL_MODE_IS.captures(text) {
        return Some(then_journal_mode_is(world, &caps[1]).await);
    }
    if THEN_SCHEMA_UNCHANGED.is_match(text) {
        return Some(then_schema_is_unchanged(world).await);
    }
    None
}

async fn table_names(pool: &sqlx::SqlitePool) -> Result<Vec<String>, String> {
    sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("list tables: {e}"))
}

pub fn given_empty_database_file(world: &mut World) -> Result<(), String> {
    let (dir, db_path) = new_temp_db_path("acceptance.db")?;
    world.db_path = Some(db_path);
    world.tmp_dir = Some(dir);
    Ok(())
}

pub async fn given_already_migrated_database(world: &mut World) -> Result<(), String> {
    given_empty_database_file(world)?;
    let db_path = world.db_path()?.clone();
    let pool = connected_and_migrated(&db_path).await?;
    world.schema_snapshot = Some(table_names(&pool).await?);
    world.pool = Some(pool);
    Ok(())
}

pub async fn when_migration_command_is_run(world: &mut World) -> Result<(), String> {
    let db_path = world.db_path()?.clone();
    let pool = if let Some(pool) = &world.pool {
        pool.clone()
    } else {
        let pool = trellis_server::platform::db::connect(&db_path)
            .await
            .map_err(|e| format!("connect: {e}"))?;
        world.pool = Some(pool.clone());
        pool
    };
    let result = trellis_server::platform::db::run_migrations(&pool)
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn migrated_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
        let (dir, db_path) = new_temp_db_path("acceptance.db").unwrap();
        let pool = trellis_server::platform::db::connect(&db_path)
            .await
            .unwrap();
        trellis_server::platform::db::run_migrations(&pool)
            .await
            .unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn table_names_lists_the_migrated_tables() {
        let (_dir, pool) = migrated_pool().await;
        let names = table_names(&pool).await.unwrap();
        assert!(
            names.iter().any(|n| n == "captures"),
            "expected a captures table, got {names:?}"
        );
    }

    #[test]
    fn then_migration_exits_successfully_passes_on_a_recorded_success() {
        let mut world = World::new();
        world.migration_result = Some(Ok(()));
        assert_eq!(then_migration_exits_successfully(&mut world), Ok(()));
    }

    #[test]
    fn then_migration_exits_successfully_errors_on_a_recorded_failure() {
        let mut world = World::new();
        world.migration_result = Some(Err("boom".to_string()));
        assert!(then_migration_exits_successfully(&mut world).is_err());
    }

    #[test]
    fn then_migration_exits_successfully_errors_when_never_run() {
        let mut world = World::new();
        assert!(then_migration_exits_successfully(&mut world).is_err());
    }

    #[tokio::test]
    async fn then_journal_mode_is_matches_case_insensitively() {
        let (_dir, pool) = migrated_pool().await;
        let mut world = World::new();
        world.pool = Some(pool);
        assert_eq!(then_journal_mode_is(&mut world, "WAL").await, Ok(()));
    }

    #[tokio::test]
    async fn then_journal_mode_is_errors_on_a_mismatched_mode() {
        let (_dir, pool) = migrated_pool().await;
        let mut world = World::new();
        world.pool = Some(pool);
        assert!(then_journal_mode_is(&mut world, "delete").await.is_err());
    }

    #[tokio::test]
    async fn then_schema_is_unchanged_passes_when_the_snapshot_matches() {
        let (_dir, pool) = migrated_pool().await;
        let snapshot = table_names(&pool).await.unwrap();
        let mut world = World::new();
        world.pool = Some(pool);
        world.schema_snapshot = Some(snapshot);
        assert_eq!(then_schema_is_unchanged(&mut world).await, Ok(()));
    }

    #[tokio::test]
    async fn then_schema_is_unchanged_errors_when_the_snapshot_differs() {
        let (_dir, pool) = migrated_pool().await;
        let mut world = World::new();
        world.pool = Some(pool);
        world.schema_snapshot = Some(vec!["not_a_real_table".to_string()]);
        assert!(then_schema_is_unchanged(&mut world).await.is_err());
    }

    #[tokio::test]
    async fn then_schema_is_unchanged_errors_without_a_prior_snapshot() {
        let (_dir, pool) = migrated_pool().await;
        let mut world = World::new();
        world.pool = Some(pool);
        assert!(then_schema_is_unchanged(&mut world).await.is_err());
    }
}
