//! OHLCV bar type.
use serde::{Deserialize, Serialize};

use crate::money::{Price, Quantity};
use crate::symbol::Symbol;
use crate::time::Timestamp;
use crate::venue::Venue;

/// Candlestick timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timeframe {
    /// 1-second bars, aggregated client-side from the raw `Tick` stream.
    ///
    /// Bucketing is deterministic on epoch microseconds: the bucket key
    /// is `floor(tick.venue_ts.unix_micros() / 1_000_000)` (UTC second
    /// boundary). Empty seconds emit no bar. See
    /// `spec/features/v1-5b-multi-venue.md` Q5.
    OneSecond,
    /// 1-minute bars (v0 default).
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Timeframe::OneSecond => "1s",
            Timeframe::OneMinute => "1m",
            Timeframe::FiveMinutes => "5m",
            Timeframe::FifteenMinutes => "15m",
            Timeframe::OneHour => "1h",
            Timeframe::FourHours => "4h",
            Timeframe::OneDay => "1d",
        };
        write!(f, "{s}")
    }
}

/// An OHLCV candlestick bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub symbol: Symbol,
    pub tf: Timeframe,
    /// Venue timestamp — bar open.
    pub open_ts: Timestamp,
    /// Venue timestamp — bar close.
    pub close_ts: Timestamp,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    /// Volume in base-asset units.
    pub volume: Quantity,
    pub trade_count: u32,
    /// Local receive timestamp, used for clock-skew detection (R1.3).
    pub local_recv_ts: Timestamp,
    /// Originating exchange (v1.5b multi-venue, Q4 — required, not Option).
    pub venue: Venue,
}
