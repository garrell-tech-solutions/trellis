//! **Inbox** — the first page: `Recent`. What is still waiting to be
//! triaged, newest first, and the three most recently triaged captures
//! reading what they became (#140).
//!
//! **One list, not two.** It was the untriaged queue *plus* a flat `Tasks`
//! list of every task in the database — `D-visible-slices`' proof, back when
//! triage wrote somewhere with nothing to show for it. There are four screens
//! for that now (`D-four-screens`), so the second list is gone and a triaged
//! capture stays in the first one instead.
//!
//! The inbox owns the shape of a listed row ([`view`]) and the `#lists`
//! fragment those rows live in (`lists`), because both are what a reader
//! sees on this page — and that is why [`crate::capture`],
//! [`crate::triage`] and [`crate::dismiss`] reach in here. Creating a
//! capture returns the new inbox row; triaging or dismissing one re-renders
//! the inbox's list. Neither is the inbox reaching outward: the inbox
//! is the surface those capabilities act on.
//!
//! It also owns **membership**: whether a capture is still in the inbox, and
//! taking one out. See `capture_is_open` and `close_capture` below.
//!
//! Everything another capability may ask of the inbox is in this file, and
//! nothing else here is reachable from outside it — `lists` is private, and
//! the two membership queries are `pub(super)` in [`store`]. That is
//! `T-one-front-door-per-capability` enforced by the compiler rather than
//! described in a comment.

pub mod http;
mod lists;
mod shown_kind;
pub mod store;
pub mod view;

use axum::http::StatusCode;
use axum::response::Response;
use sqlx::SqlitePool;

/// The prose for `!capture_is_open`, in the one place the fact itself lives.
///
/// Two capabilities word it: `triage` in the `capture_not_open` rejection
/// body *and* the failing row's message, `dismiss` in that same message. A
/// string retyped in three places is three chances to change two of them.
pub(crate) const CAPTURE_NOT_OPEN_MESSAGE: &str = "the capture is no longer in the inbox";

/// Whether the inbox still holds `capture_id` — the question both of its
/// exits must ask before taking it.
///
/// **Why this is the inbox's and not the asker's.** `T-capability-owns-its-queries`
/// says a business domain owns the SQL it issues, and its worked example was
/// this very table: *"`triage` stamps `triaged_at`"*. That was true while the
/// column meant "triage happened". Migration `0005` renamed it to
/// `left_inbox_at` and widened it to "left the inbox, by either exit", and
/// that rename moved the fact: `left_inbox_at IS NULL` is now the definition
/// of [`store::list_recent`]'s untriaged half — inbox membership — asked
/// about one row instead of all of them. `triage` and `dismiss` ask the same
/// question for the same reason, which is one fact, not the two facts that
/// decision licenses separate copies of.
///
/// So it comes through the front door (`T-one-front-door-per-capability`),
/// which is the complement that keeps "own your own queries" from
/// degenerating into every capability hand-assembling another's internals.
/// Before this, `left_inbox_at IS NULL` and its meaning were spelled out in
/// three modules; a fourth exit would have made four.
pub(crate) async fn capture_is_open(
    pool: &SqlitePool,
    capture_id: i64,
) -> Result<bool, sqlx::Error> {
    store::capture_is_open(pool, capture_id).await
}

/// Takes `capture_id` out of the inbox, stamping when it left. Both exits
/// call it: triage after writing the task, dismissal on its own.
///
/// **Which exit it was stays derivable, and deliberately is not stored** —
/// a `tasks` row references the capture, or it does not. A second column
/// would let a row claim both exits at once, which is the shape
/// `T-archived-at-only` exists to forbid; migration `0005`'s header argues
/// it in full.
pub(crate) async fn close_capture(
    pool: &SqlitePool,
    capture_id: i64,
    left_inbox_at_ms: i64,
) -> Result<(), sqlx::Error> {
    store::close_capture(pool, capture_id, left_inbox_at_ms).await
}

/// The `#lists` fragment, re-rendered from current state at `status`, with
/// `error` attached to the row that caused it — `T-forms-swap-one-fragment`'s
/// response contract, which every form acting on an inbox row owes.
///
/// Both exits end here, and neither knows what the fragment is made of. The
/// third one will not either.
pub(crate) async fn render_lists(
    pool: &SqlitePool,
    status: StatusCode,
    error: Option<(i64, String)>,
) -> Result<Response, StatusCode> {
    lists::respond(pool, status, error).await
}
