---
slug: ui-v05-blockers
status: resolved
owner: ui-designer
updated: 2026-04-19
run_id: ui-v05-partial-2026-04-19
commit: HEAD (local — ui-designer sub-agent partial run, resumed + closed)
verdict: T512 landed; T526 landed in resume run. T_FINAL_B still deferred on developer T_FINAL_A (four v0.5 backtest reports).
---

# UI v0.5 — partial completion + blocker report

ui-designer sub-agent landed the independent slice of v0.5 cockpit work and
is blocked on developer task **T512** before T526 and T_FINAL_B can
progress. This report is the synchronization point — the orchestrator
re-spawns ui-designer once T512 is merged.

## Scope reminder

v0.5 ui-designer scope (from
[spec/v05-composed-strategies/tasks.md](../tasks.md)):

| Task | Status |
|---|---|
| T522 — `ui::strings` additions | **landed** |
| T523 — `state.rs` extensions | **landed** |
| T524 — `ui::widgets::strategies` panel | **landed** |
| T525 — `ui::fixtures` additions | **landed** |
| T526 — `ui::live` three new subscribers | **blocked on T512** |
| T527 — cockpit layout update (Q4) | **landed** |
| T528 — screenshots README update | **landed** |
| T_FINAL_B — UI smoke extension | **blocked on T526 + T_FINAL_A** |

## Developer gates

| Gate task | Produces | Why ui-designer needs it |
|---|---|---|
| **T501** — `trading_core` new message types + read-side views | `StrategyLoaded`, `StrategySwapped`, `StrategyLoadError`, `StrategyEventView`, `StrategyEventKind` | Type-build for the `Message` enum variants + the `StrategyRow::last_event` field. **Confirmed landed** 2026-04-19. |
| **T512** — `agent::EventBus` three new broadcast channels + publishers | `bus.strategy_loaded_rx() / strategy_swapped_rx() / strategy_error_rx()` + `publish_strategy_loaded(..)` etc. + a v0.5 extension section appended to `spec/v0-paper-sma/reports/dev-week2-broadcast-api-2026-04-18.md`. | Without these the `ui::live` subscribers in T526 cannot compile. |

Confirmation scan (2026-04-19, at ui-designer spawn time):

- T501 — present. `crates/core/src/strategy_events.rs` defines all five
  types, round-trip tests green; `crates/core/src/lib.rs` re-exports them
  at the crate root.
- T512 — **absent**. `crates/agent/src/bus.rs` still carries the v0 six
  channels only (`fills / positions / bars / ticks / pnl / mode`). No
  `strategy_*` channel, no `publish_strategy_*` method, no v0.5
  extension section in the dev-week2-broadcast-api report.

## What landed in this run

### T522 — ui::strings additions

- Keys landed: `PANEL_STRATEGIES_TITLE`, `STRATEGIES_LOADING`,
  `STRATEGIES_EMPTY`, `STRATEGIES_ERROR_PREFIX`, 6× `STRATEGIES_COL_*`,
  3× `STRATEGIES_STATUS_*`, 4× `STRATEGIES_EVENT_*`,
  `STRATEGIES_POSITION_HELD`, `STRATEGIES_POSITION_FLAT`. All routed
  through `ui::strings::all()` so the dedup + non-empty tests still
  pass.
- Consistency contract enforced — no inline literal in the widget.

### T523 — state.rs extensions

- `StrategyRow`, `StrategyStatus`, `SignalWindow` (per-strategy 60s
  counter), `STRATEGIES_RECENT_EVENT_CAP`, `STRATEGIES_SIGNAL_WINDOW_SECS`.
- `Cockpit` fields: `strategies: PanelState<Vec<StrategyRow>>`,
  `strategies_signal_counters: HashMap<StrategyId, SignalWindow>`,
  `strategies_recent_events: VecDeque<StrategyEventView>`.
- `Message` variants: `StrategyLoaded`, `StrategySwapped`,
  `StrategyLoadError`, `StrategiesRefreshed`, `StrategiesError`,
  `StrategySignalObserved(StrategyId, Timestamp)`. No catch-all.
- Helpers: `hash_strings` (32-byte sha256 → `(short, full)` hex),
  `apply_strategy_loaded / _swapped / _load_error`, `upsert_row`,
  `push_recent_event`.
- Unit tests added: seven new state-transition tests cover each
  variant plus the per-row-error-without-id edge case.

### T524 — widgets::strategies

- Table header with six R5.1 columns; per-row status pill colored via
  `color::{POS, FG_MUTED, NEG}`; per-row error badge (caption-sized,
  `NEG`) carrying the `error_summary`. Recent-events footer colored by
  event kind (`ACCENT` / `WARN` / `FG_MUTED` / `NEG`).
- Frame reuse: `panel(..)`, `muted_body(..)`, `error_body(..)`,
  `col_header(..)` all shared with the v0 panels.
- Snapshot tests added in `tests/panel_snapshots.rs`: `strategies_loading`,
  `strategies_empty`, `strategies_error`, `strategies_ready_three_rows`,
  `strategies_per_row_error_badge` — plus the T527 layout snapshot
  `cockpit_layout_strategies_above_positions`.

### T525 — fixtures

- `fake_strategy_row_ready / _loading / _error`, `fake_strategy_rows`,
  `fake_event_load / _swap / _reject`, `fake_recent_events`,
  `fake_cockpit_with_strategies`. Deterministic hashes so snapshots are
  byte-stable.
- `cargo run --bin cockpit --features fixtures` now boots with the
  strategies panel in the Ready state with three rows.

### T527 — cockpit layout

- `bin/cockpit.rs` right column order: `strategies::view(..)` →
  `positions::view(..)` → `tape::view(..)`. Left column unchanged.
- Fixtures path swapped from `fake_cockpit_ready()` to
  `fake_cockpit_with_strategies()` so the layout smoke covers the full
  column stack.

### T528 — screenshots README row

- Added §4.5 "`strategies` — loaded strategies + swap log" to
  [spec/v0-paper-sma/reports/screenshots/README.md](screenshots/v0-paper-sma/README.md)
  covering all four panel states + the per-row-error visual + the list
  of string keys + the subscription → message mapping for T526.
- Bumped the "Panels landed" text in §2 / §4 intro from four panels to
  five.

## What is blocked

### T526 — ui::live three new subscribers

Implementation plan (ready to land once T512 exists):

- Add three `Channel` variants (`StrategyLoaded`, `StrategySwapped`,
  `StrategyError`) to the `live::Channel` enum. Each gets a stream
  builder that subscribes to the matching `bus.strategy_*()` receiver
  and maps `Ok(..)` → the corresponding `Message::Strategy*` variant.
- `RecvError::Lagged(n)` → `warn!(channel = "strategy_*", skipped = n)`
  + continue (matches the v0 fills / positions pattern).
- `RecvError::Closed` → `Message::StrategiesError(SmolStr::new(
  strings::CONNECTION_CHANNEL_CLOSED))` (reuses the v0 copy per the
  feature spec → "Registry channel closed — restart the agent." lives
  in `CONNECTION_CHANNEL_CLOSED`; the error prefix is added by the
  widget via `error_body(STRATEGIES_ERROR_PREFIX, ..)`).
- Integration test `crates/ui/tests/strategies_subscription.rs`
  (feature-gated on `live`): spins up a fake `EventBus`, publishes
  one of each event, asserts the right `Message::Strategy*` variant
  reaches `update(..)` within 2s. A fourth lagged-receiver test
  overflows the channel and asserts no panic.

### T_FINAL_B — UI smoke extension

Depends on T526 + T_FINAL_A (developer). The scripted run that drives
the panel through empty → loading → ready → error → ready-after-
recovery needs the live subscription to actually receive events; the
fixtures path alone does not exercise the subscription error paths.

## Quality gates run this pass

| Gate | Command | Status |
|---|---|---|
| fmt | `cargo fmt -p ui -- --check` | PASS |
| clippy (default) | `cargo clippy -p ui --all-targets -- -D warnings` | PASS |
| clippy (all features) | `cargo clippy -p ui --all-targets --all-features -- -D warnings` | PASS |
| check | `cargo check -p ui --all-features` | PASS |
| tests (default) | `cargo test -p ui` | PASS — 25 lib + 2 consistency + 30 snapshots = 57 |
| tests (live) | `cargo test -p ui --features live` | PASS — no change from v0 baseline; T526 test batch pending |
| cockpit build (fixtures) | `cargo build -p ui --bin cockpit --features fixtures` | PASS |
| cockpit build (live) | `cargo build -p ui --bin cockpit --features live` | PASS |
| consistency audit — inline strings in widgets | `no_inline_user_visible_strings_in_widgets` | 0 — PASS |
| consistency audit — inline hex | `no_inline_hex_colors_in_widgets_or_state` | 0 — PASS |

Workspace regression check (`cargo test --workspace`) is the developer's
responsibility at T512 merge — the ui-designer did not touch developer
crates.

## Handoff

HANDOFF → developer

Blocker: waiting on **T512** (`agent::EventBus` three new broadcast
channels + publishers + v0.5 extension section in
`spec/v0-paper-sma/reports/dev-week2-broadcast-api-2026-04-18.md`).

Tasks complete (ui-designer): T522, T523, T524, T525, T527, T528.
Tasks blocked (ui-designer): T526 (gated on T512), T_FINAL_B (gated on
T526 + T_FINAL_A).

Next steps: once T512 merges, the orchestrator re-spawns ui-designer
with the full implementation plan carried in §"What is blocked" above.
Expected additional test-count delta: +3 to +5 live-suite tests; zero
new default-suite tests (T526 is pure subscription wiring).
