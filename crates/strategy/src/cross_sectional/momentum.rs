//! `MomentumStrategy` — v1 cross-sectional momentum strategy (T606).
//!
//! Implements the `Strategy` trait verbatim (v0 shape, unchanged per Q5).
//! Q5 strategy-side filtering: out-of-universe bars are a fast early-return.
//! Q3 long-only: only `Side::Buy` orders; `Side::Sell` only to close positions
//! that fell out of the top-K.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use smol_str::SmolStr;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Tick, Timestamp};

use crate::Strategy;
use crate::cross_sectional::config::{
    CrossSectionalMomentumConfig, Direction, ScoreSource, SelectionMode,
};
use crate::cross_sectional::selector::{bottom_k_short, select_above_threshold, top_k_long};
use features::{RingBuffer, score_trailing_log_return, score_vol_adjusted_return};

/// v1 cross-sectional momentum strategy.
///
/// Implements `Strategy` verbatim (v0 trait shape, no trait change per Q5).
/// `on_tick` returns `vec![]` (momentum is bar-close only).
/// Out-of-universe bars return `vec![]` immediately (Q5 strategy-side filter).
pub struct MomentumStrategy {
    id: StrategyId,
    /// Sorted set of universe symbols (BTreeMap for deterministic iteration).
    universe_symbols: BTreeMap<Symbol, ()>,
    lookback_minutes: u32,
    rebalance_minutes: u32,
    k_long: u32,
    /// Number of shorts to hold. `0` in all non-LongShort modes; `> 0` only
    /// under `SelectionMode::LongShort` (M-DEV-2, D-MN.5).
    k_short: u32,
    vol_floor: Decimal,
    #[allow(dead_code)]
    drift_threshold: Decimal,
    exposure_cap: Decimal,
    /// Strategy family direction (D-MR.0). Default = `Momentum` (v1 behavior).
    /// `Reversion` negates the score at the cache-write boundary so `top_k_long`
    /// selects bottom-K losers instead of top-K winners.
    direction: Direction,
    /// Score source (M-DEV-5, D-CARRY.1). Default = `VolAdjustedReturn` (anchor-neutral).
    /// `FundingCarry` switches the score to `−trailing_mean(funding)` (R-CARRY.2 sign).
    score_source: ScoreSource,
    /// Selection mode (M-DEV-1, D-TSM.1). Default = `CrossSectionalTopK` (anchor-neutral).
    /// `TimeSeriesLongFlat` switches selection to per-asset threshold gating (D-TSM.1).
    selection_mode: SelectionMode,
    /// Flat/entry threshold for `TimeSeriesLongFlat` (D-TSM.1). Default = `Decimal::ZERO`.
    /// Only consumed under `SelectionMode::TimeSeriesLongFlat`.
    entry_threshold: Decimal,

    /// Per-symbol ring buffers of close prices (size = lookback_minutes + 1).
    histories: BTreeMap<Symbol, RingBuffer>,
    /// Per-symbol latest score cache (None = warming up).
    scores: BTreeMap<Symbol, Option<Decimal>>,
    /// Timestamp of the last rebalance bar close (None before first rebalance).
    last_rebalance_ts: Option<Timestamp>,
    /// Current per-symbol long position — tracked as held/flat.
    /// Maintained from the signals emitted (approximate).
    held_symbols: BTreeMap<Symbol, bool>,
    /// Current per-symbol short position (M-DEV-2, D-MN.5).
    /// `false` for all symbols in non-LongShort modes → zero overhead.
    held_short_symbols: BTreeMap<Symbol, bool>,

    // ── Carry-strategy funding state (M-DEV-5, D-CARRY.1) ────────────────────
    //
    // Populated via `with_funding(...)` after `from_config`; `None` for every
    // momentum/MR run → anchor-neutral, zero overhead.
    //
    /// Injected funding lookup: `(Symbol, open_ts) → Decimal`.
    /// Built from `GeneratedPath.funding_by_symbol` + synthetic open_ts.
    funding_map: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
    /// Per-symbol settlement ring: the last L settled funding rates, in
    /// ascending settlement order. `VecDeque` with a capacity of `funding_lookback`.
    funding_rings: BTreeMap<Symbol, std::collections::VecDeque<Decimal>>,
    /// Number of settlements in the trailing mean (L). Maps to the config's
    /// `lookback_minutes` field when `score_source == FundingCarry` (D-CARRY.2-LOCKED:
    /// the grid column is L in settlements, passed literally as `lookback_minutes`).
    funding_lookback: usize,

    /// Injected basis score lookup for the basis⊥funding residual arm (M-DEV-4, D-MN.6).
    ///
    /// For `BasisFundingResidual`: carries the BASIS values (same as `funding_map` in
    /// `BasisReversal`). `funding_map` then carries the FUNDING rates for the residual
    /// rank computation. `None` for every non-residual run → anchor-neutral, zero overhead.
    ///
    /// Set via `with_basis_score(…)` after `from_config`. The naming follows the design:
    /// "basis_score" = the map that yields the basis-reversal half of the rank-residual.
    basis_score_map: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
    /// Per-symbol ring buffer for basis scores in the residual arm (M-DEV-4, D-MN.6).
    ///
    /// Mirrors `funding_rings` but used exclusively when `score_source == BasisFundingResidual`
    /// to buffer the trailing basis values (matching the `funding_lookback` window).
    /// `None` entries → warming up; empty for every non-residual run → anchor-neutral.
    basis_score_rings: BTreeMap<Symbol, std::collections::VecDeque<Decimal>>,

    /// SHA-256 of canonicalized config — 32 bytes.
    pub hash: [u8; 32],
    pub source_path: SmolStr,
}

impl MomentumStrategy {
    /// Construct from a validated config.
    #[must_use]
    pub fn from_config(cfg: CrossSectionalMomentumConfig, source_path: SmolStr) -> Self {
        let capacity = cfg.lookback_minutes as usize + 1;
        let symbols: Vec<Symbol> = cfg
            .universe
            .iter()
            .map(|s| Symbol::new(s.as_str()))
            .collect();

        let universe_symbols: BTreeMap<Symbol, ()> =
            symbols.iter().map(|s| (s.clone(), ())).collect();
        let histories: BTreeMap<Symbol, RingBuffer> = symbols
            .iter()
            .map(|s| (s.clone(), RingBuffer::new(capacity)))
            .collect();
        let scores: BTreeMap<Symbol, Option<Decimal>> =
            symbols.iter().map(|s| (s.clone(), None)).collect();
        let held_symbols: BTreeMap<Symbol, bool> =
            symbols.iter().map(|s| (s.clone(), false)).collect();
        let held_short_symbols: BTreeMap<Symbol, bool> =
            symbols.iter().map(|s| (s.clone(), false)).collect();

        // For FundingCarry, `lookback_minutes` encodes L (settlements).
        let funding_lookback = cfg.lookback_minutes as usize;
        let funding_rings: BTreeMap<Symbol, std::collections::VecDeque<Decimal>> = symbols
            .iter()
            .map(|s| {
                (
                    s.clone(),
                    std::collections::VecDeque::with_capacity(funding_lookback + 1),
                )
            })
            .collect();

        let hash = compute_config_hash(&cfg);

        // basis_score_rings: parallel ring buffers for BasisFundingResidual arm.
        // Mirrors funding_rings but for the basis half of the residual.
        // Empty VecDeques for every non-residual run → anchor-neutral, zero overhead.
        let basis_score_rings: BTreeMap<Symbol, std::collections::VecDeque<Decimal>> = symbols
            .iter()
            .map(|s| {
                (
                    s.clone(),
                    std::collections::VecDeque::with_capacity(funding_lookback + 1),
                )
            })
            .collect();

        // Review 1-21: the struct-literal seam bypasses every loader guard.
        //
        // `CrossSectionalMomentumConfig`'s fields are all `pub`, so the sweep driver
        // (`cell_config`) and every e2e fixture build configs WITHOUT going through
        // `from_str` — which is where `DegenerateResidualArm` and its sibling guards
        // live. `from_config` is the one funnel every construction path passes through,
        // so the seam that a loader guard cannot see gets a warning here.
        //
        // Behaviour is UNCHANGED (no bail, no panic — `from_config` returns `Self`, and
        // a panic in library code violates CLAUDE.md). The combination never occurs in
        // production: every anchored MN residual surface ran `long_short`. This exists so
        // that if it ever DOES occur, the run says so instead of quietly emitting a
        // θ-surface labelled `basis_funding_residual` whose numbers came from the
        // basis-reversal signal.
        if cfg.score_source == ScoreSource::BasisFundingResidual
            && cfg.selection_mode != SelectionMode::LongShort
        {
            tracing::warn!(
                strategy_id = %cfg.id,
                score_source = ?cfg.score_source,
                selection_mode = ?cfg.selection_mode,
                "DEGENERATE ARM: score_source=basis_funding_residual computes its rank \
                 residual ONLY under selection_mode=long_short. Under this mode the cached \
                 trailing basis mean is used instead — this run will behave like \
                 basis_reversal while identifying (and hashing) as basis_funding_residual. \
                 `CrossSectionalMomentumConfig::from_str` rejects this combination; it was \
                 reached through the struct-literal seam."
            );
        }

        Self {
            id: StrategyId::new(cfg.id.as_str()),
            universe_symbols,
            lookback_minutes: cfg.lookback_minutes,
            rebalance_minutes: cfg.rebalance_minutes,
            k_long: cfg.k_long,
            k_short: cfg.k_short,
            vol_floor: cfg.vol_floor,
            drift_threshold: cfg.drift_rebalance_threshold,
            exposure_cap: cfg.exposure_cap,
            direction: cfg.direction,
            score_source: cfg.score_source,
            selection_mode: cfg.selection_mode,
            entry_threshold: cfg.entry_threshold,
            histories,
            scores,
            last_rebalance_ts: None,
            held_symbols,
            held_short_symbols,
            funding_map: None,
            funding_rings,
            funding_lookback,
            basis_score_map: None,
            basis_score_rings,
            hash,
            source_path,
        }
    }

    /// Inject the carry funding lookup (M-DEV-5, D-CARRY.1).
    ///
    /// Called by the harness AFTER `from_config` when `score_source == FundingCarry`.
    /// `None` for every momentum/MR run → anchor-neutral zero-overhead default.
    ///
    /// The map is keyed by `(Symbol, open_ts)` on the **same synthetic timestamps**
    /// the bootstrap emits, so `carry_score` looks up funding by the bar's own `open_ts`.
    #[must_use]
    pub fn with_funding(mut self, funding: Option<BTreeMap<(Symbol, Timestamp), Decimal>>) -> Self {
        self.funding_map = funding;
        self
    }

    /// Inject the basis score lookup for the basis⊥funding residual arm (M-DEV-4, D-MN.6).
    ///
    /// For `BasisFundingResidual`: the basis values ride `basis_score_map` (this field),
    /// while `funding_map` carries the FUNDING rates for the cross-sectional rank.
    /// `None` for every non-residual run → anchor-neutral, zero overhead.
    ///
    /// Called by the sweep harness AFTER `from_config` + `with_funding(funding_map)` when
    /// `score_source == BasisFundingResidual`. For all other arms this must NOT be called
    /// (or called with `None`) to preserve anchor-neutrality.
    #[must_use]
    pub fn with_basis_score(
        mut self,
        basis_score: Option<BTreeMap<(Symbol, Timestamp), Decimal>>,
    ) -> Self {
        self.basis_score_map = basis_score;
        self
    }

    /// Return k_short — the number of short legs (M-DEV-3, D-MN.2).
    /// Read by `run_path` to gate the short-side branch. `0` for all non-LongShort
    /// strategies → dead code in `run_path` → anchor-neutral.
    #[must_use]
    pub fn k_short(&self) -> u32 {
        self.k_short
    }

    /// Inherent method — introspection only (Q5: not a trait method).
    /// Returns universe symbols in alphabetical order.
    pub fn universe(&self) -> impl Iterator<Item = &Symbol> {
        self.universe_symbols.keys()
    }

    fn is_rebalance_bar(&self, bar: &Bar) -> bool {
        match self.last_rebalance_ts {
            None => self.all_warmed(),
            Some(prev) => {
                let elapsed_minutes = minutes_since(prev, bar.close_ts);
                elapsed_minutes >= i64::from(self.rebalance_minutes)
            }
        }
    }

    fn all_warmed(&self) -> bool {
        match self.selection_mode {
            SelectionMode::TimeSeriesLongFlat => {
                // TS warm-up: all price history ring buffers must be full.
                // Same as VolAdjustedReturn — the TS trend score uses the price ring.
                // (FundingCarry / BasisReversal are not used under TimeSeriesLongFlat.)
                self.histories.values().all(|rb| rb.is_full())
            }
            SelectionMode::CrossSectionalTopK | SelectionMode::LongShort => {
                match self.score_source {
                    ScoreSource::VolAdjustedReturn => {
                        // Original path: all price history ring buffers must be full.
                        self.histories.values().all(|rb| rb.is_full())
                    }
                    ScoreSource::FundingCarry => {
                        // Carry warm-up: every symbol's funding ring must have ≥ L settlements.
                        // A symbol with no ring entry is not yet warmed (it has seen 0 settlements).
                        self.universe_symbols.keys().all(|sym| {
                            self.funding_rings
                                .get(sym)
                                .is_some_and(|ring| ring.len() >= self.funding_lookback)
                        })
                    }
                    ScoreSource::BasisReversal => {
                        // Basis warm-up: every symbol's funding ring must have ≥ L bars.
                        // The basis arm reuses `funding_rings` as a generic sidecar ring
                        // (D-BR.3 channel reuse). The ring counts BARS (not 8h settlements)
                        // because the basis is native 1h — every bar pushes a basis value.
                        self.universe_symbols.keys().all(|sym| {
                            self.funding_rings
                                .get(sym)
                                .is_some_and(|ring| ring.len() >= self.funding_lookback)
                        })
                    }
                    ScoreSource::BasisFundingResidual => {
                        // Residual warm-up: BOTH the basis ring (basis_score_rings) AND the
                        // funding ring (funding_rings) must have ≥ L entries. The warm-up gate
                        // is the more-conservative conjunction of both sidecars.
                        let basis_ready = self.universe_symbols.keys().all(|sym| {
                            self.basis_score_rings
                                .get(sym)
                                .is_some_and(|ring| ring.len() >= self.funding_lookback)
                        });
                        let funding_ready = self.universe_symbols.keys().all(|sym| {
                            self.funding_rings
                                .get(sym)
                                .is_some_and(|ring| ring.len() >= self.funding_lookback)
                        });
                        basis_ready && funding_ready
                    }
                }
            }
        }
    }

    fn build_rebalance_signals(&mut self, bar: &Bar) -> Vec<Signal> {
        // Fork on selection_mode (D-TSM.1 / D-MN.5):
        // CrossSectionalTopK → top_k_long (VERBATIM byte-identical to v1 path).
        // TimeSeriesLongFlat → select_above_threshold (new, per-asset threshold gating).
        // LongShort → top_k_long (long book) + bottom_k_short (short book).
        //   For BasisFundingResidual: the residual scores override self.scores at rebalance
        //   time — compute them fresh and use them for both top_k_long and bottom_k_short.
        //
        // DEGENERATION NOTE (review 1-21 — the fallback is DOCUMENTED, not silent):
        // when `score_source == BasisFundingResidual` but the mode is NOT LongShort, the
        // `unwrap_or(&self.scores)` below falls back to the per-bar cache, which for this
        // arm holds `basis_trailing_mean_for_residual` — a plain −mean(basis), i.e. the
        // BasisReversal signal, NOT a residual. That combination is rejected outright by
        // `CrossSectionalMomentumConfig::from_str`
        // (`CrossSectionalLoadError::DegenerateResidualArm`) and warned about once in
        // `from_config` for the struct-literal seam that bypasses the loader. Reaching
        // this fallback with the residual score source therefore means someone built the
        // config by hand and ignored a warning.
        let effective_scores: Option<BTreeMap<Symbol, Option<Decimal>>> = if self.selection_mode
            == SelectionMode::LongShort
            && self.score_source == ScoreSource::BasisFundingResidual
        {
            Some(self.build_residual_scores())
        } else {
            None
        };
        let scores_ref: &BTreeMap<Symbol, Option<Decimal>> =
            effective_scores.as_ref().unwrap_or(&self.scores);

        let target_weights = match self.selection_mode {
            SelectionMode::CrossSectionalTopK => {
                top_k_long(&self.scores, self.k_long, self.exposure_cap)
            }
            SelectionMode::TimeSeriesLongFlat => {
                select_above_threshold(&self.scores, self.entry_threshold, self.exposure_cap)
            }
            SelectionMode::LongShort => {
                // Dollar-neutral: long book handled in the normal long-signal loop below;
                // short book emitted as open_short signals (evidence tag "open_short").
                // `target_weights` here is the LONG book — the short book is computed
                // separately below and emitted as explicit Sell signals with "open_short".
                // For BasisFundingResidual: use the residual scores (scores_ref).
                top_k_long(scores_ref, self.k_long, self.exposure_cap)
            }
        };

        let mut signals = Vec::new();
        let ts = bar.close_ts;

        // ── Long-book signals (all modes) ─────────────────────────────────────
        // Iterate in alphabetical order (BTreeMap) per R12.5
        for symbol in self.universe_symbols.keys() {
            let currently_held = *self.held_symbols.get(symbol).unwrap_or(&false);
            let target_weight = target_weights.get(symbol);

            let action = match (currently_held, target_weight) {
                (false, Some(_)) => {
                    // Not held, in new top-K → Open long
                    Some((SignalKind::Buy, "open"))
                }
                (true, None) => {
                    // Held, fell out of top-K → Close long
                    Some((SignalKind::Sell, "close"))
                }
                (true, Some(_)) => {
                    // Held and still in top-K → hold
                    None
                }
                (false, None) => None,
            };

            if let Some((kind, action_str)) = action {
                match kind {
                    SignalKind::Buy => {
                        self.held_symbols.insert(symbol.clone(), true);
                    }
                    SignalKind::Sell => {
                        self.held_symbols.insert(symbol.clone(), false);
                    }
                    SignalKind::Hold => {}
                    // v1.5a pair variants are not emitted by MomentumStrategy
                    _ => {}
                }
                signals.push(Signal {
                    strategy_id: self.id.clone(),
                    symbol: symbol.clone(),
                    ts,
                    kind,
                    evidence: SignalEvidence::momentum(
                        action_str,
                        self.scores
                            .get(symbol)
                            .copied()
                            .flatten()
                            .unwrap_or(Decimal::ZERO),
                    ),
                    pair_data: None, // v1.5a — not a pair signal
                });
            }
        }

        // ── Short-book signals (LongShort mode only — dead code when k_short==0) ─
        // Gated: only entered when selection_mode == LongShort and k_short > 0.
        // The Sell signals emitted here use evidence tag "open_short" so run_path
        // can fork the Sell arm on current_qty (D-MN.5 / D-MN.3 layer 1).
        // For BasisFundingResidual: use scores_ref (the residual scores, already computed above).
        if self.selection_mode == SelectionMode::LongShort && self.k_short > 0 {
            let short_weights = bottom_k_short(scores_ref, self.k_short, self.exposure_cap);

            for symbol in self.universe_symbols.keys() {
                let currently_short = *self.held_short_symbols.get(symbol).unwrap_or(&false);
                let target_short = short_weights.get(symbol);

                let action = match (currently_short, target_short) {
                    (false, Some(_)) => {
                        // Not short, in new bottom-K → Open short (Sell signal, "open_short")
                        Some((SignalKind::Sell, "open_short"))
                    }
                    (true, None) => {
                        // Held short, fell out of bottom-K → Cover short ("close_short" tag)
                        // run_path forks: current_qty < 0 + "close_short" → buy-to-cover.
                        Some((SignalKind::Buy, "close_short"))
                    }
                    (true, Some(_)) => None, // still in bottom-K → hold short
                    (false, None) => None,
                };

                if let Some((kind, action_str)) = action {
                    match kind {
                        SignalKind::Sell => {
                            self.held_short_symbols.insert(symbol.clone(), true);
                        }
                        SignalKind::Buy => {
                            self.held_short_symbols.insert(symbol.clone(), false);
                        }
                        _ => {}
                    }
                    signals.push(Signal {
                        strategy_id: self.id.clone(),
                        symbol: symbol.clone(),
                        ts,
                        kind,
                        evidence: SignalEvidence::momentum(
                            action_str,
                            self.scores
                                .get(symbol)
                                .copied()
                                .flatten()
                                .unwrap_or(Decimal::ZERO),
                        ),
                        pair_data: None,
                    });
                }
            }
        }

        signals
    }

    /// Compute the basis-reversal score for `symbol` at bar `open_ts`
    /// (M-DEV-3, R-BR.1/2 — perp-basis-signal-robustness).
    ///
    /// # Sign convention (R-BR.2 — LOAD-BEARING)
    ///
    /// The perp-spot basis `(markPrice − indexPrice)/indexPrice`:
    /// - **POSITIVE basis** (perp > spot) → crowded long, leveraged longs → the
    ///   crowd subsequently **UNDERPERFORMS** (reversal). HIGH basis → underweight.
    /// - **NEGATIVE basis** (perp < spot) → cheapest perp → the crowd is not
    ///   crowded long → subsequently **OUTPERFORMS**. LOW basis → overweight.
    ///
    /// Therefore: `basis_reversal_score = −trailing_mean(basis)`
    ///
    /// The leading minus makes the **LOWEST-basis** name have the **HIGHEST** score,
    /// which floats it to the TOP of the unchanged descending `top_k_long`. A long
    /// on it IS the reversal (long the reversal-favored leg). This is the ONE place
    /// the sign lives (D-BR.1) — guarded by the R-BR.2 sign-assertion test (RED on flip).
    ///
    /// **A sign flip here turns the arm into a basis-MOMENTUM payer** — it would long
    /// the crowded-long names that subsequently underperform. The sign-assertion
    /// falsifier (`r_br2_sign_assertion_longs_low_basis_name`) catches this exactly.
    ///
    /// # Channel reuse (D-BR.3 — CRITICAL COMMENT, MANDATORY)
    ///
    /// **The basis arm reuses the `funding_by_symbol`/`funding_map` channel as a
    /// generic sidecar carrier — the value is the BASIS, not funding, and is consumed
    /// ONLY by `basis_reversal_score`, NEVER by the `run_path` accrual (which stays
    /// gated `None` for the basis arm — D-BR.1).** The `run_path` accrual block
    /// (`montecarlo.rs:322`) is only entered when `funding_override` in
    /// `TcnScenarioInput` is `Some`; for the basis arm it is `None` → no cashflow.
    /// The basis IS a selection signal (R-BR.9 confirmed), NOT a cash settlement.
    ///
    /// # Bar-ring warm-up
    ///
    /// The ring must hold ≥ `funding_lookback` bars before a score is valid.
    /// Before that, returns `None` (excluded from the rank — same as a warming-up
    /// momentum or carry score). Warm-up count is in BARS (the basis is native 1h),
    /// not 8h settlements — every bar pushes a basis value via the map.
    ///
    /// # Averaging window — `(t−L, t]`, INCLUSIVE of the current bar (review 1-20 M)
    ///
    /// The current bar's value is pushed into the ring BEFORE the mean is taken,
    /// so the window is `(t−L, t]` — the last `L` values ending at and including
    /// `t`. It is **not** "the L values strictly before `t`", which is what the
    /// spec prose said. Two consequences worth knowing:
    ///
    /// - At `L = 1` the mean degenerates to the identity (`sum/len` over one
    ///   element) and the eviction loop never evicts, so an `L = 1` test cannot
    ///   observe the trailing-mean or ring logic at all. The production grid runs
    ///   `L ∈ {24, 60, 168}` (§ D-BR.2-LOCKED); the coverage for that lives in
    ///   `r_br_trailing_mean_ring_at_production_lookback`.
    /// - The value at `t` is the one `basis_data::basis_as_of` returned for `t`,
    ///   which on the aligned 1h grid is `basis_close[t]` — a value realised at
    ///   that bar's CLOSE, not its open. That is causal in the anchored lane only
    ///   because `PaperEngine` fills at the bar close too
    ///   (`FillPriceMode::BarClose`), so the score is priced at the instant its
    ///   inputs are realised. It would be a one-bar look-ahead in any consumer
    ///   that fills earlier than the bar close. The full argument, and the
    ///   correction of the old (wrong) `basis_close[t-1]` claim, is in the
    ///   "Join key and causality" block on `backtest::basis_data::basis_as_of`.
    ///   Changing the join re-prices every anchored basis/MN surface and is owned
    ///   by story 1-25, not by this signal.
    fn basis_reversal_score(&mut self, symbol: &Symbol, open_ts: Timestamp) -> Option<Decimal> {
        // Fetch the basis value for this (symbol, bar_ts) pair.
        // The basis map is keyed for EVERY bar (the co-resampled value is the
        // basis-in-force at that real return step on the native 1h grid).
        // We look up regardless and push if Some — same pattern as carry_score.
        let basis_value = self
            .funding_map
            .as_ref()
            .and_then(|m| m.get(&(symbol.clone(), open_ts)).copied());

        // Push into the sidecar ring on any non-None basis lookup.
        // The ring is the SAME `funding_rings` as carry (D-BR.3 channel reuse).
        // We push on every bar because the basis is native 1h (every bar is a
        // "settlement" on the basis grid — unlike carry's sparse 8h cadence).
        if let Some(value) = basis_value {
            let ring = self.funding_rings.entry(symbol.clone()).or_default();
            ring.push_back(value);
            // Keep only the last L bars.
            while ring.len() > self.funding_lookback {
                ring.pop_front();
            }
        }

        // Compute the trailing mean only when the ring is full (warm-up guard).
        let ring = self.funding_rings.get(symbol)?;
        // `is_empty` guards the DIVISION below (review 1-20 wave-2 L).
        // `funding_lookback == 0` makes the drain loop above empty the ring the
        // instant it is pushed, and `0 < 0` is false — so control reaches
        // `sum / Decimal::from(0)`, which PANICS. `lookback_minutes = 0` is
        // rejected by `CrossSectionalMomentumConfig::from_str`, but every field
        // on that struct is `pub`, so the struct-literal seam (which the e2e
        // tests use) bypasses that check entirely. A panic in library code is a
        // CLAUDE.md violation; return the same `None` the warm-up path returns.
        // For every `funding_lookback >= 1` this is a strict no-op: a ring
        // entry only exists after a push, and a push leaves at least one
        // element, so `is_empty()` is unreachable there.
        if ring.is_empty() || ring.len() < self.funding_lookback {
            return None;
        }
        let sum: Decimal = ring.iter().copied().sum();
        let mean = sum / Decimal::from(ring.len() as u64);
        // R-BR.2: return −mean so the lowest-basis name has the highest score.
        // HIGH basis → HIGH crowd → underperforms → −mean is NEGATIVE → bottom rank.
        // LOW basis → LOW crowd → outperforms → −mean is POSITIVE → top rank.
        // **This is the load-bearing sign. ONE place. A flip → basis-MOMENTUM payer.**
        Some(-mean)
    }

    /// Compute the carry score for `symbol` at bar `open_ts` (M-DEV-5, R-CARRY.1/2).
    ///
    /// # Sign convention (R-CARRY.2 — LOAD-BEARING)
    ///
    /// Binance perpetual funding: **positive funding → LONGS pay shorts**; to EARN
    /// the funding, hold the SHORT (paid) side. Under framing (a) long-only
    /// (D-CARRY.0), we LONG the most-**negative**-funding names — those are the
    /// names where the **LONG side is the paid side** (shorts pay longs when funding
    /// is negative). Therefore:
    ///
    ///   `carry_score = −trailing_mean(funding)`
    ///
    /// The leading minus flips the sign so the most-negative-funding name has the
    /// **highest** carry score, which floats it to the TOP of the unchanged descending
    /// `top_k_long`. This is the one place the sign lives (D-CARRY.1); guarded by
    /// the R-CARRY.2 sign-assertion test.
    ///
    /// # Settlement-ring warm-up
    ///
    /// The ring must hold ≥ `funding_lookback` settlements before a score is valid.
    /// Before that, returns `None` (excluded from the rank — same as a warming-up
    /// momentum score).
    ///
    /// # Funding injection
    ///
    /// Funding is looked up from `self.funding_map` by `(symbol, open_ts)`. If the
    /// map is `None` or the key is absent, no settlement is recorded for this bar
    /// and the score remains `None` until the ring is full from actual settlements.
    fn carry_score(&mut self, symbol: &Symbol, open_ts: Timestamp) -> Option<Decimal> {
        // Fetch the funding rate for this (symbol, bar_ts) pair.
        // Each synthetic bar maps to a funding value co-resampled by the same idx_seq.
        // Not every bar is a settlement boundary — only every 8th bar carries a
        // non-None funding value in the map. We look up regardless and push if Some.
        let funding_rate = self
            .funding_map
            .as_ref()
            .and_then(|m| m.get(&(symbol.clone(), open_ts)).copied());

        // Push into the settlement ring on any non-None funding lookup.
        // The funding map is keyed for EVERY bar (not just 8h boundaries) — the
        // co-resampled value is the funding-in-force at that real return step, which
        // updates every 8h. Only push when we actually see a value.
        if let Some(rate) = funding_rate {
            let ring = self.funding_rings.entry(symbol.clone()).or_default();
            ring.push_back(rate);
            // Keep only the last L settlements.
            while ring.len() > self.funding_lookback {
                ring.pop_front();
            }
        }

        // Compute the trailing mean only when the ring is full (warm-up guard).
        let ring = self.funding_rings.get(symbol)?;
        // `is_empty` guards the DIVISION below (review 1-20 wave-2 L).
        // `funding_lookback == 0` makes the drain loop above empty the ring the
        // instant it is pushed, and `0 < 0` is false — so control reaches
        // `sum / Decimal::from(0)`, which PANICS. `lookback_minutes = 0` is
        // rejected by `CrossSectionalMomentumConfig::from_str`, but every field
        // on that struct is `pub`, so the struct-literal seam (which the e2e
        // tests use) bypasses that check entirely. A panic in library code is a
        // CLAUDE.md violation; return the same `None` the warm-up path returns.
        // For every `funding_lookback >= 1` this is a strict no-op: a ring
        // entry only exists after a push, and a push leaves at least one
        // element, so `is_empty()` is unreachable there.
        if ring.is_empty() || ring.len() < self.funding_lookback {
            return None;
        }
        let sum: Decimal = ring.iter().copied().sum();
        let mean = sum / Decimal::from(ring.len() as u64);
        // R-CARRY.2: return −mean so the most-negative-funding name has the highest score.
        Some(-mean)
    }

    // ── BasisFundingResidual helpers (M-DEV-4, D-MN.6) ────────────────────────

    /// Push a basis value into `basis_score_rings` for the residual arm.
    ///
    /// The basis value is keyed by `(symbol, open_ts)` in `basis_score_map`.
    /// Mirrors `basis_reversal_score` but writes to `basis_score_rings` instead of
    /// `funding_rings`, so that the funding ring stays free for actual funding rates.
    fn push_basis_score_ring(&mut self, symbol: &Symbol, open_ts: Timestamp) {
        let basis_value = self
            .basis_score_map
            .as_ref()
            .and_then(|m| m.get(&(symbol.clone(), open_ts)).copied());
        if let Some(value) = basis_value {
            let ring = self.basis_score_rings.entry(symbol.clone()).or_default();
            ring.push_back(value);
            while ring.len() > self.funding_lookback {
                ring.pop_front();
            }
        }
    }

    /// Push a funding value into `funding_rings` for the residual arm.
    ///
    /// Mirrors the carry push in `carry_score` but is called explicitly from the
    /// `BasisFundingResidual` branch so the ring is populated per-bar.
    fn push_funding_ring(&mut self, symbol: &Symbol, open_ts: Timestamp) {
        let funding_rate = self
            .funding_map
            .as_ref()
            .and_then(|m| m.get(&(symbol.clone(), open_ts)).copied());
        if let Some(rate) = funding_rate {
            let ring = self.funding_rings.entry(symbol.clone()).or_default();
            ring.push_back(rate);
            while ring.len() > self.funding_lookback {
                ring.pop_front();
            }
        }
    }

    /// Compute the trailing basis mean for the residual arm (warm-up gate).
    ///
    /// Returns `Some(−mean)` when `basis_score_rings[symbol].len() >= funding_lookback`,
    /// else `None` (warming up). The negation mirrors `basis_reversal_score` so the
    /// score stored in `self.scores` has the correct direction for the warm-up gate
    /// (a non-None value = warmed, a None = still warming).
    fn basis_trailing_mean_for_residual(&self, symbol: &Symbol) -> Option<Decimal> {
        let ring = self.basis_score_rings.get(symbol)?;
        // See `basis_reversal_score`: `is_empty` guards the division when
        // `funding_lookback == 0` (review 1-20 wave-2 L). No-op for lookback >= 1.
        if ring.is_empty() || ring.len() < self.funding_lookback {
            return None;
        }
        let sum: Decimal = ring.iter().copied().sum();
        let mean = sum / Decimal::from(ring.len() as u64);
        Some(-mean) // negated for the same direction as basis_reversal_score
    }

    /// Build the cross-sectional rank-residual scores for the `BasisFundingResidual` arm.
    ///
    /// For each warmed symbol in the cross-section, compute:
    ///   `residual_score[sym] = rank(basis_reversal_score) − rank(funding_carry_score)`
    ///
    /// Both ranks are 1..N integers (Decimal-exact, NO division, NO rounding, NO f64).
    /// Ties in either rank use alphabetical `BTreeMap` order (the existing tie-break).
    ///
    /// Long = highest residual (low-basis RELATIVE to its funding level).
    /// Short = lowest residual (high-basis relative to its funding level).
    ///
    /// This is the Spearman-style residual that matches the rank-IC channel the spike
    /// measured (the −0.10 is a RANK IC). D-MN.6 / ADR-0003 Decimal-exact requirement.
    fn build_residual_scores(&self) -> BTreeMap<Symbol, Option<Decimal>> {
        // Compute basis trailing means for all warmed symbols (negated, as in basis_reversal_score).
        // Collect (symbol, basis_score) pairs with Some values — alphabetical BTreeMap order.
        let mut basis_warmed: Vec<(Symbol, Decimal)> = self
            .universe_symbols
            .keys()
            .filter_map(|sym| {
                let ring = self.basis_score_rings.get(sym)?;
                // Division guard — see `basis_reversal_score` (1-20 wave-2 L).
                if ring.is_empty() || ring.len() < self.funding_lookback {
                    return None;
                }
                let sum: Decimal = ring.iter().copied().sum();
                let mean = sum / Decimal::from(ring.len() as u64);
                Some((sym.clone(), -mean)) // negated: lowest basis → highest score
            })
            .collect();

        // Compute funding trailing means for all warmed symbols (negated, as in carry_score).
        let funding_warmed: Vec<(Symbol, Decimal)> = self
            .universe_symbols
            .keys()
            .filter_map(|sym| {
                let ring = self.funding_rings.get(sym)?;
                // Division guard — see `basis_reversal_score` (1-20 wave-2 L).
                if ring.is_empty() || ring.len() < self.funding_lookback {
                    return None;
                }
                let sum: Decimal = ring.iter().copied().sum();
                let mean = sum / Decimal::from(ring.len() as u64);
                Some((sym.clone(), -mean)) // negated: most-negative-funding → highest score
            })
            .collect();

        // Find the common warmed set (symbols warmed in BOTH basis and funding rings).
        // Use a BTreeMap for deterministic ordering (the rank tie-break is alphabetical).
        let funding_map: BTreeMap<Symbol, Decimal> =
            funding_warmed.iter().cloned().collect::<BTreeMap<_, _>>();
        basis_warmed.retain(|(sym, _)| funding_map.contains_key(sym));

        // N = number of commonly-warmed symbols.
        let n = basis_warmed.len();
        if n == 0 {
            // No commonly-warmed symbol → return None for all.
            return self
                .universe_symbols
                .keys()
                .map(|sym| (sym.clone(), None))
                .collect();
        }

        // Rank basis scores (stable sort descending; alphabetical tie-break via BTreeMap order).
        // basis_warmed is already in alphabetical order (BTreeMap iteration).
        // Stable sort descending → equal basis scores keep alphabetical order.
        basis_warmed.sort_by(|a, b| b.1.cmp(&a.1));
        let basis_rank: BTreeMap<Symbol, Decimal> = basis_warmed
            .iter()
            .enumerate()
            .map(|(i, (sym, _))| (sym.clone(), Decimal::from(i as u64 + 1)))
            .collect();

        // Rank funding scores (stable sort descending; alphabetical tie-break).
        // funding_warmed subset: keep only the commonly-warmed symbols.
        let mut funding_subset: Vec<(Symbol, Decimal)> = funding_map
            .into_iter()
            .filter(|(sym, _)| basis_rank.contains_key(sym))
            .collect();
        // Sort alphabetically first to get deterministic tie-break, then stable sort descending.
        funding_subset.sort_by(|a, b| a.0.cmp(&b.0));
        funding_subset.sort_by(|a, b| b.1.cmp(&a.1));
        let funding_rank: BTreeMap<Symbol, Decimal> = funding_subset
            .iter()
            .enumerate()
            .map(|(i, (sym, _))| (sym.clone(), Decimal::from(i as u64 + 1)))
            .collect();

        // residual_score = rank(funding) − rank(basis) — integer arithmetic, NO division.
        //
        // ── DIRECTION FIX, bug-log #76 (story 1-25) ──────────────────────────
        // This was `rank(basis) − rank(funding)`, which ranked the basis axis
        // INVERTED against this arm's own spec. The chain:
        //
        //   basis_reversal_score = −trailing_mean(basis)          (see :495)
        //   ranks are built by DESCENDING sort  ⇒  rank 1 = best
        //   ⇒ rank 1 = highest reversal score = LOWEST basis
        //
        // The old comment reasoned "positive residual: basis rank higher …
        // more-reversal-favored", but with rank 1 = best a HIGHER rank NUMBER is
        // WORSE. So `rank(basis) − rank(funding)` was maximal for the WORST basis,
        // and `top_k_long` (which takes the HIGHEST scores) therefore longed the
        // HIGHEST basis — the exact opposite of `config.rs:85`'s contract:
        // "highest residual → long (lowest basis relative to funding)".
        //
        // Worked example, universe of 10:
        //   ideal long = lowest basis (rank_basis 1) whose funding is unfavourable
        //   (rank_funding 10) — its basis is better than funding predicts.
        //     OLD: 1 − 10 = −9  → lowest score → the arm SHORTED it
        //     NEW: 10 − 1 = +9  → highest score → the arm LONGS it   ✓
        //
        // Positive residual: funding rank worse than basis rank → the basis is more
        // favourable than funding alone predicts. Long these.
        // Negative residual: short these.
        let mut result: BTreeMap<Symbol, Option<Decimal>> = self
            .universe_symbols
            .keys()
            .map(|sym| (sym.clone(), None))
            .collect();

        for sym in basis_rank.keys() {
            if let (Some(&br), Some(&fr)) = (basis_rank.get(sym), funding_rank.get(sym)) {
                *result.entry(sym.clone()).or_insert(None) = Some(fr - br);
            }
        }

        result
    }
}

impl Strategy for MomentumStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // Q5 strategy-side filtering — out-of-universe bar is a no-op.
        if !self.universe_symbols.contains_key(&bar.symbol) {
            return Vec::new();
        }

        // Compute score — fork on selection_mode first (D-TSM.1 / M-DEV-3):
        //   TimeSeriesLongFlat → raw cumulative log-return over L (D-TSM.2-note).
        //   CrossSectionalTopK → existing score_source fork (VolAdjustedReturn / FundingCarry).
        // The TS score branch is fully independent; the existing score_source arms are
        // byte-untouched (CrossSectionalTopK path is the DEFAULT).
        let score = match self.selection_mode {
            SelectionMode::TimeSeriesLongFlat => {
                // TS trend score: raw Σ log-return over L bars (D-TSM.2-note).
                // Push close into the ring (same ring as VolAdjustedReturn).
                if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                    rb.push(bar.close.get());
                }
                // Compute raw trailing log-return. None → warmup (excluded from selection).
                match self.histories.get(&bar.symbol).map(|rb| {
                    (
                        score_trailing_log_return(rb, self.lookback_minutes),
                        rb.is_full(),
                    )
                }) {
                    Some((Ok(score), _)) => Some(score),
                    Some((Err(err), ring_full)) => {
                        // Review 1-17: a POST-warmup score error on a HELD symbol
                        // silently force-exits (None → absent from
                        // select_above_threshold → Sell at the next rebalance;
                        // recovery → Buy = a fee-paying round-trip on a data
                        // glitch). Trace it loudly; behavior is UNCHANGED (the
                        // score stays None). Warmup Nones (ring not yet full)
                        // are expected and stay silent.
                        if ring_full && *self.held_symbols.get(&bar.symbol).unwrap_or(&false) {
                            tracing::warn!(
                                symbol = %bar.symbol,
                                bar_ts = %bar.close_ts,
                                error = %err,
                                "TS trend-score error on a held symbol post-warmup — \
                                 the symbol will be force-exited at the next rebalance \
                                 (score = None → absent from selection)"
                            );
                        }
                        None
                    }
                    None => None,
                }
                // Direction is ignored under TimeSeriesLongFlat (no inversion — the
                // threshold comparison IS the direction signal, D-TSM.1).
            }
            SelectionMode::CrossSectionalTopK | SelectionMode::LongShort => {
                // EXISTING path — byte-identical to pre-TS code for CrossSectionalTopK.
                // LongShort uses the SAME score computation (same signal, different selection).
                match self.score_source {
                    ScoreSource::VolAdjustedReturn => {
                        // EXISTING path — byte-identical to pre-carry code.
                        // Push close into the symbol's ring buffer.
                        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                            rb.push(bar.close.get());
                        }
                        // Recompute score for this symbol.
                        let score = self.histories.get(&bar.symbol).and_then(|rb| {
                            score_vol_adjusted_return(rb, self.lookback_minutes, self.vol_floor)
                                .ok()
                        });
                        // D-MR.1: invert at the cache boundary.
                        // Momentum stores +score; Reversion stores −score so the unchanged
                        // descending `top_k_long` selects the bottom-K losers.
                        match self.direction {
                            Direction::Momentum => score,
                            Direction::Reversion => score.map(|s| -s),
                        }
                    }
                    ScoreSource::FundingCarry => {
                        // NEW carry path (M-DEV-5): −trailing_mean(funding) over L settlements.
                        // The sign is in carry_score (R-CARRY.2); Direction stays Momentum (identity).
                        // We still push close for history (no-op for the score but keeps the ring
                        // consistent if score_source ever changes mid-run — defensive).
                        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                            rb.push(bar.close.get());
                        }
                        self.carry_score(&bar.symbol, bar.open_ts)
                    }
                    ScoreSource::BasisReversal => {
                        // BASIS-REVERSAL path (M-DEV-3): −trailing_mean(basis) over L bars.
                        // The sign is in basis_reversal_score (R-BR.2 — LOAD-BEARING).
                        // Direction::Momentum (identity) stays — the sign lives in the score.
                        // We still push close for history consistency (defensive, same as carry).
                        //
                        // CHANNEL REUSE NOTE (D-BR.3 — mandatory):
                        // The basis arm reuses the `funding_by_symbol`/`funding_map` channel as
                        // a generic sidecar carrier — the value IS the BASIS, not funding, and is
                        // consumed ONLY by `basis_reversal_score`. The `run_path` accrual (which
                        // is entered only when `TcnScenarioInput.funding_override.is_some()`) is
                        // NEVER entered for the basis arm — no cashflow (D-BR.1, R-BR.9).
                        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                            rb.push(bar.close.get());
                        }
                        self.basis_reversal_score(&bar.symbol, bar.open_ts)
                    }
                    ScoreSource::BasisFundingResidual => {
                        // BASIS⊥FUNDING RANK-RESIDUAL path (M-DEV-4, D-MN.6).
                        // Per-bar: push both the basis value (into basis_score_rings, keyed via
                        // basis_score_map) and the funding value (into funding_rings, keyed via
                        // funding_map). The SCORE returned is a placeholder (the raw basis mean)
                        // that gets stored in `self.scores`. At rebalance time, the cross-section
                        // is ranked and residual = rank(basis_score) − rank(funding_score)
                        // is computed via `build_residual_scores` and used for selection.
                        //
                        // Here we push per-symbol values into both ring buffers so they are full
                        // and ready for ranking at the next rebalance. The score stored in
                        // `self.scores` is the raw basis value for warm-up purposes; the actual
                        // selection uses `build_residual_scores` at rebalance time.
                        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                            rb.push(bar.close.get());
                        }
                        // Push basis value into basis_score_rings.
                        self.push_basis_score_ring(&bar.symbol, bar.open_ts);
                        // Push funding value into funding_rings (reuse existing carry push path).
                        self.push_funding_ring(&bar.symbol, bar.open_ts);
                        // Return the trailing basis mean as the score (warm-up gate via funding_lookback).
                        // At rebalance time, `build_rebalance_signals` overrides with residual.
                        self.basis_trailing_mean_for_residual(&bar.symbol)
                    }
                }
            }
        };
        self.scores.insert(bar.symbol.clone(), score);

        // Decide if this is a rebalance bar.
        if !self.is_rebalance_bar(bar) {
            return Vec::new();
        }

        let signals = self.build_rebalance_signals(bar);
        self.last_rebalance_ts = Some(bar.close_ts);
        signals
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        Vec::new()
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        CrossSectionalMomentumConfig::json_schema()
    }
}

// ── Config hash ───────────────────────────────────────────────────────────────

/// SHA-256 over the canonicalized config string — the strategy's K3 identity.
///
/// # Hash-domain continuity note (review 1-16)
///
/// The canonical string's DOMAIN has migrated as axes were added: story 1-16
/// (D-MR.0) appended `;direction=…` for ALL configs — including every pre-1-16
/// momentum config — and M-DEV-5/M-DEV-1 later appended
/// `;score_source=…`, `;selection_mode=…`, and `;entry_threshold=…`. A config
/// hashed before an append therefore has a DIFFERENT hash today than when it
/// shipped, even though its behavior is pinned (anchors byte-identical):
/// identity migrated, behavior did not. Consumers of this hash — the strategy
/// lifecycle events (`audit::journal::strategy_event` `old_hash`/`new_hash`,
/// written via `crates/agent/src/watcher.rs` from `MomentumStrategy::hash`) and
/// the `core::strategy_events` broadcast/read-side views — must NOT compare a
/// stored historical hash against a freshly computed one across such a domain
/// migration. Forward continuity from 1-16 onward is pinned by
/// `config_hash_momentum_default_pinned` below; extending the canonical string
/// again is a deliberate migration — update that pin and this note together.
fn compute_config_hash(cfg: &CrossSectionalMomentumConfig) -> [u8; 32] {
    // Canonicalized: sort universe alphabetically, then hash the joined fields.
    let mut universe_sorted = cfg.universe.clone();
    universe_sorted.sort();

    // M-DEV-5: append ;score_source={...} so carry-vs-momentum at the same θ
    // hashes differently (K3 — the config hash distinguishes strategy variants).
    // M-DEV-1: append ;selection_mode={...};entry_threshold={...} so a TS cell
    // hashes differently from a momentum cell at the same lookback (K3).
    let canonical = format!(
        "id={id};universe={uni};lookback={lb};rebalance={rb};k_long={kl};k_short={ks};\
         exposure_cap={ec};drift={dt};vol_floor={vf};direction={dir:?};score_source={ss:?};\
         selection_mode={sm:?};entry_threshold={et}",
        id = cfg.id,
        uni = universe_sorted.join(","),
        lb = cfg.lookback_minutes,
        rb = cfg.rebalance_minutes,
        kl = cfg.k_long,
        ks = cfg.k_short,
        ec = cfg.exposure_cap,
        dt = cfg.drift_rebalance_threshold,
        vf = cfg.vol_floor,
        dir = cfg.direction,
        ss = cfg.score_source,
        sm = cfg.selection_mode,
        et = cfg.entry_threshold,
    );

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}

// ── Timestamp helpers ─────────────────────────────────────────────────────────

fn minutes_since(prev: Timestamp, now: Timestamp) -> i64 {
    let diff_ns = now.inner().unix_timestamp_nanos() - prev.inner().unix_timestamp_nanos();
    let diff_minutes = diff_ns / 60_000_000_000_i128;
    i64::try_from(diff_minutes).unwrap_or(i64::MAX)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Price, Quantity, Timeframe, Venue};

    fn make_bar(symbol: &str, close: Decimal, offset_minutes: i64) -> Bar {
        let base = OffsetDateTime::UNIX_EPOCH;
        let ts = Timestamp::new(base + time::Duration::minutes(offset_minutes));
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneMinute,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn make_strategy(lookback: u32, rebalance: u32, k_long: u32) -> MomentumStrategy {
        let toml = format!(
            r#"
id = "test_momentum"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
lookback_minutes = {lookback}
rebalance_minutes = {rebalance}
k_long = {k_long}
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#
        );
        let cfg =
            crate::cross_sectional::config::CrossSectionalMomentumConfig::from_str(&toml).unwrap();
        MomentumStrategy::from_config(cfg, SmolStr::new("test"))
    }

    fn make_strategy_with_direction(
        lookback: u32,
        rebalance: u32,
        k_long: u32,
        direction: crate::cross_sectional::config::Direction,
    ) -> MomentumStrategy {
        use crate::cross_sectional::config::CrossSectionalMomentumConfig;
        let mut cfg = CrossSectionalMomentumConfig::from_str(&format!(
            r#"
id = "test_dir"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"]
lookback_minutes = {lookback}
rebalance_minutes = {rebalance}
k_long = {k_long}
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#
        ))
        .unwrap();
        cfg.direction = direction;
        MomentumStrategy::from_config(cfg, SmolStr::new("test"))
    }

    #[test]
    fn t606_out_of_universe_bar_ignored() {
        let mut strat = make_strategy(5, 10, 2);
        let bar = make_bar("XRPUSDT", dec!(100), 1);
        let signals = strat.on_bar(&bar);
        assert!(
            signals.is_empty(),
            "out-of-universe bar should return no signals"
        );
    }

    #[test]
    fn t606_warmup_no_signals() {
        let mut strat = make_strategy(5, 10, 2);
        // Push only 3 bars (need 6 for lookback=5+1)
        for i in 0..3i64 {
            for sym in &["BTCUSDT", "ETHUSDT", "BNBUSDT"] {
                let signals = strat.on_bar(&make_bar(sym, dec!(100) + Decimal::from(i), i));
                assert!(signals.is_empty(), "warming up — expect no signals");
            }
        }
    }

    #[test]
    fn t606_warmup_then_rebalance() {
        let lookback: u32 = 5;
        let rebalance: u32 = 6;
        let mut strat = make_strategy(lookback, rebalance, 2);

        let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];
        // Different trends for each symbol to differentiate scores
        let prices: BTreeMap<&str, Vec<f64>> = [
            (
                "BTCUSDT",
                (0..=20).map(|i| 10000.0 + i as f64 * 50.0).collect(),
            ),
            (
                "ETHUSDT",
                (0..=20).map(|i| 500.0 + i as f64 * 5.0).collect(),
            ),
            (
                "BNBUSDT",
                (0..=20).map(|i| 300.0 - i as f64 * 1.0).collect(),
            ),
        ]
        .into_iter()
        .collect();

        let mut last_signals = Vec::new();
        for bar_idx in 0..=20i64 {
            for sym in &symbols {
                let price_f = prices[sym][bar_idx as usize];
                let price = Decimal::try_from(price_f).unwrap();
                let bar = make_bar(sym, price, bar_idx);
                let signals = strat.on_bar(&bar);
                if !signals.is_empty() {
                    last_signals = signals;
                }
            }
        }
        // After enough bars, should have generated at least one rebalance signal
        // (when all symbols are warmed and rebalance_minutes elapsed)
        // Note: exact timing depends on bar ordering, just check structure is valid
        for sig in &last_signals {
            assert!(
                symbols.contains(&sig.symbol.0.as_str()),
                "signal symbol should be in universe"
            );
        }
    }

    #[test]
    fn t606_deterministic_two_runs() {
        let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];

        let run = || {
            let mut strat = make_strategy(5, 6, 2);
            let mut all_kinds: Vec<(String, String)> = Vec::new();
            for bar_idx in 0..30i64 {
                for (si, sym) in symbols.iter().enumerate() {
                    let price = Decimal::from(1000u32 + si as u32 * 100 + bar_idx as u32 * 10);
                    let signals = strat.on_bar(&make_bar(sym, price, bar_idx));
                    for s in signals {
                        all_kinds.push((s.symbol.to_string(), format!("{:?}", s.kind)));
                    }
                }
            }
            all_kinds
        };

        let run1 = run();
        let run2 = run();
        assert_eq!(
            run1, run2,
            "signal sequence must be identical across two runs"
        );
    }

    // ── M-DEV-2: Score inversion — Reversion selects opposite symbols from Momentum ─

    /// M-DEV-2: With a 3-symbol universe and K=1, Momentum picks the top winner
    /// and Reversion picks the worst loser — the two selected-symbol sets are disjoint.
    ///
    /// BTCUSDT trends strongly up (+5.0 absolute price step per bar from 100):
    /// highest momentum score.
    /// ETHUSDT is flat.
    /// BNBUSDT trends strongly down (−1.5 absolute price step per bar from 30):
    /// lowest momentum score / highest MR score.
    /// (Review 1-16 truthfix: the fixture applies LINEAR absolute per-bar steps,
    /// not constant-percentage moves — the old comment claimed ±5%/bar.)
    ///
    /// Momentum K=1 → BTCUSDT.
    /// Reversion K=1 → BNBUSDT (negated score floats it to the top).
    #[test]
    fn mr_dev2_reversion_selects_opposite_symbols() {
        use crate::cross_sectional::config::Direction;

        let lookback: u32 = 3;
        let rebalance: u32 = 3;
        let k_long: u32 = 1; // K=1 < universe size (3) → sets guaranteed disjoint

        // BTC: strong uptrend; ETH: flat; BNB: strong downtrend.
        // After lookback bars, momentum score: BTC > ETH > BNB.
        // After negation (Reversion): BNB_neg > ETH_neg > BTC_neg.
        let prices_mom: &[(&str, f64, f64)] = &[
            ("BTCUSDT", 100.0, 5.0), // start=100, per-bar Δ=+5
            ("ETHUSDT", 50.0, 0.0),  // flat
            ("BNBUSDT", 30.0, -1.5), // downtrend
        ];

        // Helper: run a strategy for N bars and return the set of symbols that had
        // a Buy signal on the LAST rebalance.
        let run_and_collect_buys = |direction: Direction,
                                    n_bars: usize|
         -> std::collections::BTreeSet<String> {
            let mut strat = make_strategy_with_direction(lookback, rebalance, k_long, direction);
            let mut last_buy_symbols = std::collections::BTreeSet::new();

            for bar_idx in 0..n_bars as i64 {
                let mut signals = Vec::new();
                // Feed all symbols for this timestep.
                for (sym_name, start, delta) in prices_mom {
                    #[allow(clippy::cast_precision_loss)]
                    let price =
                        Decimal::try_from(start + delta * bar_idx as f64).unwrap_or(dec!(1));
                    let bar = make_bar(sym_name, price.max(dec!(0.01)), bar_idx);
                    signals.extend(strat.on_bar(&bar));
                }
                if signals.iter().any(|s| s.kind == SignalKind::Buy) {
                    last_buy_symbols = signals
                        .iter()
                        .filter(|s| s.kind == SignalKind::Buy)
                        .map(|s| s.symbol.to_string())
                        .collect();
                }
            }
            last_buy_symbols
        };

        let mom_buys = run_and_collect_buys(Direction::Momentum, 20);
        let rev_buys = run_and_collect_buys(Direction::Reversion, 20);

        assert!(
            !mom_buys.is_empty(),
            "M-DEV-2: Momentum strategy must have generated Buy signals"
        );
        assert!(
            !rev_buys.is_empty(),
            "M-DEV-2: Reversion strategy must have generated Buy signals"
        );

        // The two sets must be disjoint — Momentum picks top-K, Reversion bottom-K.
        let intersection: std::collections::BTreeSet<&String> =
            mom_buys.intersection(&rev_buys).collect();
        assert!(
            intersection.is_empty(),
            "M-DEV-2: Momentum and Reversion selected-symbol sets MUST be disjoint when K=1 \
             and all 3 symbols have distinct scores. \
             mom_buys={mom_buys:?}, rev_buys={rev_buys:?}, intersection={intersection:?}"
        );

        // Sanity: Momentum should prefer BTCUSDT (uptrend), Reversion should prefer BNBUSDT.
        assert!(
            mom_buys.contains("BTCUSDT"),
            "M-DEV-2: Momentum K=1 should pick BTCUSDT (strongest uptrend). Got: {mom_buys:?}"
        );
        assert!(
            rev_buys.contains("BNBUSDT"),
            "M-DEV-2: Reversion K=1 should pick BNBUSDT (strongest downtrend). Got: {rev_buys:?}"
        );
    }

    // ── M-DEV-5 / R-CARRY.2: Sign-assertion test (MANDATORY, day-1) ──────────

    /// Helper: build a carry strategy with K=1 and a minimal synthetic funding map.
    fn make_carry_strategy_with_funding(
        lookback_settlements: u32,
        funding_map: BTreeMap<(Symbol, Timestamp), Decimal>,
    ) -> MomentumStrategy {
        use crate::cross_sectional::config::ScoreSource;
        let toml = format!(
            r#"
id = "test_carry"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = {lookback_settlements}
rebalance_minutes = {lookback_settlements}
k_long = 1
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
score_source = "funding_carry"
"#
        );
        let mut cfg = crate::cross_sectional::config::CrossSectionalMomentumConfig::from_str(&toml)
            .expect("valid carry config");
        cfg.score_source = ScoreSource::FundingCarry;
        MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
            .with_funding(Some(funding_map))
    }

    /// R-CARRY.2 sign-assertion test (MANDATORY, day-1).
    ///
    /// Universe: BTCUSDT (positive funding = longs pay) + ETHUSDT (negative funding = shorts pay).
    /// K=1, lookback=1 settlement.
    ///
    /// Sign convention (R-CARRY.2):
    ///   - ETHUSDT has negative funding → the LONG side is the PAID side → `carry_score = +|funding|` (top).
    ///   - BTCUSDT has positive funding → the LONG side PAYS → `carry_score = −|funding|` (bottom).
    ///   - With K=1, the strategy MUST select ETHUSDT (the paid-to-be-long name).
    ///
    /// **RED-on-mutation**: if the sign in `carry_score` is flipped (returns `+mean` instead
    /// of `−mean`), BTCUSDT would score higher and be selected — the test fails exactly there.
    #[test]
    fn r_carry2_sign_assertion_longs_negative_funding_name() {
        use time::OffsetDateTime;

        // Build synthetic funding: BTCUSDT = +0.01% (positive), ETHUSDT = −0.01% (negative).
        // We inject funding at ts=0 (bar 0) so it is seen by the first bar.
        let base_ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        let positive_rate = dec!(0.0001); // +0.01% — LONGS pay, we don't want to be long
        let negative_rate = dec!(-0.0001); // −0.01% — SHORTS pay, the LONG side earns

        // funding_map: keyed by (symbol, open_ts); funded at ts=0 so bar-0 sees it.
        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((btc.clone(), base_ts), positive_rate);
        funding_map.insert((eth.clone(), base_ts), negative_rate);

        // L=1: one settlement needed to fill the ring.
        let mut strategy = make_carry_strategy_with_funding(1, funding_map);

        // Drive one bar per symbol at ts=0 so the funding is recorded.
        // rebalance_minutes = 1, lookback_minutes = 1.
        let bar_btc = make_bar("BTCUSDT", dec!(50_000), 0);
        let bar_eth = make_bar("ETHUSDT", dec!(3_000), 0);

        let mut all_buy_signals: Vec<String> = Vec::new();
        for sig in strategy.on_bar(&bar_btc) {
            if sig.kind == SignalKind::Buy {
                all_buy_signals.push(sig.symbol.to_string());
            }
        }
        for sig in strategy.on_bar(&bar_eth) {
            if sig.kind == SignalKind::Buy {
                all_buy_signals.push(sig.symbol.to_string());
            }
        }

        // Must have at least one buy (strategy warmed up with L=1 settlement at bar 0).
        assert!(
            !all_buy_signals.is_empty(),
            "R-CARRY.2: carry strategy must generate at least one Buy after seeing L=1 funding. \
             Got no signals. funding_map keys saw all symbols."
        );

        // K=1: exactly one symbol selected.
        // ETHUSDT (negative funding) MUST be selected — the paid-to-be-long name.
        // If BTCUSDT is selected instead, the sign is WRONG (funding-payer, not harvester).
        assert!(
            all_buy_signals.contains(&"ETHUSDT".to_string()),
            "R-CARRY.2 SIGN VIOLATION: carry strategy with K=1 MUST select ETHUSDT \
             (negative funding = longs are the paid side) but got: {:?}. \
             This means carry_score returns +mean instead of −mean — the sign is flipped \
             and the strategy is a funding-PAYER, not a funding-harvester.",
            all_buy_signals
        );
        assert!(
            !all_buy_signals.contains(&"BTCUSDT".to_string()),
            "R-CARRY.2 SIGN VIOLATION: carry strategy MUST NOT select BTCUSDT \
             (positive funding = longs pay, NOT the paid side). Got: {:?}",
            all_buy_signals
        );
    }

    /// R-CARRY.2 RED-on-mutation proof: the sign-assertion above WOULD fail if
    /// `carry_score` returned `+mean` instead of `−mean`. This test directly verifies
    /// the score ordering: ETHUSDT (negative funding) must have a HIGHER carry_score
    /// than BTCUSDT (positive funding).
    ///
    /// We verify this at the score level so the assertion is independent of K and
    /// the rebalance timing.
    #[test]
    fn r_carry2_carry_score_negative_funding_outscores_positive() {
        use time::OffsetDateTime;

        let base_ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        let positive_rate = dec!(0.0001);
        let negative_rate = dec!(-0.0001);

        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((btc.clone(), base_ts), positive_rate);
        funding_map.insert((eth.clone(), base_ts), negative_rate);

        let mut strat = make_carry_strategy_with_funding(1, funding_map);

        // Drive one bar per symbol so the funding is recorded and ring is full.
        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));

        // Inspect the scores cache directly.
        let btc_score = strat.scores.get(&btc).copied().flatten();
        let eth_score = strat.scores.get(&eth).copied().flatten();

        assert!(
            btc_score.is_some(),
            "R-CARRY.2: BTCUSDT must have a carry score after L=1 settlements"
        );
        assert!(
            eth_score.is_some(),
            "R-CARRY.2: ETHUSDT must have a carry score after L=1 settlements"
        );

        let btc_s = btc_score.unwrap();
        let eth_s = eth_score.unwrap();

        // ETHUSDT (−0.01%) → carry_score = −(−0.0001) = +0.0001
        // BTCUSDT (+0.01%) → carry_score = −(+0.0001) = −0.0001
        // Expected: eth_s > btc_s (positive > negative).
        assert!(
            eth_s > btc_s,
            "R-CARRY.2 SIGN VIOLATION: carry_score(ETHUSDT, negative_funding)={eth_s} \
             must be > carry_score(BTCUSDT, positive_funding)={btc_s}. \
             The sign `−mean` means the most-negative-funding name has the highest score. \
             If this fails, carry_score is returning +mean (the harvest-payer bug)."
        );
        // Exact values: ETHUSDT score should be +0.0001, BTCUSDT should be −0.0001.
        assert_eq!(
            eth_s,
            dec!(0.0001),
            "R-CARRY.2: ETHUSDT score must be +0.0001 (−(−0.0001))"
        );
        assert_eq!(
            btc_s,
            dec!(-0.0001),
            "R-CARRY.2: BTCUSDT score must be −0.0001 (−(+0.0001))"
        );
    }

    /// R-CARRY.6 no-look-ahead test (strategy level).
    ///
    /// At bar with ts=0, only funding settled at-or-before ts=0 is visible.
    /// Funding at ts=1 (the NEXT bar) must NOT affect the carry score at ts=0.
    ///
    /// The funding_map only injects at ts=0 → the score at ts=0 uses ts=0 funding.
    /// A separate strategy with funding injected at ts=1 gets None score at ts=0.
    #[test]
    fn r_carry6_no_look_ahead_strategy_level() {
        use time::OffsetDateTime;

        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        let rate = dec!(-0.0001);

        // Strategy A: funding at ts=0 → score is available at ts=0.
        let mut map_a: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        map_a.insert((btc.clone(), ts0), rate);
        map_a.insert((eth.clone(), ts0), rate);
        let mut strat_a = make_carry_strategy_with_funding(1, map_a);
        strat_a.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat_a.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));
        let score_a = strat_a.scores.get(&btc).copied().flatten();

        // Strategy B: funding at ts=1 only (the future, not yet settled at ts=0).
        let mut map_b: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        map_b.insert((btc.clone(), ts1), rate);
        map_b.insert((eth.clone(), ts1), rate);
        let mut strat_b = make_carry_strategy_with_funding(1, map_b);
        strat_b.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat_b.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));
        let score_b = strat_b.scores.get(&btc).copied().flatten();

        assert!(
            score_a.is_some(),
            "R-CARRY.6: strategy with funding at ts=0 must produce a carry score at ts=0"
        );
        assert!(
            score_b.is_none(),
            "R-CARRY.6 NO-LOOK-AHEAD VIOLATION: strategy with funding only at ts=1 \
             must produce None score at ts=0 (the future funding must not leak). \
             Got: {score_b:?}"
        );
    }

    // ── M-DEV-3: TimeSeriesLongFlat strategy-level tests ─────────────────────

    /// Helper: build a TS-momentum strategy (TimeSeriesLongFlat) for a 2-symbol universe.
    fn make_ts_strategy(
        lookback: u32,
        rebalance: u32,
        entry_threshold: Decimal,
    ) -> MomentumStrategy {
        use crate::cross_sectional::config::{CrossSectionalMomentumConfig, SelectionMode};
        let toml = format!(
            r#"
id = "test_ts"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = {lookback}
rebalance_minutes = {rebalance}
k_long = 2
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
selection_mode = "time_series_long_flat"
"#
        );
        let mut cfg = CrossSectionalMomentumConfig::from_str(&toml).unwrap();
        cfg.selection_mode = SelectionMode::TimeSeriesLongFlat;
        cfg.entry_threshold = entry_threshold;
        MomentumStrategy::from_config(cfg, SmolStr::new("test"))
    }

    /// M-DEV-3 strategy-level (a): TS strategy goes LONG on a clear uptrend
    /// and FLAT on a clear downtrend.
    ///
    /// BTC: uptrend (100→200 over lookback bars) → positive log-return → LONG.
    /// ETH: downtrend (200→100 over lookback bars) → negative log-return → FLAT.
    /// entry_threshold = 0.00 (pure long/flat-on-sign).
    #[test]
    fn m_dev3_ts_long_on_uptrend_flat_on_downtrend() {
        let lookback: u32 = 5;
        let rebalance: u32 = 6; // rebalance once all warmed (at bar 6)
        let mut strat = make_ts_strategy(lookback, rebalance, Decimal::ZERO);

        // Warmup: lookback+1 = 6 bars needed.
        // BTC: strictly up (100→150 in 6 steps).
        // ETH: strictly down (200→150 in 6 steps).
        let btc_prices: Vec<Decimal> = (0..=6u32).map(|i| Decimal::from(100u32 + i * 10)).collect();
        let eth_prices: Vec<Decimal> = (0..=6u32).map(|i| Decimal::from(200u32 - i * 10)).collect();

        let mut buy_signals: Vec<String> = Vec::new();
        let mut sell_signals: Vec<String> = Vec::new();

        for bar_idx in 0..=6i64 {
            let btc_bar = make_bar("BTCUSDT", btc_prices[bar_idx as usize], bar_idx);
            let eth_bar = make_bar("ETHUSDT", eth_prices[bar_idx as usize], bar_idx);
            for sig in strat.on_bar(&btc_bar) {
                if sig.kind == SignalKind::Buy {
                    buy_signals.push(sig.symbol.to_string());
                } else if sig.kind == SignalKind::Sell {
                    sell_signals.push(sig.symbol.to_string());
                }
            }
            for sig in strat.on_bar(&eth_bar) {
                if sig.kind == SignalKind::Buy {
                    buy_signals.push(sig.symbol.to_string());
                } else if sig.kind == SignalKind::Sell {
                    sell_signals.push(sig.symbol.to_string());
                }
            }
        }

        // After warmup, BTC has positive log-return → Buy; ETH has negative → no Buy.
        assert!(
            buy_signals.contains(&"BTCUSDT".to_string()),
            "M-DEV-3: TS strategy must Buy BTCUSDT (uptrend, positive log-return above threshold=0). \
             buy_signals={buy_signals:?}"
        );
        assert!(
            !buy_signals.contains(&"ETHUSDT".to_string()),
            "M-DEV-3: TS strategy must NOT Buy ETHUSDT (downtrend, negative log-return below threshold=0). \
             buy_signals={buy_signals:?}"
        );
    }

    /// M-DEV-3 strategy-level (b): TS strategy defaults-off (SelectionMode = CrossSectionalTopK)
    /// when selection_mode is omitted — the existing top-K behavior is byte-untouched.
    #[test]
    fn m_dev3_default_is_cross_sectional_top_k() {
        // Build a plain (non-TS) strategy and verify it is CrossSectionalTopK by default.
        let strat = make_strategy(5, 6, 2);
        assert_eq!(
            strat.selection_mode,
            SelectionMode::CrossSectionalTopK,
            "M-DEV-3: omitting selection_mode must default to CrossSectionalTopK (anchor-neutral)"
        );
    }

    // ── M-DEV-3 / R-BR.2: BasisReversal sign-assertion test (MANDATORY, day-1) ─

    /// Helper: build a BasisReversal strategy with K=1 and a synthetic basis map.
    ///
    /// The basis arm reuses the `with_funding` injection channel (D-BR.3 channel reuse).
    fn make_basis_reversal_strategy_with_map(
        lookback_bars: u32,
        basis_map: BTreeMap<(Symbol, Timestamp), Decimal>,
    ) -> MomentumStrategy {
        use crate::cross_sectional::config::ScoreSource;
        let toml = format!(
            r#"
id = "test_basis_reversal"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = {lookback_bars}
rebalance_minutes = {lookback_bars}
k_long = 1
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
score_source = "basis_reversal"
"#
        );
        let mut cfg = crate::cross_sectional::config::CrossSectionalMomentumConfig::from_str(&toml)
            .expect("valid basis_reversal config");
        cfg.score_source = ScoreSource::BasisReversal;
        MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
            .with_funding(Some(basis_map))
    }

    /// R-BR.2 sign-assertion test (MANDATORY, day-1).
    ///
    /// Universe: BTCUSDT (HIGH basis = crowded long) + ETHUSDT (LOW basis = least crowded).
    /// K=1, lookback=1 bar.
    ///
    /// Sign convention (R-BR.2):
    ///   - BTCUSDT has HIGH positive basis → crowd → will underperform → should be UNDERWEIGHTED.
    ///   - ETHUSDT has LOW (near-zero or negative) basis → not crowded → will outperform → OVERWEIGHT.
    ///   - `basis_reversal_score = −trailing_mean(basis)` → ETHUSDT scores HIGHER than BTCUSDT.
    ///   - With K=1, the strategy MUST select ETHUSDT (the reversal-favored leg).
    ///
    /// **RED-on-mutation**: if the sign in `basis_reversal_score` is flipped (returns `+mean`
    /// instead of `−mean`), BTCUSDT would score higher and be selected — a basis-MOMENTUM payer.
    /// The test fails exactly there with an explicit message naming the sign flip.
    #[test]
    fn r_br2_sign_assertion_longs_low_basis_name() {
        use time::OffsetDateTime;

        let base_ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        // BTCUSDT: HIGH positive basis (perp rich, crowded long → will underperform).
        // ETHUSDT: LOW (negative) basis (perp cheap → will outperform).
        let high_basis = dec!(0.02); // +2% — clearly crowded, the reversal-short target
        let low_basis = dec!(-0.005); // −0.5% — cheapest perp, the reversal-long target

        // Inject via the funding_map channel (D-BR.3 reuse): keyed by (symbol, open_ts).
        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((btc.clone(), base_ts), high_basis);
        basis_map.insert((eth.clone(), base_ts), low_basis);

        // L=1: one bar needed to fill the ring.
        let mut strategy = make_basis_reversal_strategy_with_map(1, basis_map);

        // Drive one bar per symbol at ts=0 so the basis is recorded.
        let bar_btc = make_bar("BTCUSDT", dec!(50_000), 0);
        let bar_eth = make_bar("ETHUSDT", dec!(3_000), 0);

        let mut all_buy_signals: Vec<String> = Vec::new();
        for sig in strategy.on_bar(&bar_btc) {
            if sig.kind == SignalKind::Buy {
                all_buy_signals.push(sig.symbol.to_string());
            }
        }
        for sig in strategy.on_bar(&bar_eth) {
            if sig.kind == SignalKind::Buy {
                all_buy_signals.push(sig.symbol.to_string());
            }
        }

        // Must have at least one buy (strategy warmed up with L=1 basis bar at bar 0).
        assert!(
            !all_buy_signals.is_empty(),
            "R-BR.2: basis-reversal strategy must generate at least one Buy after seeing L=1 basis bar. \
             Got no signals. Both symbols had basis injected."
        );

        // K=1: the LOW-basis name (ETHUSDT) MUST be selected (the reversal-favored leg).
        // If BTCUSDT is selected instead, the sign is WRONG → basis-MOMENTUM payer.
        assert!(
            all_buy_signals.contains(&"ETHUSDT".to_string()),
            "R-BR.2 SIGN VIOLATION: basis-reversal strategy with K=1 MUST select ETHUSDT \
             (low/negative basis = reversal-favored leg) but got: {:?}. \
             This means basis_reversal_score returns +mean instead of −mean — the sign is \
             FLIPPED and the strategy is a basis-MOMENTUM payer (longs crowded-long names \
             that subsequently underperform). FIX: ensure `Some(-mean)` in basis_reversal_score.",
            all_buy_signals
        );
        assert!(
            !all_buy_signals.contains(&"BTCUSDT".to_string()),
            "R-BR.2 SIGN VIOLATION: basis-reversal strategy MUST NOT select BTCUSDT \
             (high positive basis = crowded long = reversal-short target). Got: {:?}",
            all_buy_signals
        );
    }

    /// R-BR.2 score-level assertion: ETHUSDT (low basis) must outscored BTCUSDT (high basis).
    ///
    /// Verifies the score ordering independently of K and rebalance timing.
    /// This is the exact `−mean` verification: `score(low basis) > score(high basis)`.
    #[test]
    fn r_br2_basis_reversal_score_low_basis_outscores_high_basis() {
        use time::OffsetDateTime;

        let base_ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        let high_basis = dec!(0.02); // +2% — high → −mean = −0.02 → LOWER score
        let low_basis = dec!(-0.005); // −0.5% — low → −mean = +0.005 → HIGHER score

        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((btc.clone(), base_ts), high_basis);
        basis_map.insert((eth.clone(), base_ts), low_basis);

        let mut strat = make_basis_reversal_strategy_with_map(1, basis_map);

        // Drive one bar per symbol so the basis is recorded and ring is full.
        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));

        // Inspect the scores cache directly.
        let btc_score = strat.scores.get(&btc).copied().flatten();
        let eth_score = strat.scores.get(&eth).copied().flatten();

        assert!(
            btc_score.is_some(),
            "R-BR.2: BTCUSDT must have a basis-reversal score after L=1 bars"
        );
        assert!(
            eth_score.is_some(),
            "R-BR.2: ETHUSDT must have a basis-reversal score after L=1 bars"
        );

        let btc_s = btc_score.unwrap();
        let eth_s = eth_score.unwrap();

        // BTCUSDT (basis=+0.02) → basis_reversal_score = −(+0.02) = −0.02
        // ETHUSDT (basis=−0.005) → basis_reversal_score = −(−0.005) = +0.005
        // Expected: eth_s > btc_s (positive > negative).
        assert!(
            eth_s > btc_s,
            "R-BR.2 SIGN VIOLATION: basis_reversal_score(ETHUSDT, low_basis={low_basis})={eth_s} \
             must be > basis_reversal_score(BTCUSDT, high_basis={high_basis})={btc_s}. \
             The sign `−mean` means the lowest-basis name has the highest score. \
             If this fails, basis_reversal_score is returning +mean (the basis-MOMENTUM bug)."
        );
        // Exact values: ETHUSDT score should be +0.005, BTCUSDT should be -0.02.
        assert_eq!(
            eth_s,
            dec!(0.005),
            "R-BR.2: ETHUSDT score must be +0.005 (−(−0.005))"
        );
        assert_eq!(
            btc_s,
            dec!(-0.02),
            "R-BR.2: BTCUSDT score must be −0.02 (−(+0.02))"
        );
    }

    /// Review 1-20 M: the trailing-mean ring at a PRODUCTION-realistic `L`.
    ///
    /// Every other basis test in the tree runs `L = 1`, where
    /// `basis_reversal_score` degenerates: `sum/len` over a 1-element ring is
    /// the identity, and the `while ring.len() > L { pop_front() }` eviction
    /// loop is exercised only in its most trivial form. The anchored production
    /// grid runs `L ∈ {24, 60, 168}` (§ D-BR.2-LOCKED), so the averaging and
    /// eviction that actually price the surface had NO coverage at all.
    ///
    /// This test runs `L = 24` over 48 bars with a REGIME FLIP at bar 24 and
    /// asserts three separate things the `L = 1` tests cannot see:
    ///
    /// 1. **Warm-up counts bars, not settlements** — `None` at bar 22 (ring has
    ///    23 of 24), `Some` at bar 23 (ring full).
    /// 2. **The mean is a real trailing mean, not the last value** — measured
    ///    mid-flip at bar 30, where 17 pre-flip and 7 post-flip values are in
    ///    the ring and the score differs from `−last_value`.
    /// 3. **The ring EVICTS** — at bar 47 the ring must hold bars 24..=47 ONLY,
    ///    so the score is exactly `−(post-flip value)`. If the eviction loop
    ///    regressed (ring grows unbounded), the score would be the mean over
    ///    all 48 bars instead, and the cross-sectional ORDER would flip too:
    ///    BTCUSDT would still outscore ETHUSDT, so this also catches a
    ///    selection-level regression, not just an arithmetic one.
    #[test]
    fn r_br_trailing_mean_ring_at_production_lookback() {
        use time::OffsetDateTime;

        const L: u32 = 24; // the smallest production rung (§ D-BR.2-LOCKED)
        const N_BARS: i64 = 48;
        const FLIP_BAR: i64 = 24;

        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        // BTCUSDT: −0.02 for bars 0..23, then +0.01 for bars 24..47.
        // ETHUSDT: +0.01 for bars 0..23, then −0.005 for bars 24..47.
        // The cross-sectional winner therefore FLIPS from BTC to ETH at the
        // moment the ring has fully rolled over — but only if it evicts.
        let btc_pre = dec!(-0.02);
        let btc_post = dec!(0.01);
        let eth_pre = dec!(0.01);
        let eth_post = dec!(-0.005);

        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        for i in 0..N_BARS {
            let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(i));
            let (b, e) = if i < FLIP_BAR {
                (btc_pre, eth_pre)
            } else {
                (btc_post, eth_post)
            };
            basis_map.insert((btc.clone(), ts), b);
            basis_map.insert((eth.clone(), ts), e);
        }

        let mut strat = make_basis_reversal_strategy_with_map(L, basis_map);

        // ── (1) Warm-up: the ring must hold L bars before ANY score exists ────
        for i in 0..23_i64 {
            strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), i));
            strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), i));
        }
        assert_eq!(
            strat.scores.get(&btc).copied().flatten(),
            None,
            "L={L}: after 23 bars the ring holds 23 < {L} values — the score must \
             still be None (warm-up counts BARS)"
        );

        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 23));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 23));
        assert_eq!(
            strat.scores.get(&btc).copied().flatten(),
            Some(-btc_pre),
            "L={L}: at bar 23 the ring is exactly full of the pre-flip value, so \
             the score is −mean = −({btc_pre})"
        );

        // ── (2) Mid-flip: the mean must differ from −last_value ───────────────
        for i in 24..=30_i64 {
            strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), i));
            strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), i));
        }
        // Ring now holds bars 7..=30: 17 pre-flip values and 7 post-flip ones.
        let expected_mid = -((Decimal::from(17_u32) * btc_pre + Decimal::from(7_u32) * btc_post)
            / Decimal::from(24_u32));
        let mid = strat
            .scores
            .get(&btc)
            .copied()
            .flatten()
            .expect("score must exist once warm");
        assert_eq!(
            mid, expected_mid,
            "L={L}: at bar 30 the ring holds 17×{btc_pre} + 7×{btc_post}; the score \
             must be the −MEAN of that window"
        );
        assert_ne!(
            mid, -btc_post,
            "L={L} VACUITY GUARD: the score must NOT equal −last_value. If it does, \
             the trailing mean has collapsed to the identity and every L>1 cell on \
             the anchored grid is silently running L=1"
        );

        // ── (3) Full roll-over: the ring must have EVICTED every pre-flip bar ─
        for i in 31..N_BARS {
            strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), i));
            strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), i));
        }
        let btc_end = strat
            .scores
            .get(&btc)
            .copied()
            .flatten()
            .expect("BTCUSDT score");
        let eth_end = strat
            .scores
            .get(&eth)
            .copied()
            .flatten()
            .expect("ETHUSDT score");

        // If eviction works the ring holds bars 24..=47 only → the mean is the
        // post-flip constant. If it does NOT, the ring holds all 48 bars and
        // the score would be −(24·pre + 24·post)/48 = +0.005 for BTC.
        assert_eq!(
            btc_end, -btc_post,
            "L={L} EVICTION REGRESSION: after 24 post-flip bars the ring must hold \
             bars 24..=47 ONLY, so the score is −({btc_post}). Getting −mean over all \
             48 bars instead means `while ring.len() > funding_lookback` stopped \
             evicting and the window grows without bound"
        );
        assert_eq!(
            eth_end, -eth_post,
            "L={L} EVICTION REGRESSION: ETHUSDT score must be −({eth_post}) after full \
             roll-over"
        );

        // The cross-sectional ORDER must have flipped with the regime. Under a
        // broken eviction the un-evicted means are BTC +0.005 vs ETH −0.0025,
        // i.e. BTC still wins — so this assertion is RED on that regression too.
        assert!(
            eth_end > btc_end,
            "L={L}: after the regime flip ETHUSDT (now the LOW-basis name) must \
             outscore BTCUSDT. eth={eth_end}, btc={btc_end}. If BTCUSDT still wins, \
             the ring never evicted the pre-flip regime and the arm is trading a \
             stale window"
        );
    }

    // ── Review 1-20 wave-2 L: zero-lookback must not PANIC ───────────────────
    //
    // `funding_lookback = cfg.lookback_minutes as usize`. The TOML loader
    // rejects `lookback_minutes < 1` (`config.rs`: `InvalidLookback`), but
    // `CrossSectionalMomentumConfig` has all-`pub` fields, so a struct literal
    // reaches `MomentumStrategy::from_config` with `lookback_minutes: 0` and
    // never sees that check — and the e2e suites build their configs exactly
    // that way. At `lookback_minutes == 0` the ring is pushed and then drained
    // empty by `while ring.len() > 0`, `0 < 0` is false, and the trailing mean
    // divides by `Decimal::from(0)` → panic. A panic in library code is a
    // CLAUDE.md violation. Each arm must return `None` instead.
    //
    // Each test below PANICS on the un-guarded code and passes on the guarded
    // code; there is one per score arm because each arm has its own division.

    /// Build a config by STRUCT LITERAL — the seam that bypasses the TOML
    /// validation — with an arbitrary lookback and score source.
    fn struct_literal_cfg(
        lookback: u32,
        score_source: crate::cross_sectional::config::ScoreSource,
    ) -> crate::cross_sectional::config::CrossSectionalMomentumConfig {
        crate::cross_sectional::config::CrossSectionalMomentumConfig {
            id: smol_str::SmolStr::new("zero_lookback_probe"),
            universe: vec![
                smol_str::SmolStr::new("BTCUSDT"),
                smol_str::SmolStr::new("ETHUSDT"),
            ],
            lookback_minutes: lookback,
            rebalance_minutes: 1,
            k_long: 1,
            k_short: 0,
            exposure_cap: dec!(0.5),
            drift_rebalance_threshold: dec!(0.10),
            vol_floor: dec!(0.000001),
            stage: smol_str::SmolStr::new("research"),
            direction: crate::cross_sectional::config::Direction::Momentum,
            score_source,
            selection_mode: crate::cross_sectional::config::SelectionMode::CrossSectionalTopK,
            entry_threshold: Decimal::ZERO,
        }
    }

    /// A sidecar map with a value for both symbols at bar 0, so the ring IS
    /// pushed (and therefore drained empty when the lookback is 0).
    fn two_symbol_sidecar(value: Decimal) -> BTreeMap<(Symbol, Timestamp), Decimal> {
        use time::OffsetDateTime;
        let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let mut m: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        m.insert((Symbol::new("BTCUSDT"), ts), value);
        m.insert((Symbol::new("ETHUSDT"), ts), value);
        m
    }

    /// BASIS arm: `lookback_minutes = 0` must return `None`, not divide by zero.
    #[test]
    fn zero_lookback_basis_arm_returns_none_instead_of_panicking() {
        use crate::cross_sectional::config::ScoreSource;

        let cfg = struct_literal_cfg(0, ScoreSource::BasisReversal);
        let mut strat = MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
            .with_funding(Some(two_symbol_sidecar(dec!(-0.005))));

        // Drives basis_reversal_score for both symbols. UN-GUARDED: panics with
        // "Division by zero" inside `sum / Decimal::from(ring.len())`.
        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));

        assert_eq!(
            strat.scores.get(&Symbol::new("BTCUSDT")).copied().flatten(),
            None,
            "at lookback 0 the basis ring drains empty, so there is no trailing mean to \
             report — the score must be None (never warm), NOT a division by zero"
        );
    }

    /// CARRY arm: same seam, same requirement.
    #[test]
    fn zero_lookback_carry_arm_returns_none_instead_of_panicking() {
        use crate::cross_sectional::config::ScoreSource;

        let cfg = struct_literal_cfg(0, ScoreSource::FundingCarry);
        let mut strat = MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
            .with_funding(Some(two_symbol_sidecar(dec!(0.0001))));

        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));

        assert_eq!(
            strat.scores.get(&Symbol::new("BTCUSDT")).copied().flatten(),
            None,
            "at lookback 0 the carry settlement ring drains empty — the score must be None"
        );
    }

    /// RESIDUAL arm: three separate divisions live on this path
    /// (`basis_trailing_mean_for_residual` plus the two inline `filter_map`
    /// closures that build the rank vectors). Driving `on_bar` exercises all of
    /// them.
    #[test]
    fn zero_lookback_residual_arm_returns_none_instead_of_panicking() {
        use crate::cross_sectional::config::ScoreSource;

        let cfg = struct_literal_cfg(0, ScoreSource::BasisFundingResidual);
        let mut strat = MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
            .with_funding(Some(two_symbol_sidecar(dec!(0.0001))))
            .with_basis_score(Some(two_symbol_sidecar(dec!(-0.005))));

        strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));

        assert_eq!(
            strat.scores.get(&Symbol::new("BTCUSDT")).copied().flatten(),
            None,
            "at lookback 0 both residual rings drain empty — the score must be None"
        );
    }

    /// The guard must be a strict NO-OP at every production lookback: the
    /// anchored grid runs L ∈ {24, 60, 168} and the `is_empty()` disjunct can
    /// never fire there (a ring entry exists only after a push, and a push
    /// leaves at least one element when the lookback is >= 1).
    #[test]
    fn zero_lookback_guard_is_a_no_op_at_production_lookbacks() {
        use crate::cross_sectional::config::ScoreSource;
        use time::OffsetDateTime;

        for lookback in [1_u32, 24, 60, 168] {
            let cfg = struct_literal_cfg(lookback, ScoreSource::BasisReversal);
            let mut map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
            for i in 0..i64::from(lookback) {
                let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(i));
                map.insert((Symbol::new("BTCUSDT"), ts), dec!(-0.005));
                map.insert((Symbol::new("ETHUSDT"), ts), dec!(0.02));
            }
            let mut strat = MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test"))
                .with_funding(Some(map));
            for i in 0..i64::from(lookback) {
                strat.on_bar(&make_bar("BTCUSDT", dec!(50_000), i));
                strat.on_bar(&make_bar("ETHUSDT", dec!(3_000), i));
            }
            assert_eq!(
                strat.scores.get(&Symbol::new("BTCUSDT")).copied().flatten(),
                Some(dec!(0.005)),
                "L={lookback}: once the ring is full the score is unchanged by the \
                 division guard (−mean of a constant −0.005 series)"
            );
        }
    }

    /// R-BR.7 #5 no-look-ahead test (strategy level, M-DEV-3).
    ///
    /// At bar with ts=0, only basis settled at-or-before ts=0 is visible.
    /// Basis at ts=1 (the NEXT bar) must NOT affect the score at ts=0.
    ///
    /// Mirrors `r_carry6_no_look_ahead_strategy_level` exactly for BasisReversal.
    #[test]
    fn r_br5_no_look_ahead_strategy_level() {
        use time::OffsetDateTime;

        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let ts1 = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1));
        let btc = Symbol::new("BTCUSDT");
        let eth = Symbol::new("ETHUSDT");

        let basis_val = dec!(-0.005);

        // Strategy A: basis at ts=0 → score is available at ts=0.
        let mut map_a: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        map_a.insert((btc.clone(), ts0), basis_val);
        map_a.insert((eth.clone(), ts0), basis_val);
        let mut strat_a = make_basis_reversal_strategy_with_map(1, map_a);
        strat_a.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat_a.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));
        let score_a = strat_a.scores.get(&btc).copied().flatten();

        // Strategy B: basis at ts=1 only (the future, not yet settled at ts=0).
        let mut map_b: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        map_b.insert((btc.clone(), ts1), basis_val);
        map_b.insert((eth.clone(), ts1), basis_val);
        let mut strat_b = make_basis_reversal_strategy_with_map(1, map_b);
        strat_b.on_bar(&make_bar("BTCUSDT", dec!(50_000), 0));
        strat_b.on_bar(&make_bar("ETHUSDT", dec!(3_000), 0));
        let score_b = strat_b.scores.get(&btc).copied().flatten();

        assert!(
            score_a.is_some(),
            "R-BR.5: strategy with basis at ts=0 must produce a score at ts=0"
        );
        assert!(
            score_b.is_none(),
            "R-BR.5 NO-LOOK-AHEAD VIOLATION: strategy with basis only at ts=1 \
             must produce None score at ts=0 (future basis must not leak). \
             Got: {score_b:?}"
        );
    }

    /// M-DEV-3 strategy-level (c): TS strategy with high entry_threshold stays flat
    /// even on a moderate uptrend (the wide-band / no-trade-zone behavior).
    #[test]
    fn m_dev3_ts_wide_band_stays_flat_on_moderate_trend() {
        let lookback: u32 = 4;
        let rebalance: u32 = 5;
        // entry_threshold = 0.50 (50% log-return required — very wide band).
        let entry_threshold = dec!(0.50);
        let mut strat = make_ts_strategy(lookback, rebalance, entry_threshold);

        // BTC: modest uptrend (100→110 over 4 bars → log-ret ≈ 9.5% < 50%).
        let btc_prices: Vec<Decimal> = (0..=5u32)
            .map(|i| Decimal::from(100u32 + i * 2)) // 100, 102, 104, 106, 108, 110
            .collect();
        let eth_prices: Vec<Decimal> = btc_prices.clone(); // same — both modest uptrend

        let mut buy_signals: Vec<String> = Vec::new();

        for bar_idx in 0..=5i64 {
            let btc_bar = make_bar("BTCUSDT", btc_prices[bar_idx as usize], bar_idx);
            let eth_bar = make_bar("ETHUSDT", eth_prices[bar_idx as usize], bar_idx);
            for sig in strat.on_bar(&btc_bar) {
                if sig.kind == SignalKind::Buy {
                    buy_signals.push(sig.symbol.to_string());
                }
            }
            for sig in strat.on_bar(&eth_bar) {
                if sig.kind == SignalKind::Buy {
                    buy_signals.push(sig.symbol.to_string());
                }
            }
        }

        // With a 50% threshold, a 10% uptrend should NOT generate Buy signals.
        assert!(
            buy_signals.is_empty(),
            "M-DEV-3: TS strategy with entry_threshold=0.50 must stay flat on a ~10% uptrend. \
             buy_signals={buy_signals:?}"
        );
    }

    // ── M-DEV-4: BasisFundingResidual rank-residual unit tests ────────────────

    /// Build a `BasisFundingResidual` strategy with injected basis and funding maps.
    fn make_residual_strategy_with_maps(
        lookback: u32,
        k_long: u32,
        k_short: u32,
        basis_score_map: BTreeMap<(Symbol, Timestamp), Decimal>,
        funding_map: BTreeMap<(Symbol, Timestamp), Decimal>,
    ) -> MomentumStrategy {
        use crate::cross_sectional::config::CrossSectionalMomentumConfig;
        let mut cfg = CrossSectionalMomentumConfig::from_str(&format!(
            r#"
id = "test_residual"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["AAUSDT", "BBUSDT", "CCUSDT"]
lookback_minutes = {lookback}
rebalance_minutes = 1
k_long = {k_long}
k_short = {k_short}
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
selection_mode = "long_short"
"#
        ))
        .unwrap();
        cfg.score_source = crate::cross_sectional::config::ScoreSource::BasisFundingResidual;
        MomentumStrategy::from_config(cfg, SmolStr::new("test_residual"))
            .with_funding(Some(funding_map))
            .with_basis_score(Some(basis_score_map))
    }

    /// M-DEV-4 (a): Rank-residual is integer-valued Decimal, NO division.
    ///
    /// With 3 symbols and distinct basis/funding ranks, the residual must be
    /// an exact integer Decimal (1, 0, -1 etc.), not a fraction.
    #[test]
    fn m_dev4_rank_residual_is_integer_valued() {
        use time::OffsetDateTime;
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let aaa = Symbol::new("AAUSDT");
        let bbb = Symbol::new("BBUSDT");
        let ccc = Symbol::new("CCUSDT");

        // basis: AA=−0.02 (rank 1 high score after −mean), BB=0.00, CC=+0.02 (rank 3)
        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((aaa.clone(), ts0), dec!(-0.02));
        basis_map.insert((bbb.clone(), ts0), dec!(0.00));
        basis_map.insert((ccc.clone(), ts0), dec!(0.02));

        // funding: AA=−0.01 (rank 1 high after −mean), BB=0.00, CC=+0.01 (rank 3)
        // Same ordering → residual should be 0 for all.
        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((aaa.clone(), ts0), dec!(-0.01));
        funding_map.insert((bbb.clone(), ts0), dec!(0.00));
        funding_map.insert((ccc.clone(), ts0), dec!(0.01));

        let mut strat =
            make_residual_strategy_with_maps(1, 1, 1, basis_map.clone(), funding_map.clone());
        let prices = [
            ("AAUSDT", dec!(100)),
            ("BBUSDT", dec!(200)),
            ("CCUSDT", dec!(300)),
        ];
        for (sym, p) in &prices {
            strat.on_bar(&make_bar(sym, *p, 0));
        }
        let residuals = strat.build_residual_scores();
        for (sym, score_opt) in &residuals {
            if let Some(score) = score_opt {
                // Residual must be an integer-valued Decimal (no fractional part).
                assert_eq!(
                    *score,
                    score.round(),
                    "M-DEV-4: residual for {sym} must be integer-valued, got {score}"
                );
            }
        }
    }

    /// M-DEV-4 (b): Residual differs from raw basis when funding ranking diverges.
    ///
    /// When the funding rank differs from the basis rank, the residual score differs
    /// from the raw basis-reversal score — proving the residualization is non-trivial.
    #[test]
    fn m_dev4_residual_differs_from_raw_basis_when_funding_diverges() {
        use time::OffsetDateTime;
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let aaa = Symbol::new("AAUSDT");
        let bbb = Symbol::new("BBUSDT");
        let ccc = Symbol::new("CCUSDT");

        // basis: AAUSDT=−0.02 (rank 1, lowest), BBUSDT=0.00, CCUSDT=+0.02 (rank 3, highest)
        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((aaa.clone(), ts0), dec!(-0.02));
        basis_map.insert((bbb.clone(), ts0), dec!(0.00));
        basis_map.insert((ccc.clone(), ts0), dec!(0.02));

        // funding: CCUSDT=−0.03 (most negative → rank 1 after −mean), BBUSDT=0.00, AAUSDT=+0.03
        // INVERTED funding vs basis order — the residual should reveal different ranking.
        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((aaa.clone(), ts0), dec!(0.03)); // high funding → PAYS → low carry score
        funding_map.insert((bbb.clone(), ts0), dec!(0.00));
        funding_map.insert((ccc.clone(), ts0), dec!(-0.03)); // negative → EARNS → high carry score

        let mut strat = make_residual_strategy_with_maps(1, 1, 1, basis_map, funding_map);
        let prices = [
            ("AAUSDT", dec!(100)),
            ("BBUSDT", dec!(200)),
            ("CCUSDT", dec!(300)),
        ];
        for (sym, p) in &prices {
            strat.on_bar(&make_bar(sym, *p, 0));
        }

        let residuals = strat.build_residual_scores();

        // AA: basis rank 1 (most neg → high score), funding rank 3 (most pos → low score)
        //   → residual = rank(funding)3 − rank(basis)1 = +2
        let aa_res = residuals.get(&aaa).copied().flatten();
        // CC: basis rank 3 (most pos → low score), funding rank 1 (most neg → high score)
        //   → residual = rank(funding)1 − rank(basis)3 = −2
        let cc_res = residuals.get(&ccc).copied().flatten();

        assert!(
            aa_res.is_some() && cc_res.is_some(),
            "M-DEV-4: both AAUSDT and CCUSDT must have residual scores after warm-up"
        );
        // The residual ranking must differ from the raw basis ranking.
        // Raw basis: AA (rank 1) > BB > CC (rank 3), so long AA short CC.
        // Residual: CC (residual +2) > BB > AA (residual −2), so long CC short AA.
        //
        // ── REWRITTEN AT THE 1-25 RE-LOCK (bug-log #76) ─────────────────────
        // This block previously asserted the INVERTED direction and carried a note
        // saying its own prose was "FACTUALLY BACKWARDS for this fixture" — left
        // uncorrected so #76's evidence stayed quotable, and explicitly scheduled to
        // be "rewritten at the 1-25 re-lock, with the code". The code is fixed
        // (`residual = rank(funding) - rank(basis)`), so this is that rewrite: the
        // assertions now state the CORRECT direction and the prose matches the fixture.
        assert!(
            aa_res.unwrap() > cc_res.unwrap(),
            "M-DEV-4 (#76 FIXED): AAUSDT (LOWEST basis) must out-rank CCUSDT (HIGHEST) on the residual. \
             CC residual={cc_res:?}, AA residual={aa_res:?}"
        );
        assert!(
            aa_res.unwrap() > dec!(0),
            "M-DEV-4 (#76 FIXED): AAUSDT has the LOWEST basis (-0.02) and the WORST funding (+0.03, it pays) \
             -> its basis beats what funding predicts -> residual must be POSITIVE (got {aa_res:?})"
        );
        assert!(
            cc_res.unwrap() < dec!(0),
            "M-DEV-4 (#76 FIXED): CCUSDT has the HIGHEST basis (+0.02) and the BEST funding (-0.03, it earns) \
             -> its basis is worse than funding predicts -> residual must be NEGATIVE (got {cc_res:?})"
        );
    }

    /// M-DEV-4 (c): Two-run identity — same inputs → identical residual scores.
    #[test]
    fn m_dev4_residual_two_run_identity() {
        use time::OffsetDateTime;
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let aaa = Symbol::new("AAUSDT");
        let bbb = Symbol::new("BBUSDT");
        let ccc = Symbol::new("CCUSDT");

        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((aaa.clone(), ts0), dec!(-0.02));
        basis_map.insert((bbb.clone(), ts0), dec!(0.01));
        basis_map.insert((ccc.clone(), ts0), dec!(0.03));

        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((aaa.clone(), ts0), dec!(0.02));
        funding_map.insert((bbb.clone(), ts0), dec!(-0.01));
        funding_map.insert((ccc.clone(), ts0), dec!(-0.03));

        let run_residuals = || {
            let mut strat =
                make_residual_strategy_with_maps(1, 1, 1, basis_map.clone(), funding_map.clone());
            let prices = [
                ("AAUSDT", dec!(100)),
                ("BBUSDT", dec!(200)),
                ("CCUSDT", dec!(300)),
            ];
            for (sym, p) in &prices {
                strat.on_bar(&make_bar(sym, *p, 0));
            }
            strat.build_residual_scores()
        };

        let r1 = run_residuals();
        let r2 = run_residuals();
        assert_eq!(
            r1, r2,
            "M-DEV-4: two runs on the same input must produce byte-identical residual scores"
        );
    }

    /// **DOCUMENTS A KNOWN DEFECT — bug-log #76. MUST BE INVERTED AT THE 1-25 RE-LOCK.**
    ///
    /// # Intended behaviour (what `build_residual_scores`' own doc promises)
    ///
    /// > "Long = highest residual (**low-basis** RELATIVE to its funding level).
    /// >  Short = lowest residual (high-basis relative to its funding level)."
    ///
    /// The same claim appears on `ScoreSource::BasisFundingResidual` in `config.rs`. Under
    /// that intent, the arm should LONG the name whose basis is LOW for its funding level.
    ///
    /// # Actual behaviour today (what this test pins, with literal values)
    ///
    /// The basis half is ranked with `rank = 1` for the HIGHEST basis-reversal score,
    /// i.e. the LOWEST basis. `residual = rank(basis) − rank(funding)`, and `top_k_long`
    /// takes the HIGHEST residual — which needs a LARGE `rank(basis)`, i.e. a HIGH basis.
    /// So the arm longs the **highest-basis** name: the basis axis is inverted relative to
    /// the specification, and to the long-only `BasisReversal` arm built from the same
    /// convention.
    ///
    /// Fixture (the story's own numbers, from
    /// `m_dev4_residual_differs_from_raw_basis_when_funding_diverges`):
    ///
    /// | sym | basis  | rank(basis) | funding | rank(funding) | residual |
    /// |-----|--------|-------------|---------|---------------|----------|
    /// | AA  | −0.02  | 1 (lowest basis)  | +0.03 | 3 | **−2** |
    /// | CC  | +0.02  | 3 (highest basis) | −0.03 | 1 | **+2** |
    ///
    /// `top_k_long` ⇒ **CC**, whose basis is +0.02, the HIGHEST in the cross-section.
    /// Plain `BasisReversal` on the identical basis values longs **AA**. The two arms
    /// disagree on the basis axis by construction, not by residualisation.
    ///
    /// This is asserted on VALUES, in the shape of the long-only arm's sign guards
    /// (`r_br2_sign_assertion_longs_low_basis_name`), because every existing residual test
    /// asserts a DIFFERENCE — and a difference is satisfied just as well by the inverse as
    /// by the intended construction. That is precisely why the inversion survived.
    ///
    /// # At the 1-25 re-lock
    ///
    /// Decide the intended direction, make code and doc agree, then INVERT this test
    /// (long = lowest-basis-for-its-funding = AAUSDT here) and re-run anchors #116-#119.
    /// Do not delete it — the flip is the record that the direction was chosen, not
    /// inherited.
    #[test]
    fn direction_gate_bug76_residual_arm_longs_the_lowest_basis_name() {
        use time::OffsetDateTime;
        let ts0 = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let aaa = Symbol::new("AAUSDT");
        let bbb = Symbol::new("BBUSDT");
        let ccc = Symbol::new("CCUSDT");

        // basis: AA = −0.02 (LOWEST basis), BB = 0.00, CC = +0.02 (HIGHEST basis)
        let mut basis_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        basis_map.insert((aaa.clone(), ts0), dec!(-0.02));
        basis_map.insert((bbb.clone(), ts0), dec!(0.00));
        basis_map.insert((ccc.clone(), ts0), dec!(0.02));

        // funding: inverted versus the basis order, so the residual is non-degenerate.
        let mut funding_map: BTreeMap<(Symbol, Timestamp), Decimal> = BTreeMap::new();
        funding_map.insert((aaa.clone(), ts0), dec!(0.03));
        funding_map.insert((bbb.clone(), ts0), dec!(0.00));
        funding_map.insert((ccc.clone(), ts0), dec!(-0.03));

        let mut strat =
            make_residual_strategy_with_maps(1, 1, 1, basis_map.clone(), funding_map.clone());
        let prices = [
            ("AAUSDT", dec!(100)),
            ("BBUSDT", dec!(200)),
            ("CCUSDT", dec!(300)),
        ];
        for (sym, p) in &prices {
            strat.on_bar(&make_bar(sym, *p, 0));
        }

        let residuals = strat.build_residual_scores();
        let aa_res = residuals.get(&aaa).copied().flatten().expect("AA warmed");
        let cc_res = residuals.get(&ccc).copied().flatten().expect("CC warmed");

        // LITERAL residual values — not an inequality (bug-log #76's moral: every
        // pre-fix residual test asserted only DIFFERENCE, which the inverted arm
        // satisfied just as well). These are the values AFTER the direction fix.
        assert_eq!(
            aa_res,
            dec!(2),
            "DIRECTION GATE (#76 FIXED): AAUSDT (basis −0.02, the LOWEST) must score \
             residual = rank(funding)3 − rank(basis)1 = +2 — the strongest LONG. Got {aa_res}. \
             If this reads −2 the subtraction order regressed to rank(basis) − rank(funding)."
        );
        assert_eq!(
            cc_res,
            dec!(-2),
            "DIRECTION GATE (#76 FIXED): CCUSDT (basis +0.02, the HIGHEST) must score \
             residual = rank(funding)1 − rank(basis)3 = −2 — the strongest SHORT. Got {cc_res}."
        );

        // The selection that follows from those values, asserted through the PUBLIC
        // selector the strategy actually calls — so this pins behaviour, not arithmetic.
        let longs = crate::cross_sectional::selector::top_k_long(&residuals, 1, dec!(0.5));
        let shorts = crate::cross_sectional::selector::bottom_k_short(&residuals, 1, dec!(0.5));
        assert!(
            longs.contains_key(&aaa),
            "DIRECTION GATE (#76 FIXED): the residual arm must LONG AAUSDT — the LOWEST-basis \
             name (−0.02) — matching config.rs:85's contract 'highest residual → long (lowest \
             basis relative to funding)'. longs={longs:?}. Before the fix it longed CCUSDT, \
             the HIGHEST-basis name; if that returns, anchors #116-#119 are invalid again."
        );
        assert!(
            shorts.contains_key(&ccc),
            "DIRECTION GATE (#76 FIXED): the residual arm must SHORT CCUSDT — the HIGHEST-basis \
             name (+0.02). shorts={shorts:?}"
        );

        // The contrast that makes it a defect rather than a choice: the long-only
        // BasisReversal arm, on the SAME basis values, longs the opposite name.
        let basis_scores: BTreeMap<Symbol, Option<Decimal>> = [
            (aaa.clone(), Some(dec!(0.02))),  // −mean(−0.02)
            (bbb.clone(), Some(dec!(0.00))),  // −mean(0.00)
            (ccc.clone(), Some(dec!(-0.02))), // −mean(+0.02)
        ]
        .into_iter()
        .collect();
        let basis_longs = crate::cross_sectional::selector::top_k_long(&basis_scores, 1, dec!(0.5));
        assert!(
            basis_longs.contains_key(&aaa),
            "control: plain BasisReversal longs AAUSDT (the LOWEST basis). AFTER the #76 fix \
             the residual arm AGREES with it — both long AAUSDT. That agreement IS the gate: \
             the two arms must not disagree on the sign of the basis axis itself. \
             basis_longs={basis_longs:?}"
        );
    }

    /// M-DEV-4 (d): Config hash includes BasisFundingResidual distinctly from BasisReversal.
    #[test]
    fn m_dev4_config_hash_basis_funding_residual_distinct() {
        use crate::cross_sectional::config::CrossSectionalMomentumConfig;

        let make_cfg = |ss: crate::cross_sectional::config::ScoreSource| {
            let mut cfg = CrossSectionalMomentumConfig::from_str(
                r#"
id = "test_hash"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["AAUSDT", "BBUSDT", "CCUSDT"]
lookback_minutes = 60
rebalance_minutes = 480
k_long = 3
k_short = 3
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
selection_mode = "long_short"
"#,
            )
            .unwrap();
            cfg.score_source = ss;
            MomentumStrategy::from_config(cfg, SmolStr::new("test"))
        };

        let strat_basis = make_cfg(crate::cross_sectional::config::ScoreSource::BasisReversal);
        let strat_residual =
            make_cfg(crate::cross_sectional::config::ScoreSource::BasisFundingResidual);

        assert_ne!(
            strat_basis.hash, strat_residual.hash,
            "M-DEV-4: BasisReversal and BasisFundingResidual must hash differently (K3)"
        );
    }

    /// Review 1-16: config-hash forward-continuity pin.
    ///
    /// Pins the EXACT hash of the canonical momentum default — the field values
    /// of the one checked-in production TOML
    /// (`config/strategies/top10_momentum_h1.toml`) — as computed TODAY (post
    /// the 1-16 `;direction=` + M-DEV-5/M-DEV-1 domain appends). The hash flows
    /// into strategy lifecycle events and the agent watcher's reload identity,
    /// so silently extending `compute_config_hash`'s canonical string re-keys
    /// every stored identity (the 1-16 migration did exactly that for all
    /// pre-1-16 configs, without a note). If this test fails, you are migrating
    /// the hash domain: do it deliberately — update this pin AND the continuity
    /// note at `compute_config_hash` in the same change.
    #[test]
    fn config_hash_momentum_default_pinned() {
        use crate::cross_sectional::config::CrossSectionalMomentumConfig;

        // The canonical momentum default (mirrors config/strategies/top10_momentum_h1.toml).
        let cfg = CrossSectionalMomentumConfig::from_str(
            r#"
id     = "top10_momentum_h1"
kind   = "cross_sectional_momentum"
stage  = "research"
universe = [
    "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT",
    "DOTUSDT", "ETHUSDT", "LINKUSDT", "SOLUSDT", "XRPUSDT",
]
lookback_minutes      = 60
rebalance_minutes     = 60
k_long                = 3
k_short               = 0
exposure_cap          = "0.50"
drift_rebalance_threshold = "0.10"
vol_floor             = "0.000001"
size                  = "equal_weight"
"#,
        )
        .expect("canonical momentum default must parse");
        let strat = MomentumStrategy::from_config(cfg, SmolStr::new("test"));

        let hex: String = strat.hash.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        });
        assert_eq!(
            hex, "52ba625567f18c9b4d3eeb2f104520bcd9ba4c4709eb04da56983a41d098ac2f",
            "review 1-16 continuity pin: the canonical momentum-default config hash moved — \
             the hash domain has been migrated (see compute_config_hash's continuity note)"
        );
    }
}
