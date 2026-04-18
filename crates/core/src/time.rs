//! Timestamp wrapper around `time::OffsetDateTime`.
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A point in time carrying a UTC `OffsetDateTime`.
/// Used for both venue timestamps and local receive timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] pub OffsetDateTime);

impl Timestamp {
    /// Create a `Timestamp` from an `OffsetDateTime`.
    #[must_use]
    pub fn new(dt: OffsetDateTime) -> Self {
        Self(dt)
    }

    /// Returns the inner `OffsetDateTime`.
    #[must_use]
    pub fn inner(&self) -> OffsetDateTime {
        self.0
    }

    /// Current time in UTC.
    #[must_use]
    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    /// Unix timestamp in milliseconds.
    #[must_use]
    pub fn unix_millis(&self) -> i64 {
        let nanos = self.0.unix_timestamp_nanos();
        // i128 -> i64: timestamps in ms fit comfortably in i64 for any
        // date within ±292 years of the Unix epoch.
        i64::try_from(nanos / 1_000_000).unwrap_or(i64::MAX)
    }
}

impl From<OffsetDateTime> for Timestamp {
    fn from(dt: OffsetDateTime) -> Self {
        Self(dt)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
