//! `PaperEngine` — simple bps slippage + taker fee matching engine.
//!
//! v0 ships this as the only `MatchingEngine` implementation.
//! Full implementation in T24; this is the stub.
use async_trait::async_trait;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use trading_core::{
    Bar, FeeTier, Fill, FillId, Liquidity, Money, Order, Price, Quantity, Side, Timestamp,
};

use crate::cli_types::VenueFilterMode;
use crate::engine::{MatchError, MatchingEngine};

/// Fill price mode: use bar close or bar VWAP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillPriceMode {
    BarClose,
    BarVwap,
}

/// Configuration for the `PaperEngine`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub slippage_bps: u32,
    pub taker_fee_bps: u32,
    pub maker_fee_bps: u32,
    pub fill_price_mode: FillPriceMode,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 2,
            taker_fee_bps: 4,
            maker_fee_bps: 2,
            fill_price_mode: FillPriceMode::BarClose,
        }
    }
}

/// Exec-sim-filter statistics accumulated by `PaperEngine` across a run
/// (ADR-0087 § D4). Currently carries only the lot-size/min-notional skip
/// tally; the shape is additive for future filter-mode stats.
///
/// This is the **primary, in-memory home** for the skip record on the
/// advisor sim path (bake-off + forward loop keep cash/equity in-memory and
/// do not write to `audit::Ledger` — ADR-0087 § D4.1). The **live-agent**
/// home (`AuditEvent::StrategyEvent`, `kind = "min_notional_skip"`) is
/// reserved-but-unbuilt — see the doc-comment at `venue_filter_for`'s call
/// site in `step` below (ADR-0087 § D4.2 / T8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SimFilterStats {
    /// Count of orders dropped (no `Fill` pushed) because the venue-filter
    /// mode rounded the qty to zero or the rounded notional was below
    /// `min_notional`. Always `0` when `venue_filter` is `None` (the
    /// default) — see `paper_step_none_is_byte_identical`.
    pub skipped_min_notional: u64,
}

/// Simple paper matching engine.
pub struct PaperEngine {
    config: MatchConfig,
    /// Seeded RNG for deterministic tie-breaking (T24 will use it more extensively).
    #[allow(dead_code)]
    rng: ChaCha20Rng,
    /// Opt-in venue-filter realism mode (ADR-0087). `None` (the default,
    /// set by every existing constructor) is byte-identical to the
    /// pre-ADR-0087 fill path — see `paper_step_none_is_byte_identical`
    /// (D6 contract, NEVER DELETE).
    venue_filter: Option<VenueFilterMode>,
    /// Count of orders skipped by the venue-filter mode (D4). Always `0`
    /// when `venue_filter` is `None`.
    skipped_min_notional: u64,
}

impl PaperEngine {
    #[must_use]
    pub fn new(config: MatchConfig, seed: u64) -> Self {
        Self {
            config,
            rng: ChaCha20Rng::seed_from_u64(seed),
            venue_filter: None,
            skipped_min_notional: 0,
        }
    }

    #[must_use]
    pub fn with_default_seed(config: MatchConfig) -> Self {
        Self::new(config, 0x00C0_FFEE)
    }

    /// Opt-in builder (ADR-0087): enable the venue-filter exec-sim mode.
    ///
    /// Never calling this (the constructors above default to `None`) is the
    /// byte-identical, pre-ADR-0087 path (D6 opt-in-forever contract).
    #[must_use]
    pub fn with_venue_filter_mode(mut self, mode: Option<VenueFilterMode>) -> Self {
        self.venue_filter = mode;
        self
    }

    /// Exec-sim filter stats accumulated so far (ADR-0087 § D4) — the
    /// advisor sim's primary, in-memory skip-tally home.
    #[must_use]
    pub fn sim_filter_stats(&self) -> SimFilterStats {
        SimFilterStats {
            skipped_min_notional: self.skipped_min_notional,
        }
    }
}

#[async_trait]
impl MatchingEngine for PaperEngine {
    async fn step(&mut self, bar: &Bar, orders: Vec<Order>) -> Result<Vec<Fill>, MatchError> {
        let mut fills = Vec::with_capacity(orders.len());

        // VWAP is not yet available in Bar; both modes fall back to close.
        #[allow(clippy::match_same_arms)]
        let base_price = match self.config.fill_price_mode {
            FillPriceMode::BarClose => bar.close.get(),
            FillPriceMode::BarVwap => bar.close.get(),
        };

        let slippage = Decimal::from(self.config.slippage_bps);
        let ten_k = Decimal::new(10_000, 0);
        let fee_bps = Decimal::from(self.config.taker_fee_bps);

        for order in orders {
            let fill_price = match order.side() {
                Side::Buy => base_price * (Decimal::ONE + slippage / ten_k),
                Side::Sell => base_price * (Decimal::ONE - slippage / ten_k),
            };

            // ADR-0087 § D1 — opt-in lot-size rounding + min-notional reject,
            // applied to `qty` BEFORE the `Fill` is constructed (the sole
            // place `qty` is finalized; every downstream cash/position
            // update reads `fill.qty.get()`). `venue_filter == None` (the
            // default) unconditionally takes the first arm: `qty ==
            // order.qty()` byte-for-byte — see
            // `paper_step_none_is_byte_identical` (D6 contract, NEVER
            // weaken this arm without updating that test).
            let qty = if let Some(VenueFilterMode::LotSizeAndMinNotional) = self.venue_filter {
                if let Some(filter) = cost::venue_filter_for(order.symbol()) {
                    if let Some(rounded) = filter.admit(order.qty().get(), fill_price) {
                        Quantity::new(rounded).map_err(|e| MatchError::FillError(e.to_string()))?
                    } else {
                        // Sub-min-notional / zero-after-round: skip this
                        // order. Push NO `Fill` — this is NOT a
                        // `MatchError` (D1): a min-notional reject is a
                        // normal venue outcome, not a fault.
                        //
                        // ADR-0087 § D4.2 / T8 (reserved, NOT built here):
                        // the advisor sim path (bake-off + forward loop)
                        // keeps cash/equity in-memory and does not own an
                        // `audit::Ledger`, so the PRIMARY skip record is the
                        // in-memory `self.skipped_min_notional` tally below,
                        // surfaced via `sim_filter_stats()` (`SimFilterStats`
                        // doc-comment, this file). The LIVE-AGENT home —
                        // reserved, unbuilt, no live path ships here — is
                        // `audit::journal::strategy_event` writing a
                        // `StrategyEventWrite { kind: "min_notional_skip",
                        // .. }` row to `strategy_events`
                        // (`crates/audit/src/journal.rs:1623`), the same
                        // pattern as the shipped `rebalance_rejected` event
                        // (`crates/audit/src/journal.rs:1722`). No new
                        // `AuditEvent` variant needed. Wire this only when a
                        // live-agent caller (one that owns a `Ledger`)
                        // actually constructs `PaperEngine` with
                        // `venue_filter` enabled.
                        self.skipped_min_notional += 1;
                        continue;
                    }
                } else {
                    // Unknown symbol: the mode is a no-op for THIS order —
                    // never a panic, never a silently-wrong number (D3).
                    order.qty()
                }
            } else {
                order.qty()
            };

            let notional = fill_price * qty.get();
            let fee_amount = notional * (fee_bps / ten_k);
            let fee = Money::from_decimal(fee_amount);

            let price = Price::new(fill_price).map_err(|e| MatchError::FillError(e.to_string()))?;

            let fill = Fill {
                id: FillId::new(),
                order_id: order.id(),
                symbol: order.symbol().clone(),
                side: order.side(),
                qty,
                price,
                fee,
                fee_tier: FeeTier::Taker,
                venue_ts: bar.close_ts,
                local_ts: Timestamp::now(),
                liquidity: Liquidity::Taker,
                transaction_id: None,
            };
            fills.push(fill);
        }

        Ok(fills)
    }

    fn config(&self) -> MatchConfig {
        self.config.clone()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::{
        Bar, OrderKind, Position, Price, Quantity, RiskLimits, Side, StrategyId, Symbol,
        TimeInForce, Timeframe, Timestamp, Venue,
    };

    fn make_bar_at_close(close: Decimal) -> Bar {
        let ts = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH);
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(Decimal::ONE).unwrap(),
            trade_count: 0,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn make_order(side: Side, qty: Decimal) -> Order {
        let mark = Price::new(dec!(40_000)).unwrap();
        let pos = Position::empty(Symbol::new(""));
        let limits = RiskLimits {
            per_symbol_exposure_cap: dec!(1.0), // allow large orders in test
            price_sanity_band: dec!(0.5),
            portfolio_exposure_cap: None,
        };
        Order::new(
            StrategyId::new("test"),
            Symbol::new("BTCUSDT"),
            side,
            Quantity::new(qty).unwrap(),
            OrderKind::Market,
            TimeInForce::Ioc,
            &pos,
            mark,
            &limits,
            dec!(100_000_000), // large equity
        )
        .unwrap()
    }

    // ── ADR-0087 venue-filter seam helpers ──────────────────────────────

    fn make_bar_at_close_symbol(symbol: Symbol, close: Decimal) -> Bar {
        let ts = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH);
        Bar {
            symbol,
            tf: Timeframe::OneMinute,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(Decimal::ONE).unwrap(),
            trade_count: 0,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn make_order_symbol(symbol: Symbol, side: Side, qty: Decimal) -> Order {
        let mark = Price::new(dec!(40_000)).unwrap();
        let pos = Position::empty(Symbol::new(""));
        let limits = RiskLimits {
            per_symbol_exposure_cap: dec!(1.0), // allow large orders in test
            price_sanity_band: dec!(0.5),
            portfolio_exposure_cap: None,
        };
        Order::new(
            StrategyId::new("test"),
            symbol,
            side,
            Quantity::new(qty).unwrap(),
            OrderKind::Market,
            TimeInForce::Ioc,
            &pos,
            mark,
            &limits,
            dec!(100_000_000), // large equity — never trips the exposure cap
        )
        .unwrap()
    }

    /// T24 acceptance: `slippage_bps`=2, `taker_fee_bps`=4, `bar.close`=`40_000`
    /// buy 0.1 BTC → `fill.price`=`40_008`, `fill.fee`=1.60032 USDT
    #[tokio::test]
    async fn t24_fill_price_and_fee() {
        let config = MatchConfig {
            slippage_bps: 2,
            taker_fee_bps: 4,
            maker_fee_bps: 2,
            fill_price_mode: FillPriceMode::BarClose,
        };
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE);
        let bar = make_bar_at_close(dec!(40_000));
        let order = make_order(Side::Buy, dec!(0.1));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(fills.len(), 1);
        let fill = &fills[0];

        // fill.price = 40_000 * (1 + 2/10_000) = 40_000 * 1.0002 = 40_008
        assert_eq!(fill.price.get(), dec!(40_008));

        // notional = 40_008 * 0.1 = 4_000.8
        // fee = 4_000.8 * 4 / 10_000 = 1.60032
        assert_eq!(fill.fee.amount(), dec!(1.60032));
    }

    #[tokio::test]
    async fn t24_sell_slippage_is_negative() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::with_default_seed(config);
        let bar = make_bar_at_close(dec!(40_000));
        let order = make_order(Side::Sell, dec!(0.1));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(fills.len(), 1);
        // sell: 40_000 * (1 - 2/10_000) = 40_000 * 0.9998 = 39_992
        assert_eq!(fills[0].price.get(), dec!(39_992));
    }

    #[tokio::test]
    async fn t24_deterministic_across_runs() {
        let config = MatchConfig::default();
        let bar = make_bar_at_close(dec!(30_000));
        let order1 = make_order(Side::Buy, dec!(0.5));
        let order2 = make_order(Side::Buy, dec!(0.5));

        let mut eng1 = PaperEngine::new(config.clone(), 42);
        let mut eng2 = PaperEngine::new(config, 42);

        let fills1 = eng1.step(&bar, vec![order1]).await.unwrap();
        let fills2 = eng2.step(&bar, vec![order2]).await.unwrap();

        assert_eq!(fills1[0].price.get(), fills2[0].price.get());
        assert_eq!(fills1[0].fee.amount(), fills2[0].fee.amount());
    }

    // ── ADR-0087 § D6 — anchor-safety enforcement (T7, NEVER DELETE) ────────

    /// NEVER DELETE — D6 contract: the config default carries no venue
    /// filter (mirrors ADR-0081's `default_is_linear_bps_8`). If this ever
    /// flips, `venue_filter` stopped being opt-in-forever.
    #[test]
    fn venue_filter_default_is_none() {
        assert!(
            crate::cli_types::LatencySlippageSimConfig::default()
                .venue_filter
                .is_none()
        );
    }

    /// NEVER DELETE — D6 contract: the proof obligation that a default run
    /// (`venue_filter` never set — every existing `PaperEngine` constructor
    /// call site) is byte-identical to the pre-ADR-0087 fill path. Uses
    /// DOGEUSDT (a symbol THAT IS in the filter table) with a deliberately
    /// fractional qty that WOULD be rounded if the mode were enabled,
    /// proving the `None` path never rounds even for a table symbol.
    #[tokio::test]
    async fn paper_step_none_is_byte_identical() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE);
        let bar = make_bar_at_close_symbol(Symbol::new("DOGEUSDT"), dec!(0.30));
        let order = make_order_symbol(Symbol::new("DOGEUSDT"), Side::Buy, dec!(12.7));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(
            fills[0].qty.get(),
            dec!(12.7),
            "venue_filter=None must never round qty, even for a table symbol"
        );
        assert_eq!(engine.sim_filter_stats().skipped_min_notional, 0);
    }

    // ── ADR-0087 § D1 — seam-branch coverage (T5) ───────────────────────────

    /// Enabled mode + known table symbol + qty clearing min-notional after
    /// rounding → `Fill.qty` is the floored value (DOGEUSDT `step_size` = 1).
    #[tokio::test]
    async fn venue_filter_rounds_qty_down_when_enabled() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE)
            .with_venue_filter_mode(Some(VenueFilterMode::LotSizeAndMinNotional));
        let bar = make_bar_at_close_symbol(Symbol::new("DOGEUSDT"), dec!(1.00));
        let order = make_order_symbol(Symbol::new("DOGEUSDT"), Side::Buy, dec!(12.7));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(
            fills[0].qty.get(),
            dec!(12),
            "must floor to whole DOGE (step_size=1)"
        );
        assert_eq!(engine.sim_filter_stats().skipped_min_notional, 0);
    }

    /// Enabled mode + known table symbol + qty that rounds to a
    /// sub-min-notional notional → NO `Fill` pushed, `step` still returns
    /// `Ok` (a min-notional reject is NOT a `MatchError` — ADR-0087 § D1),
    /// and the skip is tallied.
    #[tokio::test]
    async fn venue_filter_rejects_sub_min_notional_order_no_fill_no_error() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE)
            .with_venue_filter_mode(Some(VenueFilterMode::LotSizeAndMinNotional));
        let bar = make_bar_at_close_symbol(Symbol::new("DOGEUSDT"), dec!(1.00));
        // 3.5 DOGE floors to 3; 3 * ~1.0002 ≈ 3.0006 < 5 (min_notional) → reject.
        let order = make_order_symbol(Symbol::new("DOGEUSDT"), Side::Buy, dec!(3.5));

        let result = engine.step(&bar, vec![order]).await;
        assert!(
            result.is_ok(),
            "a min-notional reject must NOT be a MatchError"
        );
        let fills = result.unwrap();
        assert!(fills.is_empty(), "a rejected order must push NO Fill");
        assert_eq!(engine.sim_filter_stats().skipped_min_notional, 1);
    }

    /// Enabled mode + a symbol NOT in the filter table → no-op for that
    /// order (fills un-rounded, never a panic, never a silently-wrong
    /// number — ADR-0087 § D3).
    #[tokio::test]
    async fn venue_filter_unknown_symbol_is_noop_when_enabled() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE)
            .with_venue_filter_mode(Some(VenueFilterMode::LotSizeAndMinNotional));
        let bar = make_bar_at_close_symbol(Symbol::new("SHIBUSDT"), dec!(40_000));
        // `make_order_symbol`'s `Order::new` exposure check uses a fixed
        // mark of 40_000 against a 100_000_000 equity — keep qty small
        // enough to clear the 40% exposure cap regardless of symbol.
        let order = make_order_symbol(Symbol::new("SHIBUSDT"), Side::Buy, dec!(0.123456));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(
            fills[0].qty.get(),
            dec!(0.123456),
            "unknown symbol under an enabled mode must fill un-rounded (no-op)"
        );
        assert_eq!(engine.sim_filter_stats().skipped_min_notional, 0);
    }

    /// `with_venue_filter_mode` is a builder: chaining preserves the rest of
    /// the engine's construction (seed, config) unchanged.
    #[tokio::test]
    async fn with_venue_filter_mode_is_a_pure_builder() {
        let config = MatchConfig::default();
        let mut engine = PaperEngine::new(config, 0x00C0_FFEE).with_venue_filter_mode(None);
        let bar = make_bar_at_close(dec!(40_000));
        let order = make_order(Side::Buy, dec!(0.1));

        let fills = engine.step(&bar, vec![order]).await.unwrap();
        assert_eq!(
            fills[0].price.get(),
            dec!(40_008),
            "unrelated config unaffected"
        );
    }
}
