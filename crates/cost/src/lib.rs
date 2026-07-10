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

/// Static venue lot-size / min-notional filter table (ADR-0087, opt-in-forever).
/// Backtest-only exec-sim realism mode — see module docs for the full contract.
pub mod venue_filter;

pub use budget::{BudgetError, CostBudget};
pub use event::{AgentRole, CostEvent, InfraLine, LlmTier, ProviderKind};
pub use sink::{CostSink, LedgerCostSink, NoopCostSink};
pub use slippage::{
    DEFAULT_VOL_SCALED_SPREAD, MAX_SLIPPAGE_BPS, SlippageModel, apply_slippage,
    apply_slippage_model, apply_slippage_model_with_returns, fee_sensitivity_report,
};
pub use venue_filter::{
    SNAPSHOT_DATE as VENUE_FILTER_SNAPSHOT_DATE, VenueFilter, round_down_to_step, venue_filter_for,
};
