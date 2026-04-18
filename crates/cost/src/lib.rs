//! Cost telemetry scaffold (R10).
//!
//! Ships the full surface in v0 with zero emitters.
//! v0.5 drops `cost_sink.record(CostEvent::Llm { .. })` calls at emit sites.

pub mod budget;
pub mod event;
pub mod sink;

pub use budget::CostBudget;
pub use event::{AgentRole, CostEvent, InfraLine, LlmProvider, LlmTier};
pub use sink::{CostSink, LedgerCostSink, NoopCostSink};
