//! Integration tests for `retry::run_with_backoff` (T1902 acceptance).
//!
//! Re-states the three T1902 acceptance points against the public
//! `llm::retry` surface (`cargo test -p llm --test retry_test`):
//!
//! - (a) 3 × 429 → 200 succeeds.
//! - (b) 4 × 429 returns `LlmError::RateLimited { retries: 3 }`.
//! - (c) `Retry-After: 2` pushes the next sleep to ≥ 2s.
//!
//! Uses `tokio::time::pause()` so wall-clock-sensitive assertions are
//! deterministic. The in-crate unit tests in `crates/llm/src/retry.rs`
//! cover the same surface plus `Fatal` propagation and `Transient`
//! handling — this file re-asserts the acceptance contract from the
//! crate's public-API edge.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use llm::LlmError;
use llm::retry::{RetryError, run_with_backoff};

#[tokio::test(start_paused = true)]
async fn t1902_three_429s_then_success() {
    let attempts = Arc::new(AtomicU8::new(0));
    let a2 = attempts.clone();
    let result: Result<&'static str, LlmError> = run_with_backoff(3, || {
        let a = a2.clone();
        async move {
            let n = a.fetch_add(1, Ordering::SeqCst);
            if n < 3 {
                Err(RetryError::RateLimited { retry_after: None })
            } else {
                Ok("ok")
            }
        }
    })
    .await;
    assert!(matches!(result, Ok("ok")));
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
}

#[tokio::test(start_paused = true)]
async fn t1902_four_429s_exhausts_budget() {
    let result: Result<(), LlmError> = run_with_backoff(3, || async {
        Err::<(), _>(RetryError::RateLimited { retry_after: None })
    })
    .await;
    match result {
        Err(LlmError::RateLimited { retries }) => assert_eq!(retries, 3),
        other => panic!("expected RateLimited {{ retries: 3 }}, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn t1902_retry_after_2s_pushes_sleep() {
    let start = tokio::time::Instant::now();
    let attempts = Arc::new(AtomicU8::new(0));
    let a2 = attempts.clone();
    let result: Result<&'static str, LlmError> = run_with_backoff(3, || {
        let a = a2.clone();
        async move {
            let n = a.fetch_add(1, Ordering::SeqCst);
            if n < 1 {
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
        elapsed >= Duration::from_secs(2),
        "elapsed {elapsed:?} >= 2s"
    );
}
