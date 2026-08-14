//! **Inbox** — the first page: what is still waiting to be triaged, newest
//! first, alongside the tasks triage has already produced.
//!
//! The inbox owns the shape of a listed row ([`view`]) and the `#lists`
//! fragment those rows live in ([`lists`]), because both are what a reader
//! sees on this page — and that is why [`crate::capture`] and
//! [`crate::triage`] reach in here for them. Creating a capture returns the
//! new inbox row; triaging one re-renders the inbox's two lists. Neither is
//! the inbox reaching outward: the inbox is the surface those capabilities
//! act on.

pub mod http;
pub mod lists;
pub mod store;
pub mod view;
