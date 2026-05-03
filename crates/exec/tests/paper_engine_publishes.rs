//! T903a integration test — exercises the public API of
//! `exec::PaperEnginePublisher` exactly as `agent::runtime::run` will
//! call it.
//!
//! Mirrors the unit-test pair in `crates/exec/src/paper.rs::tests` but
//! lives at the crate boundary (`use exec::*`) so a future refactor
//! that breaks `pub use` re-exports trips this test independently.
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use exec::{FillPublisher, NullPublisher, PaperEnginePublisher};
use rust_decimal_macros::dec;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Position, Price, Quantity, Side, Symbol,
    Timestamp,
};

struct Recorder {
    fills: Mutex<Vec<Fill>>,
    positions: Mutex<Vec<Position>>,
}

impl Recorder {
    fn new() -> Self {
        Self {
            fills: Mutex::new(Vec::new()),
            positions: Mutex::new(Vec::new()),
        }
    }
}

impl FillPublisher for Recorder {
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

/// T903a — wired publisher receives both events for each fill
/// processed by the live-mode paper engine.
#[test]
fn t903a_paper_publishes_fill_and_position() {
    let recorder = Arc::new(Recorder::new());
    let pub_: Arc<dyn FillPublisher> = recorder.clone();
    let engine = PaperEnginePublisher::with_publisher(pub_);

    engine.on_fill(&sample_fill(), &sample_position());

    assert_eq!(recorder.fills.lock().unwrap().len(), 1);
    assert_eq!(recorder.positions.lock().unwrap().len(), 1);
}

/// T903a — series of 10 fills produces 10 published fills + 10
/// published position updates (one per fill).
#[test]
fn t903a_publish_counts_match_fill_count() {
    let recorder = Arc::new(Recorder::new());
    let pub_: Arc<dyn FillPublisher> = recorder.clone();
    let engine = PaperEnginePublisher::with_publisher(pub_);

    for _ in 0..10 {
        engine.on_fill(&sample_fill(), &sample_position());
    }

    assert_eq!(recorder.fills.lock().unwrap().len(), 10);
    assert_eq!(recorder.positions.lock().unwrap().len(), 10);
}

/// R15 invariant — backtests construct the engine with the
/// `NullPublisher` and the call sequence is a no-op.  Verifies the
/// type-level bytes-identity guarantee for `verify_anchors.sh`.
#[test]
fn t903a_backtest_path_is_byte_identical_no_op() {
    let engine = PaperEnginePublisher::with_publisher(Arc::new(NullPublisher::new()));
    // Process 50 fills — none of these may panic, allocate observable
    // shared state, or do any I/O.  If a regression added a side
    // effect, anchor reports would drift.
    for _ in 0..50 {
        engine.on_fill(&sample_fill(), &sample_position());
    }
}
