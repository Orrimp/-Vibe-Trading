---
generated: 2026-04-18
author: developer
feature: v0-paper-sma
scope: broadcast-bus API for ui-designer
---

# Week 2 Backend — Broadcast Bus API

This document describes the `agent::EventBus` API that the `ui` crate
(cockpit) will subscribe to once T32 lands.

---

## Crate

`crates/agent/src/bus.rs` — exported as `agent::EventBus`.

## Construction

```rust
let bus = agent::EventBus::new(&cfg.bus);
```

`cfg.bus` is a `BusConfig` from `config/agent.toml` (channel capacity, etc.).

In v0, all channels are same-process (in-memory `tokio::sync::broadcast`).
No IPC layer is needed until v0.5.

---

## Channels

All channels are `tokio::sync::broadcast`.  Capacity is configurable;
default 1024 (fills), 128 (bars/ticks/pnl/mode), 32 (positions).

| Channel | Type | Description |
|---------|------|-------------|
| `fills` | `trading_core::Fill` | Every paper fill, real-time |
| `positions` | `trading_core::Position` | Snapshot after each fill |
| `bars` | `trading_core::Bar` | 1-minute bar close events |
| `ticks` | `trading_core::Tick` | Individual trade ticks |
| `pnl` | `trading_core::PnlSnapshot` | Equity / P&L snapshot per bar |
| `mode` | `agent::AgentMode` | `Running` / `Halted { reason }` |

---

## Publisher API (agent-side)

```rust
// Publish a fill to all UI subscribers
bus.publish_fill(fill);

// Publish a position update
bus.publish_position(position);

// Publish a bar close event
bus.publish_bar(bar);

// Publish a tick
bus.publish_tick(tick);

// Publish a P&L snapshot
bus.publish_pnl(snapshot);

// Publish a mode change (Running → Halted)
bus.publish_mode(AgentMode::Halted { reason: "halt file detected".into() });
```

All methods are infallible (lagged receivers are silently dropped).

---

## Subscriber API (ui-side, T32)

```rust
// Subscribe to fills
let mut fill_rx = bus.subscribe_fills();
// Receive:
while let Ok(fill) = fill_rx.recv().await { /* update live tape */ }

// Subscribe to mode changes
let mut mode_rx = bus.subscribe_mode();
while let Ok(mode) = mode_rx.recv().await {
    if let AgentMode::Halted { reason } = mode {
        // show red halted banner
    }
}
```

All `subscribe_*` methods return a `tokio::sync::broadcast::Receiver<T>`.
Lag handling: if the UI receiver falls behind by more than capacity events,
the next `recv()` returns `Err(RecvError::Lagged(n))` — handle by logging
and continuing.

---

## AgentMode enum

```rust
pub enum AgentMode {
    Running,
    Halted { reason: String },
}
```

Sent on the `mode` channel when the kill switch trips.

---

## Integration notes for ui-designer

1. **Feature flag**: Gate the real subscription behind `#[cfg(feature = "live")]`
   and keep `ui::fixtures` as the default.  The bus channel types are all from
   `trading_core` which `ui` already depends on.

2. **Cockpit `Subscription`**: Use `iced::subscription::unfold` wrapping the
   broadcast receiver.  On `Lagged` error, log and re-subscribe.

3. **Kill-switch button**: Subscribe to `mode` channel; on
   `AgentMode::Halted` update the `Model` field `halted: true` and render the
   red banner.  The button itself should call
   `agent::write_halt_file(&cfg.kill_switch.halt_file)` which the watcher will
   detect within 500 ms.

4. **P&L card**: `PnlSnapshot` carries `cash`, `unrealized`, `realized`,
   `total_equity`, `daily_return_pct`.  Available after T31 wires the
   reconciler.  In v0 snapshot is computed per-bar by the backtest loop;
   in the live agent it will be emitted by the reconciler task.

---

## Deviations from spec (v0)

- IPC model: same-process broadcast; no Unix socket / gRPC in v0.
- `pnl` channel: not yet emitted in the backtest binary (T25); will be added
  in the live agent (T31).  UI should handle zero/missing gracefully.
- `ticks` channel: fed by `BinanceFeed` in paper mode only; empty in
  research/replay mode.

---

*This document is the handoff contract between developer (T31) and
ui-designer (T32).*
