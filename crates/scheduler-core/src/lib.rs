//! Pure scheduling algorithm core. No async runtime, no database.
//! Enforced by scheduler_core_purity.feature: this crate must never
//! depend on tokio or sqlx, directly or transitively.
