//! `CashHoldStrategy` — degenerate cash-hold fallback (ADR-0049 § D3).
//!
//! ## Contract
//!
//! - Emits `SignalKind::Hold` for **every** (symbol, bar) pair.
//! - NO state. NO I/O. Pure function.
//! - Existing positions are HELD, not liquidated, when the regime dispatcher
//!   routes to this strategy — this is the SUPPRESSION-NOT-LIQUIDATION
//!   semantic (ADR-0049 § D3, architect option (i)).
//! - Natural exits still fire via the composed exit policy (ADR-0010).
//!
//! ## Usage
//!
//! ```rust,no_run
//! use strategy::CashHoldStrategy;
//! let mut s = CashHoldStrategy::new();
//! ```
//!
//! ## Cross-references
//!
//! - ADR-0049 § D3 — dispatcher + cash-fallback contract.
//! - `crates/strategy/src/regime_dispatcher.rs` — routes to this for
//!   `Volatile` and `Calm` regimes.
//! - v0.2.0 follow-on: `v1.5-mean-reversion-for-regime-dispatcher` replaces
//!   this with `MeanReversionStrategy`; no dispatcher rewire needed.

use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Tick};

use crate::Strategy;

// ── CashHoldStrategy ─────────────────────────────────────────────────────────

/// Degenerate cash-hold strategy — emits `SignalKind::Hold` for every bar.
///
/// This is the v0.1.0 fallback for `Volatile` and `Calm` regimes in the
/// `RegimeDispatcher`.  Its job is pure suppression: when the regime
/// classifier labels the current market state as non-trending, no new entry
/// signals are generated.  **Existing positions are not touched** — the
/// executor's composed exit policy (ADR-0010) handles natural exits.
///
/// ## Invariant (K6 falsifier target)
///
/// Every `on_bar` call returns exactly one `Hold` signal for the bar's symbol.
/// The `regime_dispatcher_end_to_end.rs` test asserts that routing to this
/// strategy causes the equity curve to diverge from the unconditioned
/// `MomentumStrategy` baseline — confirming the dispatcher is not a no-op.
#[derive(Debug)]
pub struct CashHoldStrategy {
    id: StrategyId,
}

impl CashHoldStrategy {
    /// Construct a new `CashHoldStrategy`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: StrategyId::new("cash_hold"),
        }
    }
}

impl Default for CashHoldStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Strategy for CashHoldStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    /// Emits exactly one `SignalKind::Hold` for the bar's symbol.
    ///
    /// No state mutation. No I/O. Pure function (ADR-0049 § D3 contract).
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        vec![Signal {
            strategy_id: self.id.clone(),
            symbol: bar.symbol.clone(),
            ts: bar.close_ts,
            kind: SignalKind::Hold,
            evidence: SignalEvidence::empty(),
            pair_data: None,
        }]
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        // CashHoldStrategy is bar-close only; tick-level signals are suppressed.
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
    fn cash_hold_emits_only_hold() {
        let mut s = CashHoldStrategy::new();
        // Feed multiple symbols across multiple bars — every signal must be Hold.
        for symbol in &["BTCUSDT", "ETHUSDT", "BNBUSDT"] {
            for i in 0..5_i64 {
                let bar = make_bar(symbol, i * 3600, dec!(50_000.0));
                let signals = s.on_bar(&bar);
                assert_eq!(signals.len(), 1, "expected exactly 1 signal per bar");
                assert_eq!(
                    signals[0].kind,
                    SignalKind::Hold,
                    "CashHoldStrategy must emit Hold, got {:?}",
                    signals[0].kind
                );
                assert_eq!(
                    signals[0].symbol.0.as_str(),
                    *symbol,
                    "Hold signal must carry the bar's symbol"
                );
            }
        }
    }

    #[test]
    fn cash_hold_on_tick_is_empty() {
        use trading_core::{Side, Tick};
        let mut s = CashHoldStrategy::new();
        let ts =
            Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000));
        let tick = Tick {
            symbol: Symbol::new("BTCUSDT"),
            venue_ts: ts,
            local_recv_ts: ts,
            price: Price::new(dec!(50_000.0)).unwrap(),
            qty: Quantity::new(dec!(1.0)).unwrap(),
            side: Side::Buy,
            trade_id: 0,
            venue: Venue::Binance,
        };
        let signals = s.on_tick(&tick);
        assert!(
            signals.is_empty(),
            "on_tick must return empty vec for CashHoldStrategy"
        );
    }

    #[test]
    fn cash_hold_id_is_cash_hold() {
        let s = CashHoldStrategy::new();
        assert_eq!(s.id().0.as_str(), "cash_hold");
    }

    #[test]
    fn cash_hold_default_same_as_new() {
        let s = CashHoldStrategy::default();
        assert_eq!(s.id().0.as_str(), "cash_hold");
    }
}
