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
use futures::Stream;
use futures::stream::BoxStream;
use iced::advanced::subscription::{EventStream, Hasher, Recipe, from_recipe};
use rust_decimal::Decimal;
use smol_str::SmolStr;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};
use trading_core::{Fill, FillView, Position, PositionView, RiskTelemetry};

use agent::ActivityEvent;

use crate::state::{AgentMode, Message, RiskState};
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
        subscription_for(Channel::StrategyError, Arc::clone(&bus)),
        // T1508 — market-health watchdog channel (v1.5b contract).
        subscription_for(Channel::MarketHealth, Arc::clone(&bus)),
        // T1707 — risk-telemetry publisher (Phase 3 Q3).
        subscription_for(Channel::RiskTelemetry, bus),
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
    /// T1508 — market-health watchdog (v1.5b). Per-venue freshness events.
    MarketHealth,
    /// T1707 — risk-telemetry publisher (Phase 3 Q3). 1 Hz risk-engine snapshot.
    RiskTelemetry,
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
        // Each `stream_*` function only needs `&EventBus` to call `bus.fills()`
        // etc. — those methods return an owned `broadcast::Receiver<T>` that
        // doesn't borrow from `bus`. The receiver keeps the underlying
        // `broadcast::Sender` alive via the bus's internal Weak<->Strong
        // wiring, so dropping `bus` after subscription is safe. The returned
        // `impl Stream + 'static` only captures `rx`, never `bus`.
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
            Channel::MarketHealth => Box::pin(stream_market_health(&bus)),
            Channel::RiskTelemetry => Box::pin(stream_risk_telemetry(&bus)),
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
pub fn stream_fills(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_positions(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_pnl(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_ticks(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_bars(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_mode(bus: &EventBus) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_strategy_loaded(
    bus: &EventBus,
) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_strategy_swapped(
    bus: &EventBus,
) -> impl Stream<Item = Message> + Send + use<> + 'static {
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
pub fn stream_strategy_error(
    bus: &EventBus,
) -> impl Stream<Item = Message> + Send + use<> + 'static {
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

// ── T1508 — Market-health watchdog stream ───────────────────────────────────
//
// Subscribes to `bus.market_health()` (the v1.5b watchdog channel) and
// converts each `MarketHealth` bus event into a `Message::MarketHealthUpdated`.
//
// Closed-channel semantics differ from fills/positions/pnl: the watchdog
// channel closing is not a fatal cockpit error (no operator-facing panel
// to flip into error state). We log at `warn` level and stop the stream;
// the connection dot in the status bar will show the last known state.

/// `market_health` → `Message::MarketHealthUpdated`.
pub fn stream_market_health(
    bus: &EventBus,
) -> impl Stream<Item = Message> + Send + use<> + 'static {
    let mut rx = bus.market_health();
    stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::MarketHealthUpdated(event),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "market_health", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    warn!(channel = "market_health", "broadcast closed");
                    break;
                }
            }
        }
    }
}

// ── T1707 — Risk-telemetry publisher stream (Phase 3 Q3) ────────────────────
//
// Subscribes to `bus.risk_telemetry()` (1 Hz risk-engine snapshot) and
// converts each `RiskTelemetry` bus event into a
// `Message::RiskStateRefreshed(RiskState)` via the
// `RiskTelemetry → RiskState` reduction (identical field shape; the UI
// type is decoupled so a future `core::RiskTelemetry` rename does not
// ripple `state.rs`).
//
// Closed-channel semantics mirror `market_health`: the publisher
// closing is not a fatal cockpit error (the Risk screen falls back to
// `Loading`); we log at `warn` and stop the stream.

/// `risk_telemetry` → `Message::RiskStateRefreshed`.
pub fn stream_risk_telemetry(
    bus: &EventBus,
) -> impl Stream<Item = Message> + Send + use<> + 'static {
    let mut rx = bus.risk_telemetry();
    stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::RiskStateRefreshed(risk_telemetry_to_state(&event)),
                Err(RecvError::Lagged(n)) => {
                    warn!(channel = "risk_telemetry", skipped = n, "broadcast lagged");
                }
                Err(RecvError::Closed) => {
                    warn!(channel = "risk_telemetry", "broadcast closed");
                    break;
                }
            }
        }
    }
}

/// Convert a bus-side `RiskTelemetry` snapshot into the cockpit's
/// `RiskState` mirror. Identical field shape — the conversion is
/// straightforward field copying. The seam exists so a future
/// `core::RiskTelemetry` schema change ripples here, not into
/// `state.rs`.
#[must_use]
pub fn risk_telemetry_to_state(t: &RiskTelemetry) -> RiskState {
    RiskState {
        per_symbol_exposure: t.per_symbol_exposure.clone(),
        per_symbol_caps: t.per_symbol_caps.clone(),
        daily_loss_used_pct: t.daily_loss_used_pct,
        daily_loss_cap_pct: t.daily_loss_cap_pct,
        heartbeat_age_ms: t.heartbeat_age_ms,
        heartbeat_timeout_ms: t.heartbeat_timeout_ms,
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

// ── Phase D+ — Trail-mirror Subscription bridge (ui-rethink-phase-d-trail-followup) ─
//
// R1.1: `trail_mirror_subscription` builds an iced `Subscription` wrapping the
// `TrailMirrorHandle::tick_tx` broadcast sender.  Mirrors `BusRecipe` above.
//
// R1.2: The `From<reflection::trail_mirror::TrailMirrorTick> for TrailMirrorUiTick`
// conversion lives here (under `#[cfg(feature = "live")]`) so `state.rs` (the
// `Message` API) never sees a `reflection` import.

use crate::state::{ReconstructedTrailUi, TrailMirrorUiTick, TrailStageUi};
use crate::widgets::trail_node::{TrailNode, TrailNodeKind};
use reflection::trail_mirror::TrailMirrorHandle;

/// Build an iced `Subscription` that delivers `Message::TrailMirrorTick`
/// messages from the trail-mirror's broadcast channel.
///
/// Returns `iced::Subscription::none()` if the handle is `None` (mirror not
/// armed — e.g. `tick_bus_capacity = 0` in config).
pub fn trail_mirror_subscription(handle: TrailMirrorHandle) -> iced::Subscription<Message> {
    iced::advanced::subscription::from_recipe(TrailMirrorRecipe { handle })
}

/// iced `Recipe` wrapping the `TrailMirrorHandle::tick_tx` broadcast sender.
/// One recipe, batched alongside `BusRecipe` + `ServerTimeRecipe`.
struct TrailMirrorRecipe {
    handle: TrailMirrorHandle,
}

impl Recipe for TrailMirrorRecipe {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::any::TypeId;
        use std::hash::Hash;
        // Stable identity: one recipe per subscription batch.
        TypeId::of::<Self>().hash(state);
        // Static discriminant so iced never merges this with BusRecipe.
        0xD0_05u16.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        // Subscribe eagerly (before the stream is first polled) to avoid the
        // publish-before-subscribe race — mirrors BusRecipe's pattern.
        let rx = self.handle.tick_tx.subscribe();
        trail_mirror_stream_impl(rx)
    }
}

/// Inner stream logic for `TrailMirrorRecipe`, extracted so integration tests
/// can drive it directly without needing a running iced application or an
/// `EventStream`.
///
/// - Loops on `rx.recv()`, converting each `TrailMirrorTick` to
///   `Message::TrailMirrorTick(TrailMirrorUiTick::from(tick))`.
/// - On `Lagged(n)`: logs a warning and continues (R1.5 drop-on-lag policy).
/// - On `Closed`: logs at debug level and terminates the stream cleanly.
///
/// The caller (i.e. `Recipe::stream`) is responsible for subscribing eagerly
/// before passing `rx` here, to avoid the publish-before-subscribe race.
pub fn trail_mirror_stream_impl(
    mut rx: tokio::sync::broadcast::Receiver<reflection::trail_mirror::TrailMirrorTick>,
) -> BoxStream<'static, Message> {
    Box::pin(async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(tick) => yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(channel = "trail_mirror", skipped = n, "broadcast lagged — dropping");
                    // Continue: lag is not fatal; the cockpit re-fetches on
                    // the next visible chevron click (R1.5 drop-on-lag policy).
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Mirror task exited (agent shutdown or config disabled).
                    debug!(channel = "trail_mirror", "broadcast closed — stopping recipe");
                    break;
                }
            }
        }
    })
}

/// Crate-boundary conversion: `reflection::trail_mirror::TrailMirrorTick`
/// → `crate::state::TrailMirrorUiTick`.
///
/// Lives here (under `#[cfg(feature = "live")]`) so `state.rs` never imports
/// `reflection::*` types (Q2 (b) resolution).
impl From<reflection::trail_mirror::TrailMirrorTick> for TrailMirrorUiTick {
    fn from(tick: reflection::trail_mirror::TrailMirrorTick) -> Self {
        match tick {
            reflection::trail_mirror::TrailMirrorTick::TrailReady(boxed) => {
                let fill = trail_stage_to_ui(&boxed.fill);
                let signal = trail_stage_to_ui(&boxed.signal);
                let forecast = trail_stage_to_ui(&boxed.forecast);
                let debate = trail_stage_to_ui(&boxed.debate);
                // Pre-build TrailNode structs (upstream-first: Forecast, LlmDebate, Signal, Fill)
                // so screens::trail::view can borrow &TrailNode with the Cockpit's lifetime
                // (trail_node::view<'a>(node: &'a TrailNode) -> Element<'a>).
                let nodes = vec![
                    TrailNode {
                        kind: TrailNodeKind::Forecast,
                        timestamp: forecast.timestamp.clone(),
                        actor: forecast.actor.clone(),
                        headline: forecast.headline.clone(),
                    },
                    TrailNode {
                        kind: TrailNodeKind::LlmDebate,
                        timestamp: debate.timestamp.clone(),
                        actor: debate.actor.clone(),
                        headline: debate.headline.clone(),
                    },
                    TrailNode {
                        kind: TrailNodeKind::Signal,
                        timestamp: signal.timestamp.clone(),
                        actor: signal.actor.clone(),
                        headline: signal.headline.clone(),
                    },
                    TrailNode {
                        kind: TrailNodeKind::Fill,
                        timestamp: fill.timestamp.clone(),
                        actor: fill.actor.clone(),
                        headline: fill.headline.clone(),
                    },
                ];
                TrailMirrorUiTick::TrailReady(Box::new(ReconstructedTrailUi {
                    audit_id: SmolStr::new(&boxed.audit_id),
                    fill,
                    signal,
                    forecast,
                    debate,
                    nodes,
                }))
            }
            reflection::trail_mirror::TrailMirrorTick::TrailUpdated(s) => {
                TrailMirrorUiTick::TrailUpdated(SmolStr::new(s))
            }
        }
    }
}

fn trail_stage_to_ui(stage: &reflection::trail_mirror::TrailStage) -> TrailStageUi {
    // TrailStageUi fields are Option<String> (not SmolStr) so trail.rs can
    // borrow them from model with the Cockpit's lifetime (avoids E0515).
    TrailStageUi {
        timestamp: stage.timestamp.clone(),
        actor: stage.actor.clone(),
        headline: stage.headline.clone(),
        raw_payload: stage.raw_payload.clone(),
    }
}

// ── cockpit-activity-status-bar v0.1.0 — ActivityRecipe (T-D-N5) ────────────
//
// Subscribes to `bus.activity()` (the broadcast channel added to `EventBus`
// in Wave A) and emits one `Message::ActivityEventReceived(event)` per
// `ActivityEvent`. Lag is handled the same way as every other BusRecipe
// channel: warn + continue; Closed ends the stream cleanly.
//
// `stream_impl` is extracted (mirror of `lab/progress.rs`) so integration
// tests can drive it directly without needing a running iced application.

/// iced `Recipe` that subscribes to the activity broadcast channel and
/// emits `Message::ActivityEventReceived` messages into the cockpit loop.
///
/// Wired into the `Subscription::batch` call in `cockpit_live::subscription()`.
pub struct ActivityRecipe {
    pub bus: Arc<EventBus>,
}

impl Recipe for ActivityRecipe {
    type Output = Message;

    fn hash(&self, state: &mut Hasher) {
        use std::any::TypeId;
        use std::hash::Hash;
        TypeId::of::<Self>().hash(state);
        // Static discriminant so iced never merges this with BusRecipe.
        0xAC71_u16.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        // Subscribe eagerly (before the stream is first polled) to avoid
        // the publish-before-subscribe race — mirrors BusRecipe pattern.
        let rx = self.bus.activity().subscribe();
        activity_stream_impl(rx)
    }
}

/// Inner stream logic for `ActivityRecipe`, extracted so integration tests
/// can drive it directly without needing a running iced application.
///
/// - Loops on `rx.recv()`, yielding `Message::ActivityEventReceived(event)`.
/// - On `Lagged(n)`: logs a warning and continues (display-only; no replay).
/// - On `Closed`: logs at debug level and terminates the stream cleanly.
pub fn activity_stream_impl(
    mut rx: tokio::sync::broadcast::Receiver<ActivityEvent>,
) -> BoxStream<'static, Message> {
    Box::pin(async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => yield Message::ActivityEventReceived(event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        target: "ui.activity",
                        lagged = n,
                        "ActivityRecipe missed events — display may be briefly stale"
                    );
                    // Continue: lag is not fatal. The tape may show stale
                    // state until the next event lands (R6.3 contract).
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    debug!(target: "ui.activity", "activity broadcast closed — stopping recipe");
                    break;
                }
            }
        }
    })
}

// ── subscription-pipe-server-time-template — ServerTimeRecipe helper ─────────
//
// Extracted so integration tests can drive the stream directly without
// constructing a running iced application or an `EventStream`.
//
// See `crates/ui/tests/server_time_recipe_stream.rs` for the test suite.
//
// ## K8 mitigation (same as `LabProgressRecipe` / `TrailMirrorRecipe`)
//
// `tokio::time::interval()` must be created inside the tokio runtime
// context. The `rt_handle.enter()` guard is scoped to a `{ ... }` block
// that ends **before** `Box::pin(...)`, so the `EnterGuard` (which is
// `!Send`) does not leak into the returned `BoxStream<'static, Message>`
// (which must be `Send + 'static` on native per iced_futures contract).

/// Inner stream logic for `ServerTimeRecipe`, extracted so integration tests
/// can drive it directly without needing a running iced application or an
/// `EventStream`.
///
/// Emits `Message::ServerTimeTick(Timestamp::now())` once per second.
/// The first (immediate) tick from `tokio::time::interval` is skipped so
/// the clock shows the time ~1 s after subscription, not immediately at boot.
///
/// The stream never terminates — `ServerTimeRecipe` is a process-lifetime
/// always-on recipe (K2 contract). The only way to stop it is to drop the
/// `BoxStream` (i.e. iced removes the subscription).
///
/// ## Runtime context
///
/// `rt_handle` must be the agent-runtime `Handle` (the multi-thread tokio
/// runtime spawned in `cockpit_live::main`). On iced's `futures::ThreadPool`
/// executor there is no tokio reactor context; this helper enters it via
/// `rt_handle.enter()` so `tokio::time::interval` finds the time driver.
/// The guard is dropped before `Box::pin` per K8.
#[must_use]
pub fn server_time_stream_impl(rt_handle: &tokio::runtime::Handle) -> BoxStream<'static, Message> {
    use std::time::Duration;
    use trading_core::Timestamp;

    let mut interval = {
        let _guard = rt_handle.enter();
        tokio::time::interval(Duration::from_secs(1))
    };
    Box::pin(async_stream::stream! {
        // Skip the first (immediate) tick so the first ServerTimeTick
        // arrives ~1 s after subscription, not immediately at boot.
        interval.tick().await;
        loop {
            interval.tick().await;
            yield Message::ServerTimeTick(Timestamp::now());
        }
    })
}

// ── subscription-pipe-toast-dismiss-template — ToastDismissRecipe helper ─────
//
// Extracted so integration tests can drive the stream directly without
// constructing a running iced application or an `EventStream`.
//
// Mirrors `server_time_stream_impl` (above) pattern: tokio interval inside
// `rt_handle.enter()` guard, guard dropped before `Box::pin(...)` so the
// `EnterGuard` (`!Send`) does not leak into the `BoxStream<'static, Message>`.
//
// See `ServerTimeRecipe` / `server_time_stream_impl` for the full K8 rationale.

/// Inner stream logic for `ToastDismissRecipe`, extracted for test reachability.
///
/// Emits `Message::ToastTick(Instant::now())` every 500 ms.
/// The first tick from `tokio::time::interval` is skipped so the first
/// `ToastTick` arrives ~500 ms after subscription.
///
/// The stream never terminates — `ToastDismissRecipe` is an always-on
/// process-lifetime recipe (same as `ServerTimeRecipe`).
///
/// ## Runtime context
///
/// `rt_handle` must be the agent-runtime `Handle`. Entered via `rt_handle.enter()`
/// before `tokio::time::interval` is called; guard dropped before `Box::pin`.
#[must_use]
pub fn toast_dismiss_stream_impl(
    rt_handle: &tokio::runtime::Handle,
) -> BoxStream<'static, Message> {
    use std::time::{Duration, Instant};

    let mut interval = {
        let _guard = rt_handle.enter();
        tokio::time::interval(Duration::from_millis(500))
    };
    Box::pin(async_stream::stream! {
        // Skip the first (immediate) tick so the first ToastTick arrives
        // ~500 ms after subscription, not immediately at boot.
        interval.tick().await;
        loop {
            interval.tick().await;
            yield Message::ToastTick(Instant::now());
        }
    })
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

    // ── T-D-N5 — ActivityRecipe stream tests ─────────────────────────────────

    fn make_activity_event(id: u64) -> ActivityEvent {
        ActivityEvent {
            id: agent::ActivityId(id),
            kind: agent::ActivityKind::YahooPreload,
            label: format!("test {id}"),
            phase: agent::ActivityPhase::Start { total_units: None },
            ts_ms: 0,
        }
    }

    /// T-D-N5 test 1 — `activity_stream_impl` emits `ActivityEventReceived`
    /// messages in order for 3 events pushed through a fake broadcast.
    #[tokio::test]
    async fn activity_recipe_emits_messages() {
        use tokio::sync::broadcast;

        let (tx, rx) = broadcast::channel::<ActivityEvent>(64);
        let mut stream = activity_stream_impl(rx);

        // Push 3 events.
        for i in 1..=3u64 {
            tx.send(make_activity_event(i)).expect("send ok");
        }

        // Collect 3 messages.
        let mut received = Vec::new();
        for _ in 0..3 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
                .await
                .expect("within 2s")
                .expect("stream yielded");
            received.push(msg);
        }

        assert_eq!(received.len(), 3, "expected 3 messages");
        for (i, msg) in received.iter().enumerate() {
            let id = i as u64 + 1;
            assert!(
                matches!(msg, Message::ActivityEventReceived(e) if e.id == agent::ActivityId(id)),
                "message {i} wrong: {msg:?}"
            );
        }
    }

    /// T-D-N5 test 2 — `activity_stream_impl` swallows `Lagged` errors and
    /// continues (does not panic; stream remains open).
    #[tokio::test]
    async fn activity_recipe_handles_lag() {
        use tokio::sync::broadcast;

        // Small channel (capacity 4) so we can force a lag easily.
        let (tx, rx) = broadcast::channel::<ActivityEvent>(4);

        // Fill the channel completely before the consumer reads — this
        // forces the next send to overflow and the consumer to get Lagged.
        for i in 0..4u64 {
            tx.send(make_activity_event(i)).expect("fill send ok");
        }
        // One more send causes the oldest to be evicted — consumer will see Lagged.
        tx.send(make_activity_event(99)).expect("overflow send ok");

        let mut stream = activity_stream_impl(rx);

        // The stream must NOT panic; it must swallow Lagged and yield the
        // surviving messages. We just verify it produces at least one message
        // after the lagged gap without hanging.
        let msg = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .expect("within 2s")
            .expect("stream yielded at least one message after lag");

        assert!(
            matches!(msg, Message::ActivityEventReceived(_)),
            "expected ActivityEventReceived after lag, got {msg:?}"
        );
    }
}
