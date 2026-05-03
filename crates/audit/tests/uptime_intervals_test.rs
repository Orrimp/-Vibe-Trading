//! T806 — `agent_uptime` table + writers + reader integration tests.
//!
//! Acceptance: full open / heartbeat / close cycle round-trips through
//! `uptime_intervals_since` with the correct row shape.

use audit::query::{uptime_intervals_since, UptimeInterval};
use audit::{bootstrap, journal, Ledger};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::Timestamp;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

fn parse_ts(s: &str) -> Timestamp {
    Timestamp::new(OffsetDateTime::parse(s, &Rfc3339).expect("parse"))
}

#[tokio::test]
async fn t806_full_open_heartbeat_close_cycle() {
    let ledger = open_ledger().await;

    let boot_id = "11111111-1111-4111-8111-111111111111";
    let opened_at = "2030-06-01T00:00:00.000000Z";
    let heartbeat_1 = "2030-06-01T00:00:30.000000Z";
    let heartbeat_2 = "2030-06-01T00:01:00.000000Z";
    let closed_at = "2030-06-01T00:01:30.000000Z";

    journal::open_uptime_interval(&ledger, boot_id, Some(opened_at))
        .await
        .expect("open");

    journal::heartbeat_uptime(&ledger, boot_id, Some(heartbeat_1))
        .await
        .expect("heartbeat 1");
    journal::heartbeat_uptime(&ledger, boot_id, Some(heartbeat_2))
        .await
        .expect("heartbeat 2");

    journal::close_uptime_interval(&ledger, boot_id, Some(closed_at))
        .await
        .expect("close");

    let rows = uptime_intervals_since(&ledger, parse_ts("2030-01-01T00:00:00Z"))
        .await
        .expect("uptime_intervals_since");

    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.boot_id.as_str(), boot_id);
    assert_eq!(row.started_at, parse_ts(opened_at));
    assert_eq!(
        row.last_heartbeat_at,
        parse_ts(heartbeat_2),
        "last_heartbeat_at must reflect the most recent heartbeat"
    );
    assert_eq!(row.stopped_at, Some(parse_ts(closed_at)));
}

#[tokio::test]
async fn t806_running_agent_has_stopped_at_none() {
    let ledger = open_ledger().await;

    let boot_id = "22222222-2222-4222-8222-222222222222";
    journal::open_uptime_interval(&ledger, boot_id, Some("2030-06-02T00:00:00.000000Z"))
        .await
        .expect("open");

    let rows = uptime_intervals_since(&ledger, parse_ts("2030-01-01T00:00:00Z"))
        .await
        .expect("uptime_intervals_since");
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].stopped_at.is_none(),
        "agent that hasn't shut down has stopped_at = NULL"
    );
}

#[tokio::test]
async fn t806_two_intervals_returned_in_chronological_order() {
    let ledger = open_ledger().await;

    // Insert second boot first to verify the sort.
    journal::open_uptime_interval(
        &ledger,
        "33333333-3333-4333-8333-333333333333",
        Some("2030-06-04T00:00:00.000000Z"),
    )
    .await
    .expect("open second boot");
    journal::open_uptime_interval(
        &ledger,
        "44444444-4444-4444-8444-444444444444",
        Some("2030-06-03T00:00:00.000000Z"),
    )
    .await
    .expect("open first boot");

    let rows = uptime_intervals_since(&ledger, parse_ts("2030-01-01T00:00:00Z"))
        .await
        .expect("uptime_intervals_since");

    assert_eq!(rows.len(), 2);
    // chronological: 6-03 before 6-04
    assert_eq!(rows[0].started_at, parse_ts("2030-06-03T00:00:00.000000Z"));
    assert_eq!(rows[1].started_at, parse_ts("2030-06-04T00:00:00.000000Z"));
}

#[tokio::test]
async fn t806_filter_by_since_excludes_earlier_rows() {
    let ledger = open_ledger().await;

    journal::open_uptime_interval(
        &ledger,
        "55555555-5555-4555-8555-555555555555",
        Some("2025-01-01T00:00:00.000000Z"),
    )
    .await
    .expect("old boot");
    journal::open_uptime_interval(
        &ledger,
        "66666666-6666-4666-8666-666666666666",
        Some("2030-01-01T00:00:00.000000Z"),
    )
    .await
    .expect("recent boot");

    let rows = uptime_intervals_since(&ledger, parse_ts("2028-01-01T00:00:00Z"))
        .await
        .expect("uptime_intervals_since");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].boot_id.as_str(),
        "66666666-6666-4666-8666-666666666666"
    );
}

// ── ts uses microsecond fractional-second format (HF-3 / determinism) ───────

#[tokio::test]
async fn t806_default_ts_uses_microsecond_format() {
    // When `ts = None`, the writer uses the same 6-digit fractional-second
    // format the `strategy_events` writer uses.  This guarantees stable
    // ORDER BY ts in the reports query and avoids the v1.5a `Rfc3339`
    // second-precision regression.
    let ledger = open_ledger().await;

    let boot_id = "77777777-7777-4777-8777-777777777777";
    journal::open_uptime_interval(&ledger, boot_id, None)
        .await
        .expect("open");

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT started_at FROM agent_uptime WHERE boot_id = ?")
            .bind(boot_id)
            .fetch_all(ledger.pool())
            .await
            .expect("select");

    assert_eq!(rows.len(), 1);
    let ts = &rows[0].0;
    // Format: yyyy-mm-ddTHH:MM:SS.uuuuuuZ (length 27).
    assert_eq!(ts.len(), 27, "expected 27-char microsecond ts, got {ts}");
    assert!(ts.ends_with('Z'), "must end with Z: {ts}");
    assert!(
        ts.chars().nth(19) == Some('.'),
        "must have '.' at offset 19: {ts}"
    );
    let fractional: &str = &ts[20..26];
    assert_eq!(fractional.len(), 6, "expected 6 fractional digits");
    assert!(
        fractional.chars().all(|c| c.is_ascii_digit()),
        "fractional must be digits: {fractional}"
    );
}

// ── helpers test that StopOrLastHb computation is sound ─────────────────────

#[tokio::test]
async fn t806_uptime_interval_carries_no_money() {
    // The uptime table must NOT affect the reconciler — Σ debits == Σ credits
    // is unchanged regardless of how many uptime rows are written.
    let ledger = open_ledger().await;
    let (dr_before, cr_before) = audit::query::global_debit_credit_sum(&ledger)
        .await
        .expect("sum before");

    for i in 0..5 {
        let boot = format!("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeee{i:02}");
        journal::open_uptime_interval(
            &ledger,
            &boot,
            Some(&format!("2030-06-0{}T00:00:00.000000Z", i + 1)),
        )
        .await
        .expect("open");
        journal::heartbeat_uptime(
            &ledger,
            &boot,
            Some(&format!("2030-06-0{}T00:00:30.000000Z", i + 1)),
        )
        .await
        .expect("heartbeat");
        journal::close_uptime_interval(
            &ledger,
            &boot,
            Some(&format!("2030-06-0{}T00:01:00.000000Z", i + 1)),
        )
        .await
        .expect("close");
    }

    let (dr_after, cr_after) = audit::query::global_debit_credit_sum(&ledger)
        .await
        .expect("sum after");
    assert_eq!(dr_before, dr_after);
    assert_eq!(cr_before, cr_after);
    let rows = uptime_intervals_since(&ledger, parse_ts("2030-01-01T00:00:00Z"))
        .await
        .expect("read");
    assert_eq!(rows.len(), 5);
    let _ = std::mem::size_of::<UptimeInterval>();
}
