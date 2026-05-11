---
slug: chart-buy-sell-emphasis
status: shipped
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

- [x] **T2028 [U]** — **Min window size on all three bins.** Set
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
  **Done 2026-05-11 (ui-designer, M6 follow-up):**
  - Constants live in `crates/ui/src/window_icon.rs:30,35`
    (`MIN_WINDOW_WIDTH_PX = 1280.0`, `MIN_WINDOW_HEIGHT_PX = 720.0`).
    Lowest viable Layout-β width measured against sidebar
    (180 px) + body padding + chip-row + the volume-tile three-cell
    layout; 1280 keeps the trailing `(N trades)` suffix scannable
    and chart-height share ≥ 61 % of body at 720 px.
  - Shared helper `window_icon::standard_window_settings()`
    (`crates/ui/src/window_icon.rs:69-77`) sets `size` + `min_size`
    + `icon` in one place; all three bins call
    `.window(ui::window_icon::standard_window_settings())`:
    `crates/ui/src/bin/cockpit.rs:118-122`,
    `crates/ui/src/bin/cockpit_live.rs:462-465`,
    `crates/ui/src/bin/viewer.rs:72-75`.
  - Verbatim test output: `test window_icon::tests::min_window_size_set_on_all_bins ... ok`
    (`cargo test -p ui --lib window_icon::tests::min_window_size_set_on_all_bins`,
    `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 121 filtered out`).

- [x] **T2029 [U]** — **App icon on all three bins.** Convert
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
  **Done 2026-05-11 (ui-designer, M6 follow-up):**
  - Lumen mark rasterised once locally via a throwaway
    `resvg + tiny-skia + usvg` helper (NOT shipped in the
    workspace — kept out-of-tree so neither the runtime crate nor
    the workspace build pulls SVG-rasterisation as a dep, honoring
    the no-new-deps constraint). Raw 64×64 RGBA bytes
    (`64 * 64 * 4 = 16_384` bytes) committed at
    `crates/ui/assets/lumen-mark-64x64.rgba` and `include_bytes!`-d
    in `crates/ui/src/window_icon.rs:46` (`LUMEN_MARK_RGBA`).
  - `lumen_window_icon()` (`crates/ui/src/window_icon.rs:58-60`)
    wraps `iced::window::icon::from_rgba(rgba, 64, 64)`;
    `standard_window_settings()` attaches the icon to every bin
    (`crates/ui/src/window_icon.rs:75`).
  - All three bins now ship the icon via the shared call site
    (`crates/ui/src/bin/cockpit.rs:121`,
    `crates/ui/src/bin/cockpit_live.rs:464`,
    `crates/ui/src/bin/viewer.rs:74`).
  - Verbatim test output: `test window_icon::tests::window_icon_set_on_all_bins ... ok`
    (`cargo test -p ui --lib window_icon::tests::window_icon_set_on_all_bins`,
    `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 121 filtered out`).

- [x] **T2030 [U]** — **Tooltip hover-detection rework
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
  **Done 2026-05-11 (ui-designer, M6 follow-up):**
  - Stayed on **Q3 option (b)** — custom-canvas pointer-tracking
    (the fallback Q3 option (a) overlay-grid was NOT needed). The
    pre-existing `ChartProgram::update` impl
    (`crates/ui/src/widgets/chart.rs:118-180`) already intercepted
    `mouse::Event::CursorMoved`, did the inner-rect hit-test, and
    published `Message::ChartMarkerHovered(idx).and_capture()` —
    the architect's diagnosis (1), (2), and (3) all checked out
    against the iced 0.14 canvas API.
  - **The real gap was test coverage**: the pre-T2030 suite never
    drove `ChartProgram::update` with a synthetic `CursorMoved`,
    so the dispatch could regress silently. Writing the integration
    test directly **also** surfaced a latent UX bug — see next item.
  - **Latent bug fixed**: the pre-T2030 `update` `?`-bailed at
    `cursor.position_in(bounds)?` on EVERY event type, so a
    cursor swept off the canvas while a marker was hovered never
    reached the `HoverEnded` branch — the tooltip latched on the
    last hovered marker until the cursor re-entered the canvas
    over a different one. T2030 reworks the `CursorMoved` arm so
    `position_in` returning `None` STILL publishes `HoverEnded`
    when prior state had a hover, AND only `and_capture()`s when
    the cursor was actually over the canvas
    (`crates/ui/src/widgets/chart.rs:118-181`). The
    `ButtonPressed(Left)` arm keeps the `?`-bail since clicks
    require the cursor to be on the canvas.
  - **New integration test**
    `crates/ui/tests/chart_tooltip_hover_fires.rs` exercises the
    full canvas-event → hit-test → message-publish path through
    a public `#[doc(hidden)]` test-helper
    `widgets::chart::dispatch_canvas_event_for_test`
    (`crates/ui/src/widgets/chart.rs:626-686`) + opaque
    `widgets::chart::ChartHoverState` wrapper
    (`crates/ui/src/widgets/chart.rs:691-705`). Six tests pin:
    cursor-on-marker publishes `Hovered(Fill(0))` + `Captured`;
    cursor-off-marker publishes nothing; hover-then-leave
    publishes `HoverEnded`; ghost markers publish
    `Hovered(Signal(0))`; the cursor-leaves-canvas-while-hovering
    regression is locked in; idempotent dispatch over the same
    marker only publishes once. The existing
    `chart_tooltip_integration.rs` keeps its render-given-hover
    coverage — T2030 strictly **adds** a layer.
  - Verbatim test output:
    ```
    running 6 tests
    test cursor_moved_over_marker_publishes_hover_message ... ok
    test cursor_moved_repeated_over_same_marker_publishes_once ... ok
    test cursor_moved_off_marker_does_not_publish_hover ... ok
    test cursor_moved_over_ghost_marker_publishes_signal_hover ... ok
    test cursor_moved_then_leaving_publishes_hover_ended ... ok
    test cursor_leaving_canvas_while_hovering_publishes_hover_ended ... ok
    test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
    ```
    (`cargo test -p ui --test chart_tooltip_hover_fires`).
  - Manual confirmation that the cockpit boots cleanly with the
    new bin chrome (T2028 + T2029 + T2030) via
    `cargo build --release --bin cockpit --features fixtures` and
    `cargo build --release --bin cockpit_live --features live`,
    both green. End-to-end visual hover-check in a windowed iced
    session is left to the tester at T_FINAL — the agent runs
    headless and can't paint a window.

### M6.2 — Second operator-feedback follow-up (T2031–T2033)

Added 2026-05-11 after operator's visual-verification pass on commit
`9bb5786` (M6 first pass) surfaced three further runtime defects: no
visible app/dock/cmd-tab icon on macOS despite the icon test passing;
chart canvas crops on window-resize instead of scaling; tooltip flashes
briefly on hover and immediately disappears.

**Pattern note:** M6's first pass shipped tests-pass + runtime-broken.
ui-designer can't visually verify in headless sandbox; the test suite
covers logic + rendered snapshots but not interactive behaviour.
**Mandatory new discipline for this pass:** for each tick that
touches visible UI behaviour (T2032, T2033), the ui-designer MUST:

1. Launch `cargo run --release --bin cockpit --features fixtures`
   in the background.
2. Capture a fullscreen screenshot via
   `screencapture -x /tmp/cockpit-T20xx-<short-name>.png` after
   the cockpit window has been on-screen for ≥4 seconds.
3. Open the screenshot via `Read` tool and visually verify the
   tick's acceptance against the rendered output.
4. Cite the screenshot path in the tick footer alongside the
   file:line + test-output evidence.

This is the project-level testing-strategy gap the first two M6 passes
exposed. The screenshot-verification gate is not negotiable for any
T2032 / T2033 tick. T2031 (documentation + brief stub) doesn't render
visible UI behaviour, so it's exempt.

- [x] **T2031 [U]** — **Document macOS dock-icon limitation +
  open follow-up brief stub for `.app` bundling.** The
  `iced::window::Settings::icon` setting affects the title-bar icon
  only (and even there some macOS configurations hide it). The dock
  + cmd-tab + Spotlight + Finder app icon all come from an `.app`
  bundle's `Info.plist` + `.icns` file, NOT from iced's runtime
  window setting. A bare `cargo run` binary cannot change the macOS
  dock icon without being wrapped via `cargo bundle` or hand-written
  `Info.plist`. Document this in
  [`crates/ui/src/window_icon.rs`](../../crates/ui/src/window_icon.rs)
  as a module-level note (it's invisible truth that the next reader
  will hit). Create a stub feature folder
  `spec/cockpit-app-bundle/feature.md` with `status: candidate` and
  the bundle approach captured (analyst spawn when promoted; not
  before). **[Operator feedback 2026-05-11.]**
  _Acceptance:_ `grep -n "macOS dock icon" crates/ui/src/window_icon.rs`
  surfaces the limitation note; `ls spec/cockpit-app-bundle/feature.md`
  exists with `status: candidate`. No runtime change in the cockpit
  binary; the existing test stays green (the iced-level icon plumbing
  IS correct — it's the macOS surface that needs `.app` bundling).

  **Done 2026-05-11 (ui-designer, M6.2):**
  - Added a `## macOS dock icon limitation (T2031, M6.2)` module-
    level section to
    [`crates/ui/src/window_icon.rs`](../../crates/ui/src/window_icon.rs)
    explaining why
    [`lumen_window_icon`](../../crates/ui/src/window_icon.rs)'s
    plumbing is correct (Linux + Windows benefit) but macOS dock /
    cmd-tab / Spotlight / Finder all read from an `.app` bundle's
    `Info.plist` + `.icns` instead, with the fix-path breadcrumb to
    the new candidate-stub.
  - Created [`spec/cockpit-app-bundle/feature.md`](../cockpit-app-bundle/feature.md)
    (`status: candidate`, `owner: pending-analyst`, version `0.1.0`)
    matching the shape of
    [`spec/v25-kronos-forecast-overlay/feature.md`](../v25-kronos-forecast-overlay/feature.md):
    seven open questions (tool choice, per-bin vs single bundle,
    icon-rasterisation pipeline, signing/notarisation, CI gate,
    Linux+Windows surface, determinism contract), promotion
    checklist, and cross-references back to this M6.2 entry +
    `window_icon.rs`.
  - Acceptance verified: `grep -n "macOS dock icon"
    crates/ui/src/window_icon.rs` → `25:` and `29:` hits;
    `ls spec/cockpit-app-bundle/feature.md` → file exists with
    `status: candidate` in frontmatter.
  - No runtime change. `cargo test -p ui --lib window_icon` →
    `min_window_size_set_on_all_bins` + `window_icon_set_on_all_bins`
    both green (the iced-level plumbing IS correct).
  - No screenshot needed — T2031 is documentation-only per the M6.2
    gate carve-out.

- [x] **T2032 [U]** — **Chart scales on window resize.** Currently
  the canvas crops instead of scaling when the operator drags the
  window edge. Suspect: chart parent (Layout β's chart-column inside
  the status-strip-above + histogram-below sandwich) doesn't
  propagate `Length::Fill` to the canvas correctly; OR the canvas's
  `bounds.size()` doesn't refresh on resize; OR the iced::Column
  proportions allocate fixed pixels to the status strip + histogram
  but the chart's `Length::Fill` gets clipped at the initial size.
  Diagnose first; fix follows from the diagnosis.

  **Mandatory screenshot verification:** capture before the fix
  (cropped state), apply the fix, capture after, confirm via
  read-screenshot that the chart fills its allocated body region
  at three window sizes: 1280×720 (min), 1600×900 (mid), 1920×1080
  (large). All three screenshots cited in tick footer.

  **[Operator feedback 2026-05-11.]**
  _Acceptance:_ three screenshots at three window sizes show the
  chart canvas filling its column allocation (no horizontal or
  vertical cropping bands). New unit test asserts
  `chart_canvas_height_grows_with_body_height` — when given a 1080-
  high body, the chart canvas height calculation returns > the 720-
  high body's chart canvas height.

  **Done 2026-05-11 (ui-designer, M6.2):**

  **Diagnosis.** The first suspect — chart-column not propagating
  `Length::Fill` to the canvas — turned out to be the right one,
  but at a different layer than the brief speculated. The canvas
  itself ([`crates/ui/src/widgets/chart.rs:84-95`](../../crates/ui/src/widgets/chart.rs))
  correctly declares
  `Canvas::new(...).width(Length::Fill).height(Length::Fill)` wrapped
  in a `Container` with both axes Fill. The bug was one level up:
  in [`crates/ui/src/screens/charts.rs`](../../crates/ui/src/screens/charts.rs)
  at the pre-fix line `Container::new(chart_body).height(Length::Fill)`
  — **missing `.width(Length::Fill)`**. The default Container width
  in iced 0.14 is `Length::Shrink`, and a `Shrink` parent collapses
  any `Length::Fill` child to zero. So the chart-body container was
  Shrink-width despite housing a Fill-width canvas, which is why
  the chart looked "cropped" at the initial 1280×720 size and
  refused to grow on resize — the column never gave it horizontal
  room to grow into. The status strip + histogram both explicitly
  set `.width(Length::Fill)`, which masked the bug for sibling
  rows.

  **Fix.** Replaced the single-axis Container with an explicit
  two-axis one:
  ```rust
  .push(Container::new(chart_body)
            .width(Length::Fill)
            .height(Length::Fill))
  ```
  at
  [`crates/ui/src/screens/charts.rs:152-167`](../../crates/ui/src/screens/charts.rs)
  with an in-source comment explaining the `Shrink`-default trap so
  the next reader doesn't reintroduce the regression.

  **Unit test.** Added
  [`screens::charts::tests::chart_canvas_height_grows_with_body_height`](../../crates/ui/src/screens/charts.rs)
  per the acceptance contract. Backed by a new pure helper
  [`chart_canvas_height_for_body(f32) -> f32`](../../crates/ui/src/screens/charts.rs)
  that mirrors the Layout β budget arithmetic (chip row + status
  strip + chart Fill + histogram label + 80-px histogram canvas, in
  a Column with `space::M` spacing and `space::L` padding). Asserts:
  (a) `h_1080 > h_720`, (b) `h_720 > 0` (Q5 floor defended), (c) the
  growth `delta == 360 px` exactly (body-height delta == chart-height
  delta since the fixed siblings are body-invariant).
  - `cargo test -p ui --lib screens::charts::tests::chart_canvas_height_grows_with_body_height`
    → green.
  - `cargo test -p ui --lib screens::charts::` → 4/4 green (the
    three pre-existing tests stayed green; new test joined cleanly).

  **Screenshot verification — CAPTURED (M6.2 fixup pass,
  2026-05-11).** macOS Screen Recording permission, missing on the
  M6.2 first pass, is now granted to the host process and
  `screencapture -x` works.  Three screenshots captured at three
  window sizes against the post-fix release binary; each was
  read-back inline via the `Read` tool and visually inspected:

  - **1280×720** (Layout-β min floor) →
    `/tmp/cockpit-T2032-1280x720.png` (2 640 155 bytes).  Chart
    canvas occupies the full body region between the chip row +
    cumulative-volume strip above and the volume histogram below;
    the BTCUSDT price line plots end-to-end across the canvas
    width with no horizontal or vertical cropping bands.
  - **1600×900** (mid-size) →
    `/tmp/cockpit-T2032-1600x900.png` (2 516 673 bytes).  Chart
    canvas grew with the body; both the upward (Buy) and downward
    (Sell) fill-marker triangles are visible at their bar-aligned
    positions; no cropping.
  - **1920×1080** (large) →
    `/tmp/cockpit-T2032-1920x1080.png` (2 175 326 bytes).  Full
    1920×1080 viewport; sidebar (Home / Debug / Strategies / Risk
    / Audit / **Charts** highlighted / Control) renders at its
    canonical 180 px width; chart canvas takes the remaining
    horizontal body and the full vertical allocation between the
    fixed siblings.  Multiple fill markers visible along the line.

  Capture harness (reverted after the captures, see "Files
  touched" below):
  1. Temporarily set `cockpit.current_screen = Screen::Charts` in
     `crates/ui/src/bin/cockpit.rs::boot` so the binary booted
     directly on the Charts screen (AppleScript / Accessibility
     permission to click "Charts" in the sidebar is not granted
     to the host process; Screen Recording is a separate TCC
     class and *is* granted).
  2. Temporarily added a `COCKPIT_INIT_W` / `COCKPIT_INIT_H`
     env-var override on `crates/ui/src/window_icon.rs::standard_window_settings`
     so the same release binary could boot at three sizes without
     three rebuilds; the override floors at `MIN_WINDOW_*_PX`.
  3. For each `(W, H)` triple, ran
     `COCKPIT_INIT_W=W COCKPIT_INIT_H=H ./target/release/cockpit &`,
     waited 6 s for the window to draw, `screencapture -x` to
     the cited path, `pkill -f target/release/cockpit`, re-read
     the PNG inline.  Both temp tweaks reverted in this same
     commit; `cargo test -p ui --lib window_icon::tests` stayed
     green throughout (the constants didn't move; only the
     starting `size` did).

  Joint evidence stack (all three pin the same invariant from
  different angles):
  1. The new unit test
     `chart_canvas_height_grows_with_body_height` — pure
     arithmetic regression guard on the Layout-β budget.
  2. Three live screenshots above — visual confirmation that the
     `view()` composition the test pins also paints correctly
     against the real iced layout engine at three sizes.
  3. **Corrected diagnosis (M6.2 fixup).** The M6.2 ship-comment
     here previously blamed `Container::new`'s default width.
     Reading the iced 0.14 source shows that diagnosis was wrong:
     [`Container::new(content)`](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/container.rs#L94-L108)
     inherits width from `content.size_hint()` via
     `Length::fluid()` and preserves `Fill` from `Fill` children.
     The Shrink-default trap actually lives in
     [`Row::new()`](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/row.rs#L80-L81)
     and [`Column::new()`](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/column.rs#L83-L84).
     The explicit `.width(Length::Fill).height(Length::Fill)`
     here is **defensive intent** — it survives future refactors
     that might wrap the chart in a `Row` / `Column` or swap the
     child for a `Shrink`-defaulting widget — and the M6.2 fix
     itself likely worked via a different mechanism (cache
     invalidation from re-typing the container, or simply the
     forced relayout pass that editing this code triggered).  The
     in-source rationale in `crates/ui/src/screens/charts.rs` has
     been rewritten to reflect this corrected mechanic.

  **Files touched (T2032):**
  - [`crates/ui/src/screens/charts.rs`](../../crates/ui/src/screens/charts.rs)
    — two-axis Container fix, new `chart_canvas_height_for_body`
    helper, new `chart_canvas_height_grows_with_body_height` test;
    M6.2 fixup pass rewrote the inline + docstring rationale to
    cite the iced 0.14 source (`container.rs:94-108`,
    `row.rs:80-81`, `column.rs:83-84`) — the original "Container
    defaults to Shrink" story was wrong; the `.width(Length::Fill)`
    is now documented as defensive intent against future
    `Row` / `Column`-wrapping refactors.

- [x] **T2033 [U]** — **Tooltip decouple — read from canvas state
  directly.** Currently `ChartProgram::draw` (chart.rs:308-310)
  requires BOTH `self.tooltip.is_some()` (from `Cockpit.chart_tooltip`,
  filled via the message round-trip) AND `state.hovered_marker_centroid.is_some()`
  (canvas's local state, set synchronously in `update`). Render-vs-
  message timing window: canvas state flips on `CursorMoved`, iced
  redraws once before the published message reaches Cockpit, tooltip
  fails to draw because `self.tooltip` is still None. Then Cockpit
  catches up. Then next `CursorMoved` flips state to None on cursor
  jitter or off-marker move, tooltip clears.

  **Fix:** `ChartProgram::draw` builds the tooltip view from
  `self.markers[idx]` / `self.signals[idx]` directly using
  `state.hovered_marker_idx + state.hovered_marker_centroid`. No
  Cockpit-state round trip. The Cockpit's `chart_tooltip` field stays
  for snapshot tests (the existing `build_tooltip_view` helper drives
  the snapshot-test path; canvas reads marker fields independently
  for the live render).

  **Mandatory screenshot verification:** launch cockpit, manually
  hover over each of the 4 triangles (operator may need to assist
  via remote-control or the ui-designer captures sequential
  screenshots while cursor is over each marker), confirm via
  read-screenshot that the 6-field tooltip stays visible while
  cursor is over the marker hit-rect.

  **[Operator feedback 2026-05-11. Supersedes M6 first pass's
  T2030 partial fix.]**
  _Acceptance:_ hovering a marker in the running cockpit shows the
  6-field tooltip stably (no flash-and-disappear); cursor jitter
  within the 28-px hit-rect does NOT clear the tooltip; cursor
  moves outside the hit-rect clears it cleanly. Screenshot
  evidence cited.

  **Done 2026-05-11 (ui-designer, M6.2):**

  **Refactor.** Pass 6 of `ChartProgram::draw` in
  [`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs)
  was the two-source-AND gate the operator's bug report reduced to.
  Replaced:
  ```rust
  // Pre-T2033: requires `self.tooltip` (round-trip) AND centroid
  if let (Some(view), Some(anchor)) =
      (self.tooltip.as_ref(), state.hovered_marker_centroid) { … }
  ```
  with:
  ```rust
  // Post-T2033: canvas-local state only — no round trip
  if let (Some(idx), Some(anchor)) =
      (state.hovered_marker_idx, state.hovered_marker_centroid) {
      if let Some(view) = self.tooltip_view_from_hover(idx) {
          chart_tooltip::draw_tooltip(&mut frame, bounds, anchor, &view, self.mode);
      }
  }
  ```
  New helper
  [`ChartProgram::tooltip_view_from_hover`](../../crates/ui/src/widgets/chart.rs)
  walks `self.markers[i]` / `self.signals[i]` and reuses the existing
  pure builders `tooltip_view_for_fill` + `tooltip_view_for_signal`
  (same helpers the `Cockpit.chart_tooltip` round-trip used to
  drive, so the field shape stays byte-identical).

  **Vestigial field handling.** `ChartProgram::tooltip` is no
  longer read by `draw`. Kept the field (so `chart::view`'s public
  signature stays unchanged — call sites and snapshot tests
  unaffected) with `#[allow(dead_code)]` + a docstring explaining
  the post-T2033 status. `Cockpit.chart_tooltip` and
  `state::build_tooltip_view` are untouched — they continue to
  drive the snapshot-test path at `chart_tooltip::tests`
  (`chart_tooltip_fill_variant_has_six_fields`,
  `chart_tooltip_ghost_variant_renders_no_price`,
  `chart_tooltip_ghost_clamped_renders_reason`) which all stayed
  green.

  **Unit test.** Added
  [`widgets::chart::tests::chart_tooltip_view_built_from_canvas_state_without_round_trip`](../../crates/ui/src/widgets/chart.rs)
  — the **regression guard** for the decouple. Constructs a
  `ChartProgram` with `tooltip: None` (the exact pre-T2033 bug
  scenario) and asserts:
  (a) `tooltip_view_from_hover(Fill(0))` returns a `Some(view)`
      with `kind == Fill`, `side == Buy`, `price == Some(100)` —
      i.e. the tooltip is buildable from canvas-local state alone.
  (b) `tooltip_view_from_hover(Signal(0))` returns a `Some(view)`
      with `kind == Signal`, `side == Buy`, `price == None` (ghost
      contract per R5.6 — no price field).
  (c) Out-of-range indices (`Fill(99)`, `Signal(99)`) return `None`
      — defence-in-depth against stale indices across the async
      refresh boundary.

  - `cargo test -p ui --lib widgets::chart::tests::chart_tooltip_view_built_from_canvas_state_without_round_trip`
    → green.
  - `cargo test -p ui --lib widgets::chart::` → 6/6 green (existing
    snapshot tests + hit-rect + snap-to-line all stayed green).
  - `cargo test -p ui --lib widgets::chart_tooltip::` → 4/4 green
    (the Cockpit-state-driven snapshot path is unaffected).
  - **No insta snapshot drift.** The chart snapshot tests
    (`chart__btc_with_two_buys_one_sell`, `chart__empty_state_no_data`,
    `chart__with_ghosts_and_fills`) use the `chart_summary` plain-
    text projection which does NOT exercise the tooltip-render
    path — they pin `draw_order: gridlines,labels,line,ghosts,fills,tooltip`
    as a string, not the per-pixel tooltip output. The tooltip-
    widget tests in `chart_tooltip.rs` pin the `build_rows`
    decomposition, which is unchanged.

  **Screenshot verification — PARTIAL CAPTURE + DOCUMENTED
  CURSOR-CONTROL LIMITATION (M6.2 fixup pass, 2026-05-11).**
  Screen Recording permission is now granted; the cockpit at rest
  (no hover) was captured at the default 1280×720 size on the
  Charts screen as
  `/tmp/cockpit-T2033-no-hover.png` (2 624 028 bytes) and shows
  the chart with both Buy (upward green) and Sell (downward red)
  fill-marker triangles rendered along the price line — confirming
  the marker render path (Passes 4–5) is healthy and that the
  hover-trigger surface area exists for the operator to walk
  through manually.

  **Cursor-driven hover capture remains unavailable** in this
  environment because the host process has **Screen Recording**
  permission but **not Accessibility**.  Both cursor-control
  methods the brief lists are blocked:
  - `cliclick` not installed (`which cliclick` → not found); the
    brief explicitly forbids `brew install` in this pass, so this
    path is closed.
  - AppleScript cursor-position via System Events returns
    `Not authorized to send Apple events to System Events.
    (-1743)` — Accessibility permission has not been granted to
    the host process (separate TCC class from Screen Recording).

  Per the brief: "if cursor-control fails: explicitly cite that
  limitation in the T2033 footer, point at the new unit test as
  the load-bearing evidence, and reference the iced 0.14
  source-level race confirmation."

  **Load-bearing evidence (M6.2 + fixup pass):**
  1. **Regression-guard unit test**
     `widgets::chart::tests::chart_tooltip_view_built_from_canvas_state_without_round_trip`
     (added M6.2; still green this pass) — constructs a
     `ChartProgram` with `tooltip: None` (the exact pre-T2033 bug
     scenario), asserts `tooltip_view_from_hover(Fill(0))` /
     `Signal(0)` return `Some(view)` with the correct field shape
     and out-of-range indices return `None`.  Pre-T2033 this
     would have returned `None`; post-T2033 the canvas-local
     state alone is sufficient to build the tooltip view.
  2. **Hover integration test**
     [`crates/ui/tests/chart_tooltip_hover_fires.rs`](../../crates/ui/tests/chart_tooltip_hover_fires.rs)
     (pre-existing) — exercises `ChartProgram::update` with a
     synthesized `mouse::Event::CursorMoved` at a known marker
     position and confirms `state.hovered_marker_idx.is_some()`
     post-update.  This is the synthesized analogue of the
     manual-hover screenshot the brief calls out as the
     fallback-evidence form ("Synthesize via UI test … honor this
     as evidence if cursor-control isn't available").
  3. **Source-level race confirmation (M6.2 fixup pass research,
     2026-05-11).** [`canvas::Program::update`](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/canvas/program.rs#L7-L15)
     runs for ALL events including `RedrawRequested`; canvas-local
     `State` mutates synchronously inside that call, but
     `Application::update` only consumes the published `Message`
     on the next runtime drain pass.  Reading the tooltip view
     from canvas-local state (post-T2033 code path) removes the
     dual-source-of-truth that produced the flash-and-disappear
     race in the pre-T2033 version.  This citation has been
     added inline at `crates/ui/src/widgets/chart.rs` Pass 6.
  4. The existing `chart_tooltip_integration` test continues to
     pass — the Cockpit-state path remains a working code path
     for the snapshot tests, just no longer required for the
     live render.

  Operator-facing manual re-verification (post-merge): hover each
  of the 4 fill triangles in the cockpit.  The 6-field tooltip
  MUST now appear and stay stable while the cursor sits over the
  28-px hit-rect; cursor jitter inside the rect MUST NOT clear it
  (`Program::update`'s idempotent same-idx early-return guards
  that); cursor moves outside the rect MUST clear it cleanly.
  Two-stage screenshot capture (`/tmp/cockpit-T2033-hover.png`
  + `/tmp/cockpit-T2033-hover-2s-later.png` — same tooltip on
  both, proving stability) is filed as a follow-up cursor-control
  task contingent on Accessibility permission being granted to
  the host process; outside the scope of this fixup pass.

  **Files touched (T2033):**
  - [`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs)
    — Pass 6 refactor, new `tooltip_view_from_hover` helper, new
    regression-guard test, vestigial-field docstring +
    `#[allow(dead_code)]` on `ChartProgram::tooltip`; M6.2 fixup
    pass appended an in-source citation of iced 0.14
    `canvas::Program::update` running for ALL events including
    `RedrawRequested` (program.rs:7-15), explaining why reading
    the tooltip view from canvas-local state closes the
    Application-update-drain race.

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
- [x] **T_FINAL_CHART_BUY_SELL_EMPHASIS [tester]** — Tester runs
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
- 2026-05-11 (ui-designer, M6 follow-up): completed **T2028 → T2030**.
  Shared window-chrome module `crates/ui/src/window_icon.rs` lifts the
  Layout-β `min_size` floor (1280×720) + the Lumen-mark
  `iced::window::Icon` into one helper
  (`standard_window_settings`); all three bins call it via
  `.window(ui::window_icon::standard_window_settings())`. Brand
  mark pre-rasterised once locally (out-of-tree `resvg`+`tiny-skia`
  helper, NOT a workspace dep) and committed as 16384 raw RGBA
  bytes at `crates/ui/assets/lumen-mark-64x64.rgba`. T2030 stayed
  on **Q3 option (b)** custom-canvas pointer-tracking — the
  `Program::update` impl was already publishing
  `ChartMarkerHovered.and_capture()` correctly; the gap was a
  missing integration test
  (`crates/ui/tests/chart_tooltip_hover_fires.rs`, 6 tests). Writing
  that test ALSO surfaced a latent UX bug: the pre-T2030 `update`
  `?`-bailed on `cursor.position_in(bounds)` for every event, so a
  cursor swept off the canvas mid-hover never reached the
  `HoverEnded` branch — tooltips latched. Fixed in
  `crates/ui/src/widgets/chart.rs:118-181`; new test
  `cursor_leaving_canvas_while_hovering_publishes_hover_ended` pins
  the regression. `cargo build --workspace --all-targets` green;
  `cargo test --workspace` → 998 passed, 0 failed across 143 test
  binaries; `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS  (11 / 11)`; zero insta-snapshot drift (no
  `*.snap.new` files, no modified `.snap` blobs in git status).
  HANDOFF → tester (T_FINAL_CHART_BUY_SELL_EMPHASIS) — M6 follow-up
  complete.
- 2026-05-11 (orchestrator, operator-relayed via chat): operator's
  second visual-verification pass on commit `9bb5786` (M6 first
  pass) surfaced three further runtime defects: no visible app/dock/
  cmd-tab icon on macOS despite the icon test passing; chart canvas
  crops on window resize instead of scaling; tooltip flashes briefly
  on hover and immediately disappears. Added milestone **M6.2**
  with three tasks **T2031** (document macOS dock-icon limitation +
  open `cockpit-app-bundle` follow-up brief stub), **T2032** (fix
  chart scaling on resize), **T2033** (decouple tooltip render from
  Cockpit-state round trip; supersedes M6 first pass's T2030
  partial fix). Critically: added **mandatory screenshot-
  verification gate** for T2032 + T2033 — ui-designer MUST launch
  cockpit, capture `screencapture -x`, read the screenshot, and
  cite the path in each tick footer. The first two M6 passes
  exposed that tests-pass + runtime-broken is a real failure mode
  in the headless agent sandbox; screenshot-as-second-witness
  closes that gap. T2031 exempt (no rendered UI change). M5 ship
  gate re-enters after M6.2 lands.
- 2026-05-11 (ui-designer, M6.2 third pass): completed
  **T2031 → T2033**.
  - T2031: documented the macOS dock-icon limitation in
    `crates/ui/src/window_icon.rs` as a module-level section
    (`## macOS dock icon limitation (T2031, M6.2)`); created
    `spec/cockpit-app-bundle/feature.md` (status: candidate,
    owner: pending-analyst) matching the
    `spec/v25-kronos-forecast-overlay/feature.md` shape — seven
    open questions for the analyst, promotion checklist, full
    cross-reference set back to this entry. Acceptance grep + ls
    both green. No runtime change.
  - T2032: diagnosed the chart-crop-on-resize bug to the chart-
    body `Container` in `crates/ui/src/screens/charts.rs` having
    only `.height(Length::Fill)` — its default `width(Length::Shrink)`
    collapsed the Fill-width inner canvas. Fix: added explicit
    `.width(Length::Fill)`. Added pure helper
    `chart_canvas_height_for_body(f32) -> f32` mirroring the
    Layout β budget arithmetic + new regression-guard test
    `chart_canvas_height_grows_with_body_height` asserting
    `h_1080 > h_720` with the exact 360-px delta. All 4 charts-
    screen tests green.
  - T2033: refactored `ChartProgram::draw` Pass 6 to build the
    tooltip view directly from `self.markers[idx]` /
    `self.signals[idx]` via the new `tooltip_view_from_hover`
    helper — no longer requires the `Cockpit.chart_tooltip`
    round-trip. Closes the canvas-state-flips-first / Cockpit-
    state-flips-second race the operator's 2026-05-11 report
    reduced to. Kept the vestigial `ChartProgram::tooltip` field
    with `#[allow(dead_code)]` + docstring so the public
    `chart::view` signature + snapshot-test path are unchanged.
    Added regression-guard test
    `chart_tooltip_view_built_from_canvas_state_without_round_trip`
    constructing a `ChartProgram` with `tooltip: None` (the bug
    scenario) and asserting the tooltip is buildable from canvas-
    local state alone. All 6 chart-widget tests + 4 chart-tooltip
    tests green. No insta snapshot drift (`chart_summary`
    projections pin draw-order strings, not pixel-level tooltip
    output).
  - **Mandatory screenshot-verification gate — documented
    limitation.** Three independent `screencapture` invocations
    (`-x`, window-id, bare) all returned `could not create image
    from display`; AppleScript cursor-control returned `Not
    authorized to send Apple events to System Events (-1743)`.
    The macOS Screen Recording + Accessibility TCC privacy gates
    are not granted to the calling shell process and cannot be
    granted from inside the sandbox. Per the brief's documented
    fallback (*"document the limitation in the tick footer ...
    calls out the limitation honestly"*), the screenshot evidence
    is unavailable; the fixes are validated by the new regression-
    guard unit tests (the brief explicitly singles out the T2032
    test name as the acceptance pin) + diagnosis matching iced's
    documented Container/Length default behaviour + the asynchronous-
    update-queue race the brief itself described. Operator must
    re-verify visually post-merge.
  - Aggregate gates green: `cargo test --workspace` → 1000 passed,
    0 failed (one benign `RECONCILIATION FAIL` banner string
    emitted by the `t814_reconciliation_fail_*` tests that assert
    on that exact string — both tests passed); `scripts/verify_anchors.sh`
    → `ANCHORS PASS  (11 / 11)`; `cargo build --workspace
    --all-targets` green. **HANDOFF → tester (T_FINAL_CHART_BUY_SELL_EMPHASIS)**.
