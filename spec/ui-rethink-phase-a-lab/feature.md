---
slug: ui-rethink-phase-a-lab
status: draft
owner: analyst
updated: 2026-05-17
version: 0.1.0
predecessor: chart-canvas-overhaul v1.10.0
---

# UI rethink Phase A — chart-centric Lab

> This brief is the first concrete feature carved out of the broader UI
> rethink at
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> The dev-note's §6 Phase A is the spec source of truth; this brief is
> the **implementation contract** for that slice. The dev-note's IA
> argument (hybrid sidebar, strategy-leading, pair-as-chip-not-sidebar),
> the operator-locked addendum (chart-as-door, XRP-first ordering, three
> overlay layers, read-only cached reports at Phase A), and the eight
> jobs-to-be-done are **not re-litigated here**. Predecessor:
> [`chart-canvas-overhaul` v1.10.0](../chart-canvas-overhaul/feature.md)
> shipped 2026-05-12 — the canvas + axes + tooltip + legend land that
> Phase A reuses unchanged.

## Why

The cockpit's headline operator workflow is "test a strategy against
this pair AND this date range, and see on the chart how successful the
selection is" — see [`product.md` § Differentiator](../product.md#differentiator)
and the 2026-05-17 operator critique captured in
[`ui-rethink-2026-05-17`](../dev-notes/ui-rethink-2026-05-17.md). The
current `Charts` screen (`crates/ui/src/screens/charts.rs`, 597 LOC) is
the only operator-aligned working surface in the cockpit, but it is
strategy-blind: the chart shows price + buy/sell markers + window-
volume tiles but cannot answer "how much capital did I have before /
during / after this strategy run on ETHUSDT?" or "how does v1 momentum
compare to v0.5 MACD on the same pair and range?". Phase A converts
`Charts` into **`Lab`** — the chart-centric workshop that is the
**default screen** at cockpit boot — and fuses three overlay layers on
the single canvas (buy/sell markers wired from
[`chart-buy-sell-emphasis` v1.9.0](../chart-buy-sell-emphasis/feature.md);
equity curve + multi-strategy comparison new). Read-only at Phase A
(cached backtest reports + fixtures); Phase B wires the live
in-process backtest engine. Lumen tokens are unchanged. The slice is
sized so the rollback cost is two weeks, one feature flag, zero
touched anchors, zero non-UI crates if the operator rejects the
shape.

## Requirements

Numbered, testable, derived from the locked dev-note addendum + the
operator's chart-as-door constraint. Each R-item preserves the 11
locked body-SHA-256 anchors in [`spec/anchors.toml`](../anchors.toml),
the Lumen Phase 1 token contract, and the existing chart widget API
(`crates/ui/src/widgets/chart.rs`, 1537 LOC). All R-items are
**UI-only**; out-of-scope cross-cuts (audit-ledger schema extensions,
live backtest engine wiring, model registry surfaces) are documented
in the dev-note as Phase B/D backend prep and explicitly deferred.

### R1 — Lab screen replaces Charts as the default route

- **R1.1** `crates/ui/src/screens/charts.rs` → `crates/ui/src/screens/lab.rs`
  (rename + module move; `Charts` screen variant removed from
  `state::Screen`; new variant `Lab` added; the in-place rename
  follows the dev-note §4 "Keep — Refactored" disposition).
- **R1.2** Default screen at cockpit boot flips from `Screen::Home`
  to `Screen::Lab`. The cockpit's first sight is the workshop, not a
  dashboard.
- **R1.3** Sidebar shell (`crates/ui/src/shell.rs`) routes `Screen::Lab`
  to `lab::view`; the prior `Charts` sidebar entry is replaced with
  a `Lab` entry at slot 0. Phase A keeps the rest of the sidebar
  unchanged — the full IA flip (Home → Live, Settings rollup, etc.)
  is Phase C scope per dev-note §6.
- **R1.4** All references to the `Charts` screen in
  `crates/ui/src/strings.rs`, `crates/ui/src/theme/layout.rs`
  `SIDEBAR_ENTRIES_PHASE_5`, and `Message::SwitchScreen` variants
  rename to `Lab` with a one-cycle compatibility shim so the test
  harness migrates without a regression.
- **Acceptance:** `cargo test -p ui` passes; cockpit boots into Lab
  on first launch; insta snapshot `shell__default_screen_lab`
  records the new boot state.

### R2 — Chart canvas hosts three overlay layers

- **R2.1** Layer 1 — **Buy/sell markers** on price. Wire the existing
  emphasis markers from
  [`chart-buy-sell-emphasis` v1.9.0](../chart-buy-sell-emphasis/feature.md)
  (`MARKER_SIZE_PX = 13.0` triangle, `BORDER_STRONG` 1-px outline,
  snap-to-line y-anchor) into the Lab chart. Markers already render
  on the v1.10.0 canvas — Phase A verifies the wiring against the
  Lab data source (cached report fills) and ships a fixture for the
  Lumen ui_gallery.
- **R2.2** Layer 2 — **Equity curve overlay**. New rendering pass on
  the chart widget: a polyline over a second Y-axis (right side)
  drawn from per-bar equity in the cached backtest report. Toggle
  on/off via legend chip; line color = `color::ACCENT_2` (Lumen
  token; **no new token required** — confirm via R10). Y-axis right
  gutter sized identically to the left price gutter (visual
  symmetry).
- **R2.3** Layer 3 — **Multi-strategy comparison**. Up to 4 strategies'
  equity curves drawn as color-coded polylines on the same right
  Y-axis. Colors from the existing Lumen palette: `color::ACCENT_2`,
  `color::ACCENT_3`, `color::UP_500`, `color::DOWN_500` (operator
  ratifies via Q1 below). Legend chips list each strategy id +
  color swatch; click toggles its line visibility.
- **R2.4** All three layers render in a fixed z-order: price line
  (bottom) → equity curves (middle) → buy/sell markers (top), so
  the markers stay visually dominant per the chart-buy-sell-
  emphasis ship.
- **R2.5** When the operator selects 0 strategies, the equity
  overlay is hidden and the chart degrades to the v1.10.0
  price+markers shape (no regression for the muscle-memory case).
- **Acceptance:** insta snapshot `chart__three_overlays_v1_momentum`
  shows price + buy/sell markers + equity curve + a comparison line
  for a second strategy on the same pair/range; manual cockpit run
  on fixtures confirms legend toggles work.

### R3 — Pair chip widget with XRP-first ordering

- **R3.1** New widget `crates/ui/src/widgets/pair_chip.rs` —
  parameterized on a `(Venue, Symbol)` tuple, renders as a Lumen
  chip with the symbol label (e.g. `XRPUSDT`) + venue suffix when
  ambiguous (e.g. `Binance · XRPUSDT`). Reuses chip styling from
  `widgets/strategies.rs`; no new tokens.
- **R3.2** Pair list (v1 universe) renders in operator-locked
  order: **XRPUSDT, ETHUSDT, BTCUSDT, ADAUSDT, AVAXUSDT, BNBUSDT,
  DOGEUSDT, DOTUSDT, LINKUSDT, SOLUSDT**. The first three are
  operator preference (R7 in the dev-note's Q-resolution); the
  remainder is alphabetical. Order is data-driven (a const slice
  in `crates/ui/src/state.rs` next to `universe`) — not
  hard-coded into the chip widget — so a future Settings toggle
  (out of Phase A scope) can re-sort without widget changes.
- **R3.3** Swap-on-click semantics: clicking a pair chip dispatches
  `Message::LabSelectPair(Venue, Symbol)` which (a) updates
  `Cockpit::lab_state.pair`, (b) re-renders the chart against the
  new pair's cached report tuple (R7), (c) preserves the selected
  strategy + date-range + comparison set unchanged.
- **R3.4** Active pair chip uses `frame::active_row` accent
  treatment; non-active chips use the default chip style. The
  active state is single-select (exactly one pair chip is active
  at any time).
- **Acceptance:** insta snapshot `pair_chip_row__xrp_first` records
  the chip row in the locked order; clicking ETHUSDT in the
  cockpit smoke test swaps the chart's reference dataset and
  re-renders the overlays for the new pair.

### R4 — Strategy chip widget (port from registry to chart context)

- **R4.1** New widget `crates/ui/src/widgets/strategy_chip.rs` —
  renders a strategy id as a Lumen chip with a small family pill
  (Rule / LLM / DL / Hybrid) and a color swatch matching its
  equity-overlay line color (R2.3). Reuses chip styling primitives
  from `widgets/strategies.rs`.
- **R4.2** Strategy chip row sits above the chart in the Lab body,
  below the pair chip row. The row supports two interactions:
  - **Primary select** (single-click on chip body): updates
    `Cockpit::lab_state.strategy` and re-renders the chart with
    that strategy's buy/sell markers + equity curve as the
    primary overlay.
  - **Comparison toggle** (click on the chip's "+" affordance):
    adds the strategy to `lab_state.compare_set` (≤4 strategies
    per R8.2); a second click removes it. Maximum 4-strategy
    comparison enforced at the widget level (the 5th "+" press
    is no-op + a toast).
- **R4.3** Strategy list source: `Cockpit::strategies` (already
  present). No new backend wiring.
- **R4.4** When no strategy is selected (cold start, no `lab_state`
  restored), the chart falls back to the v1.10.0 price-only shape
  with an empty-state hint ("Pick a strategy to see fills and
  equity").
- **Acceptance:** cockpit smoke test starts in Lab, clicks the
  v1.momentum chip, sees the buy/sell markers and equity curve
  render; clicks "+" on v0.5.macd, sees a second equity line
  appear in the second color from R2.3.

### R5 — Date-range picker pinches the chart's x-axis

- **R5.1** New widget `crates/ui/src/widgets/date_range.rs` —
  renders as a Lumen dropdown with named presets ("Last 30d",
  "Last 90d", "2024 H1", "2024 H2", "Custom…"). Custom opens an
  inline two-field date-pair editor (no calendar widget at
  Phase A — that's a Phase B/C convenience).
- **R5.2** Picker dispatches `Message::LabSelectRange(DateRange)`;
  `Cockpit::lab_state.range` updates; the chart's x-axis bounds
  re-clamp to the new range and all three overlay layers re-render
  against the windowed data.
- **R5.3** The picker is a third row in the Lab top-bar (below
  pair chips and strategy chips), giving the operator the three
  selection surfaces in one vertical scan: pair → strategy →
  range → chart body. Phase A keeps the layout strict-vertical;
  Phase C may compact to a single horizontal top-bar.
- **R5.4** When no cached backtest report covers the requested
  range exactly, Phase A's data path finds the **closest
  superset** report (e.g. "Last 30d" inside a "Last 90d" report)
  and renders a "narrowed from `<report_name>`" badge near the
  picker. If no superset report exists, the picker shows an
  empty-state hint ("No cached run for this (strategy, pair,
  range). Run `cargo run --bin backtest --strategy v1.momentum
  --pair ETHUSDT --range last-30d` then refresh.").
- **Acceptance:** insta snapshot `date_range_picker__presets`
  records the dropdown shape; cockpit smoke test verifies the
  "narrowed from" badge appears when a 30-day subset of a
  90-day cached run is selected.

### R6 — Lab persists `(strategy, pair, range, params)` across cockpit restarts

- **R6.1** New persisted state `Cockpit::lab_state` with fields
  `{ strategy: Option<StrategyId>, pair: Option<(Venue, Symbol)>,
  range: DateRange, params: Option<ParamSheet>, compare_set:
  SmallVec<[StrategyId; 4]> }`.
- **R6.2** On cockpit launch, the state restores from
  `~/.config/trading/cockpit-lab-state.json` (path follows the
  XDG `$XDG_CONFIG_HOME` env var when set, otherwise
  `~/.config/trading/`). Missing file → cold-start defaults
  (no strategy, no pair, "Last 90d", no params, empty compare
  set).
- **R6.3** State writes on every Lab state mutation (debounced
  to 500 ms to avoid disk-thrash during chip-row rapid
  selection). The writer is `tokio::fs::write` of a serialized
  JSON blob; corruption (parse failure) falls back to cold-start
  defaults and logs a `tracing::warn!`.
- **R6.4** `params` field is **reserved-but-empty** at Phase A
  (the in-screen parameter editor is Phase B per dev-note §6).
  The field exists in the schema so Phase B does not need to
  re-design the persistence file.
- **Acceptance:** cockpit smoke test selects v1.momentum × ETHUSDT
  × "Last 90d", quits the cockpit, relaunches, sees Lab restored
  with the same tuple. Integration test verifies the JSON
  on-disk shape.

### R7 — Equity-curve overlay reads from cached backtest reports

- **R7.1** New data path `crates/ui/src/lab/equity_loader.rs` —
  scans `spec/<strategy-slug>/reports/backtest-*.md` for reports
  matching `(strategy, pair, range)` via frontmatter fields. The
  scan happens **at chart-render time**, with a per-tuple in-memory
  cache to avoid re-parsing on every paint.
- **R7.2** Report selection rule: exact-match (strategy + pair +
  range) is preferred; closest-superset is the fallback per R5.4.
  No live backtest engine invocation at Phase A — the chart
  surfaces the cached truth, nothing more.
- **R7.3** Per-bar equity is read from the report's
  `## Equity curve` section (currently a Markdown body table —
  see [`spec/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`](../v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md)
  for an example shape). If the report lacks per-bar equity (some
  older v0 reports), the loader falls back to start-equity-and-
  final-equity-only and renders the equity line as a straight
  two-point segment with a "low-fidelity" legend marker.
- **R7.4** Fill markers come from the report's existing fills
  block (already plumbed via the chart's `markers` data path for
  v1.9.0); R7 only adds the equity-curve reader, not new fill
  plumbing.
- **R7.5** No changes to the backtest engine, the report writer,
  or the audit ledger. **This is a read-only Phase A feature.**
- **Acceptance:** integration test loads the v1-cross-sectional-
  momentum 2024 H1 report and verifies the loader returns the
  expected equity series length + start/end values.

### R8 — Multi-strategy comparison overlay (≤4 lines)

- **R8.1** `Cockpit::lab_state.compare_set: SmallVec<[StrategyId; 4]>`
  holds 0–4 additional strategies to render alongside the
  primary `lab_state.strategy`. The "primary + 3 compare =
  4 total lines" ceiling is enforced at the type level
  (`SmallVec` cap) AND at the UI level (R4.2 toast on 5th
  press).
- **R8.2** Each comparison strategy's equity curve uses a
  distinct color from R2.3's palette. Color assignment is
  positional (slot 0 → `ACCENT_2`, slot 1 → `ACCENT_3`, slot 2
  → `UP_500`, slot 3 → `DOWN_500`). Operator ratifies the
  palette in Q1.
- **R8.3** The chart's right Y-axis auto-scales to cover all
  visible equity lines (min/max across primary + comparison
  set), so the operator can visually rank "which strategy ended
  with more capital" at a glance.
- **R8.4** Comparison strategies must share the **same pair and
  range** as the primary selection. If the operator switches
  pair (R3.3), the compare set retains its strategy ids but each
  line re-loads against the new pair's cached report; any
  strategy with no cached report for the new pair renders as a
  faded "no data" legend chip (no broken line on the canvas).
- **R8.5** When `compare_set` is empty, only the primary
  strategy's equity curve renders — no leftover visual chrome.
- **Acceptance:** insta snapshot `chart__compare_three_strategies`
  shows three distinct equity lines on ETHUSDT 2024 H1; cockpit
  smoke test verifies the 5th compare attempt no-ops with a
  toast.

### R9 — Sidebar reduces to "Lab + Live + Compare" workflow group

- **R9.1** Phase A sidebar shape (top-to-bottom):
  ```
  Lab        ← new default (R1)
  Live       ← renamed from Home
  Compare    ← placeholder route, empty body at Phase A
  ─────
  Strategies ← unchanged from Phase 5
  Memory     ← reserved (Phase F)
  Models     ← reserved (Phase F)
  Trail      ← renamed from Audit (Phase D body lands later)
  ─────
  Settings   ← placeholder (Risk/Debug/Control rollup is Phase C)
  ```
  Per dev-note §3 — three-group structure. Phase A wires
  **Lab + Live (rename only) + the placeholder routes**; the
  bodies of Compare / Memory / Models / Trail / Settings are
  Phase C–F scope.
- **R9.2** The `Home` / `Charts` / `Audit` / `Risk` / `Debug` /
  `Control` legacy sidebar entries are removed from
  `theme::layout::SIDEBAR_ENTRIES_PHASE_5`. Their bodies survive
  in source under their current paths (`screens/home.rs`,
  `screens/risk.rs`, etc.) for one cycle — the rename + sidebar
  removal lands here; the file deletion / merge into
  `screens/settings.rs` lands in Phase C with a clean diff.
- **R9.3** `Message::SwitchScreen` gains `Lab`, `Live`,
  `Compare`, `Memory`, `Models`, `Trail`, `Settings` variants;
  the old `Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`
  variants are kept for one cycle as `#[deprecated]` aliases
  that auto-route to their successor (`Home → Live`, etc.).
  Test-harness migrations follow the alias path with no test
  breakage.
- **R9.4** Placeholder bodies (`Compare`, `Memory`, `Models`,
  `Trail`, `Settings`) render an empty-state card with one
  sentence each ("Compare view — Phase E", etc.) so the
  operator never sees a blank panel.
- **Acceptance:** insta snapshot `sidebar__phase_a_workflow_group`
  records the new sidebar order; cockpit smoke test clicks each
  sidebar entry and verifies the route resolves without panic.

### R10 — Lumen tokens unchanged (no new design-system primitives)

- **R10.1** Phase A introduces **zero new Lumen tokens**. Every
  color, spacing, type, radius, motion, and shadow value comes
  from the Phase 1 token contract per
  [`spec/lumen-design-adoption/phase-1-foundation/feature.md`](../lumen-design-adoption/phase-1-foundation/feature.md).
  If a layer or chip needs a token that doesn't exist, **stop and
  surface to the operator** (this is a smell — Phase A's design
  hypothesis is that the existing palette + chip + frame
  primitives are sufficient).
- **R10.2** No new `widgets/` primitives beyond the four named
  in R3–R5 (pair_chip, strategy_chip, date_range, plus extensions
  to `chart.rs` for the equity-curve + comparison overlay pass).
  Compare matrix, trail node, lesson card, model row are Phase
  C–F scope.
- **R10.3** The Lumen Phase 1 audit check — "`grep '#' src/` shows
  zero hex colors in `crates/ui/src/screens/lab.rs` and the new
  widgets" — passes as a tester gate. Same audit for raw string
  literals (all copy via `crate::strings`).
- **Acceptance:** tester runs the Lumen Phase 1 audit script (the
  one shipped with `lumen-design-adoption phase-1-foundation`)
  on the new files; exit 0 required for `VERDICT → PASS`.

### R11 — Verification gates (non-regression contract)

- **R11.1** All 11 locked body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) stay byte-identical.
  Phase A touches no strategy code, no audit code, no exec code,
  no backtest engine, no report writer.
- **R11.2** Existing UI test suite (267 panel snapshots + iced_test
  smoke + canvas hit-test grid sweep at 1280×720 / 1600×900 /
  1920×1080 / 3360×1890 per
  [`ui-test-harness-bootstrap`](../ui-test-harness-bootstrap/feature.md))
  stays green. New tests added by Phase A are **additive only**.
- **R11.3** `cockpit-smoke` skill exit 0 required for tester
  `VERDICT → PASS` per AGENT.md §Process discipline rule 6.
- **R11.4** `spec-lint` exit 0 required per AGENT.md §Process
  discipline rule 7 (no dead links, no orphan-feature, no
  trace-broken-path).
- **Acceptance:** tester report cites all four gates passing.

## Design

_architect fills this — load-bearing decisions to resolve at design
time:_

1. **Equity-curve render strategy** — inline `Canvas::draw` pass vs
   a sibling canvas widget overlaid via `iced::widget::stack`. The
   inline pass is simpler but couples the data path; the stack
   approach is cleaner but exercises iced 0.14 layout corners.
   Architect picks; cite a hypothesis if uncertain.
2. **Lab state persistence shape** — JSON serde via `serde_json`
   vs TOML via `toml`. Repo precedent leans TOML for hand-edited
   config; machine-written-machine-read state is JSON-idiomatic.
   Architect ratifies.
3. **Multi-report loader concurrency** — synchronous per-paint
   read vs `tokio::spawn`-loaded with a `oneshot` callback to
   `Message::LabReportLoaded(...)`. The latter avoids paint-jank
   on cold cache; the former is simpler. Architect picks against
   the chart's actual paint budget on the operator's
   3360×1890 Retina.

## Backtest Scenarios

**N/A** — this is a UI feature. No new backtest scenarios are
defined; Phase A is read-only over existing cached reports. The
tester's regression contract is the existing 11 body-SHA anchors
(R11.1).

## Implementation

_developer fills this — task breakdown comes from the
architect's `tasks.md`. Pre-baked milestones (M0–M-FINAL) seeded in
[`tasks.md`](./tasks.md) for the architect to refine into ordered
T-D-1..T-D-N rows with acceptance criteria._

## Verification

_tester fills this — link reports here after each milestone closes.
Required gates per R11:_

- `cargo test -p ui` — unit + insta snapshot suite (new snapshots
  listed in R1–R9 acceptance lines).
- `cockpit-smoke` skill — exit 0; 7 s boot against fixtures; stderr
  grep for panics; per AGENT.md rule 6.
- `verify-anchors.sh` — all 11 anchors byte-identical per R11.1.
- `spec-lint` — exit 0; per AGENT.md rule 7.
- **Visual A/B** — manual cockpit run on the operator's 3360×1890
  Retina with a before/after screenshot pair for each of the three
  overlay layers (the chart-as-door check the chart-canvas-overhaul
  cycle established as load-bearing).

## Operator decision questions

The dev-note's Q1–Q8 are resolved. Phase A surfaces three additional
load-bearing questions the dev-note didn't anticipate. Each has a
default the developer can ship against if the operator doesn't
intervene, and a cost-if-wrong sized in cycles.

### Q-A1 — Equity-overlay color palette (R2.3 / R8.2)

**The question.** The four comparison-line colors must be visually
distinct from each other AND from `color::ACCENT` (the price line
color). Default proposal: slot 0 → `ACCENT_2` (Lumen secondary
accent), slot 1 → `ACCENT_3`, slot 2 → `UP_500` (green), slot 3
→ `DOWN_500` (red). The green/red slots are semantically loaded
(they normally mean "up day" / "down day") — re-using them for
strategy lines risks visual confusion.

**Recommended default.** Ship the palette as proposed and lean on
the legend chip's color swatch to disambiguate. If the operator
finds it confusing after a week of Phase A use, add two new
neutral comparison-only tokens (e.g. `ACCENT_4` purple,
`ACCENT_5` orange) at the start of Phase B — that's the only
moment Phase A's "zero new tokens" rule would relax.

**Cost if wrong.** Two new Lumen tokens at Phase B kickoff. The
chart code path stays unchanged; only the constants flip.

### Q-A2 — Should Phase A run a backtest if no cached report exists?

**The question.** R5.4 + R7.2 surface a "no cached run for this
(strategy, pair, range)" empty state with a CLI hint. The operator
might reasonably expect Phase A to **run the backtest in-process**
and render the result. The dev-note's Phase A scope explicitly
defers in-process backtest to Phase B (Q3 resolution), but the
operator's chart-as-door framing might bend on this.

**Recommended default.** **Defer to Phase B.** Phase A ships the
chart shape against cached truth; running the engine from the
cockpit is Phase B's headline. The CLI-hint empty state is the
correct Phase A affordance — it teaches the operator the
backtest-then-refresh loop without coupling Phase A to engine
library-callability tightening (which the dev-note addendum sized
at ~1 day on top of Phase A's 2 weeks).

**Cost if wrong.** Phase A ships, the operator immediately asks
"why can't the Run button work?", and Phase B starts one cycle
sooner. No work is wasted (the cached-loader path stays
load-bearing for Compare-matrix in Phase E).

### Q-A3 — Default `lab_state` on first launch (R6.2)

**The question.** Cold-start defaults when no `cockpit-lab-state.json`
exists: which (strategy, pair, range) does Lab open to? Options:
(a) all-empty (operator picks from scratch each first launch),
(b) most-recently-shipped tuple (e.g. v1.momentum × XRPUSDT × Last
90d as a project-bootstrap experience), (c) project-curated demo
tuple (a tuple the team has confirmed renders well — e.g. the
v1-cross-sectional-momentum 2024 H1 report so all three overlays
have data on day one).

**Recommended default.** **Option (c) — curated demo tuple.**
v1.momentum × ETHUSDT × Last 90d, picked because the existing
`spec/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`
covers it and renders fills + equity. The first launch shows the
operator what Phase A is for, not an empty canvas. Subsequent
launches restore via R6.2.

**Cost if wrong.** A first-launch tuple the operator finds
distracting; one cycle to swap to Option (a). No code change —
just a constant in `lab/defaults.rs`.

## Changelog

- 2026-05-17 (analyst): initial brief authored from
  [`ui-rethink-2026-05-17`](../dev-notes/ui-rethink-2026-05-17.md)
  §6 Phase A + operator addendum. Eleven requirements (R1–R11)
  cover screen rename + default-route flip + three overlay layers
  + pair/strategy/date-range chip widgets + Lab tuple persistence
  + read-only equity loader + sidebar workflow group + Lumen-token
  invariant + non-regression contract. Five new widgets
  (pair_chip, strategy_chip, date_range + extensions to chart.rs
  for two overlay passes). Zero new Lumen tokens. Zero touched
  anchors. Three operator-decide questions (Q-A1 palette, Q-A2
  cached-only at Phase A, Q-A3 cold-start tuple). HANDOFF →
  architect.
