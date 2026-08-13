//! Fresh per-execution state shared by background and scenario steps
//! (APS acceptance-generator.md step handler contract).

use std::path::PathBuf;
use std::time::Duration;

#[derive(Default)]
pub struct World {
    pub tmp_dir: Option<tempfile::TempDir>,
    pub db_path: Option<PathBuf>,
    pub pool: Option<sqlx::SqlitePool>,

    pub last_status: Option<u16>,
    pub last_elapsed: Option<Duration>,
    pub last_response_body: Option<serde_json::Value>,
    pub last_html_body: Option<String>,

    pub last_capture_id: Option<i64>,

    pub migration_result: Option<Result<(), String>>,
    pub schema_snapshot: Option<Vec<String>>,

    pub cargo_tree_output: Option<String>,
    pub release_build_ok: Option<bool>,
    pub release_binaries: Option<Vec<PathBuf>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn db_path(&self) -> Result<&PathBuf, String> {
        self.db_path
            .as_ref()
            .ok_or_else(|| "no database file set up for this scenario".to_string())
    }

    pub fn pool(&self) -> Result<&sqlx::SqlitePool, String> {
        self.pool
            .as_ref()
            .ok_or_else(|| "no database pool set up for this scenario".to_string())
    }
}
