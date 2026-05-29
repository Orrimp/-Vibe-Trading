//! Cost telemetry scaffold (R10).
//!
//! Ships the full surface in v0 with zero emitters.
//! v0.5 drops `cost_sink.record(CostEvent::Llm { .. })` calls at emit sites.

pub mod budget;
pub mod event;
pub mod sink;

/// Linear-bps slippage simulation for backtest fills (v5-latency-slippage-sim R3).
/// Backtest-only — not called from live-mode paths (ADR-0043 § D5).
pub mod slippage;

pub use budget::{BudgetError, CostBudget};
pub use event::{AgentRole, CostEvent, InfraLine, LlmTier, ProviderKind};
pub use sink::{CostSink, LedgerCostSink, NoopCostSink};
pub use slippage::{MAX_SLIPPAGE_BPS, SlippageModel, apply_slippage, apply_slippage_model};
