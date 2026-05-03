//! Broadcast event bus — T31, extended for v0.5 (T512).
//!
//! The `EventBus` holds one `broadcast::Sender<T>` per domain event stream.
//! Both the `agent` binary (producer) and the `cockpit` binary (consumer) access
//! these via `agent::bus::EventBus`.
//!
//! ## Broadcast API (for ui-designer / T32, T512)
//!
//! | Channel              | Type                           | Capacity | Notes                         |
//! |----------------------|--------------------------------|----------|-------------------------------|
//! | `fills`              | `trading_core::Fill`           | 1024     | Every paper/live fill         |
//! | `positions`          | `trading_core::Position`       | 256      | Updated on every fill         |
//! | `bars`               | `trading_core::Bar`            | 1024     | Published at bar close        |
//! | `ticks`              | `trading_core::Tick`           | 8192     | Raw trades                    |
//! | `pnl`                | `trading_core::PnlSnapshot`    | 256      | After each bar close          |
//! | `agent_mode`         | `agent::AgentMode`             | 32       | Running / Halted              |
//! | `strategy_loaded`    | `trading_core::StrategyLoaded` | 32       | Config loaded / swapped in    |
//! | `strategy_swapped`   | `trading_core::StrategySwapped`| 32       | Hot-swap of existing strategy |
//! | `strategy_error`     | `trading_core::StrategyLoadError`| 32     | Parse / typecheck rejection   |
//!
//! ### v0.5 extension — strategy lifecycle channels
//!
//! Three new channels carry `StrategyLoaded`, `StrategySwapped`, and
//! `StrategyLoadError` events emitted by the file watcher (T513) and the
//! initial load path (T514).  Subscribers receive events within one Tokio
//! task-yield of the causal `strategy_event` audit write.
//!
//! **Backpressure**: same policy as v0 channels — bounded ring-buffer of 32
//! items; slow consumers get `RecvError::Lagged(n)` and skip events; the
//! publisher never blocks.  `RecvError::Closed` indicates no remaining
//! subscribers (benign — strategy events are also persisted in the ledger).
//!
//! ### How to subscribe from the cockpit
//!
//! ```rust,ignore
//! // Both `agent` and `cockpit` depend on the `agent` crate as a library.
//! // Cockpit receives a reference to the running agent's bus at startup.
//!
//! let bus = agent::bus::EventBus::new(&config.bus);
//! let mut fill_rx = bus.fills();
//! while let Ok(fill) = fill_rx.recv().await {
//!     // render fill in tape panel
//! }
//! ```
//!
//! ### IPC model
//!
//! v0: **same process**. Both `agent` (as library) and `cockpit` run in the same
//! process. The cockpit imports `agent` as a Rust crate dependency and shares the
//! `EventBus` via `Arc`. No domain sockets or TCP needed.
//!
//! ### Backpressure
//!
//! Each channel is bounded. If the cockpit lags and the channel fills, the sender
//! gets a `SendError::Closed` (no receivers) — this is silently ignored. A slow
//! consumer gets `RecvError::Lagged(n)` and skips `n` messages. The cockpit
//! handles lag by showing a "replay lag" indicator, not by blocking the agent.

use tokio::sync::broadcast;
use trading_core::{
    Bar, Fill, FundingObs, PnlSnapshot, Position, StrategyLoadError, StrategyLoaded,
    StrategySwapped, Tick,
};

use crate::config::BusConfig;
use crate::kill_switch::AgentMode;

/// The agent's event bus.
///
/// Clone-able — all clones share the same underlying channels.
///
/// v1 adds `funding_obs` channel (capacity 32, Q2 — observation-only).
#[derive(Clone)]
pub struct EventBus {
    fills_tx: broadcast::Sender<Fill>,
    positions_tx: broadcast::Sender<Position>,
    bars_tx: broadcast::Sender<Bar>,
    ticks_tx: broadcast::Sender<Tick>,
    pnl_tx: broadcast::Sender<PnlSnapshot>,
    mode_tx: broadcast::Sender<AgentMode>,
    strategy_loaded_tx: broadcast::Sender<StrategyLoaded>,
    strategy_swapped_tx: broadcast::Sender<StrategySwapped>,
    strategy_error_tx: broadcast::Sender<StrategyLoadError>,
    /// v1 Q2 — funding-rate observations (observation-only; strategy does not subscribe).
    funding_obs_tx: broadcast::Sender<FundingObs>,
}

impl EventBus {
    /// Create a new event bus from the `[bus]` config section.
    #[must_use]
    pub fn new(cfg: &BusConfig) -> Self {
        let (fills_tx, _) = broadcast::channel(cfg.fills_capacity);
        let (positions_tx, _) = broadcast::channel(256);
        let (bars_tx, _) = broadcast::channel(cfg.bars_capacity);
        let (ticks_tx, _) = broadcast::channel(cfg.ticks_capacity);
        let (pnl_tx, _) = broadcast::channel(256);
        let (mode_tx, _) = broadcast::channel(32);
        let (strategy_loaded_tx, _) = broadcast::channel(32);
        let (strategy_swapped_tx, _) = broadcast::channel(32);
        let (strategy_error_tx, _) = broadcast::channel(32);
        let (funding_obs_tx, _) = broadcast::channel(32); // v1 Q2
        Self {
            fills_tx,
            positions_tx,
            bars_tx,
            ticks_tx,
            pnl_tx,
            mode_tx,
            strategy_loaded_tx,
            strategy_swapped_tx,
            strategy_error_tx,
            funding_obs_tx,
        }
    }

    // ── Producers ─────────────────────────────────────────────────────────────

    /// Publish a fill event.  No-op if there are no subscribers.
    pub fn publish_fill(&self, fill: Fill) {
        let _ = self.fills_tx.send(fill);
    }

    /// Publish a position update.
    pub fn publish_position(&self, pos: Position) {
        let _ = self.positions_tx.send(pos);
    }

    /// Publish a bar event.
    pub fn publish_bar(&self, bar: Bar) {
        let _ = self.bars_tx.send(bar);
    }

    /// Publish a tick event.
    pub fn publish_tick(&self, tick: Tick) {
        let _ = self.ticks_tx.send(tick);
    }

    /// Publish a P&L snapshot.
    pub fn publish_pnl(&self, snap: PnlSnapshot) {
        let _ = self.pnl_tx.send(snap);
    }

    /// Publish an agent mode change.
    pub fn publish_mode(&self, mode: AgentMode) {
        let _ = self.mode_tx.send(mode);
    }

    /// Publish a `StrategyLoaded` event (file watcher: initial load or reload).
    pub fn publish_strategy_loaded(&self, event: StrategyLoaded) {
        let _ = self.strategy_loaded_tx.send(event);
    }

    /// Publish a `StrategySwapped` event (file watcher: hot-swap of an existing strategy).
    pub fn publish_strategy_swapped(&self, event: StrategySwapped) {
        let _ = self.strategy_swapped_tx.send(event);
    }

    /// Publish a `StrategyLoadError` event (parse / typecheck rejection — old strategy kept).
    pub fn publish_strategy_error(&self, event: StrategyLoadError) {
        let _ = self.strategy_error_tx.send(event);
    }

    /// Publish a `FundingObs` event (v1 Q2 — observation-only, no strategy consumes this).
    pub fn publish_funding_obs(&self, obs: FundingObs) {
        let _ = self.funding_obs_tx.send(obs);
    }

    /// Return a clone of the raw `funding_obs` sender.
    ///
    /// Used by `agent::main` to hand the sender directly to `FundingPoller::run`,
    /// which calls `tx.send()` itself (avoiding the `publish_funding_obs` wrapper
    /// so the poller does not need to clone the whole `EventBus`).
    #[must_use]
    pub fn funding_obs_sender(&self) -> broadcast::Sender<FundingObs> {
        self.funding_obs_tx.clone()
    }

    // ── Consumers (subscribe) ────────────────────────────────────────────────

    /// Subscribe to fill events.
    #[must_use]
    pub fn fills(&self) -> broadcast::Receiver<Fill> {
        self.fills_tx.subscribe()
    }

    /// Subscribe to position updates.
    #[must_use]
    pub fn positions(&self) -> broadcast::Receiver<Position> {
        self.positions_tx.subscribe()
    }

    /// Subscribe to bar events.
    #[must_use]
    pub fn bars(&self) -> broadcast::Receiver<Bar> {
        self.bars_tx.subscribe()
    }

    /// Subscribe to tick events.
    #[must_use]
    pub fn ticks(&self) -> broadcast::Receiver<Tick> {
        self.ticks_tx.subscribe()
    }

    /// Subscribe to P&L snapshots.
    #[must_use]
    pub fn pnl(&self) -> broadcast::Receiver<PnlSnapshot> {
        self.pnl_tx.subscribe()
    }

    /// Subscribe to agent mode changes.
    #[must_use]
    pub fn mode(&self) -> broadcast::Receiver<AgentMode> {
        self.mode_tx.subscribe()
    }

    /// Subscribe to `StrategyLoaded` events.
    #[must_use]
    pub fn strategy_loaded(&self) -> broadcast::Receiver<StrategyLoaded> {
        self.strategy_loaded_tx.subscribe()
    }

    /// Subscribe to `StrategySwapped` events.
    #[must_use]
    pub fn strategy_swapped(&self) -> broadcast::Receiver<StrategySwapped> {
        self.strategy_swapped_tx.subscribe()
    }

    /// Subscribe to `StrategyLoadError` events.
    #[must_use]
    pub fn strategy_error(&self) -> broadcast::Receiver<StrategyLoadError> {
        self.strategy_error_tx.subscribe()
    }

    /// Subscribe to `FundingObs` events (v1 Q2 — observation-only).
    #[must_use]
    pub fn funding_obs(&self) -> broadcast::Receiver<FundingObs> {
        self.funding_obs_tx.subscribe()
    }
}

// ── T903a-glue — `FillPublisher` impl on the bus ─────────────────────────────
//
// Closes the `live-cockpit-unified` Wave 2 bus-wiring loop: Dev A landed
// `exec::FillPublisher` + `exec::PaperEnginePublisher` inside `crates/exec/`
// (no agent-side dependency, to keep the dep graph acyclic).  This impl
// is the single agent-side glue the design [Q-resolution table row Q6 +
// Bus producer wiring → `fills` / `positions` rows] calls for: it lets a
// `Arc<EventBus>` be handed in as `Arc<dyn FillPublisher>` to
// `PaperEnginePublisher::with_publisher(...)` so every fill the live paper
// engine produces fans out on `bus.fills_tx` + `bus.positions_tx`.
//
// Body just delegates to the existing `publish_fill` / `publish_position`
// methods, so the broadcast policy (no-op when no subscribers; lag handling
// on the receiver side) is shared with all other publish paths.  The
// callers pass `&Fill` / `&Position` (per the trait signature); we clone
// once at the publish boundary because `broadcast::Sender::send` needs an
// owned value — same allocation cost the trait already accepts on the
// `RecordingPublisher` test helper in `crates/exec/src/paper.rs`.
impl exec::FillPublisher for EventBus {
    fn publish_fill(&self, fill: &trading_core::Fill) {
        // Delegate via the existing publisher; clone matches the
        // broadcast-channel ownership requirement.  `let _ = ...` swallows
        // the `SendError::Closed` case (no subscribers attached) — same
        // policy as `EventBus::publish_fill`.
        EventBus::publish_fill(self, fill.clone());
    }

    fn publish_position(&self, pos: &trading_core::Position) {
        EventBus::publish_position(self, pos.clone());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use exec::{FillPublisher, PaperEnginePublisher};
    use rust_decimal_macros::dec;
    use trading_core::{
        FeeTier, Fill, FillId, Liquidity, Money, OrderId, Position, Price, Quantity, Side, Symbol,
        Timestamp,
    };

    use super::*;

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

    /// T903a-glue acceptance — `EventBus` implements `FillPublisher`,
    /// and a publish on the trait surface fans out on the same
    /// `fills_tx` / `positions_tx` channels that `publish_fill` /
    /// `publish_position` use.  The bus wired directly via
    /// `Arc<dyn FillPublisher>` is the cheapest correct way to hand the
    /// bus to the live `PaperEnginePublisher`.
    #[tokio::test(flavor = "current_thread")]
    async fn t903a_glue_event_bus_impls_fill_publisher() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut fills_rx = bus.fills();
        let mut pos_rx = bus.positions();

        // Use the trait surface directly.  Coerce via `as` because
        // `Arc::clone` infers the source type and won't unsize on its own.
        let publisher: Arc<dyn FillPublisher> = bus.clone() as Arc<dyn FillPublisher>;
        let fill = sample_fill();
        let pos = Position::empty(Symbol::new("BTCUSDT"));
        publisher.publish_fill(&fill);
        publisher.publish_position(&pos);

        let got_fill = tokio::time::timeout(std::time::Duration::from_millis(500), fills_rx.recv())
            .await
            .expect("fill recv timed out")
            .expect("fill channel closed");
        assert_eq!(got_fill.id, fill.id);

        let got_pos = tokio::time::timeout(std::time::Duration::from_millis(500), pos_rx.recv())
            .await
            .expect("pos recv timed out")
            .expect("pos channel closed");
        assert_eq!(got_pos.symbol, pos.symbol);
    }

    /// T903a-glue end-to-end — wire `PaperEnginePublisher` against the
    /// `EventBus` (the way `agent::runtime::run` does at task-spawn
    /// time) and assert that calling `on_fill` fans out one `Fill` and
    /// one `Position` event on the bus.
    #[tokio::test(flavor = "current_thread")]
    async fn t903a_glue_paper_engine_publisher_routes_to_bus() {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let mut fills_rx = bus.fills();
        let mut pos_rx = bus.positions();

        let publisher: Arc<dyn FillPublisher> = bus.clone() as Arc<dyn FillPublisher>;
        let engine = PaperEnginePublisher::with_publisher(publisher);

        engine.on_fill(&sample_fill(), &Position::empty(Symbol::new("BTCUSDT")));

        // Both channels must observe exactly one event.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), fills_rx.recv())
            .await
            .expect("fill recv timed out")
            .expect("fill channel closed");
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), pos_rx.recv())
            .await
            .expect("pos recv timed out")
            .expect("pos channel closed");
    }
}
