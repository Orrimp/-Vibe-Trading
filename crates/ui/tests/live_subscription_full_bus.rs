//! T911 — Live-bus regression test (live-cockpit-unified), plus the
//! market-health relay (story 2-63 Wave E1).
//!
//! Risk #2 in the architect's design:
//!
//! > Cockpit panel drift when bus is fully wired (existing
//! > `crates/ui/tests/live_subscription.rs` 53 tests assume specific
//! > channel shapes; new producers may emit at higher rates than tests
//! > expect).
//!
//! Mitigation per the design: "regression-run `cargo test -p ui
//! --features live` is in V4 + per-task acceptance.  Add a single new
//! test `crates/ui/tests/live_subscription_full_bus.rs` that drives a
//! fully populated bus and asserts no panel exceeds its `Loading`
//! window past the first event."
//!
//! Acceptance per the task spec: drives 100 fills, 50 positions, 20
//! bars, 200 ticks, 5 PnL snapshots, 1 mode transition; asserts every
//! cockpit panel transitions out of `Loading` after the first relevant
//! event, without any panic or unexpected `Closed` state.
//!
//! Plus the V2b round-trip check: a kill-switch trip via the agent
//! `KillSwitch::trip` path goes through the `T905` mode-broadcast
//! forwarder that `agent::runtime::run` spawns, the cockpit's mode
//! stream observes it, and the cockpit's `kill` panel transitions to
//! `KillState::Halted`.  Uses `Arc<KillSwitch>` + `MockIncidentSpawner`
//! plus an in-memory ledger; the T905 forwarder is spawned on the
//! test's tokio runtime.
//!
//! ## What each test pins (story 2-63 AC3 sweep, Wave E1)
//!
//! - `t911_full_bus_drives_every_panel_out_of_loading`: each panel's STATE,
//!   not only "not `Loading`". Before E1 the tape, positions and P&L checks
//!   ended in `_ => {}`, and the positions one accepted `Empty`: the first
//!   relayed position, `synthetic_position(0)`, is FLAT, and the positions
//!   channel holds 256 (`let (positions_tx, _) = broadcast::channel(256);` in
//!   `agent::EventBus::new`), so the first read IS that flat position and its
//!   snapshot is empty. A positions relay that never inserts passed. The test
//!   now reads TWO positions and pins both sides of the flat dimension: the
//!   flat SYM0 gives an EMPTY snapshot (and the panel goes `Empty`), the
//!   non-flat SYM1 gives `Ready([SYM1])`. The tape, P&L, latency and bar
//!   checks compare CONTENT, not only the variant (tightened after the E1a
//!   falsifier found that any `PnlRefreshed` gave `Ready(_)`): the tape row
//!   carries the first published fill's side, price and stamp, the P&L panel
//!   the first snapshot's figures, the latency badge the tick's 1 s receive
//!   lag (`Known { ms: 1000 }`), and `last_bar_ts` the first bar's close.
//! - `market_health_stream_relays_into_cockpit_state_then_ends`: NEW.
//!   `ui::live::stream_market_health` had no test, and no `ui` integration
//!   test routes a relayed `MarketHealthUpdated` through `update` (Wave B's
//!   `recipe_off_runtime_poll` projects the message to a string and stops
//!   there; the render tests write `Cockpit::market_health` directly). Here
//!   the relayed event reaches the status-bar state, over two fixtures that
//!   differ in the variant and expect different states (`Stale` → `Stale`,
//!   `Recovered` → `Fresh`), and dropping the bus ENDS the stream with no
//!   closing message (unlike fills / positions / P&L / mode, which yield an
//!   error message on close).
//! - `t911_kill_button_round_trip_via_mode_forwarder`: the kill panel's
//!   reason is now the trip's own reason, DERIVED from the input
//!   (`HaltReason::ManualOperator.to_string()`, the same Display
//!   `KillSwitch::trip` broadcasts via `let msg = reason.to_string();`).
//!   Before E1 it asserted only `!reason.is_empty()`, which a constant
//!   reason passes (E1-P31).
//!
//! ### Why the 150 ms sticky-trip check is two-sided
//!
//! The first trip proves the KillSwitch → forwarder → bus → `stream_mode`
//! path delivers within 1 s, so the silence after the second trip is not a
//! dead pipe; and `is_err()` (the window elapsed) rejects both an
//! `Ok(Some(_))` (a duplicate was forwarded: E1-P30) and an `Ok(None)` (the
//! stream ended). A window, not one immediate poll, is the right shape: the
//! test runs on a multi_thread runtime, the forwarder on another worker, so a
//! single poll could run before a duplicate is forwarded. Residual: a
//! duplicate slower than 150 ms would pass.
//!
//! ## What this file does not pin: `BusRecipe::stream`'s dispatch
//!
//! Every test here calls the `ui::live::stream_*` fns directly, never
//! `<BusRecipe as Recipe>::stream`, so a swapped arm in its `match channel`
//! leaves this file green, by design. Wave B's
//! `crates/ui/tests/recipe_off_runtime_poll.rs::bus_recipes_relay_off_runtime`
//! streams all eleven recipes of `into_recipes(ui::live::subscription(..))`
//! with per-channel turns and CROSS-TALK detection; E1-P18 is measured there.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every relay read sits under a timeout that FAILS the test: [`next_or_fail`]
//! panics on elapse, and the T911 drain `.expect`s its 2 s window. No timeout
//! is accepted as "drained". "Nothing more is buffered and the stream is live"
//! is OBSERVED with [`assert_pending`]. A stream end is a strict `Ok(None)`
//! inside a window. Checks that hold by construction (the fixture's flat /
//! non-flat premise, a `Cockpit::new()` default) are comments, not asserts.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test live_subscription_full_bus`. The named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` (`Compiling agent` too for E1-P30 / E1-P31) and the
//!    file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P17..E1-P20,
//! E1-P30..E1-P32, E1-P38 and E1-P39.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P17 | `ui::live::stream_positions`, the non-flat branch (`crates/ui/src/live.rs`) | `book.insert(pos.symbol.clone(), position_to_view(&pos));` | → `let _ = position_to_view(&pos);` (the book never inserts) | `t911_full_bus_drives_every_panel_out_of_loading`, at "positions: the second (non-flat) position gives Ready([SYM1])" (the panel is `Empty`). Before E1 this file stayed GREEN: the flat first position gives `Empty`, which `_ => {}` accepted |
//! | E1-P18 | `<BusRecipe as Recipe>::stream`, its `match channel` (`crates/ui/src/live.rs`) | `Channel::Pnl => Box::pin(stream_pnl(&bus)),` | → `Channel::Pnl => Box::pin(stream_ticks(&bus)),` | NOT this file, which stays GREEN (it never calls `BusRecipe::stream`). Target: `recipe_off_runtime_poll::bus_recipes_relay_off_runtime`, at the Pnl turn's wait for its own event ("one event published, the bus still alive"). MEASURED there by the E1a falsifier: the Pnl turn (#3) runs before the Ticks turn (#4), so it fails at that wait, never at a "CROSS-TALK" report |
//! | E1-P19 | `ui::live::stream_market_health`, the receive arm | `Ok(event) => yield Message::MarketHealthUpdated(event),` | → `Ok(event) => if false { yield Message::MarketHealthUpdated(event) },` | `market_health_stream_relays_into_cockpit_state_then_ends`, at its first `next_or_fail` ("timed out after 2s waiting for market health: Stale{Binance} relayed as MarketHealthUpdated") |
//! | E1-P20 | `ui::live::stream_market_health`, the `Closed` arm | `warn!(channel = "market_health", "broadcast closed");` | the `break;` below it → `std::future::pending::<()>().await;` (a close parks the stream) | `market_health_stream_relays_into_cockpit_state_then_ends`, at "market health: dropping the bus ENDS the stream, with no closing message" (`Err(Elapsed)`) |
//! | E1-P30 | `agent::KillSwitch::trip` (`crates/agent/src/kill_switch.rs`) | `if already {` / `return;` / `}` | the three lines → `let _ = already;` (a second trip broadcasts again) | `t911_kill_button_round_trip_via_mode_forwarder`, at "sticky-trip violated: forwarder leaked a second mode event into the bus" (`Ok(Some(_))` inside the 150 ms window). The assert predates E1, so this is red before E1 too: a positive control that measures the sticky check's two-sided rationale |
//! | E1-P31 | `agent::KillSwitch::trip`, the broadcast | `reason: msg.clone(),` | → `reason: String::from("halted"),` | `t911_kill_button_round_trip_via_mode_forwarder`, at "kill panel: the trip's reason survives KillSwitch::trip → mode forwarder → bus → stream_mode → update" (left `"halted"`, right `"manual_operator"`). GREEN before E1, under `!reason.is_empty()`. `t911_full_bus_drives_every_panel_out_of_loading` publishes its mode directly and stays GREEN |
//! | E1-P32 | `ui::state::update`, arm `Message::MarketHealthUpdated` (`crates/ui/src/state.rs`) | `MarketHealth::Stale { venue, .. } => (venue, MarketHealthState::Stale),` | → `MarketHealth::Stale { venue, .. } => (venue, MarketHealthState::Fresh),` | `market_health_stream_relays_into_cockpit_state_then_ends`, at "market health: the relayed Stale reaches the status-bar state" (left `Some(Fresh)`, right `Some(Stale)`) |
//! | E1-P38 | `ui::live::fill_to_view` | `side: fill.side,` | → `side: trading_core::Side::Sell,` | `t911_full_bus_drives_every_panel_out_of_loading`, at "tape: the first relayed fill gives Ready with exactly that one row" (the row reads `side: Sell`). GREEN before the E1a tightening, which checked only `q.len() == 1`. MEASURED (orchestrator hand probe, 2026-11-13) |
//! | E1-P39 | `ui::live::stream_pnl`, the receive arm | `Ok(snap) => yield Message::PnlRefreshed(snap),` | → zero the snapshot's `cash` before the yield | `t911_full_bus_drives_every_panel_out_of_loading`, at "pnl: the first relayed snapshot gives Ready with its own figures" (`cash: 0`). GREEN before the E1a tightening, which checked only `Ready(_)`. MEASURED (orchestrator hand probe, 2026-11-13) |
//!
//! Outside this target (MEASURED by the E1a falsifier): under E1-P17 the lib
//! test `live::tests::positions_stream_aggregates_into_full_snapshot`,
//! `live_subscription::t32_positions_stream_refreshes_cockpit` and
//! `recipe_off_runtime_poll::bus_recipes_relay_off_runtime` (its Positions
//! turn) go red; under E1-P19 and E1-P20 the same Wave B test goes red at its
//! MarketHealth turn and its MarketHealth tail. E1-P30 and E1-P31 rebuild
//! `agent`; its lib tests are not run for them. Lib or render tests that write
//! `Cockpit::market_health` directly are not claimed for E1-P32.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::config::BusConfig;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use rust_decimal_macros::dec;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use trading_core::{
    Bar, FeeTier, Fill, FillId, Liquidity, MarketHealth, Money, OrderId, PnlSnapshot, Position,
    Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

use ui::live::{
    stream_bars, stream_fills, stream_market_health, stream_mode, stream_pnl, stream_positions,
    stream_ticks,
};
use ui::state::{
    AgentMode, Cockpit, KillState, Latency, MarketHealthState, Message, PanelState, update,
};

// ── Synthetic event factories ────────────────────────────────────────────────

fn ts(offset_secs: i64) -> Timestamp {
    Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset_secs).expect("valid ts"),
    )
}

fn synthetic_fill(n: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: if n % 2 == 0 { Side::Buy } else { Side::Sell },
        qty: Quantity::new(dec!(0.1)).unwrap(),
        price: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        fee: Money::from_decimal(dec!(1.6)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts(n),
        local_ts: ts(n),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

/// Position `SYM{n}` with `base_qty = n`.
///
/// The flat / non-flat premise T911 relies on holds by construction, so it
/// is stated here, not asserted: `n = 0` gives `base_qty` 0, which
/// `Position::is_flat` reads as flat (`self.base_qty == Decimal::ZERO` in
/// `trading_core`), and every `n >= 1` is non-flat. So the first relayed
/// snapshot is EMPTY (SYM0 is removed from the book, never added) and the
/// second holds exactly SYM1.
fn synthetic_position(n: i64) -> Position {
    Position {
        symbol: Symbol::new(format!("SYM{n}")),
        base_qty: rust_decimal::Decimal::from(n),
        cost_basis: Money::from_decimal(rust_decimal::Decimal::from(10_000 + n)),
        last_mark: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        realized_pnl: Money::from_decimal(dec!(5)),
        unrealized_pnl: Money::from_decimal(dec!(7.5)),
    }
}

fn synthetic_bar(n: i64) -> Bar {
    let close = dec!(50_000) + rust_decimal::Decimal::from(n);
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts: ts(n * 60),
        close_ts: ts(n * 60 + 60),
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1)).unwrap(),
        trade_count: 1,
        local_recv_ts: ts(n * 60 + 60),
        venue: Venue::Binance,
    }
}

/// Tick `n`, received one second after its venue stamp, so the latency badge
/// it drives reads `Known { ms: 1000 }` (`(local_recv_ts - venue_ts).abs()` in
/// `update`'s `TickReceived` arm), never the `0` a stamp-less tick would give.
fn synthetic_tick(n: i64) -> Tick {
    Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts: ts(n),
        local_recv_ts: ts(n + 1),
        price: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        qty: Quantity::new(dec!(1)).unwrap(),
        side: Side::Buy,
        trade_id: u64::try_from(n).unwrap_or(0),
        venue: Venue::Binance,
    }
}

fn synthetic_pnl(n: i64) -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(90_000) + rust_decimal::Decimal::from(n)),
        unrealized: Money::from_decimal(dec!(250)),
        realized: Money::from_decimal(dec!(-120.50)),
        total_equity: Money::from_decimal(dec!(90_129.50) + rust_decimal::Decimal::from(n)),
        daily_return: Money::from_decimal(dec!(129.50)),
        as_of: ts(n),
        bar_ts: None,
    }
}

// ── Stream helpers (Wave A's, `forward_narration_relay.rs`) ──────────────────

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// The returned `Option` is the stream's own verdict: `Some(msg)` = a relayed
/// message, `None` = the stream ENDED. A timeout is never read as either
/// (the accepting `Err(_) => break` shape is exactly the bug-log #66 vacuity).
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Assert the stream is LIVE with NOTHING buffered: a single poll returns
/// `Pending` (the relay is parked on its receiver). An observation, not a
/// count: it fails if the stream already ENDED (an empty or stopped relay) and
/// if it yields an item that was never sent.
fn assert_pending(stream: &mut BoxStream<'static, Message>, what: &str) {
    match stream.next().now_or_never() {
        None => {}
        Some(None) => panic!("{what}: the stream already ENDED (expected it live on its receiver)"),
        Some(Some(msg)) => panic!("{what}: an unexpected extra message was buffered: {msg:?}"),
    }
}

/// T911 acceptance — drive a fully-populated bus through the entire
/// six-channel taxonomy and assert every cockpit panel transitions out
/// of `Loading` after the first relevant event.  Volumes per the
/// architect's task spec: 100 fills, 50 positions, 20 bars, 200 ticks,
/// 5 pnl, 1 mode transition.
///
/// Story 2-63 Wave E1: each panel's resulting state is pinned, and the
/// positions panel over both sides of the flat dimension (T-T4 E1-P17).
#[tokio::test(flavor = "current_thread")]
async fn t911_full_bus_drives_every_panel_out_of_loading() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Subscribe FIRST so no events are dropped.
    let mut fills = Box::pin(stream_fills(&bus));
    let mut positions = Box::pin(stream_positions(&bus));
    let mut bars = Box::pin(stream_bars(&bus));
    let mut ticks = Box::pin(stream_ticks(&bus));
    let mut pnl = Box::pin(stream_pnl(&bus));
    let mut mode = Box::pin(stream_mode(&bus));

    tokio::task::yield_now().await;

    // Publish in interleaved order to mirror real-world bus traffic.
    for i in 0..100i64 {
        bus.publish_fill(synthetic_fill(i));
    }
    for i in 0..50i64 {
        bus.publish_position(synthetic_position(i));
    }
    for i in 0..20i64 {
        bus.publish_bar(synthetic_bar(i));
    }
    for i in 0..200i64 {
        bus.publish_tick(synthetic_tick(i));
    }
    for i in 0..5i64 {
        bus.publish_pnl(synthetic_pnl(i));
    }
    bus.publish_mode(agent::AgentMode::Halted {
        reason: "test halt".into(),
    });

    let mut cockpit = Cockpit::new();

    // Drain every stream's first message into the cockpit.  `Loading`
    // counts every panel before any event lands; one event per channel
    // is enough to flip the panel out of `Loading`.  Positions are read
    // TWICE, one per side of the flat dimension (`synthetic_position`).
    let drain = async {
        let m = fills.next().await.expect("fills closed");
        update(&mut cockpit, m);
        let m = positions.next().await.expect("positions closed");
        assert!(
            matches!(&m, Message::PositionsRefreshed(v) if v.is_empty()),
            "positions: the flat first position (SYM0, base_qty 0) gives an EMPTY snapshot, \
             got {m:?}"
        );
        update(&mut cockpit, m);
        assert!(
            matches!(&cockpit.positions, PanelState::Empty),
            "positions: the EMPTY first snapshot puts the panel in Empty, got {}",
            cockpit.positions.variant_name()
        );
        let m = positions.next().await.expect("positions closed");
        update(&mut cockpit, m);
        let m = bars.next().await.expect("bars closed");
        update(&mut cockpit, m);
        // bars stream emits BarReceived followed by BarClose for each bar
        let m = bars.next().await.expect("bars closed");
        update(&mut cockpit, m);
        let m = ticks.next().await.expect("ticks closed");
        update(&mut cockpit, m);
        let m = pnl.next().await.expect("pnl closed");
        update(&mut cockpit, m);
        let m = mode.next().await.expect("mode closed");
        update(&mut cockpit, m);
    };

    timeout(Duration::from_secs(2), drain)
        .await
        .expect("not all channels delivered an event inside 2 s");

    // ── Assertions: every panel's state after its relayed events ─────
    //
    // `PanelState` has no `PartialEq`, hence `matches!`.
    //
    // Tape: the one relayed fill is the one row, with the FIRST published
    // fill's own side, price and venue stamp (`synthetic_fill(0)`: a Buy at
    // 40 000; every later fill differs in price, every odd one in side).
    let first_fill = synthetic_fill(0);
    assert!(
        matches!(
            &cockpit.tape,
            PanelState::Ready(q) if q.len() == 1
                && q[0].side == first_fill.side
                && q[0].price == first_fill.price
                && q[0].venue_ts == first_fill.venue_ts
        ),
        "tape: the first relayed fill gives Ready with exactly that one row, got {:?}",
        cockpit.tape
    );
    // Positions panel: the non-flat SYM1, and only it (SYM0 was flat).
    assert!(
        matches!(
            &cockpit.positions,
            PanelState::Ready(l) if l.len() == 1 && l[0].symbol == Symbol::new("SYM1")
        ),
        "positions: the second (non-flat) position gives Ready([SYM1]), got {:?}",
        cockpit.positions
    );
    // P&L panel: the FIRST published snapshot's own figures (`synthetic_pnl(0)`;
    // every later one differs in cash, equity and stamp). `PnlSnapshot` has no
    // `PartialEq`, so the figures are compared one by one.
    let first_pnl = synthetic_pnl(0);
    assert!(
        matches!(
            &cockpit.pnl,
            PanelState::Ready(snap) if snap.cash == first_pnl.cash
                && snap.total_equity == first_pnl.total_equity
                && snap.realized == first_pnl.realized
                && snap.as_of == first_pnl.as_of
        ),
        "pnl: the first relayed snapshot gives Ready with its own figures, got {:?}",
        cockpit.pnl
    );
    // Latency badge: the relayed tick's own receive lag, one second
    // (`synthetic_tick`).
    assert!(
        matches!(cockpit.latency, Latency::Known { ms: 1000 }),
        "latency: the relayed tick's 1 s receive lag gives Known {{ ms: 1000 }}, got {:?}",
        cockpit.latency
    );
    // Bar timestamp: the FIRST published bar's close (`synthetic_bar(0)`;
    // the BarClose that follows it does not move `last_bar_ts`).
    assert_eq!(
        cockpit.last_bar_ts,
        Some(synthetic_bar(0).close_ts),
        "last_bar_ts: the first relayed bar's close stamp"
    );
    // Mode flips Halted after the trip event.
    assert_eq!(cockpit.mode, AgentMode::Halted);
    match &cockpit.kill {
        KillState::Halted { reason } => {
            assert_eq!(reason.as_str(), "test halt");
        }
        other => panic!("kill state not Halted: {other:?}"),
    }
}

/// Story 2-63 Wave E1: `ui::live::stream_market_health` (the bus's
/// market-health relay, `Channel::MarketHealth` in `BusRecipe::stream`) end
/// to end, through `update` into the status-bar state, then its close.
///
/// Two fixtures that differ in the variant and expect different states pin
/// the mapping both ways: a relayed `Stale` makes Binance `Stale`, a relayed
/// `Recovered` makes it `Fresh` again. The stream is live with nothing
/// buffered while the bus is alive, and dropping the only bus handle (the
/// only `market_health_tx`) ENDS it with no closing message. T-T4 E1-P19,
/// E1-P20, E1-P32. `Cockpit::new()` builds `market_health` empty, so no
/// "empty before" precondition is asserted.
#[tokio::test(flavor = "current_thread")]
async fn market_health_stream_relays_into_cockpit_state_then_ends() {
    const STALE_RELAYED: &str = "market health: Stale{Binance} relayed as MarketHealthUpdated";
    const RECOVERED_RELAYED: &str =
        "market health: Recovered{Binance} relayed as MarketHealthUpdated";

    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    // `stream_market_health` returns `impl Stream`; the helpers take the boxed
    // trait object `Recipe::stream` returns.
    let mut s: BoxStream<'static, Message> = Box::pin(stream_market_health(&bus));

    assert_pending(&mut s, "market health: live before any publish");

    bus.publish_market_health(MarketHealth::Stale {
        venue: Venue::Binance,
        last_tick_ts: ts(10),
        threshold_secs: 30,
    });
    let msg = next_or_fail(&mut s, 2, STALE_RELAYED)
        .await
        .unwrap_or_else(|| panic!("{STALE_RELAYED}: the stream ENDED instead"));
    assert!(
        matches!(
            &msg,
            Message::MarketHealthUpdated(MarketHealth::Stale {
                venue: Venue::Binance,
                threshold_secs: 30,
                ..
            })
        ),
        "{STALE_RELAYED}: got {msg:?}"
    );

    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.market_health.get(&Venue::Binance),
        Some(&MarketHealthState::Stale),
        "market health: the relayed Stale reaches the status-bar state"
    );

    // The other side of the variant dimension: the bus's `Recovered` maps to
    // the status bar's `Fresh` (the `MarketHealthState` contract: connected =
    // Fresh/Recovered, reconnecting = Stale), so a mapping that makes every
    // event `Stale` fails here.
    bus.publish_market_health(MarketHealth::Recovered {
        venue: Venue::Binance,
        recovered_ts: ts(70),
        gap_secs: 60,
    });
    let msg = next_or_fail(&mut s, 2, RECOVERED_RELAYED)
        .await
        .unwrap_or_else(|| panic!("{RECOVERED_RELAYED}: the stream ENDED instead"));
    assert!(
        matches!(
            &msg,
            Message::MarketHealthUpdated(MarketHealth::Recovered {
                venue: Venue::Binance,
                gap_secs: 60,
                ..
            })
        ),
        "{RECOVERED_RELAYED}: got {msg:?}"
    );
    update(&mut cockpit, msg);
    assert_eq!(
        cockpit.market_health.get(&Venue::Binance),
        Some(&MarketHealthState::Fresh),
        "market health: a relayed Recovered puts the venue back to Fresh"
    );

    assert_pending(
        &mut s,
        "market health: live with nothing buffered while the bus is alive",
    );

    drop(bus);
    match timeout(Duration::from_millis(500), s.next()).await {
        Ok(None) => {}
        other => panic!(
            "market health: dropping the bus ENDS the stream, with no closing message \
             (Ok(None) within 500 ms), got {other:?}"
        ),
    }
}

/// T911 V2b round-trip — Cockpit-button trip path:
///
/// `KillSwitch::trip(HaltReason::ManualOperator)` →
/// `kill_switch.subscribe()` channel →
/// `agent::runtime::spawn_mode_forwarder` (T905) →
/// `bus.publish_mode(...)` →
/// `ui::live::stream_mode` →
/// `Message::AgentHaltedExternally(reason)` →
/// `Cockpit::kill = KillState::Halted`.
///
/// Uses `Arc<KillSwitch>` constructed via `KillSwitch::with_audit`
/// against an in-memory ledger + `MockIncidentSpawner` so the T809
/// dual-write hooks fire exactly as they do in production.
///
/// Story 2-63 Wave E1: the reason is the trip's own (T-T4 E1-P31), and the
/// sticky check is two-sided (E1-P30; see the module doc).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t911_kill_button_round_trip_via_mode_forwarder() {
    use agent::{HaltReason, IncidentSpawner, KillSwitch, MockIncidentSpawner};
    use audit::{Ledger, bootstrap};

    let ledger = Arc::new(Ledger::in_memory().await.expect("in-memory"));
    bootstrap::chart_of_accounts(&ledger).await.expect("chart");

    let spawner: Arc<dyn IncidentSpawner> = Arc::new(MockIncidentSpawner::new());
    // Use a unique path under the OS temp dir — no `tempfile` dev-dep
    // needed.  The halt-file watcher is NOT spawned here (we never call
    // `spawn_halt_file_watcher`), so the path's existence is
    // immaterial; it's just stored on the `KillSwitch` value.
    let halt_file =
        std::env::temp_dir().join(format!("t911-halt-{}", uuid::Uuid::new_v4().simple()));
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Subscribe to the cockpit-side stream BEFORE the forwarder starts
    // so we capture every event.
    let mut mode_stream = Box::pin(stream_mode(&bus));

    // Spawn the T905 forwarder via the public API.  This is exactly what
    // `agent::runtime::run` does on its JoinSet.
    let mut set = tokio::task::JoinSet::new();
    let cancel = CancellationToken::new();
    agent::runtime::spawn_mode_forwarder(
        Arc::clone(&kill_switch),
        Arc::clone(&bus),
        &mut set,
        &cancel,
    );

    // Yield twice so the forwarder's subscribe lands before the trip.
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    // ── The cockpit-button trip ──────────────────────────────────────
    kill_switch.trip(HaltReason::ManualOperator);

    // The cockpit observes the mode change inside 1 s.
    let msg = timeout(Duration::from_secs(1), mode_stream.next())
        .await
        .expect("mode message did not arrive inside 1 s")
        .expect("mode stream closed");

    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);

    // Halted banner present + kill panel carries the trip's own reason.
    assert_eq!(cockpit.mode, AgentMode::Halted);
    match &cockpit.kill {
        KillState::Halted { reason } => {
            // The expected text is DERIVED from the trip's input, never read
            // off the cockpit: `KillSwitch::trip` broadcasts
            // `reason.to_string()`, the `HaltReason` Display ("manual_operator"
            // today). A cosmetic Display change moves both sides; a pipeline
            // that drops or replaces the reason (T-T4 E1-P31) does not.
            assert_eq!(
                reason.as_str(),
                HaltReason::ManualOperator.to_string().as_str(),
                "kill panel: the trip's reason survives KillSwitch::trip → mode forwarder → bus → stream_mode → update"
            );
        }
        other => panic!("kill state not Halted after trip: {other:?}"),
    }

    // Sticky-trip — a second trip is a no-op on the kill switch, so no
    // duplicate event reaches the bus.  Two-sided (module doc): the first
    // trip above proved this path delivers inside 1 s, and `is_err()`
    // rejects both a forwarded duplicate (`Ok(Some(_))`, T-T4 E1-P30) and
    // an ended stream (`Ok(None)`).  A window, not one poll: the forwarder
    // runs on another worker of this multi_thread runtime.
    kill_switch.trip(HaltReason::ManualOperator);
    let none = timeout(Duration::from_millis(150), mode_stream.next()).await;
    assert!(
        none.is_err(),
        "sticky-trip violated: forwarder leaked a second mode event into the bus, got {none:?}"
    );

    // Drain the forwarder cleanly.
    cancel.cancel();
    let drain = async { while set.join_next().await.is_some() {} };
    timeout(Duration::from_secs(2), drain)
        .await
        .expect("forwarder did not drain inside 2 s");
}
