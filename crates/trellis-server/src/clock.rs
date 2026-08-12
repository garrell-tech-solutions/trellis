use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_millis() as i64
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
}
