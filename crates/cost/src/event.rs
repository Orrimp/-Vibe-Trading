//! `CostEvent` enum per architecture.md cost telemetry section.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// LLM provider identifier.
///
/// Renamed from `LlmProvider` → `ProviderKind` in v2 to free the
/// `LlmProvider` name for the `llm::LlmProvider` trait. The `serde`
/// `rename_all = "snake_case"` representation is unchanged (variant
/// names serialize identically), so on-the-wire / on-disk records
/// remain compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    OpenRouter,
    DeepSeek,
    Other(String),
}

/// LLM tier: affects latency and cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmTier {
    DeepThink,
    QuickThink,
}

impl std::fmt::Display for LlmTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmTier::DeepThink => write!(f, "deep_think"),
            LlmTier::QuickThink => write!(f, "quick_think"),
        }
    }
}

/// Which agent role emitted the cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Trader,
    SentimentAnalyst,
    RiskManager,
    PortfolioManager,
    Other(String),
}

/// Infrastructure cost line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfraLine {
    VpsHosting,
    CloudStorage,
    BandwidthEgress,
    Other(String),
}

/// A cost event to be recorded in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CostEvent {
    Llm {
        provider: ProviderKind,
        model: String,
        tier: LlmTier,
        role: AgentRole,
        tokens_in: u64,
        tokens_out: u64,
        tokens_cached_in: u64,
        usd: Decimal,
        correlation_id: Uuid,
    },
    /// v1+ infrastructure cost line.
    Infra {
        line: InfraLine,
        usd: Decimal,
        period_month: String,
    },
    /// v1+ data feed cost.
    Data {
        feed: String,
        usd: Decimal,
        period_month: String,
    },
    /// v1+ storage cost.
    Storage {
        bytes: u64,
        usd: Decimal,
        period_month: String,
    },
}

impl CostEvent {
    /// USD cost of this event.
    pub fn usd(&self) -> Decimal {
        match self {
            CostEvent::Llm { usd, .. }
            | CostEvent::Infra { usd, .. }
            | CostEvent::Data { usd, .. }
            | CostEvent::Storage { usd, .. } => *usd,
        }
    }
}
