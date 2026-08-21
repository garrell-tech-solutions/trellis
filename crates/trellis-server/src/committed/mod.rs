//! **Committed** -- what has a date on it, listed chronologically (#94,
//! `D-four-screens`'s third Menu tab).
//!
//! `D-committed-is-at-or-by`: a committed item is either an **at** (a fixed
//! block) or a **by** (a deadline with slack); the canvas draws no
//! distinction, so how it looks is this slice's to invent. `store` owns its
//! one query, `view` turns a store row into what the template renders
//! (`scheduler_core::committed_screen::order` does the actual date math),
//! `http` serves `GET /committed`.

pub mod http;
pub mod store;
pub mod view;
