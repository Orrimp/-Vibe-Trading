---
slug: chart-buy-sell-emphasis
status: shipped
owner: architect
updated: 2026-05-11
version: 1.9.0
---

# Chart buy/sell emphasis

## Why

This brief promotes operator feedback from the **2026-05-10 cockpit
review** into a real feature. The operator opened the **Charts** screen
shipped by Lumen Phase 2
([`spec/lumen-design-adoption/phase-2-shell-ia-charts/feature.md`](../lumen-design-adoption/phase-2-shell-ia-charts/feature.md)
T1610) and said, verbatim:

> "I want to visually see (maybe green and red arrow) when the strategy
> is buying and when it is selling. Also the current amount of buy and
> sell. This will help me as human to determine if the strategy buys at
> the right time."

The Phase 2 chart already renders buy/sell triangles — but the operator
identified that they are **present and visually broken in three ways**
AND **missing one piece of functionality**:

1. **Too small to see.** `MARKER_SIZE_PX = 6.0` filled triangles
   ([`crates/ui/src/widgets/chart.rs:39`](../../crates/ui/src/widgets/chart.rs))
   disappear against the 1-pixel `ACCENT` price line at typical
   cockpit density on a 60-bar window.
2. **Hidden under the line.** The line stroke is drawn **after** the
   markers
   ([`crates/ui/src/widgets/chart.rs:124-167`](../../crates/ui/src/widgets/chart.rs)),
   so the polyline literally covers the triangles. **Z-order**
   (the layering order of overlapping draw passes — later passes paint
   on top) needs flipping.
3. **Floating off the line.** Marker `y` is computed from
   `fill.price.get()`
   ([`crates/ui/src/widgets/chart.rs:156`](../../crates/ui/src/widgets/chart.rs))
   — the actual execution price — rather than from where the price
   line sits at the marker's `x`. Result: a fill that printed at a
   different price than the bar close (slippage, mid-bar tick) renders
   above or below the line with no visual anchor. The operator's
   question is "did the strategy buy at the right **time**?" — timing
   is the x-axis. Vertical anchoring to the line gives the visual
   anchor for that timing; the actual fill price lives in the tooltip.
4. **No way to inspect a marker.** The operator can see "a buy
   happened around bar 32" but cannot answer "at what price, what
   size, what notional, what strategy?" without leaving Charts and
   grepping the audit ledger. The
   [tape-row → audit modal](../tape-row-audit-modal/feature.md) ships
   click-through-to-journal-entries from the **tape**; the equivalent
   on the **chart** is missing.

The operator's **layered marker** ask ("strategy signals before
risk-clamping, alongside executed fills") closes a fifth gap — the
visual difference between "what the strategy wanted to do" and "what
the risk engine actually let through". Today only **executed fills**
are queryable (`audit::query::recent_fills` /
`recent_fills_filtered` at
[`crates/audit/src/query.rs:178`](../../crates/audit/src/query.rs) /
`:223`). Strategy signals — the per-bar pre-risk-clamp intent —
are transient: emitted by the `Strategy::on_bar` arm, consumed by the
risk engine, never persisted. **Surfacing them is the load-bearing
architect question** for this brief (Q1).

This is the **first feature against the Charts screen since Phase 2
shipped 2026-05-05**. It is **pure UI + (possibly) one additive audit
query**. No strategy logic changes; no edge claim; **no impact on the
9 locked strategy-backtest body-SHA-256 anchors** in
[`spec/anchors.toml`](../anchors.toml). The two operator-success-report
anchors (`report-sample-7d` / `report-sample-90d`) likewise stay
byte-identical: this feature touches no report-rendering code path.

**Terms-of-art (one-line glosses, used throughout):**

- **Z-order** — the layering order of overlapping canvas draw passes;
  later passes paint on top of earlier ones.
- **Snap-to-line** — placing a marker's `y` at the polyline's `y` at
  the marker's `x` instead of at the marker's intrinsic `y` value.
- **Ghost layer** — a faded/reduced-opacity render of "what could have
  happened" (strategy signals) under the bold render of "what did
  happen" (executed fills).
- **Hover tooltip** — a small overlay that appears when the pointer
  rests over a target; surfaces metadata without a click.
- **Hit-rect** — an invisible interactive rectangle around a small
  visual target (a 14-px triangle gets a ~28-px square hit-rect for
  pointer forgiveness, per Fitts's law).
- **RFC3339** — the ISO-8601 timestamp profile the cockpit and audit
  ledger use (e.g. `2026-05-09T13:42:18Z`).
- **In-memory signal buffer** — an ephemeral ring buffer in the cockpit
  process holding the last N strategy signals; not persisted.
- **Body-SHA-256** — the deterministic body-only hash of a report,
  locked into the regression gate per
  [operator-success-reports R10.3](../operator-success-reports/feature.md#r10--determinism-body-vs-front-matter-discipline).

**Version proposal: `1.9.0`** — continues the main v1.x feature line
that the cockpit is currently on (reflection-memory shipped at
`1.8.0`; the in-flight `1.8.x` slot is occupied). This is **not** a
Lumen 2.x phase: Lumen Phases 1–5 ship as a sequential, operator-locked
roadmap with Phase 6 (Assistant slot) reserved for v2 LLM; inserting a
"Phase 2.5" mid-stream re-litigates the phase order. A
chart-improvement feature on the main version line is the natural fit
and matches the precedent of tape-row-audit-modal landing at `1.6.0`
between Phase 2 (`2.1.0`) and Phase 3 work without claiming a Lumen
phase slot.

## Requirements

Numbered, testable, derived from the seven operator-confirmed scope
decisions in the orchestrator's spawn message + the existing Phase 2
chart shape at
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs)
+ [`crates/ui/src/screens/lab.rs`](../../crates/ui/src/screens/lab.rs).
Each ends with a one-line **Acceptance** clause the tester can verify by
running a specific command.

All R-items preserve the `Strategy` trait shape (no trait changes), the
audit chart of accounts (no new accounts), the existing `strategy_events`
schema (additive only — see Q1), the public `audit::query::*` surface
(additive only), and the 11 locked anchor SHA-256s (no impact). This
feature **adds no new bus channels** in the recommended R5 resolution
(architect re-confirms at Design time) and consumes **no LLM tokens**.

### R1 — Markers visually obvious

- **R1.1** Default triangle size grows from `MARKER_SIZE_PX = 6.0`
  ([`chart.rs:39`](../../crates/ui/src/widgets/chart.rs)) to the
  architect-picked Lumen-system value — analyst strawman **`13.0`**
  (≈2.2× area). Architect confirms (Q6).
- **R1.2** Marker edge contrast: the triangle gets a **1-px outline**
  in `BORDER_STRONG` (dark mode: `#3A4456`; the token shipped with
  [tape-row-audit-modal R6 / Q3](../tape-row-audit-modal/feature.md#r6--theme-9-shipped-tokens--proposed-info--border_strong--bg_overlay)).
  This separates the marker from both the price line (drawn in
  `ACCENT`) and the panel background.
- **R1.3** Buy-marker fill remains `color::UP_500.current(mode)`;
  sell-marker fill remains `color::DOWN_500.current(mode)`. No new
  fill tokens. Light/dark parity follows the existing token swap.
- **R1.4** Markers re-render correctly across the **60-bar / 60-minute**
  default window the Phase 2 chart ships (`ChartBuffer` cap 60 at
  [`crates/ui/src/state.rs:649`](../../crates/ui/src/state.rs)) at
  cockpit's default panel size; no overlap-clipping artifacts for
  three closely-spaced markers (test fixture: 3 fills within
  consecutive bars).
- **Acceptance:** updated insta snapshot
  `chart__btc_with_two_buys_one_sell` reflects the new
  `MARKER_SIZE_PX` constant; manual cockpit run on the 60-bar fixture
  shows triangles distinctly larger and visually separable from the
  line at default zoom.

### R2 — Markers above the price line in z-order

- **R2.1** Re-order the `ChartProgram::draw` body
  ([`crates/ui/src/widgets/chart.rs:79-167`](../../crates/ui/src/widgets/chart.rs))
  so the marker-fill pass runs **after** the line-stroke pass. New
  pass order: gridlines → axis labels → line stroke → ghost-signal
  triangles (R5) → executed-fill triangles → tooltip overlay (R4
  on-demand).
- **R2.2** No new `Geometry` per pass — single `Frame::into_geometry`
  call preserved; only the order of `frame.stroke` / `frame.fill`
  calls changes.
- **Acceptance:** a unit test renders a single buy fill whose price
  equals the bar `close` at that index (so marker and line are
  visually coincident) and asserts — via the existing chart-summary
  snapshot helper extended with a `draw_order:` line — that the
  draw-order field reads `gridlines,labels,line,ghosts,fills`.

### R3 — Marker y anchored to the price line

- **R3.1** Marker `y` is computed from the **polyline's y at the
  marker's x**, not from `fill.price.get()` (current line 156). The
  fill price moves to the tooltip (R4).
- **R3.2** Snap method: **linear interpolation** between the
  bracketing bars' `close` prices, expressed in y-space (Q2 — analyst
  recommends (b) interpolation over (a) nearest-bar).
- **R3.3** Edge cases:
  - `fills` outside the visible `[min_ts, max_ts]` window: continue
    to be filtered out (existing line 149-153 defence-in-depth clip).
  - `fills` exactly at `min_ts` (first bar): y = bar[0].close in
    y-space.
  - `fills` exactly at `max_ts` (last bar): y = bar[N-1].close in
    y-space.
  - Empty bar window: nothing rendered (existing empty-state path).
- **R3.4** **Buy triangles point up; sell triangles point down** —
  unchanged from the current `(side, upward)` match at
  [`chart.rs:158-161`](../../crates/ui/src/widgets/chart.rs). The
  triangle's *anchor point* (the centroid for the snap math) sits on
  the line; the apex protrudes above (Buy) or below (Sell). Operator
  reads "up = green = buy" and "down = red = sell" at a glance.
- **Acceptance:** a unit test against a fixture where a fill's
  `venue_ts` falls exactly midway between two bars whose `close`
  differs by `dec!(100)` asserts the rendered marker `y` equals the
  interpolated midpoint y-pixel ± 0.5px (rounding tolerance for
  `f32`).

### R4 — Hover tooltip on each marker

- **R4.1** Pointer hover over a marker surfaces a **tooltip overlay**
  with fill metadata. iced canvas has no native hover behaviour; the
  implementation shape is architect's call (Q3 — analyst recommends
  (b) custom pointer-tracking + custom-drawn tooltip overlay).
- **R4.2** Tooltip content fields (analyst strawman; operator confirms
  Q4):
  - **Side** — `Buy` or `Sell` badge (`UP_500` / `DOWN_500` background,
    `FG_1` text).
  - **Price** — fill price in USDT, 4 decimals (e.g. `52,341.2000`).
  - **Quantity** — base asset, 4 decimals (e.g. `0.4000` BTC).
  - **Notional** — `price × qty` in USDT, 2 decimals
    (e.g. `20,936.48 USDT`).
  - **Timestamp** — RFC3339 UTC (e.g. `2026-05-09T13:42:18Z`).
  - **Transaction ID** — truncated to 8 chars + `…` (full UUID
    available via R4.5 click-through).
  - **Strategy ID** — `strategy_id` if surfaceable from
    `FillView`/`JournalTransactionMetadata`; otherwise `—`.
- **R4.3** **Hit-rect** is a 28-px square centered on the marker
  centroid (~2× the triangle bounding box, per Fitts's-law forgiveness
  for ~13-px targets).
- **R4.4** Tooltip positioning: prefers above-and-right of the
  marker; flips to below-and-left if the marker is in the upper-right
  quadrant of the canvas (avoid clipping at the canvas edge).
- **R4.5** Click on a marker (not just hover) opens the same modal
  the tape uses — `Message::TapeRowClicked(transaction_id)` from
  [tape-row-audit-modal R1](../tape-row-audit-modal/feature.md#r1--tape-rows-clickable-emit-messagetaperowclickedtransaction_id).
  Reuses the shipped `widgets::journal_transaction_modal`. **No new
  modal widget.**
- **R4.6** Tooltip dismissal: pointer leaves the hit-rect.
- **R4.7** Tooltip strings (all via `ui::strings`, zero inline per the
  existing
  `no_inline_user_visible_strings_in_widgets`
  [`crates/ui/tests/consistency.rs`](../../crates/ui/tests/consistency.rs)
  guard):
  - `CHART_TOOLTIP_SIDE_BUY = "Buy"`
  - `CHART_TOOLTIP_SIDE_SELL = "Sell"`
  - `CHART_TOOLTIP_PRICE_LABEL = "Price"`
  - `CHART_TOOLTIP_QTY_LABEL = "Quantity"`
  - `CHART_TOOLTIP_NOTIONAL_LABEL = "Notional"`
  - `CHART_TOOLTIP_TS_LABEL = "Time"`
  - `CHART_TOOLTIP_TX_LABEL = "Tx"`
  - `CHART_TOOLTIP_STRATEGY_LABEL = "Strategy"`
  - `CHART_TOOLTIP_STRATEGY_NONE = "—"`
  - `CHART_TOOLTIP_GHOST_BADGE = "Signal (not executed)"` — for the
    R5 ghost-layer tooltip variant.
- **Acceptance:** an integration test injects a synthetic pointer
  event at a known marker's hit-rect, asserts `cockpit.chart_tooltip
  == Some(ChartTooltipView { … })` with the expected field values
  matching the fill fixture; a panel snapshot
  `chart_tooltip_buy_paper_fill.snap` captures the rendered overlay
  in compact density on dark mode.

### R5 — Layered marker source: fills + signals

- **R5.1** The chart renders **two layers**:
  - **Ghost-signal layer** (back, rendered before the fill layer per
    R2.1 pass order): strategy intent before risk-clamping. Smaller
    (≈ 60% of fill-marker size — analyst strawman `8.0` if fill is
    `13.0` per R1.1), at 60% opacity, no outline, no drop shadow.
  - **Executed-fill layer** (front): per R1 + R2 + R3 + R4.
- **R5.2** **Signal source plumbing is architect's call.** Three
  options surfaced as Q1 (a) new audit log row, (b) in-memory signal
  buffer in the cockpit, (c) replay-from-backtest only. Analyst
  recommends (a) **new audit log row** behind a config gate
  (`enable_signal_log = false` default for v1.9, opt-in flipped by
  this feature's ship). R-items below assume (a); if architect picks
  (b) or (c), R5.3 / R5.4 / R5.5 morph accordingly and the brief
  re-anchors (architect annotates the changes in the Design section).
- **R5.3** **(Assumes Q1 = (a).)** New `audit::query::recent_signals`
  reader, sibling of `recent_fills_filtered`:
  ```rust
  pub async fn recent_signals(
      ledger: &Ledger,
      venue: Venue,
      symbol: &Symbol,
      since: Timestamp,
      until: Timestamp,
  ) -> Result<Vec<SignalView>, LedgerError>;
  ```
  `SignalView` is a new `core` type: `{ symbol, side, intended_qty,
  signal_ts, strategy_id, was_clamped: bool, clamp_reason: Option<…> }`.
  Architect picks the exact shape + home.
- **R5.4** Cockpit state gains `chart_signals: PanelState<Vec<SignalView>>`,
  sibling of `chart_markers` at
  [`crates/ui/src/state.rs:654`](../../crates/ui/src/state.rs). Same
  Loading/Ready/Error tri-state shape.
- **R5.5** Signal-emit path: the agent's `strategy_events` table
  ([`crates/audit/migrations/`](../../crates/audit/migrations))
  gains a new `kind` value (`signal_emitted`) OR a separate
  `strategy_signals` table — architect's call (Q1 sub-question).
  Additive; existing rows untouched.
- **R5.6** Ghost-marker hover renders a **distinct** tooltip variant
  — same fields where applicable (`Side`, `Quantity`, `Timestamp`,
  `Strategy ID`) plus the `CHART_TOOLTIP_GHOST_BADGE` string + the
  `was_clamped` / `clamp_reason` if non-`None`. **No `Price` field**
  (signals carry no price — they're emitted at signal-eval-time, not
  fill-time). **No `Notional` field** (no price to multiply).
- **R5.7** **Config gate.** `enable_signal_log: bool` in the agent's
  TOML, default `false`. When `false`: no signal rows written, no
  ghost layer rendered (cockpit reads empty `Vec<SignalView>`,
  renders no ghosts; `chart_signals: PanelState::Ready(vec![])`).
  When `true`: signal rows written on every strategy emission;
  ghost layer rendered. **Audit-ledger size growth** flagged as
  Q1's load-bearing concern: with 4 active strategies × 60 bars/hour
  × 24 hours × 30 days = ≈173k signal rows/month vs typical fills
  volume in the low hundreds/month. Operator opts in knowing the
  budget cost.
- **Acceptance (Q1 = (a) path):** unit test against a fixture ledger
  with deliberately-injected `signal_emitted` rows asserts
  `recent_signals` returns the expected `Vec<SignalView>`; integration
  test asserts the chart renders both layers (count of ghosts +
  count of fills matches expectations); `enable_signal_log = false`
  default path produces zero ghost-layer renders and zero new audit
  rows (regression-safe for operators who don't opt in).

### R6 — Keep the triangle shape

- **R6.1** Geometry stays a filled **triangle** (existing `draw_triangle`
  helper at
  [`chart.rs:197-213`](../../crates/ui/src/widgets/chart.rs)). No
  stem-arrow, no chevron, no circle-with-letter, no SVG icon.
- **R6.2** Visual treatment per R1 (bigger + outline) is the upgrade
  envelope; the primitive shape stays a triangle. **Operator-locked.**
- **R6.3** Optional **drop shadow** for the fill-marker layer
  (1.5-px subtle offset, Lumen "whisper shadow" — architect picks the
  exact alpha + offset per Q6). Ghost layer = no shadow.
- **Acceptance:** `draw_triangle` helper retains its current
  signature shape (`fn draw_triangle(frame: &mut Frame, anchor: Point,
  color: Color, upward: bool)` with optional new outline/shadow
  parameters); the snapshot helper's `marker_shape: triangle` line
  stays present in `chart__btc_with_two_buys_one_sell`.

### R7 — Three counter views alongside the chart

- **R7.1** **Cumulative window volume tile** (R7a). Two number-pair
  tiles rendered next to or above the chart (layout Q5):
  - **Buys this window** — `+$X.XX (n trades)` in `UP_500`.
  - **Sells this window** — `−$Y.YY (m trades)` in `DOWN_500`.
  - **Net** — `$Z.ZZ` colored by sign.
  Source: sum over `chart_markers`
  ([`state.rs:654`](../../crates/ui/src/state.rs)) of `price × qty`
  per side, restricted to the visible `[min_ts, max_ts]` window.
  Reuses the `widgets::kpi_strip` widget shape (current consumers in
  [`crates/ui/src/widgets/kpi_strip.rs`](../../crates/ui/src/widgets/kpi_strip.rs)
  emit number tiles with label + value + color — same surface).
- **R7.2** **Per-bar volume histogram** (R7b). Below the chart
  canvas. Per bar: two stacked bars — green for cumulative buy USD
  volume in that bar, red for cumulative sell USD volume in that
  bar. Bar width matches the chart's bar spacing. Fixed height
  (analyst strawman `80px`; architect Q7 picks the exact widget
  shape — reuse `sparkline` or new `volume_histogram` widget).
- **R7.3** **Open position mirror** (R7c). Reuses the existing
  `widgets::positions` widget already shown on the Home screen
  ([`crates/ui/src/screens/home.rs`](../../crates/ui/src/screens/home.rs)
  — and the `positions: PanelState<Vec<OpenPosition>>` field on
  `Cockpit`). Filters to the **active symbol only** (the one the
  chart is showing). Layout: per Q5; analyst strawman is a single
  one-row strip above or below the chart.
- **R7.4** Tiles + histogram + position mirror are **read-only**.
  No order entry, no edits, no audit writes. Cockpit invariant
  preserved.
- **R7.5** **Symbol-switch reactivity.** All three counter views
  re-compute when the operator switches the active symbol via the
  chip row (`Message::SelectSymbol`). `chart_markers` already
  re-fetches; the tile + histogram derive from `chart_markers` so
  they update mechanically. The position mirror re-filters
  `model.positions` to the new active symbol.
- **R7.6** **Empty state.** If the visible window contains zero
  buys, zero sells, or no open position: render `"—"` placeholders
  (not blank space; principles "No blank screens"
  [§](../ui-design-principles.md#no-blank-screens)).
- **R7.7** Strings (all via `ui::strings`, zero inline):
  - `CHART_VOLUME_TILE_BUYS_LABEL = "Buys in window"`
  - `CHART_VOLUME_TILE_SELLS_LABEL = "Sells in window"`
  - `CHART_VOLUME_TILE_NET_LABEL = "Net"`
  - `CHART_VOLUME_TILE_TRADES_SUFFIX = " trade(s)"` (or use `widgets::num`
    pluralisation helper if architect prefers).
  - `CHART_VOLUME_HISTOGRAM_LABEL = "Per-bar volume (USDT)"`
  - `CHART_POSITION_MIRROR_LABEL = "Open position"`
  - `CHART_POSITION_MIRROR_NONE = "No open position in this symbol."`
- **Acceptance:** insta snapshot
  `charts_screen_with_counters_and_chart.snap` captures the rendered
  Charts screen with: 3 buys + 2 sells in the window, an open
  long-BTC position, and the per-bar histogram populated. Unit
  test on the tile-arithmetic asserts `buys_usdt + sells_usdt`
  sum matches a hand-computed value against a 3-buy 2-sell fixture.

### R8 — Charts-screen layout reshape

- **R8.1** Layout for the chart + the three counter views per the
  operator's **Q5 layout choice** (operator-decide, analyst
  recommends (β)).
- **R8.2** **Analyst's recommended (β) layout:**
  ```
  ┌──────────────────────────────────────────────────────────┐
  │ chip row (symbol selector, existing)                     │
  ├──────────────────────────────────────────────────────────┤
  │ R7.1 tile strip │ R7.3 open-position strip               │
  ├──────────────────────────────────────────────────────────┤
  │                                                          │
  │ price chart (R1–R6)                                      │
  │                                                          │
  ├──────────────────────────────────────────────────────────┤
  │ R7.2 per-bar volume histogram (80px tall)                │
  └──────────────────────────────────────────────────────────┘
  ```
- **R8.3** The chart canvas remains `Length::Fill` for the central
  region; the surrounding strips are `Length::Shrink`. Vertical
  space budget on a 900-px-tall cockpit window: chip row ~32 px,
  tile + position strip ~56 px, chart fills middle (≈700 px),
  histogram fixed 80 px, plus padding. No horizontal squeeze on the
  chart (regression-safe vs the Phase 2 layout, which gave the chart
  full width).
- **R8.4** Layout adapts to operator's Q5 pick at architect resolution
  time. Five plausible layouts enumerated in Q5.
- **Acceptance:** rendering the Charts screen at the default cockpit
  window size produces no overlapping widgets, no clipped tile labels,
  and the chart canvas height stays > 50% of the screen body height.

### R9 — Determinism + read-only + bus-channel invariants

- **R9.1** **No new bus channels** in the recommended Q1 = (a) path.
  Signal-emit writes into the existing audit ledger; cockpit reads
  via the existing audit-query subscription pattern (precedent at
  [architecture.md → Cockpit ← `audit::query`](../architecture.md#cockpit--auditquery)).
- **R9.2** Cockpit stays **read-only** vs the audit ledger
  (operator-success-reports invariant T802 / T805 / live-cockpit-unified
  invariant T906–T908). No new write surfaces from the chart click /
  hover.
- **R9.3** Tooltip + counter views are **derived state**: they
  compute from `chart_markers` / `chart_signals` / `positions` on
  every render. No new persisted state.
- **R9.4** **Anchor risk: zero.** This feature touches no
  strategy code, no risk engine code, no backtest engine, no report
  rendering. The 11 anchored reports (9 backtest + 2 operator-success)
  stay byte-identical.
- **Acceptance:** `bash scripts/verify_anchors.sh` outputs
  `ANCHORS PASS  (11 / 11)`, zero diffs vs `spec/anchors.toml`
  (regression gate, V-item V8 below).

### R10 — Consistency tests stay green

- **R10.1** `no_inline_hex_colors_in_widgets_or_state` — every new
  marker, tile, histogram, tooltip, and overlay colour flows from
  `theme::color::*`. No `#hex` literal anywhere in `crates/ui/src/`.
- **R10.2** `no_inline_user_visible_strings_in_widgets` — every
  new string from `ui::strings::*`. R4.7 + R7.7 enumerate the new
  constants.
- **R10.3** `Message::*` exhaustiveness — adding
  `Message::ChartMarkerHovered(usize)`,
  `Message::ChartMarkerHoverEnded`,
  `Message::ChartMarkersLoaded(...)`,
  `Message::ChartSignalsLoaded(...)` (architect picks exact arms;
  Q3 may collapse some). No `_ =>` catch-all.
- **Acceptance:** `cargo test -p ui --test consistency` passes;
  `grep -rn '#[0-9a-fA-F]\{6\}' crates/ui/src/widgets/chart.rs
  crates/ui/src/widgets/volume_histogram.rs
  crates/ui/src/widgets/chart_tooltip.rs` returns zero hits.

### R11 — Cross-feature invariants must hold

- **R11.1** **Lumen Phase 1** focus-ring + status-bar contract
  (T1507 active-row pattern, always-visible status bar at
  [`crates/ui/src/widgets/status_bar.rs`](../../crates/ui/src/widgets/status_bar.rs))
  unaffected — this feature touches no shell-level widgets.
- **R11.2** **Lumen Phase 2** chart-buffer rolling-60-bar shape
  ([`state.rs:649`](../../crates/ui/src/state.rs)) unchanged.
- **R11.3** **tape-row-audit-modal** modal pattern reused
  unchanged; this feature is its **second consumer** (the first
  was the tape; the chart marker is the second). Per
  [tape-row-audit-modal Q7](../tape-row-audit-modal/feature.md#q7--generic-vs-specific-modal-widget):
  *"specific until a third consumer materialises (principles
  three-uses rule). Refactor on the third."* This feature does
  **not** refactor — `widgets::journal_transaction_modal` stays
  modal-specific.
- **R11.4** **operator-success-reports** R10/R11 invariants — no
  changes to report-rendering code; both `report-sample-*` anchors
  byte-identical.
- **R11.5** **reflection-memory** card-write + retrieval surface
  untouched. Memory highlights body unchanged.
- **Acceptance:** `cargo test --workspace` + `cargo test -p ui
  --features live` + `bash scripts/verify_anchors.sh` all green
  (regression sweep — see V8).

### R12 — Backtest-viewer parity scope

- **R12.1** **This feature is cockpit-only.** The `viewer` binary
  (backtest report viewer at
  [`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs))
  does NOT inherit the new chart shape in v1.9.0. Operator decides
  (Q8 — analyst recommends (b) cockpit-only, viewer parity is a
  follow-up brief).
- **R12.2** If operator picks Q8 = (a), R12.1 inverts and a new
  R-item is added at Design time: the viewer's existing
  `KPIStrip + EquityCurve + DrawdownBand` composition extends with
  a `Charts`-style price-with-markers view. This is a significant
  scope addition; analyst flags it as a follow-up brief
  (`chart-buy-sell-emphasis-viewer-parity`).
- **Acceptance:** `cargo run --bin viewer -- <report>` renders
  identically to its Phase 4 baseline; no chart with markers
  appears (Q8 = (b) confirmed at operator resolution).

## Verification (V-items)

Tester contract — each maps to one or more R-items above. Failure
routing per the standard analyst→architect→developer→tester loop.

### V1 — Marker visual upgrade in snapshot

`cargo test -p ui --test panel_snapshots chart__btc_with_two_buys_one_sell`
green; updated baseline reflects new `MARKER_SIZE_PX` constant + outline
flag + draw-order line. R1, R2, R6.

### V2 — Marker y snaps to the polyline

`cargo test -p ui chart_marker_y_snaps_to_line` — new unit test in
`crates/ui/src/widgets/chart.rs::tests`. Fixture: fill at exact-midpoint
ts between two bars whose closes differ by `dec!(100)`; asserts rendered
y is the interpolated midpoint ± 0.5px. R3.

### V3 — Tooltip surfaces on hover

`cargo test -p ui --test chart_tooltip_integration` — new integration
test (in `crates/ui/tests/`). Synthetic pointer event at a known
marker's hit-rect → `cockpit.chart_tooltip == Some(view)` with the
expected fields. R4.

### V4 — Click-through opens the journal-transaction modal

`cargo test -p ui --test chart_marker_click_opens_modal` — new
integration test. Synthetic click at a marker's hit-rect →
`Message::TapeRowClicked(tx_id)` → `cockpit.tape_audit_modal ==
Some(PanelState::Ready(view))` with `view.transaction_id` matching the
clicked marker. R4.5.

### V5 — Ghost-signal layer renders behind fills

`cargo test -p ui chart_renders_ghost_and_fill_layers` — new unit test.
Fixture with 2 `SignalView`s and 1 `FillView` at overlapping bars.
Snapshot extension asserts the chart-summary contains
`ghost_count: 2` and `fill_count: 1` AND `draw_order:
gridlines,labels,line,ghosts,fills`. R5.

### V6 — Counter-view tile arithmetic correctness

`cargo test -p ui chart_counter_tile_sums` — unit test. Fixture: 3
buys (total $30,000) + 2 sells (total $20,000); assert tile renders
`Buys in window: +$30,000.00 (3)` / `Sells in window: −$20,000.00 (2)`
/ `Net: +$10,000.00`. R7.1.

### V7 — Per-bar histogram + open-position mirror render

`cargo test -p ui --test panel_snapshots
charts_screen_with_counters_and_chart` — new snapshot. R7.2, R7.3,
R8.

### V8 — Anchor regression 11/11 PASS

`bash scripts/verify_anchors.sh`; output `ANCHORS PASS  (11 / 11)`,
zero diffs vs `spec/anchors.toml`. **Hard gate**. R9.4, R11.4.

### V9 — Existing UI tests stay green

`cargo test -p ui` + `cargo test -p ui --features live` + `cargo
test --workspace` — zero failures. Phase 2's existing
`chart__btc_with_two_buys_one_sell` is **expected to churn** (V1
updates the baseline); all other panel snapshots stay byte-identical.
R10, R11.

### V10 — Determinism: two consecutive runs of V1+V3+V5+V7 byte-identical

Each new snapshot file is run twice; second run is byte-identical to
the first. Catches floating-point non-determinism in y-snap math (R3)
or histogram-bin math (R7.2). Same precedent as Phase 2's
`chart__btc_with_two_buys_one_sell` determinism contract.

### V11 — New audit reader unit-tested (Q1 = (a) path only)

`cargo test -p audit recent_signals` — new test in
`crates/audit/tests/recent_signals.rs`. V11a: known venue+symbol+window
→ correct `Vec<SignalView>` with consistent ordering. V11b: empty
window → `Ok(vec![])`. V11c: `enable_signal_log = false` ledger →
`Ok(vec![])` regardless of window (gate-respecting). R5.3, R5.7.

### V12 — Config gate default-off behaviour

`cargo test -p agent config_signal_log_default_off` — new unit test.
Loads a TOML without `enable_signal_log`, asserts the parsed config
has the field `false`; loads a TOML with `enable_signal_log = true`,
asserts the field is `true`. R5.7.

### V13 — Consistency tests stay green

`cargo test -p ui --test consistency` — green. R10.

Failure routing:

- Static / test failure → `developer`.
- Architect-question regression (e.g. Q1 / Q2 / Q3 / Q6 / Q7
  resolution incompatible with V-items) → `architect`.
- Operator-decide regression (Q4 / Q5 / Q8 reopened) → `analyst`
  (re-scope; not in-place edits).
- Anchor diff → `developer` (per
  [`spec/anchors.toml`](../anchors.toml) gate routing).

## Notes / Open questions

The analyst defers these decisions to the architect (or to the
operator where flagged). The brief is written so each question can
be answered without reshaping the requirements above; R-items that
hinge on Q-resolution explicitly say so.

### Q1 — Signal source plumbing [ARCHITECT-DECIDE] — load-bearing

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q1]

R5 layers strategy signals (intent before risk-clamping) onto the
chart. Strategy signals are **currently transient** — emitted per
bar, consumed by the risk engine, never persisted. Architect picks
the source:

- **Option (a) — new audit log row per signal.** Additive
  `strategy_signals` table OR new `kind` value (`signal_emitted`)
  on existing `strategy_events`. Queryable via a new
  `audit::query::recent_signals(venue, symbol, since, until)`.
  - **Pro:** persistent, replayable, audit-trail-clean. Same query
    pattern as `recent_fills_filtered`.
  - **Con:** writes a row per bar per active strategy. At 4 strategies
    × 60 bars/hr × 24 hr × 30 days ≈ 173k signal rows/month. Audit
    DB size growth ≈ 8 MiB/month (assuming ~50 B/row). Config-gated
    (R5.7) so the cost only lands when an operator opts in.
- **Option (b) — in-memory signal buffer in the cockpit.** Cockpit
  subscribes to a new bus channel emitting `(strategy_id, symbol,
  ts, signal)`. Ring buffer holds the last ~1k for the visible
  window.
  - **Pro:** zero ledger writes; zero audit DB growth.
  - **Con:** **violates the v2 / v1.5 / v1 "no new bus channel" hard
    constraint pattern** (every prior feature has expanded an
    existing bus channel rather than added a new one — precedent:
    operator-success-reports R-items, live-cockpit-unified). Signals
    don't replay across cockpit restarts (operator loses ghost
    history after a kill-switch press).
- **Option (c) — replay-from-backtest only.** Signals only render
  when the operator is viewing a backtest report in the viewer
  binary; cockpit's live chart stays fill-only.
  - **Pro:** simplest. No new audit row, no new bus channel.
  - **Con:** defeats the operator's stated use case
    ("watch the strategy buy/sell at the right *time*" is a
    live-monitoring need, not a backtest-review need).

**[ANALYST-RECOMMENDATION]:** **Option (a)** with the config gate
flipped opt-in. Reasons: (1) preserves the "no new bus channel"
constraint, (2) replayable across cockpit restarts, (3) the
audit-trail discipline matches the
[product.md → Differentiator](../product.md#differentiator) moat
bet — "every order, signal, fill, risk veto, and strategy event is
click-through to its decision trail in the audit ledger" already
applies to fills and risk vetoes; signals belong in the same
ledger by the same principle. Architect confirms or revises;
flipping to (b) is mechanical (R5.4 stays, R5.3 dies, a new
`SignalsMirrored` Message arm replaces the query reader); flipping
to (c) collapses R5 into the viewer-parity follow-up (Q8) and
R-items renumber.

### Q2 — Marker y-snap method [ARCHITECT-DECIDE]

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q2]

R3 says markers anchor to the price line at the marker's
x-coordinate. Two implementation paths:

- **Option (a) — snap to nearest bar's `close`.** Cheap; misaligns
  slightly when fill `venue_ts` falls between two bars (typical
  case for ticks arriving mid-bar — most paper-engine fills land
  between bar boundaries).
- **Option (b) — linear interpolation between bracketing bars'
  closes.** More accurate visual line-crossing; ~4 extra `f32` ops
  per marker, trivially in budget.

**[ANALYST-RECOMMENDATION]:** **Option (b)** for visual smoothness.
Architect picks; (a) is fine if architect wants the simpler code
path and accepts the ≤1-pixel y-misalignment for sub-bar-cadence
fills.

### Q3 — Tooltip implementation in iced canvas [ARCHITECT-DECIDE]

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q3]

iced canvas has no native hover. Three paths:

- **Option (a) — iced's `tooltip` widget on a transparent overlay
  grid.** Each marker gets a hit-rectangle widget on top of the
  canvas; hover surfaces an iced-native `tooltip`. Native look, but
  rectangle-grid placement might drift if markers move
  (relayout-per-bar overhead). Documentation: see
  [iced 0.14 widget docs](https://docs.rs/iced/0.14/iced/widget/fn.tooltip.html).
- **Option (b) — custom canvas pointer-tracking + custom-drawn
  tooltip overlay.** Pointer events on the canvas via
  `canvas::Event::Mouse` (currently the chart's
  `canvas::Program::State = ()` — would become a `ChartState`
  struct holding `hovered_marker_idx: Option<usize>`); render the
  tooltip on the same canvas as a final pass. Full control over
  positioning + look; more code; full-on hover state lives entirely
  in iced widget state.
- **Option (c) — click-to-open modal (sibling of
  tape-row-audit-modal).** No hover; click marker → modal opens.
  Clearer interaction; more clicks per inspection; partial overlap
  with R4.5 (which already wires click → modal).

**[ANALYST-RECOMMENDATION]:** **Option (b)** for hover fidelity —
matches the operator's "tooltip or something" mental model better
than click-modal. **(a) is the fallback** if (b) blows the
implementation budget at developer time (iced custom pointer
tracking on a canvas is non-trivial; architect should sanity-check
budget). Note that R4.5 keeps the click-to-open-modal path
regardless of Q3 choice, so (c)-only would NOT be a regression on
the operator's "I want to inspect a fill" use case — but it's a
strictly worse experience than hover.

### Q4 — Tooltip content fields [OPERATOR-DECIDE]

[RESOLVED 2026-05-10 — operator confirmed analyst strawman via orchestrator chat AND accepted the sub-question recommendation: **drop the truncated transaction ID from the tooltip**. The full UUID is one click away in the journal-transaction modal (R4.5), so duplicating a truncated form in the tooltip wastes vertical space. Final tooltip fields: Side (Buy/Sell badge), Price (USDT, 4 decimals), Quantity (base asset, 4 decimals), Notional (price × qty, USDT, 2 decimals), Timestamp (RFC3339 UTC), Strategy ID (if surfaceable). Six fields total. Architect's R4.4 positioning math + R4.7 strings constants enumerate exactly these six.]


R4.2 strawman: **Side** (Buy/Sell badge), **Price** (USDT, 4
decimals), **Quantity** (base asset, 4 decimals), **Notional**
(price × qty, USDT, 2 decimals), **Timestamp** (RFC3339 UTC),
**Transaction ID** (truncated, full UUID via R4.5 modal), **Strategy
ID** (if surfaceable).

Operator picks: confirm the strawman, trim a field, add a field, or
re-order. Directly affects the tooltip's width + vertical space (R4.4
positioning math) and the new `ui::strings` constants enumerated in
R4.7.

**Open sub-question for operator:** is the **truncated transaction
ID** useful in the tooltip if R4.5 already opens the full modal on
click, OR should it be dropped to save vertical space?

### Q5 — Layout for the three counter views [OPERATOR-DECIDE]

[RESOLVED 2026-05-10 — operator picked **Layout (β)** via orchestrator chat (analyst's recommendation). Final shape: chart keeps full width; cumulative-window-volume tile + open-position mirror sit **above** the chart in a status strip; per-bar volume histogram sits **below** the chart at a fixed ~80px height. Eye-line flows tile → chart → histogram. R8.1's layout reshape pins this; architect's M4 task enumeration locks exact pixel heights / spacing per Lumen tokens.]


Five plausible layouts for the cumulative tile (R7.1) + per-bar
histogram (R7.2) + open-position mirror (R7.3):

- **Layout (α) — all three stacked vertically right of the chart.**
  Chart loses ~30% width.
- **Layout (β) — tile + position above the chart in a status strip;
  histogram below the chart at fixed ~80px height.** Chart keeps
  full width.
- **Layout (γ) — tile + position toggleable in a collapsible right
  panel; histogram below the chart.** Operator can hide.
- **Layout (δ) — mode toggle "Quiet view" (tile only, hide histogram
  + position) vs "Detailed view" (all three).** Two presets.
- **Layout (ε) — tile inline in the chart's existing chip row;
  histogram below; position omitted (already on Home).** Most
  minimal.

**[ANALYST-RECOMMENDATION]:** **Layout (β).** Keeps the chart
prominent at full width, surfaces the most operator-useful tile +
open-position info above where the eye lands first, dedicates a fixed
bottom strip to the histogram (which doubles as a visual time axis
for the chart above it). Operator picks.

### Q6 — Marker contrast / outline / drop shadow [ARCHITECT-DECIDE]

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q6]

R1 says bigger; the architect picks the exact visual treatment per
the Lumen design system in [`spec/design/`](../design/). Analyst
strawman:

- **Fill-marker layer:** 13-px filled triangle with a 1-px
  `BORDER_STRONG` outline and a 1.5-px subtle drop shadow (Lumen
  "whisper shadow" — alpha ≈ 0.15, offset `(0, 1.5)`).
- **Ghost-signal layer:** 8-px filled triangle at 60% opacity, no
  outline, no shadow.

Architect confirms exact pixel values, alpha values, and shadow
offsets, citing the Lumen design tokens. May also pick a different
triangle aspect ratio (current `draw_triangle` makes equilateral
triangles; an isoceles taller-than-wide variant reads more "arrow-y"
while still honoring R6 "Keep triangle").

### Q7 — Per-bar histogram widget shape [ARCHITECT-DECIDE]

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q7]

R7.2 needs a per-bar two-colour bar widget. Two paths:

- **Option (a) — reuse `widgets::sparkline`.** Sparkline is single-line
  60-cell encoding (currently 60 ▁▂▃▄▅▆▇█ Unicode blocks); a
  histogram needs paired green/red bars per bar. Reusing the
  sparkline widget means extending it with a "paired" mode, which
  ripples into the equity-curve consumers in
  [`crates/ui/src/widgets/equity_curve.rs`](../../crates/ui/src/widgets/equity_curve.rs).
- **Option (b) — new `widgets::volume_histogram`.** Sibling widget;
  no ripple. Reuses the `canvas_chart` core (gridlines, inner_rect)
  per Phase 2's
  [`crates/ui/src/widgets/canvas_chart.rs`](../../crates/ui/src/widgets/canvas_chart.rs)
  precedent.

**[ANALYST-RECOMMENDATION]:** **Option (b)** — sparkline and
volume-histogram have different data shapes (single-series scalar vs
paired buy/sell sums per bin); merging them under one widget would
gain code reuse but cost type clarity. New widget per the principles
"three-uses rule" only when a third volume-histogram consumer appears.

### Q8 — Backtest-viewer parity [OPERATOR-DECIDE]

[RESOLVED 2026-05-10 — operator picked **Option (b) defer** via orchestrator chat (analyst's recommendation). Cockpit-only this round; viewer parity is a follow-up brief named e.g. `viewer-charts-parity` queued post-ship. Reasons: cockpit serves the live-monitoring use case the operator described ("did the strategy buy at the right time"); viewer's purpose is post-hoc backtest review where the existing KPI strip + equity curve + drawdown band already cover the same operator question through different lenses. R12 in this brief explicitly excludes viewer changes.]


Should the `viewer` binary
([`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs))
— the backtest report viewer — inherit the new chart shape?

- **Option (a) — yes, extend viewer with the same Charts-screen
  composition.** Viewer parity with cockpit. Significant scope: the
  viewer currently renders KPI strip + equity curve + drawdown band
  (no price chart with markers).
- **Option (b) — no, cockpit-only in v1.9.** Viewer parity becomes a
  follow-up brief.

**[ANALYST-RECOMMENDATION]:** **Option (b)** — the viewer's purpose
is backtest review; the operator's stated use case ("did the
strategy buy at the right time") is live-monitoring, which lives in
cockpit. Viewer parity is a follow-up brief
(`chart-buy-sell-emphasis-viewer-parity`) if/when the operator wants
the same view against historical backtest fills. Operator confirms.

### Q9 — `SignalView` type home + exact shape [ARCHITECT-DECIDE]

[RESOLVED 2026-05-10 — see ## Design § chart-buy-sell-emphasis Q9]

R5.3 introduces a new `SignalView` type. Architect picks:

- **Home:** `crates/core/src/views.rs` (sibling of `FillView`) is the
  natural fit per the precedent set by tape-row-audit-modal's
  Q2-resolved `JournalEntry` placement.
- **Shape:** analyst strawman `{ symbol: Symbol, side: Side,
  intended_qty: Quantity, signal_ts: Timestamp, strategy_id:
  StrategyId, was_clamped: bool, clamp_reason: Option<SmolStr> }`.
  Architect may add `intended_price: Option<Price>` if the strategy
  emits a price (limit-order shapes); v1 strategies (sma_crossover,
  v0.5 composed, v1 cross-sectional, v1.5a pairs MR) all emit
  market-priced signals so `intended_price` would be `None`
  pervasively.

**[ANALYST-RECOMMENDATION]:** strawman as-is, `intended_price`
omitted in v1; add at Design time only if v2 LLM or v2.5 Kronos
strategies need it.

### Q10 — Anything else?

- **Marker keyboard navigation?** Should arrow keys cycle through
  visible markers (Tab → next marker → focus ring + tooltip)?
  Principles "Accessibility minimums" hints at this; analyst's prior
  is **defer to follow-up** (the chart canvas has no existing
  keyboard interaction in Phase 2; introducing one is a Phase 5–style
  focus-ring extension, separate scope).
- **Symbol-switch animation?** When the operator clicks a chip and
  the active symbol changes, the marker layer re-fetches and
  re-renders abruptly. Worth a fade-transition? Analyst's prior:
  **defer**; abrupt switch matches Phase 2's existing UX.
- **Multi-strategy colour-coding?** With the ghost layer (R5), if
  three different strategies emit signals at the same bar, the
  ghost markers overlap. Should ghosts be coloured per-strategy
  rather than per-side? Analyst's prior: **defer** to follow-up if
  operator-multi-strategy attribution is operator-confirmed needed;
  per-side colouring matches the fill layer and reads consistently.

## Backtest scenarios

_n/a — UI feature, no new backtest scenarios. Existing 9 backtest
anchors guard rendering / strategy / audit-write-path drift; this
feature touches none of those code paths (R9.4)._

## Design

Resolves the six `[ARCHITECT-DECIDE]` questions and pins the
implementable shape for M1–M5. Each sub-section uses the standard
template (Decision / Rationale / How it shows up in code). The
closing `### Crate / module surface` enumerates every new file and
every existing file the developer will touch.

The six resolutions together are framed by **four invariants that do
not move under this brief**:

1. **`Strategy` trait shape is fixed** — `fn on_bar(&mut self,
   &Bar) -> Vec<Signal>` continues to return signals; strategies
   never call into the audit ledger. The signal-emit tap point
   lives **outside** the trait, inside the agent loop, after the
   strategy returns and before the risk engine consumes (see Q1
   below). The trait's six callers (`backtest`, `agent`, plus the
   four strategy crates) compile unchanged.
2. **No new bus channel** — the new persistence row writes
   straight to the audit ledger via the existing
   `journal::post_strategy_signal` writer pattern (sibling of
   `journal::post_fill`, atomic SQL transaction). Cockpit reads
   via `audit::query::recent_signals` (sibling of
   `recent_fills_filtered`), polled the same way `chart_markers`
   is — on `SelectSymbol` and `BarClose`. Same subscription shape
   as Phase 2.
3. **Atomic write contract preserved** — `journal::post_strategy_signal`
   uses the established `ledger.pool.begin() / commit()` pattern
   ([`crates/audit/src/journal.rs:72-93`](../../crates/audit/src/journal.rs))
   already used by `post_fill`, `kill_switch_tripped`,
   `strategy_paused`. No new on-disk write path; no contact with
   `reports/src/atomic_write.rs`.
4. **Anchor risk: zero** — no `strategy/*`, `risk/*`, `backtest/*`,
   or report-rendering code is touched. The 11 anchored reports
   stay byte-identical. V8 is the hard gate at T_FINAL.

#### chart-buy-sell-emphasis Q1 — Signal source plumbing: **Option (a) — additive `strategy_signals` table, config-gated, polled by cockpit via a new `audit::query::recent_signals` reader. Default `enable_signal_log = false`.**

**Decision:** A new additive SQLite migration `009_strategy_signals.sql`
creates a dedicated `strategy_signals` table (not a new `kind` value
on `strategy_events`). A new agent-side writer
`journal::post_strategy_signal(ledger, signal, venue, was_clamped,
clamp_reason) -> Result<(), LedgerError>` inserts one row per
emitted `Signal` from a new tap point in the agent's main loop,
**between** the strategy registry's `on_bar` return and the risk
engine's consume call. A new reader `audit::query::recent_signals(
ledger, venue, &Symbol, since, until) -> Result<Vec<SignalView>,
LedgerError>` mirrors the `recent_fills_filtered` shape.

The writer is **config-gated**: `agent.toml [signal_log]
enabled = false` is the default; flipping to `true` activates the
tap point. With the gate off, **zero** new rows are written and the
ghost layer renders empty (R5.7). The gate is read once at agent
boot from `crates/agent/src/config.rs::SignalLogConfig` (new section,
sibling of `[reflection]`).

**Rationale:**

- **Option (a) over (b):** Option (b) requires a new bus channel,
  which violates the established "no new bus channel" hard
  constraint (operator-success-reports R6.2, live-cockpit-unified
  R9.1, reflection-memory R7.3 — three precedents, three rejections).
  Picking (b) here would re-litigate a settled architecture decision
  to save a per-month 8 MiB ledger budget that the operator already
  accepts (R5.7 explicitly enumerated the cost). The bus-channel
  prohibition exists because every prior bus channel addition has
  cost ≥1 day of debugging cross-thread mailbox sequencing; the
  audit-ledger-as-source-of-truth pattern is the project's
  load-bearing replayability contract
  ([product.md → Differentiator](../product.md#differentiator)).
- **Option (a) over (c):** Option (c) (replay-from-backtest only)
  defeats the operator's live-monitoring use case verbatim
  ("watch the strategy buy/sell at the right *time*"). Backtest
  replay is post-hoc; the operator's question is now.
- **Dedicated table over a new `kind` value on `strategy_events`:**
  `strategy_events` is structured for **lifecycle events**
  (Load / Swap / Unload / Reject / KillSwitchTripped / FeedReconnect
  / etc — see
  [`crates/core/src/strategy_events.rs:111-134`](../../crates/core/src/strategy_events.rs)).
  A signal is a **per-bar emission**, not a lifecycle event;
  shoehorning ~173k signal rows/month into `strategy_events`
  pollutes the lifecycle-events reader (which streams every row
  for the Strategies-detail screen). A sibling table keeps the
  two access patterns separate and lets the signal-log table be
  trivially dropped or truncated without touching lifecycle history.
- **Config gate default-off:** Audit DB growth ≈ 8 MiB/month at
  the analyst-flagged 4-strategy × 60-bar × 24-hour × 30-day
  volume. Defaulting on would silently grow the ledger for every
  operator regardless of whether they care about the ghost layer.
  Pre-flagging opt-in matches the established reflection-memory v1.8
  pattern (`enable_writer` shipped default-false, flipped to true
  on operator approval 2026-05-10 — same shape applied here).
- **Schema-additive zero downtime:** Migration is `CREATE TABLE IF
  NOT EXISTS` only — no `ALTER`, no data backfill. Re-running the
  sqlx migrator is a no-op. The reader returns `Ok(vec![])` if the
  table is empty (gate-off path), so V11c is naturally satisfied
  without a special-case branch.

**Trait shape stays fixed.** Strategies continue to emit `Vec<Signal>`
from `on_bar`. The agent main loop is the new caller of
`post_strategy_signal` — strategies never touch the ledger. The
risk engine continues to consume the signals as before; the new
tap point sits **before** risk consumes, so `was_clamped` is
captured by a **second** call after the risk engine returns its
decision (the tap pair: emit-row at signal-eval-time, update-row
at risk-decide-time). Implementation captures the `(was_clamped,
clamp_reason)` fields by writing them on the same row at the
risk-decide point — single `UPDATE` after the `INSERT`. Atomic
within the agent's per-bar transaction boundary (existing
`tokio::sync::Mutex` around the per-bar critical section in the
agent runtime; no new lock).

**How it shows up in code:**

- New migration: `crates/audit/migrations/009_strategy_signals.sql`.
- New writer module: `crates/audit/src/journal.rs` gains
  `pub async fn post_strategy_signal(ledger: &Ledger, signal:
  &Signal, venue: Venue, was_clamped: bool, clamp_reason:
  Option<&str>) -> Result<(), LedgerError>` and a sibling
  `pub async fn update_signal_clamp_status(ledger: &Ledger,
  signal_id: &str, was_clamped: bool, clamp_reason: Option<&str>)
  -> Result<(), LedgerError>`.
- New reader: `crates/audit/src/query.rs` gains
  `pub async fn recent_signals(ledger: &Ledger, venue: Venue,
  symbol: &Symbol, since: Timestamp, until: Timestamp) ->
  Result<Vec<SignalView>, LedgerError>` (sibling of
  `recent_fills_filtered` at lines 223–269; same RFC3339 binding +
  `venue.to_string()` + `Symbol`-equality filter shape).
- New config section: `crates/agent/src/config.rs` gains
  `pub struct SignalLogConfig { pub enabled: bool }` with
  `#[serde(default)] enabled: false` and a `[signal_log]` TOML
  section (next to `[reflection]`).
- Agent main-loop tap point: `crates/agent/src/runtime.rs` —
  the future signal-emit dispatch site (today the live runtime
  doesn't yet invoke `registry.on_bar`; the backtest binary does at
  `crates/backtest/src/main.rs:587, 986, 1879`). Per the brief's
  scope this feature **does not** wire the live signal-emit loop
  itself — that's a parallel-track agent runtime change. **What
  this feature ships is the writer + reader + config gate + cockpit
  read path.** The integration test ledger writes signals via
  fixtures (sub-tests in `crates/audit/tests/recent_signals.rs`).
  When the live signal-emit loop lands (next brief in the agent-
  runtime track), it imports `journal::post_strategy_signal` and
  the ghost layer comes alive without any further cockpit-side
  change. Documented as a forward-compatibility note in the M3
  task block (T2017–T2020) and called out in `## Implementation`
  by the developer at landing time.

#### chart-buy-sell-emphasis Q2 — Marker y-snap method: **Option (b) — linear interpolation between bracketing bars' closes.**

**Decision:** Compute marker `y` by linear interpolation between
the two bars whose `close_ts` bracket the marker's `venue_ts`.
For a marker at `t` between bar `i` (close at `t_i`, close-price
`p_i`) and bar `i+1` (close at `t_{i+1}`, close-price `p_{i+1}`),
the snapped price is

```
frac = (t - t_i) / (t_{i+1} - t_i)
p_snapped = p_i + frac * (p_{i+1} - p_i)
y = y_for_price(p_snapped, range, inner)
```

then pass `p_snapped` (not the original `fill.price.get()`) through
the existing `y_for_price` helper at
[`chart.rs:243-249`](../../crates/ui/src/widgets/chart.rs).

**Rationale:**

- **Option (b) over (a):** Option (a) (snap-to-nearest-bar) is
  ≤1-pixel cheaper and ≤4-pixel less accurate in y-misalignment
  for sub-bar-cadence fills (which is the dominant case for paper-
  engine fills landing mid-bar). The operator's stated mental model
  is "did the strategy buy at the right *time*", and the visual
  payoff for that question is the marker visibly riding the line
  *across* a slope, not jumping in step-function quanta to the
  nearest bar-close. The extra `4` `f32` ops per marker
  (subtraction, division, multiplication, addition) at ≤10 markers
  per render is trivially in budget — the bottleneck on the iced
  canvas repaint is the GPU upload, not the marker math.
- **Determinism preserved:** `f32` linear interpolation with two
  decimal-derived inputs is bitwise-deterministic on the same
  hardware and the same Rust toolchain (no transcendental functions,
  no SIMD reduction, no FMA optimization that could vary between
  build profiles). V10 (two consecutive runs byte-identical)
  protects this.
- **Edge cases match R3.3 verbatim:** First-bar marker → `p_snapped
  = bars[0].close`; last-bar marker → `p_snapped = bars[N-1].close`;
  out-of-window marker → still filtered by the existing
  `min_ts <= fill_ts <= max_ts` clip at line 149-153.

**How it shows up in code:**

- New helper in `crates/ui/src/widgets/chart.rs`:
  ```rust
  fn snap_price_to_line(fill_ts: i64, bars: &[Bar]) -> Option<f32>
  ```
  Binary-search for the bracketing bar pair (`bars` are
  monotonically-ordered by `close_ts`; the existing
  `ChartBuffer::push_bar` invariant guarantees this), interpolate.
  Tested by the V2 unit test (`chart_marker_y_snaps_to_line`):
  fixture with a fill at midpoint between two bars whose closes
  differ by `dec!(100)` asserts `y` equals the interpolated
  midpoint within 0.5 px tolerance.
- The fill marker's existing `y_for_price(decimal_to_f32(
  &fill.price.get()), range, inner)` call at
  [`chart.rs:156-157`](../../crates/ui/src/widgets/chart.rs)
  changes to `y_for_price(snap_price_to_line(fill_ts, &self.bars)
  .unwrap_or_else(|| decimal_to_f32(&fill.price.get())), range,
  inner)` (defence-in-depth fallback to the original fill price
  if the helper returns `None`, which can only happen on an empty
  bar window that the existing line 149-153 clip already rules
  out).
- The fill's real execution price (still distinct from the snapped
  line price) is preserved in the tooltip's **Price** field per
  R4.2 — the snap is purely visual.

#### chart-buy-sell-emphasis Q3 — Tooltip implementation in iced canvas: **Option (b) — custom canvas pointer-tracking + custom-drawn tooltip overlay, with (a) `iced::widget::tooltip` overlay-grid as the documented fallback if hit-rect math at developer time blows the implementation budget.**

**Decision:** Promote `ChartProgram::State` from `()` to
`#[derive(Default)] struct ChartState { hovered_marker_idx:
Option<MarkerIndex> }` where `MarkerIndex` is a tagged enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerIndex {
    Fill(usize),   // index into ChartProgram.markers
    Signal(usize), // index into ChartProgram.signals (R5 ghost layer)
}
```

Implement `canvas::Program::update` for `ChartProgram`,
consuming `iced::widget::canvas::Event::Mouse(mouse::Event::
CursorMoved { position })` and reading `bounds` from the
`canvas::Program::update` signature. Compute the `inner` rect and
test each marker's 28-px hit-rect (R4.3) against the cursor; the
**first** marker whose hit-rect contains the cursor wins
(deterministic z-order — fills above ghosts per R2.1, so a fill
under the cursor beats a co-located ghost). Emit
`Message::ChartMarkerHovered(MarkerIndex)` when the hovered marker
**changes** (entry into a hit-rect from outside, or hit-rect-to-
hit-rect transit), `Message::ChartMarkerHoverEnded` when the
cursor exits all hit-rects. The same `update` handler dispatches
`Message::TapeRowClicked(transaction_id)` for **fill** markers on
`mouse::Event::ButtonPressed(Left)` (R4.5 click-through; ghosts
have no transaction so click does nothing).

The tooltip itself is drawn as a final canvas pass (a fifth
`frame.fill_text` block after the marker-fill pass) reading
`Cockpit.chart_tooltip: Option<ChartTooltipView>` via a small
view-data conduit (the rendered `ChartProgram` carries the tooltip
view by value, populated at compose time in `screens::charts::view`
from `model.chart_tooltip`). Positioning per R4.4: prefer
above-and-right, flip to below-and-left if the marker is in the
upper-right quadrant of the inner rect.

**Rationale:**

- **Option (b) over (a):** Option (a) (iced `tooltip` widget on a
  transparent overlay grid) requires per-bar relayout of N
  overlay rectangles every time `chart_markers` changes shape — the
  bar window is rolling and markers re-place on every `BarClose`.
  The relayout overhead is small but the **placement math is
  duplicated** between the canvas (drawing the marker) and the
  overlay grid (placing the hit-rect on top). Two sources of truth
  for the marker centroid is precisely the bug shape the precursor
  commit `a8e7110` fixed for fixture-side ts spacing — and it would
  re-emerge here under (a). Option (b) keeps the centroid
  computation in **one place** (inside `ChartProgram::draw` /
  `ChartProgram::update`, sharing the same `x_for_index` /
  `snap_price_to_line` math).
- **Option (c) (click-only modal) ruled out by operator UX:**
  R4.5 (click → modal) already exists in this brief regardless of
  Q3; the operator's hover ask is layered **on top of**, not
  instead of, the click path. (c)-only would be a strictly worse
  UX than the current ship-state of inspecting fills via the
  Audit screen.
- **Option (a) preserved as documented fallback:** Custom
  pointer-tracking on iced 0.14's canvas works (the
  `canvas::Program::update` API is stable and the
  `mouse::Event::CursorMoved` event is fully wired), but if the
  developer hits an unforeseen 0.14 gotcha during M2, the
  contingency is to fall back to (a) at the cost of duplicated
  centroid math — documented in the developer's M2 task block
  (T2010) as a back-channel for the tester at architect-question-
  regression routing (V3 / V4 failure → architect).
- **`canvas::Program::State` promotion is non-breaking:** Existing
  Phase 2 tests render snapshots through `chart_summary` which
  bypasses the `Program` impl entirely; the snapshot helper reads
  the rendered `ChartProgram` directly, not the `State`. No churn
  on the existing `chart__empty_state_no_data` baseline.

**How it shows up in code:**

- `crates/ui/src/widgets/chart.rs`: `ChartProgram::State` becomes
  `ChartState`; `ChartProgram::update` is implemented; the
  marker-hit-rect helper `marker_hit_rect(anchor: Point) ->
  Rectangle` is a new private fn returning a 28-px square
  centered on the marker centroid.
- New file: `crates/ui/src/widgets/chart_tooltip.rs` — the
  tooltip canvas-pass renderer. Surface:
  ```rust
  pub(crate) fn draw_tooltip(
      frame: &mut iced::widget::canvas::Frame,
      bounds: iced::Rectangle,
      anchor: iced::Point,
      view: &crate::state::ChartTooltipView,
      mode: crate::theme::ThemeMode,
  );
  ```
  Reads field strings from `crate::strings::CHART_TOOLTIP_*` only;
  colours from `theme::color::*` only.
- `crates/ui/src/state.rs`: new fields on `Cockpit`:
  `pub chart_tooltip: Option<ChartTooltipView>,` (defaults
  `None`), plus the new Message arms below.
- `Message::ChartMarkerHovered(ChartMarkerIndex)`,
  `Message::ChartMarkerHoverEnded` — pure-function updates
  (set / clear `chart_tooltip`). `Message::TapeRowClicked` is
  **reused unchanged** for R4.5 click-through (the second
  consumer of the existing message arm; R11.3 invariant
  preserved).
- New view-type: `crate::state::ChartTooltipView` carrying the
  six R4 / R5.6 fields plus a `kind: Fill | Signal` discriminant
  to drive the ghost-vs-fill tooltip variant.

#### chart-buy-sell-emphasis Q6 — Marker visual treatment: **13-px filled triangle, 1-px `BORDER_STRONG` outline, `shadow_1`-derived drop shadow (offset `(0, 1.5)`, alpha `0.30` dark / `0.04` light, blur `2.0`) for the fill layer; 8-px filled triangle at 60% opacity, no outline, no shadow, semantic tier-400 ramp for the ghost layer.**

**Decision:**

**Fill layer (R1 + R6):**
- Size: `MARKER_SIZE_PX = 13.0` (was `6.0`; analyst strawman
  confirmed).
- Outline: 1-px stroke in `theme::color::BORDER_STRONG` — already
  shipped, no new token.
- Drop shadow: derived from `theme::shadow::shadow_1(mode)` —
  no new shadow token. Render as a second pre-pass triangle in
  pure black (dark mode) / warm-900 (light mode) at the same
  size, offset `(0.0, 1.5)`, then alpha-blended at `0.30`
  (dark) / `0.04` (light) — exact alphas inherited verbatim from
  `shadow_1`. Drawn **before** the outline + fill so the shadow
  sits behind. This re-uses the existing `theme::shadow::shadow_1`
  for parameters; no new token needed.
- Buy fill: `color::UP_500` (sage). Sell fill: `color::DOWN_500`
  (clay). Unchanged.
- Outline + shadow are added to the existing `draw_triangle`
  helper at
  [`chart.rs:197-213`](../../crates/ui/src/widgets/chart.rs) via
  new optional parameters; the helper's signature gains
  `outline: Option<Color>` and `shadow: Option<(Vector, Color)>`
  — both `None`-able so the ghost layer can pass `None` for both.

**Ghost layer (R5):**
- Size: `GHOST_MARKER_SIZE_PX = 8.0` (≈ 60% of fill size).
- Opacity: 60% — applied as `with_alpha(color, 0.6)` using the
  existing helper at
  [`canvas_chart.rs:59`](../../crates/ui/src/widgets/canvas_chart.rs).
- Buy signal: `color::UP_400` (lighter sage). Sell signal:
  `color::DOWN_400` (lighter clay). **Uses the existing `_400` tier
  tokens** (already shipped at
  [`theme.rs:234, 252`](../../crates/ui/src/theme.rs)); no
  new ghost-layer token needed. The `_400 → _500` brightness
  contrast carries the ghost-vs-fill semantic visually.
- No outline, no shadow per R6.3.

**Rationale:**

- **13-px size:** The brief's analyst strawman; ≈ 2.2× area vs
  the shipped 6-px makes the marker visible against the 1-px
  `ACCENT` line on a 60-bar window at typical cockpit density
  (operator-confirmed pain point at the 2026-05-10 review).
  Doesn't overflow the 28-px hit-rect (R4.3) — leaves a 7.5-px
  margin on each side, comfortably within Fitts's-law
  forgiveness for ~13-px targets.
- **`BORDER_STRONG` outline:** Already shipped (tape-row-audit-modal
  R6); no new token. Visibly distinct from `BORDER_1` per
  Phase 1's
  [`theme.rs::tests::border_strong_is_visibly_distinct_from_border_1`](../../crates/ui/src/theme.rs)
  pin test — load-bearing for the line-vs-marker separation.
- **`shadow_1`-derived shadow over a new "whisper-marker-shadow"
  token:** `shadow_1` is already calibrated to "barely-there
  panel chrome" with the exact alpha-per-mode discipline Lumen
  prescribes; reusing it for the marker shadow avoids token drift
  ("just one exception" is what kills design systems). The
  developer renders the shadow as a manual pre-pass on the canvas
  (iced's `canvas::Frame` doesn't expose a `Shadow` for filled
  paths — only `container::Style` carries `iced::Shadow`); the
  shadow values (`offset_y=1.5, alpha=0.30 dark / 0.04 light,
  blur ignored on canvas`) come from `shadow_1`. No new token added
  to `theme.rs`.
- **Ghost layer uses existing `_400` tier:** `UP_400 / DOWN_400`
  ship today, used by the operator-success-reports memory-highlight
  body. Reusing them keeps the chart and the report consistent;
  introducing `UP_300 / DOWN_300` would expand the semantic ramp
  beyond what Phase 1 sanctioned without operator approval. The
  60% alpha on top of the `_400` tier produces a visibly fainter
  ghost than the `_500` fill — the perceptual delta is the cue.

**How it shows up in code:**

- `crates/ui/src/widgets/chart.rs`: bump `MARKER_SIZE_PX = 13.0`
  (already a constant — single-line change). Add
  `GHOST_MARKER_SIZE_PX = 8.0` constant. Extend `draw_triangle`
  signature with `outline: Option<Color>` and
  `shadow: Option<(iced::Vector, iced::Color)>` parameters.
  Add a new private helper `whisper_shadow(mode: ThemeMode) ->
  (iced::Vector, iced::Color)` returning `(Vector::new(0.0,
  1.5), shadow_color)` where `shadow_color` is
  `theme::shadow::shadow_1(mode).color` (existing public API
  already returns an `iced::Shadow` with the right alpha-per-
  mode).
- **No changes to `crates/ui/src/theme.rs`.** No new tokens. All
  values flow from existing `UP_400 / UP_500 / DOWN_400 /
  DOWN_500 / BORDER_STRONG / shadow_1`.

#### chart-buy-sell-emphasis Q7 — Per-bar histogram widget shape: **Option (b) — new sibling widget `crates/ui/src/widgets/volume_histogram.rs` reusing the `canvas_chart` core (`inner_rect`, `with_alpha`, gridline math).**

**Decision:** Create a new widget `widgets::volume_histogram` —
sibling of `widgets::chart`, not an extension of
`widgets::sparkline`. Signature:

```rust
pub fn view<'a>(
    bins: Vec<VolumeBin>,
    mode: ThemeMode,
) -> crate::Element<'a>;
```

where `VolumeBin` is a new local type:

```rust
pub struct VolumeBin {
    pub buys_usdt: rust_decimal::Decimal,
    pub sells_usdt: rust_decimal::Decimal,
}
```

(local to the widget — derived state, not a new core type — built
at compose time in `screens::charts::view` from `model.chart_markers`
+ `model.chart_buffer.bars(venue, &symbol)`).

The widget owns its own `canvas::Program` impl rendering N paired
two-color bars (green up, red down) inside a canvas-typed inner
rect. Fixed height 80 px per R7.2 (set at the Container level by
`screens::charts::view`, not by the widget itself — same density
pattern as the chart's `Length::Fill` semantics). Bars stack
buy-on-top, sell-on-bottom; both rooted at the y-axis baseline
that splits the inner rect 50/50 (signed two-color stacked).

**Rationale:**

- **Option (b) over (a):** Sparkline is single-line 60-cell
  Unicode-block encoding (currently feeds equity-curve consumers
  per the analyst's brief). Extending sparkline with a "paired"
  mode ripples into equity-curve and the strategies-detail screen
  — two production-render paths re-validated for a histogram-
  shaped use case they don't share. The data-shape mismatch is
  load-bearing: sparkline takes `&[Decimal]` (single series),
  histogram takes `&[(Decimal, Decimal)]` (paired buy/sell sums).
  Merging would require sparkline to grow a tagged union which
  is the wrong direction for type clarity (and would force the
  equity-curve consumer to pattern-match `single` on every render).
- **Three-uses rule honoured:** Per `spec/ui-design-principles.md`,
  abstract on the third consumer, not the first. This is the
  histogram's first consumer; a follow-up widget needs would
  trigger a refactor. Sparkline-equity-curve-strategies is its
  own three-uses cluster.
- **Reuses `canvas_chart` core:** `inner_rect`, `with_alpha`,
  and `GRIDLINE_COUNT` are already `pub(crate)` at
  [`canvas_chart.rs:18-71`](../../crates/ui/src/widgets/canvas_chart.rs).
  The new widget imports them like `widgets::chart` does — no
  new shared crate-private surface.

**How it shows up in code:**

- New file: `crates/ui/src/widgets/volume_histogram.rs` (~150
  LOC, sibling shape of `chart.rs`).
- `crates/ui/src/widgets/mod.rs`: `pub mod volume_histogram;`
  declaration added.
- `crates/ui/src/screens/lab.rs`: consume the widget below the
  chart via `.push(volume_histogram::view(bins, mode))` inside a
  `Container::new(...).height(Length::Fixed(80.0))`.
- `crates/ui/src/widgets/snapshots/`: new
  `volume_histogram__btc_three_buys_two_sells.snap` insta snapshot.

#### chart-buy-sell-emphasis Q9 — `SignalView` type home + exact shape: **Home `crates/core/src/views.rs` as a sibling of `FillView`. Shape: analyst strawman as-is, `intended_price` omitted, plus a `signal_id: SmolStr` field for the future second-row-update tap (Q1).**

**Decision:** Add to `crates/core/src/views.rs`:

```rust
/// Read-side representation of a strategy signal — pre-risk-clamp
/// intent emitted by a `Strategy::on_bar` arm, returned by
/// `audit::query::recent_signals`.
///
/// `was_clamped` is populated by a second writer call at the
/// risk-decide tap point (Q1); for in-flight signals not yet
/// risk-clamped, the reader returns `was_clamped = false` and
/// `clamp_reason = None`. For the in-flight case the column
/// stays `NULL` in the DB and the reader maps `NULL` → `false`
/// — the operator's ghost-layer experience prefers under-
/// reporting clamps to over-reporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalView {
    /// `strategy_signals.id` UUID string — for the click-through
    /// to the future signal-detail modal (deferred to a follow-up
    /// brief; carried today for forward-compat).
    pub signal_id: SmolStr,
    pub symbol: Symbol,
    pub side: Side,
    /// Intended quantity before risk-clamping. The risk engine's
    /// post-clamp quantity is captured on the fill row, not here.
    pub intended_qty: Quantity,
    pub signal_ts: Timestamp,
    pub strategy_id: StrategyId,
    /// `true` if the risk engine modified the signal before it
    /// reached the exec engine. Captured by the second-row-update
    /// writer call; defaults to `false` for in-flight signals.
    pub was_clamped: bool,
    /// Free-form short reason from the risk engine
    /// (e.g. `"per_symbol_cap"`). `None` when `was_clamped`
    /// is `false`.
    pub clamp_reason: Option<SmolStr>,
}
```

**Rationale:**

- **Home in `core::views`:** Precedent set by `FillView`,
  `JournalEntryView`, `JournalRow`, `PositionView`. Read-side DTOs
  live in `core::views`; pure data, no back-edge from `core` to
  `audit`. Architecture.md explicitly states "read-side view types
  used by `audit::query` and the UI" for this module (file
  doc-comment).
- **`intended_price` omitted:** All v0 / v0.5 / v1 / v1.5a
  strategies emit market-priced signals so the field would be
  `None` pervasively today. v2 LLM strategies and v2.5 Kronos
  forecast overlay are explicit follow-up territory; the field
  joins the strawman shape at design-time of the first strategy
  that emits a price. **Forward-compat:** the column space in
  `strategy_signals` is reserved by a single `intended_price_str
  TEXT NULL` column in migration 009; the reader maps `NULL` →
  the struct simply omits the field at v1.9 (the column is just
  unused). When v2 needs it, the column is already there.
- **`signal_id` field added vs strawman:** Required for the
  second-row `UPDATE` from the risk-decide tap (Q1's
  `update_signal_clamp_status`). The writer creates a fresh
  UUID v4 on insert (same shape as `journal::post_fill`); the
  agent's risk-decide path carries the id alongside the `Signal`
  to update the clamp status. Also forward-compatible with a
  signal-detail modal (deferred follow-up).
- **`strategy_id: StrategyId` not `Option<StrategyId>`:** Signals
  are always emitted by a strategy (no anonymous signals exist;
  the registry guarantees this), so the non-`Option` shape is
  type-honest. Distinct from `JournalRow.strategy_id:
  Option<StrategyId>` (which has to be `Option` because pre-T802
  rows existed without it).

**How it shows up in code:**

- `crates/core/src/views.rs`: add the struct above.
- `crates/core/src/lib.rs`: re-export `pub use views::SignalView;`
  alongside the existing `FillView` / `JournalRow` re-exports.
- `crates/audit/src/query.rs::recent_signals` returns
  `Vec<SignalView>` reading from `strategy_signals` via a single
  SELECT against `(venue, symbol, ts)` plus the rfc-3339 binding
  pattern (lines 230–253 of `recent_fills_filtered` as the
  template).

### Crate / module surface

**New files (8 total):**

| Path | Purpose |
| ---- | ------- |
| `crates/audit/migrations/009_strategy_signals.sql` | Additive table for per-bar strategy signals (Q1). Idempotent `CREATE TABLE IF NOT EXISTS`. Indexes on `(ts)`, `(venue, symbol, ts)`, `(strategy_id, ts)`. |
| `crates/ui/src/widgets/chart_tooltip.rs` | Canvas-pass tooltip renderer (Q3). `draw_tooltip` fn taking `Frame`, `bounds`, `anchor`, `view`, `mode`. ~120 LOC. |
| `crates/ui/src/widgets/volume_histogram.rs` | Per-bar two-color stacked-bar widget (Q7). Sibling of `widgets::chart`. ~150 LOC. |
| `crates/ui/src/widgets/snapshots/chart__btc_with_two_buys_one_sell.snap` | **Re-baseline** of the existing snapshot to capture the new `MARKER_SIZE_PX = 13.0`, outline flag, draw-order line. (V1.) |
| `crates/ui/src/widgets/snapshots/chart_tooltip__buy_paper_fill.snap` | New tooltip render snapshot for a synthetic buy fill (V3 supporting baseline). |
| `crates/ui/src/widgets/snapshots/chart__with_ghosts_and_fills.snap` | New ghost+fill layered render snapshot (V5). |
| `crates/ui/src/widgets/snapshots/volume_histogram__btc_three_buys_two_sells.snap` | New per-bar histogram snapshot (V7 supporting baseline). |
| `crates/ui/src/widgets/snapshots/charts_screen_with_counters_and_chart.snap` | New full-screen layout snapshot for the Charts screen with tile strip + chart + histogram (V7). |

Tester acceptance note for the snapshots: all five new `.snap`
files land via `cargo insta accept` after the developer signs
off — they are NOT regenerated silently. The existing
`chart__btc_with_two_buys_one_sell.snap` re-baseline is the
**only** expected snapshot churn for prior shipped features;
V9 confirms every other panel snapshot stays byte-identical.

**New audit tests file:** `crates/audit/tests/recent_signals.rs`
(~120 LOC, fixture-loaded ledger asserting V11a / V11b / V11c).

**New cockpit-side integration test file:**
`crates/ui/tests/chart_tooltip_integration.rs` (V3 hover →
tooltip-state assertion).

**New cockpit-side integration test file:**
`crates/ui/tests/chart_marker_click_opens_modal.rs` (V4
click → `Message::TapeRowClicked` dispatch + modal-state
assertion).

**Existing files modified (12 total):**

| Path | Lines (approx) | What changes |
| ---- | --- | ------------ |
| `crates/core/src/views.rs` | +24 | Add `SignalView` struct (Q9). |
| `crates/core/src/lib.rs` | +1 | Re-export `SignalView`. |
| `crates/audit/src/lib.rs` | +1 | Re-export new `journal::post_strategy_signal` if needed (mirrors `post_fill` re-export pattern). |
| `crates/audit/src/journal.rs` | +90 | New `post_strategy_signal` + `update_signal_clamp_status` writers. |
| `crates/audit/src/query.rs` | +70 | New `recent_signals` reader, sibling of `recent_fills_filtered` (lines 223–269 as template). |
| `crates/agent/src/config.rs` | +20 | New `SignalLogConfig { enabled: bool }` (default `false`) in a `[signal_log]` TOML section. |
| `crates/ui/src/widgets/chart.rs` | +200 | `MARKER_SIZE_PX = 13.0`; `GHOST_MARKER_SIZE_PX = 8.0`; `snap_price_to_line` helper; `ChartProgram::State` → `ChartState`; `ChartProgram::update` implemented; `draw_triangle` gains outline + shadow params; new ghost-layer pre-pass; final tooltip pass via `chart_tooltip::draw_tooltip`; `chart_summary` test helper extended to include `draw_order`, `ghost_count`, `fill_count`, and `marker_size_px` fields. |
| `crates/ui/src/widgets/mod.rs` | +2 | `pub mod chart_tooltip;`, `pub mod volume_histogram;`. |
| `crates/ui/src/state.rs` | +60 | New `Cockpit.chart_signals: PanelState<Vec<SignalView>>`, `Cockpit.chart_tooltip: Option<ChartTooltipView>`. New `ChartTooltipView`, `ChartMarkerIndex` types. New `Message::ChartSignalsLoaded`, `Message::ChartMarkerHovered`, `Message::ChartMarkerHoverEnded` arms. `update` arms: pure assignments. |
| `crates/ui/src/screens/lab.rs` | +60 | Layout (β) implementation per Q5: chip row → tile-strip-with-position-mirror → chart → histogram. Compute `Vec<VolumeBin>` from `chart_markers` + `chart_buffer.bars(...)` at compose time. Compute the position-mirror's filtered slice via `model.positions.filter_to(symbol)`. |
| `crates/ui/src/strings.rs` | +20 | New `CHART_TOOLTIP_*`, `CHART_VOLUME_TILE_*`, `CHART_VOLUME_HISTOGRAM_LABEL`, `CHART_POSITION_MIRROR_LABEL`, `CHART_POSITION_MIRROR_NONE` constants per R4.7 + R7.7. Plus the dual entries in the `strings.rs::tests::all_strings_present` table. |
| `crates/ui/src/bin/cockpit_live.rs` | +30 | New `Task::perform` shim after `SelectSymbol` + after `BarClose` issuing `audit::query::recent_signals` and dispatching `Message::ChartSignalsLoaded`. Sibling of the existing `recent_fills_filtered` shim at lines 610–637. |

**Total surface:** 8 new files + 12 modified files. Estimated LOC
delta: ~+1100 additions, ~30 deletions (the `MARKER_SIZE_PX`
constant change and the `fill.price`-as-y replacement).

**Crates that do NOT change** (defence-in-depth for V9 regression
safety): `strategy/*`, `risk/*`, `backtest/*`, `reports/*`,
`exec/*`, `cost/*`, `reflection/*`, `models/*`, `data/*`,
`features/*`, `llm/*`. The agent runtime's `runtime.rs` is
**not touched in this brief** — the signal-emit dispatch lives in
the audit write path (writer is called by future agent-loop work,
not by this brief); the strategy trait stays fixed; bus channels
stay closed.

## UI

_ui-designer fills this. Second feature against
[ui-design-principles.md](../ui-design-principles.md). Principles
hooks: "Show the why" (R4.5 click-through reuses the tape-row modal);
density compact (R8 layout); "No blank screens" (R7.6); colour tokens
`UP_500` / `DOWN_500` / `BORDER_STRONG` (R1); "Numbers are scannable"
(R7.1 + R7.2 number formatting via `widgets::num`); keyboard / focus
parking lot (Q10)._

## Verification — links

_tester fills this — left blank intentionally._

## Changelog

- 2026-05-10 (analyst): initial draft. Promoted from operator
  feedback at the 2026-05-10 cockpit review. **12 R-items**, **13
  V-items**, **10 open questions** (six `[ARCHITECT-DECIDE]`,
  three `[OPERATOR-DECIDE]`, one parking-lot). Anchor risk: zero
  (R9.4) — pure UI + one additive read-only audit query, no
  strategy / risk / backtest / report-rendering code path touched.
  Version proposal `1.9.0` continues the main v1.x line (justified
  in `## Why`). HANDOFF → architect (orchestrator routes after
  operator resolves Q4 / Q5 / Q8).
- 2026-05-10 (orchestrator, operator-relayed via chat): operator
  resolved the three [OPERATOR-DECIDE] questions —
  - **Q4 → analyst strawman + drop truncated transaction ID**
    (sub-question accepted). Final tooltip: 6 fields (Side,
    Price, Quantity, Notional, Timestamp, Strategy ID). The full
    UUID is one click away in the journal-transaction modal so
    the truncated form was redundant.
  - **Q5 → Layout (β)** — chart keeps full width; cumulative-
    window-volume tile + open-position mirror in a status strip
    above; per-bar volume histogram below at fixed ~80px.
  - **Q8 → defer viewer parity** to a follow-up brief
    (`viewer-charts-parity`); cockpit-only this round.
  Six [ARCHITECT-DECIDE] questions remain (Q1 signal source —
  load-bearing; Q2 y-snap method; Q3 tooltip impl; Q6 marker
  visual treatment; Q7 histogram widget shape; Q9 SignalView
  type home). Parking-lot Q10 unchanged. Routing → architect.
- 2026-05-10 (architect): resolved the six [ARCHITECT-DECIDE]
  questions Q1 / Q2 / Q3 / Q6 / Q7 / Q9. **Q1 → Option (a)**
  additive `strategy_signals` table (migration 009), new
  `journal::post_strategy_signal` writer +
  `update_signal_clamp_status` second-row updater, new
  `audit::query::recent_signals` reader, new
  `SignalLogConfig { enabled: false }` agent-config section.
  Strategy trait stays fixed; no new bus channel; atomic-write
  contract preserved via existing journal-writer pattern.
  **Q2 → Option (b)** linear interpolation y-snap.
  **Q3 → Option (b)** custom canvas pointer-tracking + custom-
  drawn tooltip overlay; `iced::tooltip` (a) is the documented
  fallback. **Q6** — 13-px filled triangle + 1-px `BORDER_STRONG`
  outline + `shadow_1`-derived drop shadow for fills; 8-px
  60%-opacity `UP_400 / DOWN_400` for ghosts. No new theme tokens.
  **Q7 → Option (b)** new `widgets::volume_histogram`. **Q9** —
  `SignalView` lives in `crates/core/src/views.rs` (sibling of
  `FillView`), shape = strawman + `signal_id: SmolStr` for the
  second-row update tap. Parking-lot Q10 unchanged.
  Task range claimed: **T2001–T2027 + `T_FINAL_CHART_BUY_SELL_EMPHASIS`**.
  Anchor risk preserved zero (R9.4); 8 new files + 12 modified.
  HANDOFF → ui-designer (parallel with developer; UI-heavy
  feature; AGENT.md workflow rule for parallelism).
