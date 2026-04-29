//! Funding-rate observation type (v1 Q2 — T601).
//!
//! `FundingObs` is broadcast on `agent::EventBus::funding_obs` once per hour
//! per universe symbol.  The v1 `MomentumStrategy` does NOT consume it —
//! the channel exists so v1.5+ strategies and operator success reports can
//! read funding history without another ingest-path build.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::symbol::Symbol;
use crate::time::Timestamp;

/// A single funding-rate observation for a perpetual contract symbol.
///
/// Published on `agent::EventBus::funding_obs` (capacity 32) at each
/// hourly poll.  Persisted to the `funding_rates` table (see migration `003`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingObs {
    /// Perpetual contract symbol (e.g. `"BTCUSDT"`).
    ///
    /// Note: this is the perp symbol; spot mapping lives in `Universe::base_asset`.
    pub symbol: Symbol,
    /// 8-hour funding rate (e.g. `0.00010000` = 0.01%).
    pub funding_rate: Decimal,
    /// Venue-published timestamp of this funding settlement.
    pub funding_ts: Timestamp,
    /// Next scheduled funding settlement timestamp.
    pub next_funding_ts: Timestamp,
    /// Wall-clock timestamp when we polled (audit trail).
    pub poll_ts: Timestamp,
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn dummy_ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH)
    }

    #[test]
    fn t601_funding_obs_round_trip() {
        let obs = FundingObs {
            symbol: Symbol::new("BTCUSDT"),
            funding_rate: rust_decimal_macros::dec!(0.0001),
            funding_ts: dummy_ts(),
            next_funding_ts: dummy_ts(),
            poll_ts: dummy_ts(),
        };
        let json = serde_json::to_string(&obs).expect("serialize");
        let back: FundingObs = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(obs.symbol, back.symbol);
        assert_eq!(obs.funding_rate, back.funding_rate);
    }
}
