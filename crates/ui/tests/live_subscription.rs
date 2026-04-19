//! T32 acceptance — end-to-end drive the cockpit model from a fake
//! `agent::EventBus`.
//!
//! The full iced `Subscription` machinery is untestable without a running
//! iced application (it needs an `EventStream`), but the recipe is a thin
//! wrapper around `ui::live::stream_*`. This test spins up a real
//! `EventBus`, hands the per-channel streams into the `update()` state
//! machine, and asserts that within 2s both a `FillReceived` and a
//! `PnlRefreshed` arrive — the stand-in for the task's "live tape
//! advances within 2s of a replay bar" acceptance.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::config::BusConfig;
use agent::EventBus;
use futures::StreamExt;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, PnlSnapshot, Position, Price, Quantity, Side,
    StrategyId, StrategyLoadError, StrategyLoaded, StrategySwapped, Symbol, Timestamp,
};

use ui::live::{
    stream_fills, stream_mode, stream_pnl, stream_positions, stream_strategy_error,
    stream_strategy_loaded, stream_strategy_swapped,
};
use ui::state::{update, Cockpit, PanelState, StrategyStatus};

fn synthetic_fill(n: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: if n % 2 == 0 { Side::Buy } else { Side::Sell },
        qty: Quantity::new(dec!(0.1)).unwrap(),
        price: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        fee: Money::from_decimal(dec!(1.6003)),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::now(),
        local_ts: Timestamp::now(),
        liquidity: Liquidity::Taker,
    }
}

fn synthetic_pnl() -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(90_000)),
        unrealized: Money::from_decimal(dec!(250)),
        realized: Money::from_decimal(dec!(-120.50)),
        total_equity: Money::from_decimal(dec!(90_129.50)),
        daily_return: Money::from_decimal(dec!(129.50)),
        as_of: Timestamp::now(),
    }
}

fn synthetic_position() -> Position {
    Position {
        symbol: Symbol::new("BTCUSDT"),
        base_qty: dec!(0.25),
        cost_basis: Money::from_decimal(dec!(10_000)),
        last_mark: Price::new(dec!(40_050)).unwrap(),
        realized_pnl: Money::from_decimal(dec!(5)),
        unrealized_pnl: Money::from_decimal(dec!(7.5)),
    }
}

/// The T32 acceptance proxy: within 2s of the bus publishing events the
/// cockpit model sees both a fill-append and a P&L refresh.
#[tokio::test(flavor = "current_thread")]
async fn t32_cockpit_sees_fill_and_pnl_within_two_seconds() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Build the two streams that feed the Tape and P&L panels.
    let mut fills = Box::pin(stream_fills(&bus));
    let mut pnl = Box::pin(stream_pnl(&bus));

    // Give each subscriber a tick to register with the broadcast channel
    // before the agent publishes.
    tokio::task::yield_now().await;
    bus.publish_fill(synthetic_fill(1));
    bus.publish_pnl(synthetic_pnl());

    let mut cockpit = Cockpit::new();

    // Read one message from each stream within 2s.
    let fill_msg = timeout(Duration::from_secs(2), fills.next())
        .await
        .expect("fill arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, fill_msg);

    let pnl_msg = timeout(Duration::from_secs(2), pnl.next())
        .await
        .expect("pnl arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, pnl_msg);

    // Assertions — mirror the T32 acceptance text.
    match &cockpit.tape {
        PanelState::Ready(q) => assert_eq!(q.len(), 1, "tape should have the fill"),
        other => panic!("tape not ready: {:?}", other.variant_name()),
    }
    match &cockpit.pnl {
        PanelState::Ready(snap) => {
            assert_eq!(snap.total_equity.amount(), dec!(90_129.50));
        }
        other => panic!("pnl not ready: {:?}", other.variant_name()),
    }
}

/// Positions stream drives `PositionsRefreshed` and fills the positions
/// panel. Covers the second common cockpit message class mentioned in the
/// T32 acceptance ("at least one FillAppended and one PnLSnapshotChanged").
#[tokio::test(flavor = "current_thread")]
async fn t32_positions_stream_refreshes_cockpit() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut positions = Box::pin(stream_positions(&bus));
    tokio::task::yield_now().await;
    bus.publish_position(synthetic_position());

    let msg = timeout(Duration::from_secs(2), positions.next())
        .await
        .expect("positions arrived within 2s")
        .expect("stream produced a message");
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);

    match &cockpit.positions {
        PanelState::Ready(v) => assert_eq!(v.len(), 1),
        other => panic!("positions not ready: {:?}", other.variant_name()),
    }
}

/// Agent halt broadcast propagates to the cockpit banner within 2s.
/// Covers the T_FINAL_B acceptance path where `.halt` on disk trips the
/// kill switch which publishes `AgentMode::Halted` on the mode channel.
#[tokio::test(flavor = "current_thread")]
async fn t32_external_halt_flips_cockpit_banner() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut mode = Box::pin(stream_mode(&bus));
    tokio::task::yield_now().await;
    bus.publish_mode(agent::AgentMode::Halted {
        reason: "halt file detected".into(),
    });

    let msg = timeout(Duration::from_secs(2), mode.next())
        .await
        .expect("halt arrived within 2s")
        .expect("stream produced a message");
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);

    assert_eq!(cockpit.mode, ui::state::AgentMode::Halted);
    match &cockpit.kill {
        ui::state::KillState::Halted { reason } => {
            assert_eq!(reason.as_str(), "halt file detected");
        }
        other => panic!("kill state not halted: {other:?}"),
    }
}

// ── T526 — strategy-registry subscribers ────────────────────────────────────
//
// Each of the three channels (`strategy_loaded`, `strategy_swapped`,
// `strategy_error`) fan-in to a distinct `Message` variant. The tests drive
// a fake `EventBus`, publish one event, and assert the cockpit's state
// machine reflects it within 2s — matching the T32 contract carried forward
// into v0.5 per `spec/reports/ui-v05-blockers-2026-04-19.md`.

fn synthetic_strategy_loaded(id: &str) -> StrategyLoaded {
    StrategyLoaded {
        id: StrategyId::new(id),
        hash: [0xAAu8; 32],
        source_path: SmolStr::new(format!("config/strategies/{id}.toml")),
        ts: Timestamp::now(),
    }
}

fn synthetic_strategy_swapped(id: &str) -> StrategySwapped {
    StrategySwapped {
        id: StrategyId::new(id),
        old_hash: [0xAAu8; 32],
        new_hash: [0xBBu8; 32],
        source_path: SmolStr::new(format!("config/strategies/{id}.toml")),
        ts: Timestamp::now(),
    }
}

fn synthetic_strategy_load_error(id: Option<&str>) -> StrategyLoadError {
    StrategyLoadError {
        source_path: SmolStr::new("config/strategies/bad.toml"),
        strategy_id: id.map(StrategyId::new),
        error_code: SmolStr::new("toml_parse"),
        error_summary: SmolStr::new("unexpected token at line 3"),
        ts: Timestamp::now(),
    }
}

/// `strategy_loaded` events reach the cockpit model as `Message::StrategyLoaded`
/// within 2s and upsert a `Ready` row in the strategies panel.
#[tokio::test(flavor = "current_thread")]
async fn t526_strategy_loaded_stream_refreshes_cockpit() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut s = Box::pin(stream_strategy_loaded(&bus));
    tokio::task::yield_now().await;
    bus.publish_strategy_loaded(synthetic_strategy_loaded("btc_macd_trend"));

    let msg = timeout(Duration::from_secs(2), s.next())
        .await
        .expect("strategy_loaded arrived within 2s")
        .expect("stream produced a message");
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);

    match &cockpit.strategies {
        PanelState::Ready(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].id, StrategyId::new("btc_macd_trend"));
            assert_eq!(rows[0].status, StrategyStatus::Ready);
        }
        other => panic!("strategies not ready: {:?}", other.variant_name()),
    }
}

/// `strategy_swapped` events reach the cockpit model as `Message::StrategySwapped`
/// within 2s and flip the existing row's hash while clearing any prior error.
#[tokio::test(flavor = "current_thread")]
async fn t526_strategy_swapped_stream_updates_cockpit() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut loaded = Box::pin(stream_strategy_loaded(&bus));
    let mut swapped = Box::pin(stream_strategy_swapped(&bus));
    tokio::task::yield_now().await;
    bus.publish_strategy_loaded(synthetic_strategy_loaded("btc_macd_trend"));
    bus.publish_strategy_swapped(synthetic_strategy_swapped("btc_macd_trend"));

    let mut cockpit = Cockpit::new();
    let load_msg = timeout(Duration::from_secs(2), loaded.next())
        .await
        .expect("load arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, load_msg);

    let swap_msg = timeout(Duration::from_secs(2), swapped.next())
        .await
        .expect("swap arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, swap_msg);

    match &cockpit.strategies {
        PanelState::Ready(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].status, StrategyStatus::Ready);
            // Hash flipped from 0xAA… → 0xBB…
            assert!(
                rows[0].full_hash.starts_with("bb"),
                "expected new hash to start with 'bb', got {}",
                rows[0].full_hash
            );
        }
        other => panic!("strategies not ready: {:?}", other.variant_name()),
    }
}

/// `strategy_error` events reach the cockpit model as `Message::StrategyLoadError`
/// within 2s and flip the matching row into the per-row `Error` state while
/// the overall panel stays `Ready` (per R8 — malformed TOML doesn't tear down
/// the panel; old strategies keep running).
#[tokio::test(flavor = "current_thread")]
async fn t526_strategy_error_stream_flips_row_to_error() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let mut loaded = Box::pin(stream_strategy_loaded(&bus));
    let mut errored = Box::pin(stream_strategy_error(&bus));
    tokio::task::yield_now().await;
    bus.publish_strategy_loaded(synthetic_strategy_loaded("btc_macd_trend"));
    bus.publish_strategy_error(synthetic_strategy_load_error(Some("btc_macd_trend")));

    let mut cockpit = Cockpit::new();
    let load_msg = timeout(Duration::from_secs(2), loaded.next())
        .await
        .expect("load arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, load_msg);

    let err_msg = timeout(Duration::from_secs(2), errored.next())
        .await
        .expect("error arrived within 2s")
        .expect("stream produced a message");
    update(&mut cockpit, err_msg);

    match &cockpit.strategies {
        PanelState::Ready(rows) => {
            assert_eq!(rows.len(), 1);
            match &rows[0].status {
                StrategyStatus::Error(summary) => {
                    assert_eq!(summary.as_str(), "unexpected token at line 3");
                }
                other => panic!("expected per-row Error, got {other:?}"),
            }
        }
        other => panic!(
            "strategies panel should stay Ready on per-row error, got {:?}",
            other.variant_name()
        ),
    }
}
