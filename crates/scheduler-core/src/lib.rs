//! Pure scheduling algorithm core. No async runtime, no database.
//! Enforced by scheduler_core_purity.feature: this crate must never
//! depend on tokio or sqlx, directly or transitively.
//!
//! This is where the product's rules live. Delivery mechanisms (HTTP today)
//! and persistence adapters depend on this crate; it depends on neither.

pub mod task;
