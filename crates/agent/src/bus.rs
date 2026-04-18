//! Broadcast event bus — T31.
//!
//! The `EventBus` holds one `broadcast::Sender<T>` per domain event stream.
//! Both the `agent` binary (producer) and the `cockpit` binary (consumer) access
//! these via `agent::bus::EventBus`.
//!
//! ## Broadcast API (for ui-designer / T32)
//!
//! | Channel          | Type                     | Capacity | Notes                      |
//! |------------------|--------------------------|----------|----------------------------|
//! | `fills`          | `trading_core::Fill`     | 1024     | Every paper/live fill      |
//! | `positions`      | `trading_core::Position` | 256      | Updated on every fill      |
//! | `bars`           | `trading_core::Bar`      | 1024     | Published at bar close     |
//! | `ticks`          | `trading_core::Tick`     | 8192     | Raw trades                 |
//! | `pnl`            | `trading_core::PnlSnapshot`| 256    | After each bar close       |
//! | `agent_mode`     | `agent::AgentMode`       | 32       | Running / Halted           |
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
use trading_core::{Bar, Fill, PnlSnapshot, Position, Tick};

use crate::config::BusConfig;
use crate::kill_switch::AgentMode;

/// The agent's event bus.
///
/// Clone-able — all clones share the same underlying channels.
#[derive(Clone)]
pub struct EventBus {
    fills_tx: broadcast::Sender<Fill>,
    positions_tx: broadcast::Sender<Position>,
    bars_tx: broadcast::Sender<Bar>,
    ticks_tx: broadcast::Sender<Tick>,
    pnl_tx: broadcast::Sender<PnlSnapshot>,
    mode_tx: broadcast::Sender<AgentMode>,
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
        Self {
            fills_tx,
            positions_tx,
            bars_tx,
            ticks_tx,
            pnl_tx,
            mode_tx,
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
}
