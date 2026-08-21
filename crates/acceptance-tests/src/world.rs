//! Fresh per-execution state shared by background and scenario steps
//! (APS acceptance-generator.md step handler contract).

use std::path::PathBuf;

#[derive(Default)]
pub struct World {
    pub tmp_dir: Option<tempfile::TempDir>,
    pub db_path: Option<PathBuf>,
    pub pool: Option<sqlx::SqlitePool>,

    pub last_status: Option<u16>,
    pub last_response_body: Option<serde_json::Value>,
    pub last_html_body: Option<String>,

    pub last_capture_id: Option<i64>,

    /// The instant the server under test believes it is, when a scenario
    /// has pinned one (`trellis serve --now`'s acceptance-side counterpart,
    /// #60's DST scenarios). `None` means the real system clock, the same
    /// as every scenario before this one.
    pub pinned_now_ms: Option<i64>,

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

    /// The clock every request this scenario sends should be built against:
    /// pinned if a step has set [`Self::pinned_now_ms`], the real clock
    /// otherwise. One method so every step module's own request-building
    /// helper reads the same pin instead of each hardcoding `Clock::system()`.
    pub fn clock(&self) -> trellis_server::platform::clock::Clock {
        match self.pinned_now_ms {
            Some(pinned) => trellis_server::platform::clock::Clock::pinned_at(pinned),
            None => trellis_server::platform::clock::Clock::system(),
        }
    }
}
