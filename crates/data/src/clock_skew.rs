//! Clock-skew detector (R1.3, T11).
//!
//! Watches venue timestamps vs local monotonic time and emits `WARN` logs
//! when skew exceeds `warn_ms` for 3 consecutive samples, or trips the kill
//! switch (returns `ObserveResult::TripKillSwitch`) when skew exceeds `halt_ms`.
//!
//! The `clock_skew_ms{feed}` Prometheus gauge is updated on every observation.

use tracing::{info, warn};

// ── Config ────────────────────────────────────────────────────────────────────

/// Configuration for clock-skew detection.
#[derive(Debug, Clone)]
pub struct ClockSkewConfig {
    /// Warn if `|local_ms - venue_ms| >= warn_ms` for 3 consecutive messages.
    pub warn_ms: i64,
    /// Trip kill switch if `|local_ms - venue_ms| >= halt_ms`.
    pub halt_ms: i64,
    /// Feed label used in tracing / metrics, e.g. `"binance"`.
    pub feed_label: String,
}

impl Default for ClockSkewConfig {
    fn default() -> Self {
        Self {
            warn_ms: 2_000,
            halt_ms: 10_000,
            feed_label: "unknown".into(),
        }
    }
}

impl ClockSkewConfig {
    /// Builder: set the feed label.
    #[must_use]
    pub fn with_feed(mut self, label: impl Into<String>) -> Self {
        self.feed_label = label.into();
        self
    }
}

// ── Result ────────────────────────────────────────────────────────────────────

/// Result of a single skew observation.
#[derive(Debug, PartialEq, Eq)]
pub enum ObserveResult {
    /// Skew is within acceptable bounds.
    Ok,
    /// Skew exceeded `warn_ms` for `consecutive` consecutive messages.
    Warn { delta_ms: i64, consecutive: u32 },
    /// Skew exceeded `halt_ms` — kill switch should be tripped.
    TripKillSwitch { delta_ms: i64 },
}

// ── Detector ─────────────────────────────────────────────────────────────────

/// Stateful detector: tracks consecutive over-threshold samples.
pub struct ClockSkewDetector {
    config: ClockSkewConfig,
    consecutive_warn: u32,
}

impl ClockSkewDetector {
    /// Create a new detector with the given config.
    #[must_use]
    pub fn new(config: ClockSkewConfig) -> Self {
        Self {
            config,
            consecutive_warn: 0,
        }
    }

    /// Observe a `(local_ms, venue_ms)` pair.
    ///
    /// - If `|delta| >= halt_ms`: returns `TripKillSwitch`, emits `ERROR` log.
    /// - If `|delta| >= warn_ms` for 3+ consecutive samples: returns `Warn`,
    ///   emits `WARN` log.
    /// - Otherwise: returns `Ok`, resets the consecutive-warn counter.
    ///
    /// The `clock_skew_ms{feed}` gauge is recorded on every call.
    pub fn observe(&mut self, local_ms: i64, venue_ms: i64) -> ObserveResult {
        let delta = (local_ms - venue_ms).abs();

        // Update Prometheus gauge
        metrics::gauge!(
            "clock_skew_ms",
            "feed" => self.config.feed_label.clone()
        )
        .set(delta as f64);

        if delta >= self.config.halt_ms {
            self.consecutive_warn = 0;
            tracing::error!(
                feed = %self.config.feed_label,
                delta_ms = delta,
                halt_ms = self.config.halt_ms,
                "clock skew exceeds halt threshold — TRIP KILL SWITCH"
            );
            return ObserveResult::TripKillSwitch { delta_ms: delta };
        }

        if delta >= self.config.warn_ms {
            self.consecutive_warn += 1;
            if self.consecutive_warn >= 3 {
                warn!(
                    feed = %self.config.feed_label,
                    delta_ms = delta,
                    consecutive = self.consecutive_warn,
                    warn_ms = self.config.warn_ms,
                    "clock skew above warn threshold for {} consecutive samples",
                    self.consecutive_warn
                );
                return ObserveResult::Warn {
                    delta_ms: delta,
                    consecutive: self.consecutive_warn,
                };
            }
        } else {
            if self.consecutive_warn > 0 {
                info!(
                    feed = %self.config.feed_label,
                    "clock skew back within bounds"
                );
            }
            self.consecutive_warn = 0;
        }

        ObserveResult::Ok
    }

    /// Current consecutive warn count.
    #[must_use]
    pub fn consecutive_warn(&self) -> u32 {
        self.consecutive_warn
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> ClockSkewDetector {
        ClockSkewDetector::new(ClockSkewConfig {
            warn_ms: 2_000,
            halt_ms: 10_000,
            feed_label: "test".into(),
        })
    }

    #[test]
    fn t11_ok_when_within_bounds() {
        let mut d = detector();
        assert_eq!(d.observe(1000, 1500), ObserveResult::Ok);
    }

    #[test]
    fn t11_warn_after_3_consecutive_over_threshold() {
        let mut d = detector();
        let local = 0;
        let venue = 3000; // delta = 3000 ms > 2000 warn_ms
        assert_eq!(d.observe(local, venue), ObserveResult::Ok); // 1st
        assert_eq!(d.observe(local, venue), ObserveResult::Ok); // 2nd
                                                                // 3rd consecutive — should warn
        assert!(matches!(
            d.observe(local, venue),
            ObserveResult::Warn { .. }
        ));
    }

    #[test]
    fn t11_trip_kill_switch_at_halt_threshold() {
        let mut d = detector();
        // 15 000 ms in the past — exceeds halt_ms (10 000)
        let result = d.observe(0, 15_000);
        assert!(
            matches!(result, ObserveResult::TripKillSwitch { delta_ms: 15_000 }),
            "expected TripKillSwitch, got {result:?}"
        );
    }

    #[test]
    fn t11_consecutive_resets_after_ok() {
        let mut d = detector();
        let local = 0;
        let venue = 3000;
        d.observe(local, venue);
        d.observe(local, venue);
        // Back within bounds
        d.observe(local, 100); // delta = 100 ms — ok, resets counter
        assert_eq!(d.consecutive_warn(), 0);
        // Need 3 consecutive again
        assert_eq!(d.observe(local, venue), ObserveResult::Ok);
    }
}
