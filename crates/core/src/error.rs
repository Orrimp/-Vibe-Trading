//! Shared error types for the `core` crate.
use rust_decimal::Decimal;
use thiserror::Error;

/// Error constructing a `Quantity`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QtyError {
    #[error("quantity must be non-negative, got {0}")]
    Negative(Decimal),
}

/// Error constructing a `Price`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PriceError {
    #[error("price must be strictly positive, got {0}")]
    NonPositive(Decimal),
}

/// Error constructing or validating an `Order`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrderError {
    #[error("quantity must be strictly positive: {0}")]
    NonPositiveQty(Decimal),
    #[error("price must be strictly positive: {0}")]
    NonPositivePrice(Decimal),
    #[error("asset mismatch: order symbol {symbol} does not match position symbol {position}")]
    AssetMismatch { symbol: String, position: String },
    #[error("price {price} outside sanity band [{lo}, {hi}]")]
    PriceOutsideBand {
        price: Decimal,
        lo: Decimal,
        hi: Decimal,
    },
    #[error("exposure breach: proposed notional {proposed} would exceed cap {cap}")]
    ExposureBreach { proposed: Decimal, cap: Decimal },
    #[error("risk error: {0}")]
    Risk(#[from] RiskError),
    #[error("qty error: {0}")]
    Qty(#[from] QtyError),
    #[error("price error: {0}")]
    Price(#[from] PriceError),
}

/// Risk-limit violation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RiskError {
    #[error("exposure cap breached: proposed {proposed} > cap {cap}")]
    ExposureCap { proposed: Decimal, cap: Decimal },
    #[error("daily loss stop hit: daily pnl {pnl_pct}% < stop {stop_pct}%")]
    DailyLossStop { pnl_pct: Decimal, stop_pct: Decimal },
    #[error("max drawdown stop hit: drawdown {drawdown_pct}% < stop {stop_pct}%")]
    MaxDrawdownStop {
        drawdown_pct: Decimal,
        stop_pct: Decimal,
    },
    #[error("unsupported mode: {0}")]
    UnsupportedMode(String),
}

/// Ledger / audit error.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("imbalance detected: debits {debits} != credits {credits}")]
    Imbalance { debits: Decimal, credits: Decimal },
    #[error("transaction failed: {0}")]
    TransactionFailed(String),
    #[error("database error: {0}")]
    Database(String),
}

/// Feed / data error.
#[derive(Debug, Error)]
pub enum FeedError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("stream closed")]
    StreamClosed,
    #[error("clock skew: local={local_ms}ms venue={venue_ms}ms delta={delta_ms}ms")]
    ClockSkew {
        local_ms: i64,
        venue_ms: i64,
        delta_ms: i64,
    },
    #[error("io error: {0}")]
    Io(String),
}

/// Configuration error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("unsupported mode: '{0}'. v0 supports 'research' and 'paper' only.")]
    UnsupportedMode(String),
    #[error("invalid value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },
    #[error("missing required field: '{0}'")]
    MissingField(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("fast_len ({fast}) must be strictly less than slow_len ({slow}) for SMA")]
    SmaWindowOrder { fast: usize, slow: usize },
}

/// Strategy error.
#[derive(Debug, Error)]
pub enum StrategyError {
    #[error("strategy not found: {0}")]
    NotFound(String),
    #[error("strategy already registered: {0}")]
    AlreadyRegistered(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// Cost tracking error.
#[derive(Debug, Error)]
pub enum CostError {
    #[error("ledger error: {0}")]
    Ledger(#[from] LedgerError),
    #[error("record failed: {0}")]
    RecordFailed(String),
}
