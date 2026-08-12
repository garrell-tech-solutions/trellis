//! The trellis server: adapters around `scheduler_core`.
//!
//! Three layers, with dependencies pointing inward:
//!
//! - `scheduler_core` (its own crate) — the rules. Knows nothing below.
//! - [`http`] — delivery. Translates requests into core inputs and core
//!   decisions into responses.
//! - [`store`] — persistence. Translates core types into rows.
//!
//! [`app`] composes the two adapters; [`db`] owns the connection itself.
//! `http` and `store` do not depend on each other: a handler calls the store,
//! and the store never calls back.

pub mod app;
mod clock;
pub mod db;
pub mod http;
pub mod store;
#[cfg(test)]
mod test_support;
