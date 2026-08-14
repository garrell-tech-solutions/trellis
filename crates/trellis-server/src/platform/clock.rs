use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Added to the real wall clock on every read. Zero until `set_pinned_now`
/// runs, and unchanged by the clock's own advance from then on -- `--now`
/// offsets the clock, it does not freeze it (`qa/stats_ratio.md`).
static OFFSET_MS: AtomicI64 = AtomicI64::new(0);

fn real_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
}

/// The offset that makes `real_now_ms` read as `pinned_ms` right now: added
/// once, it keeps its value while the real clock underneath it keeps
/// advancing, which is what turns "pin" into "offset" rather than "freeze".
fn offset_for_pin(pinned_ms: i64, real_ms: i64) -> i64 {
    pinned_ms - real_ms
}

pub(crate) fn now_ms() -> i64 {
    real_now_ms() + OFFSET_MS.load(Ordering::Relaxed)
}

/// `--now`'s whole implementation: pins the process clock so `now_ms` reads
/// `pinned_ms` right now, and advances normally from there. Applies for the
/// life of the process (`OFFSET_MS` is process-global) -- there is exactly
/// one clock a process runs on, and `--now` must be supplied on every start
/// (`qa/stats_ratio.md`: "applies per process").
pub fn set_pinned_now(pinned_ms: i64) {
    OFFSET_MS.store(offset_for_pin(pinned_ms, real_now_ms()), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    #[test]
    fn now_ms_reflects_the_current_wall_clock_time() {
        let before = epoch_ms();
        let got = now_ms();
        let after = epoch_ms();
        assert!(
            got >= before && got <= after,
            "now_ms={got} not within [{before}, {after}]"
        );
    }

    #[test]
    fn offset_for_pin_is_the_difference_between_the_pinned_and_real_instant() {
        assert_eq!(offset_for_pin(1_000, 200), 800);
        assert_eq!(offset_for_pin(200, 1_000), -800);
        assert_eq!(offset_for_pin(500, 500), 0);
    }

    /// Exercises the stateful pair together, then restores `OFFSET_MS` to
    /// zero itself rather than leaving the process pinned -- this static is
    /// shared by every test in the binary, unlike the pure math above.
    #[test]
    fn set_pinned_now_makes_now_ms_read_the_pinned_instant_and_keep_advancing() {
        let real_before = real_now_ms();
        let pinned = real_before + 1_000_000_000;

        set_pinned_now(pinned);
        let first = now_ms();
        let second = now_ms();

        OFFSET_MS.store(0, Ordering::Relaxed);

        assert!(
            first >= pinned && first < pinned + 1_000,
            "now_ms={first} not within a second of the pinned instant {pinned}"
        );
        assert!(
            second >= first,
            "the clock must keep advancing after the pin, got {first} then {second}"
        );
    }
}
