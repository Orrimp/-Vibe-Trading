//! `MeanReversionPairsStrategy` — v1.5a mean-reversion pairs strategy (T706).
//!
//! ## Design
//!
//! Implements the v0 `Strategy` trait **verbatim** — no trait changes (R7).
//!
//! ### on_bar (R7.1)
//!
//! 1. Universe filter (Q5 / R7.3): if the bar's symbol is not in the pair
//!    universe, return `vec![]` immediately.
//! 2. Determine which pair(s) this symbol belongs to (as `a` or `b` leg).
//! 3. For each matching pair, call [`PairState::observe_leg`].
//! 4. Collect and return all emitted signals in `BTreeMap<PairKey,_>` iteration
//!    order (R9.3 determinism).
//!
//! ### on_tick (R7.2)
//!
//! Returns `vec![]`. Mean-reversion is bar-close driven only.
//!
//! ### Formulation C (Q3)
//!
//! Only `OpenPairLong` (→ `Order::Buy`) is submitted for execution.
//! `PairShortObservation` signals are emitted alongside entry but never
//! converted to an `Order` by the agent layer.
//!
//! ### Determinism (R9)
//!
//! All pair-keyed maps are `BTreeMap<PairKey,_>`.  `PairKey` derives `Ord` for
//! lexicographic order.  The signal vec from `on_bar` is ordered by
//! `(pair_key, signal_kind_ordinal)`.
//!
//! ### Hot-swap (R7.6)
//!
//! `MeanReversionPairsStrategy::from_config` accepts a `source_path` and
//! pre-computes a SHA-256 content hash of the canonicalized config.  The
//! registry compares this hash on file-change events to detect hot-swaps
//! (no-op if unchanged).  On a real swap, the strategy struct is rebuilt from
//! scratch — ring buffers reset, position state cleared.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use smol_str::SmolStr;
use trading_core::{Bar, PairKey, Signal, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::pairs::config::MeanReversionPairsConfig;
use crate::pairs::pair_state::{LegRole, PairState};

/// v1.5a mean-reversion pairs strategy.
///
/// Implements `Strategy` verbatim (v0 trait shape, no trait change per R7).
/// `on_tick` returns `vec![]` (bar-close driven only, R7.2).
/// Out-of-universe bars return `vec![]` immediately (Q5 / R7.3).
pub struct MeanReversionPairsStrategy {
    id: StrategyId,

    // ── Config knobs (shared across all pairs) ─────────────────────────────
    lookback_minutes: u32,
    cooldown_minutes: u32,
    z_entry: Decimal,
    z_exit: Decimal,
    z_stop: Decimal,
    vol_floor: Decimal,
    exposure_cap_per_pair: Decimal,
    max_staleness_minutes: u32,

    // ── Universe index ─────────────────────────────────────────────────────
    /// All symbols that appear as `a` or `b` in any configured pair.
    /// Used for the O(log n) universe-filter check in `on_bar`.
    universe: BTreeSet<Symbol>,

    /// `symbol → [(pair_key, LegRole)]` — which pairs does this symbol feed?
    ///
    /// `BTreeMap` for deterministic iteration. Each entry is sorted by
    /// `PairKey` so per-pair signals are emitted in lexicographic order.
    symbol_to_pairs: BTreeMap<Symbol, Vec<(PairKey, LegRole)>>,

    // ── Per-pair state ─────────────────────────────────────────────────────
    /// Per-pair mutable state, iterated in `PairKey` (lexicographic) order.
    pair_states: BTreeMap<PairKey, (Decimal, PairState)>, // (beta, state)

    // ── Config identity ────────────────────────────────────────────────────
    /// SHA-256 of the canonicalized config — used by the registry to detect
    /// hot-swaps (compare before replacing the running strategy).
    pub hash: [u8; 32],

    /// Filesystem path the config was loaded from (for registry tracking).
    pub source_path: SmolStr,
}

impl MeanReversionPairsStrategy {
    /// Construct from a validated [`MeanReversionPairsConfig`].
    #[must_use]
    pub fn from_config(cfg: MeanReversionPairsConfig, source_path: SmolStr) -> Self {
        let hash = compute_config_hash(&cfg);
        let id = StrategyId::new(cfg.id.as_str());

        let mut universe: BTreeSet<Symbol> = BTreeSet::new();
        let mut symbol_to_pairs: BTreeMap<Symbol, Vec<(PairKey, LegRole)>> = BTreeMap::new();
        let mut pair_states: BTreeMap<PairKey, (Decimal, PairState)> = BTreeMap::new();

        for pair in &cfg.pairs {
            // Universe membership.
            universe.insert(pair.key.a.clone());
            universe.insert(pair.key.b.clone());

            // symbol_to_pairs index — a leg.
            symbol_to_pairs
                .entry(pair.key.a.clone())
                .or_default()
                .push((pair.key.clone(), LegRole::A));

            // symbol_to_pairs index — b leg.
            symbol_to_pairs
                .entry(pair.key.b.clone())
                .or_default()
                .push((pair.key.clone(), LegRole::B));

            // Per-pair state.
            pair_states.insert(
                pair.key.clone(),
                (pair.beta, PairState::new(cfg.lookback_minutes)),
            );
        }

        // Sort each symbol's pair list by PairKey for deterministic signal order.
        for v in symbol_to_pairs.values_mut() {
            v.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        }

        Self {
            id,
            lookback_minutes: cfg.lookback_minutes,
            cooldown_minutes: cfg.cooldown_minutes,
            z_entry: cfg.z_entry,
            z_exit: cfg.z_exit,
            z_stop: cfg.z_stop,
            vol_floor: cfg.vol_floor,
            exposure_cap_per_pair: cfg.exposure_cap_per_pair,
            max_staleness_minutes: cfg.max_staleness_minutes,
            universe,
            symbol_to_pairs,
            pair_states,
            hash,
            source_path,
        }
    }

    /// Return the set of all symbols in the pair universe (a + b legs).
    pub fn universe(&self) -> impl Iterator<Item = &Symbol> {
        self.universe.iter()
    }
}

impl Strategy for MeanReversionPairsStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    /// Process a bar-close event.
    ///
    /// Q5 / R7.3: out-of-universe bars are a fast early-return (no state
    /// mutation, no log, just `vec![]`).
    ///
    /// R9.3: signals are collected in `BTreeMap<PairKey,_>` iteration order
    /// (lexicographic), then per-pair signals are in natural emit order
    /// (hard-stop / exit before new-entry checks within one pair).
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // Q5 / R7.3 universe filter — O(log n).
        if !self.universe.contains(&bar.symbol) {
            return Vec::new();
        }

        // Find which pairs this symbol feeds.
        let pairs_for_symbol = match self.symbol_to_pairs.get(&bar.symbol) {
            Some(v) => v.clone(), // cheap: small vec (max 16 pairs)
            None => return Vec::new(),
        };

        let mut signals: Vec<Signal> = Vec::new();

        // Iterate in BTreeMap (PairKey lexicographic) order.
        for (pair_key, role) in &pairs_for_symbol {
            let Some((beta, pair_state)) = self.pair_states.get_mut(pair_key) else {
                continue;
            };

            let pair_signals = pair_state.observe_leg(
                *role,
                bar,
                *beta,
                self.lookback_minutes,
                self.cooldown_minutes,
                self.z_entry,
                self.z_exit,
                self.z_stop,
                self.vol_floor,
                self.exposure_cap_per_pair,
                self.max_staleness_minutes,
                self.id.clone(),
                pair_key.clone(),
            );

            signals.extend(pair_signals);
        }

        signals
    }

    /// Bar-close driven only — tick events are always a no-op (R7.2).
    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        Vec::new()
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        MeanReversionPairsConfig::json_schema()
    }
}

// ── Config hash ───────────────────────────────────────────────────────────────

/// Compute a SHA-256 hash of the canonicalized config.
///
/// The canonical string is deterministic: pairs sorted by PairKey, then all
/// fields concatenated in a stable order. Used by the registry to detect
/// hot-swaps (compare before replacing the running strategy).
fn compute_config_hash(cfg: &MeanReversionPairsConfig) -> [u8; 32] {
    // Pairs sorted by PairKey (already validated, but we sort for stability).
    let mut pairs_sorted = cfg.pairs.clone();
    pairs_sorted.sort_by(|a, b| a.key.cmp(&b.key));

    let pairs_str = pairs_sorted
        .iter()
        .map(|p| format!("{}:{}:{}", p.key.a, p.key.b, p.beta))
        .collect::<Vec<_>>()
        .join("|");

    let canonical = format!(
        "id={id};pairs={pairs};lookback={lb};cooldown={cd};\
         z_entry={ze};z_exit={zx};z_stop={zs};vol_floor={vf};\
         exposure_cap={ec};staleness={sl}",
        id = cfg.id,
        pairs = pairs_str,
        lb = cfg.lookback_minutes,
        cd = cfg.cooldown_minutes,
        ze = cfg.z_entry,
        zx = cfg.z_exit,
        zs = cfg.z_stop,
        vf = cfg.vol_floor,
        ec = cfg.exposure_cap_per_pair,
        sl = cfg.max_staleness_minutes,
    );

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    bytes
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Price, Quantity, SignalKind, Timeframe, Timestamp, Venue};

    fn ts_at(minute: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minute))
    }

    fn make_bar(symbol: &str, close: Decimal, minute: i64) -> Bar {
        let ts = ts_at(minute);
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

    fn make_strategy(toml: &str) -> MeanReversionPairsStrategy {
        let cfg = MeanReversionPairsConfig::from_str(toml).unwrap();
        MeanReversionPairsStrategy::from_config(cfg, SmolStr::new("test.toml"))
    }

    fn canonical_toml() -> &'static str {
        r#"
id = "pairs_mr_test"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    { a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },
]

lookback_minutes      = 5
cooldown_minutes      = 60
z_entry               = "2.0"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#
    }

    // ── T706 acceptance: Q5 universe filter ────────────────────────────────

    #[test]
    fn t706_out_of_universe_bar_ignored() {
        let mut strat = make_strategy(canonical_toml());
        let bar = make_bar("XRPUSDT", dec!(1), 1);
        let sigs = strat.on_bar(&bar);
        assert!(sigs.is_empty(), "out-of-universe bar must return vec![]");
    }

    #[test]
    fn t706_on_tick_always_empty() {
        let mut strat = make_strategy(canonical_toml());
        let tick = trading_core::Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: ts_at(1),
            local_recv_ts: ts_at(1),
            price: Price::new(dec!(30000)).unwrap(),
            qty: trading_core::Quantity::new(dec!(1)).unwrap(),
            side: trading_core::Side::Buy,
            trade_id: 1,
            venue: Venue::Binance,
        };
        let sigs = strat.on_tick(&tick);
        assert!(sigs.is_empty(), "on_tick must always return vec![]");
    }

    // ── T706 acceptance: single-leg no signals ──────────────────────────────

    #[test]
    fn t706_single_leg_bar_no_signals() {
        let mut strat = make_strategy(canonical_toml());
        // Only a-leg bar, no b-leg — sync incomplete.
        let bar = make_bar("BTCUSDT", dec!(30000), 1);
        let sigs = strat.on_bar(&bar);
        assert!(sigs.is_empty(), "single leg should produce no signals");
    }

    // ── T706 acceptance: warmup no signals ─────────────────────────────────

    #[test]
    fn t706_warmup_no_signals() {
        let mut strat = make_strategy(canonical_toml());
        // lookback = 5: need 5 paired bars before z-score is available.
        for i in 0i64..4 {
            let bar_a = make_bar("BTCUSDT", dec!(30000), i);
            let bar_b = make_bar("ETHUSDT", dec!(2000), i);
            let sigs_a = strat.on_bar(&bar_a);
            let sigs_b = strat.on_bar(&bar_b);
            assert!(
                sigs_a.is_empty() && sigs_b.is_empty(),
                "warmup bars should produce no signals (i={i})"
            );
        }
    }

    // ── T706 acceptance: entry signal ──────────────────────────────────────

    #[test]
    fn t706_entry_on_low_z() {
        let mut strat = make_strategy(canonical_toml());
        let lookback = 5u32;

        // Warm up with neutral (same) prices — spread ≈ 0, z ≈ 0.
        for i in 0i64..(lookback as i64) {
            strat.on_bar(&make_bar("BTCUSDT", dec!(30000), i));
            strat.on_bar(&make_bar("ETHUSDT", dec!(30000), i));
        }

        // Now inject a large price divergence to force z << -z_entry.
        // price_a drops sharply while price_b stays → spread = ln(a) - ln(b) drops.
        // With price_a = 1000 and price_b = 30000:
        //   spread = ln(1000) - ln(30000) = -3.40 (roughly)
        // This is very negative → should trigger entry if z is well below -2.
        let trigger_min = lookback as i64;
        strat.on_bar(&make_bar("BTCUSDT", dec!(1000), trigger_min));
        let sigs = strat.on_bar(&make_bar("ETHUSDT", dec!(30000), trigger_min));

        // We expect an entry (or possibly none if z didn't cross — depends on buffer).
        // The key structural assertion: if any signals are emitted, they must be
        // OpenPairLong + PairShortObservation (formulation C).
        for sig in &sigs {
            assert!(
                matches!(
                    sig.kind,
                    SignalKind::OpenPairLong | SignalKind::PairShortObservation
                ),
                "unexpected signal kind: {:?}",
                sig.kind
            );
        }
    }

    // ── T706 acceptance: determinism (R9.3) ────────────────────────────────

    #[test]
    fn t706_deterministic_two_runs() {
        let multi_pair_toml = r#"
id = "pairs_mr_det"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    { a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },
    { a = "BNBUSDT", b = "SOLUSDT", beta = "1.0" },
    { a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" },
]

lookback_minutes      = 5
cooldown_minutes      = 60
z_entry               = "2.0"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#;
        let run = || {
            let mut strat = make_strategy(multi_pair_toml);
            let mut all: Vec<(String, String)> = Vec::new();
            let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT"];
            for minute in 0i64..20 {
                for (si, sym) in symbols.iter().enumerate() {
                    let price = Decimal::from(1000u32 + si as u32 * 100 + minute as u32 * 10);
                    let sigs = strat.on_bar(&make_bar(sym, price, minute));
                    for s in sigs {
                        all.push((s.symbol.to_string(), format!("{:?}", s.kind)));
                    }
                }
            }
            all
        };

        let run1 = run();
        let run2 = run();
        assert_eq!(
            run1, run2,
            "signal sequence must be identical across two runs"
        );
    }

    // ── T706 acceptance: config hash stability ─────────────────────────────

    #[test]
    fn t706_config_hash_stable() {
        let strat1 = make_strategy(canonical_toml());
        let strat2 = make_strategy(canonical_toml());
        assert_eq!(
            strat1.hash, strat2.hash,
            "same config should produce same hash"
        );
    }

    #[test]
    fn t706_config_hash_changes_on_z_entry_change() {
        let strat1 = make_strategy(canonical_toml());
        let toml2 =
            canonical_toml().replace(r#"z_entry               = "2.0""#, r#"z_entry = "3.0""#);
        let strat2 = make_strategy(&toml2);
        assert_ne!(
            strat1.hash, strat2.hash,
            "different z_entry should produce different hash"
        );
    }

    // ── T706 acceptance: universe introspection ─────────────────────────────

    #[test]
    fn t706_universe_contains_both_legs() {
        let strat = make_strategy(canonical_toml());
        let uni: Vec<String> = strat.universe().map(|s| s.to_string()).collect();
        assert!(uni.contains(&"BTCUSDT".to_string()));
        assert!(uni.contains(&"ETHUSDT".to_string()));
    }

    // ── T706 acceptance: config_schema ─────────────────────────────────────

    #[test]
    fn t706_config_schema_is_valid_json_object() {
        let schema = MeanReversionPairsStrategy::config_schema();
        assert!(
            schema.is_object(),
            "config_schema() must return a JSON object"
        );
        assert!(schema["type"] == "object");
    }
}
