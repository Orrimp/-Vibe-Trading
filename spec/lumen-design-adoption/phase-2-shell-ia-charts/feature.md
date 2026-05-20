---
slug: lumen-phase-2-shell-ia-charts
status: shipped
owner: architect
updated: 2026-05-05
version: 2.1.0
<!-- last-edited: 2026-05-05 (tester): status active → shipped on T_FINAL_LUMEN_PHASE_2 PASS. All 8 gates green first-pass; report `spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`. HANDOFF → presenter. -->
<!-- last-edited: 2026-05-04 (architect): appended `## Design` resolving Q1–Q11 (11/11 ratified, zero deviations). Cockpit state diff (Screen enum × 6, ChartBuffer cap 60, three new Message variants); sidebar nav widget contract; chart widget contract (canvas, line series, single-symbol); recent_fills_filtered (since: Timestamp, until: Timestamp); synthetic_candles per-symbol seed via DefaultHasher; right-rail Length::Fixed(0.0). TD-1 deferred — iced still =0.14.0 on disk. Task list at spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md (T1601–T1616 + T_FINAL). HANDOFF → developer ‖ ui-designer. -->
---

# Lumen Phase 2 — Shell IA + Charts (sidebar nav · Home/Debug/Charts screens · price chart)

> **Phase 2 of 6** in the
> [`lumen-design-adoption`](../feature.md) initiative.
> Master roadmap is the orientation; this brief is the **shippable
> feature**. Operator-locked constraints (no brand, no voice rewrite,
> sequential phases, Phase 6 reserved, no icons until needed) are
> documented in the master file and apply here without re-litigation.
>
> **Operator-locked decisions inherited from the 2026-05-04 master
> revision (Q11–Q14) — not re-opened in this brief:**
>
> - **Q11** — sidebar nav primacy = **fixed-width** (~180 px, always
>   visible, text-labelled, no icons).
> - **Q12** — chart data source = **both modes**. Live ticks roll
>   into the existing `bars_tx` channel; fixtures bin uses
>   deterministic synthetic candles in `ui::fixtures`.
> - **Q13** — buy/sell marker query method placement = **extend
>   [`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs)**
>   (additive, not a new module). Working name
>   `recent_fills_filtered(venue, symbol, time_range)`. Architect
>   ratifies the exact signature at design (see Q4 below).
> - **Q14** — Phase 2 vs Phase 3 split = **kept**. Phase 2 ships
>   sidebar + Home + Debug + Charts; Phase 3 ships Strategies / Risk
>   / Audit detail screens.
>
> The brief expands on the post-Phase-1 roadmap revision dated
> 2026-05-04. Phase 1 (Foundation) shipped 2026-05-04 with tester
> third-pass `VERDICT → PASS`; Phase 2 inherits Phase 1's tokens,
> tiers, T1507 active-row pattern, and the always-visible status bar.

## Why

The Phase 1 cockpit ships a **single-page** layout: pnl + positions
+ strategies + tape + kill + status bar, all visible together. As
the operator pointed out at the 2026-05-04 session, this surface
has two problems:

1. **No information hierarchy beyond panel size.** Operations chrome
   (kill switch, latency badge, market-health detail) shares the
   primary scan with trading data (PnL, positions, strategies).
   The operator wants the trading view "clean" and the operations
   view "available but separate".
2. **No way to look at one symbol's chart with the trades on it.**
   The audit ledger has every fill; the cockpit has every fill in
   a tape; but the operator's natural cross-check — "did the
   strategy buy at the low or the high of this candle?" — has no
   surface today.

Phase 2 closes both gaps with a **left sidebar nav** that splits
the cockpit into three starter screens (Home / Debug / Charts) and
a **per-symbol price chart with buy/sell markers** as the new
third screen. The chart is a read-only cross-check surface: it
shows what the audit ledger says happened against the price the
market printed, with no order-entry, drawing tools, or annotations
([`spec/ui-design-principles.md`](../../ui-design-principles.md) §
Charts — price plot with audit-anchored markers).

## Scope (high-level)

Phase 2 ships, in one merge:

- A new **sidebar nav widget** (R1) and a **screen-routed shell**
  (R2–R3) that wraps both bins.
- The existing Phase 1 widgets re-housed onto a **Home screen**
  (R4) and a **Debug screen** (R5) — no widget code changes
  beyond their composition under the new shell.
- A new **`widgets::chart`** widget on a **Charts screen** (R6–R9)
  with a chip-row symbol selector, a canvas-rendered price plot,
  and buy/sell markers from the audit ledger.
- A **per-`(venue, symbol)` rolling buffer** on `Cockpit` (R10) fed
  by the existing `bars_tx` channel in live mode and by
  deterministic synthetic candles in fixtures mode (R11).
- An **additive `audit::query::recent_fills_filtered`** method (R12)
  consumed by the chart for marker rendering.
- A **right-rail track reservation** (R13) in the shell grid for
  the Phase 6 Assistant slot (zero-width until v2 LLM ships).
- Cross-feature invariant preservation (R14) and a 11/11 anchor
  regression PASS (R15).

R-items are grouped: R1–R3 shell + nav, R4–R5 Home + Debug
screens, R6–R9 Charts screen + chart widget, R10–R11 chart data
sources, R12 audit query extension, R13 right-rail reservation,
R14–R15 invariants + anchors.

## Anchor risk

**Zero. State this loudly.** Phase 2 is purely additive over
committed audit data + UI shell + a new read-only widget:

- `recent_fills_filtered` is a generalisation of the existing
  `recent_fills(limit)` ([`crates/audit/src/query.rs:134`](../../../crates/audit/src/query.rs))
  — same description-prefixed-rows scan, narrower predicate. It
  does not alter committed report bodies, does not introduce a
  new report-rendering path, does not write the ledger.
- Both screen-routing and the chart widget are UI-shell additions;
  no strategy, exec, or backtest code path is touched.
- The 11/11 anchor regression goal stays **byte-identical** at
  the Phase 2 tester gate. No re-lock budget. No exceptions.

## Snapshot ripple

Expected: ~36 existing baselines refresh once (every widget moves
from a single-page layout to a screen-routed shell, so each
baseline's surrounding chrome differs — sidebar present, screen
body padding shifted) + ~5 net-new baselines (sidebar + chart
empty state + chart with markers + symbol-selector chip-row +
Debug screen body) for a total of ≈ 41 baselines, accepted in one
`cargo insta review` pass per Phase 1 Q2 precedent.

## Requirements

Numbered, testable, derived from the master roadmap's Phase 2
scope, the architecture-level **Cockpit screen routing (Phase 2+
contract)** in [architecture.md § 3272](../../architecture.md), and
the **Charts** + **Information architecture** sections of
[`spec/ui-design-principles.md`](../../ui-design-principles.md). Each
R-item ends with a one-line acceptance the tester verifies. Every
R-item preserves the operator-locked constraints (no brand, no
voice rewrite, no icons, sequential phases) and the cross-feature
invariants in the
[master roadmap](../feature.md#cross-feature-invariants).

### R1 — Sidebar nav widget (new)

- **R1.1** New widget `crates/ui/src/widgets/sidebar_nav.rs`. One
  file, matches the existing one-file-per-widget rule (Phase 1 Q4
  precedent).
- **R1.2** Layout: vertical column, **fixed width 180 px**
  (operator-locked Q11), `background = PANEL` (Tier 1),
  `border-right = 1 px BORDER_1`, top-padded by `space::M (12)`,
  rows separated by `space::S (8)`.
- **R1.3** Phase 2 entry set in operator scan order: **Home →
  Debug → Charts**. Each entry is a text-only label using
  `theme::text::BODY (13 px)` in `FG_2`; no icon glyphs (icons-by-
  default operator-lock; collapsibility forces icon adoption per
  the Q11 justification).
- **R1.4** Selected entry uses the **T1507 active-row pattern** — 2
  px ACCENT left rule, **no fill change** to the row
  (`crates/ui/src/widgets/positions.rs` precedent at Phase 1 R12;
  desktop.css:357–360). Hover styling = `PANEL_SUNKEN` row tint
  per Phase 1 R12.3; an actively-selected hovered row shows both.
- **R1.5** Each entry emits `Message::SwitchScreen(Screen)` on
  click. The widget is **stateless** — `current_screen` lives on
  `Cockpit`; the widget reads it as a parameter to know which row
  to draw with the active rule.
- **R1.6** Phase 3 inserts **Strategies → Risk → Audit** between
  Debug and Charts (master roadmap; architect-resolved at Phase 3).
  Phase 2's widget API must accept the entry list as a parameter
  (or read it from a typed enum the architect picks at design) so
  Phase 3 adds rows without touching the widget body.
- **Acceptance:** a `sidebar_nav_three_entries` insta snapshot
  renders the three rows; a `sidebar_nav_active_home` snapshot
  renders the same widget with the Home row carrying the 2 px
  accent rule; `cargo test -p ui sidebar_nav` PASSES.

### R2 — `Screen` enum + `current_screen` on `Cockpit`

- **R2.1** Add `pub enum Screen { Home, Debug, Charts }` to
  `crates/ui/src/state.rs`. Phase 3 extends to add `Strategies`,
  `Risk`, `Audit` (master roadmap; not in Phase 2 R-items).
- **R2.2** Add `pub current_screen: Screen` to `Cockpit` with
  `Default = Screen::Home` so cold-start lands the operator on
  trading data, not operations chrome.
- **R2.3** Add `Message::SwitchScreen(Screen)` to the `Message`
  enum in `state.rs`.
- **R2.4** `update`'s `Message::SwitchScreen(s)` arm is a **pure
  assignment** — `model.current_screen = s;` — no side effects,
  no async work, no mutation of any other field. Matches
  [`spec/ui-design-principles.md` § Information architecture →
  Screens are pure render dispatches](../../ui-design-principles.md).
- **R2.5** Sidebar nav writes `Message::SwitchScreen` only;
  **never** an audit writer, never a bus event, never an agent
  state change. Mirrors the `audit::query` one-way contract:
  the cockpit sees, the cockpit doesn't tell.
- **Acceptance:** `cargo test -p ui state::tests::switch_screen_is_pure`
  PASSES; the test calls `update` with each `Screen` variant and
  asserts only `current_screen` changed (every other field
  byte-identical via `Debug` formatting).

### R3 — Cockpit shell view dispatches on `current_screen`

- **R3.1** The cockpit shell's `view()` (today: a single
  `Column` of panels in
  `crates/ui/src/bin/cockpit.rs` and `cockpit_live.rs`) becomes:

  ```text
  Row [
      sidebar_nav::view(current_screen)         // 180 px fixed
      Column [
          screen_body(current_screen, &cockpit) // Length::Fill
          status_bar::view(&cockpit)            // 24 px fixed (Phase 1)
      ]
      <reserved right-rail track, R13>          // 0 px until Phase 6
  ]
  ```

- **R3.2** `screen_body(current_screen, &cockpit)` is a free
  function (or trivial dispatch table) that matches on
  `current_screen` and returns the appropriate screen `Element`.
  No screen "loads" anything on entry — every screen reads its
  data straight from `Cockpit` (data freshness is the bus's job;
  the screen switch is instantaneous). See
  [`spec/ui-design-principles.md` § Screens are pure render
  dispatches](../../ui-design-principles.md).
- **R3.3** Status bar continues to span the bottom of every
  screen — Phase 1's `widgets::status_bar` is unchanged. The
  halted-banner contract from
  [`live-cockpit-unified`](../../live-cockpit-unified/feature.md) renders
  **above the screen body, below the title bar** so it remains
  visible regardless of the active screen.
- **R3.4** Both binaries adopt the new shell:
  [`crates/ui/src/bin/cockpit.rs`](../../../crates/ui/src/bin/cockpit.rs)
  (fixtures) and
  [`crates/ui/src/bin/cockpit_live.rs`](../../../crates/ui/src/bin/cockpit_live.rs)
  (unified live). Same shell code in both; shell lives in a
  shared module the bins import (architect picks the module
  location at design — most likely `crates/ui/src/shell.rs` new).
- **Acceptance:** `cargo run --bin cockpit --features fixtures`
  and `cargo run --bin cockpit_live --features live` both launch
  with the sidebar visible and Home selected by default; tester
  records both runs in the Phase 2 presentation.

### R4 — Home screen composes existing Phase 1 widgets

- **R4.1** Home screen body assembles the existing four widgets in
  a 2×2 grid: PnL + Positions on the top row, Strategies (summary)
  + Tape on the bottom row. Same widget code, same panel chrome
  (Tier 1 styling per Phase 1 R10), no behavioural change.
- **R4.2** The Phase 1 single-page-shell margin / gap conventions
  (`space::M = 12 px` between panels; `space::L = 16 px` outer
  padding) carry over verbatim — only the surrounding chrome
  shifts, not the panel internals.
- **R4.3** Tape-row → audit modal (T1208 / [`tape-row-audit-modal`](../../tape-row-audit-modal/feature.md))
  trigger is preserved unchanged. The modal continues to render
  via the existing `widgets::journal_transaction_modal` and is
  reachable from the Home screen's tape rows.
- **R4.4** Cross-feature invariants preserved on Home:
  `real-mtm-unrealized-pnl` (PnL card), `per-symbol-position-accounts`
  (Positions chip), `tape-row-audit-modal` (modal trigger),
  `journal-tx-metadata` (modal header). See R14.
- **Acceptance:** a `home_screen_default` insta snapshot shows the
  four widgets in the 2×2 grid under the new shell; the existing
  per-widget snapshots refresh with the new chrome context per
  the snapshot-ripple budget.

### R5 — Debug screen collects operations chrome

- **R5.1** Debug screen body assembles the operations chrome that
  Phase 1 scattered across the single-page cockpit: kill switch
  panel, latency badge detail, per-venue market-health rows
  (read from `Cockpit::market_health` populated by the existing
  `MarketHealth` bus subscriber), server-time detail, version
  string, plus a logs/metrics output stub (text-only; structured
  metrics surface lands when a future lazy-metric infra brief
  ships — see Q9 below).
- **R5.2** The kill switch widget itself is **unchanged** —
  `crates/ui/src/widgets/kill.rs` keeps the typed-confirm phrase
  `HALT BTC` (operator-locked from
  [`live-cockpit-unified`](../../live-cockpit-unified/feature.md) +
  [`spec/ui-design-principles.md`](../../ui-design-principles.md)),
  the Phase 1 Tier 1 chrome, the Phase 1 sunken-input on the
  confirm field. **Phase 2 only changes where the widget renders,
  not how it behaves.**
- **R5.3** Per-venue market-health rendering: one row per
  `(Venue, MarketHealthState)` pair from `Cockpit::market_health`
  (Phase 1 status-bar consumes the same field — Debug surfaces
  the per-venue detail the status bar collapses into a single
  dot). Row layout: venue name, state pill (Fresh = `UP_500`
  dot + "Connected"; Stale = `WARN_500` dot + "Reconnecting"),
  last-tick-age in seconds.
- **R5.4** Server-time detail = the `Cockpit::server_time_now`
  field Phase 1 already populates via the 1 Hz `ServerTimeRecipe`
  ([`crates/ui/src/bin/cockpit.rs:76`](../../../crates/ui/src/bin/cockpit.rs)).
  Render in tabular figures via `widgets::num`. No new clock source.
- **R5.5** Latency detail surfaces the same `Cockpit::latency`
  field the Phase 1 status bar consumes; the band-name vocabulary
  reconciled at Phase 1 Q8 (OK / Slow / High / Halted) stays.
- **R5.6** Version string = `crates/ui` Cargo version + Rust
  toolchain version. Static for the session (Phase 1 R13.4
  precedent — fixtures bin uses the same value).
- **R5.7** Logs/metrics output stub: a single read-only text
  panel rendering the last N tracing events at INFO+ level (or a
  literal "Logs surface lands with a future metrics brief"
  placeholder if the architect opts for the lighter shipped
  scope at design — see Q9 below). The stub must not regress
  the 24 px status-bar height or push the kill panel below the
  fold.
- **Acceptance:** a `debug_screen_full` insta snapshot shows
  kill + latency + market-health (3 venues) + server-time +
  version + logs-stub composed under the new shell; switching
  to Debug from Home moves the kill panel off Home (verified by
  V2 + V4 below).

### R6 — Charts screen layout

- **R6.1** Charts screen body, top-to-bottom:
  - Symbol selector **chip row** at the top
    (`space::M = 12 px` outer padding).
  - **Price chart** filling the remaining vertical space.
- **R6.2** Symbol selector chip row: one chip per
  `(Venue, Symbol)` pair drawn from the configured universe.
  - **Live mode**: read from the same source the existing
    `cockpit_live` already uses for venue/symbol routing —
    architect resolves the exact `Config` field at Phase 2
    design (see Q3 below). Most likely the existing
    `Config.universe` / `Config.venues` parsed on boot.
  - **Fixtures mode**: a hard-coded 3-symbol set
    (`Binance/BTCUSDT`, `Binance/ETHUSDT`, `Binance/SOLUSDT`)
    matching the existing fixtures-mode universe.
- **R6.3** Selected chip uses the **T1507 active-row pattern** —
  a 2 px ACCENT left rule on the active chip, **no fill
  change** (consistent with sidebar nav R1.4 and positions /
  strategies row T1507 from Phase 1 R12). Architect ratifies
  at design whether the chip orientation requires the rule on
  the left edge (active-row) or below the chip body
  (active-tab) — see Q5 below.
- **R6.4** Selected `(Venue, Symbol)` persists on
  `Cockpit::selected_symbol: Option<(Venue, Symbol)>` (already
  ratified at architecture.md § 3297). Persistence is **session-
  scoped** — cleared on cockpit restart per
  [`spec/ui-design-principles.md` § Persistence: selected symbol,
  current screen](../../ui-design-principles.md).
- **R6.5** First entry to the Charts screen with no symbol
  selected defaults to the first chip (alphabetic by symbol,
  ties broken by venue name ASC) — `Cockpit::selected_symbol`
  sets to `Some((venue, symbol))` on first paint of Charts so
  the back-and-forth across screens behaves predictably.
- **Acceptance:** a `charts_screen_chip_row_active_btc` insta
  snapshot renders the chip row with the `Binance/BTCUSDT` chip
  active; a `charts_screen_chip_row_active_eth` snapshot covers
  the second-chip-active state.

### R7 — `widgets::chart` price plot

- **R7.1** New widget `crates/ui/src/widgets/chart.rs`. Canvas-
  based, **no external chart crate** (master roadmap scope; iced
  0.14's `iced::widget::canvas` is the rendering primitive).
- **R7.2** Background = `PANEL` (Tier 1); horizontal gridlines =
  `BORDER_1` at 1 px low-alpha (no vertical grid — vertical
  noise competes with marker triangles per
  [`spec/ui-design-principles.md` § Charts](../../ui-design-principles.md)).
- **R7.3** Default plot style: architect-resolved at design (see
  Q1 below). Both must be supportable from the same
  `ChartBuffer` shape (R10):
  - **Line series** in `ACCENT` connecting `Bar.close` of each
    bar — minimal, low-cognitive-load for the operator's "did
    the strategy buy the dip" cross-check.
  - **OHLC candles** with `UP_500` for `close > open` candles
    and `DOWN_500` for `close ≤ open` candles — richer
    intra-bar information at the cost of denser visual.
- **R7.4** **Buy markers** = upward triangle in `UP_500`
  anchored at `(fill.venue_ts, fill.price)`. **Sell markers** =
  downward triangle in `DOWN_500` at the same anchor. Marker
  size = 6 px; rendered above the gridlines and at the same
  z-order as (or above) the price series. The colour pair
  carries over from the P&L card so the operator's "green = my
  side won" mental model is consistent
  ([`spec/ui-design-principles.md` § Charts](../../ui-design-principles.md)).
- **R7.5** **Visible window** = fixed **60 minutes of 1-minute
  bars** for Phase 2 (master roadmap). Pan/zoom is **out of
  scope** for Phase 2 (Q2 below).
- **R7.6** **Empty state** — when the visible window contains no
  bars (rare in live mode; possible in fixtures mode at the
  very first second, or in live mode for a never-traded
  symbol), the chart renders gridlines + a centred `FG_3` "No
  data" label. **Never blank**; matches the
  [`spec/ui-design-principles.md`](../../ui-design-principles.md)
  no-blank-screens rule.
- **R7.7** The chart is **read-only**: no order entry, no draw
  tools, no annotations, no tooltip on hover (hover-tooltip is
  a Phase 4+ ask if the operator requests it). The chart's job
  is "show what the agent did against what the market did";
  any "what if I drew this trendline" surface belongs in a
  research product this codebase deliberately does not have
  (`spec/product.md` § Non-goals).
- **Acceptance:** a `chart_btc_with_two_buys_one_sell` insta
  snapshot from fixtures mode renders the price series with
  three markers (2 buys + 1 sell) deterministically; a
  `chart_empty_state_no_data` snapshot covers the empty
  variant.

### R8 — Marker source = audit ledger (never a runtime accumulator)

- **R8.1** The chart's marker layer reads from the new
  `audit::query::recent_fills_filtered(venue, symbol, time_range)`
  (R12), **never** from a runtime accumulator on `Cockpit`.
  This is the same rule as "ledger is single source of truth
  for P&L" applied to fills: the chart shows what the audit
  query returns, not what the cockpit thinks happened. Any
  ledger / chart divergence is a data bug surfacing through
  the visual cross-check the chart was added to enable
  ([`spec/ui-design-principles.md` § Charts](../../ui-design-principles.md)).
- **R8.2** Marker fetch is async, dispatched via
  `iced::Task::perform`, and routed back as a new
  `Message::ChartMarkersLoaded(Vec<FillView>, Range<Timestamp>)`
  variant (architect resolves the exact variant shape at
  design). The cockpit's `update` flips the chart's marker
  state to `Ready(markers)` per the standard `PanelState<T>`
  pattern (Phase 1 `state::PanelState` precedent).
- **R8.3** Marker fetch is debounced — at most one in-flight
  per `(venue, symbol, window)` triple. Re-fetch triggers:
  (a) selected-symbol change, (b) `BarClose` for the active
  symbol (so newly-printed fills surface within ~1 minute).
  No bar-by-bar re-fetch.
- **R8.4** **Live mode**: markers come from the running ledger
  via `recent_fills_filtered` against the wired
  `Arc<Ledger>` already in scope at `cockpit_live`.
- **R8.5** **Fixtures mode**: the existing
  `crates/ui/src/fixtures.rs::fake_fill_feed` already produces
  a deterministic seed of buy/sell fills with `transaction_id`
  `"fixture-tx-{n}"` and `venue_ts = fixed_ts(n)`. Phase 2 adds
  a `synthetic_fills_for(venue, symbol, count)` helper (or
  re-uses `fake_fill_feed` with a venue/symbol filter — see
  Q6 below) so the fixtures bin renders ≥ 1 buy and ≥ 1 sell
  marker on the chart at every run. **Snapshot-stable**.
- **Acceptance:** a `chart_markers_from_audit_query` integration
  test in `crates/ui/tests/` boots fixtures mode, switches to
  Charts, asserts the marker count matches the synthetic feed
  count for the active symbol; a runtime accumulator is
  explicitly **not** consulted.

### R9 — Chart screen interaction surface

- **R9.1** Click on a chip in the symbol selector emits
  `Message::SelectSymbol(Venue, Symbol)` (already in
  architecture.md § 3305).
- **R9.2** `update`'s `Message::SelectSymbol` arm sets
  `Cockpit::selected_symbol = Some((venue, symbol))` and
  triggers a marker re-fetch (per R8.3). Pure-function rule
  for `update` is preserved by issuing the async fetch from
  the binary's `Task::perform` shim, not from `update` itself
  — same pattern as the existing `TapeRowClicked` →
  `journal_entries_for_transaction` flow at
  [`crates/ui/src/state.rs:665`](../../../crates/ui/src/state.rs).
- **R9.3** Sidebar-nav switch to a non-Charts screen does **not**
  drop `Cockpit::selected_symbol` (session persistence per
  R6.4); switching back to Charts re-renders the same active
  chip without re-selecting from scratch.
- **R9.4** Keyboard shortcut: out of scope for Phase 2. The
  chip row is mouse-driven; arrow-key chip navigation is a
  follow-up if the operator asks. Keeps the Phase 2 surface
  area honest.
- **Acceptance:** a `select_symbol_persists_across_screen_switch`
  unit test in `state::tests` calls `update(SelectSymbol(B,
  ETHUSDT))` then `update(SwitchScreen(Home))` then
  `update(SwitchScreen(Charts))` and asserts
  `selected_symbol == Some((B, ETHUSDT))` after every step.

### R10 — `ChartBuffer` rolling buffer on `Cockpit`

- **R10.1** Add `pub struct ChartBuffer` to `crates/ui/src/state.rs`
  per architecture.md § 3318:

  ```rust
  pub struct ChartBuffer {
      pub series: HashMap<(Venue, Symbol), VecDeque<Bar>>,
  }
  ```

- **R10.2** Add `pub chart_buffer: ChartBuffer` to `Cockpit`.
  `Default` is empty (`HashMap::new()`); fixtures-mode boot
  pre-populates per R11; live-mode populates lazily via
  `BarReceived`.
- **R10.3** Buffer capacity per `(Venue, Symbol)` = **60** bars
  (60 minutes at 1-minute timeframe). Eviction = pop-oldest on
  push when at capacity. Architect resolves at design whether
  the capacity constant lives in `theme::layout` (existing
  `TAPE_MAX_ROWS` precedent — `crates/ui/src/theme/layout.rs`)
  or as a sibling of `STRATEGIES_RECENT_EVENT_CAP` in `state.rs`.
- **R10.4** **Live mode wiring**: extend the existing
  `Message::BarReceived(Bar)` arm at
  [`crates/ui/src/state.rs:503`](../../../crates/ui/src/state.rs)
  to push the bar into
  `chart_buffer.series.entry((bar.venue, bar.symbol)).or_default()`
  before updating `last_bar_ts`. **No new bus channel** — the
  existing `bars_tx` carries every produced `Bar` already. See
  the streaming subscriber at
  [`crates/ui/src/live.rs:247`](../../../crates/ui/src/live.rs).
- **R10.5** **Fixtures mode wiring**: see R11.
- **R10.6** Memory bound: 60 bars × ~200 bytes/Bar × N symbols
  ≈ 12 KB / symbol × ≤ 20 symbols ≤ 250 KB. Trivially within
  desktop budgets; no compaction needed.
- **Acceptance:** a `chart_buffer_evicts_at_capacity` unit test
  in `state::tests` pushes 61 bars for one `(venue, symbol)`,
  asserts buffer length == 60 and the oldest bar is gone; a
  `chart_buffer_keys_distinct_per_pair` test pushes one bar
  each for two pairs and asserts both keys present, both
  buffers length 1.

### R11 — Fixtures synthetic candles

- **R11.1** Add `pub fn synthetic_candles(seed: u64, venue: Venue,
  symbol: Symbol, count: usize) -> Vec<Bar>` to
  [`crates/ui/src/fixtures.rs`](../../../crates/ui/src/fixtures.rs).
  Deterministic random walk: `ChaCha20Rng::from_seed(seed)`,
  per-bar drift `dec!(0.0)`, per-bar vol `dec!(50.0)`, OHLC
  derived from `(prev_close, drift, vol, rng)` so each bar's
  open == prev_close, close == open + Normal(drift, vol),
  high = max(open, close) + |Normal(0, vol/2)|, low =
  min(open, close) - |Normal(0, vol/2)|. Seeded so each
  `(venue, symbol, count)` triple produces the same sequence
  every run.
- **R11.2** Seed convention: architect-ratified at design (see
  Q6 below — analyst recommends per-symbol seed for visually
  distinct shapes; single-seed alternative produces three
  identical traces which is visually misleading).
- **R11.3** The fixtures bin's existing fake-bus shim
  ([`crates/ui/src/bin/cockpit.rs`](../../../crates/ui/src/bin/cockpit.rs))
  pre-seeds the chart buffer at boot by calling
  `synthetic_candles` once per fixture-universe symbol and
  pushing each bar through `Message::BarReceived` (so the
  live-mode `BarReceived` arm — R10.4 — populates the buffer
  in fixtures mode by the same code path). No fixtures-only
  population code on the `update` arm.
- **R11.4** A small synthetic feed loop continues to emit
  `Bar`s 1 / second at a fixed slow drift, so the chart in
  fixtures mode has visible motion in a recorded run while
  remaining snapshot-stable per render frame.
- **R11.5** Fixtures-mode universe pinned to `Binance/BTCUSDT`,
  `Binance/ETHUSDT`, `Binance/SOLUSDT` (R6.2). The fixtures
  fills generator must produce ≥ 1 buy and ≥ 1 sell per symbol
  so the chart's marker layer renders meaningfully on every
  run.
- **Acceptance:** a `synthetic_candles_deterministic` unit
  test in `fixtures::tests` calls
  `synthetic_candles(42, Binance, BTCUSDT, 60)` twice and
  asserts byte-equal output; a `synthetic_candles_distinct_per_seed`
  test asserts that two seeds produce non-equal sequences.

### R12 — Audit query extension: `recent_fills_filtered`

- **R12.1** Add to
  [`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs)
  alongside the existing `recent_fills(limit)` (operator-locked
  Q13 — extend, do not split into a new module):

  ```rust
  /// Phase 2 addition. Return all fills for `(venue, symbol)`
  /// inside `time_range`, newest first. Same description-prefixed
  /// rows scan as `recent_fills`; narrower predicate.
  ///
  /// Read-only over committed audit data; does not alter any
  /// committed report body. Additive — `recent_fills` unchanged.
  pub async fn recent_fills_filtered(
      ledger: &Ledger,
      venue: Venue,
      symbol: Symbol,
      time_range: Range<Timestamp>,
  ) -> Result<Vec<FillView>, LedgerError>;
  ```

- **R12.2** Architect ratifies the **exact signature** at design
  (see Q4 below — `Range<Timestamp>` vs `since: Timestamp,
  until: Timestamp` vs `window: Duration`). Analyst recommends
  `Range<Timestamp>` for symmetry with `pnl_by_symbol(since,
  until)` already in `query.rs:586` while still being a
  half-open interval the chart's window naturally maps to.
- **R12.3** Implementation: SQL projection over
  `journal_transactions` filtered on
  `description LIKE 'buy %'` OR `description LIKE 'sell %'`,
  `ts >= since AND ts < until`, plus a venue predicate. The
  existing rows do **not** carry a venue column — see Q7
  below for the venue-tagging path (analyst recommends the
  description-parse approach if v1.5b's multi-venue rows
  embed the venue in the description; otherwise a JOIN
  against the strategy_events / position-account-id rows
  that v1.5b T817 introduced).
- **R12.4** Symbol filtering reuses the existing
  `extract_symbol_from_description` helper at
  [`crates/audit/src/query.rs:649`](../../../crates/audit/src/query.rs).
  No new symbol-tagging path.
- **R12.5** Determinism: rows ordered `ORDER BY ts DESC, rowid
  DESC` (matching `recent_journal` precedent at
  [`crates/audit/src/query.rs:241`](../../../crates/audit/src/query.rs)).
  No `f64`; `Decimal` arithmetic only.
- **R12.6** Empty result: returns `Ok(vec![])` for windows
  with no fills — never `Err`. Mirrors
  `journal_entries_for_transaction`'s empty-result contract
  ([`crates/audit/src/query.rs:288`](../../../crates/audit/src/query.rs)).
- **R12.7** **Mandatory unit test** in
  `crates/audit/src/query.rs::tests` (or a sibling module if
  the architect prefers): seed a fixture ledger with N fills
  spanning two venues + two symbols, assert
  `recent_fills_filtered` returns only the matching subset
  for each `(venue, symbol, range)` triple, and that the
  fills are time-ordered newest-first.
- **R12.8** **Optional integration test** at
  `crates/audit/tests/recent_fills_filtered.rs`. Architect
  resolves at design whether to require this in Phase 2 (see
  Q10 below — analyst recommends optional in Phase 2;
  Phase 3's Audit screen will need it anyway, so the test
  promotes naturally).
- **Acceptance:** `cargo test -p audit query::tests::recent_fills_filtered_*`
  PASSES; the unit test exercises both an empty-window
  result and a populated-window result.

### R13 — Right-rail track reservation for Phase 6

- **R13.1** The shell grid in R3.1 reserves a **right
  column-track** for the Phase 6 Assistant slot — a
  zero-width column when the v2 LLM strategy is not enabled
  (master roadmap Constraint 4; architecture.md § 3381).
- **R13.2** "Reserved" = the iced layout puts the column in
  the `Row` spec but with `Length::Fixed(0.0)`. No widget
  renders in it; no token references it; the layout simply
  doesn't consume the rightmost track. Architect ratifies the
  exact iced shape at design (see Q7 below — analyst
  recommends an inline placeholder element rather than a
  dedicated `right_rail` widget that adds dead code).
- **R13.3** No Phase 2 visual surface change from this — the
  reservation is invisible until Phase 6 sets a non-zero
  width. The Phase 2 presentation does **not** show a hidden
  rail; the rail is structural only.
- **R13.4** Phase 2 must not consume the right column-track
  for any other purpose. (Common-mistake guard: the chart
  widget's natural impulse is "let me float a tooltip on the
  right" — explicitly forbidden in Phase 2.)
- **Acceptance:** a `shell_grid_reserves_right_rail` unit
  test in `crates/ui/tests/` asserts the shell layout has a
  rightmost column with width = 0.0 when the v2-LLM gate is
  off (the gate is a `cfg!` flag or a `Cockpit` field; the
  architect picks at design — Phase 2's job is to leave the
  spot, not to wire the gate).

### R14 — Cross-feature invariants

- **R14.1** [`operator-success-reports`](../../operator-success-reports/feature.md)
  R7 latency badges: latency badge **moves to Debug screen**;
  colour mapping unchanged (Phase 1 R15.1 + Phase 1 Q8
  reconcile). Tester verifies the band-name-to-colour mapping
  still renders correctly under Debug.
- **R14.2** [`live-cockpit-unified`](../../live-cockpit-unified/feature.md):
  `cockpit_live` bin launches against the agent runtime;
  halted-banner trips on file watch / kill / heartbeat. The
  banner renders **above the screen body, below the title bar**
  (R3.3) so it's visible regardless of `current_screen`.
  Banner trigger preserved.
- **R14.3** [`real-mtm-unrealized-pnl`](../../real-mtm-unrealized-pnl/feature.md):
  PnL card lives on Home screen; `color_for_delta` signature
  unchanged.
- **R14.4** [`per-symbol-position-accounts`](../../per-symbol-position-accounts/feature.md):
  Positions widget lives on Home screen; row contract +
  strategy-id chip styling unchanged.
- **R14.5** [`tape-row-audit-modal`](../../tape-row-audit-modal/feature.md):
  modal continues to be reachable from any tape row on the
  Home screen; modal trigger preserved. Phase 3's Audit
  screen will reuse the same modal.
- **R14.6** [`journal-tx-metadata`](journal-tx-metadata.md):
  modal-header rendering unchanged.
- **R14.7** [`v1.5b-multi-venue`](../../v1-5b-multi-venue/feature.md): venue
  dimension surfaces in **Debug screen** (per-venue
  market-health rows R5.3) and on the Charts screen's
  **chip row** (per-`(Venue, Symbol)` chip R6.2). Existing
  `cockpit_live` venue-tagged tick rendering unchanged.
- **Acceptance:** the tester's per-feature invariant table in
  the Phase 2 report shows PASS for all 7 rows.

### R15 — Anchor regression

- **R15.1** All 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../../anchors.toml) verify byte-identical
  post-Phase 2.
- **R15.2** No new anchor scenarios; no re-lock budget; zero
  exceptions. The new `recent_fills_filtered` query is
  read-only over the same description-prefixed rows
  `recent_fills` already iterates; it cannot alter any
  committed report body by construction (no writer added,
  no renderer changed).
- **R15.3** `verify-anchors` skill PASS at the Phase 2 tester
  gate.
- **Acceptance:** the tester's anchor table is 11 / 11 PASS.

## Verification (V-items)

The tester gates Phase 2 ship against these V-items.

- **V1 — Both bins launch with sidebar + Home default.**
  `cargo run --bin cockpit --features fixtures` and
  `cargo run --bin cockpit_live --features live` both launch;
  the sidebar is visible on the left at ~180 px; Home is the
  active row (T1507 accent rule on Home); the four Phase 1
  widgets (PnL + Positions + Strategies + Tape) render in the
  Home body. Verified by the recorded runs in the Phase 2
  presentation. Maps to R1, R2, R3, R4.

- **V2 — Switching to Debug moves operations chrome.** From
  the launched bin, click Debug in the sidebar. Kill switch,
  latency detail, per-venue market-health, server-time,
  version, logs/metrics stub all render on Debug; none of
  them appear on Home. Verified by visual diff between the
  `home_screen_default` and `debug_screen_full` snapshots
  + a manual run. Maps to R5.

- **V3 — Charts renders price plot + ≥ 1 buy + ≥ 1 sell.**
  From the launched fixtures bin, click Charts in the
  sidebar. The chip row renders 3 chips (`Binance/BTCUSDT`,
  `Binance/ETHUSDT`, `Binance/SOLUSDT`); `Binance/BTCUSDT` is
  active. The price plot renders the 60-bar synthetic series.
  At least one upward-triangle (buy) marker and one
  downward-triangle (sell) marker render. In live mode, same
  expectation but populated from the real audit ledger if
  any fill exists for the visible window (acceptable empty
  state if not — the tester documents which mode they
  observed). Verified by the
  `chart_btc_with_two_buys_one_sell` snapshot + a manual
  run. Maps to R6, R7, R8.

- **V4 — `recent_fills_filtered` unit test PASS.**
  `cargo test -p audit query::tests::recent_fills_filtered_*`
  PASSES; the unit test exercises an empty-window result
  and a populated-window result (≥ 2 fills) for each of two
  `(venue, symbol)` triples. Maps to R12.

- **V5 — `ChartBuffer` rolling buffer + eviction.**
  `cargo test -p ui state::tests::chart_buffer_*` PASSES;
  tests cover the 60-bar capacity, oldest-bar eviction, and
  per-`(venue, symbol)` key isolation. Maps to R10.

- **V6 — Synthetic candles deterministic.**
  `cargo test -p ui fixtures::tests::synthetic_candles_*`
  PASSES; tests cover byte-equal output across two calls
  with the same seed, and divergent output across two
  different seeds. Maps to R11.

- **V7 — Sidebar nav widget snapshots.**
  `cargo test -p ui sidebar_nav` PASSES; snapshots cover
  the three-row default, the Home-active variant, the
  Debug-active variant, and the Charts-active variant.
  Maps to R1.

- **V8 — Right-rail track reservation.**
  `cargo test -p ui shell_grid_reserves_right_rail` PASSES;
  the test asserts the rightmost column-track in the shell
  `Row` has width 0.0 when the v2-LLM gate is off. Maps
  to R13.

- **V9 — Cross-feature invariants.** All 7 invariants in R14
  PASS in the tester's per-feature invariant table.

- **V10 — Anchors 11 / 11 PASS.** `verify-anchors` PASS;
  no anchor body diff. Maps to R15.

- **V11 — Snapshot baselines refresh coherently.**
  `cargo insta accept` is run once at end-of-phase per
  Phase 1 Q2 precedent; `cargo insta test --workspace`
  returns clean; no leftover `*.pending-snap` files. ~36
  refreshed + ~5 net-new = ~41 baselines, the visual diff is
  the visible artefact in the Phase 2 review.

- **V12 — `rust-validate` PASS.** `cargo fmt`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `cargo deny
  check`, `cargo audit` (or N-A) all PASS.

## Acceptance criteria

Phase 2 ships when all of the following hold. Each bullet
traces to its R-cluster:

- **Both bins launch with the new shell.**
  `cargo run --bin cockpit --features fixtures` and
  `cargo run --bin cockpit_live --features live` launch with
  the sidebar visible on the left, Home selected by default,
  and the status bar continuing to span the bottom. Traces
  to R1, R2, R3, R4.

- **Sidebar moves operations chrome off the trading view.**
  Switching to Debug via the sidebar moves kill / latency /
  market-health / server-time / version off the Home screen
  and onto Debug. Switching back to Home shows only trading
  data. Traces to R5.

- **Charts screen renders a price plot with markers.**
  Switching to Charts via the sidebar renders a price plot
  for the selected symbol with at least one buy and one
  sell marker (in fixtures mode, deterministic; in live
  mode, populated from the audit ledger if any fill exists
  for the visible window). Traces to R6, R7, R8, R9.

- **Audit query method is reachable from a unit test.** The
  new `recent_fills_filtered` is reachable from a unit test
  in `crates/audit/src/query.rs::tests` that exercises the
  empty-window and populated-window paths. Traces to R12.

- **Both modes populate the chart.** Live mode rolls bars
  into `ChartBuffer` via the existing `bars_tx` channel;
  fixtures mode pre-seeds via `synthetic_candles` and
  continues to emit synthetic bars at 1/s. Traces to R10,
  R11.

- **Right-rail Phase 6 slot is reserved (zero-width).** The
  shell grid has a rightmost column-track with width 0.0
  when the v2-LLM gate is off; no widget renders in it. No
  Phase 2 visual surface change from this. Traces to R13.

- **Cross-feature invariants PASS.** All seven rows in the
  Phase 2 column of the master invariant table PASS. Traces
  to R14.

- **11 / 11 anchor regression PASS.** `verify-anchors` PASS
  with byte-identical bodies. Traces to R15.

- **`rust-validate` PASS.** Fmt, clippy `-D warnings`,
  cargo-deny, audit (or N-A), docs all PASS.

- **Snapshot baselines accepted in one pass.** `cargo insta
  accept` is run once at end-of-phase; ~41 baselines
  refresh; the visual diff is the visible artefact. Traces
  to V11.

## Open questions for architect

These resolutions are deliberately punted to the architect.
Q11–Q14 from the master roadmap are **operator-locked** and
are not opened here; the questions below are the genuinely-
open design choices that ratify at architect kickoff. Each
ends with a one-line **analyst recommendation** so the
architect has a starting point.

### Q1 — Default plot style: line series or OHLC candles?

**The question:** R7.3 supports both line series and OHLC
candles from the same `ChartBuffer` shape. Which is the
**default** the operator sees on first paint of the Charts
screen?

- **(a)** Line series in `ACCENT` connecting `Bar.close`. Low-
  cognitive-load; matches the operator's "did the strategy
  buy the dip" cross-check at one-glance.
- **(b)** OHLC candles in `UP_500` / `DOWN_500`. Richer
  intra-bar information; matches the operator's mental
  model from external charting tools.

**Recommended (analyst):** **(a) line series**. Phase 2's
chart is a cross-check surface, not a primary trading chart
(the cockpit is paper-trading, observation-only — see
[`spec/product.md`](../../product.md)). The operator's question
is "did the marker land on or near the line at the right
time", which a line plot answers most directly. Candles add
visual density without answering a question Phase 2 needs
to answer. **Defer the candle variant to a post-Phase-2
ask** if the operator requests it. Architect ratifies.

### Q2 — Pan/zoom in scope or deferred?

**The question:** Phase 2's chart visible window is fixed at
60 minutes (R7.5). Does Phase 2 also ship pan/zoom
controls (scroll-wheel zoom, click-drag pan, "Last 1 h /
4 h / 24 h" preset buttons)?

**Recommended (analyst):** **deferred**. Pan/zoom adds
~2-3 R-items of widget surface (axis re-scaling, hit-region
tracking, marker re-positioning under pan, snapshot-stable
default-on-first-paint) and risks bloating Phase 2 past the
"one shippable thing" budget. The 60-minute fixed window
covers the operator's "did this fill land at the right
candle" cross-check; longer windows are a Phase 4 (Backtest
panel — equity curve already needs pan) ask. Architect
ratifies.

### Q3 — Symbol-selector universe source (live mode)

**The question:** R6.2 says the live-mode chip row is drawn
from "the existing `Config.universe`/`Config.venues` parsed
on boot". Where exactly does the chip row read from?

- **(a)** A new field on `Cockpit` populated at boot from
  `agent::config::Config`.
- **(b)** A direct read of `Config` at view time (passed
  through the shell as a parameter).
- **(c)** Re-derive from the active `Cockpit::market_health`
  keys (Phase 1 already populates this on first tick).

**Recommended (analyst):** **(a) `Cockpit::universe:
Vec<(Venue, Symbol)>`, populated once at boot**. View-time
config reads (b) couple the widget to the live config
plumbing in a way fixtures mode can't satisfy without a
shim. Re-deriving from market-health (c) means a never-
ticked symbol disappears from the chip row, which is wrong
for a "show me the chart of X" surface. A boot-time field
matches the existing `Cockpit::account_label` precedent
(also boot-populated, also static for the session — Phase 1
R13.4). Architect ratifies.

### Q4 — `recent_fills_filtered` exact signature

**The question:** R12.1 working signature is
`(venue: Venue, symbol: Symbol, time_range: Range<Timestamp>)`.
Three variants are reasonable:

- **(a)** `Range<Timestamp>` — half-open interval, idiomatic
  Rust, but `Timestamp` is `OffsetDateTime`-wrapped and
  `Range<T: !Step>` doesn't iterate (fine, we only call
  `.start` and `.end`).
- **(b)** `since: Timestamp, until: Timestamp` — symmetric
  with `pnl_by_symbol(since, until)` already in `query.rs:586`.
  Half-open or inclusive at architect's discretion.
- **(c)** `window: Duration` (and the function picks `now`
  internally). Simpler call-site for the chart but couples
  the query to a clock source, breaking determinism for
  fixtures mode.

**Recommended (analyst):** **(b) `since, until` two-arg
form**, half-open (`ts >= since AND ts < until`), to match
the existing `pnl_by_symbol` and `funding_rate_history`
shape. Both functions already use `since: Timestamp,
until: Timestamp`; symmetry beats Rust idiom here. The
chart's call-site computes `(window_start, window_end)`
deterministically from `Cockpit::server_time_now` — no
hidden clock. Architect ratifies.

### Q5 — Symbol-selector chip row: active rule placement

**The question:** R6.3 says the active chip uses the T1507
active-row pattern (2 px ACCENT rule, no fill). T1507 was
designed for **rows in a table** (left rule). Chips arranged
horizontally feel more like **tabs** (rule under or above
the chip body).

- **(a)** Apply the rule on the **left edge of the chip**
  (literal T1507 reuse). Visually consistent with
  positions / strategies / sidebar nav rows.
- **(b)** Apply the rule **under the chip** (tab-style).
  Visually conventional for horizontal selectors. Requires
  a small T1507 variant.
- **(c)** Apply a 1 px hairline border on all four edges of
  the active chip in `ACCENT` (full-frame highlight).
  Different pattern entirely.

**Recommended (analyst):** **(b) under the chip, tab-style**,
documented as a T1507 variant in the Phase 2 principles-
doc append (one-line note; no doc rewrite). The active-row
left-rule pattern is the *concept* — "2 px accent, no fill
change" — and the literal edge depends on the widget's
orientation. Sidebar nav rows are vertical → left rule;
chip row is horizontal → bottom rule. Architect ratifies
the variant naming.

### Q6 — Synthetic-candle seed convention

**The question:** R11.2 — does fixtures mode use **a single
seed** for all symbols, or **a seed per symbol** so each
chip's chart looks visually distinct?

- **(a)** Single seed (e.g. `0x1F75D69C`). All three
  symbols share the same random walk shape, scaled to
  their respective starting prices. Simpler. Visually
  misleading (three identical traces).
- **(b)** Per-symbol seed (e.g. `hash(symbol_str)` modulo a
  64-bit constant). Each chip's chart looks distinct.
  Slightly more code in `synthetic_candles`. Visually
  honest.

**Recommended (analyst):** **(b) per-symbol seed**. The
fixtures bin is the ui-designer's daily-driver dev tool;
showing three identical traces hides the per-symbol layout
issues a real multi-symbol setup would expose. The added
code is a 1-line `seed = hash(symbol)` at the call site.
Architect ratifies the exact hash (analyst suggests a
deterministic FxHash of the `Symbol` `&str`).

### Q7 — Right-rail track in shell grid: structural now or deferred?

**The question:** R13 reserves a right column-track at
zero width. **Where in the iced layout does this live?**

- **(a)** A literal third column in the shell `Row` with
  `Length::Fixed(0.0)`. Architecturally honest; visually
  identical to "no column at all" because zero pixels.
- **(b)** A `cfg!(feature = "v2-llm")` gate around the
  third column — Phase 2 ships without the column at all,
  Phase 6 inserts it. Less dead code; couples to the v2
  LLM feature gate that doesn't exist yet.
- **(c)** Deferred to Phase 6 entirely — Phase 2 leaves a
  comment in the shell module pointing at the future
  column. Phase 6's brief lands the actual column.

**Recommended (analyst):** **(a) structural now**. Master
roadmap Constraint 4 is unambiguous — Phase 2 reserves the
slot. A `Length::Fixed(0.0)` column is the cheapest
honest reservation: zero render cost, zero dead code (the
column is a single `Length` constant), and Phase 6 swaps
the constant to the real width without restructuring the
shell. Phase 1 Q9 deferred this for the Phase 4 assistant
slot pre-roadmap-revision; the post-revision lock at
master Constraint 4 supersedes that. Architect ratifies.

### Q8 — Sidebar nav state persistence implementation

**The question:** [`spec/ui-design-principles.md` § Persistence:
selected symbol, current screen](../../ui-design-principles.md)
locks `current_screen` and `selected_symbol` as **session-
scoped** persistence. R2.2 + R6.4 implement that with
`Cockpit::current_screen` (default `Home`) and
`Cockpit::selected_symbol` (default `None`). **Verify with
architect that this is implementable without a new
`Cockpit` field plumbing path** — i.e. the existing
`Cockpit` is the canonical state store and no new
serialization or `~/.cockpit-state.json`-style path is
sneaking in.

**Recommended (analyst):** **confirm the two-field
addition is sufficient — no on-disk persistence**. Master
roadmap is explicit that session-scoped means in-memory
only; the operator-locked "the cockpit is an instrument,
not a browser" rule from Phase 1 forbids back-stack /
bookmark / saved-state paths. Architect ratifies as a
sanity check; if any persistence smells creep in at
design, escalate to operator.

### Q9 — Debug screen logs/metrics output stub scope

**The question:** R5.7 — the logs/metrics output panel on
the Debug screen. Three shapes:

- **(a)** A literal "Logs surface lands with a future
  metrics brief" placeholder. Zero new code; honest about
  the scope boundary. Tester verifies the placeholder
  copy is in `ui::strings`.
- **(b)** A read-only tail of the last N (e.g. 50)
  `tracing` events at INFO+ level, captured via a
  `tracing-subscriber` layer that writes into a
  `Cockpit::log_buffer: VecDeque<LogLine>` field. Useful
  for the operator. ~80 lines of new code + a test for
  the buffer ring.
- **(c)** Scoped down further — only WARN+ events, no
  buffer ring, just the latest one rendered as a status
  line. Lightweight middle-ground.

**Recommended (analyst):** **(a) placeholder**. The Debug
screen's job in Phase 2 is to **collect operations chrome
the operator only checks occasionally** — a logs surface
with no current operator-stated need is gold-plating.
Defer to a future "structured metrics surface" brief
when the operator names a specific gap (e.g. "I want to
see reconnect events on a graph"). The placeholder gives
Phase 2 a clean stopping point and Phase 3+ a clean
follow-up trigger. Architect ratifies.

### Q10 — `recent_fills_filtered` test scope

**The question:** R12.7 mandates a unit test in
`audit::query::tests`. R12.8 leaves an integration test at
`crates/audit/tests/recent_fills_filtered.rs` as optional.
**Does Phase 2 require the integration test, or only the
unit test?**

**Recommended (analyst):** **unit test only is required;
integration test is optional**. The unit test exercises
the SQL projection + the description-parse against a
fixture ledger seeded inline (the existing
`crates/audit/tests/journal_entries_for_transaction.rs`
precedent shows the boilerplate is ~30 lines per
integration). Phase 3's Audit screen will need an
integration test against a multi-venue / multi-symbol /
multi-kind ledger anyway, so the integration test
promotes naturally one phase later. Phase 2's gate is the
unit test + the V3 manual run that the chart's marker
layer renders correctly. Architect ratifies — if the
architect prefers to land the integration test now
(belt-and-braces), Phase 2's task list adds it as one
extra tick.

### Q11 — TD-1 re-evaluation: does iced 0.15+ ship the focus-ring API?

**The question:** Master roadmap TD-1 (true keyboard-focus
ring) is documented as "earliest re-evaluation at Phase 2
analyst kickoff". Has iced 0.15+ shipped with
`button::Status::Focused` AND `text_input::Style.shadow`?

**Recommended (analyst):** **architect verifies the iced
release notes at Phase 2 design**.

- **If iced 0.15+ has shipped with both fields:** the
  follow-up is a one-file sweep across
  `crates/ui/src/widgets/kill.rs` (two button styles + one
  input) and `crates/ui/src/widgets/journal_transaction_modal.rs`
  (one button style) — replace `Hovered` arms with
  `Focused` arms; add the `shadow` field on the input.
  ~30 lines net change. **Folds into Phase 2 as a small
  R-item appendix or a separate housekeeping task** —
  architect picks.
- **If iced 0.15+ has not shipped:** restate the
  deferral; re-evaluate again at Phase 3 kickoff.

The keyboard-focus halo is a **secondary signal** behind
the kill-switch typed-confirm phrase; the deviation
remains a known-bounded ergonomic gap, not a safety gap.
Phase 2 ships either way.

## Backlog updates

Effective on this brief's promotion (2026-05-04):

### Active

- **`lumen-phase-2-shell-ia-charts`** — this brief, expanded
  from stub status to active. Status: `active`. Owner:
  analyst. Pipeline next stage: **architect**.

### Queue (unchanged from master roadmap)

- **`lumen-phase-3-detail-screens`** — promotes on Phase 2
  ship. Status: queued.
- **`lumen-phase-4-backtest-panel`** — promotes on Phase 3
  ship. Status: queued.
- **`lumen-phase-5-humancontrol-agentfeed`** — promotes on
  Phase 4 ship. Status: queued.
- **`lumen-phase-6-assistant-slot`** — reserved, linked to
  v2 LLM. No analyst spawn until v2 LLM is approved.

### Recent (shipped)

- **`lumen-phase-1-foundation`** — shipped 2026-05-04
  (tester third-pass PASS).

### Stub supersede note

The 2026-05-04 stub of this brief (138 lines, queued status,
high-level scope only) is **superseded by this expansion**.
The Why section is preserved verbatim with one extension
sentence; Scope is replaced by the R-item-pointing summary;
Open questions are replaced by the architect Q-items below;
Acceptance criteria are extended to trace each bullet to
its R-cluster. Master roadmap reference unchanged: see
[`lumen-design-adoption.md` Phase 2 section](../feature.md).

## Design

_Architect-owned. Resolves Q1–Q11 — every recommendation lands as
**ratified** unless flagged "Architect override". The analyst sections
above are immutable; this section is the design contract the developer
reads alongside the task list at
[`spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md`](tasks.md)._

### Q-item resolutions

All 11 architect Q-items resolved. **11/11 ratified, zero deviations
from analyst recommendation.** Each row cites the R-item(s) the
resolution lands.

| Q   | Question                                                | Resolution                                                                                                                                                                                                                                                                                  | Ratifies   |
|-----|---------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------|
| Q1  | Default plot style: line or OHLC                        | **Line series in `ACCENT` connecting `Bar.close`.** Phase 2's chart is a cross-check surface, not a primary trading chart. The operator's question is "did the marker land on or near the line at the right time"; a line plot answers most directly. The OHLC variant remains supportable from the same `ChartBuffer` shape (R10) — defer to a post-Phase-2 ask if the operator requests it. | R7.3       |
| Q2  | Pan/zoom in scope                                       | **Deferred.** Phase 2 ships the fixed 60-minute window. Pan/zoom adds ~2–3 R-items of widget surface (axis re-scaling, hit-region tracking, marker re-positioning under pan, snapshot-stable default-on-first-paint) and risks bloating Phase 2 past the "one shippable thing" budget. Phase 4's Backtest equity curve is the natural next pan-capable surface. | R7.5       |
| Q3  | Symbol-selector universe source (live mode)             | **`Cockpit::universe: Vec<(Venue, Symbol)>`, populated once at boot.** Matches the existing `Cockpit::account_label` precedent (also boot-populated, also static for the session — Phase 1 R13.4). View-time `Config` reads couple the widget to live config plumbing in a way fixtures mode can't satisfy without a shim; market-health-key derivation (option c) means a never-ticked symbol disappears from the chip row, which is wrong for "show me the chart of X". | R6.2       |
| Q4  | `recent_fills_filtered` exact signature                 | **Two-arg `since: Timestamp, until: Timestamp` half-open form.** Symmetry with the existing `pnl_by_symbol(since, until)` at `query.rs:586` and `funding_rate_history` shape beats Rust idiom. The chart's call-site computes `(window_start, window_end)` deterministically from `Cockpit::server_time_now` — no hidden clock; the `Range<Timestamp>` form (option a) doesn't carry its weight. | R12.1, R12.2 |
| Q5  | Chip-row active rule placement                          | **Bottom-edge variant of the T1507 active-row pattern.** The active-row concept is "2 px ACCENT, no fill change"; the literal edge depends on widget orientation. Sidebar nav rows are vertical → left rule; chip row is horizontal → bottom rule. The `frame::active_row` helper grows a sibling `active_chip` helper that prepends a 2 px bottom rule (`Column::push(content).push(rule_2px_bottom)` rather than `Row::push(rule_2px_left).push(content)`). One-line note in the Phase 2 principles-doc append documents the variant. | R6.3       |
| Q6  | Synthetic-candle seed convention                        | **Per-symbol seed.** `seed = hash(format!("{venue:?}/{symbol}"))` via `std::hash::DefaultHasher` (zero new dep, deterministic across runs because `DefaultHasher` is the workspace default and the hash key is a stable string). Each chip's chart looks distinct in fixtures mode; the ui-designer's daily-driver dev tool no longer hides per-symbol layout issues. The `synthetic_candles` API takes a `seed: u64` parameter and the fixtures-bin caller computes the per-symbol seed at the call-site. | R11.1, R11.2 |
| Q7  | Right-rail track: structural now or deferred            | **Structural now.** The shell `Row` literally contains a third column with `Length::Fixed(0.0)` when the v2-LLM gate is off. Master roadmap Constraint 4 is unambiguous; a `Length::Fixed(0.0)` column is the cheapest honest reservation (zero render cost, zero dead code — a single `Length` constant), and Phase 6 swaps the constant to the real width without restructuring the shell. **No `cfg!(feature = "v2-llm")` gate** — the gate doesn't exist yet, and adding a feature flag for one zero-pixel column is more dead code than the column itself. | R13.1, R13.2 |
| Q8  | Sidebar nav state persistence implementation            | **Two-field addition only — no on-disk persistence.** `Cockpit::current_screen: Screen` (default `Home`) + `Cockpit::selected_symbol: Option<(Venue, Symbol)>` (default `None`). Both session-scoped per [`spec/ui-design-principles.md` § Persistence](../../ui-design-principles.md). No `~/.cockpit-state.json`, no `serde::Serialize` on `Cockpit`, no `Drop` impl writing state. The cockpit is an instrument, not a browser. | R2.2, R6.4 |
| Q9  | Debug screen logs/metrics output stub scope             | **(a) Placeholder.** A single `frame::muted_body("Logs surface lands with a future metrics brief")` row at the bottom of the Debug screen body, with the copy added to `ui::strings` per the no-inline-prose rule. Zero new code paths; honest about the scope boundary. Defer to a future "structured metrics surface" brief when the operator names a specific gap. | R5.7       |
| Q10 | `recent_fills_filtered` integration test required       | **Unit test only is required; integration test is optional in Phase 2.** The unit test in `crates/audit/src/query.rs::tests` exercises the SQL projection + the description-parse against a fixture ledger seeded inline (the existing `crates/audit/tests/journal_entries_for_transaction.rs` precedent shows the boilerplate is ~30 lines). Phase 3's Audit screen will need the multi-venue / multi-symbol / multi-kind integration anyway, so the integration test promotes naturally one phase later. Phase 2's gate is the unit test + V3 manual run. | R12.7, R12.8 |
| Q11 | TD-1 re-evaluation: iced 0.15+ focus-ring API           | **Restate the deferral.** Verified at design pass: `crates/ui/Cargo.toml:50` pins `iced = { version = "=0.14.0", ... }`. iced 0.15+ has not landed on disk; the `button::Status::Focused` variant and `text_input::Style.shadow` field are not available. Phase 2 ships **no focus-ring upgrade**. TD-1 row in the master roadmap stands. **Named upgrade trigger** unchanged: any iced version bump that exposes both fields promotes the ~30-line one-file sweep across `widgets/kill.rs` (two button styles + one input) and `widgets/journal_transaction_modal.rs` (one button style); next re-evaluation at Phase 3 analyst kickoff. The kill-switch destructive flow stays typed-confirm gated; the focus halo remains a secondary signal. | TD-1       |

**No principled overrides.** Analyst recommendations are
operator-aligned and consistent with the master roadmap's
operator-locked Q11–Q14 + Constraint 4; the architect ratifies all
eleven.

### Cockpit state diff

The state diff is the load-bearing scaffold for every Phase 2 widget
and is the precise edit `crates/ui/src/state.rs` receives:

```rust
// ── crates/ui/src/state.rs — Phase 2 additions ───────────────────────────

/// Phase 2 introduces the screen-routed shell. Phase 3 extends with the
/// detail screens (Strategies / Risk / Audit). All six variants ship in
/// Phase 2 so Phase 3's enum extension is a backlog item, not an enum
/// migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Home,        // Phase 2 — pnl + positions + strategies + tape grid
    Debug,       // Phase 2 — kill + latency + market-health + version + logs stub
    Charts,      // Phase 2 — chip-row + canvas chart with audit markers
    Strategies,  // Phase 3 — declared now, dispatch returns "Not yet"
    Risk,        // Phase 3 — declared now, dispatch returns "Not yet"
    Audit,       // Phase 3 — declared now, dispatch returns "Not yet"
}

/// Phase 2 chart rolling buffer. Keyed by `(Venue, Symbol)`; each value
/// is a `VecDeque<Bar>` capped at `CHART_BUFFER_CAPACITY` (60 bars =
/// 60 minutes of 1-minute bars). Eviction = pop-oldest on push when
/// at capacity.
#[derive(Debug, Default, Clone)]
pub struct ChartBuffer {
    pub series: HashMap<(Venue, Symbol), VecDeque<Bar>>,
}

impl ChartBuffer {
    /// Push a new bar onto the deque for `(venue, symbol)`,
    /// evicting the oldest if the deque is at capacity.
    pub fn push_bar(&mut self, bar: Bar) {
        let key = (bar.venue, bar.symbol.clone());
        let series = self.series.entry(key).or_default();
        if series.len() == CHART_BUFFER_CAPACITY {
            series.pop_front();
        }
        series.push_back(bar);
    }

    /// Read-only iterator over bars for `(venue, symbol)`,
    /// oldest-first (the chart canvas paints left-to-right).
    pub fn bars(&self, venue: Venue, symbol: &Symbol) -> impl Iterator<Item = &Bar> {
        self.series
            .get(&(venue, symbol.clone()))
            .into_iter()
            .flat_map(|deque| deque.iter())
    }
}

/// 60 minutes of 1-minute bars per `(venue, symbol)`. Sibling of
/// `STRATEGIES_RECENT_EVENT_CAP` and `TAPE_MAX_ROWS` — lives in
/// `state.rs` because it's a state-shape constant, not a render
/// constant. (Q for the developer: if `theme::layout::TAPE_MAX_ROWS`
/// pulls capacity constants under one roof in Phase 3, migrate.
/// For Phase 2, sibling-of-state-rs precedent wins — R10.3 notes
/// the choice as architect-resolved here.)
pub const CHART_BUFFER_CAPACITY: usize = 60;

pub struct Cockpit {
    // … all existing Phase 1 fields (mode, tape, positions, pnl, strategies,
    //   strategies_signal_counters, strategies_recent_events, kill, latency,
    //   market_health, server_time_now, account_label, last_bar_ts,
    //   last_tick_ts, tape_audit_modal, kill_switch) …

    // ── Phase 2 — Shell IA + Charts ─────────────────────────────────────
    /// Active screen for the routed shell. Default `Home` so cold-start
    /// lands the operator on trading data, not operations chrome.
    pub current_screen: Screen,

    /// Configured `(Venue, Symbol)` universe — populated once at boot
    /// from `agent::config::Config` in live mode, hard-coded to the
    /// 3-symbol Binance set in fixtures mode. Static for the session.
    /// (Q3 ratification.)
    pub universe: Vec<(Venue, Symbol)>,

    /// Currently-selected `(Venue, Symbol)` on the Charts screen.
    /// `None` until the operator first enters Charts; auto-set to the
    /// first universe entry on first paint of Charts (R6.5). Persists
    /// across Home ↔ Debug ↔ Charts switches; cleared only on cockpit
    /// restart (Q8).
    pub selected_symbol: Option<(Venue, Symbol)>,

    /// Per-`(Venue, Symbol)` rolling 60-bar buffer fed by the existing
    /// `Message::BarReceived` arm. Lives on `Cockpit` (not in a
    /// separate `ChartState` struct) because every Phase 2 message
    /// already routes through `update(model, msg)` and the buffer is
    /// state.
    pub chart_buffer: ChartBuffer,

    /// Marker layer for the Charts screen — fills filtered to the
    /// active `(venue, symbol, window)` triple. `Loading` until the
    /// first async fetch returns; `Ready(fills)` after; `Error(msg)`
    /// on query failure (matches Phase 1 `PanelState<T>` precedent).
    pub chart_markers: PanelState<Vec<FillView>>,
}

pub enum Message {
    // … all existing Phase 1 variants …

    // ── Phase 2 — Shell IA + Charts ─────────────────────────────────────
    /// Sidebar-nav row click. Pure assignment; no side effects.
    SwitchScreen(Screen),

    /// Symbol-selector chip click. Sets `selected_symbol`; the binary's
    /// `Task::perform` shim then dispatches the marker re-fetch (R8.3).
    /// Pure-function `update` discipline preserved — async work lives
    /// in the binary, not in `update`.
    SelectSymbol(Venue, Symbol),

    /// Async result of the `recent_fills_filtered` fetch issued after
    /// `SelectSymbol` or after `BarClose` for the active symbol.
    /// `Ok(fills)` → `chart_markers = Ready(fills)`; `Err(msg)` →
    /// `chart_markers = Error(msg)`. Mirrors the
    /// `TapeAuditEntriesLoaded` shape from `tape-row-audit-modal`.
    ChartMarkersLoaded(Result<Vec<FillView>, SmolStr>),

    /// New bar arrived through the existing `bars_tx` channel. The
    /// existing `BarReceived` arm is **extended** (not replaced) to
    /// also push the bar into `chart_buffer` (R10.4). No new bus
    /// channel; no new subscription.
    /// **Reuses the existing `Message::BarReceived(Bar)` variant.**
    /// (Listed here for completeness; the variant itself is unchanged.)
    ChartTickReceived,  // architecturally; in code this is BarReceived's extended arm.
}
```

**`Default` impl.** Extends Phase 1's `impl Default for Cockpit`:
`current_screen: Screen::Home` (the `#[default]` derive carries the
load), `universe: Vec::new()` (boot-populated; empty until the boot
shim runs — fixtures mode pre-populates before first paint, live mode
populates from `Config` before iced takes the runtime), `selected_symbol:
None`, `chart_buffer: ChartBuffer::default()` (empty `HashMap`),
`chart_markers: PanelState::Loading`.

**Fixtures-mode initial state.** The fixtures-bin boot shim
(`crates/ui/src/bin/cockpit.rs`) sets `cockpit.universe =
vec![(Binance, BTCUSDT), (Binance, ETHUSDT), (Binance, SOLUSDT)]`,
then iterates the universe and calls `synthetic_candles(per_symbol_seed,
venue, symbol, 60)` for each pair, dispatching each generated bar via
`Message::BarReceived` so the live-mode `BarReceived` arm (R10.4)
populates the buffer. The 1 Hz synthetic feed loop continues to emit
new bars at the head — every `view()` after first paint sees a
60-bar window and the chart's marker layer (seeded by
`fake_fill_feed` extended to ≥ 1 buy + ≥ 1 sell per fixtures symbol,
R11.5) renders at least one of each.

**Live-mode initial state.** `cockpit_live.rs` populates `universe`
from `Config.universe.usdt_symbols` × `Config.data.sources` (the
existing v1.5b config shape) into a `Vec<(Venue, Symbol)>` at boot,
before `iced::application::run`. `chart_buffer` starts empty and
populates lazily via the existing `BarReceived` arm as the agent
runtime emits bars on `bars_tx`. The Charts screen's first paint
with no buffered bars renders the empty-state ("No data" centred,
gridlines drawn — R7.6); subsequent paints catch up as bars
arrive.

### Sidebar nav widget contract

**File:** `crates/ui/src/widgets/sidebar_nav.rs` (new). One-file-per-
widget rule (Phase 1 Q4 precedent). Single `view()` entry point:

```rust
/// Render the sidebar nav. Stateless — `current_screen` lives on
/// `Cockpit`; the widget reads it as a parameter to know which row
/// carries the active T1507 left-rule.
///
/// `entries` is the operator's scan-ordered nav list. Phase 2 passes
/// `&[Screen::Home, Screen::Debug, Screen::Charts]`; Phase 3 inserts
/// `Strategies`, `Risk`, `Audit` between Debug and Charts without
/// changing the widget body — the architect's R1.6 contract.
pub fn view<'a>(
    current_screen: Screen,
    entries: &'a [Screen],
    mode: ThemeMode,
) -> Element<'a, Message>;
```

**Layout.** Vertical `Column` inside a `Container`:
`width(Length::Fixed(SIDEBAR_WIDTH_PX))` where `SIDEBAR_WIDTH_PX =
180.0` (a new constant in `theme::layout`), `height(Length::Fill)`,
`background = PANEL` (Tier 1), 1 px right-edge `BORDER_1` (rendered
the same hairline-Container trick `frame::panel` uses for the header
separator), top padding `space::M (12 px)`, row spacing
`space::S (8 px)`.

**Row composition.** Each entry is a `button` carrying
`Message::SwitchScreen(*screen)` on press, wrapped in
`frame::active_row(content, current_screen == *screen, mode)` so the
T1507 left-rule applies on the selected row (R1.4). The button's
`text_color = FG_2 (default) / FG_1 (active)` so text emphasis
mirrors the rule; **no fill change**, per the active-row pattern.
Hover styling = `PANEL_SUNKEN` row tint (R1.3); an actively-selected
hovered row shows both the rule and the tint, by composition.

**Label source.** Each entry's label string lives in `ui::strings`
as a `SIDEBAR_NAV_*` constant (`SIDEBAR_NAV_HOME = "Home"`,
`SIDEBAR_NAV_DEBUG = "Debug"`, `SIDEBAR_NAV_CHARTS = "Charts"`,
`SIDEBAR_NAV_STRATEGIES`, `SIDEBAR_NAV_RISK`, `SIDEBAR_NAV_AUDIT`).
Architect adds the constants now (Phase 2 uses the first three;
Phase 3 wakes the last three) so Phase 3 doesn't churn `ui::strings`
when extending the entry list. Operator-locked Constraint 2 (no
voice rewrite) is preserved — these are net-new strings, not
rewrites.

**Stateless contract.** The widget never reads from `Cockpit`
beyond `current_screen` and never mutates anything; the message it
emits is the only output. Test by passing each `Screen` variant as
`current_screen` and asserting the rendered active row matches.

### Chart widget contract

**File:** `crates/ui/src/widgets/chart.rs` (new). Canvas-based per
R7.1; the rendering primitive is `iced::widget::canvas` (already in
the iced 0.14 surface — verified at Phase 1 T1503 spike).

**Public API:**

```rust
/// Render the chart for the active `(venue, symbol)` against the
/// 60-bar window. Returns gridlines + line series + markers in one
/// canvas; the canvas honours iced's `tiny-skia` renderer. Empty
/// state (`bars` is empty) renders gridlines + centred "No data"
/// label only.
pub fn view<'a>(
    bars: &'a [Bar],
    markers: &'a [FillView],
    mode: ThemeMode,
) -> Element<'a, Message>;
```

**Coordinate system.** X = time, oldest-left to newest-right. Y =
price, low-bottom to high-top. The drawable region is the canvas's
inner rect minus the gridline-label gutter (`space::S = 8 px` left
and bottom). The Y-axis range = `(min_low, max_high)` over the 60-
bar window with a 5 % padding above and below so markers at the
edge of the printed range are visible.

**Gridline rules.** Five horizontal gridlines, equally spaced
across the price range, drawn in `BORDER_1` at 0.4 alpha (the "1 px
low-alpha horizontals only" rule from
[`spec/ui-design-principles.md` § Charts](../../ui-design-principles.md)).
**No vertical grid** — vertical noise competes with marker
triangles. Gridline labels (the price values) render in `text::MICRO
(11 px)` `FG_3`, right-aligned in the left gutter.

**Default plot style: line series (Q1 ratification).** A polyline
in `ACCENT` connecting `Bar.close` of each bar in the window.
Stroke width 1.5 px. The OHLC candle variant remains supportable
from the same `ChartBuffer` shape (per R7.3 — `Bar` already carries
open/high/low/close); it ships behind a future toggle if the
operator asks. Phase 2's chart widget body is line-only by default;
the OHLC drawing helper is **not** stubbed (no dead code).

**Pan/zoom scope: out of scope (Q2).** The visible window is a
fixed `&bars` slice — the last 60 bars from the buffer. No
scroll-wheel hooks, no click-drag handlers, no preset buttons. The
canvas widget renders what it's given.

**Multi-venue overlay strategy (Q3 + R6.2).** **Single-symbol-per-
chart**: the chart renders only the active `(venue, symbol)`'s
series. No same-canvas overlay of two venues' BTCUSDT lines (an
overlay invites the operator to ask "which venue printed which
line", which the operator should be answering by switching chips,
not by reading two coloured polylines). Cross-venue comparison ships
in Phase 3+ if the operator asks; Phase 2's chip-row IA already
expresses the choice.

**Buy/sell marker drawing.** Buy markers = filled upward triangle
in `UP_500`, 6 px high, anchored at the canvas pixel for `(fill
.venue_ts, fill.price)`. Sell markers = filled downward triangle
in `DOWN_500`, same size, same anchoring. Markers render after the
gridlines and after the line series so the triangle sits visually
above the line at the fill timestamp. Markers outside the visible
window's `(min_ts, max_ts)` clip silently — the marker source query
is bounded to the same window (R8.1) so this is a defence-in-depth
clip, not a normal-path filter.

**Read-only.** No hover tooltips, no click handlers, no mouse
state. Phase 2 chart consumes events only insofar as the symbol-
selector chip row above it (which is a separate widget) emits
`Message::SelectSymbol` — the chart canvas itself is silent.

### ChartBuffer shape

```rust
pub struct ChartBuffer {
    pub series: HashMap<(Venue, Symbol), VecDeque<Bar>>,
}
```

**Capacity per series:** `CHART_BUFFER_CAPACITY = 60` (R10.3). The
constant lives in `state.rs` as a sibling of
`STRATEGIES_RECENT_EVENT_CAP` per the analyst's preferred location
in R10.3 — the alternative location (`theme::layout`, sibling of
`TAPE_MAX_ROWS`) is reasonable too but `theme::layout` already
carries layout primitives, not state-shape primitives. Phase 3 may
consolidate; Phase 2 does not.

**Eviction policy.** `pop_front` on push when at capacity. The
deque is oldest-front, newest-back, which means the canvas paints
left-to-right by iterating `bars()` directly without a `.rev()`.

**Memory bound.** 60 bars × ~200 bytes/Bar × ≤ 20 symbols × ≤ 3
venues ≈ 720 KB worst case (full v1.5b multi-venue universe);
typical fixtures-mode usage is 60 × 200 × 3 = 36 KB. Trivially
within desktop budgets; no compaction needed.

**Message-handler diff.** The existing `Message::BarReceived(bar)`
arm at `crates/ui/src/state.rs:503` becomes:

```rust
Message::BarReceived(bar) => {
    model.last_bar_ts = Some(bar.close_ts);
    model.chart_buffer.push_bar(bar);  // NEW — Phase 2 R10.4
}
```

**Pure-function discipline preserved.** `push_bar` is a pure mutation
on `Cockpit`; no async work, no bus event emitted, no side effect.
Same shape as the existing `model.last_bar_ts = …` write the arm
already performs.

### Audit query extension

**Exact signature (Q4 ratification — analyst recommendation
adopted):**

```rust
/// Phase 2 addition. Return all fills for `(venue, symbol)` inside
/// the half-open interval `[since, until)`, newest-first. Same
/// description-prefixed-rows scan as `recent_fills`; narrower
/// predicate (venue + symbol + time-range vs `recent_fills`'s
/// limit-only).
///
/// Read-only over committed audit data; does not alter any
/// committed report body. Additive — `recent_fills` unchanged.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_fills_filtered(
    ledger: &Ledger,
    venue: Venue,
    symbol: Symbol,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<FillView>, LedgerError>;
```

**Implementation sketch.** The body mirrors `recent_fills` (the
description-prefixed rows scan at `crates/audit/src/query.rs:134`)
with a narrower SQL predicate:

```sql
SELECT id, ts, description
FROM journal_transactions
WHERE (description LIKE 'buy %' OR description LIKE 'sell %')
  AND ts >= ? AND ts < ?
ORDER BY ts DESC, rowid DESC
```

Time bounds are stamped via `since.inner().format(&Rfc3339)` /
`until.inner().format(&Rfc3339)` per the existing `pnl_by_symbol`
precedent (`query.rs:591–598`). Symbol filtering reuses
`extract_symbol_from_description` (`query.rs:648`). **Venue
filtering** — the existing `journal_transactions` rows do not
carry a venue column (the v1.5b T805 schema migration added `venue`
to `strategy_events`, not to `journal_transactions`); the
description format `"<side> <qty> <symbol> @ <price>"` does not
encode venue either. **Phase 2 venue handling:** the function
**accepts** the `venue` argument (so the call-site signature is
forward-stable) but in Phase 2 every fill is treated as
`Venue::Binance` (the v0–v1.5a single-venue assumption holds for
shipped fills today; v1.5b is plumbing-only — `crates/strategy`,
`crates/exec`, `crates/risk`, `crates/cost`, `crates/backtest`,
`crates/reports`, `crates/ui` are unchanged per architecture.md
§ v1.5b architectural deltas at line 2260+, no fills with non-
Binance venues exist on disk yet). The function filters returned
fills to those whose venue matches the argument; in Phase 2 that
set is "all rows when `venue == Binance`, empty when `venue !=
Binance`". Phase 3's Audit screen — when fills from Coinbase /
Kraken venues actually start landing — promotes the filter to read
a `venue` column added to `journal_transactions` via a future
migration (`009_journal_transactions_venue.sql`-shaped); that's
**Phase 3's problem**, not Phase 2's. The `venue: Venue` argument
in the Phase 2 signature is the forward-compat surface so Phase 3
doesn't ripple the call-site.

**Determinism.** `ORDER BY ts DESC, rowid DESC` matches
`recent_journal` precedent at `query.rs:241`. No `f64`; `Decimal`
arithmetic only via the existing `Price` / `Quantity` newtypes.
Empty result returns `Ok(vec![])` (mirrors
`journal_entries_for_transaction` at `query.rs:288`); never `Err`.

**Test scope (Q10 ratification — unit only, integration optional).**

- **Mandatory unit test (R12.7)** in `crates/audit/src/query.rs`
  `mod tests { … }`. Seeds an in-memory `Ledger`, writes 6 fills
  spanning two `(Venue, Symbol)` pairs (3 each, two of each pair
  inside the test window, one outside), asserts that
  `recent_fills_filtered(&ledger, Binance, BTCUSDT, since, until)`
  returns exactly the two BTCUSDT fills inside `[since, until)` in
  newest-first order, and that
  `recent_fills_filtered(&ledger, Binance, BTCUSDT, far_future,
  even_farther_future)` returns `Ok(vec![])`. Test name pattern:
  `recent_fills_filtered_returns_window_subset`,
  `recent_fills_filtered_empty_window_returns_ok_empty`,
  `recent_fills_filtered_distinct_symbols_isolated`.
- **Optional integration test (R12.8)** at
  `crates/audit/tests/recent_fills_filtered.rs` — **not landed in
  Phase 2**. The Phase 3 Audit-screen brief promotes it.

### Fixtures synthetic candles

**API:**

```rust
/// Deterministic OHLC random walk for fixtures-mode chart seeding.
///
/// `seed` controls the random walk; `count` is the number of bars
/// generated. The first bar's `open` defaults to a per-symbol
/// starting price drawn from a small built-in table
/// (`BTCUSDT = 40_000`, `ETHUSDT = 2_400`, `SOLUSDT = 90`); each
/// subsequent bar's `open` equals the previous bar's `close`.
///
/// Per-bar drift = `dec!(0.0)`, per-bar vol = symbol-scaled
/// (`BTCUSDT = 50.0`, `ETHUSDT = 8.0`, `SOLUSDT = 1.5`) so the
/// random walk's amplitude is visually appropriate for each
/// price level. OHLC derived: `open == prev_close`,
/// `close = open + Normal(drift, vol)`,
/// `high = max(open, close) + |Normal(0, vol/2)|`,
/// `low  = min(open, close) - |Normal(0, vol/2)|`.
///
/// Volume is a fixed dec!(12.5) per bar; trade_count is 100.
/// The bar's `open_ts` is `fixed_ts(offset_min * 60)` so the
/// fixtures bin's chart sits at the same epoch the rest of the
/// fixtures share (matches `fake_bar` precedent at
/// `crates/ui/src/fixtures.rs:36`).
#[must_use]
pub fn synthetic_candles(
    seed: u64,
    venue: Venue,
    symbol: Symbol,
    count: usize,
) -> Vec<Bar>;
```

**Seed convention (Q6 ratification — per-symbol).** The fixtures
caller computes:

```rust
fn seed_for(venue: Venue, symbol: &Symbol) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{venue:?}/{symbol}").hash(&mut h);
    h.finish()
}
```

Determinism note: Rust's `DefaultHasher` is **not** guaranteed to
produce the same hash across compiler versions. For Phase 2 this
is acceptable because the snapshot baselines are pinned to a
specific `cargo` invocation per CI run, and the test expectation
is "two calls within the same process produce equal output", not
"the seed equals 0xDEADBEEF". If a future task list needs cross-
version determinism (Phase 3 audit-screen integration test, for
example), promote to `seahash` or a hand-rolled FNV — neither is
in the workspace today, so adding a dep crosses the
library-compat budget for a non-load-bearing field. Phase 2 sticks
with `DefaultHasher`. The acceptance test
`synthetic_candles_deterministic` asserts byte-equal output across
two calls **within the same process**; the
`synthetic_candles_distinct_per_seed` test asserts non-equal
output across two distinct seeds — both work with
`DefaultHasher`'s in-process determinism guarantee.

**RNG.** `ChaCha20Rng::from_seed([u8; 32])` per the architect.md
determinism guardrails (no `thread_rng`, no `f64`). The seed `u64`
is broadcast into the 32-byte ChaCha20 seed by zero-padding
(`seed.to_le_bytes()` followed by 24 zero bytes) — the simplest
shape that gives stable per-seed output without bringing in a
`rand`-side seed-from-u64 helper.

**Fixtures-bin call site.**
`crates/ui/src/bin/cockpit.rs` extends its boot shim:

```rust
let universe: Vec<(Venue, Symbol)> = vec![
    (Venue::Binance, Symbol::new("BTCUSDT")),
    (Venue::Binance, Symbol::new("ETHUSDT")),
    (Venue::Binance, Symbol::new("SOLUSDT")),
];
let mut cockpit = fake_cockpit_ready();  // existing
cockpit.universe = universe.clone();
cockpit.current_screen = Screen::Home;
cockpit.selected_symbol = Some(universe[0].clone());
for (venue, symbol) in &universe {
    let seed = seed_for(*venue, symbol);
    for bar in synthetic_candles(seed, *venue, symbol.clone(), 60) {
        update(&mut cockpit, Message::BarReceived(bar));
    }
}
```

The 1 Hz synthetic feed loop existing in fixtures mode keeps
emitting `Bar`s (one per second per universe symbol) so the chart
in fixtures mode shows visible motion in a recorded run — but
because each `view()` snapshot reads the deque's current contents,
snapshot stability per render frame holds (R11.4).

**Per-symbol fills.** The existing `fake_fill_feed(n)` produces
`BTCUSDT`-only fills at `crates/ui/src/fixtures.rs:90`. Phase 2
extends or wraps it: `synthetic_fills_for(venue, symbol, count) ->
Vec<FillView>` produces `count` fills alternating `Buy`/`Sell` per
the `n % 2 == 0` rule already in `fake_fill_view`, with `symbol`
substituted in. The fixtures-bin boot shim populates the audit
ledger fixture (or, equivalently, pre-seeds `chart_markers =
PanelState::Ready(fills)` directly for fixtures mode — the simpler
path, because fixtures-mode does not run a real `Ledger`) with
≥ 1 buy + ≥ 1 sell per fixtures-universe symbol so V3 (chart
renders ≥ 1 buy + ≥ 1 sell) holds at every fixtures launch.

### Right-rail track reservation

**Q7 ratification: structural now, single `Length::Fixed(0.0)`
column in the shell `Row`.** The shell layout (per R3.1):

```rust
fn shell_view<'a>(model: &'a Cockpit, mode: ThemeMode)
    -> Element<'a, Message>
{
    Row::new()
        .push(sidebar_nav::view(
            model.current_screen,
            &SIDEBAR_ENTRIES_PHASE_2,
            mode,
        ))
        .push(
            Column::new()
                .push(screen_body(model, mode))           // Length::Fill
                .push(status_bar::view(model, mode))      // Length::Fixed(24.0) — Phase 1
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .push(
            // Phase 6 right-rail Assistant slot — zero-width reservation.
            // No widget renders here; no token references it. Phase 6
            // swaps `Length::Fixed(0.0)` to a real width when the v2
            // LLM strategy ships. R13 / Q7 ratified.
            Container::new(Space::new())
                .width(Length::Fixed(0.0))
                .height(Length::Fill),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
```

**No widget renders in the reserved column; no token references
it; no `cfg!(feature = "v2-llm")` gate** (the gate doesn't exist).
Phase 6 lands the actual width and the Assistant widget; Phase 2's
job is to leave the spot.

**`SIDEBAR_ENTRIES_PHASE_2` constant.** A
`pub const SIDEBAR_ENTRIES_PHASE_2: &[Screen] =
&[Screen::Home, Screen::Debug, Screen::Charts];` — a sibling of the
new `SIDEBAR_WIDTH_PX` constant in `theme::layout`. Phase 3 adds
`SIDEBAR_ENTRIES_PHASE_3` (six entries) and the bin call-sites
swap between them based on the cfg/feature in scope at the
moment Phase 3 lands. Phase 2 ships only the Phase 2 constant.

### TD-1 re-evaluation

**Q11 ratification: deferral restated.** Verified at design pass
on disk: `crates/ui/Cargo.toml:50` reads
`iced = { version = "=0.14.0", default-features = false, features =
["tiny-skia", "thread-pool", "advanced"] }`. iced 0.15+ has not
landed; `button::Status::Focused` and `text_input::Style.shadow`
are not available.

**Phase 2 ships no focus-ring upgrade.** Phase 1's deferred state
holds: hover-state ring on the three buttons named in T1504
(kill trigger, kill confirm, modal close); ACCENT border-shift on
the kill confirm input. The deviation remains a known-bounded
ergonomic gap, not a safety gap — the kill-switch destructive
flow is typed-confirm gated, the modal close button is read-only.

**Master-roadmap follow-up flagged.** The TD-1 row at
[`spec/lumen-design-adoption/feature.md` § Cross-phase technical-
debt items](../feature.md#td-1--true-keyboard-focus-ring-phase-1-q11-deviation-ratified-2026-05-04)
should be appended with a 2026-05-04 line under "Promotion
timing" noting:

> "Phase 2 design pass (2026-05-04): iced version on disk verified
> still pinned `=0.14.0`; deferral restated. Next re-evaluation at
> Phase 3 analyst kickoff."

**The architect does not edit the master roadmap directly** (the
roadmap is analyst-owned); the orchestrator routes this as a
follow-up to the analyst on Phase 2 ship.

### Snapshot-baseline strategy

**Q8 ratification — extends Phase 1 Q2 precedent.** Snapshot
baselines refresh in **one `cargo insta accept` pass** at the end
of the Phase 2 dev pipeline (analogous to T1511 from Phase 1).
Expected ripple per the analyst's accounting: ~36 existing
baselines refresh once because every widget moves from a single-
page layout to a screen-routed shell (sidebar present, screen
body padding shifted) + ~5 net-new (sidebar-nav default + sidebar-
nav active variants per screen + chart empty + chart with markers
+ chip-row active-BTC + chip-row active-ETH + Debug screen full)
≈ ~41 baselines.

**Net-new baseline list (locked):**

1. `sidebar_nav__three_entries.snap` — default rendering with
   Home active (matches V7 expectation).
2. `sidebar_nav__active_debug.snap` — Debug active.
3. `sidebar_nav__active_charts.snap` — Charts active.
4. `home_screen__default.snap` — 2×2 grid of PnL + Positions +
   Strategies + Tape under the new shell.
5. `debug_screen__full.snap` — kill + latency + market-health (3
   venues) + server-time + version + logs-stub.
6. `charts_screen__chip_row_active_btc.snap` — chip row with the
   `Binance/BTCUSDT` chip active; chart canvas underneath.
7. `charts_screen__chip_row_active_eth.snap` — same with ETHUSDT.
8. `chart__btc_with_two_buys_one_sell.snap` — fixtures-mode
   line chart + 3 markers (R7 acceptance).
9. `chart__empty_state_no_data.snap` — gridlines + centred "No
   data" label (R7.6 acceptance).

That is 9 net-new (the analyst's "~5" was conservative). The
existing 36 panel snapshots (`crates/ui/tests/snapshots/`) refresh
under the new shell context because each widget's surrounding
chrome differs (sidebar-on, padding shift) — the snapshot summary
files capture chrome explicitly enough that the diff is the
visible Phase 2 artefact at the operator review.

**Two-step accept workflow** (mirrors Phase 1 V5 / T1511):

```
$ cargo test -p ui --features fixtures               # produces .pending-snap files
$ cargo insta review                                  # interactive: inspect each diff
$ cargo insta accept                                  # writes baselines after review
$ cargo test -p ui --features fixtures               # green; no pending files left
```

The ui-designer reviews each diff for the expected pattern: shell
chrome shifts only (sidebar present, padding shift); per-widget
internals byte-identical (no token regression). Anything else
routes back to the developer (most likely a missed widget call-
site under the shell rewiring).

### Cross-feature invariants

Phase 2 column from the master roadmap, re-stated with the design
note:

| Feature                          | Phase 2 invariant note                                                     | How preserved                                                                                                                                                                                                                                                                                                                                                              |
|----------------------------------|----------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `operator-success-reports`       | latency badge moves to Debug screen; colour mapping unchanged              | Phase 1's `theme::color_for_latency_ms` returns `UP_500 / WARN_400 / DOWN_500` per Phase 1 R9.4 / Q8(b); Phase 2 only relocates the rendering host (Debug screen body), not the colour helper. The Phase 2 Debug-screen body composes `widgets::latency::view(&model.latency)` unchanged. R14.1.                                                                                |
| `live-cockpit-unified`           | sidebar-shell wraps cockpit; banner above screen body                      | The halted-banner trigger logic is at `crates/ui/src/state.rs` (file-watch / kill / heartbeat); Phase 2 leaves the trigger untouched. The banner renders **above the screen body, below the title bar**, inside the right-side `Column` of the shell `Row` — visible regardless of `current_screen`. R14.2 / R3.3.                                                            |
| `real-mtm-unrealized-pnl`        | PnL card lives on Home screen; `color_for_delta` unchanged                  | Phase 2 composes the existing `widgets::pnl::view(&model.pnl)` into the Home-screen 2×2 grid (R4.1). No widget code changes; signature unchanged.                                                                                                                                                                                                                            |
| `per-symbol-position-accounts`   | Positions widget lives on Home screen; row contract unchanged              | Same composition story — `widgets::positions::view(&model.positions)` slots into the Home grid. Strategy-id chip styling preserved.                                                                                                                                                                                                                                          |
| `tape-row-audit-modal`           | Tape lives on Home screen; modal trigger preserved                          | The tape widget renders in the Home-screen body's bottom-right cell. Tape-row-clicks emit `Message::TapeRowClicked(tx_id)` exactly as Phase 1; the modal opens above any screen (the modal is wrapped at the shell level, not the screen level — R3.3 banner-shape generalised). R14.5.                                                                                       |
| `journal-tx-metadata`            | modal-header rendering unchanged                                            | `widgets::journal_transaction_modal::view` body is a black box to Phase 2 — the metadata reader and modal-header rendering are untouched.                                                                                                                                                                                                                                    |
| `v1.5b-multi-venue`              | venue dimension on Debug + chip row                                         | `Cockpit::market_health` (the v1.5b-introduced field) drives the per-venue rows on Debug (R5.3). The chip row reads from `Cockpit::universe` (boot-populated from `Config.data.sources` × `Config.universe.usdt_symbols` per the v1.5b config shape). The existing `cockpit_live` venue-tagged tick rendering is untouched. R14.7.                                                |

**Acceptance:** the tester's per-feature invariant table in the
Phase 2 report shows PASS for all 7 rows.

### Anchor regression

**Zero anchor risk re-affirmed.** The design pass found no path
where Phase 2 touches committed report bodies:

- `recent_fills_filtered` is **read-only over already-committed
  audit rows** — the description-prefixed-rows scan
  `recent_fills(limit)` already iterates the same
  `journal_transactions` table; the new function adds no writer,
  no schema migration, no description-format change. Anchor body
  hashes are byte-stable by construction. R12.5 + R15.2 lock the
  invariant; the unit test in R12.7 verifies the new function
  returns the expected subset of the existing rows.
- The `Screen` enum, `ChartBuffer`, `sidebar_nav` widget, `chart`
  widget, and shell rewiring are all UI-only additions; no
  strategy / exec / risk / cost / backtest / reports crate is
  touched.
- The fixtures-mode `synthetic_candles` helper is gated behind
  `#[cfg(feature = "fixtures")]` in `crates/ui/src/fixtures.rs`;
  it cannot reach a backtest replay (the backtest crate does not
  depend on `crates/ui`).

**Verify-anchors gate at the Phase 2 tester run** must report
11 / 11 PASS with byte-identical bodies. The `R16.3`-equivalent
grep gate from Phase 1 (`grep -rni "lumen\|panel-raised\|panel-
sunken\|cool-800" spec/reports/`) remains zero — Phase 2 adds no
new rendered prose to any committed report.

### Implementation parallelism map

```
T1601 (Screen + ChartBuffer state — foundation gate, sequential)
  └─ T1602 (sidebar_nav widget — parallel after T1601)
  └─ T1603 (shell rewiring — parallel after T1602 lands)
        ├─ T1604 (Home screen body — parallel after T1603)
        ├─ T1605 (Debug screen body — parallel after T1603)
        ├─ T1606 (recent_fills_filtered — parallel; audit crate, no UI dep)
        ├─ T1607 (synthetic_candles fixtures — parallel)
        ├─ T1608 (chart widget canvas — parallel after T1601)
        ├─ T1609 (chip row active-bottom variant — parallel after T1602)
        └─ T1610 (Charts screen body wiring — sequential after T1606+T1608+T1609)
              │
              ▼
        T1611 (right-rail reservation — sequential after T1603)
        T1612 (universe boot wiring both bins — sequential after T1603)
        T1613 (snapshot refresh + accept — sequential after every visual lands)
        T1614 (cross-feature invariants verify)
        T1615 (anchor regression + R16.3 grep)
        T1616 (rust-validate + both bins launch)
              │
              ▼
        T_FINAL_LUMEN_PHASE_2 (tester gate — VERDICT → presenter on PASS)
```

T1601 is the foundation gate (state additions) — every later task
depends on its types existing. T1606 (audit query) can technically
run in parallel from T1601 because it lives in `crates/audit`, not
`crates/ui`, but the chart's marker fetch (T1610) depends on
T1606's signature. T1613 (snapshot accept) is the narrow point.

## Implementation

_developer fills this — task list at
[`spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md`](tasks.md)._

## Verification — links

_tester fills this — links to
`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-<timestamp>-lumen-phase-2-shell-ia-charts.md`._

## UI

_ui-designer fills this — links to refreshed snapshots and
the Phase 2 presentation under `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md` (phase-2-shell-ia-charts section)._

## Changelog

- 2026-05-04 (architect): appended `## Design`. Q1–Q11 ratified
  (line series default, pan/zoom deferred, `Cockpit::universe` boot-
  populated, `since/until` two-arg query signature, chip row uses
  bottom-rule T1507 variant, per-symbol synthetic-candle seed via
  `DefaultHasher` in-process, right-rail reserved structurally as
  `Length::Fixed(0.0)`, two-field session-scoped persistence, Debug
  logs as placeholder, audit-query unit test only in Phase 2,
  TD-1 deferred — iced still pinned `=0.14.0` on disk). 11/11
  ratified; zero principled overrides. Cockpit state diff specified
  — `Screen` enum (six variants — Phase 2 wires Home/Debug/Charts;
  Phase 3 declares Strategies/Risk/Audit so Phase 3 doesn't have to
  alter the enum), `current_screen`, `universe`, `selected_symbol`,
  `chart_buffer`, `chart_markers` field additions; `Message::SwitchScreen
  / SelectSymbol / ChartMarkersLoaded` variant additions;
  `Message::BarReceived` arm extended to push into `chart_buffer`.
  Sidebar nav widget contract (stateless, T1507 active row, label
  strings declared in `ui::strings` for Phase 3 forward-compat).
  Chart widget contract (canvas-based, line series in `ACCENT`, no
  vertical grid, single-symbol-per-chart, read-only). `ChartBuffer`
  capacity 60 with pop-front eviction. `recent_fills_filtered` exact
  signature with Phase 2 venue-handling note (Binance-only on disk
  per v1.5b plumbing-only; Phase 3 extends with a
  `journal_transactions.venue` migration). `synthetic_candles` API
  + per-symbol seed helper. Right-rail track reserved as a third
  `Row` column with `Length::Fixed(0.0)`. TD-1 deferral flagged
  for master-roadmap follow-up by orchestrator (architect doesn't
  edit master). Snapshot ripple budget: ~36 refresh + 9 net-new =
  ~45 baselines (analyst's "~5 net-new" revised up to 9). Cross-
  feature invariants table re-stated (7 rows). Zero anchor risk
  re-affirmed. Implementation parallelism map: T1601 foundation
  gate → fan-out across T1602–T1610 → narrow at T1613 snapshot
  accept → T_FINAL. Task list at
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md`](tasks.md)
  with 16 T16xx tasks + tester `T_FINAL_LUMEN_PHASE_2` gate.
  HANDOFF → developer ‖ ui-designer (developer takes T1601–T1616
  implementation; ui-designer takes the visual-diff attestation
  at T_FINAL after the developer's snapshot refresh pass).
- 2026-05-04 (analyst, Phase 2 kickoff expansion): expanded
  the 2026-05-04 stub into the full analyst brief — 15
  R-items grouped into 7 clusters (R1–R3 sidebar + shell;
  R4–R5 Home + Debug screens; R6–R9 Charts screen + chart
  widget; R10–R11 chart data sources; R12 audit query
  extension; R13 right-rail reservation; R14–R15 invariants
  + anchors), 12 V-items mapping cleanly onto R-clusters,
  9 acceptance criteria each tracing to its R-cluster, and
  11 architect Q-items (Q1 default plot style, Q2 pan/zoom
  scope, Q3 universe source, Q4 audit-query exact signature,
  Q5 chip-row active rule placement, Q6 synthetic-candle
  seed convention, Q7 right-rail structural-now-vs-deferred,
  Q8 nav-state persistence implementation, Q9 Debug
  logs/metrics scope, Q10 audit-query test scope, Q11 TD-1
  re-evaluation). Master roadmap operator-locked decisions
  Q11–Q14 inherited as not-re-opened. Anchor risk
  reaffirmed as **zero** (read-only audit query extension
  + UI shell + new widget). Snapshot ripple budget: ~36
  refreshed + ~5 net-new = ~41 baselines, accepted in one
  `cargo insta review` pass per Phase 1 Q2 precedent. Brief
  status `queued` → `active`; owner unchanged
  (analyst → architect at HANDOFF). HANDOFF → architect.
- 2026-05-04 (analyst, master-roadmap revision): stub
  created at the 6-phase roadmap revision. Full brief
  expansion deferred to Phase 2 kickoff per master Q3
  (per-phase analyst spawn).
