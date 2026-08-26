//! **Triage** — deciding what a capture actually is. `POST
//! /captures/{id}/triage` turns one raw capture into a task of exactly one
//! kind (pool, committed or quota), or refuses and says why.
//!
//! The rules themselves are not here: `scheduler_core::task::TaskKind` owns
//! what a valid submission is, and this module's whole job is translation —
//! request into `TriageFields`, core rejection into a status code and a body
//! (`T-module-boundary`). Accepting a triage writes the task and stamps the
//! capture as consumed; both writes live in [`store`].
//!
//! That translation is two halves, and they are separated because only one
//! of them is about a transport: [`input`] is the half that knows `axum`'s
//! extractors and `serde`, and [`rejection`] is the refusal contract
//! (`T-422-is-product-wide`) -- a product-wide promise, decidable with
//! neither a database nor a request. [`http`] is what is left: the route,
//! the decision, and the writes.

pub mod http;
mod input;
mod rejection;
pub mod store;
