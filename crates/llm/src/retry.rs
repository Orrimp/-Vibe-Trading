//! Retry helper — full-jitter exponential backoff per Design Q9.
//!
//! Q9 resolution (`spec/v2-llm-strategy/feature.md:1791`) places the
//! retry policy in **leaf provider impls** (not a generic decorator),
//! sharing one helper to avoid 3× duplicated logic. Backoff is base
//! 500ms, cap 8s, **full jitter** (AWS-recommended):
//!
//! ```text
//! sleep_ms = rng.gen_range(0..=cap_ms)
//! cap_ms   = min(8_000, 500 * 2^attempt)
//! ```
//!
//! `Retry-After: <secs>` headers (Anthropic + OpenAI) are honored: the
//! next sleep is `max(retry_after, computed_backoff)`. Ollama does not
//! 429 locally, so its provider impl passes `max_retries = 0`.
//!
//! `LlmError::Network` propagates immediately (transport-level failures
//! usually indicate config problems — DNS, TLS, wrong base URL — that
//! retries don't fix). The leaf provider classifies its outcome into
//! [`RetryError`]; this helper only sees the classification.

use std::future::Future;
use std::time::Duration;

use rand::Rng;

use crate::error::LlmError;

/// Internal classification of a single attempt's outcome.
///
/// Leaf provider impls map their raw transport result onto this enum
/// before handing back to [`run_with_backoff`]:
///
/// - HTTP 429 → [`RetryError::RateLimited`] (optionally carrying a
///   parsed `Retry-After` header).
/// - HTTP 503 → [`RetryError::Transient`].
/// - HTTP 4xx (other) / 5xx (other) / parse failures → [`RetryError::Fatal`]
///   wrapping the appropriate [`LlmError`] variant.
/// - Transport-level [`reqwest::Error`] → [`RetryError::Fatal`] wrapping
///   [`LlmError::Network`] (no retry — see module docs).
#[derive(Debug)]
pub enum RetryError {
    /// 429 Too Many Requests, optionally with a parsed `Retry-After`
    /// duration.
    RateLimited {
        /// Provider-suggested sleep duration from `Retry-After` header.
        retry_after: Option<Duration>,
    },
    /// 503 Service Unavailable or comparable transient class. Retried
    /// with the same backoff curve as `RateLimited` but no `retry_after`
    /// hint.
    Transient,
    /// Don't retry — surface immediately. Used for 4xx auth failures,
    /// 5xx non-transient, parse failures, and transport errors.
    Fatal(LlmError),
}

/// Base backoff in milliseconds (Q9a — strawman accepted).
const BACKOFF_BASE_MS: u64 = 500;
/// Cap backoff in milliseconds (Q9a — strawman accepted).
const BACKOFF_CAP_MS: u64 = 8_000;

/// Compute the full-jitter backoff cap for `attempt` (0-indexed).
///
/// `cap_ms = min(8000, 500 * 2^attempt)`. Saturating shifts so a
/// pathological caller passing `attempt = 64` still caps at 8s rather
/// than wrapping.
fn cap_ms_for_attempt(attempt: u8) -> u64 {
    BACKOFF_BASE_MS
        .checked_shl(u32::from(attempt))
        .unwrap_or(BACKOFF_CAP_MS)
        .min(BACKOFF_CAP_MS)
}

/// Sample one full-jitter sleep for the given attempt.
///
/// `sleep_ms = rng.gen_range(0..=cap_ms_for_attempt(attempt))` — the AWS
/// "full jitter" formula. Visible (`pub(crate)`) so the test module can
/// exercise it without spinning up a real provider.
fn sample_backoff(attempt: u8) -> Duration {
    let cap = cap_ms_for_attempt(attempt);
    let ms = rand::rng().random_range(0..=cap);
    Duration::from_millis(ms)
}

/// Run `operation` with full-jitter exponential backoff.
///
/// On the first attempt and each retry, `operation` is invoked and its
/// `Result<T, RetryError>` is classified:
///
/// - `Ok(t)` — returned immediately.
/// - `Err(RetryError::Fatal(e))` — `e` is returned immediately. No retry.
/// - `Err(RetryError::RateLimited { retry_after })` — sleep
///   `max(retry_after, computed_backoff)`, then retry (subject to budget).
/// - `Err(RetryError::Transient)` — sleep `computed_backoff`, then retry
///   (subject to budget).
///
/// After `max_retries` retries (so `max_retries + 1` total attempts) the
/// helper returns `LlmError::RateLimited { retries: max_retries }` (which
/// is also the route for exhausted `Transient`, because at v2 we don't
/// distinguish on exhaustion).
///
/// # Errors
///
/// - The raw `LlmError` produced by [`RetryError::Fatal`].
/// - `LlmError::RateLimited { retries }` when retry budget is exhausted.
pub async fn run_with_backoff<F, Fut, T>(max_retries: u8, mut operation: F) -> Result<T, LlmError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RetryError>>,
{
    let mut attempt: u8 = 0;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(RetryError::Fatal(e)) => return Err(e),
            Err(retryable) => {
                if attempt >= max_retries {
                    return Err(LlmError::RateLimited {
                        retries: max_retries,
                    });
                }
                let backoff = sample_backoff(attempt);
                let sleep = match retryable {
                    RetryError::RateLimited {
                        retry_after: Some(d),
                    } => std::cmp::max(d, backoff),
                    _ => backoff,
                };
                tracing::debug!(
                    target: "llm.retry",
                    attempt,
                    sleep_ms = sleep.as_millis() as u64,
                    "retrying after backoff"
                );
                tokio::time::sleep(sleep).await;
                attempt += 1;
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    /// T1902 acceptance (a): 3 × 429 then 200 succeeds within the retry
    /// budget. Uses `tokio::time::pause()` for deterministic time so the
    /// test is not flaky on slow CI.
    #[tokio::test(start_paused = true)]
    async fn t1902_three_429s_then_success_returns_ok() {
        let attempts = Arc::new(AtomicU8::new(0));
        let attempts_for_op = attempts.clone();

        let result: Result<&'static str, LlmError> = run_with_backoff(3, || {
            let attempts = attempts_for_op.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 3 {
                    Err(RetryError::RateLimited { retry_after: None })
                } else {
                    Ok("ok")
                }
            }
        })
        .await;

        assert!(matches!(result, Ok("ok")), "got: {result:?}");
        assert_eq!(attempts.load(Ordering::SeqCst), 4);
    }

    /// T1902 acceptance (b): 4 × 429 (exhausts the 3-retry budget)
    /// returns `LlmError::RateLimited { retries: 3 }`.
    #[tokio::test(start_paused = true)]
    async fn t1902_exhausted_budget_returns_rate_limited() {
        let result: Result<(), LlmError> = run_with_backoff(3, || async {
            Err::<(), _>(RetryError::RateLimited { retry_after: None })
        })
        .await;

        match result {
            Err(LlmError::RateLimited { retries }) => assert_eq!(retries, 3),
            other => panic!("expected RateLimited {{ retries: 3 }}, got {other:?}"),
        }
    }

    /// T1902 acceptance (c): `Retry-After: 2s` header pushes the next
    /// sleep to ≥ 2s. We measure with `tokio::time` paused — auto-
    /// advance between awaits — and assert the total elapsed across two
    /// honored Retry-After waits is ≥ 4s. The 2nd attempt's sleep is
    /// `max(2s, jitter)`, hence ≥ 2s regardless of the jitter draw.
    #[tokio::test(start_paused = true)]
    async fn t1902_retry_after_header_pushes_sleep() {
        let start = tokio::time::Instant::now();
        let attempts = Arc::new(AtomicU8::new(0));
        let attempts_for_op = attempts.clone();

        let result: Result<&'static str, LlmError> = run_with_backoff(3, || {
            let attempts = attempts_for_op.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(RetryError::RateLimited {
                        retry_after: Some(Duration::from_secs(2)),
                    })
                } else {
                    Ok("ok")
                }
            }
        })
        .await;

        assert!(matches!(result, Ok("ok")));
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_secs(4),
            "elapsed {elapsed:?} should be >= 4s (2 honored Retry-After waits)"
        );
    }

    /// Fatal errors propagate immediately — no retry.
    #[tokio::test(start_paused = true)]
    async fn t1902_fatal_returns_immediately() {
        let attempts = Arc::new(AtomicU8::new(0));
        let attempts_for_op = attempts.clone();

        let result: Result<(), LlmError> = run_with_backoff(3, || {
            let attempts = attempts_for_op.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(RetryError::Fatal(LlmError::Auth("missing key".into())))
            }
        })
        .await;

        assert!(matches!(result, Err(LlmError::Auth(_))));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    /// `Transient` (503-class) is retried like rate-limit, and on
    /// exhaustion also surfaces as `RateLimited { retries }` (we don't
    /// fan the exhaustion variant at v2).
    #[tokio::test(start_paused = true)]
    async fn t1902_transient_retried_then_exhausts() {
        let result: Result<(), LlmError> =
            run_with_backoff(2, || async { Err::<(), _>(RetryError::Transient) }).await;

        match result {
            Err(LlmError::RateLimited { retries }) => assert_eq!(retries, 2),
            other => panic!("expected RateLimited {{ retries: 2 }}, got {other:?}"),
        }
    }

    /// Backoff cap calculation: `attempt = 0` ≤ 500ms, `attempt = 1` ≤ 1s,
    /// …, `attempt = 4` capped at 8s.
    #[test]
    fn t1902_cap_ms_obeys_min_8s_curve() {
        assert_eq!(cap_ms_for_attempt(0), 500);
        assert_eq!(cap_ms_for_attempt(1), 1_000);
        assert_eq!(cap_ms_for_attempt(2), 2_000);
        assert_eq!(cap_ms_for_attempt(3), 4_000);
        assert_eq!(cap_ms_for_attempt(4), 8_000);
        // Cap holds for arbitrary large attempt counts.
        assert_eq!(cap_ms_for_attempt(10), 8_000);
        assert_eq!(cap_ms_for_attempt(64), 8_000);
    }
}
