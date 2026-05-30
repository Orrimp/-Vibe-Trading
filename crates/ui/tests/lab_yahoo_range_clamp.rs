//! D-ER-4 T3 + T4 — H3 future-dating + K3 clamp non-regression.
//!
//! lab-yahoo-empty-range-ux v0.1.0 — M-DEV.14.
//!
//! ## What this file tests
//!
//! **T3 — H3 future-dating boundary (D-ER-2 / Q2=(a)):**
//! - `Last30d` and `Last90d` under the wall clock: `end_ms <= now_ms`.
//!   The clamp guarantees a future end never escapes (e.g. skewed 2026 clock).
//! - A `Custom` range with a far-future `end_ms` is clamped to `now_ms`.
//!
//! **T4 — K3 clamp non-regression (byte-identical for past ranges):**
//! - `H1_2024` → exactly `(1_704_067_200_000, 1_719_792_000_000)`.
//! - `H2_2024` → exactly `(1_719_792_000_000, 1_735_689_600_000)`.
//!   The clamp is a proven no-op for these past-dated fixed ranges.
//!
//! ## `#[cfg(feature = "yahoo")]` gate
//!
//! `range_to_ms_pair` is compiled only under `yahoo`. Run with:
//! `cargo test -p ui --test lab_yahoo_range_clamp --features yahoo`

#![cfg(feature = "yahoo")]
#![allow(clippy::unwrap_used)]

use backtest::engine::DateRange;
use ui::lab::runner::range_to_ms_pair;

// ── T4 — K3 byte-identical for past fixed ranges ──────────────────────────────

/// T4a — H1_2024 returns its literal pair byte-identical (K3 falsifier).
///
/// `1_704_067_200_000` = 2024-01-01 00:00:00 UTC in millis.
/// `1_719_792_000_000` = 2024-07-01 00:00:00 UTC in millis.
///
/// The clamp `end_ms.min(now_ms)` is a proven no-op here: both ends are
/// well below any plausible `now_ms` under the 2026 clock. Any change to
/// `range_to_ms_pair` that alters these values breaks the K3 contract.
#[test]
fn h1_2024_byte_identical() {
    let (start_ms, end_ms) = range_to_ms_pair(&DateRange::H1_2024);
    assert_eq!(
        start_ms, 1_704_067_200_000,
        "H1_2024 start_ms must be 1_704_067_200_000 (2024-01-01 UTC); got {start_ms}"
    );
    assert_eq!(
        end_ms, 1_719_792_000_000,
        "H1_2024 end_ms must be 1_719_792_000_000 (2024-07-01 UTC); got {end_ms}"
    );
}

/// T4b — H2_2024 returns its literal pair byte-identical (K3 falsifier).
///
/// `1_719_792_000_000` = 2024-07-01 00:00:00 UTC in millis.
/// `1_735_689_600_000` = 2025-01-01 00:00:00 UTC in millis.
#[test]
fn h2_2024_byte_identical() {
    let (start_ms, end_ms) = range_to_ms_pair(&DateRange::H2_2024);
    assert_eq!(
        start_ms, 1_719_792_000_000,
        "H2_2024 start_ms must be 1_719_792_000_000 (2024-07-01 UTC); got {start_ms}"
    );
    assert_eq!(
        end_ms, 1_735_689_600_000,
        "H2_2024 end_ms must be 1_735_689_600_000 (2025-01-01 UTC); got {end_ms}"
    );
}

// ── T3 — H3 future-dating boundary ────────────────────────────────────────────

/// T3a — Last30d: end_ms must be <= now_ms (clamp holds).
///
/// Under the 2026 clock, `Last30d` would naturally compute end_ms == now_ms
/// (since the arm already sets `end = now_ms`). The test verifies the clamp
/// doesn't overshoot: `end_ms <= current_wall_clock_ms`.
#[test]
fn last30d_end_does_not_exceed_now() {
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    let (start_ms, end_ms) = range_to_ms_pair(&DateRange::Last30d);

    assert!(
        end_ms <= now_ms,
        "Last30d: end_ms ({end_ms}) must be <= now_ms ({now_ms}); \
         a future end escaped the clamp"
    );
    // Also verify start is before end (a degenerate future range with start>end
    // means no months are iterated → NoDataForRange — the correct outcome).
    // For Last30d the start is now - 30d which is always before now.
    assert!(
        start_ms < end_ms,
        "Last30d: start_ms ({start_ms}) must be < end_ms ({end_ms})"
    );
}

/// T3b — Last90d: end_ms must be <= now_ms (clamp holds).
#[test]
fn last90d_end_does_not_exceed_now() {
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    let (start_ms, end_ms) = range_to_ms_pair(&DateRange::Last90d);

    assert!(
        end_ms <= now_ms,
        "Last90d: end_ms ({end_ms}) must be <= now_ms ({now_ms}); \
         a future end escaped the clamp"
    );
    assert!(
        start_ms < end_ms,
        "Last90d: start_ms ({start_ms}) must be < end_ms ({end_ms})"
    );
}

/// T3c — Custom range with far-future end: end_ms is clamped to now_ms.
///
/// Uses `now + 10 days` for start and `now + 40 days` for end.
/// The clamp must reduce end_ms to now_ms (K3: applies ONLY when end > now).
/// start_ms is NEVER clamped (a future start with end clamped to now yields
/// start > end, which load_cached resolves to zero months → NoDataForRange).
#[test]
fn custom_future_end_clamped_to_now() {
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    const MS_PER_DAY: i64 = 86_400_000;

    let future_start_ms = now_ms + 10 * MS_PER_DAY;
    let future_end_ms = now_ms + 40 * MS_PER_DAY;

    let range = DateRange::Custom {
        start_ms: future_start_ms,
        end_ms: future_end_ms,
    };
    let (returned_start, returned_end) = range_to_ms_pair(&range);

    // end must be clamped to now (within 1 second tolerance for wall clock drift).
    assert!(
        returned_end <= now_ms + 1_000,
        "Custom future end: returned_end ({returned_end}) must be clamped to now_ms ({now_ms}); \
         the clamp is not working for Custom ranges"
    );
    // start is NOT clamped — it passes through unchanged.
    assert_eq!(
        returned_start, future_start_ms,
        "Custom future start: start_ms must NOT be clamped; \
         only end_ms is clamped (D-ER-2)"
    );
}

/// T3d — Custom range with past end: end_ms passes through byte-identical.
///
/// A past end (before now) must NOT be modified — the clamp applies ONLY
/// when end_ms > now_ms (K3). This falsifies any over-eager clamping that
/// would alter past ranges.
#[test]
fn custom_past_end_passes_through_unchanged() {
    const MS_PER_DAY: i64 = 86_400_000;
    // Use H1_2024 boundaries as a known-past range.
    let past_start: i64 = 1_704_067_200_000; // 2024-01-01 UTC
    let past_end: i64 = 1_719_792_000_000; // 2024-07-01 UTC

    let range = DateRange::Custom {
        start_ms: past_start,
        end_ms: past_end,
    };
    let (returned_start, returned_end) = range_to_ms_pair(&range);

    assert_eq!(
        returned_start, past_start,
        "Custom past start must be unchanged; got {returned_start}"
    );
    assert_eq!(
        returned_end, past_end,
        "Custom past end must be unchanged (K3: clamp only fires when end > now); \
         got {returned_end}"
    );
    let _ = MS_PER_DAY; // suppress unused warning
}
