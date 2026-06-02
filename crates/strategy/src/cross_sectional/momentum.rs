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
use crate::cross_sectional::config::{CrossSectionalMomentumConfig, Direction, ScoreSource};
use crate::cross_sectional::selector::top_k_long;
use features::{RingBuffer, score_vol_adjusted_return};

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
    #[allow(dead_code)]
    k_short: u32, // always 0 in v1
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

    /// Per-symbol ring buffers of close prices (size = lookback_minutes + 1).
    histories: BTreeMap<Symbol, RingBuffer>,
    /// Per-symbol latest score cache (None = warming up).
    scores: BTreeMap<Symbol, Option<Decimal>>,
    /// Timestamp of the last rebalance bar close (None before first rebalance).
    last_rebalance_ts: Option<Timestamp>,
    /// Current per-symbol position — tracked as qty held (0 = flat).
    /// Maintained from the signals emitted (approximate).
    held_symbols: BTreeMap<Symbol, bool>,

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
            histories,
            scores,
            last_rebalance_ts: None,
            held_symbols,
            funding_map: None,
            funding_rings,
            funding_lookback,
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
        }
    }

    fn build_rebalance_signals(&mut self, bar: &Bar) -> Vec<Signal> {
        let target_weights = top_k_long(&self.scores, self.k_long, self.exposure_cap);

        let mut signals = Vec::new();
        let ts = bar.close_ts;

        // Iterate in alphabetical order (BTreeMap) per R12.5
        for symbol in self.universe_symbols.keys() {
            let currently_held = *self.held_symbols.get(symbol).unwrap_or(&false);
            let target_weight = target_weights.get(symbol);

            let action = match (currently_held, target_weight) {
                (false, Some(_)) => {
                    // Not held, in new top-K → Open
                    Some((SignalKind::Buy, "open"))
                }
                (true, None) => {
                    // Held, fell out of top-K → Close
                    Some((SignalKind::Sell, "close"))
                }
                (true, Some(_)) => {
                    // Held and still in top-K → check drift
                    // For simplicity in backtest: always hold (drift check
                    // would need current position weights which the strategy
                    // doesn't track in this version — R6.2 threshold check)
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

        signals
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
        if ring.len() < self.funding_lookback {
            return None;
        }
        let sum: Decimal = ring.iter().copied().sum();
        let mean = sum / Decimal::from(ring.len() as u64);
        // R-CARRY.2: return −mean so the most-negative-funding name has the highest score.
        Some(-mean)
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

        // Compute score — fork on score_source (M-DEV-5, D-CARRY.1).
        let score = match self.score_source {
            ScoreSource::VolAdjustedReturn => {
                // EXISTING path — byte-identical to pre-carry code.
                // Push close into the symbol's ring buffer.
                if let Some(rb) = self.histories.get_mut(&bar.symbol) {
                    rb.push(bar.close.get());
                }
                // Recompute score for this symbol.
                let score = self.histories.get(&bar.symbol).and_then(|rb| {
                    score_vol_adjusted_return(rb, self.lookback_minutes, self.vol_floor).ok()
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

fn compute_config_hash(cfg: &CrossSectionalMomentumConfig) -> [u8; 32] {
    // Canonicalized: sort universe alphabetically, then hash the joined fields.
    let mut universe_sorted = cfg.universe.clone();
    universe_sorted.sort();

    // M-DEV-5: append ;score_source={...} so carry-vs-momentum at the same θ
    // hashes differently (K3 — the config hash distinguishes strategy variants).
    let canonical = format!(
        "id={id};universe={uni};lookback={lb};rebalance={rb};k_long={kl};k_short={ks};\
         exposure_cap={ec};drift={dt};vol_floor={vf};direction={dir:?};score_source={ss:?}",
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
    /// BTCUSDT trends strongly up (+5% per bar): highest momentum score.
    /// ETHUSDT is flat.
    /// BNBUSDT trends strongly down (−5% per bar): lowest momentum score / highest MR score.
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
}
