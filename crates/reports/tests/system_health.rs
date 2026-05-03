#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R7 system-health integration test.
//!
//! Asserts the 6-row table renders the correct values and gracefully
//! degrades to `unknown — see logs` on per-cell `Result::Err`.

use reports::render::system_health::{compute_uptime_pct, render, SystemHealthInputs};
use rust_decimal_macros::dec;

#[test]
fn t813_r7_renders_six_rows_with_known_values() {
    let body = render(&SystemHealthInputs {
        uptime_pct: Ok("99.50%".into()),
        kill_switch_trips: Ok("0".into()),
        clock_skew_events: Ok("0".into()),
        feed_reconnects: Ok("3".into()),
        funding_poll_rate: Ok("56 / 56".into()),
        llm_spend: Ok("$0.00 / $135".into()),
    });
    assert!(body.contains("| Uptime | 99.50% |"));
    assert!(body.contains("| Kill-switch trips | 0 |"));
    assert!(body.contains("| Clock-skew events | 0 |"));
    assert!(body.contains("| Feed reconnects | 3 |"));
    assert!(body.contains("| Funding poll success | 56 / 56 |"));
    assert!(body.contains("| LLM spend | $0.00 / $135 |"));
}

#[test]
fn t813_r7_err_cell_renders_unknown_see_logs() {
    let body = render(&SystemHealthInputs {
        uptime_pct: Ok("99.50%".into()),
        kill_switch_trips: Err("query failed".into()),
        clock_skew_events: Ok("0".into()),
        feed_reconnects: Ok("0".into()),
        funding_poll_rate: Ok("n/a".into()),
        llm_spend: Ok("$0.00 / $135".into()),
    });
    assert!(body.contains("| Kill-switch trips | unknown — see logs |"));
}

#[test]
fn t813_r7_compute_uptime_pct_full_period() {
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
