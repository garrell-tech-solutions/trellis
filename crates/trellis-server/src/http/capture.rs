//! `POST /captures`.

use crate::clock::now_ms;
use crate::http::write_failed;
use crate::store;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct CaptureRequest {
    pub raw_text: String,
    pub source: String,
}

pub async fn create_capture(
    State(pool): State<SqlitePool>,
    Json(payload): Json<CaptureRequest>,
) -> Result<StatusCode, StatusCode> {
    store::capture::insert(&pool, &payload.raw_text, &payload.source, now_ms())
        .await
        .map_err(write_failed)?;
    Ok(StatusCode::CREATED)
}
