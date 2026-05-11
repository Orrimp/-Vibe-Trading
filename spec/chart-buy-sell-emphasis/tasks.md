---
slug: chart-buy-sell-emphasis
status: in-progress
owner: ui-designer
updated: 2026-05-11
---

# Tasks — Chart buy/sell emphasis

High-level **milestones only** for now. The architect expands each
milestone into developer tasks (working name **T20xx**, but the
architect picks the exact range after Q-resolution; the next free
range after T19xx — claimed by `v2-llm-strategy` — is T20xx unless
the architect prefers a different counter) once Q1–Q9 are resolved
and Q4 / Q5 / Q8 are operator-confirmed.

Anchored to the R-items in
[`feature.md`](feature.md). Each milestone closes one or more R-items
and lands behind one or more V-items. **No file-level task fan-out
until architect Design** — premature task enumeration would re-litigate
the open questions (especially Q1 signal source plumbing, which
shapes M3 entirely).

## Milestones

### M1 — Marker visual fixes (size + outline + z-order + line-snap)

Pure rewrite of
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs).
Closes **R1** (markers obvious), **R2** (above the line), **R3**
(snapped to line), **R6** (keep triangle). Verified by **V1**, **V2**,
**V9** (existing snapshot churn expected on this milestone), **V10**
(determinism).

- Bump `MARKER_SIZE_PX` per Q6 architect resolution.
- Add `BORDER_STRONG` outline draw pass.
- Re-order `ChartProgram::draw` so the marker fill runs after the
  line stroke.
- Replace `fill.price.get()` y-source with the polyline-interpolation
  y-snap helper (Q2 architect resolution picks (a) or (b)).
- Optional drop-shadow pass per Q6 architect resolution.

**Resolved:** Q2 → linear interpolation; Q6 → 13-px triangle +
`BORDER_STRONG` outline + `shadow_1`-derived whisper shadow; ghost
layer = 8-px `UP_400 / DOWN_400` at 60% alpha. See `feature.md ##
Design`.

### M2 — Tooltip subsystem + click-through-to-modal

New widget + cockpit state + message arms + integration test. Closes
**R4** (hover tooltip + R4.5 click → modal). Verified by **V3**
(hover), **V4** (click → modal), **V9**, **V13** (consistency).

- New `widgets::chart_tooltip` (or in-canvas overlay if Q3 picks
  (b) — architect chooses widget vs canvas).
- `Cockpit.chart_tooltip: Option<ChartTooltipView>` plus
  `Message::ChartMarkerHovered(usize)` /
  `Message::ChartMarkerHoverEnded` arms.
- Wire marker click to the existing
  `Message::TapeRowClicked(transaction_id)` arm (reuses
  tape-row-audit-modal's shipped wiring).
- New `ui::strings::CHART_TOOLTIP_*` constants per R4.7.
- New panel snapshot `chart_tooltip_buy_paper_fill.snap`.

**Resolved:** Q3 → custom canvas pointer-tracking + custom-drawn
tooltip overlay (Option (b)). Q4 → operator-resolved (six tooltip
fields, no truncated tx-id). See `feature.md ## Design`.

### M3 — Signal source + layered render (ghost + fill)

The load-bearing milestone for this feature. Closes **R5**
(layered marker source), **R9** (no new bus channels in recommended
path), **R10** (consistency). Verified by **V5** (ghost+fill render),
**V11** (new audit reader), **V12** (config-gate default-off).

**Resolved Q1 → Option (a)** — additive `strategy_signals` table
(migration 009), new `journal::post_strategy_signal` +
`update_signal_clamp_status` writers, new
`audit::query::recent_signals` reader, new `core::SignalView`
type, new `SignalLogConfig { enabled: false }` agent config,
new cockpit `chart_signals: PanelState<Vec<SignalView>>` field +
`Message::ChartSignalsLoaded` arm.

**Resolved Q9** — `SignalView` lives in `crates/core/src/views.rs`
(sibling of `FillView`); shape per Design § Q9.

Layered draw-order pass (gridlines → labels → line → ghosts →
fills → tooltip) is implemented inside M1's `ChartProgram::draw`
re-order (T2004); M3 fills the ghost-layer iterator with real
`SignalView` data once T2018 lands. Ghost-marker tooltip variant
per R5.6 lands at T2019.

**Forward-compat note:** This milestone ships the writer + reader
+ config gate + cockpit read path. The live agent-runtime tap
point that actually calls `post_strategy_signal` is a parallel
agent-runtime track and a follow-up brief; with
`enable_signal_log = false` default the production ledger sees
zero new rows until an operator opts in.

See `feature.md ## Design § chart-buy-sell-emphasis Q1, Q9`.

### M4 — Counter views (tile + histogram + position mirror)

New widgets + Charts-screen layout reshape. Closes **R7** (three
counter views), **R8** (layout reshape). Verified by **V6** (tile
arithmetic), **V7** (snapshot), **V9**, **V10**.

- New `widgets::volume_tile` (or `kpi_strip` reuse — Q5 +
  architect picks).
- New `widgets::volume_histogram` per Q7 architect resolution (or
  extended `widgets::sparkline` if architect picks reuse).
- Reuse `widgets::positions` filtered to the active symbol per
  R7.3.
- Charts-screen reshape per Q5 operator resolution (analyst
  recommends layout (β)).
- New panel snapshot `charts_screen_with_counters_and_chart.snap`.

**Resolved:** Q5 → operator picked Layout (β); Q7 → new
`widgets::volume_histogram` (Option (b)). See `feature.md ##
Design § chart-buy-sell-emphasis Q7`.

### M5 — Ship gate

Tester runs the full V-pass. Closes nothing new; gates the entire
feature. Verified by **V8** (anchors 11/11), **V9** (workspace
green), **V10** (determinism), **V13** (consistency).

- `cargo test --workspace` green.
- `cargo test -p ui` + `cargo test -p ui --features live` green.
- `cargo test -p audit recent_signals` green (Q1 = (a) path only).
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`,
  zero diffs.
- New snapshots determinism-checked (two consecutive runs
  byte-identical).
- HANDOFF → presenter (operator approval gate per the standard
  spec-driven flow).

**Blocked on:** M1–M4 all green.

## Task expansion — T2001–T2027 + `T_FINAL_CHART_BUY_SELL_EMPHASIS`

**Range chosen:** T2001–T2027 (27 developer tasks) + the closing
tester gate. Next clean block after T1901–T1945 (v2-llm-strategy,
paused); leaves T1946–T2000 open for v2-llm-strategy's resume work
or smaller follow-ups. Each task cites the R-item it implements, ends
with a one-line acceptance the tester can verify, and is ordered by
dependency.

R-item citation legend below ties the T-tasks back to the brief:
`[R1.1]` = `feature.md` R1.1, `[R5.3]` = R5.3 etc. Task ownership tag
`[D]` = developer; `[U]` = ui-designer; `[D+U]` = co-owned (developer
lands the surface, ui-designer reviews/refines the visual treatment).
M5 is the tester gate.

### M1 — Marker visual fixes (T2001–T2005)

Closes **R1**, **R2**, **R3**, **R6**. Pure rewrite of
`crates/ui/src/widgets/chart.rs`. Verified by **V1**, **V2**,
**V9**, **V10**.

- [x] **T2001 [D]** — Bump `MARKER_SIZE_PX` from `6.0` → `13.0` and
  add `GHOST_MARKER_SIZE_PX = 8.0` constant in
  `crates/ui/src/widgets/chart.rs`. **[R1.1, Q6]**.
  _Acceptance:_ `grep -n "MARKER_SIZE_PX\|GHOST_MARKER_SIZE_PX"
  crates/ui/src/widgets/chart.rs` returns the two constants with
  values `13.0` and `8.0`. No other call sites change.
- [x] **T2002 [D+U]** — Extend `draw_triangle` signature with
  `outline: Option<Color>` and `shadow: Option<(iced::Vector,
  iced::Color)>` parameters; render shadow pre-pass (offset
  `(0.0, 1.5)`), then fill, then outline (1-px stroke). Add
  private helper `whisper_shadow(mode: ThemeMode) ->
  (iced::Vector, iced::Color)` reading from
  `theme::shadow::shadow_1(mode).color`. **[R1.2, R6.1, R6.3, Q6]**.
  _Acceptance:_ unit test `chart_draw_triangle_outline_and_shadow`
  asserts the helper returns the expected `(Vector{0.0,1.5},
  shadow_1.color)` for dark and light modes; no `#hex` literal
  introduced.
- [x] **T2003 [D]** — Add `snap_price_to_line(fill_ts: i64, bars:
  &[Bar]) -> Option<f32>` helper performing linear interpolation
  between the bracketing bars' `close` values. Replace the
  marker `y` derivation at the existing line 156-157 to use the
  snapped price instead of `fill.price.get()`. **[R3.1, R3.2,
  R3.3, Q2]**.
  _Acceptance:_ `cargo test -p ui chart_marker_y_snaps_to_line`
  passes — fixture: fill at exact midpoint ts between two bars
  whose closes differ by `dec!(100)` asserts rendered `y` is the
  interpolated midpoint y-pixel ± 0.5 px (V2).
- [x] **T2004 [D]** — Re-order `ChartProgram::draw` pass sequence
  to: gridlines → axis labels → line stroke → ghost-signal
  triangles (placeholder loop iterating an empty
  `Vec<SignalView>` at this milestone; M3 wires the real data)
  → executed-fill triangles → tooltip overlay placeholder.
  Extend `chart_summary` test helper with a `draw_order:`
  line emitting `gridlines,labels,line,ghosts,fills` (and
  later `,tooltip` when M2 lands). **[R2.1, R2.2]**.
  _Acceptance:_ updated insta snapshot
  `chart__btc_with_two_buys_one_sell.snap` lands via
  `cargo insta accept` reflecting the new constant + outline
  flag + `draw_order` line; second-run determinism check on the
  same fixture is byte-identical (V10).
- [x] **T2005 [D+U]** — Re-bless the existing
  `chart__btc_with_two_buys_one_sell.snap` baseline and run the
  full `cargo test -p ui` suite to confirm only the named
  snapshot churns (V1). **[R10.1, V1, V9]**.
  _Acceptance:_ `cargo test -p ui` green; `git diff
  crates/ui/src/widgets/snapshots/` shows exactly one modified
  file (`chart__btc_with_two_buys_one_sell.snap`) and zero
  others.

### M2 — Tooltip subsystem + click-through-to-modal (T2006–T2011)

Closes **R4** (R4.1–R4.7). Verified by **V3**, **V4**, **V9**,
**V13**. Can run in parallel with M3 — different code paths.

- [x] **T2006 [D]** — Add the new `Message` arms in
  `crates/ui/src/state.rs`:
  `ChartMarkerHovered(ChartMarkerIndex)`,
  `ChartMarkerHoverEnded`, `ChartSignalsLoaded(Result<
  Vec<SignalView>, SmolStr>)`. Add the new types
  `ChartMarkerIndex { Fill(usize), Signal(usize) }`,
  `ChartTooltipView { kind, side, price, qty, notional,
  ts, strategy_id, … }`. Add `Cockpit.chart_tooltip:
  Option<ChartTooltipView>` and `Cockpit.chart_signals:
  PanelState<Vec<SignalView>>` fields. Wire pure-function
  `update` arms (set / clear / replace). **[R4.1, R5.4, R10.3]**.
  _Acceptance:_ `cargo test -p ui --test consistency` green;
  `cargo build -p ui` green; no `_ =>` catch-all in the
  message-handler `match`.
- [x] **T2007 [D+U]** — Add new strings to
  `crates/ui/src/strings.rs`: `CHART_TOOLTIP_SIDE_BUY`,
  `CHART_TOOLTIP_SIDE_SELL`, `CHART_TOOLTIP_PRICE_LABEL`,
  `CHART_TOOLTIP_QTY_LABEL`, `CHART_TOOLTIP_NOTIONAL_LABEL`,
  `CHART_TOOLTIP_TS_LABEL`, `CHART_TOOLTIP_STRATEGY_LABEL`,
  `CHART_TOOLTIP_STRATEGY_NONE`, `CHART_TOOLTIP_GHOST_BADGE`.
  Register them in the `strings.rs::tests::all_strings_present`
  table. **[R4.7, R10.2]**.
  _Acceptance:_ `cargo test -p ui strings::tests::
  all_strings_present` green; `grep -n 'CHART_TOOLTIP'
  crates/ui/src/strings.rs` returns 9 lines.
- [x] **T2008 [D]** — Promote `ChartProgram::State` from `()` to
  `ChartState { hovered_marker_idx: Option<ChartMarkerIndex>,
  hovered_marker_centroid: Option<iced::Point> }`; implement
  `canvas::Program::update` consuming
  `mouse::Event::CursorMoved` and `mouse::Event::ButtonPressed`.
  Add `marker_hit_rect(anchor: Point) -> Rectangle` private
  helper returning a 28-px square. Emit
  `Message::ChartMarkerHovered` / `ChartMarkerHoverEnded` on
  hit-rect transitions. **[R4.1, R4.3, Q3]**.
  _Acceptance:_ unit test
  `chart_state_tracks_hovered_marker_idx` passes — synthetic
  `CursorMoved` event entering a marker's hit-rect transitions
  state from `None` to `Some(Fill(0))`; exit transitions back
  to `None`.
- [x] **T2009 [D+U]** — Create
  `crates/ui/src/widgets/chart_tooltip.rs` with
  `pub(crate) fn draw_tooltip(frame, bounds, anchor, view,
  mode)` rendering the six tooltip fields per R4.2. Position
  per R4.4 (prefer above-right; flip to below-left in the
  upper-right quadrant of the inner rect). Add the widget to
  `crates/ui/src/widgets/mod.rs` as `pub mod chart_tooltip;`.
  Wire as the **final** pass in `ChartProgram::draw` after the
  fill markers. **[R4.2, R4.4, R4.6]**.
  _Acceptance:_ new insta snapshot
  `chart_tooltip__buy_paper_fill.snap` lands via
  `cargo insta accept`; the captured fields match a hand-
  authored expected text fixture covering all six R4.2 fields
  (no Tx ID per Q4-operator-resolution).
- [x] **T2010 [D]** — Wire `Message::TapeRowClicked(transaction_id)`
  dispatch from the canvas-update `mouse::Event::ButtonPressed`
  arm for **fill** markers (ghost markers dispatch nothing on
  click). Reuses the existing wired-up tape-row-audit-modal
  machinery from R11.3; no new modal widget. Add the integration
  test `crates/ui/tests/chart_marker_click_opens_modal.rs`
  validating end-to-end click → modal-state assertion. **[R4.5,
  R11.3, V4]**.
  _Acceptance:_ `cargo test -p ui --test
  chart_marker_click_opens_modal` green;
  `cockpit.tape_audit_modal == Some(PanelState::Ready(view))`
  with `view.transaction_id == clicked_marker.transaction_id`.
- [x] **T2011 [D]** — Wire the cockpit-binary `Task::perform` shim
  for the hover-state update in `crates/ui/src/bin/cockpit_live.rs`
  alongside the existing `recent_fills_filtered` shim (no new
  async work — hover is pure-state; this is a no-op task for
  cockpit_live but ensures the `ChartMarkerHovered` arm has a
  shipped consumer that can be exercised by the integration
  test in T2009). Add integration test
  `crates/ui/tests/chart_tooltip_integration.rs` (V3).
  **[V3]**.
  _Acceptance:_ `cargo test -p ui --test
  chart_tooltip_integration` green; synthetic `CursorMoved`
  event at a marker hit-rect → `cockpit.chart_tooltip ==
  Some(ChartTooltipView { … })` with the expected field
  values.

### M3 — Signal source + layered render (T2012–T2020)

The load-bearing milestone — Q1 = (a). Closes **R5**, **R9**,
**R10.3** (new Message arm). Verified by **V5**, **V11**, **V12**.
Sequence: migration + writer + reader + core type + config land
**first** (T2012–T2016, independent of the UI work); the cockpit-
side ghost render + signal-fetch shim land **second** (T2017–T2020,
after T2006's `chart_signals` field).

- [x] **T2012 [D]** — Add `crates/core/src/views.rs::SignalView`
  per Q9 with the exact field shape:
  `{ signal_id: SmolStr, symbol: Symbol, side: Side,
  intended_qty: Quantity, signal_ts: Timestamp, strategy_id:
  StrategyId, was_clamped: bool, clamp_reason:
  Option<SmolStr> }`. Re-export from `crates/core/src/lib.rs`.
  **[R5.3, Q9]**.
  _Acceptance:_ `cargo build -p trading-core` green; new
  `core::SignalView` is accessible from downstream crates.
  Round-trip serde test
  `signal_view_serde_roundtrip` lands as part of
  `crates/core/src/views.rs::tests`.
  - VERIFIED — landed at `crates/core/src/views.rs:138-187` (struct)
    + `crates/core/src/views.rs:189-235` (test module);
    re-exported from `crates/core/src/lib.rs:53-55`. Test command:
    `cargo test -p trading_core signal_view_serde_roundtrip`.
    Output: `test views::tests::signal_view_serde_roundtrip ... ok`.
    Note: workspace package is `trading_core` (snake) not
    `trading-core` (hyphen); task spec uses the hyphen form but
    Cargo resolves both to the same crate. Test exercises both
    `was_clamped = true / Some(reason)` and `was_clamped = false /
    None` round-trips.
- [x] **T2013 [D]** — Author migration
  `crates/audit/migrations/009_strategy_signals.sql`. Schema:
  ```sql
  CREATE TABLE IF NOT EXISTS strategy_signals (
      id                TEXT PRIMARY KEY,
      ts                TEXT NOT NULL,       -- RFC3339 microsec
      strategy_id       TEXT NOT NULL,
      venue             TEXT NOT NULL,
      symbol            TEXT NOT NULL,
      side              TEXT NOT NULL,       -- 'buy' | 'sell' | …
      intended_qty_str  TEXT NOT NULL,       -- Decimal as TEXT
      intended_price_str TEXT,               -- forward-compat (Q9)
      was_clamped       INTEGER NOT NULL DEFAULT 0,
      clamp_reason      TEXT
  );
  CREATE INDEX IF NOT EXISTS strategy_signals_ts_idx
      ON strategy_signals(ts);
  CREATE INDEX IF NOT EXISTS strategy_signals_vs_idx
      ON strategy_signals(venue, symbol, ts);
  CREATE INDEX IF NOT EXISTS strategy_signals_sid_idx
      ON strategy_signals(strategy_id, ts);
  ```
  Header comment cites Q1 = (a), R5.3, R5.7, and the additive-
  no-data-backfill invariant. **[R5.5, Q1]**.
  _Acceptance:_ `cargo test -p audit migrations_apply_clean`
  green on an empty SQLite database; re-running the migrator is
  a no-op (sqlx tracks version).
  - VERIFIED — migration landed at
    `crates/audit/migrations/009_strategy_signals.sql`; ledger
    comment range bumped at `crates/audit/src/ledger.rs:31` from
    "001..008" to "001..009"; acceptance test at
    `crates/audit/tests/migration_009.rs:18-72`. Test command:
    `cargo test -p audit --test migration_009`.
    Output: `test result: ok. 2 passed; 0 failed; 0 ignored;
    0 measured; 0 filtered out; finished in 0.01s` (both
    `migrations_apply_clean` and `migration_009_is_idempotent`).
- [x] **T2014 [D]** — Implement
  `crates/audit/src/journal.rs::post_strategy_signal(ledger,
  signal, venue, was_clamped, clamp_reason)` and
  `update_signal_clamp_status(ledger, signal_id, was_clamped,
  clamp_reason)`. Both use the existing `ledger.pool.begin() /
  commit()` atomic-transaction pattern (sibling of
  `post_fill`). The first inserts a fresh UUID v4 id; the
  second is a single `UPDATE strategy_signals SET was_clamped,
  clamp_reason WHERE id = ?`. **[R5.5, hard-constraint 4]**.
  _Acceptance:_ `cargo test -p audit
  post_strategy_signal_writes_row` green — fixture asserts row
  count goes 0 → 1; `update_signal_clamp_status_flips_field`
  green.
  - VERIFIED with note — `post_strategy_signal` landed at
    `crates/audit/src/journal.rs:259-369`;
    `update_signal_clamp_status` at
    `crates/audit/src/journal.rs:385-417`; helper
    `signal_kind_to_side_str` at `crates/audit/src/journal.rs:420-444`.
    Tests at `crates/audit/src/journal.rs:1525-1701`. Test command:
    `cargo test -p audit --lib tests::`. Output:
    `test journal::tests::post_strategy_signal_writes_row ... ok`,
    `test journal::tests::update_signal_clamp_status_flips_field ... ok`,
    `test journal::tests::post_strategy_signal_skips_hold_kind ... ok`,
    `test journal::tests::post_strategy_signal_persists_intended_price ... ok`
    (24 tests total, all green, no regression on existing journal tests).
    NOTE: extended the architect-suggested signature from
    `(ledger, signal, venue, was_clamped, clamp_reason)` to
    `(ledger, signal, intended_qty: Quantity, intended_price:
    Option<Price>, venue, was_clamped, clamp_reason)` because the
    `strategy_signals.intended_qty_str` column is NOT NULL (per Q1
    schema in tasks.md T2013 + Q9). `Signal` does not carry quantity
    — the strategy emits intent, sizing is computed by the agent
    loop. The added params let the future agent-runtime tap point
    pass the sized quantity directly. Architect's spec implicitly
    requires this for the migration to be satisfiable; documented as
    a deviation here. Forward-compat `intended_price: Option<Price>`
    matches Q9 (`None` for v1 market strategies; `Some(_)` for v2
    limit-order shapes).
- [x] **T2015 [D]** — Implement
  `crates/audit/src/query.rs::recent_signals(ledger, venue,
  symbol, since, until) -> Result<Vec<SignalView>,
  LedgerError>`. Mirror the `recent_fills_filtered` shape
  (lines 223–269) — same RFC3339 binding, same
  `venue.to_string()` predicate, same time-window contract.
  Parse `intended_qty_str` to `Quantity`, map `NULL` →
  `clamp_reason = None`. Order `ts DESC, rowid DESC` for
  stable iteration. **[R5.3, V11]**.
  _Acceptance:_ `cargo test -p audit recent_signals` green
  with the three sub-tests V11a (correct rows, correct order),
  V11b (empty window → `Ok(vec![])`), V11c (gate-off ledger
  with no rows → `Ok(vec![])`).
  - VERIFIED — reader landed at `crates/audit/src/query.rs:335-432`;
    import added at `crates/audit/src/query.rs:13`. Integration tests
    at `crates/audit/tests/recent_signals.rs:1-234`. Test command:
    `cargo test -p audit recent_signals`.
    Output: `test result: ok. 5 passed; 0 failed; 0 ignored; 0
    measured; 0 filtered out; finished in 0.02s` covering all
    V11 sub-tests:
    - V11a: `test recent_signals_returns_window_subset ... ok`
    - V11b: `test recent_signals_empty_window_returns_ok_empty ... ok`
    - V11c: `test recent_signals_gate_off_ledger_returns_ok_empty ... ok`
    + two extra defensive sub-tests
    (`recent_signals_reflects_post_update_clamp_status`,
    `recent_signals_isolates_by_venue_and_symbol`).
- [x] **T2016 [D]** — Add `crates/agent/src/config.rs::SignalLogConfig
  { pub enabled: bool }` with `#[serde(default)] enabled: false`.
  Wire a `[signal_log]` section in `Config` (sibling of
  `[reflection]`). **[R5.7, V12]**.
  _Acceptance:_ `cargo test -p agent
  config_signal_log_default_off` green — V12 hard-asserts the
  default-off behaviour against a TOML without `[signal_log]`
  and against a TOML with `enabled = true`.
  - VERIFIED — `SignalLogConfig` landed at
    `crates/agent/src/config.rs:232-278`; wired into `Config` at
    `crates/agent/src/config.rs:480-490` and `Default` impl at
    `crates/agent/src/config.rs:506`. Tests at
    `crates/agent/src/config.rs:864-901`. Test command:
    `cargo test -p agent config_signal_log`.
    Output: `test config::tests::config_signal_log_default_off ... ok`
    + `test config::tests::config_signal_log_explicit_enable_round_trips
    ... ok` (2 passed; 0 failed). Covers both directions of V12 —
    omitted TOML section defaults to `false`, explicit `enabled =
    true` round-trips through serde.
- [x] **T2017 [D]** — Add the new `Task::perform` shim in
  `crates/ui/src/bin/cockpit_live.rs` issuing
  `audit::query::recent_signals` for the active `(venue, symbol)`
  on `SelectSymbol` and after `BarClose` for the active symbol
  (sibling of the existing `recent_fills_filtered` shim at
  lines 610–637). Dispatch `Message::ChartSignalsLoaded`.
  **[R5.4]**.
  _Acceptance:_ live-cockpit build is green
  (`cargo build -p ui --features live`); the new shim
  dispatches when a symbol is selected against a fixture
  ledger with one signal row.
- [x] **T2018 [D+U]** — Implement the ghost-signal-marker render
  pass in `ChartProgram::draw` (before the fill-marker pass,
  per the M1 draw-order). Iterate
  `self.signals: Vec<SignalView>`; for each, compute the
  same `(x, y)` via `x_for_index` + `snap_price_to_line`;
  draw an 8-px triangle in `UP_400` (Buy) or `DOWN_400`
  (Sell) at 60% alpha (`with_alpha`), no outline, no shadow.
  Plumb `Vec<SignalView>` from `Cockpit.chart_signals`
  through `screens::charts::view` into `ChartProgram`.
  **[R5.1, R5.4, Q6]**.
  _Acceptance:_ updated `chart_summary` test helper emits
  `ghost_count: N` and `fill_count: M`; new insta snapshot
  `chart__with_ghosts_and_fills.snap` captures a fixture with
  2 ghosts + 1 fill at overlapping bars and the `draw_order:
  gridlines,labels,line,ghosts,fills,tooltip` line (V5).
- [x] **T2019 [D+U]** — Extend the tooltip render to surface the
  ghost-variant with the `CHART_TOOLTIP_GHOST_BADGE` row and
  omit `Price` + `Notional` fields per R5.6. Driven by
  `ChartTooltipView.kind: Fill | Signal` from T2006. Include
  `was_clamped` / `clamp_reason` fields in the Signal-variant
  rendering. **[R5.6]**.
  _Acceptance:_ unit test
  `chart_tooltip_ghost_variant_renders_no_price` green —
  asserts that a `Signal`-kind view renders the badge and
  omits the `Price` / `Notional` label rows.
- [x] **T2020 [D]** — V5 acceptance + V11 acceptance gate:
  `cargo test -p ui chart_renders_ghost_and_fill_layers` green;
  `cargo test -p audit recent_signals` green; consistency
  test stays green. **Forward-compat documentation:** add a
  module-level comment to `crates/audit/src/journal.rs` near
  `post_strategy_signal` documenting that the live-loop
  caller (the agent runtime's per-bar signal-emit tap) is
  **deferred to a follow-up brief** — this feature ships the
  writer + reader + config gate + cockpit read path. With
  `enable_signal_log = false` default, zero rows land in
  prod; operator-flip is opt-in. **[R9.1, V5, V11]**.
  _Acceptance:_ all named tests green; doc comment present in
  the journal-writer module.

### M4 — Counter views (T2021–T2025)

Closes **R7**, **R8** (Layout β per Q5). Verified by **V6**, **V7**,
**V9**, **V10**. Can run in parallel with M3 — M4 reads only from
`chart_markers` (which already exists) and `model.positions`
(which already exists). Ghost-signal data does NOT feed any tile/
histogram in v1.9 (R7 strawman scope).

- [x] **T2021 [D]** — Add new strings to `crates/ui/src/strings.rs`:
  `CHART_VOLUME_TILE_BUYS_LABEL`, `CHART_VOLUME_TILE_SELLS_LABEL`,
  `CHART_VOLUME_TILE_NET_LABEL`, `CHART_VOLUME_TILE_TRADES_SUFFIX`,
  `CHART_VOLUME_HISTOGRAM_LABEL`, `CHART_POSITION_MIRROR_LABEL`,
  `CHART_POSITION_MIRROR_NONE`. Register in the
  `all_strings_present` table. **[R7.7, R10.2]**.
  _Acceptance:_ `cargo test -p ui --test consistency` green;
  `grep -n 'CHART_VOLUME\|CHART_POSITION' crates/ui/src/strings.rs`
  returns 7 lines.
- [x] **T2022 [D]** — Add tile-arithmetic helper to
  `crates/ui/src/screens/charts.rs` (or a private sibling
  module if size warrants): `compute_window_volume(markers:
  &[FillView]) -> (buys_usdt: Decimal, sells_usdt: Decimal,
  net_usdt: Decimal, buy_count: usize, sell_count: usize)`.
  Implement the cumulative tile widget by reusing
  `widgets::kpi_strip` shape (label + value + colour) — three
  number-pair tiles. **[R7.1, R7.4, R7.5]**.
  _Acceptance:_ `cargo test -p ui chart_counter_tile_sums`
  green (V6) — fixture: 3 buys ($30,000) + 2 sells ($20,000);
  asserts the tile renders `Buys in window: +$30,000.00 (3)`
  / `Sells in window: -$20,000.00 (2)` / `Net: +$10,000.00`.
- [x] **T2023 [D+U]** — Create
  `crates/ui/src/widgets/volume_histogram.rs` with
  `pub fn view(bins: Vec<VolumeBin>, mode: ThemeMode) ->
  crate::Element<'a>`. Sibling shape of
  `crates/ui/src/widgets/chart.rs`; reuses `inner_rect`,
  `with_alpha`, `GRIDLINE_COUNT` from `canvas_chart.rs`. Two-
  color stacked bars (buy up in `UP_500`, sell down in
  `DOWN_500`); centered on a y-axis-baseline split. Add the
  widget to `crates/ui/src/widgets/mod.rs`. **[R7.2, R7.6, Q7]**.
  _Acceptance:_ new insta snapshot
  `volume_histogram__btc_three_buys_two_sells.snap` lands via
  `cargo insta accept`; widget renders for the empty-bins case
  with the `"-"` placeholder per R7.6 (no blank screens).
- [x] **T2024 [D]** — Add the open-position-mirror helper
  filtering `model.positions: PanelState<Vec<PositionView>>`
  to the active symbol via the existing
  `widgets::positions::view` shape but with a private slice-
  filter. Reuses existing `widgets::positions` rendering — no
  new widget. **[R7.3, R7.4, R7.5]**.
  _Acceptance:_ unit test
  `position_mirror_filters_to_active_symbol` green — fixture:
  positions in `BTCUSDT` and `ETHUSDT`; mirror filtered to
  `BTCUSDT` shows the BTC row and omits ETH.
- [x] **T2025 [D+U]** — Reshape `crates/ui/src/screens/charts.rs`
  for Layout (β) (Q5-operator-resolved): chip row → tile-strip
  + open-position mirror in a horizontal strip → chart canvas
  (`Length::Fill`) → fixed 80-px histogram below. Compute
  `Vec<VolumeBin>` at compose time from `chart_markers` +
  `chart_buffer.bars(...)`. **[R7.1, R7.2, R7.3, R8.1, R8.2,
  R8.3]**.
  _Acceptance:_ new insta snapshot
  `charts_screen_with_counters_and_chart.snap` lands via
  `cargo insta accept`; rendering captures 3 buys + 2 sells +
  one open BTC long + the populated histogram (V7); chart
  canvas height stays > 50% of the screen body height at the
  default cockpit window size.

### M6 — Operator-feedback follow-up (T2028–T2030)

Added 2026-05-11 after operator's visual-verification pass on commit
`ff96ce4` surfaced three items: one bug in the T2018–T2020 deliverable
(tooltips don't fire on hover despite passing tests) plus two scope
additions (min window size; app icon). Folded into this feature's
pipeline per operator's "go with A" 2026-05-11 (alternative B/C would
have spawned a separate `cockpit-polish` feature). Re-runs `cargo
build` + `cargo test --workspace` + `bash scripts/verify_anchors.sh`
at the end of M6 before M5 ship gate is re-entered.

- [ ] **T2028 [U]** — **Min window size on all three bins.** Set
  `iced::window::Settings { min_size: Some(Size::new(1280.0, 720.0)),
  .. }` (or the lowest viable Layout-β width the ui-designer
  measures) on `crates/ui/src/bin/cockpit.rs`,
  `crates/ui/src/bin/cockpit_live.rs`, and
  `crates/ui/src/bin/viewer.rs`. Layout β (Q5 = β) requires the
  chart to stay above ~50% of body height with the status strip +
  histogram fitting their fixed allocations; that constrains the
  min width and height. **[Operator feedback 2026-05-11.]**
  _Acceptance:_ `cargo test -p ui min_window_size_set_on_all_bins`
  passes (new test verifies the `min_size` field is `Some(_)` on
  each bin's window settings); manual: shrinking the window below
  the min size in the running cockpit doesn't go below the limit.

- [ ] **T2029 [U]** — **App icon on all three bins.** Convert
  [`spec/design/project/assets/brand/lumen-mark.svg`](../design/project/assets/brand/lumen-mark.svg)
  to a pre-rendered PNG (or use an SVG-rasterising helper); embed
  the bytes via `include_bytes!` in a new
  `crates/ui/src/window_icon.rs`; apply via
  `iced::window::Settings { icon: Some(iced::window::icon::from_rgba(..., w, h)?), .. }`
  on `cockpit.rs`, `cockpit_live.rs`, `viewer.rs`. macOS uses the
  icon for dock + cmd-tab; Linux uses it for window decorations.
  No new dependencies if iced 0.14's `window::icon::from_rgba`
  accepts pre-rasterised bytes. If SVG-to-RGBA at runtime is
  needed, prefer a build-time rasterisation step via
  `build.rs` over a runtime SVG dep. **[Operator feedback
  2026-05-11.]**
  _Acceptance:_ `cargo test -p ui window_icon_set_on_all_bins`
  passes; manual: dock / cmd-tab / window decorations show the
  Lumen mark instead of the default iced placeholder.

- [ ] **T2030 [U]** — **Tooltip hover-detection rework
  (supersedes T2018–T2020 hover hookup).** Operator reported
  2026-05-11 that hovering over chart triangles produces no
  tooltip despite T2018–T2020 having shipped `[x]`. The ticks
  stay (honest-tick discipline) but this task documents the gap:
  the existing `chart_tooltip_integration.rs` test exercises
  render-given-hover-state, not hover-event-detection. The bug is
  in the pointer-event plumbing — either `canvas::Program::update`
  isn't intercepting `mouse::Event::CursorMoved`, the hit-rect
  math is off (markers' canvas coordinates vs cursor coordinates
  mismatched), or iced 0.14's canvas-pointer API behaves
  differently from the implementation's assumption.

  **Diagnosis steps:**
  1. Read `crates/ui/src/widgets/chart_tooltip.rs` `update` method;
     check that it returns `(canvas::event::Status::Captured, ...)`
     on `mouse::Event::CursorMoved`.
  2. Verify the canvas `update` method is wired into the chart's
     `canvas::Program` impl, not just defined.
  3. Print-debug the hit-rect math: do marker canvas coordinates
     (post linear-interpolation y-snap per Q2 = b) match the
     cursor's canvas-relative coordinates?
  4. If (1)–(3) are correct, fall back to the architect's
     documented Q3 option (a): replace the custom canvas pointer-
     tracking with `iced::widget::tooltip` widgets on a transparent
     overlay grid, one hit-rectangle per marker.

  **New integration test:** `crates/ui/tests/chart_tooltip_hover_fires.rs`
  exercises the actual hover-detection path — `canvas::Program::update`
  receives a synthetic `mouse::Event::CursorMoved` at the marker's
  canvas position, asserts the tooltip state flips to
  `Some(hovered_marker_idx)`. This is the test the ui-designer's
  existing `chart_tooltip_integration.rs` should have been but
  wasn't. **[Operator feedback 2026-05-11; supersedes T2018–T2020
  hover hookup.]**
  _Acceptance:_ `cargo test -p ui chart_tooltip_hover_fires`
  passes; manual: hovering over a triangle in `cargo run --release
  --bin cockpit --features fixtures` shows the 6-field tooltip
  (Side, Price, Quantity, Notional, Timestamp, Strategy ID) per
  Q4 = strawman-minus-truncated-TX-ID.

### M5 — Ship gate (T2026–T2027 + T_FINAL)

Closes nothing new. Verified by **V8** (anchors hard gate),
**V9** (workspace green), **V10** (determinism), **V13**
(consistency). Tester-only milestone.

- [x] **T2026 [D]** — Pre-tester self-validation: developer runs
  `cargo fmt`, `cargo clippy -- -D warnings`,
  `bash scripts/precheck.sh`, and the full `cargo test
  --workspace` against the local working tree. Any failure
  blocks the M5 handoff. **[R10.1, R10.2, R10.3, R11.5]**.
  _Acceptance:_ all four pre-checks green locally; signal
  HANDOFF → tester via the `present-results` skill.
- [x] **T2027 [D]** — Final cross-cut grep gate:
  `grep -rn '#[0-9a-fA-F]\{6\}'
  crates/ui/src/widgets/chart.rs
  crates/ui/src/widgets/volume_histogram.rs
  crates/ui/src/widgets/chart_tooltip.rs` returns zero hits;
  `grep -rn '"' crates/ui/src/widgets/chart.rs` audited row-by-
  row for inline user-visible strings (all should be
  `ui::strings::CHART_*` constants). **[R10.1, R10.2]**.
  _Acceptance:_ grep gates pass; consistency tests
  `no_inline_hex_colors_in_widgets_or_state` +
  `no_inline_user_visible_strings_in_widgets` green.
- [ ] **T_FINAL_CHART_BUY_SELL_EMPHASIS [tester]** — Tester runs
  the V-pass per the standard `rust-test` skill:
  - `cargo build --workspace` green (`rust-build`).
  - `cargo test --workspace` green (V9).
  - `cargo test -p ui` green (V1, V2, V3, V4, V5, V6, V7, V13).
  - `cargo test -p ui --features live` green (V9).
  - `cargo test -p audit recent_signals` green (V11).
  - `cargo test -p agent config_signal_log_default_off` green
    (V12).
  - **`bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`,
    zero diffs vs `spec/anchors.toml`** (V8 — hard gate).
  - Each new insta snapshot run **twice** consecutively
    byte-identical (V10).
  - Renders the file-paths-touched matrix per the developer's
    handoff and pin-checks that `crates/strategy/*`,
    `crates/risk/*`, `crates/backtest/*`, `crates/reports/*`,
    `crates/exec/*` show **zero** modifications.
  - Files a `spec/chart-buy-sell-emphasis/reports/test-
    <timestamp>.md` per the
    [test-report.md template](../../.claude/skills/rust-test/templates/test-report.md).
  - VERDICT → PASS triggers HANDOFF → presenter.
  _Acceptance:_ tester emits PASS verdict; presenter spawned.

## Cross-task notes for the developer

- **T2001–T2005 (M1) and T2006–T2011 (M2) can fan out in parallel.**
  Different code paths: M1 stays inside
  `crates/ui/src/widgets/chart.rs`'s `draw` body + `draw_triangle`;
  M2 touches the new `chart_tooltip.rs`, `state.rs` Message arms,
  and the canvas `update` impl. The `Vec<SignalView>` field added
  by T2006 is consumed by T2018 in M3; T2006 must precede T2018.
- **T2012–T2016 (audit writer + reader + core type + config) are
  back-end-only — no UI dependency.** Can land before any M1/M2
  work if convenient. Note that T2013 must land before T2014
  (writer needs the table) and before T2015 (reader needs the
  table); T2014 must land before T2015 (V11a needs writer fixtures
  to assert reader output).
- **T2017–T2020 (cockpit-side ghost render + integration) blocks
  on T2006 (Message arm) and T2015 (reader).** Developer sequences
  this either as M3-second-half after M2 lands, or in parallel
  with M4.
- **T2021–T2025 (M4 counter views) reads only existing
  `chart_markers` + `positions` — fully parallel with M3.**
- **Anchor-risk negative invariant test:** there is no new V-item
  for this; V8's existing 11/11 PASS rule is the existing gate.
  The architecture-level invariant is "no `crates/strategy/`,
  `crates/risk/`, `crates/backtest/`, `crates/reports/`,
  `crates/exec/` modifications" — the tester's file-paths-touched
  matrix at T_FINAL is the operator-facing evidence.
- **Hard-constraint cross-check for the developer at landing:**
  1. `Strategy` trait shape unchanged — `grep -n "pub trait
     Strategy" crates/strategy/src/traits.rs` returns one match
     with the unchanged signature.
  2. No new bus channel — `grep -rn "pub fn .*channel\|pub
     enum Bus" crates/agent/src/bus.rs` returns the same set as
     pre-T2001.
  3. Atomic-write contract preserved — `post_strategy_signal`
     uses `ledger.pool.begin() / commit()` (grep
     `crates/audit/src/journal.rs` shows the same pattern as
     `post_fill`).
  4. Body-vs-front-matter discipline preserved — no new report-
     body rendering this brief; n/a.
  5. Lumen tokens only — `grep -rn '#[0-9a-fA-F]\{6\}'
     crates/ui/src/widgets/{chart,chart_tooltip,volume_histogram}.rs`
     returns zero (T2027 gates this).
  6. Insta snapshot baselines via `cargo insta accept` only — no
     `assert_snapshot!(..., "<new-baseline>")` calls that auto-
     write.
  7. No Python / external runtime dependencies — no
     `subprocess`, no `reqwest::Client::new()` outside existing
     LLM provider crates.

## Owner tags (architect-confirmed)

- **M1:** `[D]` for code, `[U]` for visual review on T2002 /
  T2005 (marker size, outline, shadow at the Lumen-token
  level).
- **M2:** `[D+U]` — co-owned. UI-designer drives the tooltip
  visual treatment + positioning + ghost-badge layout (T2007,
  T2009, T2019); developer drives the message wiring + canvas-
  update plumbing + integration tests.
- **M3:** `[D]` for migration + writer + reader + config +
  cockpit shim (T2012–T2017, T2020); `[D+U]` for the ghost
  render pass (T2018) and tooltip ghost variant (T2019).
- **M4:** `[D+U]` predominantly UI-designer-led — new widget +
  screen layout. Developer lands the tile arithmetic helper
  (T2022).
- **M5:** `[D]` for the pre-tester sweep (T2026, T2027); the
  tester owns T_FINAL.

## Parallelism / sequencing summary

```
M1 (T2001-T2005)  ─┐
                   ├─→ M2 (T2006-T2011) ─┐
                   │   reads M1 work    │
M3 back-end (T2012-T2016)  ─────────────┼─→ M3 cockpit (T2017-T2020)
                                        │       │
M4 (T2021-T2025)  reads only existing  ─┤       │
   state, parallel anywhere             │       │
                                        ▼       ▼
                                  M5 (T2026-T2027 + T_FINAL)
```

ui-designer and developer can land M1 + M2 + M4 in parallel
fan-out. M3 back-end is fully independent (audit-crate-only).
M3 cockpit blocks on M2's `Cockpit.chart_signals` field
(T2006) **and** M3 back-end's reader (T2015). M5 is the join
point.

## Parallelism hints (analyst's prior — superseded 2026-05-10)

_Superseded by the architect's `## Parallelism / sequencing
summary` above. The analyst's prior matches the architect's
conclusion in shape (M1 ‖ M2 ‖ M3-backend ‖ M4; M3-cockpit blocks
on M2 + M3-backend; M5 joins). Kept here as a paper trail for the
analyst's framing._

- **M1 ‖ M2** — different code paths. Confirmed (T2001–T2005 ‖
  T2006–T2011 in parallel).
- **M3 critical-path.** Confirmed for the cockpit-side fan-in
  (T2017–T2020 blocks on T2006 + T2015); M3-backend (T2012–T2016)
  is fully independent.
- **M4 ‖ M3** — confirmed. M4 reads only existing
  `chart_markers` + `positions`; no `chart_signals` dependency in
  the R7 strawman (analyst's exception-clause about Q4 extension
  did not materialize — operator confirmed the strawman as-is).
- **M5 blocks on M1+M2+M3+M4 all green** — confirmed at T_FINAL.

## Owner tags (analyst's prior — superseded 2026-05-10)

_Superseded by `## Owner tags (architect-confirmed)` above. The
architect's mapping differs from the analyst's prior in two
places: M1's T2002 (visual treatment) is `[D+U]` co-owned not
pure `[U]` (the outline + shadow code lives inside `draw_triangle`,
not in a `widgets/` body); M4's T2022 is pure `[D]` (tile
arithmetic is a derived-state helper, not a widget edit)._

- **M1:** analyst said `[ui-designer]`; architect amends to
  `[D]` with `[U]` review on T2002 / T2005.
- **M2:** analyst said `[ui-designer]` for widget + snapshot,
  `[developer]` for `Message::*` arms. Architect confirms — `[D+U]`
  co-owned per the architect-confirmed table.
- **M3:** analyst said `[developer]` for back-end +
  `[ui-designer]` for ghost render + tooltip variant. Architect
  confirms — `[D]` for T2012–T2017, T2020; `[D+U]` for T2018 +
  T2019.
- **M4:** analyst said `[ui-designer]` predominantly with
  `[developer]` for derived-state helpers. Architect amends to
  `[D+U]` predominantly UI-designer-led with the developer
  owning T2022.
- **M5:** analyst said `[tester]`. Architect confirms — `[D]`
  owns the developer self-validation T2026 / T2027;
  `T_FINAL_CHART_BUY_SELL_EMPHASIS` is the tester's sole task.

## Changelog

- 2026-05-10 (architect): expanded the T-task list with 27
  developer tasks T2001–T2027 + `T_FINAL_CHART_BUY_SELL_EMPHASIS`.
  Resolved Q1 / Q2 / Q3 / Q6 / Q7 / Q9 in `feature.md ## Design`.
  Marked the analyst's prior parallelism + owner-tag sections
  as superseded; new architect-confirmed sections sit above
  the superseded ones in the file. T2001 starts M1 (size +
  outline + shadow + draw order); T_FINAL is the tester gate.
- 2026-05-11 (ui-designer): completed the UI track — T2001–T2011
  (markers + tooltip + click-through), T2017 (cockpit_live
  signals shim — landed alongside developer's T2015), T2018–T2020
  (ghost-signal render + ghost tooltip variant + V5/V11 gate),
  T2021–T2025 (counter views + Layout β reshape), T2026 + T2027
  (self-validation grep gate). `core::SignalView` was available when
  T2018 reached it (developer's T2012 had already landed). Three
  insta snapshots re-baselined via `cargo insta accept`:
  `chart__btc_with_two_buys_one_sell` (M1 churn —
  `draw_order` line + ghost/fill counts + marker sizes +
  outline/shadow tokens), `chart__with_ghosts_and_fills` (new — V5
  ghost+fill render), `volume_histogram__btc_three_buys_two_sells`
  (new — M4 widget), `charts_screen_with_counters_and_chart` (new
  — V7 Layout β snapshot). `cargo test -p ui` and
  `cargo test -p ui --features live` green; `cargo test --workspace`
  green (143 test binaries, zero failures);
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`
  zero drift. `cargo fmt -p ui` clean. T_FINAL stays
  unticked — tester-only per the orchestrator's contract.
  HANDOFF → tester (T_FINAL_CHART_BUY_SELL_EMPHASIS).
- 2026-05-11 (orchestrator, operator-relayed via chat): operator's
  visual-verification pass on commit `ff96ce4` surfaced three items
  during a cockpit launch — one bug in the T2018–T2020 deliverable
  (tooltips don't fire on hover despite passing tests) plus two
  scope additions (min window size for all three bins; app icon
  for cockpit / cockpit_live / viewer). Operator picked Option A
  (fold into this feature's pipeline; defer alternative B/C which
  would have spawned a separate `cockpit-polish` brief). Added new
  milestone **M6** with three tasks **T2028** (min window size),
  **T2029** (app icon), **T2030** (tooltip hover-detection rework
  that supersedes T2018–T2020 hover hookup). T2018–T2020 `[x]`
  ticks remain — honest-tick discipline preserves the historical
  record; T2030 documents the gap and contains the fix. M5 ship
  gate re-enters after M6 lands. HANDOFF → ui-designer (M6 pass).
