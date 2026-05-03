//! Live broadcast-bus subscription — T32.
//!
//! This module wires the cockpit's `Subscription` against the real
//! `agent::EventBus` (the broadcast bus the developer shipped in T31). It
//! replaces the `ui::fixtures`-backed stream used during Week 1.
//!
//! ### Design — same-process, shared-Arc
//!
//! Per the handoff contract in
//! `spec/reports/dev-week2-broadcast-api-2026-04-18.md`, v0 uses the
//! **same-process** model: the cockpit imports `agent` as a library
//! dependency and subscribes to the running agent's `EventBus` via a
//! shared `Arc<EventBus>`. No IPC (domain sockets / TCP) is involved.
//! When the cockpit is launched as a standalone binary without a bus
//! handed in, it constructs its own empty bus — every panel stays in
//! `Loading` because nothing publishes.
//!
//! ### Backpressure
//!
//! Each channel is a bounded `tokio::sync::broadcast`. A slow cockpit
//! receiver sees `RecvError::Lagged(n)` and we drop + log + continue
//! (the agent never blocks on the cockpit). `RecvError::Closed` means
//! the agent dropped the sender (crash / shutdown) — we emit a typed
//! error `Message` into the panel's error state so the operator sees
//! `CONNECTION_CHANNEL_CLOSED` copy instead of a frozen panel.
//!
//! ### Conversion
//!
//! The bus carries `core::Fill` / `core::Position`; the UI state uses
//! `core::FillView` / `core::PositionView`. Thin `From`-style adapters
//! live here (not in `core`, which stays free of UI concerns).
//!
//! Positions arrive one-symbol-at-a-time, but `Message::PositionsRefreshed`
//! expects the full list; we keep a per-subscription `HashMap<Symbol,
//! PositionView>` and re-emit the whole snapshot on each update.

#![cfg(feature = "live")]

use std::collections::HashMap;
use std::sync::Arc;

use agent::AgentMode as AgentBusMode;
use agent::EventBus;
use async_stream::stream;
use futures::stream::BoxStream;
use futures::Stream;
use iced::advanced::subscription::{from_recipe, EventStream, Hasher, Recipe};
use rust_decimal::Decimal;
use smol_str::SmolStr;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};
use trading_core::{Fill, FillView, Position, PositionView};

use crate::state::{AgentMode, Message};
use crate::strings;

// ── Public entry point ──────────────────────────────────────────────────────

/// Build a `Subscription` that emits cockpit `Message`s from every bus
/// channel the cockpit cares about.
///
/// Call from `cockpit::subscription()` under `#[cfg(feature = "live")]`.
pub fn subscription(bus: Arc<EventBus>) -> iced::Subscription<Message> {
    iced::Subscription::batch(vec![
        subscription_for(Channel::Fills, Arc::clone(&bus)),
        subscription_for(Channel::Positions, Arc::clone(&bus)),
        subscription_for(Channel::Pnl, Arc::clone(&bus)),
        subscription_for(Channel::Ticks, Arc::clone(&bus)),
        subscription_for(Channel::Bars, Arc::clone(&bus)),
        subscription_for(Channel::Mode, Arc::clone(&bus)),
        subscription_for(Channel::StrategyLoaded, Arc::clone(&bus)),
        subscription_for(Channel::StrategySwapped, Arc::clone(&bus)),
        subscription_for(Channel::StrategyError, bus),
    ])
}

/// Which bus channel a recipe is wired to. Used both to pick the stream
/// builder and to give the `Recipe` a stable identity hash so iced doesn't
/// duplicate or drop it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Fills,
    Positions,
    Pnl,
    Ticks,
    Bars,
    Mode,
    /// v0.5 — strategy registry: an initial load / reload of a strategy TOML.
    StrategyLoaded,
    /// v0.5 — strategy registry: a hot-swap of an existing strategy.
    StrategySwapped,
    /// v0.5 — strategy registry: a parse / typecheck rejection.
    StrategyError,
}

fn subscription_for(channel: Channel, bus: Arc<EventBus>) -> iced::Subscription<Message> {
    from_recipe(BusRecipe { channel, bus })
}

// ── Recipe ──────────────────────────────────────────────────────────────────

/// A single bus channel rendered as an iced `Recipe`. One recipe per
/// channel — batched by `subscription()`.
struct BusRecipe {
    channel: Channel,
    bus: Arc<EventBus>,
}

impl Recipe for BusRecipe {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::any::TypeId;
        use std::hash::Hash;
        TypeId::of::<Self>().hash(state);
        self.channel.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        let BusRecipe { channel, bus } = *self;
        // Eagerly subscribe each channel before the stream is first polled.
        // `bus` is only needed here for `bus.<channel>()`; the receiver
        // keeps the underlying `broadcast::Sender` alive via the bus's
        // internal Weak<->Strong wiring, so dropping `bus` after
        // subscription is safe.
        match channel {
            Channel::Fills => Box::pin(stream_fills(&bus)),
            Channel::Positions => Box::pin(stream_positions(&bus)),
            Channel::Pnl => Box::pin(stream_pnl(&bus)),
            Channel::Ticks => Box::pin(stream_ticks(&bus)),
            Channel::Bars => Box::pin(stream_bars(&bus)),
            Channel::Mode => Box::pin(stream_mode(&bus)),
            Channel::StrategyLoaded => Box::pin(stream_strategy_loaded(&bus)),
            Channel::StrategySwapped => Box::pin(stream_strategy_swapped(&bus)),
            Channel::StrategyError => Box::pin(stream_strategy_error(&bus)),
        }
    }
}

// ── Per-channel streams ─────────────────────────────────────────────────────

/// Fills → `FillReceived` + closure → `TapeError`.
///
/// Subscribes to the broadcast channel **synchronously** before returning
/// the async stream. This is critical: if we only subscribed inside the
/// `stream!` body, events published before the first `.next().await`
/// would be lost. Eager subscription closes that race.
pub fn stream_fills(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.fills();
    stream! {
        loop {
            match rx.recv().await {
                Ok(fill) => yield Message::FillReceived(fill_to_view(&fill)),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "fills", skipped = n, "broadcast lagged");
                    // Continue — lag is not fatal; the user sees a brief
                    // gap in the tape. A future patch could surface a
                    // "fell behind" banner via a dedicated message.
                }
                Err(RecvError::Closed) => {
                    yield Message::TapeError(SmolStr::new(strings::CONNECTION_CHANNEL_CLOSED));
                    break;
                }
            }
        }
    }
}

/// Positions → stateful `PositionsRefreshed(Vec<PositionView>)` — keeps a
/// per-subscription `HashMap` so the UI always sees the full snapshot.
pub fn stream_positions(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.positions();
    stream! {
        let mut book: HashMap<trading_core::Symbol, PositionView> = HashMap::new();
        loop {
            match rx.recv().await {
                Ok(pos) => {
                    if pos.is_flat() {
                        book.remove(&pos.symbol);
                    } else {
                        book.insert(pos.symbol.clone(), position_to_view(&pos));
                    }
                    let snapshot: Vec<PositionView> = book.values().cloned().collect();
                    yield Message::PositionsRefreshed(snapshot);
                }
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "positions", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    yield Message::PositionsError(SmolStr::new(
                        strings::CONNECTION_CHANNEL_CLOSED,
                    ));
                    break;
                }
            }
        }
    }
}

/// P&L → `PnlRefreshed`, `PnlError` on close.
pub fn stream_pnl(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.pnl();
    stream! {
        loop {
            match rx.recv().await {
                Ok(snap) => yield Message::PnlRefreshed(snap),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "pnl", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    yield Message::PnlError(SmolStr::new(strings::CONNECTION_CHANNEL_CLOSED));
                    break;
                }
            }
        }
    }
}

/// Ticks → `TickReceived` (drives latency badge). Lag is routine on the
/// tick channel (8192-deep) — debug-log only, never surface an error.
pub fn stream_ticks(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.ticks();
    stream! {
        loop {
            match rx.recv().await {
                Ok(tick) => yield Message::TickReceived(tick),
                Err(RecvError::Lagged(n)) => {
                    debug!(channel = "ticks", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    // Ticks are informational for latency; closing the
                    // channel is not a fatal error — the latency badge
                    // simply stays on its last reading.
                    debug!(channel = "ticks", "broadcast closed");
                    break;
                }
            }
        }
    }
}

/// Bars → `BarClose(close_ts)` + `BarReceived(bar)`. Both messages are
/// emitted so the state machine's `last_bar_ts` and panel refresh triggers
/// stay in sync.
pub fn stream_bars(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.bars();
    stream! {
        loop {
            match rx.recv().await {
                Ok(bar) => {
                    let ts = bar.close_ts;
                    yield Message::BarReceived(bar);
                    yield Message::BarClose(ts);
                }
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "bars", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    debug!(channel = "bars", "broadcast closed");
                    break;
                }
            }
        }
    }
}

/// Agent mode → `AgentModeChanged` or `AgentHaltedExternally` when the
/// kill switch trips outside the cockpit (e.g. `.halt` file on disk).
pub fn stream_mode(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.mode();
    stream! {
        loop {
            match rx.recv().await {
                Ok(mode) => yield mode_to_message(&mode),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "mode", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    // If the mode channel closes, the agent is gone; surface
                    // it as an "externally halted" event with a recognisable
                    // reason string.
                    yield Message::AgentHaltedExternally(SmolStr::new(
                        strings::CONNECTION_CHANNEL_CLOSED,
                    ));
                    break;
                }
            }
        }
    }
}

// ── Strategy-registry streams (v0.5 T526) ───────────────────────────────────
//
// Three channels — one per lifecycle event — match the pattern established by
// `stream_fills` / `stream_positions`: subscribe synchronously before yielding
// (eager-subscribe avoids the publish-before-subscribe race when events fire
// between `stream()` being called and the first `.next().await`), then loop
// on `recv`. Lagged receivers warn + continue; a closed channel flips the
// whole panel into its error state by yielding `StrategiesError` with the
// shared `CONNECTION_CHANNEL_CLOSED` copy (the widget prepends
// `STRATEGIES_ERROR_PREFIX` when rendering).
//
// Whereas the fills channel has its own `TapeError` and positions has
// `PositionsError`, all three strategy-registry channels funnel their
// `Closed` state into the single `StrategiesError` variant — the operator
// sees one panel-wide "Can't read strategies: agent disconnected" rather
// than three simultaneous red stripes saying the same thing.

/// `strategy_loaded` → `Message::StrategyLoaded`.
pub fn stream_strategy_loaded(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.strategy_loaded();
    stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::StrategyLoaded(event),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "strategy_loaded", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    yield Message::StrategiesError(SmolStr::new(
                        strings::CONNECTION_CHANNEL_CLOSED,
                    ));
                    break;
                }
            }
        }
    }
}

/// `strategy_swapped` → `Message::StrategySwapped`.
pub fn stream_strategy_swapped(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.strategy_swapped();
    stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::StrategySwapped(event),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "strategy_swapped", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    yield Message::StrategiesError(SmolStr::new(
                        strings::CONNECTION_CHANNEL_CLOSED,
                    ));
                    break;
                }
            }
        }
    }
}

/// `strategy_error` → `Message::StrategyLoadError`. The per-row error path
/// (`Reject` event) uses `StrategyLoadError`; the panel-wide closed-channel
/// path still yields `StrategiesError`.
pub fn stream_strategy_error(bus: &EventBus) -> impl Stream<Item = Message> + Send {
    let mut rx = bus.strategy_error();
    stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::StrategyLoadError(event),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "strategy_error", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    yield Message::StrategiesError(SmolStr::new(
                        strings::CONNECTION_CHANNEL_CLOSED,
                    ));
                    break;
                }
            }
        }
    }
}

// ── Conversions — bus types → UI view types ─────────────────────────────────

/// Convert an `agent::AgentMode` bus message into a cockpit `Message`. A
/// `Halted` bus event is treated as an external halt (not operator-initiated)
/// so the kill-switch banner shows "AGENT HALTED" with the reason.
#[must_use]
pub fn mode_to_message(m: &AgentBusMode) -> Message {
    match m {
        AgentBusMode::Running => Message::AgentModeChanged(AgentMode::Paper),
        AgentBusMode::Halted { reason } => {
            Message::AgentHaltedExternally(SmolStr::new(reason.as_str()))
        }
    }
}

/// Convert a full `Fill` (bus) to a `FillView` (UI).
#[must_use]
pub fn fill_to_view(fill: &Fill) -> FillView {
    FillView {
        symbol: fill.symbol.clone(),
        side: fill.side,
        price: fill.price,
        qty: fill.qty,
        fee: fill.fee,
        fee_tier: fill.fee_tier,
        venue_ts: fill.venue_ts,
        transaction_id: fill.transaction_id.clone().unwrap_or_default(),
    }
}

/// Convert a `Position` (bus) to a `PositionView` (UI). The view type
/// carries `pnl_pct` and `exposure_pct`, both of which depend on
/// account-wide equity and can't be derived from one `Position` alone —
/// leave them at zero for now and let a future P&L snapshot refresh
/// replace them. The absolute `pnl` field aggregates realized + unrealized.
#[must_use]
pub fn position_to_view(pos: &Position) -> PositionView {
    PositionView {
        symbol: pos.symbol.clone(),
        base_qty: pos.base_qty,
        cost_basis: pos.cost_basis,
        last_mark: pos.last_mark,
        // Summed for scannability; kept explicit so an empty field is never shown.
        pnl: trading_core::Money::from_decimal(
            pos.realized_pnl.amount() + pos.unrealized_pnl.amount(),
        ),
        pnl_pct: Decimal::ZERO,
        exposure_pct: Decimal::ZERO,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use agent::config::BusConfig;
    use futures::StreamExt;
    use rust_decimal_macros::dec;
    use trading_core::{
        FeeTier, Liquidity, Money, OrderId, PnlSnapshot, Price, Quantity, Side, Symbol, Timestamp,
    };

    fn make_fill() -> Fill {
        Fill {
            id: trading_core::FillId::new(),
            order_id: OrderId::new(),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            qty: Quantity::new(dec!(0.1)).unwrap(),
            price: Price::new(dec!(40_008)).unwrap(),
            fee: Money::from_decimal(dec!(1.6003)),
            fee_tier: FeeTier::Taker,
            venue_ts: Timestamp::now(),
            local_ts: Timestamp::now(),
            liquidity: Liquidity::Taker,
            transaction_id: None,
        }
    }

    fn make_position(qty: Decimal) -> Position {
        Position {
            symbol: Symbol::new("BTCUSDT"),
            base_qty: qty,
            cost_basis: Money::from_decimal(dec!(10_000)),
            last_mark: Price::new(dec!(40_050)).unwrap(),
            realized_pnl: Money::from_decimal(dec!(5)),
            unrealized_pnl: Money::from_decimal(dec!(7.5)),
        }
    }

    fn make_pnl() -> PnlSnapshot {
        PnlSnapshot {
            cash: Money::from_decimal(dec!(90_000)),
            unrealized: Money::from_decimal(dec!(250)),
            realized: Money::from_decimal(dec!(-120.50)),
            total_equity: Money::from_decimal(dec!(90_129.50)),
            daily_return: Money::from_decimal(dec!(129.50)),
            as_of: Timestamp::now(),
        }
    }

    #[tokio::test]
    async fn fills_stream_emits_fill_received() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut s = Box::pin(stream_fills(&bus));
        // Give the subscriber a tick to register.
        tokio::task::yield_now().await;
        bus.publish_fill(make_fill());
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("within 2s")
            .expect("stream yielded");
        assert!(matches!(msg, Message::FillReceived(_)), "got {msg:?}");
    }

    #[tokio::test]
    async fn pnl_stream_emits_pnl_refreshed() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut s = Box::pin(stream_pnl(&bus));
        tokio::task::yield_now().await;
        bus.publish_pnl(make_pnl());
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("within 2s")
            .expect("stream yielded");
        assert!(matches!(msg, Message::PnlRefreshed(_)), "got {msg:?}");
    }

    #[tokio::test]
    async fn positions_stream_aggregates_into_full_snapshot() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut s = Box::pin(stream_positions(&bus));
        tokio::task::yield_now().await;
        bus.publish_position(make_position(dec!(0.25)));
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("within 2s")
            .expect("stream yielded");
        match msg {
            Message::PositionsRefreshed(v) => assert_eq!(v.len(), 1),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn positions_stream_removes_flat() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut s = Box::pin(stream_positions(&bus));
        tokio::task::yield_now().await;
        bus.publish_position(make_position(dec!(0.25)));
        let _first = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("first within 2s");
        bus.publish_position(make_position(Decimal::ZERO));
        let second = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("second within 2s")
            .expect("second snapshot");
        match second {
            Message::PositionsRefreshed(v) => assert!(v.is_empty()),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn mode_stream_maps_halted_to_external_halt() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut s = Box::pin(stream_mode(&bus));
        tokio::task::yield_now().await;
        bus.publish_mode(AgentBusMode::Halted {
            reason: "halt file detected".into(),
        });
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), s.next())
            .await
            .expect("within 2s")
            .expect("stream yielded");
        assert!(
            matches!(msg, Message::AgentHaltedExternally(ref r) if r.as_str() == "halt file detected"),
            "got {msg:?}"
        );
    }

    #[test]
    fn fill_conversion_preserves_fields() {
        let f = make_fill();
        let v = fill_to_view(&f);
        assert_eq!(v.symbol, f.symbol);
        assert_eq!(v.price.get(), f.price.get());
        assert_eq!(v.qty.get(), f.qty.get());
    }

    #[test]
    fn position_conversion_sums_pnl() {
        let p = make_position(dec!(0.25));
        let v = position_to_view(&p);
        assert_eq!(v.pnl.amount(), dec!(12.5));
        assert_eq!(v.base_qty, dec!(0.25));
    }
}
