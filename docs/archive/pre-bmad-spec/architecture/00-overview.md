---
slug: architecture-00-overview
status: shipped
owner: architect
updated: 2026-05-13
---

# Architecture overview — workspace layout, naming, runtime

Cross-cutting structural rules. Where each crate lives, what each one is
named, and the runtime model that ties them together.

## Workspace layout

```
trading/
├── Cargo.toml            # virtual workspace
├── crates/
│   ├── core/             # package name: trading_core — domain types:
│   │                     # Symbol, Order, Position, Signal (see ADR-0001
│   │                     # for why the package is not named `core`)
│   ├── data/             # market-data ingestion, storage, replay
│   ├── features/         # feature engineering, indicator library
│   ├── models/           # DL/ML models (candle/burn/tract) + training
│   ├── llm/              # LLM clients, prompt caching, tool schemas
│   ├── risk/             # risk engine, position sizing, kill switches
│   ├── strategy/         # strategy runtime, signal combination
│   ├── exec/             # exchange clients, order routing, paper-trade
│   ├── backtest/         # backtest engine (bin target: backtest)
│   ├── audit/            # double-entry ledger of decisions/orders/fills/PnL
│   ├── cost/             # cost telemetry — LLM tokens, infra, data feeds (Q4)
│   ├── reports/          # v1+ operator success reports — read-only over audit
│   │                     # (lib + bin: report). Cron + on-kill-switch friendly.
│   ├── ui/               # iced desktop app — ops cockpit + backtest viewer
│   │                     # (bin targets: cockpit, cockpit_live, viewer)
│   └── agent/            # top-level orchestrator (bin target: trading;
│                         # lib also hosts agent::runtime::run shared by
│                         # cockpit_live — see live-cockpit-unified)
```

## Naming conventions

The foundation crate's package name is `trading_core`, not `core`. This is
load-bearing across the workspace: every consumer's `[dependencies]` reads
`trading_core = { path = "../core" }`, and every import reads
`use trading_core::{Symbol, Order, …};`. The directory name `crates/core/`
is unrelated to the package name and stays.

See [ADR-0001](../../_bmad-output/planning-artifacts/architecture/decisions/0001-crate-name-stdlib-collision.md) for the full
context, alternatives considered, and consequences. The short version:
naming any workspace member after a Rust stdlib crate (`core`, `alloc`,
`std`, `test`, `proc_macro`) causes silent failures in `cargo test
--workspace --doc` that `doctest = false` cannot mask.

## Runtime

- `tokio` multi-thread executor.
- Actor-ish layout: each crate exposes an owned task with typed message
  channels (`tokio::sync::mpsc`). No shared mutable state across crates.
- The `agent` crate orchestrates: it constructs the shared `Arc<EventBus>`
  / `Arc<KillSwitch>` once and threads them into each crate's task. See
  [01-data-flow.md § Public API surface — bin-shared agent runtime](01-data-flow.md#public-api-surface--bin-shared-agent-runtime-live-cockpit-unified)
  for the cross-bin runtime contract.
- The `cockpit_live` bin additionally hosts the tokio runtime on a side
  `std::thread::spawn` so iced can run on the main thread. The same
  `Arc<EventBus>` and `Arc<KillSwitch>` are shared across the two
  contexts.

## Changelog
- 2026-05-13 (architect): content migrated from
  `spec/architecture.md` lines 172–250 during Phase 1A Session 2. The
  long stdlib-collision subsection was extracted to ADR-0001 in
  Session 1 and replaced here with a one-paragraph pointer.
