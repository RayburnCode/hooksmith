use std::time::Duration;

/// Policy controlling how [`HttpClient::post_json_with_retry`] retries failed
/// requests.
///
/// Successive retries use **exponential backoff**: the delay before attempt `n`
/// (0-indexed) is `base_delay × 2ⁿ`. When `jitter` is enabled a random
/// fraction of the current step is added on top, which reduces thundering-herd
/// bursts when many senders retry simultaneously.
///
/// # Example
///
/// ```rust,ignore
/// use std::time::Duration;
/// use hooksmith_core::RetryPolicy;
///
/// let policy = RetryPolicy {
///     max_attempts: 4,
///     base_delay:   Duration::from_millis(250),
///     jitter:       true,
/// };
///
/// client.post_json_with_retry(url, &payload, &policy).await?;
/// ```
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum total attempts (including the first try).  Clamped to at least 1.
    pub max_attempts: u32,

    /// Delay before the first retry.  Each subsequent retry doubles this value.
    pub base_delay: Duration,

    /// When `true`, add a random sub-delay ≤ the current step to spread out
    /// concurrent retries.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    /// Returns a sensible default: 3 attempts, 500 ms base delay, jitter on.
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            jitter: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sensible() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_attempts, 3);
        assert_eq!(p.base_delay, Duration::from_millis(500));
        assert!(p.jitter, "jitter should be on by default");
    }

    #[test]
    fn clone_is_independent() {
        let a = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            jitter: false,
        };
        let b = a.clone();
        assert_eq!(b.max_attempts, 5);
        assert_eq!(b.base_delay, Duration::from_millis(100));
        assert!(!b.jitter);
    }

    #[test]
    fn custom_values_are_preserved() {
        let p = RetryPolicy {
            max_attempts: 10,
            base_delay: Duration::from_secs(2),
            ..Default::default()
        };
        assert_eq!(p.max_attempts, 10);
        assert_eq!(p.base_delay, Duration::from_secs(2));
        assert!(p.jitter); // inherited from Default
    }
}
