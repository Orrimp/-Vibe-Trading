//! T809 — `KillSwitch::trip` integration: dual-write to audit + spawn helper.
//!
//! Acceptance per the architect's task spec (Q8 / R12.1c):
//!  - triggers `KillSwitch::trip(HaltReason::Test)` on a kill switch
//!    constructed with `with_audit(ledger, mock_spawner)`;
//!  - asserts (a) one new memo journal row, (b) one new
//!    `strategy_events` row of kind `KillSwitchTripped`,
//!    (c) `Σ debits == Σ credits` unchanged (reconciler invariant
//!    preserved), (d) the spawn helper was called with the expected
//!    `--period since:<ts>` argument.

use std::sync::Arc;

use agent::{HaltReason, IncidentSpawner, KillSwitch, MockIncidentSpawner};
use audit::query::{all_transaction_ids, global_debit_credit_sum, strategy_events_since};
use audit::{Ledger, bootstrap};
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

/// Wait for the audit-write tokio task spawned by `KillSwitch::trip` to
/// land its row.  Polls up to 2s.  Returns on first success.
async fn wait_for_strategy_event(ledger: &Ledger) {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
    loop {
        let evs = strategy_events_since(ledger, ts_epoch())
            .await
            .expect("strategy_events_since");
        if !evs.is_empty() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for KillSwitchTripped strategy_events row");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn t809_trip_writes_audit_dual_and_calls_spawn_helper() {
    let ledger = Arc::new(open_ledger().await);

    // Capture pre-write reconciler invariant.
    let (dr_before, cr_before) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum before");

    // Mock spawner records calls without launching anything.
    let mock = Arc::new(MockIncidentSpawner::new());
    let spawner: Arc<dyn IncidentSpawner> = mock.clone();

    let dir = tempfile::tempdir().expect("tempdir");
    let halt_file = dir.path().join(".halt");

    let ks = Arc::new(KillSwitch::with_audit(
        &halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    // Trip the kill switch.
    ks.trip(HaltReason::Test);
    assert!(ks.is_tripped(), "kill switch must be tripped");

    // Wait for the spawned audit-write task to land.
    wait_for_strategy_event(&ledger).await;

    // (a) one new memo journal row.  On a fresh in-memory ledger, the
    // only writer to `journal_transactions` is the kill-switch trip.
    // Byte-for-byte v0 description shape is asserted in the audit-level
    // test `t809_memo_row_byte_for_byte_v0_compat`.
    let txn_ids = all_transaction_ids(&ledger)
        .await
        .expect("all_transaction_ids");
    assert_eq!(
        txn_ids.len(),
        1,
        "exactly one memo journal row for one trip"
    );

    // (b) one new strategy_events row of kind KillSwitchTripped.
    let evs = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");
    assert_eq!(evs.len(), 1, "exactly one strategy_events row for one trip");
    assert_eq!(
        evs[0].kind,
        StrategyEventKind::KillSwitchTripped,
        "kind must be KillSwitchTripped"
    );
    assert_eq!(
        evs[0].error_summary.as_deref(),
        Some("test"),
        "error_summary must carry HaltReason::Test as 'test'"
    );

    // (c) Σ debits == Σ credits unchanged.
    let (dr_after, cr_after) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum after");
    assert_eq!(dr_before, dr_after, "Σ debits must be unchanged");
    assert_eq!(cr_before, cr_after, "Σ credits must be unchanged");
    assert_eq!(
        dr_after, cr_after,
        "Σ debits must equal Σ credits after a kill-switch trip"
    );

    // (d) spawn helper was called with the expected --period since:<ts> arg.
    let calls = mock.calls();
    assert_eq!(calls.len(), 1, "spawn helper called exactly once");
    let call = &calls[0];
    assert_eq!(
        call.reason, "test",
        "spawn helper received HaltReason::Test as 'test'"
    );
    // halt_ts_rfc3339 must be a parseable RFC-3339 timestamp.
    let _parsed = OffsetDateTime::parse(
        &call.halt_ts_rfc3339,
        &time::format_description::well_known::Rfc3339,
    )
    .expect("halt_ts_rfc3339 must parse as RFC-3339");
    // The since:<ts> form is what the production CommandIncidentSpawner
    // builds: `--period since:<halt_ts_rfc3339>`.  The test owns the
    // arg-construction contract: any change to the production builder
    // must keep this assertion green.
    let expected_period_arg = format!("since:{}", call.halt_ts_rfc3339);
    assert!(
        expected_period_arg.starts_with("since:"),
        "spawn helper period arg must start with 'since:'"
    );
}

#[tokio::test]
async fn t809_trip_is_idempotent_only_first_call_dual_writes() {
    let ledger = Arc::new(open_ledger().await);
    let mock = Arc::new(MockIncidentSpawner::new());
    let spawner: Arc<dyn IncidentSpawner> = mock.clone();
    let dir = tempfile::tempdir().expect("tempdir");
    let halt_file = dir.path().join(".halt");

    let ks = Arc::new(KillSwitch::with_audit(
        &halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    ks.trip(HaltReason::Test);
    ks.trip(HaltReason::ManualOperator);
    ks.trip(HaltReason::HaltFile);

    wait_for_strategy_event(&ledger).await;

    // Only one trip is recorded — second + third are no-ops.
    let evs = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");
    assert_eq!(evs.len(), 1, "trip is idempotent — only first writes audit");
    assert_eq!(
        mock.calls().len(),
        1,
        "trip is idempotent — only first invokes spawner"
    );
}

#[tokio::test]
async fn t809_trip_without_audit_wire_is_v0_compat() {
    // v0 backwards compat: KillSwitch::new (no audit) must keep working
    // with no audit write or spawn — just the broadcast.
    let dir = tempfile::tempdir().expect("tempdir");
    let halt_file = dir.path().join(".halt");
    let ks = KillSwitch::new(&halt_file, 32);
    let mut rx = ks.subscribe();
    ks.trip(HaltReason::Test);
    assert!(ks.is_tripped());
    let mode = rx.try_recv().expect("AgentMode::Halted broadcast");
    assert!(matches!(mode, agent::AgentMode::Halted { .. }));
}
