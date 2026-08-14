//! `DvolRegimeStrategy` — Deribit DVOL implied-vol regime long/flat filter.
//!
//! The `v0.dvol_regime` bake-off arm (ADR-0072 D3/D4). A hand-written
//! `Strategy` that holds the coin when implied vol is CALM (below its trailing
//! 30-day median) and steps to cash when vol is ELEVATED (at or above the
//! trailing median). The signal is LOCKED (no search, no parameter tuning).
//!
//! # Pre-registered signal (ADR-0072 D3, LOCKED)
//!
//! Per coin `s ∈ {BTC, ETH}`, daily grid, strictly causal:
//!
//! ```text
//! dvol_t   = as-of DVOL daily close for s at bar t's open
//!            (most-recent daily close with day_close_ts_ms ≤ open_ts(t))
//! med30_t  = trailing median of the last W=30 DISTINCT daily closes
//!            strictly before today (deduped via LOCF forward-fill)
//! weight_t = 1 (HOLD)   if dvol_t <  med30_t   (calm regime)
//!          = 0 (CASH)   if dvol_t >= med30_t   (stress regime, tie → cash)
//! ```
//!
//! Warm-up (< W=30 distinct daily closes available): weight=1 (HOLD = buy-and-hold
//! behavior — never diverge from benchmark before the signal is defined).
//!
//! **HOLD means holding the COIN, not cash (review 3-15 CRITICAL, bug-log #78).**
//! The first shipped implementation started from `weight: 1, is_long: false` and
//! emitted signals only on weight *transitions*, so warm-up produced the tuple
//! `(1, 1, false)` → `Hold` forever: the arm sat in **cash** through warm-up and
//! could not enter until a post-warm-up stress episode ENDED. In a persistently
//! calm window it never traded at all. That is the exact opposite of ADR-0072 D3,
//! which requires the arm to "only ever *subtract* exposure and never diverge from
//! buy-and-hold before the signal is defined". The emission rule below is now
//! stated over the TARGET STATE (`new_weight` vs the position), not over the
//! weight transition, which makes warm-up genuinely long from bar 0.
//!
//! # Rationale for hand-written `Strategy` (NOT a DSL `ComposedStrategy`)
//!
//! The composed-DSL `Expr` (`crates/strategy/src/composed/ast.rs`) reads only
//! `Indicator` / `BarField` / static `Param` scalar / arithmetic — there is no
//! per-bar exogenous-series term. A per-bar DVOL weight (from an external daily
//! series) is inexpressible in the DSL (ADR-0072 fact-2, confirmed in code).
//!
//! # Median computation (Decimal-exact, even W=30)
//!
//! `W=30` is even: the median is the mean of the 15th/16th order statistics
//! computed in `Decimal` (exact, no f64). The ring of distinct daily closes is
//! sorted each bar to pick the order statistics. Sorting a W=30 ring is O(30 log 30)
//! — negligible for a daily-cadence signal.
//!
//! # Distinct-daily-close dedup
//!
//! The bake-off runs on hourly bars. Each daily DVOL close is forward-filled
//! across the 24 intraday hours until the next daily close lands. To avoid
//! counting the same daily close 24 times, the ring is updated ONLY when the
//! as-of close changes vs the prior bar's close (the classic LOCF dedup).
//!
//! # Long-only clamp
//!
//! `short_enabled=false` → the existing `sma_composed_run.rs:534` clamp is active.
//! `Sell` only executes when currently long; `Buy` only when flat. Weight=0 only
//! generates a `Sell` signal if currently long; weight=1 only generates a `Buy`
//! if currently flat. Sizing is the bar-loop's `FixedFractionSizer(0.10)` (same
//! as all `v0.*` arms); "weight=1" = fully invested at the arm's fixed fraction.

use rust_decimal::Decimal;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Tick};

use crate::Strategy;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Lookback window in DISTINCT daily closes (W=30, locked, ADR-0072 D3 M-T1.1).
///
/// Theory-motivated: DVOL *is* a 30-day forward-vol gauge, so a 30-day trailing
/// window is horizon-matched, not fit. This is the ONLY parameter; any sensitivity
/// sweep must be explicitly labeled and separate from the crowning decision.
pub const DVOL_REGIME_WINDOW: usize = 30;

// ── DvolRegimeStrategy ────────────────────────────────────────────────────────

/// `v0.dvol_regime` — a DVOL-regime long/flat filter (ADR-0072).
///
/// Constructed with a pre-resolved as-of DVOL `Vec<Option<Decimal>>` — the
/// strategy does NO joining itself (pure + unit-testable against a synthetic
/// vector). `as_of_dvol[i]` is the as-of DVOL close at bar `i`'s open.
///
/// The strategy holds the coin (HOLD = weight 1) when DVOL is calm (below the
/// trailing 30-day median) and steps to cash (CASH = weight 0) when vol is
/// elevated. Warm-up → HOLD (the benchmark behavior) — and "hold" means holding
/// the COIN, so the very first warm-up bar emits a `Buy`.
///
/// # Long-only operation
///
/// Signal emission is a function of the TARGET state and the current position
/// (review 3-15 CRITICAL — it used to be a function of the weight *transition*,
/// which left the arm in cash for the whole warm-up):
/// - target weight 1 (calm **or warm-up**) while currently FLAT → `SignalKind::Buy`.
/// - target weight 0 (stress) while currently LONG → `SignalKind::Sell`.
/// - target already matches the position → `SignalKind::Hold`.
///
/// On every regime *flip* this is identical to the old transition rule; it
/// differs only where the prior weight equalled the new weight while the
/// position disagreed — which is exactly the warm-up bug (and any future case
/// where the arm's intent and its position have fallen out of sync).
///
/// The bar-loop's long-only clamp (`sma_composed_run.rs:534`) ensures these
/// signals only result in orders when in the right position state.
#[derive(Debug)]
pub struct DvolRegimeStrategy {
    id: StrategyId,
    // Retained for diagnostics / future logging; currently not consumed in hot path.
    #[allow(dead_code)]
    symbol: Symbol,
    /// Pre-resolved as-of DVOL closes (one entry per bar, indexed by bar cursor).
    as_of_dvol: Vec<Option<Decimal>>,
    /// Lookback window in distinct daily closes (default: DVOL_REGIME_WINDOW = 30).
    w: usize,
    /// Bar cursor: index into `as_of_dvol`.
    idx: usize,
    /// Ring of the last-W DISTINCT daily closes (de-duped LOCF).
    ring: Vec<Decimal>,
    /// The most-recent DVOL close seen (to detect when the day changes).
    last_close: Option<Decimal>,
    /// Whether currently in a long position (the arm's own model of its state).
    ///
    /// This is the ONLY state the signal rule reads besides the freshly-computed
    /// target weight. The old `weight` field (the PRIOR bar's target) was deleted
    /// with the transition rule it belonged to — see the type-level docs.
    is_long: bool,
}

impl DvolRegimeStrategy {
    /// Construct a new `DvolRegimeStrategy`.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The trading symbol (e.g. `Symbol::new("BTCUSDT")`).
    /// - `as_of_dvol`: Pre-resolved as-of DVOL closes, one per bar in order.
    ///   `as_of_dvol[i] = None` means the DVOL series hasn't started yet (warm-up).
    /// - `w`: Lookback window in distinct daily closes. Use `DVOL_REGIME_WINDOW`.
    ///
    /// # Degenerate window
    ///
    /// A median needs at least two order statistics, so `w < 2` is meaningless.
    /// This used to `assert!` — a **panic in library code**, which CLAUDE.md
    /// forbids (review 3-15 LOW). It now clamps to 2 and logs a `warn!`. Every
    /// production construction passes the [`DVOL_REGIME_WINDOW`] constant, so the
    /// clamp is unreachable outside a caller bug; the clamp exists so a caller
    /// bug degrades a probe arm instead of aborting the operator's bake-off.
    #[must_use]
    pub fn new(symbol: Symbol, as_of_dvol: Vec<Option<Decimal>>, w: usize) -> Self {
        let w = if w < 2 {
            tracing::warn!(
                requested_w = w,
                "DvolRegimeStrategy: window w must be >= 2 to compute a median — clamping to 2"
            );
            2
        } else {
            w
        };
        Self {
            id: StrategyId::new("dvol_regime"),
            symbol,
            as_of_dvol,
            w,
            idx: 0,
            ring: Vec::with_capacity(w),
            last_close: None,
            // Warm-up default: HOLD THE COIN (ADR-0072 D3 M-T1.4). `is_long: false`
            // is the FACTUAL starting position (nothing bought yet); the first
            // `on_bar` sees target weight 1 while flat and emits the entering Buy.
            is_long: false,
        }
    }

    /// Compute the trailing median of the ring of distinct daily closes.
    ///
    /// For even `w`: mean of the (w/2)th and (w/2+1)th order statistics.
    /// All arithmetic in `Decimal` (exact, no f64). The ring is sorted
    /// in-place for this computation (a temporary sort of ≤30 values).
    #[must_use]
    fn compute_median(ring: &[Decimal]) -> Decimal {
        let n = ring.len();
        debug_assert!(n >= 2, "ring must have ≥ 2 elements for median");
        let mut sorted = ring.to_vec();
        sorted.sort();
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / Decimal::TWO
        }
    }
}

impl Strategy for DvolRegimeStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    /// Process one bar, updating the regime weight and emitting a signal.
    ///
    /// Per ADR-0072 D3 (LOCKED):
    ///
    /// 1. Read `as_of_dvol[self.idx]`.
    /// 2. If changed vs `last_close` (or is the first close), push to ring
    ///    (capped at `w` — oldest evicted first).
    /// 3. If ring has `w` distinct closes: compute median, classify regime.
    ///    Else: warm-up → weight=1 (HOLD the coin).
    /// 4. Emit the signal that moves the position TO the target weight
    ///    (review 3-15: target-state, not weight-transition — see the type docs).
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // Retrieve the as-of DVOL close for this bar.
        let dvol_t = self.as_of_dvol.get(self.idx).copied().flatten();
        self.idx += 1;

        // Distinct-daily-close dedup: push only when the close value changes.
        if let Some(close) = dvol_t {
            let changed = self.last_close.is_none_or(|prev| prev != close);
            if changed {
                // Evict oldest if ring is full.
                if self.ring.len() == self.w {
                    self.ring.remove(0);
                }
                self.ring.push(close);
                self.last_close = Some(close);
            }
        }
        // If dvol_t is None (warm-up / no data), we do NOT update the ring.
        // The ring may be stale, but warm-up → weight=1 overrides it.

        // Compute the new weight.
        let new_weight = if self.ring.len() < self.w {
            // Warm-up: fewer than W distinct closes → HOLD (benchmark behavior).
            1u8
        } else if let Some(dvol_current) = dvol_t {
            let median = Self::compute_median(&self.ring);
            // dvol_t < median → calm → HOLD (weight=1)
            // dvol_t >= median → stress or tie → CASH (weight=0)
            if dvol_current < median { 1 } else { 0 }
        } else {
            // No DVOL data for this bar (DVOL series hasn't started) → HOLD.
            1u8
        };

        // Determine the signal that moves the CURRENT position to the TARGET
        // weight. Review 3-15 CRITICAL / bug-log #78: this used to key off the
        // weight TRANSITION `(prev_weight, new_weight, is_long)`, which made the
        // warm-up tuple `(1, 1, false)` fall through to `Hold` — the arm sat in
        // cash for the entire warm-up and could only enter after a post-warm-up
        // stress episode ended. Keying off the target state makes warm-up hold
        // the COIN, as ADR-0072 D3 requires, and is identical to the old rule on
        // every genuine regime flip.
        let kind = match (new_weight, self.is_long) {
            // Target = hold the coin (calm regime OR warm-up) while FLAT → enter.
            (1, false) => {
                self.is_long = true;
                SignalKind::Buy
            }
            // Target = cash (stress regime or tie) while LONG → exit.
            (0, true) => {
                self.is_long = false;
                SignalKind::Sell
            }
            // Position already matches the target → nothing to do.
            _ => SignalKind::Hold,
        };

        vec![Signal {
            strategy_id: self.id.clone(),
            symbol: bar.symbol.clone(),
            ts: bar.close_ts,
            kind,
            evidence: SignalEvidence::empty(),
            pair_data: None,
        }]
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        // Bake-off is bar-driven — tick signals are suppressed.
        vec![]
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "id": "dvol_regime",
            "signal": "v0.dvol_regime",
            "window_days": DVOL_REGIME_WINDOW,
            "cut": "trailing_median",
            "tie_resolution": "cash",
            "warmup_behavior": "hold"
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::pedantic,
    clippy::identity_op
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, SignalKind, Timeframe, Timestamp, Venue};

    fn make_bar_at(idx: usize, close: Decimal) -> Bar {
        let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(idx as i64));
        let price = Price::new(close).unwrap_or_else(|_| Price::new(dec!(1)).unwrap());
        let qty = Quantity::new(Decimal::ZERO).unwrap();
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneHour,
            venue: Venue::Binance,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: qty,
            trade_count: 1,
        }
    }

    /// Helper: build a `DvolRegimeStrategy` from a fixed synthetic as-of vec
    /// and process `n_bars` bars, returning the signal sequence.
    fn run_strategy(as_of_dvol: Vec<Option<Decimal>>, n_bars: usize, w: usize) -> Vec<SignalKind> {
        let mut strat = DvolRegimeStrategy::new(Symbol::new("BTCUSDT"), as_of_dvol, w);
        (0..n_bars)
            .map(|i| {
                let bar = make_bar_at(i, dec!(100));
                let sigs = strat.on_bar(&bar);
                sigs.first().map_or(SignalKind::Hold, |s| s.kind)
            })
            .collect()
    }

    // ── Warm-up ───────────────────────────────────────────────────────────────

    /// **Review 3-15 CRITICAL — the RED-before test.**
    ///
    /// ADR-0072 D3 M-T1.4: warm-up is `weight = 1` = "HOLD = benchmark
    /// behaviour", so that the arm "only ever *subtracts* exposure and never
    /// diverges from buy-and-hold before the signal is defined". Holding the
    /// benchmark means holding the **coin**. The shipped implementation held
    /// **cash**: it started from `weight: 1, is_long: false` and keyed emission
    /// off the weight *transition*, so warm-up produced `(1, 1, false)` → `Hold`
    /// on every bar and the arm never entered.
    ///
    /// This test fails on that implementation (bar 0 was `Hold`, and no bar in a
    /// warm-up-only window was ever `Buy`). It is the single assertion that
    /// distinguishes "hold the coin" from "hold cash" — every other warm-up
    /// assertion in this file is satisfied by both.
    #[test]
    fn warm_up_enters_the_coin_on_the_first_bar() {
        // Only 10 distinct DVOL values — far below W=30 → warm-up on every bar.
        let dvol: Vec<Option<Decimal>> =
            (0..10).map(|i| Some(dec!(50) + Decimal::from(i))).collect();
        let signals = run_strategy(dvol, 10, 30);
        assert_eq!(
            signals[0],
            SignalKind::Buy,
            "warm-up = HOLD THE COIN: the first bar must ENTER (Buy). A `Hold` here \
             means the arm is sitting in cash for the whole warm-up — the ADR-0072 D3 \
             violation found by the 3-15 review."
        );
        for (i, s) in signals.iter().enumerate().skip(1) {
            assert_eq!(
                *s,
                SignalKind::Hold,
                "bar {i}: already long and still in warm-up → Hold (no repeated Buys)"
            );
        }
    }

    /// None as-of DVOL (series not started) → warm-up → hold the COIN.
    ///
    /// This is the shape the bake-off's degraded path produces when the DVOL
    /// corpus is missing (`unwrap_or_default()` → empty series → every `as_of`
    /// is `None`). Bug-log #78: five code comments called that state a
    /// "buy-and-hold proxy" while it was in fact 100% cash. With the fix the
    /// description is finally true — the arm is long the coin from bar 0.
    #[test]
    fn none_dvol_holds_the_coin() {
        let dvol: Vec<Option<Decimal>> = vec![None; 10];
        let signals = run_strategy(dvol, 10, 30);
        assert_eq!(
            signals[0],
            SignalKind::Buy,
            "an all-None DVOL series is permanent warm-up — the arm must hold the COIN"
        );
        for (i, s) in signals.iter().enumerate().skip(1) {
            assert_eq!(*s, SignalKind::Hold, "bar {i}: stays long, emits Hold");
        }
    }

    /// A degenerate `w` must NOT panic (CLAUDE.md: no panics in library code).
    ///
    /// Review 3-15 LOW — `new()` used to `assert!(w >= 2)`. It now clamps.
    #[test]
    fn degenerate_window_clamps_instead_of_panicking() {
        for w in [0_usize, 1] {
            let strat = DvolRegimeStrategy::new(Symbol::new("BTCUSDT"), vec![Some(dec!(50)); 4], w);
            assert_eq!(
                strat.w, 2,
                "w={w} must clamp to the minimum median window 2"
            );
        }
        // And the clamped strategy still runs a full bar loop without panicking.
        let signals = run_strategy(vec![Some(dec!(50)), Some(dec!(60)), Some(dec!(40))], 3, 0);
        assert_eq!(signals.len(), 3);
    }

    // ── Regime classification ─────────────────────────────────────────────────

    /// Calm regime (dvol_t < median) → after warm-up, first entry is Buy.
    ///
    /// Strategy: fill the ring with W=3 distinct closes [50, 60, 70] (median=60),
    /// then emit dvol_t=55 (< 60) → calm → Buy.
    #[test]
    fn calm_regime_emits_buy_when_flat() {
        let w = 3;
        // 3 distinct closes: 50, 60, 70. Median = 60.
        let mut dvol: Vec<Option<Decimal>> = vec![Some(dec!(50)), Some(dec!(60)), Some(dec!(70))];
        // After 3 distinct closes, ring is full. Next bar: dvol=55 < 60 → calm → Buy.
        dvol.push(Some(dec!(55)));
        let signals = run_strategy(dvol, 4, w);
        // Exact trace under the target-state rule (review 3-15):
        // bar0: ring=[50], len=1 < 3 → warm-up → target 1, is_long=false → BUY (enter),
        //       is_long=true. (Pre-fix this was `Hold` and the arm stayed in cash.)
        // bar1: ring=[50,60], len=2 < 3 → warm-up → target 1, already long → Hold.
        // bar2: ring=[50,60,70] full. dvol=70, median=60 → 70>=60 → target 0, long → SELL.
        // bar3: dvol=55 changed → evict 50 → ring=[60,70,55], median=60.
        //       55 < 60 → target 1, flat → BUY.
        assert_eq!(
            signals[0],
            SignalKind::Buy,
            "bar 0 warm-up must ENTER the coin (ADR-0072 D3 = benchmark behaviour)"
        );
        assert_eq!(signals[1], SignalKind::Hold, "bar 1 warm-up, already long");
        assert_eq!(
            signals[2],
            SignalKind::Sell,
            "bar 2 (ring just full, dvol=70 ≥ median 60) → stress → exit to cash"
        );
        assert_eq!(signals[3], SignalKind::Buy, "calm after stress → Buy");
    }

    /// Stress regime (dvol_t >= median) → Sell when long.
    #[test]
    fn stress_regime_emits_sell_when_long() {
        let w = 3;
        // Warm up ring with [50,60,70], median=60.
        // Then enter calm (dvol=55 → Buy), then stress (dvol=80 → Sell).
        let dvol = vec![
            Some(dec!(50)),
            Some(dec!(60)),
            Some(dec!(70)),
            Some(dec!(55)), // calm → Buy
            Some(dec!(80)), // stress → Sell
        ];
        let signals = run_strategy(dvol, 5, w);
        assert_eq!(signals[3], SignalKind::Buy, "calm → Buy");
        assert_eq!(signals[4], SignalKind::Sell, "stress → Sell");
    }

    /// Tie (dvol_t == median) → cash (ADR-0072 D3 M-T1.3: tie → CASH).
    #[test]
    fn tie_resolves_to_cash() {
        let w = 3;
        // Ring [50,60,70], median=60. Then dvol=60 (exact tie).
        let dvol = vec![
            Some(dec!(50)),
            Some(dec!(60)),
            Some(dec!(70)),
            Some(dec!(55)), // calm → Buy (enter long)
            Some(dec!(60)), // tie (dvol==median) → CASH → Sell
        ];
        let signals = run_strategy(dvol, 5, w);
        assert_eq!(
            signals[4],
            SignalKind::Sell,
            "tie must resolve to cash (Sell when long)"
        );
    }

    // ── Distinct-daily-close dedup ────────────────────────────────────────────

    /// Repeated as-of close (LOCF forward-fill) is NOT added to the ring.
    ///
    /// If the same DVOL close repeats for 24 bars (hourly forward-fill),
    /// the ring should grow by only 1 for those 24 bars.
    #[test]
    fn repeated_close_not_added_to_ring() {
        let w = 30;
        // One distinct value repeated 24 times.
        let dvol: Vec<Option<Decimal>> = std::iter::repeat_n(Some(dec!(50)), 24).collect();
        let mut strat = DvolRegimeStrategy::new(Symbol::new("BTCUSDT"), dvol, w);
        for i in 0..24 {
            let bar = make_bar_at(i, dec!(100));
            strat.on_bar(&bar);
        }
        // Ring should have exactly 1 distinct value (50), not 24.
        assert_eq!(
            strat.ring.len(),
            1,
            "ring must deduplicate LOCF forward-fill"
        );
    }

    /// 30 DISTINCT daily closes fill the ring and enable the signal.
    #[test]
    fn thirty_distinct_closes_fill_ring() {
        let w = 30;
        // 30 distinct daily closes: 1, 2, ..., 30.
        let dvol: Vec<Option<Decimal>> = (1..=30).map(|i| Some(Decimal::from(i))).collect();
        let mut strat = DvolRegimeStrategy::new(Symbol::new("BTCUSDT"), dvol, w);
        for i in 0..30 {
            let bar = make_bar_at(i, dec!(100));
            strat.on_bar(&bar);
        }
        assert_eq!(
            strat.ring.len(),
            30,
            "ring must hold exactly W=30 distinct closes"
        );
    }

    // ── Median exactness (even W=30 = mean of 15th/16th order stats) ─────────

    /// For W=4 (even), median = mean of 2nd and 3rd order statistics.
    /// Ring [1,3,5,7] → sorted [1,3,5,7] → median = (3+5)/2 = 4.
    #[test]
    fn even_window_median_is_mean_of_two_middle() {
        let ring = vec![dec!(7), dec!(1), dec!(5), dec!(3)]; // unsorted
        let med = DvolRegimeStrategy::compute_median(&ring);
        // sorted: [1,3,5,7], n=4 → (sorted[1] + sorted[2]) / 2 = (3+5)/2 = 4
        assert_eq!(med, dec!(4), "even-window median must be mean of 2 middle");
    }

    /// For W=3 (odd), median = middle element.
    #[test]
    fn odd_window_median_is_middle() {
        let ring = vec![dec!(5), dec!(3), dec!(7)];
        let med = DvolRegimeStrategy::compute_median(&ring);
        assert_eq!(med, dec!(5), "odd-window median must be middle element");
    }

    // ── Signal-transition edges ───────────────────────────────────────────────

    /// Consecutive calm bars after Buy → Hold (no repeated Buys).
    #[test]
    fn hold_after_buy_when_still_calm() {
        let w = 3;
        let dvol = vec![
            Some(dec!(50)),
            Some(dec!(60)),
            Some(dec!(70)),
            Some(dec!(55)), // calm → Buy
            Some(dec!(45)), // still calm → Hold (already long)
        ];
        let signals = run_strategy(dvol, 5, w);
        assert_eq!(signals[3], SignalKind::Buy);
        assert_eq!(signals[4], SignalKind::Hold, "calm after Buy → Hold");
    }

    /// Consecutive stress bars after Sell → Hold (no repeated Sells).
    #[test]
    fn hold_after_sell_when_still_stress() {
        let w = 3;
        let dvol = vec![
            Some(dec!(50)),
            Some(dec!(60)),
            Some(dec!(70)),
            Some(dec!(55)), // calm → Buy
            Some(dec!(80)), // stress → Sell
            Some(dec!(90)), // still stress → Hold (already flat)
        ];
        let signals = run_strategy(dvol, 6, w);
        assert_eq!(signals[3], SignalKind::Buy);
        assert_eq!(signals[4], SignalKind::Sell);
        assert_eq!(signals[5], SignalKind::Hold, "stress after Sell → Hold");
    }

    // ── `config_schema` stub ───────────────────────────────────────────────────

    #[test]
    fn config_schema_is_valid_json() {
        let schema = DvolRegimeStrategy::config_schema();
        assert!(
            schema.is_object(),
            "config_schema must return a JSON object"
        );
        assert_eq!(
            schema["signal"],
            serde_json::json!("v0.dvol_regime"),
            "signal id must be v0.dvol_regime"
        );
    }
}
