//! Venue (exchange) identifier and market-health event types.
//!
//! `Venue` is a closed enum — adding a new venue is a deliberate
//! multi-week analyst → architect → developer round (each new venue
//! ships an adapter + symbol normalization + rate-limit budget),
//! never a one-line config edit. Exhaustive `match` catches new
//! venues at compile time.
//!
//! See `spec/features/v1-5b-multi-venue.md` Q1, Q4, Q7.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::time::Timestamp;

/// Closed set of exchanges supported by the agent.
///
/// `Ord` is derived alphabetically — `Binance < Coinbase < Kraken`
/// matches the v1.5b feature brief's R7.4 tie-break order.
///
/// **No `Default` impl** — every `Bar` / `Tick` must construct
/// `Venue` explicitly so a venue is always declared at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue {
    Binance,
    Coinbase,
    Kraken,
}

impl fmt::Display for Venue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Venue::Binance => "binance",
            Venue::Coinbase => "coinbase",
            Venue::Kraken => "kraken",
        };
        f.write_str(s)
    }
}

/// Error returned by `Venue::from_str` for unknown venue strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVenueError {
    pub input: String,
}

impl fmt::Display for ParseVenueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown venue: {:?}", self.input)
    }
}

impl std::error::Error for ParseVenueError {}

impl FromStr for Venue {
    type Err = ParseVenueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "binance" => Ok(Venue::Binance),
            "coinbase" => Ok(Venue::Coinbase),
            "kraken" => Ok(Venue::Kraken),
            _ => Err(ParseVenueError {
                input: s.to_string(),
            }),
        }
    }
}

/// Per-venue market-data freshness event published on the bus
/// (`EventBus::market_health`). Strategies pause / resume per
/// venue based on these events.
///
/// See `spec/features/v1-5b-multi-venue.md` Q7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketHealth {
    /// Venue is producing fresh ticks (received within threshold).
    Fresh {
        venue: Venue,
        last_tick_ts: Timestamp,
    },
    /// No tick received within `threshold_secs` — strategies should pause.
    Stale {
        venue: Venue,
        last_tick_ts: Timestamp,
        threshold_secs: u32,
    },
    /// Venue produced a tick again after being stale.
    Recovered {
        venue: Venue,
        recovered_ts: Timestamp,
        gap_secs: u32,
    },
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    #[test]
    fn venue_display_lowercase() {
        assert_eq!(Venue::Binance.to_string(), "binance");
        assert_eq!(Venue::Coinbase.to_string(), "coinbase");
        assert_eq!(Venue::Kraken.to_string(), "kraken");
    }

    #[test]
    fn venue_from_str_round_trip() {
        for v in [Venue::Binance, Venue::Coinbase, Venue::Kraken] {
            let s = v.to_string();
            let parsed: Venue = s.parse().unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn venue_from_str_unknown_errors() {
        let err = "ftx".parse::<Venue>().unwrap_err();
        assert_eq!(err.input, "ftx");
    }

    #[test]
    fn venue_serde_round_trip() {
        for v in [Venue::Binance, Venue::Coinbase, Venue::Kraken] {
            let json = serde_json::to_string(&v).unwrap();
            // snake_case JSON encoding
            assert_eq!(json, format!("\"{}\"", v));
            let back: Venue = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn venue_ord_alphabetical() {
        assert!(Venue::Binance < Venue::Coinbase);
        assert!(Venue::Coinbase < Venue::Kraken);
    }

    #[test]
    fn market_health_serde_round_trip() {
        let ts = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH);
        let evt = MarketHealth::Stale {
            venue: Venue::Coinbase,
            last_tick_ts: ts,
            threshold_secs: 30,
        };
        let json = serde_json::to_string(&evt).unwrap();
        let back: MarketHealth = serde_json::from_str(&json).unwrap();
        // Compare round-tripped via JSON
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }
}
