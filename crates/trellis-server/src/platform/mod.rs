//! **Not a capability.** The machinery every capability runs on: the route
//! table that composes them ([`app`]), the database they persist through
//! ([`db`]), the clock they are handed to stamp rows with ([`clock`]), the
//! vendored script
//! the page loads ([`assets`]), the request-side helper every transport-
//! sniffing handler needs ([`request`]), the two helpers that turn a
//! template or a failed write into a response ([`response`]), and the
//! shared header every page carries ([`nav`]).
//!
//! `nav` returned in #92: #88 deleted every route but `/`, so there was
//! nothing to navigate between; `/pool` is a second one, and
//! `T-nav-is-the-site-map` says the header is the route table, not an
//! afterthought bolted onto whichever page asks for it.
//!
//! Kept in one directory with an obviously non-product name so that `ls
//! src/` lists what Trellis does and exactly one bucket that is plainly not
//! part of the answer (`T-package-by-business-domain`). Nothing here decides
//! anything about captures, triage or the inbox; anything that does belongs
//! in the capability it decides for.
//!
//! [`boundary`] is the rule that keeps that true, and it is a test module: it
//! walks the crate rather than describing it.

pub mod app;
pub mod assets;
pub mod clock;
pub mod db;
pub(crate) mod nav;
pub(crate) mod request;
pub(crate) mod response;

#[cfg(test)]
mod boundary;
#[cfg(test)]
pub(crate) mod test_support;
