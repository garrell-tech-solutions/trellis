//! **Triage** — deciding what a capture actually is. `POST
//! /captures/{id}/triage` turns one raw capture into a task of exactly one
//! kind (pool, committed or quota), or refuses and says why.
//!
//! The rules themselves are not here: `scheduler_core::task::TaskKind` owns
//! what a valid submission is, and this module's whole job is translation —
//! request into `TriageFields`, core rejection into a status code and a body
//! (`T-module-boundary`). Accepting a triage writes the task and stamps the
//! capture as consumed; both writes live in [`store`].

pub mod http;
pub mod store;
