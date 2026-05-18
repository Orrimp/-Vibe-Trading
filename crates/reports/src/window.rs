//! Report window enum + parser (R1.2).
//!
//! Accepts seven shapes and rejects malformed inputs:
//!
//! | Input shape       | Variant                |
//! |-------------------|------------------------|
//! | `7d`              | [`ReportWindow::Days7`] |
//! | `30d`             | [`ReportWindow::Days30`] |
//! | `90d`             | [`ReportWindow::Days90`] |
//! | `weekly`          | [`ReportWindow::Weekly`] |
//! | `monthly`         | [`ReportWindow::Monthly`] |
//! | `since:<RFC3339>` | [`ReportWindow::Since`] |
//! | `inception`       | [`ReportWindow::Inception`] |

use thiserror::Error;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use trading_core::Timestamp;

/// All accepted report windows for the operator-success-report binary
/// (`--period <window>`).  See [crate-level docs](crate) for the seven
/// accepted shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportWindow {
    /// `7d` — rolling 7-day window ending at `now`.
    Days7,
    /// `30d` — rolling 30-day window ending at `now`.
    Days30,
    /// `90d` — rolling 90-day window ending at `now`.
    Days90,
    /// `weekly` — same window as [`ReportWindow::Days7`] but front-matter
    /// `period:` slug reads `weekly` (operator default for cron — R12.1).
    Weekly,
    /// `monthly` — same window as [`ReportWindow::Days30`] but the
    /// front-matter slug reads `monthly`.
    Monthly,
    /// `since:<RFC3339>` — explicit start timestamp; `until` is `now`.
    Since(Timestamp),
    /// `inception` — covers the entire ledger
    /// (`since = ledger_inception_ts`, `until = now`).
    Inception,
}

/// Errors returned by [`ReportWindow::parse`].
#[derive(Debug, Error)]
pub enum WindowParseError {
    /// Input did not match any of the seven accepted shapes.
    #[error(
        "malformed window '{0}' — expected 7d|30d|90d|weekly|monthly|since:<RFC3339>|inception"
    )]
    Malformed(String),
    /// Input was `since:<ts>` but `<ts>` did not parse as RFC3339.
    #[error("malformed since:<ts> — RFC3339 parse error: {0}")]
    BadTimestamp(String),
}

impl ReportWindow {
    /// Parse a `--period` argument into a [`ReportWindow`].
    ///
    /// # Errors
    ///
    /// Returns [`WindowParseError::Malformed`] for unrecognized inputs.
    /// Returns [`WindowParseError::BadTimestamp`] when a `since:<ts>`
    /// argument carries an unparseable RFC3339 timestamp.
    pub fn parse(s: &str) -> Result<Self, WindowParseError> {
        match s {
            "7d" => Ok(ReportWindow::Days7),
            "30d" => Ok(ReportWindow::Days30),
            "90d" => Ok(ReportWindow::Days90),
            "weekly" => Ok(ReportWindow::Weekly),
            "monthly" => Ok(ReportWindow::Monthly),
            "inception" => Ok(ReportWindow::Inception),
            other if other.starts_with("since:") => {
                let ts_str = &other["since:".len()..];
                let dt = OffsetDateTime::parse(ts_str, &Rfc3339)
                    .map_err(|e| WindowParseError::BadTimestamp(e.to_string()))?;
                Ok(ReportWindow::Since(Timestamp::new(dt)))
            }
            other => Err(WindowParseError::Malformed(other.to_string())),
        }
    }

    /// Resolve `(since, until)` for this window given the current wall-clock
    /// time `now` and the ledger inception timestamp.
    ///
    /// `inception` is unused for every variant except [`ReportWindow::Inception`].
    #[must_use]
    pub fn resolve(&self, now: Timestamp, inception: Timestamp) -> (Timestamp, Timestamp) {
        let days = |d: i64| -> Timestamp { Timestamp::new(now.inner() - time::Duration::days(d)) };
        match self {
            ReportWindow::Days7 | ReportWindow::Weekly => (days(7), now),
            ReportWindow::Days30 | ReportWindow::Monthly => (days(30), now),
            ReportWindow::Days90 => (days(90), now),
            ReportWindow::Since(ts) => (*ts, now),
            ReportWindow::Inception => (inception, now),
        }
    }

    /// Period slug for the front-matter `period:` field.
    #[must_use]
    pub fn slug(&self) -> String {
        match self {
            ReportWindow::Days7 => "7d".to_string(),
            ReportWindow::Days30 => "30d".to_string(),
            ReportWindow::Days90 => "90d".to_string(),
            ReportWindow::Weekly => "weekly".to_string(),
            ReportWindow::Monthly => "monthly".to_string(),
            ReportWindow::Inception => "inception".to_string(),
            ReportWindow::Since(ts) => {
                let s = ts
                    .inner()
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| "invalid".to_string());
                format!("since:{s}")
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn t807_parses_7d() {
        assert_eq!(ReportWindow::parse("7d").unwrap(), ReportWindow::Days7);
    }

    #[test]
    fn t807_parses_30d() {
        assert_eq!(ReportWindow::parse("30d").unwrap(), ReportWindow::Days30);
    }

    #[test]
    fn t807_parses_90d() {
        assert_eq!(ReportWindow::parse("90d").unwrap(), ReportWindow::Days90);
    }

    #[test]
    fn t807_parses_weekly() {
        assert_eq!(ReportWindow::parse("weekly").unwrap(), ReportWindow::Weekly);
    }

    #[test]
    fn t807_parses_monthly() {
        assert_eq!(
            ReportWindow::parse("monthly").unwrap(),
            ReportWindow::Monthly
        );
    }

    #[test]
    fn t807_parses_inception() {
        assert_eq!(
            ReportWindow::parse("inception").unwrap(),
            ReportWindow::Inception
        );
    }

    #[test]
    fn t807_parses_since_rfc3339() {
        let parsed = ReportWindow::parse("since:2026-01-01T00:00:00Z").unwrap();
        match parsed {
            ReportWindow::Since(_) => {}
            _ => panic!("expected ReportWindow::Since"),
        }
    }

    #[test]
    fn t807_rejects_bogus() {
        assert!(matches!(
            ReportWindow::parse("bogus"),
            Err(WindowParseError::Malformed(_))
        ));
    }

    #[test]
    fn t807_rejects_empty() {
        assert!(matches!(
            ReportWindow::parse(""),
            Err(WindowParseError::Malformed(_))
        ));
    }

    #[test]
    fn t807_rejects_1d() {
        assert!(matches!(
            ReportWindow::parse("1d"),
            Err(WindowParseError::Malformed(_))
        ));
    }

    #[test]
    fn t807_rejects_since_bad_ts() {
        assert!(matches!(
            ReportWindow::parse("since:bad"),
            Err(WindowParseError::BadTimestamp(_))
        ));
    }

    #[test]
    fn t807_slug_round_trip_days() {
        assert_eq!(ReportWindow::Days7.slug(), "7d");
        assert_eq!(ReportWindow::Days30.slug(), "30d");
        assert_eq!(ReportWindow::Days90.slug(), "90d");
        assert_eq!(ReportWindow::Weekly.slug(), "weekly");
        assert_eq!(ReportWindow::Monthly.slug(), "monthly");
        assert_eq!(ReportWindow::Inception.slug(), "inception");
    }

    #[test]
    fn t807_resolve_days7() {
        let now = Timestamp::new(OffsetDateTime::parse("2026-05-01T12:00:00Z", &Rfc3339).unwrap());
        let inception =
            Timestamp::new(OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339).unwrap());
        let (since, until) = ReportWindow::Days7.resolve(now, inception);
        assert_eq!(until, now);
        assert_eq!(
            since.inner(),
            OffsetDateTime::parse("2026-04-24T12:00:00Z", &Rfc3339).unwrap()
        );
    }

    #[test]
    fn t807_resolve_inception() {
        let now = Timestamp::new(OffsetDateTime::parse("2026-05-01T00:00:00Z", &Rfc3339).unwrap());
        let inception =
            Timestamp::new(OffsetDateTime::parse("2025-01-01T00:00:00Z", &Rfc3339).unwrap());
        let (since, until) = ReportWindow::Inception.resolve(now, inception);
        assert_eq!(since, inception);
        assert_eq!(until, now);
    }
}
