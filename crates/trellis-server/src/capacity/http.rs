//! `GET /capacity`.
//!
//! Translation only: ask [`super::rows`] for every active life area's
//! number, render what it says. No arithmetic of its own -- proration,
//! demand and the warning threshold all live in `scheduler_core::capacity`.

use crate::platform::clock::Clock;
use crate::platform::nav::{self, NavLink, Page};
use crate::platform::response::{render_template, write_failed};
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

#[derive(Template)]
#[template(path = "capacity.html")]
struct CapacityTemplate {
    rows: Vec<super::view::CapacityRow>,
    nav: Vec<NavLink>,
}

pub async fn show_capacity(
    State(pool): State<SqlitePool>,
    State(clock): State<Clock>,
) -> Result<Response, StatusCode> {
    let rows = super::rows(&pool, &clock).await.map_err(write_failed)?;

    Ok(render_template(
        StatusCode::OK,
        &CapacityTemplate {
            rows,
            nav: nav::links(Page::Capacity),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::{get_ok, test_pool};

    async fn get_capacity(pool: &SqlitePool) -> String {
        get_ok(pool, Clock::system(), "/capacity").await
    }

    #[tokio::test]
    async fn a_fresh_database_lists_every_seeded_life_area() {
        let (_dir, pool) = test_pool().await;

        let body = get_capacity(&pool).await;

        for expected in ["Work", "Fitness", "Learning", "Family", "Home"] {
            assert!(body.contains(expected), "expected {expected} in:\n{body}");
        }
    }

    #[tokio::test]
    async fn hostile_text_in_a_life_area_name_is_escaped() {
        let (_dir, pool) = test_pool().await;
        crate::life_areas::store::insert(&pool, "<script>alert('boom')</script>")
            .await
            .unwrap();

        let body = get_capacity(&pool).await;

        assert!(!body.contains("<script>"), "got:\n{body}");
        assert!(body.contains("boom"), "got:\n{body}");
    }
}
