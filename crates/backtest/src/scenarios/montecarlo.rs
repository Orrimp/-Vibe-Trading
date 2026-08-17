//! Monte-Carlo robustness harness — `run_path` cell wrapper (M-DEV-3).
//!
//! Originally extracted (v0.1.0) as a behaviorally-preserving sibling of
//! `crates/backtest/src/scenarios/threshold_sweep::run_cell`; the two have
//! since DIVERGED: from v0.1.1 `run_path` carries the Bug-B long-only
//! solvency guard (pre-flight cash check + fill-loop guard), which
//! `run_cell` does NOT — `run_cell` retains the pre-Bug-B unguarded Buy
//! sizing inside the frozen-anchored threshold-sweep lane. That parity
//! question (plus the cross-symbol fill-mispricing both wrappers share) is
//! KNOWN and owned end-to-end by story 1-25-harness-fill-correctness-relock
//! (bug-log #67); do not re-fix it piecemeal here.
//!
//! The other structural difference: `run_cell` accepts a caller-supplied
//! `TcnOverlayMomentumStrategy`; `run_path` accepts a caller-supplied
//! `MomentumStrategy` (plain v1, no TCN overlay) and uses
//! `input.bars_override` to inject a bootstrap-generated path.
//!
//! ## ADR-0051 D1 — seed orthogonality
//!
//! The fill-tie-break seed is a separate parameter (`fill_seed`) and is
//! HELD CONSTANT at `0xC0FFEE` across ALL paths in the harness. The path
//! is already injected by C1's ensemble; `PaperEngine` only needs the
//! fill-tie-break seed, which must not vary per path (holding it constant
//! is the key to measuring only path-variance, not path ⊕ fill-tie-break
//! combined noise).
//!
//! ## R-NR.2 compliance
//!
//! This module contains NO change to `PaperEngine`, `MatchingEngine`, or
//! any scenario `run()`. It is a new thin wrapper that reuses the existing
//! engine with a caller-supplied path and strategy.

use anyhow::Result;
use rust_decimal::Decimal;
use trading_core::Bar;

use crate::cli_types::TcnScenarioInput;

// ── Result struct ──────────────────────────────────────────────────────────────

/// Result of a single Monte-Carlo path backtest run.
///
/// Carries the equity curve (the only output the harness reducer needs)
/// plus basic counts for observability.
#[derive(Debug, Clone)]
pub struct PathRunResult {
    /// Per-bar equity curve: `[initial_capital, equity_after_bar_0, …]`.
    /// Length = `n_bars + 1`. Used by the harness reducer to compute all
    /// per-path metric scalars.
    pub equity_curve: Vec<Decimal>,
    /// Number of fills executed on this path.
    pub trades: usize,
    /// Initial capital (carried for P(loss) computation in the reducer).
    pub initial_equity: Decimal,
    /// Final equity (convenience; redundant with `equity_curve.last()`).
    pub final_equity: Decimal,
    /// Minimum cash value observed during the run (solvency invariant probe).
    ///
    /// Under the v0.1.1 solvency guard this is guaranteed ≥ 0.
    /// Under Bug B (v0.1.0, no guard) this can be negative — the test
    /// `solvency_guard_run_path_regression_negative_cash_prevented` asserts
    /// this is ≥ 0, and goes RED when the guard is removed.
    pub min_cash_seen: Decimal,
    /// Total realized funding cashflow accrued on this path (Decimal).
    ///
    /// Sum of `notional × (−funding_rate)` over every settlement-boundary accrual.
    /// `Decimal::ZERO` for momentum/MR runs (no `funding_override` → the accrual
    /// block is never entered, so this stays zero and the existing anchors are
    /// byte-identical). Surfaced so the carry θ-surface report renders the per-cell
    /// realized-funding-harvested total (M-DEV-6) instead of a placeholder.
    pub realized_funding: Decimal,
    /// Number of bars where at least one long position was held (time-in-market counter).
    ///
    /// Pure observability counter — does NOT affect equity/sizing/accrual paths.
    /// The counter increments UNCONDITIONALLY (there is no enable flag): it is
    /// non-zero for momentum/MR/carry/basis runs too, since they hold long
    /// positions. Anchor-neutrality comes from RENDER GATING ONLY — the sweep
    /// renderer (`sweep_harness::render_surface_report`) emits the
    /// `time_in_market` column only under `selection_mode.is_ts()` (the TS
    /// lane), so this counter never reaches the hashed body of any other
    /// family's report. Rendering it unconditionally WOULD change the
    /// momentum/MR/carry/basis hashed bodies — a four-family anchor break — so
    /// do not un-gate the renderer. The counter itself does not alter any
    /// signal, order, or equity computation (M-DEV-4 / D-TSM.6.4, review 1-17).
    pub time_in_market_bars: u64,
    /// Number of maintenance-margin liquidation events on this path (M-DEV-3).
    ///
    /// Incremented each time the short-leg maintenance-margin rule force-closes ALL
    /// short positions at mark (`equity < maintenance_margin_frac × gross_short_notional`).
    /// `0` for every non-MN run (`k_short == 0` → the liquidation check is dead code).
    /// Default `0` for momentum/MR/carry/basis runs → anchor-neutral.
    pub liquidations: u64,
}

// ── run_path constants (D-MN.2 LOCKED hashed body fields) ────────────────────

/// Maximum leverage for short positions (D-MN.2 LOCKED, hashed body field).
///
/// `max_leverage = 1` = fully-collateralized shorts — the conservative v0.2.0 choice.
/// A short open reserves `margin = notional / max_leverage = notional`.
/// This constant is a hashed body field of the MN anchor; changing it = a new surface.
/// `pub` so the sweep harness can embed it in the MN grid-def string (anchor body K3).
pub const MAX_LEVERAGE: rust_decimal::Decimal = rust_decimal::Decimal::ONE;

/// Maintenance-margin fraction for short liquidation (D-MN.2 LOCKED, hashed body field).
///
/// `maintenance_margin_frac = 0.5` = liquidate shorts when equity falls below 50% of the
/// gross short notional. Conservative half-notional floor.
/// This constant is a hashed body field of the MN anchor; changing it = a new surface.
///
/// `Decimal::from_parts(5, 0, 0, false, 1)` = 5 × 10^{−1} = 0.5 exactly. A `const` is
/// needed because `dec!(0.5)` is not a valid const expression in stable Rust, and the
/// sweep harness embeds this value in the HASHED grid-def body
/// (`sweep_harness::mn_grid_def_string`) where it must render as `0.5`.
///
/// # One definition, two readers (review 1-21)
///
/// This used to be TWO independent definitions: this `const` (read by the renderer,
/// so it reached the hashed body) and a separate `maintenance_margin_frac()` returning
/// its own `dec!(0.5)` (read by the liquidation rule in `run_path`). Editing one made
/// the anchored body claim a margin the engine did not use — a silent divergence
/// between what a surface SAYS it ran at and what it ran at. The function is now a
/// thin accessor over this constant, and
/// `tests::maintenance_margin_frac_has_exactly_one_definition` pins both the equality
/// and the rendered literal.
pub const MAINTENANCE_MARGIN_FRAC: rust_decimal::Decimal =
    rust_decimal::Decimal::from_parts(5, 0, 0, false, 1);

/// Accessor for [`MAINTENANCE_MARGIN_FRAC`] — the value the liquidation rule uses.
///
/// Kept as a function purely for call-site ergonomics; it returns the SAME constant
/// the hashed body renders. See [`MAINTENANCE_MARGIN_FRAC`] for why that matters.
#[must_use]
pub fn maintenance_margin_frac() -> rust_decimal::Decimal {
    MAINTENANCE_MARGIN_FRAC
}

// ── run_path ───────────────────────────────────────────────────────────────────

/// Run one Monte-Carlo path with a caller-supplied `MomentumStrategy`.
///
/// Mirrors `threshold_sweep::run_cell` exactly, EXCEPT:
/// - Uses `strategy::MomentumStrategy` (plain v1, NOT the TCN overlay).
/// - `input.bars_override` MUST be `Some(…)` — the bootstrap path is
///   pre-loaded by the harness driver and injected here.
/// - `fill_seed` drives `PaperEngine::new` and is held CONSTANT across
///   all N paths (ADR-0051 D1 orthogonality invariant).
///
/// # Errors
///
/// Returns `Err` if `input.bars_override` is `None` (programming error —
/// the harness must always inject a path), or if the engine encounters
/// an error.
#[allow(clippy::too_many_lines)]
pub async fn run_path(
    input: TcnScenarioInput,
    fill_seed: u64,
    strategy: strategy::MomentumStrategy,
) -> Result<PathRunResult> {
    use rust_decimal_macros::dec;
    use trading_core::{
        Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
    };

    use crate::engine::MatchingEngine as _;
    use strategy::Strategy as _;

    // Extract fields we need before partially moving `input`.
    let scenario_name = input.scenario_name.clone();
    let slippage_bps = input.slippage_bps;
    let taker_fee_bps = input.taker_fee_bps;
    let initial_capital = input.initial_capital;
    // M-DEV-6: carry funding lookup (None for momentum/MR → accrual block never entered).
    let funding_override = input.funding_override;

    // M-DEV-3: bars_override MUST be set — the harness injects the bootstrap path.
    let merged_bars: Vec<Bar> = input.bars_override.ok_or_else(|| {
        anyhow::anyhow!("run_path: bars_override must be Some — inject the bootstrap path")
    })?;

    let bar_count = merged_bars.len();
    tracing::debug!(
        bar_count,
        scenario = %scenario_name,
        fill_seed,
        "montecarlo::run_path starting"
    );

    // ── Paper matching engine (ADR-0051 D1: fill_seed is CONSTANT across paths)
    let match_config = crate::paper::MatchConfig {
        slippage_bps,
        taker_fee_bps,
        maker_fee_bps: 2,
        fill_price_mode: crate::paper::FillPriceMode::BarClose,
    };
    let mut engine = crate::PaperEngine::new(match_config, fill_seed);

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: Some(dec!(0.50)),
    };

    // M-DEV-3 (MN-spread): read k_short from the strategy — ZERO overhead when k_short==0.
    // Every short-side branch in run_path is gated on `k_short > 0` so the long-only
    // path is byte-for-byte HEAD's code (D-MN.3 layer 1 — the by-construction proof).
    let k_short = strategy.k_short();

    // M-DEV-6: inject the carry funding lookup into the strategy and keep a copy
    // for the per-bar cashflow accrual. `None` for momentum/MR → zero overhead,
    // the accrual block is never entered (anchor-neutral by construction).
    //
    // M-DEV-5 (basis-reversal, D-BR.1): the basis arm pre-injects its sidecar map
    // into the strategy via `strategy.with_funding(Some(map))` in the sweep driver
    // BEFORE calling `run_path`. To avoid overwriting that map here, we only call
    // `with_funding` when `funding_override` is `Some` (carry and basis-test paths).
    // When `funding_override` is `None` (momentum/MR/basis sweep), the strategy's
    // existing funding_map is preserved. For carry: the map is passed as
    // `funding_override` so BOTH score and accrual see it. For basis: the map is
    // pre-injected; `funding_override` stays `None` → no accrual (NO cashflow — D-BR.1).
    // ── #75 score/accrual channel separation (story 1-25) ────────────────────
    // `funding_override` is the ACCRUAL channel and NOTHING ELSE. It used to also
    // be pushed into the strategy here via `with_funding(Some(map))`, which
    // silently OVERWROTE any score map the caller had already injected.
    //
    // That is bug-log #75, and it had exactly one victim. The sweep driver
    // (`param_robustness_sweep.rs`) already injects the score map itself —
    // `.with_funding(final_strategy_score_map).with_basis_score(...)` — so for
    // `MnBasisSpread` the sequence was: driver injects BASIS for scoring, then
    // this line replaced it with the FUNDING accrual map. The market-neutral
    // basis arm therefore scored on funding and became a duplicate funding run
    // (anchors #108-#111). `MnFundingSpread` and `MnBasisFundingResidual` pass the
    // same map on both channels, so the clobber was a no-op there — which is why
    // only one arm was corrupted, and why the `mn-basisperp` control differed from
    // `mn-funding` in every number while `mn-basis` differed in none.
    //
    // The score channel now belongs to the CALLER, exclusively. A caller that
    // wants the accrual map to also drive scoring must say so, by injecting it:
    //     MomentumStrategy::from_config(..).with_funding(Some(map))
    // Making that explicit is the point: the two channels can no longer be
    // conflated by accident.
    let funding_map_for_accrual = funding_override;
    let mut strategy = strategy;

    let mut cash = initial_capital;
    let mut min_cash_seen = initial_capital;
    let mut position_book: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut mark_prices: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();
    let mut trades = 0usize;
    // M-DEV-6: running sum of funding cashflow accrued across all settlement
    // boundaries (stays ZERO when funding_override is None → momentum/MR unchanged).
    let mut realized_funding_total = Decimal::ZERO;
    // M-DEV-4: time-in-market counter — bars where ≥1 long position was held.
    // Pure observability; does NOT alter any equity/order/signal path. Always
    // computed (zero overhead) — ZERO for momentum/MR/carry (they typically have
    // positions but the field is used only when the TS sweep driver requests it).
    let mut time_in_market_bars = 0u64;
    // M-DEV-3: liquidation counter (ZERO for k_short==0 → dead code path → anchor-neutral).
    let mut liquidations = 0u64;
    let mut equity_curve: Vec<Decimal> = vec![initial_capital];
    let mut peak_equity = initial_capital;
    let mut max_drawdown_tracking = Decimal::ZERO;

    // ── Funding-accrual bookkeeping (2026-08-04 carry-surface fix) ───────────
    //
    // TWO defects were fixed here; both were measured, not inferred.
    //
    // (1) MULTIPLICITY. `merged_bars` interleaves EVERY symbol's series sorted
    //     by (ts, symbol), and the accrual block used to be gated only on the
    //     bar's timestamp — so at a settlement timestamp shared by N symbols the
    //     entire position book accrued N times. Measured directly by holding ONE
    //     position and varying only the universe size: totals came out −5 / −7 /
    //     −9 units for N = 2 / 3 / 4 (see
    //     `carry_divergence_e2e::funding_accrual_is_invariant_to_universe_size`).
    //     `last_accrual_ts` collapses a timestamp's symbol-bars to ONE accrual.
    //
    // (2) CADENCE. The settlement test counts BARS on a cosmetic 1-hour ladder
    //     that `BlockBootstrapPathGen` stamps regardless of the source cadence
    //     (bug-log #72), so a 4h path settled every 32 real hours and a daily
    //     path every 8 real days. The rule no longer infers cadence from a
    //     timestamp the generator invented: `bar_span_hours` is supplied by the
    //     caller (1 for native-hourly runs) and the number of 8h settlement
    //     boundaries inside each bar's span is counted explicitly.
    let bar_span_hours: i128 = i128::from(input.bar_span_hours.max(1));
    let mut last_accrual_ts: Option<trading_core::Timestamp> = None;
    let mut settled_boundaries: i128 = 0;

    // #67 fill-symbol correctness (story 1-25, seam ratified 2026-08-16).
    // `merged_bars` interleaves EVERY symbol sorted by (ts, symbol), so the `bar`
    // in hand belongs to ONE symbol while a signal may fire for another. The
    // sizing path below already resolves the right price per symbol via
    // `mark_prices.get(&sig.symbol)`; the FILL path did not — it handed the
    // current `bar` to `engine.step`, pricing the fill at a foreign symbol's
    // close. #67 was therefore a divergence between sizing and filling INSIDE
    // one block, not a missing concept.
    //
    // This index is the fill-side twin of `mark_prices`: same key, same update
    // point, same "most recent bar for this symbol" semantics — so a fill is now
    // priced at exactly the bar whose close the sizing used.
    let mut last_bar_by_symbol: std::collections::HashMap<trading_core::Symbol, Bar> =
        std::collections::HashMap::new();

    for bar in &merged_bars {
        mark_prices.insert(bar.symbol.clone(), bar.close.get());
        last_bar_by_symbol.insert(bar.symbol.clone(), bar.clone());

        let signals = strategy.on_bar(bar);

        for sig in &signals {
            let Some(&mark) = mark_prices.get(&sig.symbol) else {
                continue;
            };
            if mark <= Decimal::ZERO {
                continue;
            }

            let position_value: Decimal = position_book
                .iter()
                .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
                .sum();
            let equity = cash + position_value;
            if equity <= Decimal::ZERO {
                continue;
            }

            let current_qty = position_book
                .get(&sig.symbol)
                .copied()
                .unwrap_or(Decimal::ZERO);

            match sig.kind {
                // ── M-DEV-3: Buy-to-cover short (k_short > 0 ONLY — dead code when k_short==0) ─
                // Placed BEFORE the long-open arm so the more-specific guard wins.
                // current_qty < 0 means we hold a short; this Buy signal covers it.
                trading_core::SignalKind::Buy if current_qty < Decimal::ZERO && k_short > 0 => {
                    // Cover the entire short position at mark.
                    let cover_qty = (-current_qty).max(Decimal::ZERO);
                    if cover_qty <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(cover_qty)
                        && let Ok(price) = Price::new(mark)
                    {
                        // Review 1-21: the order-construction and engine-step Err arms used
                        // to be swallowed by an `if let Ok(..) && let Ok(..)` chain — a
                        // risk-limit REJECTION was indistinguishable from a no-fill, with no
                        // log and no counter. On the MN lane an unfilled cover leaves a short
                        // open that the book believes is closed → a directionally-biased
                        // book that no output records. The behaviour is UNCHANGED (both arms
                        // still skip); only the diagnosis is now loud. Trace-only by
                        // construction: a counter would need a new report COLUMN, which is
                        // anchor-impacting (D-MN.8 hashed body) — see the story's triage.
                        match Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Buy,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) {
                            Ok(ord) => match engine
                                .step(
                                    // #67: fill at THIS order's symbol bar, never the
                                    // merged-loop bar. Present by construction — the
                                    // `mark_prices.get(&sig.symbol)` guard above already
                                    // returned for this symbol, and both maps are written
                                    // at the same point in the loop.
                                    last_bar_by_symbol.get(&sig.symbol).unwrap_or(bar),
                                    vec![ord],
                                )
                                .await
                            {
                                Ok(fills) => {
                                    for fill in fills {
                                        let notional_fill = fill.qty.get() * fill.price.get();
                                        let total_cost = notional_fill + fill.fee.amount();
                                        if total_cost > cash {
                                            // Solvency guard: skip rather than go negative.
                                            // NOTE: abandoning a cover leaves the short OPEN
                                            // while the strategy's `held_short_symbols` says
                                            // it is closed — the book is now directionally
                                            // biased and nothing but this line records it.
                                            tracing::warn!(
                                                symbol = %sig.symbol,
                                                cash = %cash,
                                                total_cost = %total_cost,
                                                abandoned_qty = %fill.qty.get(),
                                                "short cover solvency guard triggered — the short \
                                                 stays OPEN while the strategy believes it is flat"
                                            );
                                            continue;
                                        }
                                        cash -= total_cost;
                                        if cash < min_cash_seen {
                                            min_cash_seen = cash;
                                        }
                                        *position_book
                                            .entry(sig.symbol.clone())
                                            .or_insert(Decimal::ZERO) += fill.qty.get();
                                        trades += 1;
                                    }
                                }
                                Err(err) => tracing::warn!(
                                    symbol = %sig.symbol,
                                    side = "Buy",
                                    intent = "cover_short",
                                    notional = %(cover_qty * mark),
                                    %equity,
                                    error = %err,
                                    "engine REJECTED the buy-to-cover order — the short stays \
                                     OPEN (silently, before review 1-21)"
                                ),
                            },
                            Err(err) => tracing::warn!(
                                symbol = %sig.symbol,
                                side = "Buy",
                                intent = "cover_short",
                                notional = %(cover_qty * mark),
                                %equity,
                                per_symbol_exposure_cap = %risk_limits.per_symbol_exposure_cap,
                                portfolio_exposure_cap = ?risk_limits.portfolio_exposure_cap,
                                price_sanity_band = %risk_limits.price_sanity_band,
                                error = %err,
                                "risk limits REJECTED the buy-to-cover order — the short stays \
                                 OPEN (silently, before review 1-21)"
                            ),
                        }
                    }
                }
                trading_core::SignalKind::Buy if current_qty <= Decimal::ZERO => {
                    let fraction = dec!(0.10);
                    // Bug B fix (v0.1.1): the buy is SKIPPED when cash cannot cover
                    // notional + estimated fee, so cash can never go negative. Before
                    // this fix, notional was sized against total equity (cash +
                    // positions) without checking whether cash was sufficient, driving
                    // cash negative on fee-churn paths (up to 5 343 trades/year) and
                    // producing impossible negative equity on a long-only book. Per the
                    // solvency invariant: cash ≥ 0 AND equity ≥ 0 at ALL steps. The
                    // strategy's 10%-of-equity intent is preserved when cash is
                    // sufficient. TWO guard layers protect solvency: (1) the pre-flight
                    // skip below; (2) the defensive fill-loop guard on total_cost.
                    //
                    // Review 1-14: a former "layer 1" notional cap
                    // (`min(target_notional, cash)`) was removed as dead code — with
                    // any positive taker fee a cash-capped buy always failed the
                    // pre-flight (`cash < cash + fee`), so no downsized buy could ever
                    // execute; skip-vs-skip is byte-identical. All anchored lanes and
                    // the harness drivers use taker_fee_bps = 4.
                    let notional = equity * fraction;
                    // Estimate round-trip fee (taker_fee_bps; conservative).
                    let fee_estimate = notional * Decimal::new(i64::from(taker_fee_bps), 4); // bps → fraction
                    // Pre-flight solvency check: skip the buy outright if cash cannot
                    // cover notional + estimated fee (no partial downsizing).
                    if cash < notional + fee_estimate || notional <= Decimal::ZERO {
                        continue;
                    }
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(qty_raw)
                        && let Ok(price) = Price::new(mark)
                    {
                        // Review 1-21: see the cover arm above — a rejected order used to be
                        // silent. Behaviour unchanged (skip), diagnosis now loud.
                        match Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Buy,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) {
                            Ok(ord) => match engine
                                .step(
                                    // #67: fill at THIS order's symbol bar, never the
                                    // merged-loop bar. Present by construction — the
                                    // `mark_prices.get(&sig.symbol)` guard above already
                                    // returned for this symbol, and both maps are written
                                    // at the same point in the loop.
                                    last_bar_by_symbol.get(&sig.symbol).unwrap_or(bar),
                                    vec![ord],
                                )
                                .await
                            {
                                Ok(fills) => {
                                    for fill in fills {
                                        let notional_fill = fill.qty.get() * fill.price.get();
                                        let total_cost = notional_fill + fill.fee.amount();
                                        // Solvency guard (defensive): if somehow fill cost exceeds
                                        // cash (edge case from price movement between estimate and fill),
                                        // skip updating rather than going negative.
                                        if total_cost > cash {
                                            tracing::warn!(
                                                symbol = %sig.symbol,
                                                cash = %cash,
                                                total_cost = %total_cost,
                                                "solvency guard triggered — skipping fill to prevent negative cash"
                                            );
                                            continue;
                                        }
                                        cash -= total_cost;
                                        if cash < min_cash_seen {
                                            min_cash_seen = cash;
                                        }
                                        *position_book
                                            .entry(sig.symbol.clone())
                                            .or_insert(Decimal::ZERO) += fill.qty.get();
                                        trades += 1;
                                    }
                                }
                                Err(err) => tracing::warn!(
                                    symbol = %sig.symbol,
                                    side = "Buy",
                                    intent = "open_long",
                                    %notional,
                                    %equity,
                                    error = %err,
                                    "engine REJECTED the long-open order — the leg is absent from \
                                     the book (silently, before review 1-21)"
                                ),
                            },
                            Err(err) => tracing::warn!(
                                symbol = %sig.symbol,
                                side = "Buy",
                                intent = "open_long",
                                %notional,
                                %equity,
                                per_symbol_exposure_cap = %risk_limits.per_symbol_exposure_cap,
                                portfolio_exposure_cap = ?risk_limits.portfolio_exposure_cap,
                                price_sanity_band = %risk_limits.price_sanity_band,
                                error = %err,
                                "risk limits REJECTED the long-open order — the leg is absent from \
                                 the book (silently, before review 1-21)"
                            ),
                        }
                    }
                }
                trading_core::SignalKind::Sell if current_qty > Decimal::ZERO => {
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(current_qty)
                        && let Ok(price) = Price::new(mark)
                    {
                        // Review 1-21: see the cover arm above — a rejected order used to be
                        // silent. Behaviour unchanged (skip), diagnosis now loud. A rejected
                        // CLOSE is the worst of the four: the position stays on while the
                        // strategy's `held_symbols` records it as flat.
                        match Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Sell,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) {
                            Ok(ord) => match engine
                                .step(
                                    // #67: fill at THIS order's symbol bar, never the
                                    // merged-loop bar. Present by construction — the
                                    // `mark_prices.get(&sig.symbol)` guard above already
                                    // returned for this symbol, and both maps are written
                                    // at the same point in the loop.
                                    last_bar_by_symbol.get(&sig.symbol).unwrap_or(bar),
                                    vec![ord],
                                )
                                .await
                            {
                                Ok(fills) => {
                                    for fill in fills {
                                        let notional_fill = fill.qty.get() * fill.price.get();
                                        cash += notional_fill - fill.fee.amount();
                                        *position_book
                                            .entry(sig.symbol.clone())
                                            .or_insert(Decimal::ZERO) -= fill.qty.get();
                                        trades += 1;
                                    }
                                }
                                Err(err) => tracing::warn!(
                                    symbol = %sig.symbol,
                                    side = "Sell",
                                    intent = "close_long",
                                    notional = %(current_qty * mark),
                                    %equity,
                                    error = %err,
                                    "engine REJECTED the long-close order — the position stays \
                                     OPEN while the strategy believes it is flat (silently, \
                                     before review 1-21)"
                                ),
                            },
                            Err(err) => tracing::warn!(
                                symbol = %sig.symbol,
                                side = "Sell",
                                intent = "close_long",
                                notional = %(current_qty * mark),
                                %equity,
                                per_symbol_exposure_cap = %risk_limits.per_symbol_exposure_cap,
                                portfolio_exposure_cap = ?risk_limits.portfolio_exposure_cap,
                                price_sanity_band = %risk_limits.price_sanity_band,
                                error = %err,
                                "risk limits REJECTED the long-close order — the position stays \
                                 OPEN while the strategy believes it is flat (silently, before \
                                 review 1-21)"
                            ),
                        }
                    }
                }
                // ── M-DEV-3: Open/extend short (k_short > 0 ONLY — dead code when k_short==0) ─
                // `current_qty <= 0` means flat or already short; this Sell signal opens/extends.
                // The initial-margin gate mirrors the long Bug-B skip (D-MN.2 solvency point 2).
                trading_core::SignalKind::Sell if current_qty <= Decimal::ZERO && k_short > 0 => {
                    let fraction = dec!(0.10);
                    let target_notional = equity * fraction;
                    // Reserve margin = notional / max_leverage (max_leverage=1 → margin=notional).
                    // The short is SKIPPED (not partially filled) if cash < margin + estimated fee
                    // — mirroring the long Bug-B pre-flight skip (D-MN.2 initial-margin gate).
                    // NOTE (review 1-14): this branch deliberately KEEPS its notional cap —
                    // it is part of the LOCKED MN anchor surface (D-MN.2) and was not in the
                    // 1-14 dead-cap finding's scope (which covered the long Buy branch only).
                    let notional = if target_notional > cash {
                        cash
                    } else {
                        target_notional
                    };
                    let margin = notional / MAX_LEVERAGE;
                    let fee_estimate = notional * Decimal::new(i64::from(taker_fee_bps), 4);
                    if cash < margin + fee_estimate || notional <= Decimal::ZERO {
                        continue;
                    }
                    let qty_raw = notional / mark;
                    if qty_raw <= Decimal::ZERO {
                        continue;
                    }
                    let pos_snap = Position::empty(sig.symbol.clone());
                    if let Ok(qty) = Quantity::new(qty_raw)
                        && let Ok(price) = Price::new(mark)
                    {
                        // Review 1-21: see the cover arm above — a rejected order used to be
                        // silent. Behaviour unchanged (skip), diagnosis now loud. This is the
                        // arm the MN finding is about: a rejected SHORT-open leaves the long
                        // book standing alone, i.e. a directionally-biased "market-neutral"
                        // run that no column of the θ-surface would reveal.
                        match Order::new(
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            Side::Sell,
                            qty,
                            OrderKind::Market,
                            TimeInForce::Ioc,
                            &pos_snap,
                            price,
                            &risk_limits,
                            equity,
                        ) {
                            Ok(ord) => match engine
                                .step(
                                    // #67: fill at THIS order's symbol bar, never the
                                    // merged-loop bar. Present by construction — the
                                    // `mark_prices.get(&sig.symbol)` guard above already
                                    // returned for this symbol, and both maps are written
                                    // at the same point in the loop.
                                    last_bar_by_symbol.get(&sig.symbol).unwrap_or(bar),
                                    vec![ord],
                                )
                                .await
                            {
                                Ok(fills) => {
                                    for fill in fills {
                                        let notional_fill = fill.qty.get() * fill.price.get();
                                        // Open short: sell proceeds in, qty goes negative.
                                        // cash += notional − fee (proceeds in, fee out).
                                        cash += notional_fill - fill.fee.amount();
                                        if cash < min_cash_seen {
                                            min_cash_seen = cash;
                                        }
                                        // qty goes NEGATIVE (short position).
                                        *position_book
                                            .entry(sig.symbol.clone())
                                            .or_insert(Decimal::ZERO) -= fill.qty.get();
                                        trades += 1;
                                    }
                                }
                                Err(err) => tracing::warn!(
                                    symbol = %sig.symbol,
                                    side = "Sell",
                                    intent = "open_short",
                                    %notional,
                                    %margin,
                                    %equity,
                                    error = %err,
                                    "engine REJECTED the short-open order — the MN book is now \
                                     LONG-BIASED for this rebalance (silently, before review 1-21)"
                                ),
                            },
                            Err(err) => tracing::warn!(
                                symbol = %sig.symbol,
                                side = "Sell",
                                intent = "open_short",
                                %notional,
                                %margin,
                                %equity,
                                per_symbol_exposure_cap = %risk_limits.per_symbol_exposure_cap,
                                portfolio_exposure_cap = ?risk_limits.portfolio_exposure_cap,
                                price_sanity_band = %risk_limits.price_sanity_band,
                                error = %err,
                                "risk limits REJECTED the short-open order — the MN book is now \
                                 LONG-BIASED for this rebalance (silently, before review 1-21)"
                            ),
                        }
                    }
                }
                _ => {}
            }
        }

        // M-DEV-6 — funding-cashflow accrual (R-CARRY.8 / D-CARRY.7).
        //
        // For each held LONG position at a funding-settlement boundary, accrue:
        //   cash += position_notional × (−funding_rate)
        //
        // Framing (a) long-only (D-CARRY.0): earns on negative-funding names (longs
        // are the paid side), pays on positive-funding names (longs pay shorts).
        // The leading minus is the R-CARRY.2 sign — it's the same sign as carry_score.
        //
        // Settlement boundary: on the synthetic hourly grid (epoch_2023 + k·hours),
        // a funding settlement occurs every 8h (bar k where k % 8 == 0).
        // We detect this via: (open_ts_ns − epoch_2023_ns) / HOUR_NS % 8 == 0.
        //
        // Gated on `funding_map_for_accrual` being `Some` → accrual block is never
        // entered for momentum/MR runs; the 87 existing anchors are byte-identical.
        if let Some(ref funding_map) = funding_map_for_accrual {
            // epoch_2023 = 2023-01-01 00:00:00 UTC in nanoseconds.
            const EPOCH_2023_NS: i128 = 1_672_531_200_000_000_000_i128;
            const HOUR_NS: i128 = 3_600_000_000_000_i128;
            let open_ns = bar.open_ts.inner().unix_timestamp_nanos();
            let ladder_hours = (open_ns - EPOCH_2023_NS) / HOUR_NS;
            // The ladder index counts BARS (the generator's cosmetic 1h stamps);
            // `bar_span_hours` converts it to the simulated market time this bar
            // actually represents.
            let elapsed_hours = ladder_hours * bar_span_hours;
            // Settlement boundaries strictly inside (previous bar, this bar],
            // plus the inclusive bar-0 boundary (D-CARRY.7 convention).
            let boundaries_through_now = if elapsed_hours < 0 {
                0
            } else {
                elapsed_hours / 8 + 1 // +1 for the boundary at hour 0 itself
            };
            let is_new_ts = last_accrual_ts != Some(bar.open_ts);
            let n_settlements = if is_new_ts {
                let n = boundaries_through_now - settled_boundaries;
                if n > 0 {
                    settled_boundaries = boundaries_through_now;
                }
                n.max(0)
            } else {
                0 // this timestamp already accrued — a sibling symbol's bar event
            };
            if is_new_ts {
                last_accrual_ts = Some(bar.open_ts);
            }
            if n_settlements > 0 {
                // For each held position (long OR short), look up the funding rate and accrue.
                // Iterate in sorted order (BTreeMap) for determinism.
                // M-DEV-3: the `qty <= 0 continue` skip is replaced by a branch that also
                // accrues for held shorts (qty < 0). The existing formula is ALREADY CORRECT
                // for shorts — see the four-case sign table on the `cashflow` line below.
                // Still gated on `funding_map_for_accrual` being Some → non-MN runs unchanged.
                //
                // COMMENT CORRECTION (review 1-21). Both comment blocks here used to say
                // that a short's cashflow "is negative when rate > 0 (short pays positive
                // funding)". That is arithmetically FALSE and it contradicted the other
                // block three lines away. The CODE was and is right; only the prose was
                // wrong. The hazard of leaving it was a future editor "fixing" the code to
                // match the comment and silently inverting every MN short-leg cashflow.
                // The convention is now ENFORCED, not described:
                // `montecarlo::tests::funding_accrual_four_sign_cases_pinned` pins all four
                // (side × rate-sign) cells against literal ±100 expectations through the
                // production `run_path`.
                for (sym, &qty) in &position_book {
                    if qty == Decimal::ZERO {
                        continue; // flat — no accrual
                    }
                    // M-DEV-3: shorts (qty < 0) are no longer skipped — they pay funding.
                    // When k_short == 0, qty < 0 is impossible (no short was ever opened),
                    // so this branch is dead code for long-only runs — anchor-neutral.
                    let Some(&rate) = funding_map.get(&(sym.clone(), bar.open_ts)) else {
                        continue; // no funding data for this (symbol, ts) — skip
                    };
                    let mark = mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO);
                    if mark <= Decimal::ZERO {
                        continue;
                    }
                    let notional = qty * mark;
                    // SIGN TABLE for `cashflow = notional × (−rate)` (review 1-21 — the
                    // prose that used to sit here was arithmetically false; the code is
                    // unchanged). `notional = qty × mark`, so its sign is the sign of qty:
                    //
                    //   | side          | qty | rate | notional | cashflow = n×(−r) | meaning        |
                    //   |---------------|-----|------|----------|-------------------|----------------|
                    //   | long          |  >0 |  >0  |    >0    | NEGATIVE          | long PAYS      |
                    //   | long          |  >0 |  <0  |    >0    | POSITIVE          | long RECEIVES  |
                    //   | short         |  <0 |  >0  |    <0    | POSITIVE          | short RECEIVES |
                    //   | short         |  <0 |  <0  |    <0    | NEGATIVE          | short PAYS     |
                    //
                    // Row 3 is the one the old comment got backwards: (−|n|)·(−r) = +|n|·r.
                    // A short RECEIVING on positive funding IS the correct perp mechanic —
                    // longs pay shorts when funding is positive — and it matches
                    // `backtest::short_exec::accrue_funding`'s documented convention and its
                    // `funding_short_position_receives_positive_funding` unit test.
                    // R-CARRY.2 sign (long-only framing) is the same formula: the long earns
                    // on negative funding and pays on positive.
                    // Enforced end-to-end by `tests::funding_accrual_four_sign_cases_pinned`.
                    // One cashflow per settlement boundary inside this bar's
                    // span (1 at native hourly cadence; 3 for a daily bar).
                    // `n_settlements` is a small non-negative count (1 at hourly
                    // cadence, 3 for a daily bar); i64::try_from cannot fail for
                    // any realistic bar span, and a saturating fallback keeps the
                    // money path total rather than panicking.
                    let settlements_dec =
                        Decimal::from(i64::try_from(n_settlements.max(0)).unwrap_or(i64::MAX));
                    let cashflow = notional * (-rate) * settlements_dec;
                    cash += cashflow;
                    realized_funding_total += cashflow;
                    tracing::trace!(
                        symbol = %sym,
                        %rate,
                        %notional,
                        %cashflow,
                        "funding accrual"
                    );
                }
            }
        }

        // Update equity curve and drawdown after each bar.
        let position_value: Decimal = position_book
            .iter()
            .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
            .sum();
        let equity = cash + position_value;

        // M-DEV-3: MAINTENANCE-MARGIN LIQUIDATION (k_short > 0 ONLY — dead code when 0).
        // If equity falls below maintenance_margin_frac × gross_short_notional, force-close
        // ALL short legs at mark (deterministic BTreeMap order, no RNG). Increment liquidations.
        // Bounds the unbounded short loss exactly as a real exchange's liquidation engine does.
        // Gated on `k_short > 0` → inert for all long-only runs → anchor-neutral (D-MN.3).
        if k_short > 0 {
            let gross_short_notional: Decimal = position_book
                .iter()
                .filter(|(_, qty)| **qty < Decimal::ZERO)
                .map(|(sym, qty)| {
                    // notional of a short is positive (−qty × mark = positive)
                    (-*qty) * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO)
                })
                .sum();

            if gross_short_notional > Decimal::ZERO
                && equity < maintenance_margin_frac() * gross_short_notional
            {
                // Force-close all short positions at current mark (BTreeMap iteration order
                // is deterministic — alphabetical — no RNG draw needed).
                let short_syms: Vec<Symbol> = position_book
                    .iter()
                    .filter(|(_, qty)| **qty < Decimal::ZERO)
                    .map(|(sym, _)| sym.clone())
                    .collect();

                for sym in &short_syms {
                    let &short_qty = position_book.get(sym).unwrap_or(&Decimal::ZERO);
                    if short_qty >= Decimal::ZERO {
                        continue;
                    }
                    let cover_qty = -short_qty; // positive qty to buy-to-cover
                    let mark_price = mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO);
                    if mark_price <= Decimal::ZERO || cover_qty <= Decimal::ZERO {
                        continue;
                    }
                    // Buy-to-cover at mark: cash out = cover_qty × mark + fee.
                    let cover_notional = cover_qty * mark_price;
                    let cover_fee = cover_notional * Decimal::new(i64::from(taker_fee_bps), 4);
                    let total_cover_cost = cover_notional + cover_fee;
                    // Pay the cover cost from cash (may drive cash negative in extreme liquidation).
                    cash -= total_cover_cost;
                    if cash < min_cash_seen {
                        min_cash_seen = cash;
                    }
                    // Remove the short position.
                    position_book.remove(sym);
                    trades += 1;
                    tracing::warn!(
                        symbol = %sym,
                        %equity,
                        %gross_short_notional,
                        %cover_notional,
                        "maintenance-margin liquidation: force-covering short"
                    );
                }
                liquidations += 1;
            }
        }

        equity_curve.push(equity);
        // M-DEV-4: count bars with ≥1 long position (time-in-market, pure observability).
        if position_book.values().any(|&qty| qty > Decimal::ZERO) {
            time_in_market_bars += 1;
        }
        if equity > peak_equity {
            peak_equity = equity;
        }
        let dd = if peak_equity > Decimal::ZERO {
            (peak_equity - equity) / peak_equity
        } else {
            Decimal::ZERO
        };
        if dd > max_drawdown_tracking {
            max_drawdown_tracking = dd;
        }
    }

    let position_value: Decimal = position_book
        .iter()
        .map(|(sym, &qty)| qty * mark_prices.get(sym).copied().unwrap_or(Decimal::ZERO))
        .sum();
    let final_equity = cash + position_value;

    tracing::debug!(
        scenario = %scenario_name,
        trades,
        final_equity = %final_equity,
        bars = bar_count,
        "montecarlo::run_path complete"
    );

    Ok(PathRunResult {
        equity_curve,
        trades,
        initial_equity: initial_capital,
        final_equity,
        min_cash_seen,
        realized_funding: realized_funding_total,
        time_in_market_bars,
        liquidations,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::too_many_lines,
    clippy::doc_markdown
)]
mod tests {
    use super::*;

    /// `run_path` with `None` `bars_override` returns `Err` (programming error detection).
    #[test]
    fn run_path_requires_bars_override() {
        use rust_decimal_macros::dec;
        use std::path::PathBuf;

        let rel = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let toml_path = crate::paths::resolve_workspace_path(&rel);
        // If config doesn't exist in test environment, skip gracefully.
        let Ok(cfg) = strategy::CrossSectionalMomentumConfig::from_file(&toml_path) else {
            return; // skip: no config in unit-test context
        };
        let strat = strategy::MomentumStrategy::from_config(
            cfg,
            smol_str::SmolStr::new("top10_momentum_h1"),
        );
        let input = TcnScenarioInput {
            scenario_name: "test-no-bars".to_string(),
            start_year: 2023,
            bar_count: 100,
            initial_capital: dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: None, // intentionally None → should error
            emit_equity_bin: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            funding_override: None,
            bar_span_hours: 1,
        };
        let result = pollster::block_on(run_path(input, 0x00C0_FFEE, strat));
        assert!(
            result.is_err(),
            "run_path with None bars_override must return Err"
        );
    }

    /// R-CARRY.10b — funding cashflow non-no-op test (MANDATORY, day-1).
    ///
    /// CLAUDE.md v3-vol-overlay precedent: a computed-but-ignored cashflow is a silent no-op.
    ///
    /// This test forces the funding cashflow to zero (by using all-zero funding rates) and
    /// asserts that the equity curve WITH a non-zero carry cashflow DIVERGES from the
    /// zero-funding case. RED if the cashflow is computed-and-ignored (both curves would
    /// be identical regardless of the funding rates).
    ///
    /// Construction (SINGLE-SYMBOL ISOLATION — the confound fix):
    /// - 1 symbol: ETHUSDT (negative funding = longs earn), stable price.
    /// - Carry K=1, L=1: the single symbol is selected WITH and WITHOUT funding, so
    ///   the ONLY difference between the two runs is the cashflow (no alphabetical
    ///   tie-break confound from a 2nd symbol at a different price).
    /// - Bars hourly from `epoch_2023`; settlement boundaries at ts=0, 8, 16, ...
    /// - WITH carry: `funding_override` has non-zero rates → cashflow moves equity.
    /// - ZERO: `funding_override` all-zero → no cashflow.
    /// - Assertions: `equity_with` ≠ `equity_zero` by ≥ ε (non-no-op) AND
    ///   `equity_with` > `equity_zero` (longs EARN on the negative-funding name).
    ///
    /// The test FAILS (goes RED) if the cashflow is not applied to `cash`:
    ///   the two equity curves would be identical.
    #[test]
    fn r_carry10b_funding_cashflow_non_no_op() {
        use std::collections::BTreeMap;

        use rust_decimal_macros::dec;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

        // epoch_2023 = 2023-01-01 00:00:00 UTC
        let epoch_2023 =
            OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid timestamp");

        let make_ts = |hour: i64| Timestamp::new(epoch_2023 + time::Duration::hours(hour));

        let make_bar_at = |sym: &str, close: rust_decimal::Decimal, hour: i64| Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: make_ts(hour),
            close_ts: make_ts(hour),
            local_recv_ts: make_ts(hour),
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(100)).unwrap(),
            trade_count: 1,
        };

        // SINGLE-SYMBOL ISOLATION: one symbol ETHUSDT @3000, stable price. With one
        // symbol + K=1 the SAME symbol is selected with AND without funding, so the ONLY
        // difference between the two runs is the funding cashflow — a clean non-no-op
        // test, free of the 2-symbol alphabetical-tie-break confound.
        let n_hours = 24_i64;
        let symbols = ["ETHUSDT"];
        let prices = [dec!(3_000)];
        let mut bars: Vec<Bar> = Vec::new();
        for hour in 0..n_hours {
            for (&sym, &price) in symbols.iter().zip(prices.iter()) {
                bars.push(make_bar_at(sym, price, hour));
            }
        }
        // Sort by (open_ts ASC, symbol ASC) — the merge order.
        bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

        // Funding map: negative funding for the single symbol ETHUSDT.
        // Injected at EVERY bar ts (the strategy reads it per bar; large rate so cashflow
        // is measurable: −1% per settlement = −0.01).
        let eth_sym = Symbol::new("ETHUSDT");
        let eth_rate = dec!(-0.01); // negative: longs EARN 1% per settlement

        let mut funding_nonzero: BTreeMap<(Symbol, Timestamp), rust_decimal::Decimal> =
            BTreeMap::new();
        let mut funding_zero: BTreeMap<(Symbol, Timestamp), rust_decimal::Decimal> =
            BTreeMap::new();
        for hour in 0..n_hours {
            let ts = make_ts(hour);
            funding_nonzero.insert((eth_sym.clone(), ts), eth_rate);
            funding_zero.insert((eth_sym.clone(), ts), rust_decimal::Decimal::ZERO);
        }

        // Build carry strategy: K=1, L=1 settlement lookback, rebalance=1 bar.
        // #75 (story 1-25): `run_path` no longer injects `funding_override` into the
        // strategy — that channel is accrual-only now. This arm SCORES on funding
        // (`score_source = funding_carry`), so it must receive the map explicitly.
        // Passing it here reproduces the previous behaviour exactly; the difference
        // is that the score channel is now stated rather than inherited.
        let make_carry_strat = |funding_score: Option<
            std::collections::BTreeMap<(Symbol, trading_core::Timestamp), Decimal>,
        >| {
            let toml = r#"
id = "carry_test"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["ETHUSDT"]
lookback_minutes = 1
rebalance_minutes = 1
k_long = 1
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
score_source = "funding_carry"
"#;
            let mut cfg =
                strategy::CrossSectionalMomentumConfig::from_str(toml).expect("valid carry config");
            cfg.score_source = strategy::ScoreSource::FundingCarry;
            strategy::MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("carry_test"))
                .with_funding(funding_score)
        };

        let initial_capital = dec!(100_000);

        // Run WITH non-zero funding.
        let input_with = TcnScenarioInput {
            scenario_name: "r_carry10b_with_funding".to_string(),
            start_year: 2023,
            bar_count: bars.len(),
            initial_capital,
            slippage_bps: 0, // zero friction so cashflow is the only difference
            taker_fee_bps: 0,
            config_id: "carry_test".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: Some(bars.clone()),
            emit_equity_bin: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            funding_override: Some(funding_nonzero.clone()),
            bar_span_hours: 1,
        };
        let result_with = pollster::block_on(run_path(
            input_with,
            0x00C0_FFEE,
            make_carry_strat(Some(funding_nonzero)),
        ))
        .expect("run_path with funding must succeed");

        // Run WITH zero-rate funding (cashflow forced to zero).
        let input_zero = TcnScenarioInput {
            scenario_name: "r_carry10b_zero_funding".to_string(),
            start_year: 2023,
            bar_count: bars.len(),
            initial_capital,
            slippage_bps: 0,
            taker_fee_bps: 0,
            config_id: "carry_test".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: Some(bars),
            emit_equity_bin: None,
            latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
            funding_override: Some(funding_zero.clone()),
            bar_span_hours: 1,
        };
        let result_zero = pollster::block_on(run_path(
            input_zero,
            0x00C0_FFEE,
            make_carry_strat(Some(funding_zero)),
        ))
        .expect("run_path with zero funding must succeed");

        // The WITH-carry equity must differ from the zero-cashflow equity.
        // If the cashflow is computed-and-ignored (no-op), both final equities are identical.
        let equity_with = result_with.final_equity;
        let equity_zero = result_zero.final_equity;
        let diff = (equity_with - equity_zero).abs();

        // ε: any measurable difference. With −1% rate per settlement × ~3 settlements in 24h
        // × position_notional ≈ 100_000 × 0.10 = 10_000 notional:
        // expected cashflow ≈ 3 × 10_000 × 0.01 = 300. We gate on > 1 (1 basis point).
        let epsilon = dec!(1);
        assert!(
            diff > epsilon,
            "R-CARRY.10b NON-NO-OP VIOLATION: funding cashflow must measurably move equity. \
             equity_with={equity_with}, equity_zero={equity_zero}, diff={diff}. \
             If diff ≈ 0, the cashflow is computed-and-ignored (the v3-vol-overlay no-op pattern). \
             The accrual `cash += notional × (−rate)` must be applied inside run_path."
        );

        // Also assert equity_with > equity_zero: ETHUSDT (negative funding) earns cashflow
        // → the WITH-carry equity should be higher than the zero-cashflow equity.
        assert!(
            equity_with > equity_zero,
            "R-CARRY.10b: WITH negative-funding carry, equity_with ({equity_with}) should be \
             GREATER than equity_zero ({equity_zero}) — longs earn on negative-funding names."
        );
    }

    /// Review 1-21 LOW: the maintenance-margin fraction has exactly ONE definition.
    ///
    /// It used to have two — a `const MAINTENANCE_MARGIN_FRAC` read by
    /// `sweep_harness::mn_grid_def_string` (so it reached the HASHED anchor body) and a
    /// `maintenance_margin_frac()` returning its own `dec!(0.5)` read by the liquidation
    /// rule in `run_path`. Both said 0.5, so nothing was wrong — but editing either one
    /// alone would have made every MN anchored body claim a margin the engine did not
    /// use, with no test anywhere to notice. The function is now an accessor over the
    /// constant; this test pins BOTH legs of the contract:
    ///
    /// 1. the engine's value equals the rendered value, and
    /// 2. the rendered literal is still exactly `0.5` — i.e. the unification did not
    ///    change one byte of the hashed body (`Decimal` renders by scale, so a value
    ///    equality alone would not have caught a `0.50` vs `0.5` render change).
    #[test]
    fn maintenance_margin_frac_has_exactly_one_definition() {
        assert_eq!(
            maintenance_margin_frac(),
            MAINTENANCE_MARGIN_FRAC,
            "the liquidation rule and the hashed grid-def body must read the SAME \
             maintenance-margin fraction — two definitions is how a surface starts \
             claiming a margin the engine never used"
        );
        assert_eq!(
            format!("{MAINTENANCE_MARGIN_FRAC}"),
            "0.5",
            "the MN anchored bodies (#108-#119) render `maintenance_margin_frac=0.5` \
             literally; any change to the rendered form re-prices every MN anchor"
        );
        assert_eq!(
            format!("{}", maintenance_margin_frac()),
            "0.5",
            "the accessor must render identically to the constant (scale-preserving)"
        );
        // The unification replaced a `dec!(0.5)` body with the constant. Pin that the
        // two are identical in REPRESENTATION, not merely in value: same mantissa, same
        // scale. (A value-only equality would pass for `0.50` too, and Decimal's Display
        // is scale-driven — that is how a hashed body row changes without the number
        // changing.)
        let old_literal = rust_decimal_macros::dec!(0.5);
        assert_eq!(maintenance_margin_frac(), old_literal);
        assert_eq!(
            maintenance_margin_frac().scale(),
            old_literal.scale(),
            "the constant must carry the SAME scale as the `dec!(0.5)` it replaced, or \
             the rendered `maintenance_margin_frac=` row in every MN body changes"
        );
        assert_eq!(
            maintenance_margin_frac().mantissa(),
            old_literal.mantissa(),
            "same mantissa as the literal it replaced"
        );
    }

    /// Review 1-21 MEDIUM: the funding-accrual SIGN CONVENTION, enforced not described.
    ///
    /// Two comment blocks in the accrual loop used to state the short case BACKWARDS
    /// ("notional × (−rate) is negative when rate > 0 — short pays positive funding")
    /// while the code did the arithmetically correct — and conventionally correct —
    /// thing: with `qty < 0, rate > 0`, `(−|n|)·(−r) = +|n|·r > 0`, so the SHORT
    /// **receives**. Longs pay shorts on positive funding; that is the perp mechanic and
    /// it matches `short_exec::accrue_funding`. The comments were fixed; this test exists
    /// so the convention can never again be carried only by prose — a future editor who
    /// "corrects" the code to match a wrong comment goes RED here with literal numbers.
    ///
    /// # Construction (exact by design — no epsilon anywhere)
    ///
    /// - Two symbols, prices `AAUSDT: 800 → 900 → 1000` then FLAT at 1000, and
    ///   `BBUSDT: 1250 → 1100 → 1000` then FLAT at 1000. The early move exists only to
    ///   give `VolAdjustedReturn` a deterministic ranking (AA rising ⇒ long, BB falling
    ///   ⇒ short); the flat tail makes every notional exact.
    /// - Both symbols CONVERGE to the same 1000 before the first rebalance, deliberately:
    ///   `run_path` hands every order to `engine.step(bar, …)` with the CURRENT bar, so a
    ///   fill is priced at whatever symbol's bar is being processed (bug-log **#67**,
    ///   owned by 1-25). With equal marks that mispricing is a no-op, and this sign test
    ///   measures the accrual instead of measuring #67. A first draft of this fixture
    ///   used 1000/500 and came out at 105 000 — #67, caught by the control assertion.
    /// - `selection_mode = LongShort`, `k_long = k_short = 1`, `rebalance_minutes` huge
    ///   ⇒ exactly ONE rebalance (at warm-up completion, hour 2), so the book is fixed
    ///   for the rest of the run and cannot be perturbed by a score tie later.
    /// - Zero fees, zero slippage ⇒ the long books 10% of 100 000 = **+10 000** notional
    ///   (10 units @ 1000) and the short books **−10 000** (20 units @ 500), both exact.
    /// - 9 hours ⇒ the only settlement boundary with a live book is hour 8, so exactly
    ///   ONE accrual event: `realized_funding = 10 000·(−r_AA) + (−10 000)·(−r_BB)`.
    /// - The score source is `VolAdjustedReturn`, which never reads the funding map, so
    ///   the four cases vary the RATES without moving the selection. (That also keeps the
    ///   test independent of bug-log #75: the map is used only for accrual here.)
    ///
    /// # The four pinned cells (rate = ±0.01, notional = ±10 000 ⇒ ±100 exactly)
    ///
    /// | case | long leg rate | short leg rate | expected `realized_funding` | meaning        |
    /// |------|---------------|----------------|-----------------------------|----------------|
    /// | 1    | +0.01         | 0              | **−100**                    | long PAYS      |
    /// | 2    | −0.01         | 0              | **+100**                    | long RECEIVES  |
    /// | 3    | 0             | +0.01          | **+100**                    | short RECEIVES |
    /// | 4    | 0             | −0.01          | **−100**                    | short PAYS     |
    ///
    /// Cases 3 and 4 also go RED if the short-side branch is removed from `run_path`
    /// (no short ⇒ no short notional ⇒ `realized_funding == 0`).
    #[test]
    fn funding_accrual_four_sign_cases_pinned() {
        use std::collections::BTreeMap;

        use rust_decimal_macros::dec;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

        let epoch = OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("epoch_2023");
        let make_ts = |h: i64| Timestamp::new(epoch + time::Duration::hours(h));
        let make_bar = |sym: &str, close: Decimal, hour: i64| Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: make_ts(hour),
            close_ts: make_ts(hour),
            local_recv_ts: make_ts(hour),
            venue: Venue::Binance,
            open: Price::new(close).expect("price"),
            high: Price::new(close).expect("price"),
            low: Price::new(close).expect("price"),
            close: Price::new(close).expect("price"),
            volume: Quantity::new(dec!(100)).expect("qty"),
            trade_count: 1,
        };

        // Hours 0..=8. AA rises into a round 1000 and stays; BB falls into the SAME round
        // 1000 and stays. Equal, round marks ⇒ integral quantities, exact notionals, and
        // no exposure to the #67 cross-symbol fill mispricing.
        let price_a = [
            dec!(800),
            dec!(900),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
        ];
        let price_b = [
            dec!(1250),
            dec!(1100),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
            dec!(1000),
        ];
        // 9 hourly bars per symbol, indices carried as i64 without a lossy cast.
        let n_hours = i64::try_from(price_a.len()).expect("fixture length fits i64");
        let mut bars: Vec<Bar> = Vec::new();
        for (idx, (&pa, &pb)) in price_a.iter().zip(price_b.iter()).enumerate() {
            let hour = i64::try_from(idx).expect("fixture index fits i64");
            bars.push(make_bar("AAUSDT", pa, hour));
            bars.push(make_bar("BBUSDT", pb, hour));
        }
        bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

        let make_strat = || {
            let toml = r#"
id = "sign_table_test"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["AAUSDT", "BBUSDT"]
lookback_minutes = 2
rebalance_minutes = 100000
k_long = 1
k_short = 1
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
selection_mode = "long_short"
"#;
            let cfg = strategy::CrossSectionalMomentumConfig::from_str(toml)
                .expect("valid long-short config");
            assert_eq!(cfg.k_short, 1, "the sign table needs a live short leg");
            strategy::MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("sign_table_test"))
        };

        // (rate_AA = long leg, rate_BB = short leg) → expected realized_funding.
        let run_case = |rate_a: Decimal, rate_b: Decimal| -> PathRunResult {
            let mut map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
            for h in 0..n_hours {
                map.insert((Symbol::new("AAUSDT"), make_ts(h)), rate_a);
                map.insert((Symbol::new("BBUSDT"), make_ts(h)), rate_b);
            }
            let input = TcnScenarioInput {
                scenario_name: "funding-sign-table".to_string(),
                start_year: 2023,
                bar_count: bars.len(),
                initial_capital: dec!(100_000),
                slippage_bps: 0,
                taker_fee_bps: 0,
                config_id: "sign_table_test".to_string(),
                forecaster_id: "test".to_string(),
                bars_override: Some(bars.clone()),
                emit_equity_bin: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                funding_override: Some(map),
                bar_span_hours: 1,
            };
            pollster::block_on(run_path(input, 0x00C0_FFEE, make_strat()))
                .expect("run_path must succeed for the sign table")
        };

        // Control: with BOTH rates zero the book is unchanged and nothing accrues. This
        // pins the fixture itself (2 fills = 1 long + 1 short) so a later case that reads
        // 0 cannot be mistaken for "the fixture never traded".
        let control = run_case(Decimal::ZERO, Decimal::ZERO);
        assert_eq!(
            control.trades, 2,
            "fixture check: exactly one long open + one short open must fill \
             (trades={}); without both legs the sign table proves nothing",
            control.trades
        );
        assert_eq!(
            control.realized_funding,
            Decimal::ZERO,
            "zero rates must accrue exactly zero (the documented negative control)"
        );
        assert_eq!(
            control.final_equity,
            dec!(100_000),
            "flat prices + zero rates ⇒ equity is untouched; if this moved, the fixture \
             is not the exact ±10 000 book the sign table's literals assume"
        );

        // Case 1 — LONG × POSITIVE rate ⇒ the long PAYS.
        let c1 = run_case(dec!(0.01), Decimal::ZERO);
        assert_eq!(
            c1.realized_funding,
            dec!(-100),
            "LONG × rate>0 must PAY: +10 000 notional × (−0.01) = −100, got {}",
            c1.realized_funding
        );
        assert_eq!(c1.final_equity, dec!(99_900), "equity = 100 000 − 100");

        // Case 2 — LONG × NEGATIVE rate ⇒ the long RECEIVES.
        let c2 = run_case(dec!(-0.01), Decimal::ZERO);
        assert_eq!(
            c2.realized_funding,
            dec!(100),
            "LONG × rate<0 must RECEIVE: +10 000 × (+0.01) = +100, got {}",
            c2.realized_funding
        );
        assert_eq!(c2.final_equity, dec!(100_100), "equity = 100 000 + 100");

        // Case 3 — SHORT × POSITIVE rate ⇒ the short RECEIVES.
        // THIS is the cell the old comments described backwards.
        let c3 = run_case(Decimal::ZERO, dec!(0.01));
        assert_eq!(
            c3.realized_funding,
            dec!(100),
            "SHORT × rate>0 must RECEIVE: (−10 000) × (−0.01) = +100 — longs pay shorts \
             on positive funding (the perp mechanic; see short_exec::accrue_funding). \
             got {}. If this reads −100, someone 'fixed' the code to match the old \
             wrong comment and every MN short-leg cashflow just inverted.",
            c3.realized_funding
        );
        assert_eq!(c3.final_equity, dec!(100_100), "equity = 100 000 + 100");

        // Case 4 — SHORT × NEGATIVE rate ⇒ the short PAYS.
        let c4 = run_case(Decimal::ZERO, dec!(-0.01));
        assert_eq!(
            c4.realized_funding,
            dec!(-100),
            "SHORT × rate<0 must PAY: (−10 000) × (+0.01) = −100, got {}",
            c4.realized_funding
        );
        assert_eq!(c4.final_equity, dec!(99_900), "equity = 100 000 − 100");
    }

    /// Anchor-neutrality: `run_path` with `funding_override=None` produces identical equity
    /// to the pre-carry code (the accrual block is never entered).
    #[test]
    fn run_path_funding_none_is_anchor_neutral() {
        use rust_decimal_macros::dec;
        use std::path::PathBuf;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

        let rel = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let toml_path = crate::paths::resolve_workspace_path(&rel);
        let Ok(cfg) = strategy::CrossSectionalMomentumConfig::from_file(&toml_path) else {
            return; // skip: no config in unit-test context
        };

        let epoch = OffsetDateTime::from_unix_timestamp(1_672_531_200).unwrap();
        let make_ts = |h: i64| Timestamp::new(epoch + time::Duration::hours(h));
        let make_bar = |sym: &str, close: rust_decimal::Decimal, hour: i64| Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: make_ts(hour),
            close_ts: make_ts(hour),
            local_recv_ts: make_ts(hour),
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
        };

        let syms = [
            "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT",
            "AVAXUSDT", "DOTUSDT", "LINKUSDT",
        ];
        let mut bars: Vec<Bar> = Vec::new();
        for hour in 0..8_i64 {
            for sym in &syms {
                bars.push(make_bar(sym, dec!(1000), hour));
            }
        }
        bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

        let make_strat = || {
            strategy::MomentumStrategy::from_config(
                cfg.clone(),
                smol_str::SmolStr::new("top10_momentum_h1"),
            )
        };

        let run = |funding_override| {
            let input = TcnScenarioInput {
                scenario_name: "anchor_neutrality".to_string(),
                start_year: 2023,
                bar_count: bars.len(),
                initial_capital: dec!(100_000),
                slippage_bps: 2,
                taker_fee_bps: 4,
                config_id: "top10_momentum_h1".to_string(),
                forecaster_id: "passthrough".to_string(),
                bars_override: Some(bars.clone()),
                emit_equity_bin: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                funding_override,
                bar_span_hours: 1,
            };
            pollster::block_on(run_path(input, 0x00C0_FFEE, make_strat())).expect("run_path ok")
        };

        let r_none = run(None);
        // With funding_override=None, the accrual block is never entered → identical.
        // We run twice to confirm determinism, and assert equity is same as None.
        let r_none2 = run(None);
        assert_eq!(
            r_none.final_equity, r_none2.final_equity,
            "anchor-neutrality: two runs with funding_override=None must produce identical equity"
        );
    }

    /// Long-only (`k_short == 0`) REGRESSION PIN + determinism check.
    ///
    /// # What this test is, and what it is NOT (review 1-21 — renamed and re-documented)
    ///
    /// It was called `run_path_k_short_zero_byte_identical_to_head` and its doc claimed
    /// it "goes RED the instant any short statement leaks out of its `k_short > 0` gate",
    /// calling itself "the formal proof of the by-construction neutrality claim". It was
    /// neither. It ran ONE configuration twice and asserted the two runs matched each
    /// other — a **determinism** check, which a gate leak passes trivially because the
    /// leak corrupts both runs identically. Its `liquidations == 0` assertion was
    /// unfalsifiable for the same reason plus one more: with no shorts,
    /// `gross_short_notional == 0` short-circuits the liquidation rule before the counter
    /// can move, so the assertion holds whatever the gate does.
    ///
    /// Worse, **no** `k_short == 0` unit test can catch a short-gate leak in general:
    /// `MomentumStrategy::build_rebalance_signals` gates the short book on
    /// `selection_mode == LongShort && k_short > 0`, so at `k_short == 0` no
    /// `open_short` signal is ever emitted and the short arms in `run_path` are
    /// unreachable no matter what their own guards say. A guard on unreachable code
    /// cannot be observed from the outside.
    ///
    /// So this test now makes the honest, falsifiable claim it CAN support:
    ///
    /// 1. **Golden pin** — the long-only equity curve for a fixed fixture is checked in
    ///    literally. ANY edit to `run_path` that moves the long-only path by one digit —
    ///    including a short-side rule that starts acting on the long book, which is the
    ///    dangerous leak — turns this RED with a readable diff.
    /// 2. **Determinism** — two runs of the same configuration match.
    /// 3. `liquidations == 0` is kept but labelled: it is IMPLIED by "no shorts", not
    ///    evidence about the gate.
    ///
    /// **The real neutrality proof is `bash scripts/verify_anchors.sh` (the 107 pre-MN
    /// anchors byte-identical after the MN work landed), plus the by-construction
    /// argument in D-MN.3 layer 1.** This unit test is a fast local tripwire under it,
    /// not a substitute for it.
    ///
    /// # Measured, not argued (review 1-21, two temporary mutations, both reverted)
    ///
    /// | mutation | this test |
    /// |---|---|
    /// | short-open arm UNGATED (`&& k_short > 0` deleted) — the exact leak the old doc claimed to catch | **GREEN** — no `open_short` signal exists at `k_short == 0`, so the arm is unreachable either way |
    /// | long-open `fraction` 0.10 → 0.11 — a change to the long-only path | **RED**, with a readable curve diff |
    ///
    /// That is the whole finding in two rows: the assertion the test used to make was
    /// unfalsifiable, and the assertion it makes now is falsifiable.
    ///
    /// See: `run_path_funding_none_is_anchor_neutral` (the funding analogue, which has
    /// exactly the same shape and the same limits).
    #[test]
    fn run_path_k_short_zero_long_only_equity_curve_is_pinned() {
        // ── (1) THE GOLDEN PIN — the falsifiable half of this test ────────────
        //
        // Captured from this exact fixture at the story-1-21 review commit, on the
        // canonical box (ADR-0051 D5). It is 24 bars + the initial-capital seed = 25
        // points, rendered with `Decimal::to_string` so scale changes are visible too
        // (`101000.00` and `101000` are DIFFERENT strings and both are meaningful:
        // Decimal scale tracks the arithmetic that produced the number).
        //
        // Re-baselining this literal is legitimate ONLY together with an explanation of
        // what changed in the long-only path — the same contract as `evidence/anchors.toml`,
        // at unit-test scale and unit-test speed.
        // ── RE-BASELINED 2026-08-16 (story 1-25, bug-log #67) ────────────────
        //
        // PREVIOUS value (contaminated — kept here as the evidence, not as history
        // trivia): the curve rose to `101000.00` at index 6 and `101918.18…` at
        // index 15. Those gains were FABRICATED BY #67 and the arithmetic is exact:
        //
        //   fixture from hour 6: AAAUSDT = 1000, BBBUSDT = 1100, bars sorted
        //   (ts, symbol) so AAA is processed first; sizing is 10% => 10 000 notional.
        //
        //     BUGGY  fill at AAA close 1000 -> 10.0000 units, marked at 1100 = 11 000
        //            equity = 100 000 - 10 000 + 11 000 = 101 000   <- the old pin
        //     FIXED  fill at BBB close 1100 ->  9.0909 units, marked at 1100 = 10 000
        //            equity = 100 000                              <- this pin
        //
        // Both reproduce to the digit, so the old literal was not "a slightly different
        // number" — it was the strategy booking an instant ~1% gain for buying one
        // symbol at a different symbol's price. Correct arithmetic yields a FLAT curve
        // because this fixture's price does not move after entry.
        //
        // What changed in the long-only path: `run_path` now fills every order at its
        // OWN symbol's most recent bar (`last_bar_by_symbol`), the fill-side twin of
        // the `mark_prices` lookup the SIZING path already used. #67 was a divergence
        // between sizing and filling inside one block.
        //
        // ⚠️ The anchored surfaces (#86..#107) still carry the CONTAMINATED numbers —
        // they are byte-frozen report bodies and are regenerated only at the re-lock
        // (AC4). This pin moving is therefore EXPECTED to disagree with the anchors
        // until then; that disagreement is the defect, not a regression.
        const GOLDEN_LONG_ONLY_EQUITY_CURVE: &str = "100000,100000,100000,100000,100000,100000,100000.00,100000.00,100000.00,100000.00,100000.00,100000.00,100000.00,100000.00,100000.00,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000,100000.00000000000000000000000";

        use rust_decimal_macros::dec;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

        // epoch_2023 = 2023-01-01 00:00:00 UTC
        let epoch = OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid timestamp");
        let make_ts = |h: i64| Timestamp::new(epoch + time::Duration::hours(h));

        let make_bar = |sym: &str, close: rust_decimal::Decimal, hour: i64| Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: make_ts(hour),
            close_ts: make_ts(hour),
            local_recv_ts: make_ts(hour),
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(100)).unwrap(),
            trade_count: 1,
        };

        // Build a synthetic 2-symbol bar series with enough variation to trigger
        // both long opens and closes (exercises the full strategy path).
        let syms = ["AAAUSDT", "BBBUSDT"];
        let prices_a = [dec!(1000), dec!(900)]; // A higher → selected first
        let prices_b = [dec!(1000), dec!(1100)]; // B higher at hour 4 → swap

        let n_hours = 12_i64;
        let mut bars: Vec<Bar> = Vec::new();
        for hour in 0..n_hours {
            let prices = if hour < 6 { &prices_a } else { &prices_b };
            for (sym, &price) in syms.iter().zip(prices.iter()) {
                bars.push(make_bar(sym, price, hour));
            }
        }
        bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));

        // Build a k_short=0 strategy (the standard long-only v1 config).
        // This is the critical part: k_short is explicitly 0 → all short branches dead.
        let make_long_only_strat = || {
            let toml = r#"
id = "k_short_zero_test"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["AAAUSDT", "BBBUSDT"]
lookback_minutes = 2
rebalance_minutes = 2
k_long = 1
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
score_source = "vol_adjusted_return"
"#;
            let cfg = strategy::CrossSectionalMomentumConfig::from_str(toml)
                .expect("valid k_short=0 config");
            assert_eq!(
                cfg.k_short, 0,
                "LEAK-GUARD: k_short must be 0 for this neutrality test"
            );
            strategy::MomentumStrategy::from_config(
                cfg,
                smol_str::SmolStr::new("k_short_zero_test"),
            )
        };

        let run = || {
            let input = TcnScenarioInput {
                scenario_name: "k_short_zero_neutrality".to_string(),
                start_year: 2023,
                bar_count: bars.len(),
                initial_capital: dec!(100_000),
                slippage_bps: 0,
                taker_fee_bps: 0,
                config_id: "k_short_zero_test".to_string(),
                forecaster_id: "test".to_string(),
                bars_override: Some(bars.clone()),
                emit_equity_bin: None,
                latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
                funding_override: None,
                bar_span_hours: 1,
            };
            pollster::block_on(run_path(input, 0x00C0_FFEE, make_long_only_strat()))
                .expect("run_path ok for k_short=0 neutrality test")
        };

        let r1 = run();
        let r2 = run();

        // ── (1) The golden pin (the constant is declared at the top of this fn) ──
        let rendered = r1
            .equity_curve
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            rendered, GOLDEN_LONG_ONLY_EQUITY_CURVE,
            "LONG-ONLY REGRESSION: run_path's long-only equity curve moved. Every anchored \
             non-MN surface (#86..#107) is produced by this code path, so a change here is a \
             change to all of them — `bash scripts/verify_anchors.sh` is about to go RED. If \
             the move is intended, re-baseline this literal IN THE SAME COMMIT as the anchor \
             re-lock and say why in the commit message."
        );

        // ── (2) Determinism — two runs of the same config agree ───────────────
        // NOTE: this is the assertion the test USED to make on its own. It cannot see a
        // gate leak (a leak corrupts both runs identically); it only catches an unordered
        // fold / a HashMap iteration / an RNG draw sneaking into the loop.
        assert_eq!(
            r1.equity_curve, r2.equity_curve,
            "two runs with k_short=0 must be deterministic (bit-identical equity curve). \
             If they differ, there is a non-determinism bug in the long-only path — \
             unrelated to shorts."
        );

        // ── (3) IMPLIED, not proof ────────────────────────────────────────────
        // At k_short == 0 the strategy emits no short signals, so no short can be opened,
        // so `gross_short_notional` is 0 and the liquidation rule short-circuits before
        // the counter. This assertion therefore holds whatever the `k_short > 0` gate
        // does — it documents the expected state, it does NOT gate the gate. The
        // by-construction neutrality claim is carried by verify_anchors.sh (107/107).
        assert_eq!(
            r1.liquidations, 0,
            "k_short=0 must record no liquidations (implied by 'no shorts exist', not \
             evidence about the gate — see this test's doc comment)"
        );
        // Liveness companion (the bug-log #74 lesson): a pinned curve on an arm that
        // never traded would pin a flat line and prove nothing.
        assert!(
            r1.trades > 0,
            "the fixture must actually TRADE for the golden pin to mean anything; \
             trades={}",
            r1.trades
        );
        assert!(
            r1.min_cash_seen >= Decimal::ZERO,
            "the Bug-B solvency guard must hold on the long-only path; min_cash_seen={}",
            r1.min_cash_seen
        );
    }
}
