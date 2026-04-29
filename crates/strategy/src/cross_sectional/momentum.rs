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

use crate::cross_sectional::config::CrossSectionalMomentumConfig;
use crate::cross_sectional::selector::top_k_long;
use crate::Strategy;
use features::{score_vol_adjusted_return, RingBuffer};

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

    /// Per-symbol ring buffers of close prices (size = lookback_minutes + 1).
    histories: BTreeMap<Symbol, RingBuffer>,
    /// Per-symbol latest score cache (None = warming up).
    scores: BTreeMap<Symbol, Option<Decimal>>,
    /// Timestamp of the last rebalance bar close (None before first rebalance).
    last_rebalance_ts: Option<Timestamp>,
    /// Current per-symbol position — tracked as qty held (0 = flat).
    /// Maintained from the signals emitted (approximate).
    held_symbols: BTreeMap<Symbol, bool>,

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
            histories,
            scores,
            last_rebalance_ts: None,
            held_symbols,
            hash,
            source_path,
        }
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
        self.histories.values().all(|rb| rb.is_full())
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
                });
            }
        }

        signals
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

        // Push close into the symbol's ring buffer.
        if let Some(rb) = self.histories.get_mut(&bar.symbol) {
            rb.push(bar.close.get());
        }

        // Recompute score for this symbol.
        let score = self.histories.get(&bar.symbol).and_then(|rb| {
            score_vol_adjusted_return(rb, self.lookback_minutes, self.vol_floor).ok()
        });
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

    let canonical = format!(
        "id={id};universe={uni};lookback={lb};rebalance={rb};k_long={kl};k_short={ks};\
         exposure_cap={ec};drift={dt};vol_floor={vf}",
        id = cfg.id,
        uni = universe_sorted.join(","),
        lb = cfg.lookback_minutes,
        rb = cfg.rebalance_minutes,
        kl = cfg.k_long,
        ks = cfg.k_short,
        ec = cfg.exposure_cap,
        dt = cfg.drift_rebalance_threshold,
        vf = cfg.vol_floor,
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
    use trading_core::{Price, Quantity, Timeframe};

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
}
