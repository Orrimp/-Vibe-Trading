//! R7 — System health (uptime, kill-switch, clock-skew, feed
//! reconnects, funding poll, LLM spend).
//!
//! The renderer is pure: the orchestrator computes the six metrics
//! once (each wrapped in a `Result` so a missing source surfaces as
//! `unknown — see logs` per R9.3 / R7.3) and hands them to [`render`].

use std::fmt::Write;

use rust_decimal::Decimal;

/// One R7 row.  The orchestrator carries each metric in a `Result` so
/// per-row degradation is explicit (R7.3 / R9.3).  The `Err` arm is
/// rendered as `unknown — see logs` rather than aborting the binary.
pub type Cell = Result<String, String>;

/// Inputs for the R7 system-health section.
#[derive(Debug, Clone)]
pub struct SystemHealthInputs {
    /// Uptime as a percentage of the period (e.g. `dec!(99.5)`).
    pub uptime_pct: Cell,
    /// Kill-switch trip count.
    pub kill_switch_trips: Cell,
    /// Clock-skew event count.
    pub clock_skew_events: Cell,
    /// Feed-reconnect count.
    pub feed_reconnects: Cell,
    /// Funding-rate poll success rate (e.g. `42 / 56` or a percent).
    pub funding_poll_rate: Cell,
    /// LLM spend vs. budget (e.g. `$0.00 / $135`).
    pub llm_spend: Cell,
}

/// Render the R7 system-health table.
///
/// Six rows, in the locked order documented in R7.1.  An inner
/// `Result::Err` for any cell renders as `unknown — see logs`.
#[must_use]
pub fn render(inputs: &SystemHealthInputs) -> String {
    let mut out = String::with_capacity(384);
    out.push_str("## System health\n\n");
    out.push_str("| Metric | Value |\n");
    out.push_str("|--------|-------|\n");

    let _ = writeln!(out, "| Uptime | {} |", cell(&inputs.uptime_pct));
    let _ = writeln!(
        out,
        "| Kill-switch trips | {} |",
        cell(&inputs.kill_switch_trips)
    );
    let _ = writeln!(
        out,
        "| Clock-skew events | {} |",
        cell(&inputs.clock_skew_events)
    );
    let _ = writeln!(
        out,
        "| Feed reconnects | {} |",
        cell(&inputs.feed_reconnects)
    );
    let _ = writeln!(
        out,
        "| Funding poll success | {} |",
        cell(&inputs.funding_poll_rate)
    );
    let _ = writeln!(out, "| LLM spend | {} |", cell(&inputs.llm_spend));

    out
}

fn cell(c: &Cell) -> String {
    match c {
        Ok(s) => s.clone(),
        Err(_) => "unknown — see logs".to_string(),
    }
}

/// Compute uptime percentage given the uptime intervals returned by
/// `audit::query::uptime_intervals_since` and the period boundaries.
///
/// Per the R7.1 design:
/// `Σ (min(stopped_at_or_last_hb, period_end) - max(started_at, period_start))`
/// clamped to `[0, period_length]`, then divided by `period_length`.
///
/// Returns the percentage as a `Decimal` (e.g. `dec!(99.5)`).  Returns
/// `0` when `period_length <= 0`.
#[must_use]
pub fn compute_uptime_pct(
    intervals: &[audit::query::UptimeInterval],
    period_start_ms: i64,
    period_end_ms: i64,
) -> Decimal {
    if period_end_ms <= period_start_ms {
        return Decimal::ZERO;
    }
    let period_len = period_end_ms - period_start_ms;
    let mut covered: i64 = 0;
    for iv in intervals {
        let started = iv.started_at.unix_millis();
        let last = iv
            .stopped_at
            .map_or(iv.last_heartbeat_at.unix_millis(), |t| t.unix_millis());
        let s = started.max(period_start_ms);
        let e = last.min(period_end_ms);
        if e > s {
            covered += e - s;
        }
    }
    let pct = (Decimal::from(covered) * Decimal::from(100u32)) / Decimal::from(period_len);
    pct.round_dp(2)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ok_inputs() -> SystemHealthInputs {
        SystemHealthInputs {
            uptime_pct: Ok("99.50%".into()),
            kill_switch_trips: Ok("0".into()),
            clock_skew_events: Ok("0".into()),
            feed_reconnects: Ok("3".into()),
            funding_poll_rate: Ok("56 / 56".into()),
            llm_spend: Ok("$0.00 / $135".into()),
        }
    }

    #[test]
    fn t813_system_health_renders_six_rows() {
        let body = render(&ok_inputs());
        assert!(body.contains("## System health"));
        assert!(body.contains("| Uptime | 99.50% |"));
        assert!(body.contains("| Kill-switch trips | 0 |"));
        assert!(body.contains("| Clock-skew events | 0 |"));
        assert!(body.contains("| Feed reconnects | 3 |"));
        assert!(body.contains("| Funding poll success | 56 / 56 |"));
        assert!(body.contains("| LLM spend | $0.00 / $135 |"));
    }

    #[test]
    fn t813_system_health_err_cell_renders_unknown() {
        let mut inp = ok_inputs();
        inp.kill_switch_trips = Err("query failed".into());
        let body = render(&inp);
        assert!(body.contains("| Kill-switch trips | unknown — see logs |"));
        // Other rows still ok.
        assert!(body.contains("| Uptime | 99.50% |"));
    }

    #[test]
    fn t813_system_health_byte_stable_across_runs() {
        let inp = ok_inputs();
        let a = render(&inp);
        let b = render(&inp);
        assert_eq!(a, b);
    }

    #[test]
    fn t813_compute_uptime_pct_full_coverage() {
        use audit::query::UptimeInterval;
        use smol_str::SmolStr;
        use time::OffsetDateTime;
        use trading_core::Timestamp;

        let start = OffsetDateTime::UNIX_EPOCH + time::Duration::days(1);
        let end = start + time::Duration::days(7);
        let iv = UptimeInterval {
            boot_id: SmolStr::new("boot1"),
            started_at: Timestamp::new(start),
            last_heartbeat_at: Timestamp::new(end),
            stopped_at: None,
        };
        let pct = compute_uptime_pct(
            &[iv],
            Timestamp::new(start).unix_millis(),
            Timestamp::new(end).unix_millis(),
        );
        assert_eq!(pct, dec!(100));
    }

    #[test]
    fn t813_compute_uptime_pct_partial_coverage() {
        use audit::query::UptimeInterval;
        use smol_str::SmolStr;
        use time::OffsetDateTime;
        use trading_core::Timestamp;

        let start = OffsetDateTime::UNIX_EPOCH + time::Duration::days(1);
        let mid = start + time::Duration::days(3); // halfway through 6-day window
        let end = start + time::Duration::days(6);
        let iv = UptimeInterval {
            boot_id: SmolStr::new("boot1"),
            started_at: Timestamp::new(start),
            last_heartbeat_at: Timestamp::new(mid),
            stopped_at: Some(Timestamp::new(mid)),
        };
        let pct = compute_uptime_pct(
            &[iv],
            Timestamp::new(start).unix_millis(),
            Timestamp::new(end).unix_millis(),
        );
        // 3 of 6 days = 50 %.
        assert_eq!(pct, dec!(50));
    }
}
