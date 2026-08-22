//! Core domain types for the trading agent.
//!
//! All prices, sizes, balances, fees, and P&L are `Decimal`-backed.
//! No `f64` for money, anywhere.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod activity;
pub mod asset;
pub mod bar;
pub mod equity_series;
pub mod error;
pub mod fill;
pub mod forecast;
pub mod funding;
/// Per-bar funding-rate constant for single-coin directional shorts (ADR-0068 D4).
pub mod funding_rate;
pub mod fx;
pub mod halt;
pub mod money;
pub mod order;
pub mod pair;
pub mod pit;
pub mod position;
pub mod signal;
pub mod strategy_events;
pub mod symbol;
#[cfg(test)]
mod tests;
pub mod tick;
pub mod time;
pub mod universe;
pub mod venue;
pub mod views;

pub use activity::{ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase};
pub use asset::{Asset, Btc, Currency, Usdt};
pub use bar::{Bar, Timeframe};
pub use equity_series::{BacktestMetrics, EquityPoint, EquitySeries, EquitySeriesError};
pub use error::{
    ConfigError, CostError, FeedError, LedgerError, OrderError, PriceError, QtyError, RiskError,
    StrategyError,
};
pub use fill::{FeeTier, Fill, FillId, Liquidity};
pub use forecast::{
    Direction, ForecastError, ForecastOverlay, ForecastRequest, ForecastResponse, OhlcvBar,
    SamplingParams,
};
pub use funding::FundingObs;
pub use funding_rate::{DEFAULT_PERP_FUNDING_RATE, FundingRate, FundingRateError};
pub use fx::{BudgetConversion, DEFAULT_EUR_USD_RATE, FxNote, FxRate, FxRateError};
pub use halt::HaltReason;
pub use money::Money;
pub use money::{Price, Quantity};
pub use order::{Order, OrderId, OrderKind, ProposedOrder, RiskLimits, TimeInForce};
pub use pair::{Pair, PairError, PairKey, PairMembership};
pub use pit::{AsOf, PitError, PitSeries, TimestampMs};
pub use position::{OpenPosition, Position};
pub use signal::{Decision, PairSignalData, Signal, SignalEvidence, SignalKind, StopReason};
pub use strategy_events::{
    StrategyEventKind, StrategyEventView, StrategyLoadError, StrategyLoaded, StrategySwapped,
};
pub use symbol::{AccountId, Side, StrategyId, Symbol};
pub use tick::Tick;
pub use time::Timestamp;
pub use universe::{SymbolSet, Universe, UniverseError};
pub use venue::{MarketHealth, ParseVenueError, RiskTelemetry, Venue};
pub use views::{
    AuditKindFilter, AuditKindLabel, FillView, JournalEntry, JournalEntryView, JournalRow,
    JournalTransactionMetadata, OrphanTrainingRun, PnlSnapshot, PositionView, SignalView,
    TrainingEventRow, TrainingRunSummary,
};
