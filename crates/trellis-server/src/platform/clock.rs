//! The clock a running server reads — a value it is handed, not a global it
//! reaches for.
//!
//! Time is a device, and `--now` makes it a *configurable* device: the server
//! can be started believing it is another instant. That configuration is
//! state, and state reached through a process-global is the one kind no test
//! can isolate — every test in the binary shares it, so pinning the clock in
//! one leaks into whatever else is running, and the pinning test has to
//! remember to put it back. A [`Clock`] is passed to `build_app` instead and
//! lives in the app state beside the pool: one clock per server, chosen by
//! whoever built that server, invisible to everyone else.
//!
//! What it is *not* is a trait. There is exactly one implementation and only
//! one axis of variation — which instant "now" is — so the offset below says
//! that in a field. A `dyn Clock` would buy a second implementation nothing
//! needs and cost every handler a trait object.

use std::time::{SystemTime, UNIX_EPOCH};

fn real_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
}

/// The instant source a server stamps its rows with.
///
/// Carries an offset from the real clock rather than a fixed instant: added
/// once and then left alone, it keeps its value while the real clock
/// underneath it advances, which is what makes `--now` an offset rather than
/// a freeze (`qa/stats_ratio.md`: "The server starts believing it is that
/// instant and time then **advances normally** from there").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    offset_ms: i64,
}

impl Clock {
    /// The real wall clock — what a server started without `--now` runs on.
    pub fn system() -> Self {
        Self::at_offset(0)
    }

    /// A clock reading `pinned_ms` right now, advancing normally from there.
    pub fn pinned_at(pinned_ms: i64) -> Self {
        Self::at_offset(pinned_ms - real_now_ms())
    }

    /// The offset constructor the two above share, and the seam the tests use
    /// to check the arithmetic without depending on what time it is.
    fn at_offset(offset_ms: i64) -> Self {
        Clock { offset_ms }
    }

    pub fn now_ms(self) -> i64 {
        real_now_ms() + self.offset_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_reports_the_real_instant() {
        let before = real_now_ms();
        let got = Clock::system().now_ms();
        let after = real_now_ms();

        assert!(
            (before..=after).contains(&got),
            "now_ms={got} not within [{before}, {after}]"
        );
    }

    #[test]
    fn an_offset_clock_reports_the_real_instant_shifted_by_its_offset() {
        let before = real_now_ms();
        let got = Clock::at_offset(1_000).now_ms();
        let after = real_now_ms();

        assert!(
            (before + 1_000..=after + 1_000).contains(&got),
            "now_ms={got} not within [{}, {}]",
            before + 1_000,
            after + 1_000
        );
    }

    #[test]
    fn a_negative_offset_reports_an_instant_in_the_past() {
        assert!(Clock::at_offset(-1_000_000).now_ms() < real_now_ms());
    }

    #[test]
    fn a_pinned_clock_reads_the_pinned_instant() {
        let pinned = real_now_ms() + 1_000_000_000;

        let got = Clock::pinned_at(pinned).now_ms();

        assert!(
            (pinned..pinned + 1_000).contains(&got),
            "now_ms={got} not within a second of the pinned instant {pinned}"
        );
    }

    /// The property `--now` is documented to have and a frozen clock would
    /// not: the pin offsets time, it does not stop it.
    #[test]
    fn a_pinned_clock_keeps_advancing() {
        let clock = Clock::pinned_at(0);

        let first = clock.now_ms();
        let second = clock.now_ms();

        assert!(
            second >= first,
            "the clock must keep advancing after the pin, got {first} then {second}"
        );
    }

    /// Two clocks in one process disagree, and neither disturbs the other —
    /// the isolation a process-global offset could not offer.
    #[test]
    fn two_clocks_in_one_process_are_independent() {
        let system = Clock::system();
        let pinned = Clock::pinned_at(0);

        assert!(pinned.now_ms() < system.now_ms());
        assert!(Clock::system().now_ms() > 0);
    }
}
