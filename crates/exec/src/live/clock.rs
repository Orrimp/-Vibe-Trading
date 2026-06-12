//! Clock-skew handling (R5 / feature.md § A1 Wave B4).
//!
//! `ServerTimeOffset` syncs to `GET /api/v3/time` on construction and on any
//! `-1021` (timestamp outside recvWindow) error.  Persistent skew beyond a
//! threshold maps to `HaltReason::ClockSkew`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::live::error::ExecError;

/// Maximum acceptable absolute offset between local and server time.
///
/// Binance `recvWindow` default is 5000 ms; we allow up to 3000 ms of skew
/// (1.5× the typical 2000 ms leeway) before triggering a halt.
pub const MAX_SKEW_MS: i64 = 3_000;

/// Tracks the offset between local clock and Binance server time.
///
/// offset_ms = server_time_ms - local_time_ms (positive = local is behind).
#[derive(Debug, Clone, Default)]
pub struct ServerTimeOffset {
    offset_ms: i64,
}

impl ServerTimeOffset {
    /// Construct with a known offset (typically from a `GET /api/v3/time` call).
    pub fn new(offset_ms: i64) -> Self {
        Self { offset_ms }
    }

    /// Return the current adjusted timestamp (ms since Unix epoch).
    ///
    /// `adjusted = local_now_ms + offset_ms`
    #[must_use]
    pub fn adjusted_now_ms(&self) -> u64 {
        let local_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as i64;
        (local_ms + self.offset_ms).max(0) as u64
    }

    /// Update the offset from a freshly-read server time.
    pub fn sync(&mut self, server_time_ms: u64) {
        let local_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as i64;
        self.offset_ms = server_time_ms as i64 - local_ms;
        tracing::debug!(offset_ms = self.offset_ms, "clock synced to server time");
    }

    /// Returns `Err(ExecError::ClockSkew)` if the current offset exceeds the
    /// maximum threshold — caller should halt via `KillSwitch::trip`.
    ///
    /// # Errors
    /// [`ExecError::ClockSkew`] when `|offset_ms| > MAX_SKEW_MS`.
    pub fn check_persistent(&self) -> Result<(), ExecError> {
        if self.offset_ms.abs() > MAX_SKEW_MS {
            tracing::warn!(
                offset_ms = self.offset_ms,
                max_skew_ms = MAX_SKEW_MS,
                "persistent clock skew — halt recommended"
            );
            Err(ExecError::ClockSkew)
        } else {
            Ok(())
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Wave B4: `clock_skew_resyncs_then_halts` — persistent skew triggers
    /// `ExecError::ClockSkew`.
    #[test]
    fn clock_skew_resyncs_then_halts() {
        // Small offset — no halt.
        let ok = ServerTimeOffset::new(100);
        assert!(ok.check_persistent().is_ok());

        // Large offset — halt.
        let bad = ServerTimeOffset::new(MAX_SKEW_MS + 1);
        assert!(matches!(bad.check_persistent(), Err(ExecError::ClockSkew)));

        // Negative large offset — also halt.
        let bad_neg = ServerTimeOffset::new(-(MAX_SKEW_MS + 1));
        assert!(matches!(
            bad_neg.check_persistent(),
            Err(ExecError::ClockSkew)
        ));
    }

    #[test]
    fn adjusted_now_ms_is_reasonable() {
        let c = ServerTimeOffset::new(0);
        let t = c.adjusted_now_ms();
        // Must be after 2024-01-01 (1704067200000 ms)
        assert!(t > 1_704_067_200_000, "timestamp looks wrong: {t}");
    }

    #[test]
    fn sync_updates_offset() {
        let mut c = ServerTimeOffset::new(0);
        // Pretend server is 500 ms ahead.
        let server_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 500;
        c.sync(server_ms);
        // Offset should be close to 500 ms (within 100 ms timing jitter).
        assert!(
            c.offset_ms.abs() >= 400 && c.offset_ms.abs() <= 600,
            "offset should be ~500 ms, got {}",
            c.offset_ms
        );
    }
}
