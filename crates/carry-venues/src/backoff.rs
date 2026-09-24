use std::time::Duration;

const BASE: Duration = Duration::from_millis(500);
const MAX: Duration = Duration::from_secs(30);

pub(crate) fn backoff_delay(attempt: u32, jitter: f64) -> Duration {
    let exponential = BASE.saturating_mul(2u32.saturating_pow(attempt));
    exponential
        .min(MAX)
        .mul_f64(0.5 + 0.5 * jitter.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_per_attempt() {
        assert_eq!(backoff_delay(0, 1.0), Duration::from_millis(500));
        assert_eq!(backoff_delay(1, 1.0), Duration::from_secs(1));
        assert_eq!(backoff_delay(2, 1.0), Duration::from_secs(2));
    }

    #[test]
    fn is_capped() {
        assert_eq!(backoff_delay(10, 1.0), MAX);
        assert_eq!(backoff_delay(u32::MAX, 1.0), MAX);
    }

    #[test]
    fn jitter_scales_between_half_and_full() {
        assert_eq!(backoff_delay(1, 0.0), Duration::from_millis(500));
        assert_eq!(backoff_delay(1, 0.5), Duration::from_millis(750));
        assert_eq!(backoff_delay(1, 7.0), Duration::from_secs(1)); // clamped
    }
}
