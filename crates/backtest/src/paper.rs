//! `PaperEngine` — simple bps slippage + taker fee matching engine.
//!
//! v0 ships this as the only `MatchingEngine` implementation.
//! Full implementation in T24; this is the stub.
use async_trait::async_trait;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use trading_core::{Bar, FeeTier, Fill, FillId, Liquidity, Money, Order, Price, Side, Timestamp};

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

/// Simple paper matching engine.
pub struct PaperEngine {
    config: MatchConfig,
    /// Seeded RNG for deterministic tie-breaking (T24 will use it more extensively).
    #[allow(dead_code)]
    rng: ChaCha20Rng,
}

impl PaperEngine {
    #[must_use]
    pub fn new(config: MatchConfig, seed: u64) -> Self {
        Self {
            config,
            rng: ChaCha20Rng::seed_from_u64(seed),
        }
    }

    #[must_use]
    pub fn with_default_seed(config: MatchConfig) -> Self {
        Self::new(config, 0x00C0_FFEE)
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

            let notional = fill_price * order.qty().get();
            let fee_amount = notional * (fee_bps / ten_k);
            let fee = Money::from_decimal(fee_amount);

            let price = Price::new(fill_price).map_err(|e| MatchError::FillError(e.to_string()))?;

            let fill = Fill {
                id: FillId::new(),
                order_id: order.id(),
                symbol: order.symbol().clone(),
                side: order.side(),
                qty: order.qty(),
                price,
                fee,
                fee_tier: FeeTier::Taker,
                venue_ts: bar.close_ts,
                local_ts: Timestamp::now(),
                liquidity: Liquidity::Taker,
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
        TimeInForce, Timeframe, Timestamp,
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
        }
    }

    fn make_order(side: Side, qty: Decimal) -> Order {
        let mark = Price::new(dec!(40_000)).unwrap();
        let pos = Position::empty(Symbol::new(""));
        let limits = RiskLimits {
            per_symbol_exposure_cap: dec!(1.0), // allow large orders in test
            price_sanity_band: dec!(0.5),
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
}
