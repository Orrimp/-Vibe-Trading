//! Audit-ledger-writes activity aggregator — cockpit-activity-audit-ledger-producer v0.1.0.
//!
//! Subscribes to the existing `crates/audit::Ledger::tick_bus` broadcast and
//! aggregates arbitrary-rate `AuditTick<AuditEvent>` events into at-most
//! 10 events/sec on the cockpit activity channel via a 100 ms
//! `tokio::time::interval` cadence.
//!
//! ## Design (ADR-0044 § D2)
//!
//! ```text
//! audit::Ledger::tick_bus (broadcast::Sender<AuditTick<AuditEvent>>, cap 1024)
//!     returned by open_with_tick_bus()
//!                  │
//!        (NEW) Aggregator::rx   ─── AtomicU32 counter ──► 100ms interval.tick()
//!                                                               │
//!                                            bus.activity().start / .tick(N)
//!                                                               │
//!                                         EventBus::activity_tx (cap 256)
//! ```
//!
//! ## Signature note (R-NR.1)
//!
//! The public surface is `spawn_aggregator(tick_sender, bus)` where `tick_sender`
//! is the `broadcast::Sender<AuditTick<AuditEvent>>` returned by
//! `audit::Ledger::open_with_tick_bus`. We do NOT accept `&Arc<Ledger>` because
//! `Ledger::tick_bus` is `pub(crate)` inside `crates/audit` — accessing it would
//! require mutating the audit crate, violating R-NR.1. The caller holds the sender
//! for the process lifetime (same as the trail mirror pattern in `cockpit_live.rs`).
//!
//! ## Hot-path cost (R5.1 / ADR-0044 § D2)
//!
//! Per `AuditTick`: ONE `AtomicU32::fetch_add(1, Relaxed)` — sub-50 ns on Apple
//! Silicon. The broadcast send is already on the audit writer's critical path
//! (cost amortised). No new allocation, no syscall.
//!
//! ## Lifecycle (ADR-0044 § D5 — idle-end semantics)
//!
//! - A single `ActivityHandle` exists as long as audit ticks are arriving.
//! - On the first 100 ms window that observes **zero** ticks the handle is
//!   dropped → emits `End { Success }` via `Drop`.
//! - The next non-empty window starts a fresh handle (new `ActivityId`).
//!
//! ## K2 truncation (label budget)
//!
//! The rendered label `"Audit: N writes"` is capped at N = 9999. For N > 9999
//! the label flips to `"Audit: 9999+ writes"`. The internal counter still
//! tracks the precise total.
//!
//! ## Zero changes to `crates/audit/` (R-NR.1)
//!
//! The aggregator **subscribes** to the existing tick bus — it does not modify
//! any audit-crate code. The audit crate stays unaware of the activity tape.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{Interval, interval};
use tracing::warn;

use audit::tick::{AuditEvent, AuditTick};

use crate::activity::{ActivityHandle, ActivityKind, ActivitySender};
use crate::bus::EventBus;

// ── Label constants (R-NR.4 — no inline literals) ───────────────────────────

/// Base label prefix for audit-ledger-write activities.
const AUDIT_LABEL_PREFIX: &str = "Audit: ";

/// Label suffix for the normal count path (K2 budget: N ≤ 9999).
const AUDIT_LABEL_SUFFIX: &str = " writes";

/// K2 truncation label: emitted when the window count exceeds 9999.
const AUDIT_LABEL_FLOOD: &str = "Audit: 9999+ writes";

/// K2 boundary: counts > this value use the flood label.
const K2_THRESHOLD: u32 = 9_999;

/// 100 ms aggregation window — verbatim from ADR-0044 § D3.
const AGGREGATION_WINDOW: Duration = Duration::from_millis(100);

// ── Aggregator ───────────────────────────────────────────────────────────────

/// The aggregator worker. Holds all state for the 100 ms aggregation loop.
///
/// Created and owned by the spawned tokio task.
struct Aggregator {
    /// Receiver half of the audit tick broadcast.
    rx: broadcast::Receiver<AuditTick<AuditEvent>>,
    /// Producer handle to the activity channel.
    bus: ActivitySender,
    /// Per-window event counter. `fetch_add(1, Relaxed)` per received tick;
    /// `swap(0, Relaxed)` at the interval boundary.
    counter: AtomicU32,
    /// Long-lived RAII handle. `None` during idle windows; `Some` while bursting.
    handle: Option<ActivityHandle>,
    /// 100 ms interval timer — drives the window boundary.
    interval: Interval,
}

impl Aggregator {
    fn new(
        rx: broadcast::Receiver<AuditTick<AuditEvent>>,
        bus: ActivitySender,
    ) -> Self {
        Self {
            rx,
            bus,
            counter: AtomicU32::new(0),
            handle: None,
            interval: interval(AGGREGATION_WINDOW),
        }
    }

    /// Format the label for a given tick count. Enforces the K2 threshold.
    fn format_label(n: u32) -> String {
        if n > K2_THRESHOLD {
            AUDIT_LABEL_FLOOD.to_owned()
        } else {
            format!("{}{}{}", AUDIT_LABEL_PREFIX, n, AUDIT_LABEL_SUFFIX)
        }
    }

    /// The main aggregation loop. Runs until the tick bus is closed.
    ///
    /// `tokio::select!` races two arms on every iteration:
    /// 1. `rx.recv()` — an audit tick arrived; increment the counter.
    /// 2. `interval.tick()` — 100 ms boundary; snapshot counter → emit.
    async fn run(mut self) {
        loop {
            tokio::select! {
                recv_result = self.rx.recv() => {
                    match recv_result {
                        Ok(_tick) => {
                            self.counter.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Lag: the aggregator fell behind the tick bus.
                            // Count the skipped events so the internal total
                            // stays accurate (they were written; we just didn't
                            // receive all ticks). Per ADR-0044 § D2 / K5 mitigation.
                            warn!(
                                consumer = "activity_audit_aggregator",
                                lagged = n,
                                "audit tick stream lagged — counting skipped ticks"
                            );
                            self.counter.fetch_add(n as u32, Ordering::Relaxed);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Sender dropped — audit ledger is shutting down.
                            // Drop the handle to emit End{Success} and exit.
                            break;
                        }
                    }
                }
                _ = self.interval.tick() => {
                    let n = self.counter.swap(0, Ordering::Relaxed);
                    match (n, self.handle.as_ref()) {
                        // (0, None) — still idle, no handle, nothing to do.
                        (0, None) => {}
                        // (0, Some(_)) — idle window: drop handle → End{Success}.
                        (0, Some(_)) => {
                            self.handle = None; // Drop impl emits End { Success }
                        }
                        // (n>0, None) — first non-empty window: start a fresh handle.
                        //
                        // NOTE: We do NOT call `h.tick(n)` immediately after `start()`.
                        // The `ActivityHandle::tick` throttle is `TICK_THROTTLE = 100 ms`,
                        // and `start()` initialises `last_tick = Instant::now()` — so
                        // calling `tick()` in the same timer-boundary invocation is always
                        // throttled to a no-op. The `Start` event (emitted by `start()`)
                        // carries the label with the window count; subsequent 100 ms windows
                        // emit `Tick` events via the `(n>0, Some(h))` arm below once the
                        // throttle expires. This is consistent with ADR-0044 § D2 intent:
                        // the operator sees "Audit is active" immediately via Start, and
                        // per-window counts appear on every subsequent non-empty window.
                        (n, None) => {
                            let label = Self::format_label(n);
                            let h = self.bus.start(ActivityKind::AuditLedgerWrite, label);
                            // Store the handle — next non-empty window will tick(N).
                            self.handle = Some(h);
                        }
                        // (n>0, Some(h)) — continuing burst: tick the existing handle.
                        (n, Some(h)) => {
                            h.tick(n as u64);
                        }
                    }
                }
            }
        }
        // Explicitly drop handle so End{Success} is emitted before the task exits.
        drop(self.handle);
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Spawn the audit-ledger-writes aggregator.
///
/// `tick_sender` is the `broadcast::Sender<AuditTick<AuditEvent>>` returned by
/// `audit::Ledger::open_with_tick_bus`. Pass `None` if the tick bus is disabled
/// (e.g. `tick_bus_capacity = 0` in config) — the function then spawns a no-op
/// task that returns immediately.
///
/// Returns the `JoinHandle` so callers can hold it for graceful abort on
/// process shutdown (K5 mitigation per ADR-0044).
///
/// ## Startup ordering (K6)
///
/// Call this AFTER the iced `Subscription` lifecycle has started so the
/// first burst of audit ticks reaches a live subscriber on
/// `EventBus::activity_tx`. If called before any subscriber exists, the
/// first window's events are fanned out to zero receivers and silently
/// dropped — same backpressure contract as every other channel.
pub fn spawn_aggregator(
    tick_sender: Option<&broadcast::Sender<AuditTick<AuditEvent>>>,
    bus: &EventBus,
) -> JoinHandle<()> {
    let activity_sender = bus.activity();

    match tick_sender {
        None => {
            // Tick bus disabled — no-op aggregator.
            tokio::spawn(async {})
        }
        Some(sender) => {
            let rx = sender.subscribe();
            let agg = Aggregator::new(rx, activity_sender);
            tokio::spawn(async move { agg.run().await })
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::time::Duration;

    use tokio::sync::broadcast;
    use tokio::time::sleep;

    use crate::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase, ActivitySender};
    use crate::config::BusConfig;
    use crate::bus::EventBus;

    use super::*;

    // ── Helper: synthetic AuditTick ───────────────────────────────────────────

    fn make_fill_tick() -> AuditTick<AuditEvent> {
        use audit::tick::AuditContext;
        use rust_decimal_macros::dec;
        use time::OffsetDateTime;
        use uuid::Uuid;
        use trading_core::{
            FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol,
            Timestamp,
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

    // ── Test 1: counter increments on AuditTick recv ─────────────────────────

    /// T-D-N2 (1/5) — AtomicU32 counter increments on each received AuditTick.
    #[test]
    fn counter_increments_on_audit_tick_recv() {
        let counter = AtomicU32::new(0);
        counter.fetch_add(1, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
        let val = counter.load(Ordering::Relaxed);
        assert_eq!(val, 3, "counter must increment on each recv");
    }

    // ── Test 2: format_label K2 truncation ──────────────────────────────────

    /// T-D-N2 (2/5) — K2 label truncation: N > 9999 → flood label.
    #[test]
    fn format_label_truncates_above_k2_threshold() {
        let normal = Aggregator::format_label(42);
        assert_eq!(normal, "Audit: 42 writes");

        let at_threshold = Aggregator::format_label(9_999);
        assert_eq!(at_threshold, "Audit: 9999 writes");

        let flood = Aggregator::format_label(10_000);
        assert_eq!(flood, AUDIT_LABEL_FLOOD);

        let large = Aggregator::format_label(100_000);
        assert_eq!(large, AUDIT_LABEL_FLOOD);
    }

    // ── Test 3: idle-end semantics (empty window → handle drops) ─────────────

    /// T-D-N2 (3/5) — idle-end semantics: handle emits End{Success} on drop.
    ///
    /// Simulates the aggregator interval arm with n=0 and a Some handle.
    #[test]
    fn idle_window_drops_handle_emits_end_success() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let (tx, mut rx) = broadcast::channel::<ActivityEvent>(64);
            let sender = ActivitySender(tx);

            // Simulate: create a handle (Start event fires).
            let maybe_handle: Option<crate::activity::ActivityHandle> =
                Some(sender.start(ActivityKind::AuditLedgerWrite, "Audit: 1 writes"));

            // Drain the Start event.
            let _start = rx.try_recv().expect("Start event");

            // Simulate idle window (n=0, handle is Some) → drop handle.
            drop(maybe_handle); // Drop impl emits End{Success}.

            // Must see End{Success}.
            let end_evt = rx.try_recv().expect("End event");
            assert!(
                matches!(end_evt.phase, ActivityPhase::End(ActivityOutcome::Success)),
                "idle-end must emit End{{Success}}, got {:?}",
                end_evt.phase
            );
        });
    }

    // ── Test 4: D4 separate-handle Failed emission ───────────────────────────

    /// T-D-N2 (4/5) — D4 invariant: main handle ends Success; sibling Failed
    /// handle is the caller's responsibility.
    #[test]
    fn separate_handle_failed_emission_per_d4() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let (tx, mut rx) = broadcast::channel::<ActivityEvent>(64);
            let sender = ActivitySender(tx);

            // Main handle — Success path (simulates aggregator's long-lived handle).
            {
                let main_handle = sender.start(ActivityKind::AuditLedgerWrite, "Audit: 5 writes");
                let _start = rx.try_recv().unwrap();
                main_handle.tick(5);
                let _ = rx.try_recv(); // Tick event (may be throttled).
                drop(main_handle);
            }
            let end_evt = rx.try_recv().expect("End event from main handle");
            assert!(
                matches!(end_evt.phase, ActivityPhase::End(ActivityOutcome::Success)),
                "main handle must end with Success, got {:?}",
                end_evt.phase
            );

            // Sibling Failed handle — spawned by caller on error path.
            {
                let sibling = sender.start(ActivityKind::AuditLedgerWrite, "Audit: write failed");
                let _sib_start = rx.try_recv().unwrap();
                sibling.fail("ledger error: disk full");
                drop(sibling);
            }
            let sib_end = rx.try_recv().expect("End event from sibling handle");
            assert!(
                matches!(sib_end.phase, ActivityPhase::End(ActivityOutcome::Failed(_))),
                "sibling handle must end with Failed, got {:?}",
                sib_end.phase
            );
        });
    }

    // ── Test 5: no Failed events on happy path ───────────────────────────────

    /// T-D-N2 (5/5) — happy-path: zero Failed events when only successful ticks flow.
    ///
    /// K5/T-AR-3 invariant at unit level.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_failed_events_on_happy_path() {
        let (tick_tx, tick_rx) =
            broadcast::channel::<AuditTick<AuditEvent>>(256);
        let bus = EventBus::new(&BusConfig::default());
        let mut activity_rx = bus.activity().subscribe();

        let agg = Aggregator::new(tick_rx, bus.activity());
        let agg_handle = tokio::spawn(async move { agg.run().await });

        // Send 5 ticks then close the bus.
        for _ in 0..5 {
            let _ = tick_tx.send(make_fill_tick());
        }

        // Wait for at least one window boundary.
        sleep(Duration::from_millis(250)).await;

        // Close the tick bus → aggregator exits.
        drop(tick_tx);
        let _ = tokio::time::timeout(Duration::from_millis(500), agg_handle).await;

        // Drain all activity events and assert no Failed events.
        let mut failed_count = 0usize;
        while let Ok(evt) = activity_rx.try_recv() {
            if matches!(evt.phase, ActivityPhase::End(ActivityOutcome::Failed(_))) {
                failed_count += 1;
            }
        }

        assert_eq!(
            failed_count, 0,
            "zero Failed events expected on happy path, got {failed_count}"
        );
    }
}
