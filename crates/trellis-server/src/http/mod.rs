//! Delivery adapters: the only place in the crate that knows requests and
//! status codes exist.
//!
//! A handler's whole job is translation — request body into a
//! `scheduler_core` input, core decision into a response, store failure into
//! a status code. Any rule that survives changing HTTP for something else
//! belongs in `scheduler_core`, not here.

pub mod capture;
pub mod triage;

use axum::http::StatusCode;

/// A failed write is the delivery layer's problem to name; the store reports
/// the database error and says nothing about HTTP.
pub(crate) fn write_failed(_error: sqlx::Error) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_write_is_reported_as_an_internal_server_error() {
        assert_eq!(
            write_failed(sqlx::Error::RowNotFound),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
