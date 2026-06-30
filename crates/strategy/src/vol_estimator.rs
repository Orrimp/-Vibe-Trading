//! Shared multi-horizon σ̂ vol estimator (P1-5 / ADR-0079).
//!
//! A **pure, stateless** module — plain functions, no traits, no I/O.
//! Both overlay consumers (`vol_targeting_overlay.rs` for P1-4 and
//! `drawdown_control_overlay.rs` for P1-3) will call these after their
//! respective reparameterisations land.
//!
//! # Design decisions (v2-architecture.md §1 P1-5 + §6.0 D5)
//!
//! - **Home:** `crates/strategy/src/vol_estimator.rs` — it is a *sizing
//!   input*, both consumers live in `strategy`, and `ui` never touches it.
//!   Per operator-ratified D5 this is the binding location; do NOT move to
//!   `crates/forecast`.
//! - **`f64` for statistics:** log-returns and vol computations use `f64`.
//!   `Decimal` is used only at the money/price boundary (`log_returns_from_bars`
//!   converts `bar.close.get()` to `f64` at that boundary). Pattern mirrors
//!   `crates/backtest/src/bakeoff/scorecard.rs`.
//! - **No `SystemTime` / no RNG:** all functions are deterministic pure
//!   reductions. Determinism checklist satisfied by construction.
//!
//! # Functions provided
//!
//! | Function | Description |
//! |---|---|
//! | [`log_returns_from_bars`] | Extract `ln(close_t / close_{t-1})` from `Bar` slice |
//! | [`realized_vol_from_returns`] | Simple trailing-window standard deviation of returns |
//! | [`ewma_realized_vol`] | Exponentially-weighted moving sigma (EWMA / RiskMetrics-style) |
//! | [`har_realized_vol`] | HAR-RV: daily + weekly (5-bar) + monthly (22-bar) equal-weight blend |
//!
//! # Half-life ↔ λ relationship (EWMA)
//!
//! The smoothing parameter λ in `ewma_realized_vol` determines how fast past
//! squared returns decay:
//!
//! ```text
//! weight on bar t-k  ∝  λ^k
//!
//! Half-life H  (bars) where the weight is halved:
//!   λ^H = 0.5  →  H = ln(0.5) / ln(λ)  →  λ = exp(ln(0.5) / H) = 0.5^(1/H)
//! ```
//!
//! The architect's preferred "slow ~126-day half-life" (P1-4 / P1-A):
//!
//! ```text
//! λ = exp(ln(0.5) / 126) ≈ 0.994_514   (daily bars)
//! ```
//!
//! For hourly bars a 126-day half-life is 126 × 24 = 3024 hours:
//!
//! ```text
//! λ = exp(ln(0.5) / 3024) ≈ 0.999_771   (hourly bars)
//! ```
//!
//! RiskMetrics λ = 0.94 corresponds to a half-life of
//! `ln(0.5) / ln(0.94) ≈ 11.3` days — much faster / more reactive.
//! The constant [`LAMBDA_126D_DAILY`] exports the slow default; consumers
//! are free to override.
//!
//! # References
//!
//! - v2-architecture.md §1 P1-5 + §6.0 D5 — home decision + rationale.
//! - research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md
//!   §6 P1-C — HAR-RV recipe + EWMA discussion.
//! - research/crypto-market-structure/application-volatility-regimes-and-overlays.md
//!   §6 F — crypto-specific cross-check.
//! - Corsi 2009 — "A Simple Approximate Long-Memory Model of Realized Volatility"
//!   (HAR-RV model).

#![allow(clippy::float_arithmetic)] // statistical computations — intentional
#![allow(clippy::cast_precision_loss)] // usize → f64 casts in statistics

use trading_core::Bar;

/// Slow EWMA λ for an approximate 126-day half-life on **daily** bars.
///
/// `λ = exp(ln(0.5) / 126) ≈ 0.994_514`.
///
/// For **hourly** bars (24h/day × 126 days = 3 024 bars) use
/// [`LAMBDA_126D_HOURLY`].
pub const LAMBDA_126D_DAILY: f64 = 0.994_513_935_616_829; // exp(ln(0.5)/126)

/// Slow EWMA λ for an approximate 126-day half-life on **hourly** bars.
///
/// `λ = exp(ln(0.5) / 3024) ≈ 0.999_771`.
pub const LAMBDA_126D_HOURLY: f64 = 0.999_770_810_930_342; // exp(ln(0.5)/3024)

/// RiskMetrics λ ≈ 0.94 (≈ 11.3-day half-life on daily bars).
///
/// More reactive than the 126-day slow default. Provided for comparison /
/// consumer override. The research recommendation (P1-A) prefers the
/// slower decay to avoid overtrading.
pub const LAMBDA_RISKMETRICS: f64 = 0.94;

// ── Helper: log-returns from Bar slices ──────────────────────────────────────

/// Extract log-returns from a bar slice.
///
/// Returns `ln(close_t / close_{t-1})` for each consecutive pair.  The
/// returned vector has length `bars.len() - 1`; for `bars.len() <= 1` it
/// returns an empty `Vec`.
///
/// # Decimal → f64 boundary
///
/// `bar.close.get()` returns a `rust_decimal::Decimal`; we convert via
/// `to_string().parse::<f64>()` (the established pattern for a lossless
/// decimal→float conversion at a stats boundary, matching
/// `crates/backtest/src/bakeoff/scorecard.rs`).  Non-positive closes are
/// treated as missing (the log-return is `0.0`) to avoid `ln(0)` blowup.
///
/// # Examples
///
/// ```
/// use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
/// use rust_decimal_macros::dec;
/// use strategy::vol_estimator::log_returns_from_bars;
/// // ... (see unit tests below for a self-contained example)
/// ```
#[must_use]
pub fn log_returns_from_bars(bars: &[Bar]) -> Vec<f64> {
    if bars.len() < 2 {
        return Vec::new();
    }
    let closes: Vec<f64> = bars
        .iter()
        .map(|b| {
            // `Decimal` → `f64` at the statistical boundary.
            // Using Decimal::to_string().parse is the most accurate path;
            // ToPrimitive::to_f64 can silently lose precision on large values.
            b.close.get().to_string().parse::<f64>().unwrap_or(0.0)
        })
        .collect();

    closes
        .windows(2)
        .map(|w| {
            let prev = w[0];
            let curr = w[1];
            if prev > 0.0 && curr > 0.0 {
                (curr / prev).ln()
            } else {
                0.0
            }
        })
        .collect()
}

// ── Realized vol: simple trailing-window stddev ───────────────────────────────

/// Simple realized volatility over a trailing window.
///
/// Computes the **population standard deviation** (biased) of the last
/// `window` returns in `returns`.  Uses the last `window.min(returns.len())`
/// observations — it does NOT require the full window to be available.
///
/// Returns `0.0` when the effective window is empty.
///
/// # Half-life note
///
/// This is an equal-weight estimator — no exponential decay.  Use
/// [`ewma_realized_vol`] for a decaying weight.
///
/// # Examples
///
/// Constant-returns series → σ = 0:
///
/// ```
/// use strategy::vol_estimator::realized_vol_from_returns;
/// let returns = vec![0.01; 20];
/// assert!((realized_vol_from_returns(&returns, 20) - 0.0).abs() < 1e-14);
/// ```
#[must_use]
pub fn realized_vol_from_returns(returns: &[f64], window: usize) -> f64 {
    if window == 0 || returns.is_empty() {
        return 0.0;
    }
    let n = window.min(returns.len());
    let slice = &returns[returns.len() - n..];
    population_stddev(slice)
}

// ── EWMA realized vol ─────────────────────────────────────────────────────────

/// Exponentially-weighted moving sigma (EWMA / RiskMetrics-style).
///
/// Computes a full σ series (one per return) using the recurrence:
///
/// ```text
/// σ²_t = (1 − λ) · r_t² + λ · σ²_{t-1}
/// ```
///
/// Initialised with the unconditional variance of the full `returns` series
/// (population variance), falling back to `r[0]²` if the series is constant.
///
/// The returned `Vec` has the same length as `returns`; for an empty input
/// the result is also empty.
///
/// # λ interpretation
///
/// | λ | Behaviour |
/// |---|---|
/// | `0.0` | σ²_t = r_t² — "last return only" |
/// | `1.0` | σ²_t = σ²_{t-1} — variance never updates (constant, equal weight over infinite history) |
/// | [`LAMBDA_126D_DAILY`] ≈ 0.9945 | Slow, ~126-day half-life (architect default) |
/// | [`LAMBDA_RISKMETRICS`] = 0.94 | RiskMetrics / faster decay |
///
/// # References
///
/// - RiskMetrics Technical Document (1996) — exponentially weighted covariance.
/// - research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md §6 P1-C.
#[must_use]
pub fn ewma_realized_vol(returns: &[f64], lambda: f64) -> Vec<f64> {
    if returns.is_empty() {
        return Vec::new();
    }
    // Clamp λ to [0, 1] — caller error guard.
    let lam = lambda.clamp(0.0, 1.0);

    // Initialise variance with population variance of the full series.
    let unconditional = population_variance(returns);
    let init_var = if unconditional > 0.0 {
        unconditional
    } else {
        // Constant or zero series — seed with first squared return (or tiny ε).
        let r0sq = returns[0] * returns[0];
        if r0sq > 0.0 { r0sq } else { 1e-16 }
    };

    let mut sigma2 = init_var;
    let mut result = Vec::with_capacity(returns.len());

    for &r in returns {
        sigma2 = (1.0 - lam) * r * r + lam * sigma2;
        result.push(sigma2.sqrt());
    }
    result
}

// ── HAR-RV (Heterogeneous Autoregressive Realized Variance) ─────────────────

/// HAR-RV — Heterogeneous Autoregressive Realized Volatility.
///
/// Blends three trailing realized-vol horizons with **equal weights** (1/3
/// each) following Corsi 2009:
///
/// ```text
/// σ̂_HAR_t = (1/3)·RV_daily_t + (1/3)·RV_weekly_t + (1/3)·RV_monthly_t
/// ```
///
/// where:
/// - `RV_daily_t` = `|r_t|` (1-bar absolute return as a proxy for daily RV).
/// - `RV_weekly_t` = mean of the last **5** `|r_t|` values.
/// - `RV_monthly_t` = mean of the last **22** `|r_t|` values.
///
/// **Note on equal weights:** The original Corsi (2009) model fits OLS
/// coefficients (β_d, β_w, β_m) on historical data. Here we use the equal-
/// weight (1/3, 1/3, 1/3) variant — the "parameter-light" form that avoids
/// a separate fitting step and is appropriate for a sizing input that must
/// remain simple and robust (per the research doc's explicit recommendation
/// "do not over-engineer").
///
/// # Returns
///
/// A `Vec<f64>` with the same length as `returns`.  Early observations
/// where the 5-bar or 22-bar lookback is not yet fully populated use
/// the available prefix (warm-up: equivalent to a smaller effective window).
///
/// Empty input → empty output.
///
/// # References
///
/// - Corsi, F. (2009). "A Simple Approximate Long-Memory Model of Realized
///   Volatility." *Journal of Financial Econometrics*, 7(2), 174–196.
/// - research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md §6 P1-C.
#[must_use]
pub fn har_realized_vol(returns: &[f64]) -> Vec<f64> {
    const WEEKLY_WINDOW: usize = 5;
    const MONTHLY_WINDOW: usize = 22;

    returns
        .iter()
        .enumerate()
        .map(|(i, _)| {
            // Daily component: absolute return at index i.
            let rv_daily = returns[i].abs();

            // Weekly component: mean absolute return over last WEEKLY_WINDOW bars.
            let weekly_start = i.saturating_sub(WEEKLY_WINDOW - 1);
            let weekly_slice = &returns[weekly_start..=i];
            let rv_weekly = mean_abs(weekly_slice);

            // Monthly component: mean absolute return over last MONTHLY_WINDOW bars.
            let monthly_start = i.saturating_sub(MONTHLY_WINDOW - 1);
            let monthly_slice = &returns[monthly_start..=i];
            let rv_monthly = mean_abs(monthly_slice);

            // Equal-weight blend (1/3 each).
            (rv_daily + rv_weekly + rv_monthly) / 3.0
        })
        .collect()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Population (biased) standard deviation of a slice.
fn population_stddev(xs: &[f64]) -> f64 {
    population_variance(xs).sqrt()
}

/// Population (biased) variance of a slice.
fn population_variance(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    xs.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / n
}

/// Mean of absolute values in a slice.
fn mean_abs(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().map(|&x| x.abs()).sum::<f64>() / xs.len() as f64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::symbol::Symbol;
    use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp, Venue};

    use super::*;

    // ── Bar builder ────────────────────────────────────────────────────────

    fn make_bar_at(close_dec: rust_decimal::Decimal, minute: i64) -> Bar {
        let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minute));
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: Price::new(close_dec).unwrap(),
            high: Price::new(close_dec).unwrap(),
            low: Price::new(close_dec).unwrap(),
            close: Price::new(close_dec).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    // ── log_returns_from_bars ─────────────────────────────────────────────

    #[test]
    fn log_returns_empty_input() {
        let result = log_returns_from_bars(&[]);
        assert!(result.is_empty(), "empty input → empty output");
    }

    #[test]
    fn log_returns_single_bar() {
        let bars = vec![make_bar_at(dec!(100), 0)];
        let result = log_returns_from_bars(&bars);
        assert!(
            result.is_empty(),
            "single bar → no returns (need 2 bars for 1 return)"
        );
    }

    #[test]
    fn log_returns_constant_price() {
        let bars: Vec<Bar> = (0..5).map(|i| make_bar_at(dec!(100), i)).collect();
        let result = log_returns_from_bars(&bars);
        assert_eq!(result.len(), 4, "n bars → n-1 returns");
        for &r in &result {
            assert!(r.abs() < 1e-12, "constant price → returns ≈ 0, got {r}");
        }
    }

    #[test]
    fn log_returns_double_price_step() {
        // close goes 100 → 200: ln(200/100) = ln(2) ≈ 0.693147
        let bars = vec![make_bar_at(dec!(100), 0), make_bar_at(dec!(200), 1)];
        let result = log_returns_from_bars(&bars);
        assert_eq!(result.len(), 1);
        let expected = 2.0_f64.ln();
        assert!(
            (result[0] - expected).abs() < 1e-9,
            "100→200 should give ln(2), got {}",
            result[0]
        );
    }

    #[test]
    fn log_returns_known_sequence() {
        // Prices: 100, 105, 110 → returns ≈ [ln(1.05), ln(110/105)]
        let bars = vec![
            make_bar_at(dec!(100), 0),
            make_bar_at(dec!(105), 1),
            make_bar_at(dec!(110), 2),
        ];
        let result = log_returns_from_bars(&bars);
        assert_eq!(result.len(), 2);
        let expected0 = (105.0_f64 / 100.0).ln();
        let expected1 = (110.0_f64 / 105.0).ln();
        assert!(
            (result[0] - expected0).abs() < 1e-9,
            "first return mismatch: {} vs {}",
            result[0],
            expected0
        );
        assert!(
            (result[1] - expected1).abs() < 1e-9,
            "second return mismatch: {} vs {}",
            result[1],
            expected1
        );
    }

    // ── realized_vol_from_returns ─────────────────────────────────────────

    #[test]
    fn realized_vol_constant_series_is_zero() {
        let returns = vec![0.01; 30];
        let vol = realized_vol_from_returns(&returns, 20);
        assert!(vol.abs() < 1e-12, "constant series → σ = 0, got {vol}");
    }

    #[test]
    fn realized_vol_window_zero_returns_zero() {
        let returns = vec![0.01, 0.02, -0.01];
        assert_eq!(
            realized_vol_from_returns(&returns, 0),
            0.0,
            "window = 0 → 0.0"
        );
    }

    #[test]
    fn realized_vol_window_larger_than_len_uses_full_slice() {
        // Asking for window=100 on a 3-element series should use all 3.
        let returns = vec![0.01, -0.01, 0.01];
        let vol_full = realized_vol_from_returns(&returns, 3);
        let vol_oversized = realized_vol_from_returns(&returns, 100);
        assert!(
            (vol_full - vol_oversized).abs() < 1e-14,
            "window > len should equal window = len"
        );
    }

    #[test]
    fn realized_vol_known_sample() {
        // Known variance: returns = [0.0, 1.0, 0.0, -1.0] → mean=0, var=0.5, stddev≈0.7071.
        let returns = vec![0.0_f64, 1.0, 0.0, -1.0];
        let vol = realized_vol_from_returns(&returns, 4);
        let expected = 0.5_f64.sqrt(); // population stddev
        assert!(
            (vol - expected).abs() < 1e-12,
            "known sample: expected {expected}, got {vol}"
        );
    }

    #[test]
    fn realized_vol_empty_returns_zero() {
        assert_eq!(realized_vol_from_returns(&[], 10), 0.0);
    }

    // ── ewma_realized_vol ─────────────────────────────────────────────────

    #[test]
    fn ewma_vol_empty_input() {
        assert!(ewma_realized_vol(&[], 0.94).is_empty());
    }

    #[test]
    fn ewma_vol_output_length_matches_input() {
        let returns = vec![0.01; 50];
        let result = ewma_realized_vol(&returns, LAMBDA_126D_DAILY);
        assert_eq!(result.len(), 50, "output length must equal input length");
    }

    #[test]
    fn ewma_vol_lambda_zero_gives_current_return() {
        // λ = 0: σ²_t = r_t² → σ_t = |r_t|.
        // Edge: seed = unconditional_var; for constant r=0.01, var=0, so seed
        // falls back to r[0]^2 = 0.0001.  After first step: σ² = 0*0.0001^2 + 0*seed...
        // Actually for λ=0: σ²_t = (1-0)*r²_t + 0*σ²_{t-1} = r²_t.
        let r = 0.05_f64;
        let returns = vec![r; 10];
        let result = ewma_realized_vol(&returns, 0.0);
        // Every element should be |r| = 0.05.
        for (i, &v) in result.iter().enumerate() {
            assert!(
                (v - r.abs()).abs() < 1e-12,
                "λ=0, step {i}: expected {}, got {v}",
                r.abs()
            );
        }
    }

    #[test]
    fn ewma_vol_lambda_one_is_constant() {
        // λ = 1: σ²_t = σ²_{t-1} — variance never updates.
        let returns: Vec<f64> = (0..20)
            .map(|i| if i % 2 == 0 { 0.01 } else { -0.03 })
            .collect();
        let result = ewma_realized_vol(&returns, 1.0);
        assert_eq!(result.len(), returns.len());
        // All values should equal the initial sigma (sqrt of unconditional variance).
        let first = result[0];
        for (i, &v) in result.iter().enumerate() {
            assert!(
                (v - first).abs() < 1e-14,
                "λ=1 should give constant output, step {i}: {v} vs {first}"
            );
        }
    }

    #[test]
    fn ewma_vol_all_positive() {
        // σ must always be positive.
        let returns: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin() * 0.02).collect();
        for &v in ewma_realized_vol(&returns, LAMBDA_126D_DAILY).iter() {
            assert!(v > 0.0, "EWMA vol must be positive, got {v}");
        }
    }

    #[test]
    fn ewma_vol_monotone_weight_property() {
        // A shock at position 0 should have lower influence than a shock at the end.
        // We build a series: many zeros, then a spike at the end.
        let n = 100;
        let mut returns_end_spike = vec![0.001_f64; n];
        returns_end_spike[n - 1] = 0.1; // spike at end

        let mut returns_start_spike = vec![0.001_f64; n];
        returns_start_spike[0] = 0.1; // spike at start

        let vol_end = ewma_realized_vol(&returns_end_spike, LAMBDA_126D_DAILY);
        let vol_start = ewma_realized_vol(&returns_start_spike, LAMBDA_126D_DAILY);

        // The end-spike series should have a higher σ at the last bar.
        assert!(
            vol_end[n - 1] > vol_start[n - 1],
            "end-spike should yield higher final vol than start-spike: {} vs {}",
            vol_end[n - 1],
            vol_start[n - 1]
        );
    }

    // ── har_realized_vol ──────────────────────────────────────────────────

    #[test]
    fn har_vol_empty_input() {
        assert!(har_realized_vol(&[]).is_empty());
    }

    #[test]
    fn har_vol_output_length_matches_input() {
        let returns = vec![0.01; 30];
        assert_eq!(har_realized_vol(&returns).len(), 30);
    }

    #[test]
    fn har_vol_all_positive_on_nonzero_series() {
        let returns: Vec<f64> = (0..50).map(|i| (i as f64 * 0.2).sin() * 0.03).collect();
        for (i, &v) in har_realized_vol(&returns).iter().enumerate() {
            assert!(v >= 0.0, "HAR vol must be non-negative, step {i}: {v}");
        }
    }

    #[test]
    fn har_vol_weekly_monthly_smooth_spike() {
        // The multi-horizon blend smooths transient spikes.
        // A single spike at bar i has full weight (1/3) in the daily component
        // but its weight in the 5-bar weekly and 22-bar monthly components is
        // diluted by the other (smaller) bars in those windows.
        //
        // Build a series of 30 bars, all at 0.001, with a spike at bar 15.
        // At bar 15:
        //   daily    = 0.10 (the spike)
        //   weekly   = (4×0.001 + 0.10) / 5 = 0.104/5 ≈ 0.0208
        //   monthly  = (14×0.001 + 0.10) / 15 (since 22-window is [1..15]) ≈ 0.0076
        // HAR[15] = (0.10 + 0.0208 + 0.0076) / 3 ≈ 0.0428  < daily 0.10
        //
        // So HAR_vol[15] < realized_vol_from_returns raw (which is just |r[15]|).
        let n = 30;
        let mut returns = vec![0.001_f64; n];
        returns[15] = 0.10; // spike

        let har = har_realized_vol(&returns);

        // HAR at the spike bar damps the raw spike.
        let raw_spike = returns[15].abs();
        let har_at_spike = har[15];
        assert!(
            har_at_spike < raw_spike,
            "HAR should damp spike: HAR[15]={har_at_spike} should be < raw {raw_spike}"
        );

        // One bar after the spike, the daily component drops back to 0.001,
        // but the weekly/monthly windows still include the spike → HAR[16] > HAR[10].
        // This shows that multi-horizon persistence outlasts the daily return.
        let har_before_spike = har[10];
        let har_after_spike = har[16];
        assert!(
            har_after_spike > har_before_spike,
            "HAR[16] should retain spike memory vs HAR[10]: {} vs {}",
            har_after_spike,
            har_before_spike
        );
    }

    #[test]
    fn har_vol_known_single_bar() {
        // A single bar: all three components = |r[0]| → HAR = |r[0]|.
        let r = 0.03_f64;
        let result = har_realized_vol(&[r]);
        assert_eq!(result.len(), 1);
        let expected = r.abs(); // (|r| + |r| + |r|) / 3
        assert!(
            (result[0] - expected).abs() < 1e-12,
            "single-bar HAR: expected {expected}, got {}",
            result[0]
        );
    }

    // ── Lambda constant sanity checks ─────────────────────────────────────

    #[test]
    fn lambda_126d_daily_half_life() {
        // Verify LAMBDA_126D_DAILY has a half-life of approximately 126 bars.
        let half_life = std::f64::consts::LN_2 / (-LAMBDA_126D_DAILY.ln());
        assert!(
            (half_life - 126.0).abs() < 0.01,
            "126d-daily λ half-life should be ≈126, got {half_life}"
        );
    }

    #[test]
    fn lambda_126d_hourly_half_life() {
        // 126 days × 24 hours = 3024 bars.
        let half_life = std::f64::consts::LN_2 / (-LAMBDA_126D_HOURLY.ln());
        assert!(
            (half_life - 3024.0).abs() < 1.0,
            "126d-hourly λ half-life should be ≈3024, got {half_life}"
        );
    }
}
