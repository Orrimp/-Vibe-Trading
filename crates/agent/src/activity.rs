//! Activity types and RAII handle for the cockpit-activity-status-bar feature.
//!
//! This module owns the `ActivityEvent` type hierarchy (D1 / R1.2), the
//! `ActivitySender` thin-wrapper around the broadcast channel, and the
//! `ActivityHandle` RAII guard that auto-emits `End` on drop (R1.3).
//!
//! ## Design
//!
//! - `ActivityId` — monotonic `u64` counter, not a UUID (R1.2).
//! - `ActivityHandle::tick` — producer-side 100 ms throttle (R1.4).
//! - `Drop` — emits `End { Success }` by default; `End { Failed("dropped
//!   during panic") }` when `std::thread::panicking()` is true (R1.3 / F3).

use std::cell::Cell;
use std::time::{Duration, Instant};

use tokio::sync::broadcast;

// ── Wire types: RELOCATED to `trading_core` (bug-log #92, 2026-08-22) ────────
//
// `ActivityId` / `ActivityKind` / `ActivityOutcome` / `ActivityPhase` /
// `ActivityEvent` now live in `trading_core::activity`. They are plain data and
// `ui` consumes them, but `ui` declares `agent` only under its `live` feature —
// so importing them from here broke the documented `--no-default-features`
// build (three E0432 errors). The PRODUCER side below (`ActivitySender`,
// `ActivityHandle`, the tokio broadcast) stays here: it needs tokio, and
// `trading_core` deliberately has no async dependency.
//
// Re-exported so every existing `agent::ActivityEvent` path keeps compiling.
pub use trading_core::activity::{
    ActivityEvent, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase,
};

// ── ActivitySender ────────────────────────────────────────────────────────────

/// Thin wrapper around the activity broadcast sender.
///
/// Obtained via `EventBus::activity(&self)`. The wrapper is the public surface
/// for producers — they call `start(kind, label)` to get an `ActivityHandle`.
///
/// `ActivitySender` is `Clone` (it wraps a `broadcast::Sender` which is already
/// clone-friendly and cheap to clone).
#[derive(Clone, Debug)]
pub struct ActivitySender(pub(crate) broadcast::Sender<ActivityEvent>);

impl ActivitySender {
    /// Start a new activity, returning a RAII `ActivityHandle`.
    ///
    /// The handle's `Drop` impl emits an `End` event automatically — producers
    /// need only hold the handle for the lifetime of the work.
    ///
    /// An immediate `Start` event is emitted before returning. If there are
    /// no subscribers the send is silently dropped (matching every other
    /// channel's backpressure contract — see `EventBus::publish_fill`).
    #[must_use = "ActivityHandle must be held — dropping it emits End on the channel"]
    pub fn start(&self, kind: ActivityKind, label: impl Into<String>) -> ActivityHandle {
        let id = ActivityId::next();
        let label: String = label.into();
        let ts_ms = now_ms();
        let _ = self.0.send(ActivityEvent {
            id,
            kind,
            label: label.clone(),
            phase: ActivityPhase::Start { total_units: None },
            ts_ms,
        });
        ActivityHandle {
            sender: self.0.clone(),
            id,
            kind,
            label,
            outcome_at_drop: Cell::new(None),
            last_tick: Cell::new(Instant::now()),
            started_at: Instant::now(),
        }
    }

    /// Subscribe to the broadcast channel.
    ///
    /// Used by the `ActivityRecipe` in `crates/ui/src/live.rs` (Wave B).
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ActivityEvent> {
        self.0.subscribe()
    }
}

// ── ActivityHandle ─────────────────────────────────────────────────────────

const TICK_THROTTLE: Duration = Duration::from_millis(100);

/// RAII activity guard. Dropping it always emits an `End` event.
///
/// Producers obtain this via `ActivitySender::start`. Typical usage:
///
/// ```rust,ignore
/// let handle = bus.activity().start(ActivityKind::YahooPreload, "Yahoo BTC-USD 2y · downloading");
/// // ... do work ...
/// handle.tick(bars_done);
/// // handle drops here → emits End { Success }
/// ```
///
/// The handle is NOT `Send` by default (due to `Cell`). If you need to move
/// it across async boundaries wrap it in a `tokio::sync::Mutex` or use the
/// `fail` / `cancel` path from the same task that created it.
///
/// Note: `ActivityHandle` intentionally uses `Cell<_>` (not `RefCell` or
/// `Mutex`) for the throttle state because tick is always called from the
/// same thread/task that owns the handle — no cross-thread mutation is needed.
pub struct ActivityHandle {
    sender: broadcast::Sender<ActivityEvent>,
    id: ActivityId,
    kind: ActivityKind,
    label: String,
    /// `Some(outcome)` if `fail` or `cancel` has been called.
    outcome_at_drop: Cell<Option<ActivityOutcome>>,
    /// Tracks last successful tick emit for the 100 ms throttle (R1.4).
    last_tick: Cell<Instant>,
    /// Wall-clock start for computing `elapsed_ms` on each tick.
    started_at: Instant,
}

impl ActivityHandle {
    /// Emit a `Tick` progress event.
    ///
    /// Rate-limited: if fewer than 100 ms have elapsed since the last emit
    /// this call is a no-op (R1.4 / Q7=(a)).
    pub fn tick(&self, current: u64) {
        let now = Instant::now();
        if now.duration_since(self.last_tick.get()) < TICK_THROTTLE {
            return; // throttled — silent no-op
        }
        self.last_tick.set(now);
        let elapsed_ms = now.duration_since(self.started_at).as_millis() as u64;
        let _ = self.sender.send(ActivityEvent {
            id: self.id,
            kind: self.kind,
            label: self.label.clone(),
            phase: ActivityPhase::Tick {
                current,
                elapsed_ms,
            },
            ts_ms: now_ms(),
        });
    }

    /// Record a failure reason. The `End { Failed(reason) }` event is emitted
    /// when the handle is dropped. Calling this multiple times overwrites the
    /// previous reason (last wins).
    pub fn fail(&self, reason: impl Into<String>) {
        self.outcome_at_drop
            .set(Some(ActivityOutcome::Failed(reason.into())));
    }

    /// Record a cancellation. The `End { Cancelled }` event is emitted when
    /// the handle is dropped.
    pub fn cancel(&self) {
        self.outcome_at_drop.set(Some(ActivityOutcome::Cancelled));
    }
}

impl Drop for ActivityHandle {
    fn drop(&mut self) {
        let outcome = self.outcome_at_drop.take().unwrap_or_else(|| {
            if std::thread::panicking() {
                ActivityOutcome::Failed("dropped during panic".to_owned())
            } else {
                ActivityOutcome::Success
            }
        });
        let _ = self.sender.send(ActivityEvent {
            id: self.id,
            kind: self.kind,
            label: self.label.clone(),
            phase: ActivityPhase::End(outcome),
            ts_ms: now_ms(),
        });
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Current UTC time in milliseconds since the Unix epoch.
///
/// Uses `std::time::SystemTime` — acceptable outside backtest replay paths.
/// This module is only ever called from live UI producers (R4.1–R4.3), never
/// from deterministic backtest loops.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod activity_types {
    use std::collections::HashSet;
    use std::sync::atomic::Ordering;

    use tokio::sync::broadcast;

    use super::*;

    /// T-D-N1 — `ActivityEvent` can be cloned and the clone round-trips all
    /// fields correctly.
    #[test]
    fn activity_event_clone_round_trips() {
        let id = ActivityId(42);
        let event = ActivityEvent {
            id,
            kind: ActivityKind::YahooPreload,
            label: "test label".to_owned(),
            phase: ActivityPhase::Start {
                total_units: Some(100),
            },
            ts_ms: 1_700_000_000_000,
        };
        let cloned = event.clone();
        assert_eq!(cloned.id, event.id);
        assert_eq!(cloned.kind, event.kind);
        assert_eq!(cloned.label, event.label);
        assert_eq!(cloned.ts_ms, event.ts_ms);
        // Phase variants don't implement PartialEq — inspect structurally.
        assert!(matches!(
            cloned.phase,
            ActivityPhase::Start {
                total_units: Some(100)
            }
        ));
    }

    /// T-D-N1 — `ActivityKind` variants are hashable and usable as map keys.
    #[test]
    fn activity_kind_hash_round_trips() {
        let mut set = HashSet::new();
        set.insert(ActivityKind::YahooPreload);
        set.insert(ActivityKind::LabRun);
        set.insert(ActivityKind::Training);
        set.insert(ActivityKind::LlmCall);
        set.insert(ActivityKind::AuditLedgerWrite);
        // Insert duplicates — set must deduplicate.
        set.insert(ActivityKind::YahooPreload);
        set.insert(ActivityKind::LabRun);
        assert_eq!(set.len(), 5);
        assert!(set.contains(&ActivityKind::Training));
    }

    // T-D-N1 (`activity_id_atomic_monotonic`) MOVED to `trading_core::activity`
    // with the counter itself (bug-log #92, 2026-08-22). It reached into the
    // private `NEXT_ACTIVITY_ID` static, which now lives there — the test follows
    // the code it tests rather than reaching across a crate boundary.

    /// T-D-N3 — A dropped `ActivityHandle` emits exactly one `End { Success }`
    /// event on the channel (happy path).
    #[test]
    fn activity_handle_drop_emits_end() {
        let (tx, mut rx) = broadcast::channel::<ActivityEvent>(64);
        let sender = ActivitySender(tx);
        // Drop the handle immediately after creation.
        {
            let _handle = sender.start(ActivityKind::LabRun, "test run");
        }
        // We should have received: Start + End.
        let start_event = rx.try_recv().expect("Start event expected");
        assert!(matches!(start_event.phase, ActivityPhase::Start { .. }));
        let end_event = rx.try_recv().expect("End event expected");
        assert!(
            matches!(
                end_event.phase,
                ActivityPhase::End(ActivityOutcome::Success)
            ),
            "expected End(Success), got {:?}",
            end_event.phase
        );
        // No further events.
        assert!(rx.try_recv().is_err(), "unexpected extra event");
    }

    /// T-D-N3 — Tight-loop tick calls are throttled: ≤ 11 Tick events arrive
    /// for 100 calls fired within well under 100 ms per call.
    #[test]
    fn activity_handle_throttle_caps_at_10_hz() {
        let (tx, mut rx) = broadcast::channel::<ActivityEvent>(256);
        let sender = ActivitySender(tx);
        let handle = sender.start(ActivityKind::YahooPreload, "throttle test");
        // Fire 100 ticks in a tight loop (no sleep).
        for i in 0_u64..100 {
            handle.tick(i);
        }
        drop(handle);

        // Drain all events (clippy::while_let_loop fix 2026-05-26: prefer
        // `while let Ok(_) = ...` over `loop { match ... }`).
        let mut tick_count = 0usize;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev.phase, ActivityPhase::Tick { .. }) {
                tick_count += 1;
            }
        }
        // At most 11 Tick events (the 1-boundary-flake allowance from tasks.md).
        assert!(
            tick_count <= 11,
            "expected ≤ 11 Tick events, got {tick_count}"
        );
    }

    /// T-D-N3 — A handle dropped inside `catch_unwind` (simulating a panic)
    /// emits `End { Failed("dropped during panic") }`.
    #[test]
    fn activity_handle_drop_during_panic_emits_failed() {
        let (tx, mut rx) = broadcast::channel::<ActivityEvent>(64);
        let sender = ActivitySender(tx.clone());

        // Capture a raw sender clone so the channel stays open while we
        // assert — otherwise the receiver sees `Closed`.
        let _keep_open = tx.clone();

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _handle = sender.start(ActivityKind::Training, "panic test");
            panic!("simulated panic");
        }));

        // Drain to find the End event (clippy::while_let_loop fix).
        let mut found_failed = false;
        while let Ok(ev) = rx.try_recv() {
            if let ActivityPhase::End(ActivityOutcome::Failed(ref reason)) = ev.phase {
                assert!(
                    reason.contains("panic"),
                    "expected 'panic' in reason, got: {reason:?}"
                );
                found_failed = true;
            }
        }
        assert!(
            found_failed,
            "expected a Failed End event from the panicking drop"
        );
    }
}
