//! cockpit-activity-audit-ledger-producer v0.1.0 — Wave D invariant test (T-D-N9).
//!
//! D4 invariant gate: the aggregator's main long-lived handle NEVER emits
//! `End{Failed(...)}` on the happy path (when only successful `AuditTick`
//! events flow). Sibling Failed handles are the caller's responsibility
//! (wired in v0.1.1 per ADR-0044 § D4).
//!
//! This test runs a synthetic "backtest" for 1 second emitting audit ticks
//! at a moderate rate, then asserts zero Failed events on the activity channel.
//!
//! Run: `cargo test -p agent --test activity_audit_no_failed_events`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::sleep;

use agent::activity::{ActivityOutcome, ActivityPhase};
use agent::activity_audit_aggregator::spawn_aggregator;
use agent::bus::EventBus;
use agent::config::BusConfig;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

// ── Helper ─────────────────────────────────────────────────────────────────────

fn make_fill_tick() -> AuditTick<AuditEvent> {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use uuid::Uuid;
    use trading_core::{
        FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };

    AuditTick {
        event: AuditEvent::Fill {
            fill: Box::new(Fill {
                id: FillId::new(),
                order_id: OrderId::new(),
                symbol: Symbol::new("BTCUSDT"),
                side: Side::Buy,
                qty: Quantity::new(dec!(0.1)).unwrap(),
                price: Price::new(dec!(40_000)).unwrap(),
                fee: Money::from_decimal(dec!(1.6)),
                fee_tier: FeeTier::Taker,
                venue_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                local_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                liquidity: Liquidity::Taker,
                transaction_id: None,
            }),
            fees: dec!(1.6),
        },
        context: AuditContext {
            run_id: Uuid::nil(),
            posted_at: OffsetDateTime::UNIX_EPOCH,
            agent_pid: 0,
        },
    }
}

// ── Test ───────────────────────────────────────────────────────────────────────

/// T-D-N9 — D4 invariant: zero Failed events on the happy path.
///
/// Runs a synthetic 500 ms "backtest" emitting audit ticks at ~100 Hz.
/// Subscribes a sibling activity-channel receiver and asserts:
/// - The aggregator emits zero `End{Failed(...)}` events.
/// - The aggregator emits at least one `Tick` event (proving it was active).
/// - All emitted events are `AuditLedgerWrite` kind.
#[test]
fn no_failed_events_on_happy_path_500ms_synthetic_backtest() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async {
        // ── 1. Build tick bus and EventBus ────────────────────────────────────
        let (tick_tx, _) = broadcast::channel::<AuditTick<AuditEvent>>(1024);
        let bus = EventBus::new(&BusConfig::default());
        let mut activity_rx = bus.activity().subscribe();

        // ── 2. Spawn the aggregator ───────────────────────────────────────────
        let agg_handle = spawn_aggregator(Some(&tick_tx), &bus);

        // ── 3. Emit audit ticks at ~100 Hz for 500 ms ─────────────────────────
        // 500 ms / 10 ms = 50 batches × 1 tick = ~50 ticks total.
        // This models a moderate-rate backtest (not a 4000 Hz storm).
        for _batch in 0..50 {
            let _ = tick_tx.send(make_fill_tick());
            sleep(Duration::from_millis(10)).await;
        }

        // ── 4. Wait for idle-end (one empty window after the ticks stop) ──────
        sleep(Duration::from_millis(250)).await;

        // ── 5. Close the tick bus → aggregator exits ──────────────────────────
        drop(tick_tx);
        let _ = tokio::time::timeout(Duration::from_millis(500), agg_handle).await;

        // ── 6. Drain activity events ──────────────────────────────────────────
        let mut tick_count = 0usize;
        let mut failed_count = 0usize;
        let mut event_count = 0usize;

        while let Ok(evt) = activity_rx.try_recv() {
            event_count += 1;
            match &evt.phase {
                ActivityPhase::Tick { .. } => {
                    tick_count += 1;
                }
                ActivityPhase::End(ActivityOutcome::Failed(_)) => {
                    failed_count += 1;
                }
                _ => {}
            }
            assert_eq!(
                evt.kind,
                agent::ActivityKind::AuditLedgerWrite,
                "all events must be AuditLedgerWrite"
            );
        }

        println!(
            "T-D-N9: {event_count} total events (tick={tick_count}, failed={failed_count})"
        );

        // ── Assertion: zero Failed events ─────────────────────────────────────
        assert_eq!(
            failed_count, 0,
            "D4 invariant violated: {failed_count} Failed events on happy path \
             (expected 0 — main handle must never emit Failed on success-only ticks)"
        );

        // ── Assertion: at least one Tick event (aggregator was active) ─────────
        assert!(
            tick_count >= 1,
            "aggregator must emit at least 1 Tick event during 500 ms synthetic run \
             (got {tick_count})"
        );
    });
}
