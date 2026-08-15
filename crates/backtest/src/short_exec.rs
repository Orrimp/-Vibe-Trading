//! Pure, sync, deterministic single-coin short-execution helper (ADR-0068 D6).
//!
//! This module is the **single source of truth** for the signed open/cover/fund/
//! liquidate state transition for the single-coin directional short-selling
//! feature. Both the bake-off (`run_scenario` / `sma_composed_run`) AND the
//! agent forward loop (`spawn_trading_loop`) call it — the F5/F5b discipline,
//! Q-SS-6 resolution.
//!
//! **Scope of that claim, tightened by bug-log #80 (2026-08-15).** The two
//! lanes share the *sizing and solvency* decisions (see the caller-shapes
//! section below) but they no longer share the *execution model*: the bake-off
//! now routes its short opens/covers through `PaperEngine::step`, so they pay
//! slippage, the lot-size/min-notional venue filter and the fill-price model,
//! while the forward loop still fills at `mark` charging only the taker fee.
//! Do not read "single source of truth" as "the two lanes produce identical
//! fills" — they do not, and the same is already true of their LONG legs since
//! bug-log #79 wired the venue filter into the bake-off alone.
//!
//! ## Design
//!
//! - **Pure**: no I/O, no async, no clock reads, no global state. Arguments in,
//!   state-delta out. Trivially unit- and property-testable.
//! - **Deterministic**: no RNG; liquidation order is driven by the caller's
//!   iteration order (sorted BTreeMap or single-symbol direct call).
//! - **Port**: arithmetic ported verbatim from `montecarlo.rs:253-589` with
//!   `MAX_LEVERAGE = 1` and `maintenance_margin_frac = 0.5` inherited.
//! - **No I/O constraint**: the helper accepts and returns plain `Decimal`
//!   values; no `Order`, no `Fill`, no async. The caller owns the matching
//!   engine step; this helper owns only the P&L accounting after a fill.
//!
//! ## Two caller shapes — sizing is shared, accounting is not (bug-log #80)
//!
//! [`plan_open_short`] is the **sizing + initial-margin gate**; it touches no
//! state. Two kinds of caller consume it:
//!
//! 1. **Engine-routed** (`scenarios/sma_composed_run.rs`, the advisor/bake-off
//!    lane): takes the planned qty, submits a real [`trading_core::Order`]
//!    through `PaperEngine::step`, and does its accounting from the returned
//!    `Fill`. It therefore pays slippage, the lot-size / min-notional venue
//!    filter and the fill-price model — the same friction the long legs pay.
//!    This is the shape bug-log #80 required: a ranked comparison must not
//!    span two friction models.
//! 2. **Self-accounting** ([`try_open_short`] / [`try_cover_short`], used by
//!    the agent forward loop in `crates/agent/src/runtime.rs`): does the cash
//!    arithmetic itself at `mark`. It charges the taker fee but models no
//!    slippage, no lot filter and no fill-price mode.
//!
//! [`try_open_short`] delegates its sizing to [`plan_open_short`], so the two
//! shapes cannot drift on *what* is traded — only on the friction charged for
//! it. The forward loop is deliberately out of scope for #80 (it is not a
//! ranked comparison); routing it through an engine is a separate change.
//!
//! ## Honest unbounded-loss (ADR-0068 D5)
//!
//! Liquidation may drive `cash` negative in an extreme gap — this is modelled
//! honestly. The caller MUST NOT clamp `cash` at zero after calling
//! [`apply_liquidation`]. Losses are **NOT** capped.
//!
//! ## Single-coin vs multi-symbol difference
//!
//! The MN feature in `montecarlo.rs` manages a `BTreeMap<Symbol, Decimal>`.
//! Single-coin operates on one position directly. The helper functions here
//! are therefore single-symbol; the caller handles the symbol routing.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::FundingRate;

// ── Constants (ported verbatim from montecarlo.rs:90 and :100) ───────────────

/// Maximum leverage for short positions (1x fully-collateralized, v1 default).
///
/// Ported verbatim from `montecarlo.rs:90`. A short open at notional N
/// reserves `margin = N / MAX_LEVERAGE = N` — the full notional is reserved.
pub const MAX_LEVERAGE: Decimal = Decimal::ONE;

/// Maintenance-margin fraction for forced liquidation.
///
/// Ported verbatim from `montecarlo.rs` `maintenance_margin_frac()`.
/// Returns 0.5: liquidate when `equity < 0.5 × gross_short_notional`.
#[must_use]
pub fn maintenance_margin_frac() -> Decimal {
    dec!(0.5)
}

// ── Open-short accounting ─────────────────────────────────────────────────────

/// A sized short-open intent that cleared the initial-margin gate.
///
/// Returned by [`plan_open_short`]. Carries BOTH the qty and the notional it
/// was derived from: `qty = notional / mark` is a `Decimal` division and is
/// therefore not exactly invertible, so a caller that needs the pre-trade
/// notional (e.g. [`try_open_short`]'s own cash arithmetic) must read it here
/// rather than recompute `qty * mark`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortOpenPlan {
    /// Base-asset quantity to sell short (> 0).
    pub qty: Decimal,
    /// Quote-currency notional the qty was sized from (> 0).
    pub notional: Decimal,
}

/// Size a short open and apply the initial-margin gate — **no state change**.
///
/// This is the sizing half of [`try_open_short`], split out (bug-log #80) so
/// that a caller which routes the order through `PaperEngine::step` applies
/// exactly the same 10 %-of-equity clip and the same solvency gate as the
/// caller which does its own accounting. [`try_open_short`] delegates here, so
/// the two cannot drift.
///
/// - Sizes as 10% of equity (D-MN.2 fraction), capped at available `cash`.
/// - Reserves `margin = notional / MAX_LEVERAGE`.
/// - Returns `None` when `cash < margin + fee_estimate`, when the notional or
///   `mark` is non-positive, or when the resulting qty rounds to zero — the
///   exact skip-set of the pre-#80 `try_open_short` gate.
#[must_use]
pub fn plan_open_short(
    cash: Decimal,
    mark: Decimal,
    taker_fee_bps: u32,
    equity: Decimal,
) -> Option<ShortOpenPlan> {
    let fraction = dec!(0.10);
    let target_notional = equity * fraction;
    let notional = if target_notional > cash {
        cash
    } else {
        target_notional
    };
    let margin = notional / MAX_LEVERAGE;
    let fee_bps_decimal = Decimal::new(i64::from(taker_fee_bps), 4); // bps → fraction
    let fee_estimate = notional * fee_bps_decimal;

    if cash < margin + fee_estimate || notional <= Decimal::ZERO || mark <= Decimal::ZERO {
        return None;
    }

    let qty_raw = notional / mark;
    if qty_raw <= Decimal::ZERO {
        return None;
    }

    Some(ShortOpenPlan {
        qty: qty_raw,
        notional,
    })
}

/// Result of [`try_open_short`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenShortResult {
    /// New cash balance after the open.
    pub cash: Decimal,
    /// New (negative) position qty after the open.
    pub position_qty: Decimal,
    /// Whether the open was executed (`false` = skipped due to solvency gate).
    pub executed: bool,
}

/// Attempt to open (or extend) a short position.
///
/// Mirrors `montecarlo.rs:402-455` exactly (the "Sell when flat-or-short" arm).
///
/// - Sizes as 10% of equity (D-MN.2 fraction).
/// - Applies the initial-margin gate: skips if `cash < margin + fee_estimate`
///   (exact structure of the long Bug-B skip — symmetric solvency guards).
/// - On execution: `cash += notional − fee` (proceeds in, fee out);
///   `position_qty -= fill_qty` (goes more negative).
///
/// # Arguments
///
/// - `cash`: current cash balance.
/// - `position_qty`: current signed position (≤ 0 for flat/short).
/// - `mark`: current mark price.
/// - `taker_fee_bps`: fee in basis points (integer, e.g. 4 = 0.04%).
/// - `equity`: current equity (`cash + position_qty × mark`), used for sizing.
///
/// # Returns
///
/// [`OpenShortResult`] with the updated `cash` and `position_qty`. If the
/// solvency gate fires, `executed = false` and the values are unchanged.
#[must_use]
pub fn try_open_short(
    cash: Decimal,
    position_qty: Decimal,
    mark: Decimal,
    taker_fee_bps: u32,
    equity: Decimal,
) -> OpenShortResult {
    debug_assert!(
        position_qty <= Decimal::ZERO,
        "try_open_short: position_qty must be ≤ 0"
    );
    debug_assert!(mark > Decimal::ZERO, "try_open_short: mark must be > 0");

    // bug-log #80 — sizing + gate delegated to `plan_open_short` so the
    // engine-routed caller and this self-accounting caller can never disagree
    // about WHAT to trade. `plan.notional` (not `plan.qty * mark`) preserves
    // the pre-split arithmetic exactly: `qty = notional / mark` is a Decimal
    // division and re-multiplying does not round-trip.
    let Some(plan) = plan_open_short(cash, mark, taker_fee_bps, equity) else {
        return OpenShortResult {
            cash,
            position_qty,
            executed: false,
        };
    };
    let notional = plan.notional;
    let qty_raw = plan.qty;
    let fee_bps_decimal = Decimal::new(i64::from(taker_fee_bps), 4); // bps → fraction

    // Execute: proceeds in (notional), fee out, qty goes negative.
    let fee = notional * fee_bps_decimal;
    let new_cash = cash + notional - fee;
    let new_qty = position_qty - qty_raw;

    OpenShortResult {
        cash: new_cash,
        position_qty: new_qty,
        executed: true,
    }
}

// ── Cover-short accounting ────────────────────────────────────────────────────

/// Result of [`try_cover_short`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverShortResult {
    /// New cash balance after the cover.
    pub cash: Decimal,
    /// New position qty after the cover (should be ≈ 0 after full cover).
    pub position_qty: Decimal,
    /// Whether the cover was executed.
    pub executed: bool,
}

/// Cover an open short position (Buy-to-cover).
///
/// Mirrors `montecarlo.rs:253-299` exactly (the "Buy when short" arm).
///
/// - Covers the ENTIRE short at `mark`.
/// - Applies the solvency guard: if `cash < cover_cost + fee`, skip the cover
///   (the same guard as the long Buy arm — symmetric solvency, per D-MN.2).
/// - On execution: `cash -= notional + fee`; `position_qty += cover_qty`.
///
/// # Arguments
///
/// - `cash`: current cash balance.
/// - `position_qty`: current signed position (< 0 = short; must be negative).
/// - `mark`: current mark price.
/// - `taker_fee_bps`: fee in basis points.
///
/// # Returns
///
/// [`CoverShortResult`] with updated state. If the solvency guard fires,
/// `executed = false` and values are unchanged.
#[must_use]
pub fn try_cover_short(
    cash: Decimal,
    position_qty: Decimal,
    mark: Decimal,
    taker_fee_bps: u32,
) -> CoverShortResult {
    debug_assert!(
        position_qty < Decimal::ZERO,
        "try_cover_short: position_qty must be < 0"
    );
    debug_assert!(mark > Decimal::ZERO, "try_cover_short: mark must be > 0");

    let cover_qty = (-position_qty).max(Decimal::ZERO);
    if cover_qty <= Decimal::ZERO {
        return CoverShortResult {
            cash,
            position_qty,
            executed: false,
        };
    }

    let cover_notional = cover_qty * mark;
    let fee_bps_decimal = Decimal::new(i64::from(taker_fee_bps), 4);
    let cover_fee = cover_notional * fee_bps_decimal;
    let total_cost = cover_notional + cover_fee;

    if total_cost > cash {
        // Solvency guard: skip rather than go negative on a voluntary cover.
        // Liquidation (forced cover) is a separate path that ALLOWS negative cash.
        tracing::warn!(
            %cash,
            %total_cost,
            "short_exec::try_cover_short: solvency guard triggered — skipping cover"
        );
        return CoverShortResult {
            cash,
            position_qty,
            executed: false,
        };
    }

    let new_cash = cash - total_cost;
    let new_qty = position_qty + cover_qty; // approaches 0 from below

    CoverShortResult {
        cash: new_cash,
        position_qty: new_qty,
        executed: true,
    }
}

// ── Per-bar funding accrual ───────────────────────────────────────────────────

/// Accrue per-bar funding for a single open position.
///
/// Mirrors `montecarlo.rs:460-520` (the per-symbol accrual inner loop), adapted
/// for single-coin single-bar invocation.
///
/// Formula: `cashflow = qty × mark × (−rate_per_bar)`
///
/// For a short (`qty < 0`): `cashflow = (neg) × (−pos) = pos` → short receives
/// funding when rate > 0 (the standard perp mechanic: longs pay shorts on
/// positive funding names). For a long (`qty > 0`): `cashflow = neg` → long pays.
///
/// Returns the new cash balance (`cash + cashflow`).
///
/// Returns `cash` unchanged when `rate == 0` (the documented negative control).
#[must_use]
pub fn accrue_funding(
    cash: Decimal,
    position_qty: Decimal,
    mark: Decimal,
    funding_rate: FundingRate,
    bar_hours: Decimal,
) -> Decimal {
    if position_qty == Decimal::ZERO || mark <= Decimal::ZERO {
        return cash;
    }
    let cashflow = funding_rate.cashflow_for_position(position_qty, mark, bar_hours);
    cash + cashflow
}

// ── Maintenance-margin liquidation ───────────────────────────────────────────

/// Result of [`check_and_liquidate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidationResult {
    /// New cash balance after liquidation (may be negative — honest loss).
    pub cash: Decimal,
    /// New position qty after liquidation (0 when all shorts are covered).
    pub position_qty: Decimal,
    /// Whether a liquidation was triggered.
    pub liquidated: bool,
}

/// Check the maintenance-margin condition and force-cover if triggered.
///
/// Mirrors `montecarlo.rs:531-589` exactly (the liquidation block).
///
/// Condition: `equity < maintenance_margin_frac × gross_short_notional`
/// where `gross_short_notional = (−position_qty) × mark` for a single short.
///
/// When triggered: force-cover all shorts at `mark`; deduct `notional + fee`
/// from `cash`. Cash MAY go negative — this is the honest unbounded-loss
/// model (ADR-0068 D5). The caller MUST NOT clamp cash at zero.
///
/// When not triggered: returns `(cash, position_qty, liquidated=false)`
/// unchanged.
///
/// # Arguments
///
/// - `cash`: current cash balance.
/// - `position_qty`: current signed position (< 0 = short).
/// - `mark`: current mark price.
/// - `equity`: current equity (`cash + position_qty × mark`).
/// - `taker_fee_bps`: fee in basis points.
///
/// # ⚠️ Deliberate carve-out: this path is INVISIBLE to both friction-parity gates (bug-log #90)
///
/// This function force-covers at the **raw `mark`** with the taker fee only —
/// no slippage, no engine — and emits **no `Fill`**. Both short/long friction
/// parity gates (`backtest/tests/short_long_friction_parity_e2e.rs` and
/// `agent/tests/short_long_friction_parity_forward_e2e.rs`) measure the **fill
/// tape**, so a path that produces no fill cannot appear in their input, in
/// either direction. The gates are sound about what they see; this is outside
/// what they can see.
///
/// **This is not a repeat of #80's asymmetry.** Both production callers —
/// `agent/src/runtime.rs` (forward loop) and `scenarios/sma_composed_run.rs`
/// (ranking) — call it identically, so it is symmetric, and long positions have
/// no analogous forced-exit path to be cheaper than.
///
/// **Why it was not engine-routed with #80:** doing so would change *what* is
/// liquidated and *when*, not merely what it costs — slippage moves the cover
/// price, which moves the equity that triggered the liquidation. That is a
/// feedback loop, not a friction correction. `apply_engine_fill`'s own docs
/// record that the #80 fix scoped forced liquidation out on purpose ("a forced
/// cover … is deliberately allowed to drive cash negative").
///
/// **If you add a caller, or start routing other exits through here, say so in
/// bug-log #90 first** — the parity gates will not tell you, by construction.
#[must_use]
pub fn check_and_liquidate(
    cash: Decimal,
    position_qty: Decimal,
    mark: Decimal,
    equity: Decimal,
    taker_fee_bps: u32,
) -> LiquidationResult {
    if position_qty >= Decimal::ZERO || mark <= Decimal::ZERO {
        return LiquidationResult {
            cash,
            position_qty,
            liquidated: false,
        };
    }

    let gross_short_notional = (-position_qty) * mark;
    if gross_short_notional <= Decimal::ZERO {
        return LiquidationResult {
            cash,
            position_qty,
            liquidated: false,
        };
    }

    if equity >= maintenance_margin_frac() * gross_short_notional {
        // Margin is adequate — no liquidation.
        return LiquidationResult {
            cash,
            position_qty,
            liquidated: false,
        };
    }

    // Force-cover at mark; cash may go negative (honest unbounded-loss).
    let cover_qty = -position_qty; // positive qty to buy-to-cover
    let cover_notional = cover_qty * mark;
    let fee_bps_decimal = Decimal::new(i64::from(taker_fee_bps), 4);
    let cover_fee = cover_notional * fee_bps_decimal;
    let total_cover_cost = cover_notional + cover_fee;

    // Pay the cover cost — cash MAY go negative (honest extreme liquidation).
    let new_cash = cash - total_cover_cost;

    tracing::warn!(
        %equity,
        %gross_short_notional,
        %cover_notional,
        "short_exec::check_and_liquidate: maintenance-margin triggered, force-covering short"
    );

    LiquidationResult {
        cash: new_cash,
        position_qty: Decimal::ZERO,
        liquidated: true,
    }
}

// ── Unit + property tests ─────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const FEE_BPS: u32 = 4;

    // ── try_open_short ────────────────────────────────────────────────────────

    #[test]
    fn open_short_basic_execution() {
        // cash=100, mark=50, equity=100 (flat)
        // fraction=0.10 → target_notional=10, margin=10, fee_est=0.004
        // cash >= margin+fee_est ✓ → execute
        // qty_raw = 10/50 = 0.2
        // new_cash = 100 + 10 - 0.004 = 109.996
        // new_qty = 0 - 0.2 = -0.2
        let res = try_open_short(dec!(100), Decimal::ZERO, dec!(50), FEE_BPS, dec!(100));
        assert!(res.executed);
        assert!(
            res.position_qty < Decimal::ZERO,
            "short must have negative qty"
        );
        assert!(res.cash > dec!(100), "proceeds must increase cash");
    }

    /// bug-log #80 — `plan_open_short` and `try_open_short` must agree on the
    /// SKIP-SET and on the traded qty, or the engine-routed bake-off caller and
    /// the self-accounting forward-loop caller would size differently.
    ///
    /// Sweeps the gate boundary in both directions plus the degenerate inputs.
    #[test]
    fn plan_and_try_open_short_agree_on_skip_set_and_qty() {
        let cases: [(Decimal, Decimal, Decimal); 8] = [
            (dec!(100), dec!(50), dec!(100)),      // normal execute
            (Decimal::ZERO, dec!(50), dec!(100)),  // no cash → skip
            (dec!(100), dec!(50), Decimal::ZERO),  // zero equity → skip
            (dec!(100), dec!(50), dec!(-10)),      // negative equity → skip
            (dec!(0.001), dec!(50), dec!(100)),    // cash < margin+fee → skip
            (dec!(100), dec!(0.0001), dec!(100)),  // dust mark → executes
            (dec!(1_000_000), dec!(50), dec!(10)), // target < cash → execute
            (dec!(10), dec!(50), dec!(1_000_000)), // target > cash → clipped
        ];
        for (cash, mark, equity) in cases {
            let plan = plan_open_short(cash, mark, FEE_BPS, equity);
            let res = try_open_short(cash, Decimal::ZERO, mark, FEE_BPS, equity);
            assert_eq!(
                plan.is_some(),
                res.executed,
                "skip-set drift at cash={cash} mark={mark} equity={equity}: \
                 plan={plan:?} executed={}",
                res.executed
            );
            if let Some(plan) = plan {
                assert_eq!(
                    -res.position_qty, plan.qty,
                    "qty drift at cash={cash} mark={mark} equity={equity}"
                );
                assert_eq!(
                    res.cash,
                    cash + plan.notional - plan.notional * Decimal::new(i64::from(FEE_BPS), 4),
                    "cash arithmetic must be derived from plan.notional, not qty*mark"
                );
            }
        }
    }

    #[test]
    fn open_short_solvency_gate_fires_when_no_cash() {
        let res = try_open_short(
            Decimal::ZERO, // zero cash
            Decimal::ZERO,
            dec!(50),
            FEE_BPS,
            dec!(100),
        );
        // target_notional = 100*0.1 = 10, cash = 0 → skipped
        assert!(!res.executed);
        assert_eq!(res.cash, Decimal::ZERO);
        assert_eq!(res.position_qty, Decimal::ZERO);
    }

    #[test]
    fn open_short_zero_equity_skips() {
        let res = try_open_short(
            dec!(100),
            Decimal::ZERO,
            dec!(50),
            FEE_BPS,
            Decimal::ZERO, // equity = 0 → target_notional = 0
        );
        assert!(!res.executed);
    }

    // ── try_cover_short ───────────────────────────────────────────────────────

    #[test]
    fn cover_short_realizes_profit_on_price_drop() {
        // Open at price 100, cover at price 80.
        // open: qty=-0.1, cash += 100*0.1 - fee = 10 - 0.0004 = 9.9996; let's simplify
        // We'll directly test cover math:
        // cash=109, position_qty=-0.1, mark=80
        // cover_qty=0.1, notional=8, fee=0.0032, total_cost=8.0032
        // new_cash = 109 - 8.0032 = 100.9968  (profit relative to open)
        let res = try_cover_short(dec!(109), dec!(-0.1), dec!(80), FEE_BPS);
        assert!(res.executed);
        assert_eq!(res.position_qty, Decimal::ZERO);
        // cash after cover > initial (profit)
        // open_at_100: cash was 100, then +10-fee = 109.9996≈110 after open.
        // cover_at_80: cash goes from ~110 to 110-8-fee = ~101.99 (profit of ~2)
        assert!(
            res.cash > dec!(100),
            "covering at 80 after open at 100 should profit"
        );
    }

    #[test]
    fn cover_short_realizes_loss_on_price_rise() {
        // cash=110, position_qty=-0.1, mark=120 (price rose, loss)
        // cover_qty=0.1, notional=12, fee=0.0048, total_cost=12.0048
        // new_cash = 110 - 12.0048 = 97.9952 (loss)
        let res = try_cover_short(dec!(110), dec!(-0.1), dec!(120), FEE_BPS);
        assert!(res.executed);
        assert_eq!(res.position_qty, Decimal::ZERO);
        assert!(
            res.cash < dec!(100),
            "covering at 120 after open at 100 should lose"
        );
    }

    #[test]
    fn cover_short_solvency_guard_skips() {
        // cash=1 (insufficient), cover would cost 10
        let res = try_cover_short(dec!(1), dec!(-0.1), dec!(100), FEE_BPS);
        assert!(!res.executed);
        assert_eq!(res.cash, dec!(1));
        assert_eq!(res.position_qty, dec!(-0.1));
    }

    // ── accrue_funding ────────────────────────────────────────────────────────

    #[test]
    fn funding_zero_rate_is_no_op() {
        let rate = FundingRate::zero();
        let cash = accrue_funding(dec!(100), dec!(-0.1), dec!(50_000), rate, dec!(1));
        assert_eq!(cash, dec!(100));
    }

    #[test]
    fn funding_short_position_receives_positive_funding() {
        // rate = DEFAULT (0.0001/8h), bar_hours = 1h → rate_per_bar = 0.0001/8
        // But let's use rate=0.0008 → per_bar = 0.0001
        // qty=-1, mark=50_000 → notional=-50_000
        // cashflow = -50_000 × (-0.0001) = +5
        let rate = FundingRate::new(dec!(0.0008)).unwrap();
        let new_cash = accrue_funding(dec!(1000), dec!(-1), dec!(50_000), rate, dec!(1));
        assert_eq!(new_cash, dec!(1005));
    }

    #[test]
    fn funding_flat_position_is_no_op() {
        let rate = FundingRate::default();
        let cash = accrue_funding(dec!(100), Decimal::ZERO, dec!(50_000), rate, dec!(1));
        assert_eq!(cash, dec!(100));
    }

    // ── check_and_liquidate ───────────────────────────────────────────────────

    #[test]
    fn liquidation_fires_below_maintenance_floor() {
        // position_qty=-1, mark=100, equity=40 (< 0.5 × 100 = 50)
        // cover_qty=1, notional=100, fee=0.004, total_cost=100.004
        // new_cash = cash - 100.004 (may go negative if cash < 100.004)
        let cash = dec!(40);
        let equity = dec!(40); // already below floor
        let res = check_and_liquidate(cash, dec!(-1), dec!(100), equity, FEE_BPS);
        assert!(res.liquidated);
        assert_eq!(res.position_qty, Decimal::ZERO);
        // Cash goes negative (honest unbounded loss)
        assert!(
            res.cash < Decimal::ZERO,
            "liquidation should drive cash negative: {}",
            res.cash
        );
    }

    #[test]
    fn liquidation_does_not_fire_above_floor() {
        // equity=60 > 0.5 × 100 = 50 → no liquidation
        let res = check_and_liquidate(dec!(60), dec!(-1), dec!(100), dec!(60), FEE_BPS);
        assert!(!res.liquidated);
        assert_eq!(res.position_qty, dec!(-1));
        assert_eq!(res.cash, dec!(60));
    }

    #[test]
    fn liquidation_skips_on_long_position() {
        // positive qty → no liquidation check
        let res = check_and_liquidate(dec!(100), dec!(1), dec!(100), dec!(200), FEE_BPS);
        assert!(!res.liquidated);
    }

    #[test]
    fn liquidation_skips_on_flat() {
        let res = check_and_liquidate(dec!(100), Decimal::ZERO, dec!(100), dec!(100), FEE_BPS);
        assert!(!res.liquidated);
    }

    // ── End-to-end open-cover profit test ─────────────────────────────────────

    /// Open short at P, cover at Q < P → realize (P−Q)·qty profit minus fees.
    #[test]
    fn open_at_p_cover_at_q_realizes_profit() {
        let initial_cash = dec!(10_000);
        let open_price = dec!(100);
        let cover_price = dec!(80);
        let equity = initial_cash; // flat at start

        let open_res = try_open_short(initial_cash, Decimal::ZERO, open_price, FEE_BPS, equity);
        assert!(open_res.executed);
        let short_qty = open_res.position_qty; // negative
        let cash_after_open = open_res.cash;

        let cover_res = try_cover_short(cash_after_open, short_qty, cover_price, FEE_BPS);
        assert!(cover_res.executed);
        assert_eq!(cover_res.position_qty, Decimal::ZERO);

        // Final equity = cash_after_cover + 0 (flat) > initial
        let final_equity = cover_res.cash;
        assert!(
            final_equity > initial_cash,
            "open at {open_price} cover at {cover_price}: final={final_equity} should > initial={initial_cash}"
        );
    }

    /// Open short at P, cover at Q > P → realize (P−Q)·qty loss (negative).
    #[test]
    fn open_at_p_cover_at_q_greater_p_realizes_loss() {
        let initial_cash = dec!(10_000);
        let open_price = dec!(100);
        let cover_price = dec!(120);
        let equity = initial_cash;

        let open_res = try_open_short(initial_cash, Decimal::ZERO, open_price, FEE_BPS, equity);
        assert!(open_res.executed);
        let cover_res = try_cover_short(open_res.cash, open_res.position_qty, cover_price, FEE_BPS);
        assert!(cover_res.executed);

        let final_equity = cover_res.cash;
        assert!(
            final_equity < initial_cash,
            "open at {open_price} cover at {cover_price}: final={final_equity} should < initial={initial_cash}"
        );
    }

    /// Liquidation drives cash negative on a sharp up-gap — honest unbounded loss.
    #[test]
    fn liquidation_drives_cash_negative_on_gap() {
        // Start: cash=50, short qty=-1, mark=50 → equity=0 < 0.5×50=25
        // Force-cover at mark=200 (a 4× price spike after the short opened at 50)
        // cover_notional = 1 × 200 = 200, fee = 0.008, total_cost = 200.008
        // cash was 50 (after opening at 50: cash=50 + 50-fee≈100, position=-1)
        // Let's set it up: cash=100, position=-1, mark=200, equity=100+(-1×200)=-100
        // equity=-100 < 0.5×200=100 → liquidation fires
        // new_cash = 100 - 200 - 0.008 = -100.008 → NEGATIVE (honest loss)
        let res = check_and_liquidate(
            dec!(100),
            dec!(-1),
            dec!(200),
            dec!(-100), // equity = 100 + (-1 × 200)
            FEE_BPS,
        );
        assert!(res.liquidated);
        assert_eq!(res.position_qty, Decimal::ZERO);
        assert!(
            res.cash < Decimal::ZERO,
            "sharp gap liquidation must drive cash negative (honest loss): cash={}",
            res.cash
        );
    }
}
