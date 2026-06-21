//! SMA crossover strategy — stub for T01, full in T22.
use features::Sma;
use serde_json::Value;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Tick};

use crate::Strategy;
use crate::plan::{
    PlanContext, PlanDescribe, PlanRuleShape, PlanSignal, PlanStance, ProjectedSizing, StrategyPlan,
};

/// SMA crossover: emits Buy when fast > slow, Sell when fast < slow.
pub struct SmaCrossover {
    id: StrategyId,
    fast: Sma,
    slow: Sma,
    fast_len: usize,
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

// ── PlanDescribe impl for SmaCrossover ────────────────────────────────────────

impl PlanDescribe for SmaCrossover {
    /// Snapshot the current SMA stance + rules **without mutating indicator state**.
    ///
    /// Stance is derived by reading the current fast/slow SMA values from the
    /// already-warmed `SmaStream` (via the non-mutating `current()` getter) and
    /// applying the SAME comparison `on_bar` uses:
    ///   `Long` iff `fast > slow + epsilon`, `Flat` otherwise.
    ///
    /// `ctx.last_close` is NOT pushed into the indicators — this is a read-only
    /// snapshot of standing decision as of the last consumed bar (ADR-0062 § D2).
    fn describe_plan(&self, ctx: &PlanContext) -> StrategyPlan {
        let epsilon = rust_decimal::Decimal::new(1, 8);

        let (stance, latest_signal) = match (self.fast.current(), self.slow.current()) {
            (Some(f), Some(s)) => {
                let signal = if f > s + epsilon {
                    PlanSignal::Buy
                } else if f < s - epsilon {
                    PlanSignal::Sell
                } else {
                    PlanSignal::Hold
                };
                let stance = if f > s + epsilon {
                    PlanStance::Long
                } else {
                    PlanStance::Flat
                };
                (stance, Some(signal))
            }
            // Indicators not yet warmed — insufficient bars; report Flat / no signal.
            _ => (PlanStance::Flat, None),
        };

        let sizing = ProjectedSizing::compute(ctx.budget, ctx.budget_cap, ctx.last_close);

        StrategyPlan {
            stance,
            latest_signal,
            rule: PlanRuleShape::SmaCross {
                fast_len: self.fast_len,
                slow_len: self.slow_len,
            },
            sizing,
        }
    }
}
