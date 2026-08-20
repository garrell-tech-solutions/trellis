//! **Capture** — getting a thought out of a head and into the system before
//! it evaporates. One route, `POST /captures`, content-negotiated so the JSON
//! API and the inbox's quick-add box are one code path; a 50ms budget, which
//! is why nothing slower than an insert happens here.
//!
//! A capture is raw text. It is not yet a task, carries no kind, no deadline
//! and no priority, and is never deleted — [`crate::triage`] is what turns
//! one into a task, and [`crate::inbox`] is what shows the ones that have not
//! been triaged yet.

pub mod http;
pub mod store;
