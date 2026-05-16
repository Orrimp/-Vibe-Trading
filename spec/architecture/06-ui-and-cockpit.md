---
slug: architecture-06-ui-and-cockpit
status: shipped
owner: ui-designer
updated: 2026-05-16
---

# UI and cockpit architecture

The cockpit (live ops) and viewer (offline backtest) UI architecture
built on iced. The "why iced" decision lives in
[ADR-0023](adr/0023-iced-frontend.md); this file holds the current-state
UI architecture — screen routing, the `audit::query` read-only surface,
KPI strip widget contracts, status bar, and the Lumen design-system
integration. Content migrated from `spec/architecture.md` §
Foundation libraries — Frontend — iced during Phase 1A Session 11
(2026-05-13).

Companion docs:
- [`../ui-design-principles.md`](../ui-design-principles.md) — the prose
  contract for visual / interaction patterns.
- `crates/ui/src/theme.rs` — the executable contract for tokens.
- [`../design/`](../design/) — the Lumen bundle (source-of-record for
  future token additions).

When `theme.rs` and `spec/design/` diverge, `theme.rs` wins. See
[ADR-0018](adr/0018-lumen-phase-1-foundation.md) for the rationale.

## Stack

[iced](https://github.com/iced-rs/iced) — see [ADR-0023](adr/0023-iced-frontend.md).


Single UI stack across the project. No mixing with `egui`/`tauri`/`dioxus`.

- `iced` — Elm-architecture (`Model` / `Message` / `update` / `view` /
  `Subscription`); GPU-accelerated via wgpu; multi-window.
- `iced_aw` — community widgets (date pickers, modals, tabs, badges).
- `plotters` with `plotters-iced` backend — equity curves, indicator overlays,
  drawdown plots in the backtest viewer. Architect to spike `plotters` vs
  hand-rolled `iced::widget::Canvas` for the live candlestick view before
  locking in.

#### Why iced fits

- **Subscriptions** wrap our `tokio::sync::mpsc` and `BroadcastStream` feeds —
  the existing actor pattern composes directly. No bespoke glue code.
- **Pure `update` functions** make every state mutation reviewable; matches
  the auditability goal in [product.md](../product.md).
- **Multi-window** lets ops cockpit (real-time) and backtest viewer (offline)
  run as separate top-level apps in the same crate, sharing widgets.

#### App layout

| Binary         | Window(s) (post-Phase-2 contract)                            | Data source                          |
|----------------|--------------------------------------------------------------|--------------------------------------|
| `cockpit`      | Sidebar shell · screens routed via `Cockpit::current_screen`  | `ui::fixtures` (deterministic)        |
| `cockpit_live` | Sidebar shell · screens routed via `Cockpit::current_screen`  | `agent` over `Arc<EventBus>` (in-process broadcast) |
| `viewer`       | Backtest report shell · KPI strip + equity curve + drawdown band + markdown body (Phase 4 — shipped) | `spec/reports/` markdown + `<stem>__equity.csv` companion |

The cockpit's sidebar exposes the screens shipped per phase:
**Phase 2** — Home / Debug / Charts. **Phase 3** — adds Strategies /
Risk / Audit. **Phase 6** — adds the right-rail Assistant (gated on
v2 LLM). Phase-1-shipped widgets compose into the appropriate screen
body; the cockpit shell renders the sidebar + selected screen body +
the always-visible status bar at the bottom. See
[Cockpit screen routing (Phase 2+ contract)](#cockpit-screen-routing-phase-2-contract--added-2026-05-04)
below for the state shape and bus-path contract.

**UI isolation rule.** The `ui` crate — both its library and every
binary target it ships (`cockpit`, `cockpit_live`, `viewer`) — depends
only on `core` (shared domain types), `audit` (read-only ledger queries
via `audit::query`), and `agent` (the public `agent::runtime::run`
surface plus the shared `Arc<EventBus>` / `Arc<KillSwitch>` /
`Arc<StrategyRegistry>` handles `agent` constructs). It **never**
depends — directly or transitively from any `crates/ui/src/` file,
including `crates/ui/src/bin/*` — on `strategy`, `exec`, `models`,
or `llm`.

Bootstrap of `strategy::StrategyRegistry`, `exec::PaperEngine`,
`models::*`, and `llm::*` happens in `agent` (typically inside the
`agent::runtime::run` setup path). The UI receives them as already-
constructed `Arc<…>` handles threaded through `RunHandles`. No UI
file may `use strategy::…` / `use exec::…` / `use models::…` /
`use llm::…`. There are no "for now" carveouts: if a cockpit
feature needs a registry type, it consumes it through the
`agent`-owned handle or via `audit::query` projections — never by
adding a direct dependency edge from `ui` to those crates.

This keeps the UI swappable without touching trading logic and
keeps the dependency graph acyclic (`agent → ui` would be a cycle;
`ui → strategy/exec/models/llm` would entangle render concerns with
trading logic and make UI rebuilds drag the trading stack along).

#### `audit::query` — the read-only surface for `ui` — confirmed 2026-04-17

`ui`'s ledger dependency is limited to a dedicated `audit::query` module that
exposes `Decimal` aggregates and slice iterators. No SQL string, no `sqlx`
type, and no mutable handle leaks into `ui`. Minimum surface for v0 (signatures
indicative):

```rust
pub mod audit::query {
    pub fn cash_balance(asset: Asset) -> Result<Money<Usdt>, QueryError>;
    pub fn position(asset: Asset) -> Result<Position, QueryError>;
    pub fn equity() -> Result<Money<Usdt>, QueryError>;
    pub fn realized_pnl_since(ts: Timestamp) -> Result<Money<Usdt>, QueryError>;
    pub fn unrealized_pnl() -> Result<Money<Usdt>, QueryError>;
    pub fn recent_fills(limit: usize) -> Result<Vec<FillView>, QueryError>;
    pub fn recent_journal(limit: usize) -> Result<Vec<JournalEntryView>, QueryError>;
}
```

`FillView` / `JournalEntryView` are `core`-defined read-side types (no crate
back-edge from `core` to `audit`). The cockpit P&L card calls `equity()` /
`realized_pnl_since()` / `unrealized_pnl()`; the live tape calls
`recent_fills()`. This makes the ledger — not a cockpit accumulator — the
single source of truth for P&L (locks R3.6 in the v0-paper-sma feature brief).

**Rationale:** approved per existing UI constraint, now made explicit and
signed.

**Alternatives considered:** duplicate P&L computation in `ui` for speed —
rejected; violates R3.6 and creates the exact reconciliation gap the
differentiator argues against.

#### Constraints

- Pin a specific iced version per workspace; do not chase releases mid-feature
  (iced API churns between minors).
- All copy lives in a `ui::strings` module so it can be reviewed for clarity
  and later localized.
- Color, spacing, and typography flow from a single `ui::theme` module — no
  ad-hoc styles inside widgets.
- Destructive actions (kill switch, close all positions, cancel all orders)
  require a confirm dialog with a typed-input safety phrase.
- Empty, loading, and error states are first-class for every view — no blank
  screens.

#### Frontend ↔ backend interfaces — confirmed 2026-05-02

The cockpit + viewer are the only operator-facing surfaces; this subsection
formalizes every load-bearing interface they consume so a future
ui-designer / developer / architect doesn't have to grep the codebase to
find the contract. See also [spec/ui-design-principles.md](../ui-design-principles.md)
for the design-system rules these interfaces dress.

##### Surface map

| Interface                                | Direction | Producer                            | Consumer                          | Doc anchor                              |
|------------------------------------------|-----------|-------------------------------------|-----------------------------------|-----------------------------------------|
| `Arc<EventBus>` broadcast channels       | →         | `agent::runtime::run`               | `ui::live::subscription`           | [bus shape](#cockpit--eventbus)         |
| `audit::query` read-only API             | →         | `crates/audit/src/query.rs`         | `cockpit_live` Subscription, `viewer` | [query module](#cockpit--auditquery) |
| `KillTripFn` closure                     | ←         | `cockpit_live` builds, calls `KillSwitch::trip` | `agent::KillSwitch::trip`         | [kill switch](#cockpit--arckillswitch)  |
| `spec/reports/**/*.md` markdown          | →         | `tester` (writes), `presenter`       | `viewer` binary (reads)           | [viewer](#viewer--specreports)          |
| `theme` tokens                           | (closed)  | `ui::theme`                          | every `ui::widgets::*`            | [theme](#theme--widget)                 |
| `strings` constants                      | (closed)  | `ui::strings`                        | every `ui::widgets::*`            | [strings](#strings--widget)             |
| `fixtures` data                          | →         | `ui::fixtures` (`--features fixtures`)| `cockpit` (dev-mode subscription)| [fixtures](#fixtures--widget)           |

The arrows are deliberately one-way for read paths. The cockpit
**never** mutates the agent except via the kill-switch closure (the
sole operator → backend write surface) and **never** writes to the
audit ledger directly. The agent never reads cockpit state.

##### Cockpit ← `EventBus`

Live event push, same-process. The `agent::EventBus` (defined in
`crates/agent/src/bus.rs`) holds one `tokio::sync::broadcast::Sender<T>`
per domain channel. The `cockpit_live` binary constructs the bus once
on the main thread, hands an `Arc<EventBus>` to both the agent runtime
side-thread and the iced cockpit's `Subscription`, and the broadcast
fan-out does the rest. No IPC.

| Channel                     | Type                       | Capacity | Sender call site                                | Receiver                                  |
|-----------------------------|----------------------------|----------|-------------------------------------------------|-------------------------------------------|
| `fills_tx`                  | `core::Fill`               | 1024     | `EventBus as exec::FillPublisher` (paper engine glue) | `bus.fills()` → `Message::FillReceived`   |
| `positions_tx`              | `core::Position`           | 256      | `EventBus as exec::FillPublisher` + reconciler  | `bus.positions()` → `Message::PositionsRefreshed` |
| `bars_tx`                   | `core::Bar`                | 1024     | data feed (`agent::runtime` bar fan-out)        | `bus.bars()` → `Message::BarReceived` + `Message::BarClose` |
| `ticks_tx`                  | `core::Tick`               | 8192     | data feed (`agent::runtime` tick fan-out)       | `bus.ticks()` → `Message::TickReceived`   |
| `pnl_tx`                    | `core::PnlSnapshot`        | 256      | reconciler (post bar close)                     | `bus.pnl()` → `Message::PnlRefreshed`     |
| `mode_tx`                   | `agent::AgentMode`         | 32       | `KillSwitch::trip` + boot                       | `bus.mode()` → `Message::AgentModeChanged` / `AgentHaltedExternally` |
| `strategy_loaded_tx`        | `core::StrategyLoaded`     | 32       | `agent::watcher` initial-load + reload          | `bus.strategy_loaded()` → `Message::StrategyLoaded` |
| `strategy_swapped_tx`       | `core::StrategySwapped`    | 32       | `agent::watcher` hot-swap                       | `bus.strategy_swapped()` → `Message::StrategySwapped` |
| `strategy_error_tx`         | `core::StrategyLoadError`  | 32       | `agent::watcher` parse / typecheck failure      | `bus.strategy_error()` → `Message::StrategyLoadError` |
| `funding_obs_tx`            | `core::FundingObs`         | 32       | `FundingPoller::run` (v1 Q2)                    | observation-only (no cockpit subscriber today) |

The `ui::live::subscription` function (in `crates/ui/src/live.rs`) batches
nine `iced::Recipe`s — one per channel except `funding_obs` (no consumer
yet). Each recipe **subscribes synchronously** before yielding the
async stream so events published before the first `.next().await` are
not dropped (eager-subscribe is the documented contract).

**Backpressure policy** (single rule, applied per channel):

- `Ok(event)` → translate to a typed `Message::*` variant and yield.
- `RecvError::Lagged(n)` → log via `tracing` and continue. The cockpit
  is allowed to fall behind; the agent never blocks. Chosen log level
  is `warn` for events the operator should know about (`fills`,
  `positions`, `pnl`, `bars`, `mode`, all three `strategy_*`),
  `debug` for routine high-volume lag (`ticks`).
- `RecvError::Closed` → emit a panel-specific error variant with the
  shared `strings::CONNECTION_CHANNEL_CLOSED` copy
  (`Message::TapeError` / `PositionsError` / `PnlError` /
  `StrategiesError`), then break the stream. The mode channel closing
  is treated as `Message::AgentHaltedExternally` because losing the
  mode broadcaster means the agent process is gone.

The bus type-conversion layer (`fill_to_view`, `position_to_view`,
`mode_to_message`) lives in `ui::live` — not in `core` — so `core`
stays free of UI concerns. `FillView` / `PositionView` are the
audit-facing read types; `Fill` / `Position` are the runtime-facing
types. The conversion is lossy by design (UI doesn't need
`order_id`, `local_ts`, etc.) and is the only legal way bus types
enter the `Message` enum.

##### Viewer ← `spec/reports/`

The offline `viewer` binary consumes **markdown files**, not a live
data path. It reads `spec/reports/**/*.md` directly off disk and
renders the body in iced. There is no agent dependency.

File-naming convention (locked):

| Pattern                                       | Producer       | Body content                            |
|-----------------------------------------------|----------------|-----------------------------------------|
| `backtest-<YYYYMMDD>-<HHMMSS>-<scenario>.md`  | `crates/backtest` binary | Equity curve, trade log, anchored body |
| `success-<YYYYMMDD>-<scenario>.md`            | `crates/reports` binary  | Operator-facing weekly success summary |
| `test-<YYYYMMDDD>-<HHMM>-<slug>.md`           | `tester` agent + `rust-test` skill | Test verdict, table, anchors  |
| `dev-<feature>-<slug>-<date>.md`              | developer agent (handoff) | Implementation notes for tester    |
| `ui-debt-<YYYY-MM-DD>.md`                     | ui-designer agent | Consistency drift reports         |
| `ui-week<N>-smoke-checklist-<date>.md`        | ui-designer agent | Operator-side manual smoke checklist|

**Body-vs-front-matter discipline** (re-stated, since the viewer
reads both): anything that may differ between two equivalent runs
(`generated:` timestamp, host, pid, git commit, wall-clock seconds,
data-source variants) belongs in YAML front-matter — never in the
body. The body is what gets hashed for anchor verification
(`scripts/hash_report.py`). The viewer renders front-matter as a
collapsed metadata table at the top of the screen and the body as
the main reading surface; reading-fidelity for the body is what
matters.

The viewer is read-only on the spec tree — never edits, never
deletes. Re-running a backtest writes a new file with a new
timestamp; old reports are immutable history.

##### Cockpit ← `audit::query`

Read-only ledger access. The cockpit (and viewer) calls into
`crates/audit/src/query.rs` for any aggregate that lives in the
ledger but isn't published on the bus. The shape is async functions
that take `&Ledger` and return typed `Result<T, LedgerError>`. No
`sqlx` types leak across the boundary.

Public API the cockpit may call (the union of what's currently used
plus what's expected to be used by v1.5+):

| Function                                  | Returns                          | Used by                                |
|-------------------------------------------|----------------------------------|----------------------------------------|
| `cash_balance(&Ledger)`                   | `Money<Usdt>`                    | P&L card snapshot                       |
| `realized_pnl_since(&Ledger, Timestamp)`  | `Money<Usdt>`                    | P&L card                                |
| `total_fees(&Ledger)`                     | `Money<Usdt>`                    | future cost-card / footer              |
| `recent_fills(&Ledger, usize)`            | `Vec<FillView>`                  | live tape (boot snapshot)              |
| `recent_journal(&Ledger, usize)`          | `Vec<JournalEntryView>`          | future "show the why" modal (collapsed view) |
| `journal_entries_for_transaction(&Ledger, &str)` | `Vec<JournalEntry>`        | tape-row → audit modal (un-collapsed dr/cr) |
| `journal_transaction_metadata(&Ledger, &str)` | `Option<JournalTransactionMetadata>` | tape-row → audit modal header (description + strategy_id) |
| `open_positions_at(&Ledger, Timestamp)`   | `Vec<OpenPosition>`              | positions panel snapshot               |
| `pnl_by_symbol(&Ledger, ...)`             | per-symbol P&L                   | positions panel                         |
| `pnl_by_strategy(&Ledger, ...)`           | per-strategy P&L                 | strategies panel                        |
| `pnl_by_pair(&Ledger, ...)`               | per-pair P&L (v1.5a)             | future pairs panel                      |
| `strategy_events_since(&Ledger, Timestamp)` | `Vec<StrategyEventView>`        | strategies panel footer                |
| `strategy_history(&Ledger, StrategyId)`   | `Vec<StrategyEventView>`         | future strategy-detail modal           |
| `funding_rate_history(&Ledger, ...)`      | `Vec<FundingObs>`                | future funding-observation panel       |
| `uptime_intervals_since(&Ledger, Timestamp)` | `Vec<UptimeInterval>`         | future operator-uptime card            |
| `ledger_snapshot_sha(&Path)`              | `[u8; 32]`                       | viewer integrity check                  |
| `ledger_inception_ts(&Ledger)`            | `Timestamp`                      | viewer "since" timestamps               |

**Hard constraint:** the cockpit MUST NOT call `audit::ledger`
writers (`Ledger::post_fill`, `Ledger::post_strategy_event`, etc.).
The bus is the **only** event-push surface from the operator to the
audit ledger, and it is mediated by the agent runtime — the cockpit
never bypasses the agent's invariant checks. The single exception
is the kill-switch trip closure, which goes through
`agent::KillSwitch::trip`, which itself calls the audit writer
(T809 dual-write) — the cockpit never touches `Ledger` directly.

##### Cockpit ← `Arc<KillSwitch>`

The only operator → backend write surface. Resolved per the
[live-cockpit-unified Q6](#v05--cockpit-strategies-panel-layout-q4--confirmed-2026-04-19)
follow-up — the cockpit holds a `KillTripFn` closure
(`Arc<dyn Fn(agent::HaltReason) + Send + Sync>`) defined in
`crates/ui/src/state.rs` under `#[cfg(feature = "live")]`.

The closure exists because `KillSwitch::trip(reason)` uses
`tokio::spawn` internally for its T809 audit dual-write side effect,
which requires a tokio runtime in scope at the call site. The iced
`update` function runs on the iced thread, where there is **no
tokio runtime**. The closure injects a `tokio::runtime::Handle`
captured in `cockpit_live::main` from the side-thread runtime,
so the spawn lands on it.

Construction sequence in `cockpit_live::main`:

1. Build a `tokio::runtime::Builder::new_multi_thread().enable_all().build()`
   runtime on the main thread.
2. Capture `runtime.handle().clone()` BEFORE moving the runtime
   into the side thread.
3. Build the closure: `move |reason| handle.spawn(async move { kill_switch.trip(reason).await })`.
4. Move the runtime to the side thread, run `agent::runtime::run`.
5. Pass the closure into `Cockpit { kill_switch: Some(closure), … }`
   on the iced main thread.
6. The `Message::KillConfirmed` arm in `state::update` calls the
   closure with `agent::HaltReason::ManualOperator` after the
   safety-phrase gate passes; the closure spawns
   `KillSwitch::trip(ManualOperator)` onto the side-thread runtime,
   the trip writes the audit memo + `strategy_events` row + spawns
   the incident-report helper, and broadcasts `AgentMode::Halted`
   on `mode_tx`. The cockpit observes the halt via its existing
   mode subscription.

The fixture-only `cockpit` binary (built without `--features live`
or with `--features fixtures`) constructs `kill_switch: None`. The
kill button still flips the UI to `KillState::Flattening` for smoke
testing but does not contact any agent. This separation is the
contract that lets the `cockpit` binary be a pure-design dev tool.

##### Theme ↔ widget

The widget code's only legal source of color, spacing, type sizes,
border radii, and latency thresholds is `ui::theme`. The four sub-
modules are closed sets:

- `theme::color` — 12 shipped semantic tokens (9 v0–v1.5a +
  3 added by [tape-row-audit-modal](../tape-row-audit-modal/feature.md):
  `bg_overlay = #0B0D12`, `info = #7BC2FF`,
  `border_strong = #3A4456` — first concrete consumer is the
  journal-transaction modal). `Color::from_rgb(…)` outside
  `theme.rs` is a build break.
- `theme::space` — `XS=4, S=8, M=12, L=16, XL=24, XXL=32`.
- `theme::text` — `CAPTION=11, BODY=13, TITLE=16, DISPLAY=22`.
- `theme::radius` — `SMALL=2.0, MEDIUM=4.0`.
- `theme::layout` — panel padding, gap, max tape rows.
- `theme::latency` — `OK_MS=500, WARN_MS=2000, HALTED_MS=10000`.

Helper functions encode "color-from-data" rules so widgets don't
re-implement them inline:

- `theme::color_for_delta(Decimal)` → `pos` / `neg` / `fg_muted`
  for signed values. The single rule: zero is muted, sign drives
  color.
- `theme::color_for_latency_ms(i64)` → `pos` / `warn` / `neg`
  per the threshold bands.

The consistency test `crates/ui/tests/consistency.rs` enforces "no
inline hex" by grep-failing the build on any
`Color::from_rgb` or `#rrggbb`-shaped literal outside `theme.rs`.
Adding a new token is a `theme.rs` change plus a principles-doc
update — never a one-off in a widget.

##### Strings ↔ widget

Same shape as theme: `crates/ui/src/strings.rs` is the single source
of operator-visible copy. Every constant is a `pub const &str`. The
`strings::all()` function returns an ordered slice of `(key, value)`
pairs, exercised by tests for uniqueness and non-emptiness, and
designed to be a future i18n extraction point.

Pattern conventions:

- `*_TITLE` — panel / dialog titles.
- `*_COL_*` — table column headers.
- `*_LOADING` / `*_EMPTY` / `*_ERROR_PREFIX` — first-class panel state copy.
- `*_LABEL` / `*_HELP` — button labels and helper tooltips.
- `*_SAFETY_PHRASE` — typed-confirm phrases (currently only
  `KILL_SAFETY_PHRASE = "HALT BTC"`).

The consistency test `no_inline_user_visible_strings_in_widgets`
fails the build on any string literal in `crates/ui/src/widgets/`.
The `strings.rs` file is the one place a copy review happens.

##### Fixtures ↔ widget

The `cockpit` binary built with `--features fixtures` reads from
`ui::fixtures::*` instead of subscribing to a real bus. The same
`Cockpit` model is populated; the same widgets render; only the
data source differs. This is the design dev-mode loop — the
ui-designer can iterate on a widget without booting the full agent.

Fixture functions are deterministic (`ChaCha20Rng::from_seed`) so
two runs of `cargo run --bin cockpit --features fixtures` produce
the same screenshot. This is the contract for the
`spec/<slug>/reports/screenshots/` PNG regeneration path.

The `cockpit` binary is **never** the production runtime — that
role belongs to `cockpit_live` (with the live agent attached to a
real bus). Fixture-mode is dev-only.

##### Cockpit screen routing (Phase 2+ contract — added 2026-05-04)

Phase 1 (shipped) kept the cockpit as a single-page layout — every
panel visible together. The
[`lumen-design-adoption`](../lumen-design-adoption/feature.md) Phase 2
brief introduces a **left-sidebar shell with multiple screens**.
This sub-section is the architecture-level contract that every
Phase 2+ widget plugs into; per-phase R-items live in the per-phase
briefs.

**State shape — additions to `Cockpit`:**

```rust
pub enum Screen {
    Home,    // Phase 2 — pnl + positions + strategies + tape
    Debug,   // Phase 2 — kill + latency + market_health + version
    Charts,  // Phase 2 — per-symbol price chart with buy/sell markers
    Strategies, // Phase 3
    Risk,    // Phase 3
    Audit,   // Phase 3
}

pub struct Cockpit {
    // … existing fields …
    pub current_screen: Screen,                 // Phase 2 default = Home
    pub selected_symbol: Option<(Venue, Symbol)>, // Charts screen state
    pub chart_buffer: ChartBuffer,              // see below
    // … later phases extend …
}

pub enum Message {
    // … existing variants …
    SwitchScreen(Screen),                       // Phase 2
    SelectSymbol(Venue, Symbol),                // Phase 2 (Charts)
    ChartTickReceived(Venue, Symbol, Tick),     // Phase 2 (Charts) — bus path
    // … later phases extend …
}
```

The cockpit shell's `view()` dispatches on `cockpit.current_screen`
and renders the appropriate screen body. The `SwitchScreen`
handler is a pure assignment; no side effects.

**Chart data path** — per-`(venue, symbol)` rolling buffer:

```rust
pub struct ChartBuffer {
    // Keyed by (venue, symbol). Each value is a fixed-capacity
    // ring buffer of OHLC bars (default capacity = 60 minutes of
    // 1-minute bars).
    pub series: HashMap<(Venue, Symbol), VecDeque<Bar>>,
}
```

- **Live mode** (`cockpit_live`, `--features live`): the existing
  `bars_tx` channel on the `EventBus` already carries every
  `core::Bar` produced by the data feed. The cockpit's existing
  `Message::BarReceived` handler is extended to push the bar into
  `chart_buffer.series.entry((venue, bar.symbol)).or_default()`,
  popping the oldest if the buffer is at capacity. **No new bus
  channel.**
- **Fixtures mode** (`cockpit --features fixtures`): the existing
  `ui::fixtures` module gains a deterministic `synthetic_candles(seed,
  venue, symbol, count)` helper (random walk via
  `ChaCha20Rng::from_seed`). The fixtures-mode subscription seeds
  the chart buffer at boot and continues to emit synthetic
  `Bar` values via the existing fixture-bus shim, so the same
  `Message::BarReceived` path populates the buffer in both modes.
- **Snapshot stability**: fixtures-mode uses a single fixed seed
  per symbol, so `cargo run --bin cockpit --features fixtures`
  produces a chart with the same bar shape every run — the
  contract for the chart's snapshot baselines.

**Audit query extension** — additive to existing
[`audit::query`](#cockpit--auditquery):

```rust
pub mod audit::query {
    // … existing API unchanged …

    /// Phase 2 addition. Returns fills filtered by venue, symbol,
    /// and time-range — used by the Charts screen to populate
    /// buy/sell markers within the chart's visible window.
    ///
    /// Read-only over the same description-prefixed rows that
    /// `recent_fills` already iterates. Additive; does not alter
    /// any committed report body.
    pub fn recent_fills_filtered(
        venue: Venue,
        symbol: Symbol,
        time_range: Range<Timestamp>,
    ) -> Result<Vec<FillView>, QueryError>;
}
```

Phase 3 may extend the signature with an `Option<&str> kind`
parameter for the Audit screen's filter row; Phase 2 ships the
narrow signature. Architect resolves the exact column-projection
shape at Phase 2 design — the contract above is the master-roadmap
intent, not the final R-item language.

**Sidebar nav widget** — `crates/ui/src/widgets/sidebar_nav.rs`
(new in Phase 2). Renders a vertical column of nav entries with
the T1507 active-row pattern (2 px ACCENT left rule on the
selected entry). Width fixed at ~180 px; Tier 1 background; text-
only labels until icon adoption is re-litigated. The widget emits
a `Message::SwitchScreen(Screen)` per selected entry and is
otherwise stateless — `current_screen` lives on `Cockpit`.

**Right-rail track reservation for Phase 6** — Phase 2's shell grid
**reserves** a right column-track for the Phase 6 Assistant slot
(see
[`lumen-phase-6-assistant-slot.md`](../lumen-phase-6-assistant-slot/feature.md)).
Reservation = the column exists in the grid spec but has zero
width when the v2 LLM strategy is not enabled. No widget renders
in it; no token references it; the layout simply doesn't consume
the rightmost track. When v2 LLM ships, Phase 6 sets the track
width and inserts the assistant widget — no Phase 2-side change
needed.

**Bin parity.** Both `cockpit` (fixtures) and `cockpit_live` adopt
the sidebar shell + every screen routed through `Cockpit::current_screen`
in Phase 2. Phase 1's status bar continues to span the bottom of
every screen (see [`widgets::status_bar`](#cockpit--auditquery)).

##### Q1–Q11 ratification (Phase 2, confirmed 2026-05-04)

Architect's Phase 2 design landing for the
[lumen-design-adoption](../lumen-design-adoption/feature.md) initiative,
ratifying the analyst's brief at
[lumen-phase-2-shell-ia-charts](../lumen-phase-2-shell-ia-charts/feature.md).
Phase 2 is the second of six sequential phases; only Phase 2 ships
through this gate. Phases 3 / 4 / 5 are queued; Phase 6 is reserved
for the v2 LLM strategy. **11 / 11 architect Q-items ratified; zero
principled overrides.** Each Q resolution cites the R-item(s) it
ratifies; full Q-resolution table lives in the Phase 2 brief.

**Q1 — default plot style: line series in `ACCENT`** (R7.3). Phase 2's
chart is a cross-check surface, not a primary trading chart. The
operator's question is "did the marker land on or near the line at
the right time"; a line plot answers most directly. The OHLC variant
remains supportable from the same `ChartBuffer` shape (R10) — defer
to a post-Phase-2 ask if the operator requests it. The chart widget
body is line-only by default; the OHLC drawing helper is **not**
stubbed (no dead code).

**Q2 — pan/zoom: deferred** (R7.5). Phase 2 ships the fixed 60-minute
window. Pan/zoom adds ~2–3 R-items of widget surface (axis re-scaling,
hit-region tracking, marker re-positioning, snapshot stability) and
risks bloating Phase 2 past the "one shippable thing" budget.
Phase 4's Backtest equity curve is the natural next pan-capable
surface.

**Q3 — symbol-selector universe source: `Cockpit::universe`,
boot-populated** (R6.2). Live mode reads `Config.universe.usdt_symbols`
× `Config.data.sources` once before `iced::application::run`;
fixtures mode hard-codes the 3-symbol set. Static for the session.
Matches the existing `Cockpit::account_label` precedent (Phase 1
R13.4) — view-time `Config` reads couple the widget to live config
plumbing in a way fixtures mode can't satisfy without a shim;
market-health-key derivation drops never-ticked symbols from the
chip row, which is wrong for "show me the chart of X".

**Q4 — `recent_fills_filtered` signature: `(ledger, venue, symbol,
since: Timestamp, until: Timestamp)` half-open form** (R12.1, R12.2).
Symmetric with the existing `pnl_by_symbol(since, until)` at
`crates/audit/src/query.rs:586` and `funding_rate_history` shape;
symmetry beats Rust idiom. The chart's call-site computes
`(window_start, window_end)` deterministically from
`Cockpit::server_time_now` — no hidden clock. **Phase 2 venue
handling**: the `journal_transactions` rows are all `Venue::Binance`
per v1.5b plumbing-only state (architecture.md § v1.5b architectural
deltas line 2260+); the function returns the matching subset when
`venue == Binance`, `Ok(vec![])` when `venue != Binance`. Phase 3's
Audit screen promotes this to read a `journal_transactions.venue`
column added via a future migration. The forward-compat surface
keeps Phase 3 from rippling the call-site.

**Q5 — chip-row active rule: bottom-edge T1507 variant** (R6.3).
The active-row concept is "2 px ACCENT, no fill change"; the literal
edge depends on widget orientation. Sidebar-nav rows are vertical →
left rule; chip row is horizontal → bottom rule. The
`frame::active_chip` helper prepends a 2 px bottom rule via
`Column::push(content).push(rule_2px_bottom)` (mirror of the existing
`active_row` Row-composition helper). One-line note in the Phase 2
principles-doc append (a follow-up the orchestrator routes to the
analyst — architect does not edit `spec/ui-design-principles.md`,
operator-locked Phase 1 Q7).

**Q6 — synthetic-candle seed: per-symbol, in-process determinism**
(R11.1, R11.2). `seed_for(venue, symbol) = DefaultHasher` over
`format!("{venue:?}/{symbol}")`. Each chip's chart looks distinct in
fixtures mode. Determinism caveat: `DefaultHasher` is **not**
guaranteed to produce the same hash across compiler versions, only
within a single process. Acceptable for Phase 2 because snapshot
baselines are pinned per CI run and the test expectation is "two
calls within the same process produce equal output" — not "the seed
equals 0xDEADBEEF". Phase 3+ that needs cross-version determinism
promotes to `seahash` or a hand-rolled FNV; Phase 2 sticks with
`DefaultHasher` (zero new dep crosses the library-compat budget for
a non-load-bearing field). RNG remains
`ChaCha20Rng::from_seed([u8; 32])` per architect.md determinism
guardrails (no `f64`, no `thread_rng`).

**Q7 — right-rail track: structural now, single
`Length::Fixed(0.0)` column** (R13.1, R13.2). Master roadmap
Constraint 4 is unambiguous; a `Length::Fixed(0.0)` column is the
cheapest honest reservation (zero render cost, zero dead code — a
single `Length` constant), and Phase 6 swaps the constant to the
real width without restructuring the shell. **No
`cfg!(feature = "v2-llm")` gate** — the gate doesn't exist, and
adding a feature flag for one zero-pixel column is more dead code
than the column itself. Phase 1 Q9 deferred this for the original
Phase 4 assistant slot pre-roadmap-revision; the post-revision lock
at master Constraint 4 supersedes that.

**Q8 — sidebar nav state persistence: two-field addition only, no
on-disk persistence** (R2.2, R6.4). `Cockpit::current_screen: Screen`
(default `Home`) + `Cockpit::selected_symbol: Option<(Venue, Symbol)>`
(default `None`). Both session-scoped per
[ui-design-principles.md § Persistence](../ui-design-principles.md). No
`~/.cockpit-state.json`, no `serde::Serialize` on `Cockpit`, no
`Drop` impl writing state. The cockpit is an instrument, not a
browser.

**Q9 — Debug screen logs/metrics output stub: placeholder** (R5.7).
A single `frame::muted_body(strings::DEBUG_LOGS_PLACEHOLDER)` row at
the bottom of the Debug screen body; copy added to `ui::strings` per
the no-inline-prose rule. Zero new code paths; honest about the
scope boundary. Defer to a future "structured metrics surface" brief
when the operator names a specific gap (e.g. "I want to see reconnect
events on a graph"). The placeholder gives Phase 2 a clean stopping
point and Phase 3+ a clean follow-up trigger.

**Q10 — `recent_fills_filtered` test scope: unit only is required;
integration test optional in Phase 2** (R12.7, R12.8). The unit test
exercises the SQL projection + the description-parse against a
fixture ledger seeded inline (the existing
`crates/audit/tests/journal_entries_for_transaction.rs` precedent
shows the boilerplate is ~30 lines per integration). Phase 3's Audit
screen needs the multi-venue / multi-symbol / multi-kind integration
anyway, so the integration test promotes naturally one phase later.
Phase 2's gate is the unit test (`recent_fills_filtered_returns_window_subset`,
`recent_fills_filtered_empty_window_returns_ok_empty`,
`recent_fills_filtered_distinct_symbols_isolated`) + the V3 manual
chart-renders-markers run.

**Q11 — TD-1 re-evaluation: deferral restated** (TD-1 cross-phase
row). Verified at design pass on disk: `crates/ui/Cargo.toml:50`
reads `iced = { version = "=0.14.0", default-features = false,
features = ["tiny-skia", "thread-pool", "advanced"] }`. iced 0.15+
has not landed; the `button::Status::Focused` variant and
`text_input::Style.shadow` field are **not** available. **Phase 2
ships no focus-ring upgrade.** Phase 1's deferred state holds:
hover-state ring on the three buttons named in T1504 (kill trigger,
kill confirm, modal close); ACCENT border-shift on the kill confirm
input. Operator-impact bound is unchanged — kill-switch destructive
flow is typed-confirm gated, focus halo is a secondary signal.
Named upgrade trigger unchanged — any iced version bump that
exposes both fields promotes the ~30-line one-file sweep across
`widgets/kill.rs` (two button styles + one input) and
`widgets/journal_transaction_modal.rs` (one button style); next
re-evaluation at Phase 3 analyst kickoff. The TD-1 row in the
master roadmap should be appended with a 2026-05-04 line under
"Promotion timing" noting the Phase 2 design verification — that's
a follow-up the orchestrator routes to the analyst on Phase 2 ship
(architect does not edit the master roadmap directly).

**Cross-feature invariants preserved.** All 7 prior shipped features
([master roadmap cross-feature invariants
table](../lumen-design-adoption/feature.md#cross-feature-invariants))
remain green post-Phase 2 — see the brief's "Cross-feature
invariants" sub-section for the row-by-row preservation note.

**Anchor budget.** Zero touched. UI shell + read-only audit query;
`crates/strategy/`, `crates/exec/`, `crates/risk/`, `crates/cost/`,
`crates/backtest/`, `crates/reports/` unchanged. The 11 backtest
body-SHA-256 anchors in [`spec/anchors.toml`](../anchors.toml) verify
byte-identical post-Phase 2; the `recent_fills_filtered` query is
read-only over the same description-prefixed rows `recent_fills`
already iterates and cannot alter any committed report body by
construction.

**Library compatibility checklist.**

- iced still pinned `=0.14.0` (`crates/ui/Cargo.toml:50`); no new
  iced version, no new dep. **Q11 deferral verified on disk.**
- `rand_chacha::ChaCha20Rng` already in the workspace via prior
  fixtures use; the `synthetic_candles` helper reuses the existing
  dep — no new addition.
- `std::hash::DefaultHasher` is in `std`; no new dep for the
  per-symbol seed.
- Lucide icons explicitly out of scope (master Constraint).

**Tasks.** `T1601–T1616` + `T_FINAL_LUMEN_PHASE_2` filed at
[tasks/lumen-phase-2-shell-ia-charts.md](../lumen-phase-2-shell-ia-charts/tasks.md).
T1601 is the foundation gate (state additions); T1602 (sidebar nav
widget) and T1603 (shell rewiring) sequence after. After T1603,
eight tasks fan out (T1604 Home / T1605 Debug / T1606 audit query
/ T1607 fixtures / T1608 chart canvas / T1609 chip-row variant /
T1610 Charts wiring / T1611 right-rail / T1612 universe boot).
T1613 (snapshot accept) is the narrow point. T1614 / T1615 / T1616
close out before the tester gate.

##### Q1–Q11 ratification (Phase 3, confirmed 2026-05-05)

Architect's Phase 3 design landing for the
[lumen-design-adoption](../lumen-design-adoption/feature.md) initiative,
ratifying the analyst's brief at
[lumen-phase-3-detail-screens](../lumen-phase-3-detail-screens/feature.md).
Phase 3 is the third of six sequential phases (Phase 1 + Phase 2
shipped 2026-05-04 / 2026-05-05); Phases 4 / 5 are queued; Phase 6
is reserved for the v2 LLM strategy. **11 / 11 architect Q-items
ratified; zero principled overrides.** Each Q resolution cites the
R-item(s) it ratifies; full resolution table lives in the Phase 3
brief.

**Q1 — `journal_transactions.venue` migration scope: ship in Phase 3**
(R13.1–R13.6). Migration filename
`crates/audit/migrations/008_journal_transactions_venue.sql` (next-
numbered after the existing `007_strategy_events_venue.sql`). SQL is
`ALTER TABLE journal_transactions ADD COLUMN venue TEXT NOT NULL
DEFAULT 'Binance';` — the `DEFAULT 'Binance'` clause backfills every
existing row in one statement (no separate UPDATE pass needed; every
shipped fill on disk is Binance per the Phase 2 venue-handling note).
The writer at `crates/audit/src/journal.rs::post_fill` gains a
`venue: Venue` parameter; the existing `Fill` struct does not carry
venue (only `venue_ts` / `local_ts`), so the runtime caller stamps it
explicitly. The two other `INSERT INTO journal_transactions`
call-sites (funding-obs writer + reconciliation writer) take the same
treatment. Phase 2's `recent_fills_filtered` venue gate (`if venue !=
Venue::Binance { return Ok(Vec::new()) }`) drops; replaced with
`WHERE venue = ?` in the SQL. Splitting as Phase 3.5 was rejected —
~30 LOC, additive, one consumer (the Audit screen); gate friction
without isolating actual risk.

**Q2 — Strategies-detail signal-history: filter
`Cockpit::strategies_recent_events`** (R5.4). Phase 1 R5 already
populates the buffer from the strategy-event subscription; Phase 3
filters by `selected_strategy` at view time. Zero new audit writers —
Phase 5 HumanControl introduces the first new operator-write paths.
A new `signal_emitted` audit writer was rejected as a violation of
the master-roadmap "no new audit writers in Phase 3" stance and as
unnecessary anchor-risk surface for a screen that only renders
existing telemetry.

**Q3 — Risk screen exposure source: new tokio channel**
(R8.1–R8.5). New `RiskTelemetry` event type sibling of `MarketHealth`
on the agent runtime's `EventBus`; published by
`crates/risk/src/portfolio.rs` at 1 Hz; consumed by the cockpit's
`Subscription::batch` recipe in `crates/ui/src/live.rs` mapped to
`Message::RiskStateRefreshed(RiskState)`. ~40 LOC publisher + ~20 LOC
subscriber. Polled view-time reads were rejected — couples render-rate
to agent-state-cache locking and breaks the "screens are pure render
dispatches" invariant. The cockpit-thread isolation rule (no UI-thread
reads from agent-runtime mutexes) is load-bearing; the Phase 1
`MarketHealth` channel is the canonical example.

**Q4 — Audit screen pagination: fixed 250 rows / page** (R9.3, R10.1).
`AUDIT_PAGE_SIZE = 250` constant in `theme::layout`. Cockpit IA forbids
surfaces without operator-stated need; fixed 250 keeps the SQL `LIMIT`
constant, snapshot baselines deterministic, and the call-site one
line shorter. Operator-configurable chip selectors and infinite-scroll
were both rejected.

**Q5 — Audit filter persistence: in-session only** (R9.4, R10.1).
Filter state on `Cockpit::audit_screen_state.filter`; cleared on
restart. No `~/.cockpit-state.json`; no `serde::Serialize`; no `Drop`
impl. Matches Phase 2 Q8's "the cockpit is an instrument, not a
browser" + the principles-doc session-scoped persistence rule.

**Q6 — Strategies-detail equity sparkline: defer to Phase 4**
(R6.1–R6.4). Design-pass measurement: `Cockpit::pnl: PanelState<PnlSnapshot>`
is a single snapshot, not a historical buffer; there is no
per-strategy history field on `Cockpit` today. Wiring a 60-bar
per-strategy buffer requires either (a) a new bus subscription on
top of `pnl_by_strategy(ledger, strategy_id, since, until)` ticked
at bar-close, or (b) a one-shot fetch on chip-select — both cost
more than the 50-LOC "cheap path" budget the analyst named. Phase
3 ships the deferred placeholder copy
(`STRATEGIES_SPARKLINE_DEFERRED`) top-right of the Strategies
screen; the snapshot baseline locks the deferral so Phase 4 has a
clear "this is the seam" target. Phase 4 (Backtest) already needs
the same equity-history primitive.

**Q7 — Audit-query method shape: add a sibling
`recent_journal_filtered`** (R12.1–R12.6). Fills predicate scans
`journal_transactions` filtered by description-prefix regex
(`description LIKE 'buy %' OR LIKE 'sell %'`); non-fill rows scan
`strategy_events` (LEFT JOIN on `transaction_id`) and reconciliation
rows. Different table set, different join shape; cramming both into
one method either ships two SQL paths inside one function (code-smell)
or unifies the queries onto a single rows view (premature). Two
siblings is honest about the data shape diverging. Signature:
`(ledger, venues: &[Venue], symbol: Option<&Symbol>, kind:
AuditKindFilter, since, until, page_offset, page_size) ->
Result<(Vec<JournalRow>, u64), LedgerError>` — returns the page rows
+ total count so the screen header can render "Showing N–M of T"
without a separate `COUNT(*)` round-trip. Empty venue set ↔ all
venues; symbol `None` ↔ all symbols; `kind == All` ↔ all kinds.
Empty result returns `Ok((vec![], 0))`; never `Err` for "no rows".

**Q8 — Sidebar entry insertion order: master-roadmap order** (R1.1).
`Home → Debug → Strategies → Risk → Audit → Charts`. Trading data
first, ops chrome second, detail screens third, cross-check chart
last. The widget body at `crates/ui/src/widgets/sidebar_nav.rs:48`
is unchanged — Phase 2 R1.6 parameterised `entries: &[Screen]`;
Phase 3 swaps `SIDEBAR_ENTRIES_PHASE_2` for `SIDEBAR_ENTRIES_PHASE_3`
(constant-only diff). The `label_for(Screen)` match arm and the six
`SIDEBAR_NAV_*` strings already ship from Phase 2 declare-now;
**no `ui::strings` rewrite** (operator-locked Constraint 2).

**Q9 — Risk kill-threshold gauge style: horizontal bar** (R7.2–R7.3).
Visual consistency with the per-venue exposure section + daily-loss
section (both horizontal bars) and Phase 1's `theme::color_for_latency_ms`
colour ramp (`ACCENT` < 70 %, `WARN_500` ≥ 70 %, `DOWN_500` ≥ 90 %).
A new `frame::threshold_bar(used: Decimal, cap: Decimal, mode:
ThemeMode)` helper in `crates/ui/src/widgets/frame.rs` (additive,
sibling of Phase 1 `active_row` and Phase 2 `active_chip`) renders
the bar. Radial dials add a new chart primitive Phase 3 doesn't
otherwise need; numeric-only loses the at-a-glance signal.

**Q10 — Per-strategy params + risk caps: read-only** (R4.4, R7.5).
Matches `spec/product.md` § Cockpit IA → "`config/agent.toml` is
hand-edited; the cockpit never writes config. (Risk and execution-
mode toggles in Phase 5 are exceptions ratified there.)" Phase 3
holds the line; Phase 5 HumanControl ratifies the operator-write
exceptions. No edit / pause / deploy / "raise the limit" buttons on
Strategies or Risk screens.

**Q11 — Snapshot ripple budget + cross-link Message variant: ~13
baselines, compound dispatch** (R5.2, R5.5). Q11a — ripple ≈ 13:
~3 per detail screen × 3 = 9 net-new + 3 sidebar variants
(`active_strategies / active_risk / active_audit`) + 1 refreshed
Phase 2 sidebar (`sidebar_nav__three_entries` → `_six_entries`).
Single `cargo insta accept` pass per Phase 1 Q2 / Phase 2 V11
precedent. Q11b — compound dispatch: Phase 2 R8.2 established the
pattern (chip-select uses `SelectSymbol` plus binary-side
`Task::perform`); reusing it keeps the `Message` enum smaller. The
Home → Strategies-summary row click emits
`Message::SelectStrategy(id)`; the binary's wiring chains
`Task::done(Message::SwitchScreen(Screen::Strategies))` only when
`current_screen != Strategies`. No new `OpenStrategy` variant.

**TD-1 re-evaluation (Phase 3 design pass).** Verified at design
pass on disk: `crates/ui/Cargo.toml:52` reads
`iced = { version = "=0.14.0", default-features = false, features =
["tiny-skia", "thread-pool", "advanced", "canvas"] }`. iced 0.15+
has not landed; the `button::Status::Focused` variant and
`text_input::Style.shadow` field are **not** available. **Phase 3
ships no focus-ring upgrade.** Phase 1's deferred state holds.
Operator-impact bound is unchanged — kill-switch destructive flow
typed-confirm gated, focus halo a secondary signal. Named upgrade
trigger unchanged — any iced version bump exposing both fields
promotes the ~30-line one-file sweep across `widgets/kill.rs` and
`widgets/journal_transaction_modal.rs`. **Next re-evaluation: Phase
4 (Backtest panel) analyst kickoff.** If iced upstream stalls
through Phase 4, re-evaluate at Phase 5 (HumanControl) — the new
operator-write controls there sharpen the cost/benefit on the
custom-widget escape-hatch path. The TD-1 row in the master roadmap
should be appended with a 2026-05-05 line under "Promotion timing"
noting the Phase 3 design verification — that's a follow-up the
orchestrator routes to the analyst on Phase 3 ship (architect does
not edit the master roadmap directly).

**Cross-feature invariants preserved.** All 7 prior shipped features
([master roadmap cross-feature invariants
table](../lumen-design-adoption/feature.md#cross-feature-invariants))
remain green post-Phase 3 — see the Phase 3 brief's "Cross-feature
invariants" sub-section for the row-by-row preservation note. Notable
delta: the `tape-row-audit-modal` invariant gains the Audit screen as
a second host — the modal trigger flow is identical to Home (literal
reuse of `Message::TapeRowClicked(tx_id)`, no new variant).

**Anchor budget.** Zero touched. UI screens + read-only audit query
addition + additive schema migration with constant-string backfill.
`crates/strategy/`, `crates/cost/`, `crates/backtest/`,
`crates/reports/` unchanged. The 11 backtest body-SHA-256 anchors in
[`spec/anchors.toml`](../anchors.toml) verify byte-identical post-Phase
3. The `008_journal_transactions_venue.sql` migration is **additive**:
`ADD COLUMN … DEFAULT 'Binance'`. SQLite's column-add is a schema
change, not a row-body rewrite; existing rows' description / amount /
strategy_id bytes are untouched. The `recent_journal_filtered` query
is read-only over the same `journal_transactions` rows that
`recent_journal` and `recent_fills_filtered` already iterate; cannot
alter any committed report body by construction.

**Library compatibility checklist.**

- iced still pinned `=0.14.0` (`crates/ui/Cargo.toml:52`); no new
  iced version, no new dep. **Q11 deferral verified on disk;
  re-evaluation deferred to Phase 4.**
- No new dep — the `RiskTelemetry` channel reuses the existing
  `tokio::sync::broadcast` shape from `MarketHealth` /
  `publish_market_health`; the audit query addition uses the
  existing `sqlx` surface.
- `rust_decimal::Decimal` covers all numeric fields on `RiskState`;
  no `f64`. Money math discipline preserved per architect.md
  determinism guardrails.
- Lucide icons explicitly out of scope (master Constraint).

**Tasks.** `T1701–T1716` + `T_FINAL_LUMEN_PHASE_3` filed at
[tasks/lumen-phase-3-detail-screens.md](../lumen-phase-3-detail-screens/tasks.md).
T1701 is the foundation gate (state additions). T1702 (`008`
migration + writer wiring), T1703 (sidebar swap), T1707
(`RiskTelemetry` channel), T1709 (audit filter row), and T1712
(`recent_journal_filtered`) all fan out from T1701 in parallel.
T1704–T1706 (Strategies-detail), T1708 (Risk screen), and
T1710–T1711 (Audit screen body + modal reuse) complete the visual
surface. T1713 (snapshot accept + ui-designer attestation
sub-block) is the narrow point. T1714 / T1715 / T1716 close out
before the tester gate.

##### Q1–Q12 ratification (Phase 4, confirmed 2026-05-06)

Architect's Phase 4 design landing for the
[lumen-design-adoption](../lumen-design-adoption/feature.md) initiative,
ratifying the analyst's brief at
[lumen-phase-4-backtest-panel](../lumen-phase-4-backtest-panel/feature.md).
Phase 4 is the fourth of six sequential phases (Phases 1 / 2 / 3
shipped 2026-05-04 / 2026-05-05 / 2026-05-06; Phase 5 queued; Phase 6
reserved for the v2 LLM strategy). **12 / 12 architect Q-items
ratified; zero principled overrides.** Each Q resolution cites the
R-item(s) it ratifies; full resolution table lives in the Phase 4
brief.

**Q1 — `EquitySeries` field set: richer with precomputed drawdown**
(R10.2–R10.5). The cross-phase primitive carries `points:
Vec<EquityPoint>` where `EquityPoint = { ts, equity, drawdown_pct }`,
plus `peak`, `trough`, `max_drawdown_pct`, `inception_ts`, `as_of_ts`.
The drawdown vector lives **inside** `EquityPoint` (not a parallel
`Vec<Decimal>` per the analyst sketch — the per-point struct shape
eliminates implicit length-coupling and off-by-one risk between two
parallel vectors). Single O(N) `Decimal` walk at build time
(`EquitySeries::from_points`) computes running peak / trough /
drawdown / max-DD; consumers branchless-render straight from the
struct. Two consumers (viewer offline + cockpit sparkline online)
need the same vector — precomputing once at construction eliminates
per-render divergence risk. Minimal-shape rejected: forces every
consumer to re-implement the drawdown walk, precision-bug divergence
risk between consumers.

**Q2 — Chart-widget reuse: shared `widgets::canvas_chart` core**
(R5.1–R5.4). Phase 2's internal helpers (`draw_gridlines`,
`inner_rect`, `with_alpha` + the 5-gridline `BORDER_1 @ 0.4`
constant + `LINE_STROKE_PX` + `RANGE_PAD_FRACTION`) promote to
`pub(crate)` in a new `crates/ui/src/widgets/canvas_chart.rs`. Phase
2's existing `widgets::chart` becomes a wrapper that consumes the
core for the Charts-screen surface — **public `view` signature
byte-stable**, Phase 2 baseline byte-identical post-refactor. Phase 4
adds three sibling wrappers consuming the same core: `widgets::equity_curve`
(line + filled area), `widgets::drawdown_band` (line + filled area,
inverted Y), `widgets::sparkline` (line-only at `fill_alpha = 0.0`).
A new `widgets::canvas_chart::polyline_with_fill(frame, inner, points,
line_color, fill_color, fill_alpha)` primitive is the single drawing
function shared across all four wrappers. Copy-paste rejected on
divergence-risk grounds; re-export of internals without a refactor
rejected on visibility grounds (Phase 2's `pub(crate)` was
intentional).

**Q3 — KPI source format: stable-contract via existing markdown body
(Q3a)** (R3.1–R3.5). All 11 anchored reports already carry the six
metrics (with CAGR + Win rate marked-absent on the live samples) in
the `## Summary` table; new module `crates/reports/src/parse.rs`
houses `BacktestMetrics::parse_from_report(path)`. **Failure mode is
graceful** — if a future report-format change breaks the parser, the
viewer renders the R2.6 empty state (six `—` dashes +
`VIEWER_METRICS_UNAVAILABLE` muted-body) and continues to render the
equity curve + drawdown band + body. Sidecar `report.json` (Q3b)
rejected on cost / benefit: write-path ripple in `crates/reports`,
backfill question for past reports, two-source-of-truth divergence
between table and JSON. The equity-points contract (R11) is
independent — both options reuse the existing
`<stem>__equity.csv` companion file regardless. Anchor risk zero:
parser is read-only over committed bodies, no write-path change.

**Q4 — Operator-defined report file picker: CLI-only** (R1.2). The
viewer accepts a `clap`-parsed positional `<report-path>`; missing
arg → exit 2; non-existent path → exit 3. No file-dialog widget
surface, no recents list. Matches the v1 "single operator,
config-driven" non-goal. A future phase may add a recents list if
asked. `iced_aw` file-picker rejected — out-of-band of the
"config-driven, operator-typed paths" discipline.

**Q5 — Equity-curve large-report performance: cap at 2000 points**
(R6.3). iced 0.14 `tiny-skia` paints 2000 polyline segments
comfortably within a 16 ms frame. `EquitySeries::downsample(2000)`
enforces the cap via equal-stride bucketing (last-value-wins per
bucket); preserves `points[0]` and `points[N-1]` exactly so peak /
trough / inception / as-of survive the downsample. `Decimal`
arithmetic only; no `f64`. 90-day 1-min reports (~129 600 points)
remain deliberately out of scope — downsample at metric-emit time
upstream of the viewer.

**Q6 — Drawdown band fill: solid `DOWN_500 @ 0.18`** (R7.2). Matches
Lumen's own `Backtest.jsx:103` flat fill and the Phase 2 line-fill
style. Equity curve's `UP_500 @ 0.18` fill takes the same solid
treatment for consistency. Gradients in iced 0.14 require
`Brush::Gradient` paths — extra complexity below the operator's
"perceptible difference at workstation distance" threshold; one
fewer code path to test.

**Q7 — `equity_curve_for_strategy` signature: `(ledger, strategy_id,
since, until: Option<Timestamp>) -> Result<EquitySeries, LedgerError>`**
(R12.1–R12.6). Sibling-style consistency with Phase 3
`recent_journal_filtered` (positional timestamps, not `Range`).
`Option<until>` is explicit about "to now" semantics; the cockpit
consumer (R13.4) calls `until: None`, saving a clock read at the
call-site. Column projection: `SELECT je.ts, je.debit_amount,
je.credit_amount FROM journal_entries je JOIN journal_transactions jt
ON je.transaction_id = jt.id WHERE je.account_id =
'income:realized_pnl' AND jt.strategy_id = ? AND je.ts >= ? AND je.ts
< ? ORDER BY je.ts ASC, je.id ASC` — same row set as `pnl_by_strategy`,
emitted as a vector of running-equity samples (`running += cr - dr`
walk seeded from the existing `cash_balance(&ledger)` baseline)
rather than a single aggregate. Returns `Err(LedgerError::EmptyWindow)`
on zero rows so the cockpit consumer can render the R13.8 empty state
without inspecting `Ok(EquitySeries)` for `points.is_empty()` —
keeps the `EquitySeries::from_points` `Empty` invariant load-bearing.
Determinism: `ORDER BY` ties broken on `je.id ASC`; `Decimal`
arithmetic only. Read-only sibling; `pnl_by_strategy` unchanged.
`Range<Timestamp>` rejected — inconsistent with sibling-method style;
`until: Timestamp` no-`Option` rejected — forces caller-side clock
read.

**Q8 — Strategies-detail sparkline placement: above (top-right of
the chip row)** (R13.1, R13.2). Matches the Phase 3 deferred-
placeholder slot at `crates/ui/src/screens/strategies.rs:135`. Same
160 px-wide `Container`, same scan position; the Phase 4 change is
"the placeholder retires; the canvas widget lands". Right-of-params
rejected (narrow-width regression); bottom rejected (low in scan
order for an "is this working" signal).

**Q9 — Cockpit sparkline render budget: cap + downsample at fetch,
no live update** (R13.5–R13.6). `SPARKLINE_POINT_CAP = 120` (in
`theme::layout` next to Phase 3's `AUDIT_PAGE_SIZE`); the fetched
series goes through `EquitySeries::downsample(120)` before landing
on `Cockpit::strategy_equity`. No `Subscription::batch` recipe —
refresh is one-shot on `Message::SelectStrategy(id)` (Phase 3 Q11b
compound-dispatch) firing a `Task::perform(audit::query::equity_curve_for_strategy(...))`.
Live-rebuild rejected — couples render rate to ledger write rate,
violates the "screens are pure render dispatches" invariant
established in Phase 3 Q3. Future phase may add a 1-Hz live recipe;
not in Phase 4.

**Q10 — Viewer dark-default cold-start** (R1.1). Inherits cockpit's
dark default. Phase 1 `theme::ThemeMode::Dark` is the cold-start;
the viewer threads the same default through `ViewerModel::new()`. No
OS-detection plumbing; long-session-at-a-desk operator context fits
dark. Light default rejected (inconsistent with cockpit); `dark-light`
crate detection rejected (new dep for marginal value).

**Q11 — Snapshot baseline budget: 5 net-new + 1 deletion, single
`cargo insta accept` pass** (all visual R-items). `viewer__kpi_strip__sample_report.snap`
+ `viewer__equity_curve__sample_report.snap` + `viewer__drawdown_band__sample_report.snap`
+ `viewer__full_view__sample_report.snap` (the 4 viewer-bin
baselines) + `strategies_screen__sparkline_present.snap` (replaces
the deferred placeholder). `strategies_screen__sparkline_deferred.snap`
retires (deleted in same commit). Phase 1 / 2 / 3 baselines stay
byte-identical (viewer is a separate bin; the cockpit-side change
is local to the sparkline placement in the existing 160 px slot).
Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 single-pass precedent
preserved. Staged review (KPI first, curve next, band next)
rejected — three review passes for tightly-coupled visuals is
overhead without value.

**Q12 — `EquitySeries` module placement: new module
`crates/core/src/equity_series.rs`** (R10.1, R10.6). The type carries
non-trivial constructor logic (drawdown walk + peak / trough);
`views.rs` is a "thin DTOs" file by convention. The new module
co-locates `EquitySeries` + `EquityPoint` + `EquitySeriesError` +
`BacktestMetrics` (the architect groups `BacktestMetrics` with
`EquitySeries` rather than placing it at module root — both Phase 4
cross-phase primitives travel together to consumers). `crates/core/src/lib.rs`
re-exports. Test module has space for the seven mandatory unit tests
without crowding. Sibling-in-`views.rs` rejected (logical-group
mismatch — `views.rs` is read-side projections, not computed
aggregates).

**TD-1 re-evaluation (Phase 4 design pass).** Verified at design
pass on disk: `crates/ui/Cargo.toml:52` reads
`iced = { version = "=0.14.0", default-features = false, features =
["tiny-skia", "thread-pool", "advanced", "canvas"] }`. iced 0.15+
has not landed; the `button::Status::Focused` variant and
`text_input::Style.shadow` field are **not** available. **Phase 4
ships no focus-ring upgrade.** The viewer is a **zero-button surface**
(R14.1 / R14.2 — no "Deploy live", no "Export", no file-picker) and
the cockpit-side Strategies-detail change is a sparkline canvas
(non-focusable, no destructive action), so the Phase 4 deliverable
adds zero new focus-ring exposure. Phase 1's bounded approximation
holds. Operator-impact bound is unchanged: kill-switch destructive
flow remains typed-confirm gated, focus halo a secondary signal.
**Next re-evaluation: Phase 5 (HumanControl) analyst kickoff.** Phase
5 introduces the first new operator-write controls (pause-strategy,
override-risk-veto) where the focus-ring ergonomic gap sharpens —
architect re-evaluates the cost / benefit on the custom-widget
escape-hatch path at that point. The TD-1 row in the master roadmap
should be appended with a 2026-05-06 line under "Promotion timing"
noting the Phase 4 design verification — that's a follow-up the
orchestrator routes to the analyst on Phase 4 ship (architect does
not edit the master roadmap directly).

**Cross-feature invariants preserved.** All 7 prior shipped features
([master roadmap cross-feature invariants
table](../lumen-design-adoption/feature.md#cross-feature-invariants))
remain green post-Phase 4 — see the Phase 4 brief's "Cross-feature
invariants" sub-section for the row-by-row preservation note. Notable
delta: the `v1.5b-multi-venue` invariant gains a passive note — the
new `equity_curve_for_strategy` SQL has no `venue` predicate (equity
is per-strategy, not per-venue), so v1.5b plumbing-only state remains
untouched and the Phase 3 `008_journal_transactions_venue.sql`
column is read-only present on the row set, not required by the
query.

**Anchor budget.** Zero touched. Read-only over committed reports +
read-only audit query addition + UI-only screens. `crates/strategy/`,
`crates/cost/`, `crates/backtest/`, `crates/reports/src/render/`
unchanged. The 11 backtest body-SHA-256 anchors in
[`spec/anchors.toml`](../anchors.toml) verify byte-identical post-Phase
4. The KPI-strip parser at `crates/reports/src/parse.rs` is read-only
over the existing markdown bodies; the viewer's equity-curve
companion-CSV reader is read-only over the existing
`<stem>__equity.csv` files (`EquitySample` row type unchanged); the
audit query addition is read-only over committed `journal_entries`
rows. The viewer is read-only on the spec tree
([architecture.md:3116](#viewer--specreports)) — build-time test in
`crates/ui/tests/viewer_read_only.rs` asserts the bin declares no
`File::create` / `tokio::fs::write` against `spec/**`.

**Library compatibility checklist.**

- iced still pinned `=0.14.0` (`crates/ui/Cargo.toml:52`); no new
  iced version, no new dep. **Q11 (TD-1) deferral verified on disk;
  re-evaluation deferred to Phase 5.**
- No new dep — the `widgets::canvas_chart` core extraction is a
  pure refactor of Phase 2's existing helpers; the new
  `polyline_with_fill` primitive uses the existing
  `iced::widget::canvas` surface; `BacktestMetrics::parse_from_report`
  uses `Decimal::from_str` + standard string slicing; the audit query
  addition uses the existing `sqlx` surface; the viewer body
  renderer is ~30 LOC of in-module heading-pre-pass (no
  `pulldown-cmark`).
- `rust_decimal::Decimal` covers all numeric fields on
  `BacktestMetrics` + `EquityPoint::drawdown_pct` + `EquitySeries::max_drawdown_pct`;
  no `f64`. Money math discipline preserved per architect.md
  determinism guardrails.
- Lucide icons explicitly out of scope (master Constraint).

**App-layout table updated.** The `viewer` row at
[architecture.md:2947–2951](#app-layout) is updated to reflect the
Phase 4 deliverable shape: window contract becomes "Backtest report
shell · KPI strip + equity curve + drawdown band + markdown body
(Phase 4 — shipped)"; data source becomes "`spec/reports/` markdown
+ `<stem>__equity.csv` companion" (the viewer reads both the body
markdown for the KPI parser + the `EquitySample` companion CSV for
the curve / band; the audit query addition is for the cockpit-side
sparkline consumer, not the viewer).

**Tasks.** `T1801–T1815` + `T_FINAL_LUMEN_PHASE_4` filed at
[tasks/lumen-phase-4-backtest-panel.md](../lumen-phase-4-backtest-panel/tasks.md).
T1801 is the foundation gate (`core::EquitySeries` + `BacktestMetrics`
+ `Cockpit::strategy_equity` field + `Message::StrategyEquityRefreshed`
variant). T1802 (audit query), T1803 (viewer skeleton), T1804
(canvas-chart core extraction), and T1808 (reports parser) all fan
out from T1801 in parallel. T1805–T1807 + T1809 (the four widget
modules — KPI strip, equity curve, drawdown band, sparkline) share
T1804's canvas-chart core. T1810 (viewer composition) gates on the
four widgets + the parser. T1811 (cockpit Strategies-detail
sparkline replacement; closes the Phase 3 Q6 deferral) gates on
T1802 + T1809. T1812 (snapshot accept + ui-designer attestation
sub-block) is the narrow point. T1813–T1815 close out before the
tester gate.

##### Q1–Q15 ratification (Phase 5, confirmed 2026-05-06)

Architect's Phase 5 design landing for the
[lumen-design-adoption](../lumen-design-adoption/feature.md) initiative,
ratifying the analyst's brief at
[lumen-phase-5-humancontrol-agentfeed](../lumen-phase-5-humancontrol-agentfeed/feature.md).
Phase 5 is the fifth of six sequential phases (Phases 1–4 shipped
2026-05-04 / 2026-05-05 / 2026-05-06 / 2026-05-06; Phase 6 reserved
for the v2 LLM strategy). **15 / 15 architect Q-items ratified; zero
principled overrides on substance.** Q5 (TD-1) carries a load-bearing
concrete commitment grounded in the on-disk iced version verification
below. Each Q resolution cites the R-item(s) it ratifies; full
resolution table lives in the Phase 5 brief.

**Q1 — HumanControl panel placement: 7th sidebar entry**
(R1.3, R2.2). HumanControl lives as `Screen::Control`, a 7th sidebar
entry after the existing six (Home / Debug / Charts / Strategies /
Risk / Audit). Consistency with the Phase 2 / 3 IA (every cockpit
surface is a sidebar entry); Lumen's "always-visible" framing
(`HumanControl.jsx:2`) maps cleanly onto a persistent sidebar entry.
The Phase 2 R1.6 sidebar widget API is parameterised — absorbing a
7th entry is an additive `entries.push(...)` in the binary's sidebar
build. Implication: the Debug-screen kill placement migrates into
HumanControl as the bottom action via a new
`widgets::kill::view_inner` body-extraction helper (the public
`widgets::kill::view` retains its current shape per R2.3); the
Debug-screen kill row retires (one regenerated baseline per Q11).
Home-screen header card rejected (breaks the four-panel grid +
hides kill behind a click); footer-panel rejected (panel is ~6–8
rows, exceeds status-bar-adjacent slot).

**Q2 / Q3 — New audit writers: `strategy_paused` + `risk_veto_overridden`**
(R5.1–R5.5, R8.1–R8.5). Two new sibling-of-`kill_switch_tripped`
functions in `crates/audit/src/journal.rs`. Operator decisions belong
in the ledger (`spec/ui-design-principles.md:282–284` — "audit ledger
is the canonical why"); compliance-bounded for the override case.
Atomic dual-write per `kill_switch_tripped` (memo row in
`journal_transactions` + `strategy_events` row in one txn). Memo `ts`
uses `Rfc3339` second precision (preserved from
`kill_switch_tripped`); `strategy_events` `ts` uses 6-digit
fractional-second format (HF-3 gate). Column projection table:

| Column          | `strategy_paused`                       | `risk_veto_overridden`              |
|-----------------|-----------------------------------------|-------------------------------------|
| `kind`          | `"StrategyPaused"`                      | `"RiskVetoOverridden"`              |
| `strategy_id`   | `Some(strategy_id.as_str())`            | `Some(strategy_id.as_str())`        |
| `error_code`    | `Some("strategy_paused")`               | `Some("risk_veto_overridden")`      |
| `error_summary` | `Some("paused")` / `Some("resumed")`    | `Some(reason)` (verbatim)           |
| `venue`         | `None`                                  | `None`                              |

`StrategyEventKind` extends with two new PascalCase variants
(`StrategyPaused`, `RiskVetoOverridden`) at
`crates/core/src/strategy_events.rs:99–113`. **No SQL migration** —
the `strategy_events.kind` column at
`crates/audit/migrations/002_strategy_events.sql` is `TEXT`. Test
scope per Q10: unit + integration + audit-row snapshot baseline (all
three). Runtime-only persistence rejected — leaves no audit trail;
bisects the principles-doc rule.

**Q4 — Execution-mode persistence: runtime-only for v1** (R10.1–R10.4).
Cold-start = `ExecutionMode::Observe` (safest default). No
`config/agent.toml` write; no audit writer (mode is prospective, not
a decision). The shipped tree has zero config-write surfaces (v0–v4
are config-driven); introducing one for session ergonomics is out of
bounds. `config/agent.toml` write rejected — bisects the
"config-driven, no UI-write-to-disk" non-goal; corruption-on-crash
risk.

**Q5 — TD-1 resolution (load-bearing): path (b) — custom-widget
escape hatch** (R13.1, R13.3, R13.5). **Verified at design pass:**
`crates/ui/Cargo.toml:69` reads `iced = { version = "=0.14.0",
default-features = false, features = ["tiny-skia", "thread-pool",
"advanced", "canvas"] }`. iced 0.15+ has not landed; neither
`button::Status::Focused` nor `text_input::Style.shadow` is available.
**Path (a) fold-in is unavailable.** Path (c) restate-with-deadline is
**rejected**: Phase 6 is gated on v2 LLM (operationally indefinite —
may take quarters); a fifth restatement is no longer viable, and
Phase 5 is exactly the moment the cost/benefit tightened (three new
operator-write surfaces — execution-mode toggle + pause-strategy +
override-risk-veto). The architect commits to **path (b)**: new
module `crates/ui/src/widgets/focus_ring.rs` implementing a
focus-state-owning wrapper that owns focus state via a
`Subscription` on `iced::keyboard::on_key_press` filtered to `Tab`
+ `ArrowDown` / `ArrowUp`, emits a synthetic
`Message::FocusChanged(WidgetId)` on focus traversal. Cockpit gains
a new `focused_widget: Option<SmolStr>` field (treated as Phase-5
internal state — no audit writer, no persistence). Focus-ring
rendering uses the existing `theme::focus::ring(mode)` token (3 px
low-alpha accent). **Consumer sites:** all four destructive surfaces
gating on focus — kill button + kill confirm input
(`widgets::kill`); override-risk-veto confirm input + cancel +
confirm buttons (`widgets::override_risk_veto`); per-strategy pause
button (`widgets::strategies::pause_button`); execution-mode segment
buttons (`widgets::human_control::mode_segment`). The four-phase
TD-1 deferral closes at Phase 5 ship.

**Q6 — `tape` → `AgentFeed` snapshot rename: rename via `git mv`**
(R11.1, R12.1, R12.4). Snapshot filenames are operator-greppable;
stale-filename → new-module mismatch breaks the convention. The 9
baselines (5 panel-states + 4 audit-modal variants) move via `git
mv` to preserve git-history continuity; the body diff is
title-string only (`PANEL_TAPE_TITLE` → `PANEL_AGENT_FEED_TITLE`).
After the moves land, a single `cargo insta accept` pass at end of
phase regenerates the body content for the title-string change.
**Not** delete-then-regenerate via `cargo insta accept` over deleted
baselines — that path loses git history and bloats the diff for
review.

**Q7 — HumanControl panel field set: full Lumen set (mode + 3 limits
+ kill)** (R3.1–R3.5). All three limit fields read from existing
`Cockpit::risk_state` + `Cockpit::pnl` (no new backend wiring).
Daily-loss reads `risk_state.daily_loss_cap_pct`; max-position
derives from `risk_state.per_symbol_caps`; used-today reads from
`Cockpit::pnl` with sentiment colouring per
`widgets::pnl::color_for_delta` (R14.3 — helper signature unchanged;
read-only consumption). Trimmed (mode + kill only) rejected —
hides the daily-loss-limit context the principles doc calls out as
load-bearing.

**Q8 — Pause-strategy resume semantics: single-click resume** (R4.4,
R6.1). Pause is bounded-destructive (skips future signals; doesn't
reverse past decisions); resume returns to default state — principles-
doc "undo where physically possible" case
(`spec/ui-design-principles.md:275–278`). Typed-confirm both sides
rejected — friction without proportional safety value.

**Q9 — Override-risk-veto scope: per-veto override** (R7.3). One
button per surfaced `VetoEvent`, not per strategy. **Forward-only**
— the veto is dismissed + audit row recorded; the agent does NOT
re-emit the blocked signal. Per-strategy override rejected — too
broad ("disable risk-engine for this strategy" is exactly what the
engine exists to prevent); loses per-decision audit trail.

**Q10 — Audit-writer test scope: unit + integration + audit-row
snapshot baseline (all three)** (R5, R8). Unit tests cover the
dual-write contract (sibling of `kill_switch_tripped` tests);
integration tests cover cockpit → bus → writer wiring; snapshot
baselines lock the audit row format
(`strategy_events__strategy_paused_row.snap`,
`strategy_events__risk_veto_overridden_row.snap`). Unit-only
rejected — leaves cockpit-side wiring untested; cockpit ↔ writer
is the load-bearing seam.

**Q11 — Snapshot baseline budget: ~9 rename pairs + ~12 net-new + 1
Q1-driven Debug regen + 1 focus-ring net-new; single `cargo insta
accept` pass** (R12, V12). Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 /
Phase 4 V12 single-pass precedent. Q1's 7th-sidebar-entry pick → kill
migrates from Debug-screen → Debug-screen baseline regenerates (1
row); Home stays byte-identical. Net-new: 4 HumanControl mode
baselines (observe/supervised/auto + kill armed) + 2 HumanControl
limits (loading/error) + 2 Strategies-screen pause baselines + 3
Strategies-screen override baselines (button idle + 2 modal states)
+ 1 focus-ring (focused kill button) + 2 audit-row baselines under
`crates/audit/tests/snapshots/`. Staged review rejected — three
review passes for tightly-coupled visuals is overhead without value.

**Q12 — Kill button copy in HumanControl: preserve "Stop trading"**
(R2.4). Master Constraint 2 (no voice rewrite) + principles-doc
"exact phrase not negotiable mid-session"
(`spec/ui-design-principles.md:391–393`). Lumen `"Halt all agents"`
rejected.

**Q13 — Risk-engine veto-emit wiring: placeholder feed in Phase 5;
defer real upstream wiring** (R7.2). Phase 5 ships the cockpit-side
override flow (typed-confirm + audit writer + clear-from-list) over
`Cockpit::risk_veto_events: Vec<VetoEvent>` populated by fixtures;
live emits empty `Vec`. **The deferred upstream wiring tracks as a
new `TD-2` row** in the master roadmap's Cross-phase technical-debt
items section (architect flags for orchestrator to append on Phase
5 ship — architect does not edit master roadmap directly):

> **TD-2 — Risk-engine veto-emit upstream wiring (Phase 5 Q13
> deferral, ratified 2026-05-06).** The agent runtime's
> `default_risk_telemetry_stub` at `crates/agent/src/runtime.rs:1023–1090`
> does not emit `VetoEvent`s upstream of the cockpit. Phase 5 ships
> the operator-side surface over a placeholder; live override
> surface is empty (no surfaced vetoes → no overrides possible) but
> safety primary is preserved (risk engine still vetoes upstream of
> the executor). Promotion timing: Phase 6 (Assistant slot) if v2
> LLM lands first, else a standalone backend brief.

Wire-full-pipeline rejected — couples Phase 5 ship to a larger
backend refactor; Phase 5's in-scope is the operator-facing override
surface.

**Q14 — `Cockpit::tape` field rename: preserve `Cockpit::tape`
field name** (R11.4). Field is referenced by every Phase 1–4 test
fixture and the `tape-row-audit-modal` modal-trigger import path;
rename would ripple through ~100+ test sites for cosmetic value.
Phase 5 is module rename, not state-shape rename. Mismatch documented
via a code-comment annotation on the field pointing at the
`widgets::agent_feed` module path. Rename-the-field rejected —
disproportionate test ripple; out of Phase 5 scope.

**Q15 — Audit-query reader for "recent operator writes": NO — defer**
(none). New `StrategyPaused` / `RiskVetoOverridden` rows are
queryable via Phase 3's `recent_journal_filtered` (with `kind`
filtering); Phase 3's Audit screen is the canonical surface for
`strategy_events`. A dedicated "recent operator activity" panel is
a separate future brief; not Phase 5 scope. Add-a-reader rejected —
scope creep.

**TD-1 closure (Phase 5 design pass).** Verified at design pass on
disk: `crates/ui/Cargo.toml:69` reads `iced = "=0.14.0"`. iced 0.15+
has not landed; the `button::Status::Focused` variant and
`text_input::Style.shadow` field are **not** available. Path (a)
fold-in unavailable; path (c) restate-with-deadline rejected (Phase
6 v2-LLM gated, operationally indefinite); **Phase 5 closes the
four-phase deferral via path (b) — custom-widget escape hatch** at
`crates/ui/src/widgets/focus_ring.rs`. The TD-1 row in the master
roadmap should be appended with a 2026-05-06 closure note —
architect flags for orchestrator to route to the analyst on Phase 5
ship (architect does not edit the master roadmap directly).

**Cross-feature invariants preserved.** All 7 prior shipped features
remain green post-Phase 5 — see the Phase 5 brief's "Cross-feature
invariants" sub-section for the row-by-row preservation note.
Notable delta: the `tape-row-audit-modal` invariant is preserved
because the `Cockpit::tape` field name stays (Q14); the modal
trigger reads from the same field. The `live-cockpit-unified`
invariant gains two additive `EventBus` channels (`pause_strategy_tx`
+ `execution_mode_tx`) — additive, no existing channel touched.

**Anchor budget.** Zero touched. The two new audit writers are
**additive** — new `StrategyEventKind` enum variants + sibling
functions following the `kill_switch_tripped` pattern verbatim. No
existing row's body is altered; `kind` column is `TEXT` so no
schema migration. No new backtest scenarios; no committed report
body re-renders. The `tape` → `agent_feed` rename is module-path +
snapshot-filename + title-string only — no committed report
references the `tape` widget module. The 11 backtest body-SHA-256
anchors in [`spec/anchors.toml`](../anchors.toml) verify byte-identical
post-Phase 5.

**Library compatibility checklist.**

- iced still pinned `=0.14.0` (`crates/ui/Cargo.toml:69`); no new
  iced version, no new workspace dep. **TD-1 resolved via custom-
  widget escape hatch (path b), not version bump.**
- No new dep — the `widgets::focus_ring`, `widgets::human_control`,
  and `widgets::override_risk_veto` modules use only the existing
  `iced::widget` + `iced::keyboard` surfaces. The two new audit
  writers use the existing `sqlx` + `time` + `uuid` surfaces.
- `rust_decimal::Decimal` is unchanged in this phase; no money math
  surfaces extend.
- Lucide icons explicitly out of scope (master Constraint).

**Tasks.** `T1901–T1916` + `T_FINAL_LUMEN_PHASE_5` filed at
[tasks/lumen-phase-5-humancontrol-agentfeed.md](../lumen-phase-5-humancontrol-agentfeed/tasks.md).
T1901 is the foundation gate (Cockpit state additions). After T1901,
**five** tasks fan out in parallel: T1902 (audit writers — separate
crate, no UI dep), T1903 (`tape` → `agent_feed` rename via `git mv`
— mechanical, reviewable in isolation), T1904 (HumanControl
skeleton), T1909 (override modal skeleton), T1912 (focus-ring widget
— TD-1 path b). T1905 / T1906 / T1911 share T1904's HumanControl
skeleton + T1912's focus-ring wrapper. T1907 / T1908 share T1902's
audit writers + T1912's focus-ring wrapper. T1910 shares T1909's
modal + T1912's focus-ring + T1902's audit writer. T1913 (snapshot
accept) is the narrow point. T1914–T1916 close out before the tester
gate.



## Changelog
- 2026-05-16 (architect): D2 — strengthened the UI isolation rule
  in § App layout. Prior wording said "Both cockpit binaries live
  in the `ui` crate and depend only on `core` / `audit` / `agent` —
  never on `strategy`, `exec`, or `models`." Replaced with an
  unambiguous, carveout-free statement: the `ui` crate (lib + every
  binary target) never depends on `strategy`, `exec`, `models`, or
  `llm`; bootstrap of those types happens in `agent`, which threads
  already-constructed `Arc<…>` handles into the UI via
  `RunHandles`. Prepares for the developer-driven revert of the
  `strategy::StrategyRegistry::new()` construction site at
  `crates/ui/src/bin/cockpit_live.rs` (the code change itself is
  the developer's, not this architect pass).
- 2026-05-13 (architect / ui-designer): content migrated from
  `spec/architecture.md` § Foundation libraries — Frontend — iced
  during Phase 1A Session 11. The "why iced" decision was extracted
  to [ADR-0023](adr/0023-iced-frontend.md) alongside this body move.
  The chart-canvas custom-Canvas-vs-plotters spike outcome (custom
  Canvas wins) is recorded in [ADR-0020](adr/0020-chart-buy-sell-emphasis.md).
