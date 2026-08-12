use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
pub struct CaptureRequest {
    pub raw_text: String,
    pub source: String,
}

pub async fn create_capture(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CaptureRequest>,
) -> Result<StatusCode, StatusCode> {
    let created_at_ms = now_ms();
    sqlx::query("INSERT INTO captures (raw_text, source, created_at_ms) VALUES (?, ?, ?)")
        .bind(&payload.raw_text)
        .bind(&payload.source)
        .bind(created_at_ms)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
}
