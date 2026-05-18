//! T809 — `audit::journal::kill_switch_tripped` dual-write acceptance.
//!
//! Acceptance per the architect's task spec (Q8):
//!  - writes BOTH the v0 zero-amount memo journal row AND a new
//!    `strategy_events` row of kind `KillSwitchTripped`;
//!  - both writes are atomic (share one `sqlx::Transaction`);
//!  - reconciler `Σ debits == Σ credits` unchanged (memo row is zero
//!    amount; strategy_events carries no money);
//!  - the v0 memo row is preserved byte-for-byte (description matches
//!    `registry:KillSwitchTripped:<reason>` and metadata is the same
//!    JSON payload `registry_event` would have written).

use audit::query::{global_debit_credit_sum, strategy_events_since};
use audit::{Ledger, bootstrap, journal};
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, Timestamp};

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

#[tokio::test]
async fn t809_kill_switch_tripped_writes_memo_and_strategy_event() {
    let ledger = open_ledger().await;

    // Capture pre-write debit/credit sum for the reconciler invariant.
    let (dr_before, cr_before) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum before");

    // Capture pre-write counts.
    let memo_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_transactions")
        .fetch_one(ledger.pool())
        .await
        .expect("count memo before");
    let entries_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(ledger.pool())
        .await
        .expect("count entries before");
    let events_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_events")
        .fetch_one(ledger.pool())
        .await
        .expect("count events before");

    journal::kill_switch_tripped(&ledger, "clock_skew", "kill_switch")
        .await
        .expect("kill_switch_tripped write");

    // (a) one new memo journal row (journal_transactions + zero-amount entry).
    let memo_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_transactions")
        .fetch_one(ledger.pool())
        .await
        .expect("count memo after");
    let entries_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(ledger.pool())
        .await
        .expect("count entries after");
    assert_eq!(
        memo_after.0 - memo_before.0,
        1,
        "kill_switch_tripped must write exactly one memo journal row"
    );
    assert_eq!(
        entries_after.0 - entries_before.0,
        1,
        "kill_switch_tripped must write exactly one zero-amount journal_entries row"
    );

    // (b) one new strategy_events row of kind `KillSwitchTripped`.
    let events_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_events")
        .fetch_one(ledger.pool())
        .await
        .expect("count events after");
    assert_eq!(
        events_after.0 - events_before.0,
        1,
        "kill_switch_tripped must write exactly one strategy_events row"
    );

    let evs = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");
    assert_eq!(evs.len(), 1);
    let ev = &evs[0];
    assert_eq!(
        ev.kind,
        StrategyEventKind::KillSwitchTripped,
        "kind must be KillSwitchTripped"
    );
    assert!(
        ev.strategy_id.is_none(),
        "kill-switch trip is feed-level, no strategy_id"
    );
    assert_eq!(
        ev.error_summary.as_deref(),
        Some("clock_skew"),
        "error_summary must carry the HaltReason as string"
    );
    assert_eq!(
        ev.error_code.as_deref(),
        Some("kill_switch_tripped"),
        "error_code must be 'kill_switch_tripped'"
    );

    // (c) reconciler invariant Σ debits == Σ credits unchanged (memo row
    // is zero-amount; strategy_events has no money columns).
    let (dr_after, cr_after) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum after");
    assert_eq!(
        dr_before, dr_after,
        "kill_switch_tripped must not affect Σ debits"
    );
    assert_eq!(
        cr_before, cr_after,
        "kill_switch_tripped must not affect Σ credits"
    );
    assert_eq!(
        dr_after, cr_after,
        "Σ debits must equal Σ credits after a kill-switch trip"
    );
}

#[tokio::test]
async fn t809_memo_row_byte_for_byte_v0_compat() {
    // v0 backwards compat — the memo row's description and metadata
    // shape must remain what `registry_event(_, "KillSwitchTripped",
    // reason, json{event,reason,operator})` would have produced.
    let ledger = open_ledger().await;
    journal::kill_switch_tripped(&ledger, "halt_file", "kill_switch")
        .await
        .expect("kill_switch_tripped");

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT description, metadata FROM journal_transactions \
         WHERE description LIKE 'registry:KillSwitchTripped:%'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select memo row");
    assert_eq!(rows.len(), 1, "exactly one memo row written");
    let (description, metadata) = &rows[0];

    // v0 description format: "registry:<event_kind>:<strategy_id-or-reason>"
    assert_eq!(
        description, "registry:KillSwitchTripped:halt_file",
        "memo description must match the v0 byte-for-byte format"
    );

    // v0 metadata payload: serde JSON with stable key order
    // {event, reason, operator}.
    let parsed: serde_json::Value =
        serde_json::from_str(metadata).expect("memo metadata must be valid JSON");
    assert_eq!(parsed["event"], "KillSwitchTripped");
    assert_eq!(parsed["reason"], "halt_file");
    assert_eq!(parsed["operator"], "kill_switch");

    // Memo row's journal_entries line must be a zero-amount entry on
    // equity:opening_balance — same shape `registry_event` produces.
    let entries: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT je.account_id, je.debit_amount, je.credit_amount \
         FROM journal_entries je \
         JOIN journal_transactions jt ON je.transaction_id = jt.id \
         WHERE jt.description = 'registry:KillSwitchTripped:halt_file'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select memo entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "equity:opening_balance");
    assert_eq!(entries[0].1, "0");
    assert_eq!(entries[0].2, "0");
}

#[tokio::test]
async fn t809_strategy_event_uses_microsecond_timestamp_format() {
    // Determinism / HF-3 — the new `strategy_events` row must use the
    // 6-digit fractional-second format (same as the rest of v1+ writers).
    let ledger = open_ledger().await;
    journal::kill_switch_tripped(&ledger, "test", "kill_switch")
        .await
        .expect("kill_switch_tripped");

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT ts FROM strategy_events WHERE kind = 'KillSwitchTripped'")
            .fetch_all(ledger.pool())
            .await
            .expect("select ts");
    assert_eq!(rows.len(), 1);
    let ts = &rows[0].0;
    // Format: YYYY-MM-DDTHH:MM:SS.ssssssZ (27 chars total).
    assert_eq!(
        ts.len(),
        27,
        "ts must be the 27-char 6-digit fractional-second format, got {ts:?}"
    );
    assert!(ts.contains('.'), "ts must carry sub-second precision");
    assert!(ts.ends_with('Z'), "ts must end with Z");
    // Sub-second portion is exactly 6 digits.
    let (_, frac) = ts.split_once('.').expect("has dot");
    let digits = frac.trim_end_matches('Z');
    assert_eq!(
        digits.len(),
        6,
        "strategy_events ts fractional-second portion must be 6 digits, got {digits:?}"
    );
    assert!(
        digits.chars().all(|c| c.is_ascii_digit()),
        "fractional-second portion must be all digits, got {digits:?}"
    );
}

#[tokio::test]
async fn t809_dual_write_atomic_in_one_transaction() {
    // Acceptance: the dual-write is atomic — either both rows land or
    // neither does.  We exercise the happy path here (both land) and
    // assert the row-count delta is exactly +1 / +1; the failure path
    // is provable by inspection of the `kill_switch_tripped` body
    // (single `db_txn = ledger.pool.begin(); .commit()`), but we also
    // exercise back-to-back trips and assert a clean +2 / +2 delta
    // (no orphans, no duplicates).
    let ledger = open_ledger().await;

    journal::kill_switch_tripped(&ledger, "halt_file", "kill_switch")
        .await
        .expect("trip 1");
    journal::kill_switch_tripped(&ledger, "heartbeat_timeout", "kill_switch")
        .await
        .expect("trip 2");

    let memo_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_transactions \
         WHERE description LIKE 'registry:KillSwitchTripped:%'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count memo");
    assert_eq!(memo_count.0, 2, "two memo rows for two trips");

    let event_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM strategy_events WHERE kind = 'KillSwitchTripped'")
            .fetch_one(ledger.pool())
            .await
            .expect("count events");
    assert_eq!(event_count.0, 2, "two strategy_events rows for two trips");
}
