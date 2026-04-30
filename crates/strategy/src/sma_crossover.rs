//! SMA crossover strategy — stub for T01, full in T22.
use features::Sma;
use serde_json::Value;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Tick};

use crate::Strategy;

/// SMA crossover: emits Buy when fast > slow, Sell when fast < slow.
pub struct SmaCrossover {
    id: StrategyId,
    fast: Sma,
    slow: Sma,
    #[allow(dead_code)] // used for config_schema()
    fast_len: usize,
    #[allow(dead_code)] // used for config_schema()
    slow_len: usize,
}

impl SmaCrossover {
    #[must_use]
    pub fn new(fast_len: usize, slow_len: usize) -> Self {
        Self {
            id: StrategyId::new("sma_crossover"),
            fast: Sma::new(fast_len),
            slow: Sma::new(slow_len),
            fast_len,
            slow_len,
        }
    }
}

impl Strategy for SmaCrossover {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        let close = bar.close.get();
        let fast_val = self.fast.push(close);
        let slow_val = self.slow.push(close);

        match (fast_val, slow_val) {
            (Some(f), Some(s)) => {
                let epsilon = rust_decimal::Decimal::new(1, 8); // 1 satoshi
                let kind = if f > s + epsilon {
                    SignalKind::Buy
                } else if f < s - epsilon {
                    SignalKind::Sell
                } else {
                    SignalKind::Hold
                };
                vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind,
                    evidence: SignalEvidence::sma(f, s),
                    pair_data: None, // v1.5a — not a pair signal
                }]
            }
            _ => vec![],
        }
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        // v0: SMA is a bar-close strategy; tick-level signals unsupported.
        vec![]
    }

    fn config_schema() -> Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "type": "object",
            "properties": {
                "fast_len": { "type": "integer", "default": 20 },
                "slow_len": { "type": "integer", "default": 50 }
            },
            "required": ["fast_len", "slow_len"]
        })
    }
}
