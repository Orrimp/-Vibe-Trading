//! `AlwaysLongStrategy` — buy-and-hold forward-paper engine (F5b).
//!
//! ## Semantic
//!
//! Emits `SignalKind::Buy` on the **first bar** seen for a given symbol,
//! then `SignalKind::Hold` on every subsequent bar for that symbol.
//!
//! This matches the buy-and-hold bake-off benchmark (`bakeoff::buyhold`) at the
//! signal layer: buy once at the first bar's close price and hold indefinitely.
//! The executor converts the `Buy` signal into an order; subsequent `Hold`
//! signals leave the position untouched.
//!
//! ## Why not `bakeoff::buyhold::run_buyhold_path`?
//!
//! `run_buyhold_path` is a standalone equity-curve function, not a `Strategy`
//! trait impl.  The forward paper loop consumes a `StrategyRegistry` that routes
//! `Bar` events to `Box<dyn Strategy>` objects.  `AlwaysLongStrategy` bridges
//! the two: the same semantics (buy once, hold forever) expressed as a proper
//! `Strategy` so it can be registered and driven bar-by-bar.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use strategy::AlwaysLongStrategy;
//! let mut s = AlwaysLongStrategy::new();
//! ```

use std::collections::HashSet;

use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Tick};

use crate::Strategy;

// ── AlwaysLongStrategy ────────────────────────────────────────────────────────

/// Buy-and-hold forward-paper strategy: Buy on first bar per symbol, Hold after.
///
/// State: set of symbols for which the initial `Buy` has already been emitted.
/// Thread-safe via `Send + Sync` (the `HashSet` is owned, no interior mutability
/// needed — `Strategy::on_bar` takes `&mut self`).
#[derive(Debug)]
pub struct AlwaysLongStrategy {
    id: StrategyId,
    /// Symbols that have already received the initial `Buy` signal.
    bought: HashSet<Symbol>,
}

impl AlwaysLongStrategy {
    /// Construct a new `AlwaysLongStrategy`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: StrategyId::new("always_long"),
            bought: HashSet::new(),
        }
    }
}

impl Default for AlwaysLongStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for AlwaysLongStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    /// Emits `Buy` on the first bar for a symbol, `Hold` on every subsequent bar.
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        let kind = if self.bought.contains(&bar.symbol) {
            SignalKind::Hold
        } else {
            self.bought.insert(bar.symbol.clone());
            SignalKind::Buy
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
        // Buy-and-hold is bar-close only; tick-level signals are suppressed.
        vec![]
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({})
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::symbol::Symbol;
    use trading_core::{Bar, Price, Quantity, SignalKind, Timeframe, Timestamp, Venue};

    use super::*;

    fn make_bar(symbol: &str, ts_secs: i64, close: rust_decimal::Decimal) -> Bar {
        let ts = Timestamp::new(
            OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_secs),
        );
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: ts,
            local_recv_ts: ts,
            venue: Venue::Binance,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1.0)).unwrap(),
            trade_count: 1,
        }
    }

    #[test]
    fn first_bar_emits_buy() {
        let mut s = AlwaysLongStrategy::new();
        let bar = make_bar("BTCUSDT", 0, dec!(50_000));
        let sigs = s.on_bar(&bar);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].kind, SignalKind::Buy);
        assert_eq!(sigs[0].symbol.0.as_str(), "BTCUSDT");
    }

    #[test]
    fn subsequent_bars_emit_hold() {
        let mut s = AlwaysLongStrategy::new();
        // First bar: Buy
        let _ = s.on_bar(&make_bar("BTCUSDT", 0, dec!(50_000)));
        // Second bar: Hold
        let sigs = s.on_bar(&make_bar("BTCUSDT", 3600, dec!(51_000)));
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].kind, SignalKind::Hold);
        // Third bar: Hold
        let sigs = s.on_bar(&make_bar("BTCUSDT", 7200, dec!(52_000)));
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].kind, SignalKind::Hold);
    }

    #[test]
    fn different_symbols_each_get_initial_buy() {
        let mut s = AlwaysLongStrategy::new();
        // BTC: first bar → Buy
        let sigs = s.on_bar(&make_bar("BTCUSDT", 0, dec!(50_000)));
        assert_eq!(sigs[0].kind, SignalKind::Buy);
        // ETH: first bar → Buy (independent state)
        let sigs = s.on_bar(&make_bar("ETHUSDT", 0, dec!(2_000)));
        assert_eq!(sigs[0].kind, SignalKind::Buy);
        // BTC: second bar → Hold
        let sigs = s.on_bar(&make_bar("BTCUSDT", 3600, dec!(51_000)));
        assert_eq!(sigs[0].kind, SignalKind::Hold);
        // ETH: second bar → Hold
        let sigs = s.on_bar(&make_bar("ETHUSDT", 3600, dec!(2_100)));
        assert_eq!(sigs[0].kind, SignalKind::Hold);
    }

    #[test]
    fn id_is_always_long() {
        let s = AlwaysLongStrategy::new();
        assert_eq!(s.id().0.as_str(), "always_long");
    }

    #[test]
    fn on_tick_returns_empty() {
        use trading_core::{Side, Tick};
        let mut s = AlwaysLongStrategy::new();
        let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let tick = Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: ts,
            local_recv_ts: ts,
            price: Price::new(dec!(50_000)).unwrap(),
            qty: Quantity::new(dec!(1.0)).unwrap(),
            side: Side::Buy,
            trade_id: 0,
            venue: Venue::Binance,
        };
        let sigs = s.on_tick(&tick);
        assert!(sigs.is_empty());
    }
}
