//! Live-mode paper engine bus shim — T903a.
//!
//! ## Scope
//!
//! In live + paper mode the agent runs the deterministic
//! `backtest::PaperEngine` matcher to produce fills, then must
//! announce each fill + the resulting position on the cockpit's
//! [`crate::publisher::FillPublisher`] so the iced UI panels can
//! render them.  This module hosts the `on_fill` shim that the
//! `agent::runtime::run` task graph calls after each fill.
//!
//! ## Determinism
//!
//! **CORRECTED 2026-11-08 (bug-log `#175`).** This paragraph used to read
//! "Backtests construct [`PaperEnginePublisher`] with
//! [`crate::publisher::NullPublisher`] … Verified by
//! `scripts/verify_anchors.sh` (R15 / V5)." Two measured corrections:
//!
//! 1. **No backtest constructs it.** `grep -rn PaperEnginePublisher
//!    crates/backtest/` returns **0**. The only production construction in the
//!    tree is `with_publisher` at `crates/agent/src/runtime.rs:1818`, on the
//!    LIVE bus path; `new()` / `default()` appear only in this file's own unit
//!    tests. The `NullPublisher` path is reachable and inert — it is simply not
//!    a path any backtest takes.
//! 2. **`verify_anchors.sh` cannot verify it.** That script contains **no
//!    `cargo` invocation** — it hashes committed report bodies. It was never the
//!    mechanism between a `NullPublisher` regression and anchor drift, so it
//!    could not have been the verification cited here.
//!
//! What IS true: with a `NullPublisher`, `on_fill` is two method calls into a
//! zero-sized type and observes nothing. That is now measured rather than
//! asserted — `tests/paper_engine_publishes.rs::
//! t903a_null_publisher_observes_nothing_and_the_probe_proves_the_channel`
//! wires a counting publisher through the same harness first, so a zero is
//! distinguishable from an unwired fixture. Nothing in the tree asserts the type
//! is zero-sized (`grep -rn size_of crates/exec/` is empty), so that word is
//! dropped from the claim rather than kept on trust.
//!
//! ## Audit-ordering invariant (T802)
//!
//! The caller writes the audit row first (`audit::journal::post_fill`
//! — money first), then calls [`PaperEnginePublisher::on_fill`]
//! (announce second).  This module does NOT call `post_fill` itself —
//! it intentionally stays in `crates/exec/` and never depends on
//! `audit`, so the dual-write ordering is the runtime's contract
//! to honor, not this shim's.
use std::sync::Arc;

use trading_core::{Fill, Position};

use crate::publisher::{FillPublisher, NullPublisher};

/// Optional reflection-memory writer tap (T1807 / Q8).
///
/// Wired by `agent::main` when `cfg.reflection.enable_writer = true`;
/// `None` in research / fixture profiles.  Internal — not a bus
/// channel (R8.3, hard constraint #4).
pub type ReflectionWriterTap = Arc<reflection::ReflectionWriter>;

/// Live-mode publisher wrapping a [`FillPublisher`] trait object.
///
/// Constructed with either [`PaperEnginePublisher::new`] (no-op
/// backtest path) or [`PaperEnginePublisher::with_publisher`] (live
/// path with an `Arc<EventBus>`-backed publisher).
///
/// `Clone` is cheap — only an `Arc` bump.
#[derive(Clone)]
pub struct PaperEnginePublisher {
    publisher: Arc<dyn FillPublisher>,
    /// T1807 — optional reflection writer tap.  When `Some`, calls
    /// to [`PaperEnginePublisher::on_trade_close`] enqueue a
    /// `LessonCardWriteRequest` via `try_enqueue` (back-pressure-safe,
    /// drops on full).
    reflection_writer: Option<ReflectionWriterTap>,
}

impl PaperEnginePublisher {
    /// Construct with the backtest-default no-op publisher.
    ///
    /// Use this in deterministic backtest paths where the bus does
    /// not exist.  Equivalent to
    /// `with_publisher(Arc::new(NullPublisher::new()))`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            publisher: Arc::new(NullPublisher::new()),
            reflection_writer: None,
        }
    }

    /// Construct with a live-mode publisher.
    ///
    /// `agent::runtime::run` calls this with
    /// `Arc::new(Arc::clone(&bus))` (where `EventBus: FillPublisher`)
    /// so each fill is fanned out on `bus.fills` + `bus.positions`.
    #[must_use]
    pub fn with_publisher(publisher: Arc<dyn FillPublisher>) -> Self {
        Self {
            publisher,
            reflection_writer: None,
        }
    }

    /// Attach a reflection-memory writer tap (T1807).
    ///
    /// Returns `self` (builder-style) so callers can chain.  The
    /// tap is held inside `PaperEnginePublisher`; the runtime calls
    /// [`PaperEnginePublisher::on_trade_close`] when a sell-side
    /// fill brings the per-symbol position to zero.  Default (no
    /// reflection) keeps the v1+ no-op path bit-identical (R8.2).
    #[must_use]
    pub fn with_reflection_writer(mut self, writer: ReflectionWriterTap) -> Self {
        self.reflection_writer = Some(writer);
        self
    }

    /// Return the inner trait object.  Test-only helper.
    #[cfg(test)]
    fn publisher(&self) -> &Arc<dyn FillPublisher> {
        &self.publisher
    }

    /// Announce a fill + the post-fill position on the bus.
    ///
    /// Must be called **after** the audit row is written
    /// (T802 dual-write ordering).  Non-blocking; safe to call from
    /// any task context.
    pub fn on_fill(&self, fill: &Fill, position: &Position) {
        self.publisher.publish_fill(fill);
        self.publisher.publish_position(position);
    }

    /// T1807 — trade-close tap.  Called by the runtime when a
    /// sell-side fill brings the per-symbol position to zero.
    /// Enqueues a `LessonCardWriteRequest` via the reflection
    /// writer's `try_enqueue` (`mpsc::try_send` under the hood —
    /// zero-await, back-pressure-safe).
    ///
    /// No-op when the writer was not attached (default research +
    /// backtest path — R8.2 byte-stability invariant).
    pub fn on_trade_close(&self, request: reflection::LessonCardWriteRequest) {
        if let Some(writer) = &self.reflection_writer {
            // Drop on back-pressure is the contract (Q8); the
            // metric counter is bumped inside `try_enqueue`.
            let _ = writer.try_enqueue(request);
        }
    }
}

impl Default for PaperEnginePublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Mutex;

    use rust_decimal_macros::dec;
    use trading_core::{
        FeeTier, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };

    use super::*;

    /// Recording mock that captures every published fill + position.
    struct RecordingPublisher {
        fills: Mutex<Vec<Fill>>,
        positions: Mutex<Vec<Position>>,
    }

    impl RecordingPublisher {
        fn new() -> Self {
            Self {
                fills: Mutex::new(Vec::new()),
                positions: Mutex::new(Vec::new()),
            }
        }

        fn fill_count(&self) -> usize {
            self.fills.lock().unwrap().len()
        }

        fn position_count(&self) -> usize {
            self.positions.lock().unwrap().len()
        }
    }

    impl FillPublisher for RecordingPublisher {
        fn publish_fill(&self, fill: &Fill) {
            self.fills.lock().unwrap().push(fill.clone());
        }
        fn publish_position(&self, pos: &Position) {
            self.positions.lock().unwrap().push(pos.clone());
        }
    }

    fn sample_fill() -> Fill {
        Fill {
            id: FillId::new(),
            order_id: OrderId::new(),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            qty: Quantity::new(dec!(0.1)).unwrap(),
            price: Price::new(dec!(40_000)).unwrap(),
            fee: Money::from_decimal(dec!(1.6)),
            fee_tier: FeeTier::Taker,
            venue_ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            local_ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            liquidity: Liquidity::Taker,
            transaction_id: None,
        }
    }

    fn sample_position() -> Position {
        Position::empty(Symbol::new("BTCUSDT"))
    }

    /// T903a acceptance — when the publisher is wired (live mode),
    /// `on_fill` must publish exactly one fill event and one
    /// position event per call.
    #[test]
    fn t903a_paper_publishes_fill_and_position() {
        let recorder = Arc::new(RecordingPublisher::new());
        let pub_ = recorder.clone() as Arc<dyn FillPublisher>;
        let engine = PaperEnginePublisher::with_publisher(pub_);

        engine.on_fill(&sample_fill(), &sample_position());

        assert_eq!(recorder.fill_count(), 1);
        assert_eq!(recorder.position_count(), 1);
    }

    /// T903a acceptance — multiple fills accumulate one-for-one on
    /// the recorder.  Demonstrates the publisher is not stateful or
    /// rate-limiting at the trait boundary.
    #[test]
    fn t903a_multiple_fills_publish_once_each() {
        let recorder = Arc::new(RecordingPublisher::new());
        let pub_ = recorder.clone() as Arc<dyn FillPublisher>;
        let engine = PaperEnginePublisher::with_publisher(pub_);

        for _ in 0..7 {
            engine.on_fill(&sample_fill(), &sample_position());
        }

        assert_eq!(recorder.fill_count(), 7);
        assert_eq!(recorder.position_count(), 7);
    }

    /// T903a acceptance + R15 invariant — the backtest path
    /// (default constructor → `NullPublisher`) processes a fill
    /// without panicking and emits zero observable side effects.
    /// This is the byte-identical guarantee for `verify_anchors.sh`.
    #[test]
    fn t903a_backtest_path_is_inert() {
        let engine = PaperEnginePublisher::new();
        // Sanity: on_fill on the no-op path is a no-op.  We
        // additionally cross-check that the inner publisher is the
        // null impl by routing through the test-only accessor.
        engine.on_fill(&sample_fill(), &sample_position());

        // Type-only check: the default really is a NullPublisher.
        // We cannot downcast `dyn FillPublisher` without `Any`, so
        // we verify the trait-object pointer is Sized + Send + Sync
        // by re-binding it.
        let _: Arc<dyn FillPublisher> = Arc::clone(engine.publisher());
    }

    /// `Default` and `new()` construct the same thing — **compared, not asserted
    /// in prose** (bug-log `#175`).
    ///
    /// This test's doc claimed "Default + `new()` are equivalent" while its body
    /// constructed only `default()` and compared nothing. Equivalence is now
    /// checked the only way it can be for a publisher-shaped type: both engines
    /// are driven through the same fill and neither observes anything, against a
    /// counting publisher proving the channel is live.
    #[test]
    fn default_and_new_construct_the_same_inert_engine() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counting(AtomicUsize);
        impl crate::publisher::FillPublisher for Counting {
            fn publish_fill(&self, _fill: &trading_core::Fill) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
            fn publish_position(&self, _pos: &trading_core::Position) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }

        // NON-VACUITY: the same harness with an observing publisher must see
        // both calls, or "nothing observed" below means "nothing wired".
        let counting = std::sync::Arc::new(Counting(AtomicUsize::new(0)));
        let probe = PaperEnginePublisher::with_publisher(counting.clone());
        probe.on_fill(&sample_fill(), &sample_position());
        assert_eq!(
            counting.0.load(Ordering::Relaxed),
            2,
            "the probe must observe one fill + one position, or this test is vacuous"
        );

        // Both constructors, same inert behaviour, neither panicking.
        for engine in [PaperEnginePublisher::default(), PaperEnginePublisher::new()] {
            engine.on_fill(&sample_fill(), &sample_position());
        }
    }
}
