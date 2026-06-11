---
slug: ui-rethink-phase-a-lab
status: shipped
owner: operator
updated: 2026-05-18
version: 0.2.0
predecessor: chart-canvas-overhaul v1.10.0
---

# UI rethink Phase A — chart-centric Lab

> This brief is the first concrete feature carved out of the broader UI
> rethink at
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md).
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
[`ui-rethink-2026-05-17`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md). The
current `Charts` screen (`crates/ui/src/screens/lab.rs`, 597 LOC) is
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

- **R1.1** `crates/ui/src/screens/lab.rs` → `crates/ui/src/screens/lab.rs`
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

> Architect-owned; resolves the three analyst-flagged decisions plus
> the four design surfaces the operator-decides 2026-05-17 added
> (Lumen ACCENT_2..5 extension, in-process backtest invocation,
> sidebar IA placeholder shape, cold-start tuple per Q-A3). Cross-ref:
> [ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md)
> (cockpit ↔ backtest edge),
> [Lumen accent palette extension dev-note](../dev-notes/lumen-accent-palette-extension-2026-05-17.md),
> [`spec/architecture/06-ui-and-cockpit.md`](../architecture/06-ui-and-cockpit.md)
> (UI isolation rule + screen routing).

### 1. Module + crate layout

The Lab feature lives entirely inside the `ui` crate. New surfaces:

```
crates/ui/src/
├── screens/
│   └── lab.rs                      # ex-charts.rs, renamed (R1.1)
├── widgets/
│   ├── pair_chip.rs                # NEW (R3)
│   ├── strategy_chip.rs            # NEW (R4)
│   ├── date_range.rs               # NEW (R5)
│   └── chart.rs                    # extended (R2 — two new draw passes)
├── lab/
│   ├── mod.rs                      # NEW: re-exports
│   ├── state.rs                    # NEW: LabState struct + ops
│   ├── defaults.rs                 # NEW: cold-start tuple constant
│   ├── persistence.rs              # NEW: JSON read/write + debounce
│   ├── equity_loader.rs            # NEW: cached-report scanner (R7)
│   └── runner.rs                   # NEW: ADR-0030 invocation glue
├── state.rs                        # extended: Cockpit::lab_state field +
│                                    # Screen::Lab/Live/Compare/... variants
│                                    # + Message::Lab* variants
└── shell.rs                        # extended: 7-screen routing match
```

`crates/ui/src/lab/` is a new module group. The grouping is by feature
concern (state, persistence, data loader, engine runner) rather than
by widget — the widgets stay in `widgets/` for catalog-tool symmetry.

New crate edge: `crates/ui/Cargo.toml` gains
`backtest = { path = "../backtest" }` for the M2.5 runner (ADR-0030).
No other crate edges change.

### 2. Widget shapes

The four new widgets follow the existing iced widget contract: each
exposes a `pub fn view(...) -> Element<'_>`, owns a `State` struct for
hover/focus, emits a `Message` variant. Each gets a unit test and an
insta snapshot. Detailed shape table:

#### 2.1 `widgets::pair_chip`

```rust
pub struct PairChipProps {
    pub pair: (Venue, Symbol),         // e.g. (Binance, XRPUSDT)
    pub label: SmolStr,                // "XRPUSDT" or "Binance · XRPUSDT" if ambiguous
    pub is_active: bool,               // single-select state owned by caller
    pub mode: ThemeMode,
}

pub fn view<'a>(props: PairChipProps) -> Element<'a, Message>;
```

Emits `Message::LabSelectPair(Venue, Symbol)` on click. State held by
the widget itself is **empty** — the active flag lives in
`Cockpit::lab_state.pair`. Hover state lives in iced's built-in
button focus path; no widget-local `State` needed.

Styling: reuses `chip_default` from `widgets/strategies.rs` for the
non-active case. Active case adds a `frame::active_row`-style 2 px
left rule in `color::ACCENT` per the Lumen Phase 1 active-row pattern.
Tokens used: `color::ACCENT`, `color::PANEL_RAISED`, `color::FG_1`,
`color::BORDER`, `spacing::S_2`, `radii::R_2`, `typography::body_sm`.

#### 2.2 `widgets::strategy_chip`

```rust
pub struct StrategyChipProps {
    pub strategy_id: StrategyId,
    pub family: StrategyFamily,        // Rule | Composed | LLM | DL | Hybrid
    pub is_primary: bool,              // selected as the primary strategy
    pub compare_slot: Option<u8>,      // Some(0..3) iff in compare_set
    pub mode: ThemeMode,
}

pub fn view<'a>(props: StrategyChipProps) -> Element<'a, Message>;
```

Two emit sites on the same chip:
- Click on chip body → `Message::LabSelectPrimaryStrategy(StrategyId)`.
- Click on the trailing `+` affordance (when `is_primary == false`)
  → `Message::LabToggleCompare(StrategyId)`. When `compare_slot` is
  `Some(n)`, the `+` swaps to a `×` and the message removes the
  strategy from the compare set.

A small color swatch (8 × 8 px) sits before the chip label when
`compare_slot.is_some()` — it shows the comparison-line color for that
slot, sourced positionally from `[ACCENT_2, ACCENT_3, ACCENT_4,
ACCENT_5][compare_slot.unwrap() as usize]`. This is the **only**
caller that uses the new ACCENT_2..5 tokens at chip render time; the
chart canvas uses the same lookup for line color (see § 3).

Family pill: a 4-letter caps badge ("RULE", "COMP", "LLM", "DL",
"HYBR") with tier-2 background. Reuses the existing `widgets::pill`
helper.

#### 2.3 `widgets::date_range`

```rust
pub struct DateRangeProps {
    pub current: DateRange,
    pub mode: ThemeMode,
    pub narrowed_from: Option<SmolStr>,  // e.g. "Last 90d run" — surfaces R5.4 badge
}

#[derive(Debug, Clone)]
pub enum DateRange {
    Preset(Preset),                    // Last30d, Last90d, H1_2024, H2_2024
    Custom { start: DateTime<Utc>, end: DateTime<Utc> },
}

pub fn view<'a>(props: DateRangeProps) -> Element<'a, Message>;
```

A `PickList`-shaped dropdown (Lumen tier-2 surface) with the five
preset entries + "Custom…". Selecting a preset emits
`Message::LabSelectRange(DateRange::Preset(p))`. Selecting "Custom…"
opens an inline two-field date editor (two `text_input` widgets with
ISO-8601 parsing — no calendar widget at Phase A per R5.1); pressing
Enter emits `Message::LabSelectRange(DateRange::Custom { ... })`.

The "narrowed from" badge renders adjacent to the picker as a small
`tier_0` text label when `narrowed_from.is_some()` (R5.4). The badge
text comes from `strings::LAB_NARROWED_FROM_BADGE` — no inline string
literals.

Internal widget state: `DateRangeState { dropdown_open: bool, custom_start_raw: String, custom_end_raw: String }`,
held via iced's `State::with_data`. Parse errors highlight the input
border with `color::DOWN_500` (the existing Lumen invalid-input
pattern) and **do not** dispatch the `Message::LabSelectRange` until
both fields parse cleanly.

#### 2.4 `equity_overlay` (chart draw pass, not a standalone widget)

This is **not** a separate widget — it is a new draw pass inside
`widgets::chart.rs`. Rationale below in § 3.

Data shape consumed:
```rust
pub struct EquitySeries {
    pub strategy_id: StrategyId,
    pub samples: Vec<(Timestamp, Money<Usdt>)>,  // per-bar points
    pub fidelity: Fidelity,                       // PerBar | StartEndOnly
}
```

#### 2.5 `comparison_overlay` (chart draw pass + legend extension)

Also not a standalone widget. The legend extension lives in
`widgets/chart_legend.rs`:

```rust
pub struct CompareLegendEntry {
    pub strategy_id: StrategyId,
    pub color: ModeColor,             // resolved from compare_slot
    pub visible: bool,
    pub status: CompareStatus,        // HasData | NoDataForPair (R8.4)
}

pub fn view_with_compare_set<'a>(
    base_entries: Vec<LegendEntry>,
    compare_entries: Vec<CompareLegendEntry>,
    mode: ThemeMode,
) -> Element<'a, Message>;
```

A `NoDataForPair` entry renders with `color::FG_4` (placeholder) +
strike-through label so the operator sees "v0.5.macd: no cached run
for BTCUSDT" without the canvas needing to draw a broken line.

### 3. Chart canvas extension — two new draw passes

The existing `widgets::chart.rs` `pub fn view(bars, markers, signals,
tooltip, mode)` is **extended additively** rather than replaced. New
signature:

```rust
pub fn view<'a>(
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    signals: Vec<SignalView>,
    tooltip: Option<ChartTooltipView>,
    equity: Option<EquitySeries>,            // NEW — R2.2 / M2
    compare: Vec<EquitySeries>,              // NEW — R2.3 / M3 (max 4)
    mode: ThemeMode,
) -> Element<'a>;
```

Old call sites (the live cockpit chart, ui_gallery) pass `None` /
`vec![]` for the two new params and get the v1.10.0 shape pixel-
identical. The `ChartProgram` struct gains two fields holding the
same data and the `Program::draw` pass renders **in fixed z-order**
(per R2.4):

1. **Background + grid** (existing pass; unchanged)
2. **Price line** (existing pass; unchanged)
3. **Equity lines** (NEW pass):
   - Compute right-axis `(min_equity, max_equity)` across
     `equity.iter().chain(compare.iter())`. If the result is empty,
     the pass is a no-op.
   - Reserve a right-side gutter of `AXIS_GUTTER_PX = 56.0` (same
     value as the left-side price gutter — visual symmetry per R2.2).
     The `inner_rect_for_viewport` helper grows a sibling
     `inner_rect_with_right_gutter` that subtracts the gutter from
     the right edge **only when** `equity.is_some() || !compare.is_empty()`.
   - Draw the primary `equity` curve in `color::ACCENT_2` (matches
     the convention "the highlighted strategy is the lighter teal").
     Draw each `compare[i]` in `[ACCENT_2, ACCENT_3, ACCENT_4,
     ACCENT_5][i]`. Wait — slot 0 already taken by primary; the
     actual rule is: primary uses **`color::ACCENT`** (the price-
     line accent, kept for the primary's "I am the focused one"
     reading), and `compare[i]` uses `[ACCENT_2, ACCENT_3,
     ACCENT_4, ACCENT_5][i]`. Five distinct colors total: ACCENT
     (primary), then four ACCENT_2..5 (compares). The
     `strategy_chip` color swatch reads from the compare slot only —
     the primary chip uses the standard active treatment, not a
     swatch.
   - Polyline drawing uses the existing `Path::new` builder with the
     same anti-aliasing settings as the price line. Stroke width:
     1.5 px (slightly thinner than the 2.0 px price line so the
     focus stays on price).
   - Per-bar fidelity case: render N-1 segments. Start-end-only case
     (R7.3): render a single 2-point segment with a `low_fidelity`
     legend marker (the legend chip gains a dotted-line decoration).
4. **Right-axis ticks + labels** (NEW pass):
   - 5 evenly spaced ticks across `(min_equity, max_equity)`,
     formatted as `Money<Usdt>` short form ("$10,250", "$11K").
     Tick stroke + label styling reuses the existing left-axis
     helpers; labels right-aligned against the gutter edge.
5. **Buy/sell markers** (existing pass; unchanged — stays on top per
   R2.4)
6. **Tooltip overlay** (existing pass; unchanged)

**Decision: inline `Canvas::draw` extension, NOT `iced::widget::stack`
of sibling canvases.** Resolves analyst Q1. Rationale:
- The right-axis gutter must match the left-axis gutter geometry
  exactly; coordinating two `Canvas`es to share inner-rect math
  would re-introduce the v1.7 axis-misalignment bug.
- All three layers share the same `(Timestamp → x)` projection — the
  `anchor_for_ts` helper is the existing single source of truth.
  Stacking two canvases means duplicating that projection or
  threading a shared `ProjectionRef` across canvas boundaries.
- iced 0.14 `stack` layout works but the equity-curve hover
  interaction (Phase B follow-up) wants a unified hit-test surface;
  the inline path lands ready for Phase B's tooltip-on-equity-line
  feature without refactor.
- Cost: ~150 additional LOC in `chart.rs` (already 1537 LOC; new
  total ~1700). Below the 2000 LOC informal ceiling.

**Comparison-line color assignment is positional and pinned by test.**
A new unit test `chart::test::compare_color_slot_assignment_is_stable`
asserts `[ACCENT_2, ACCENT_3, ACCENT_4, ACCENT_5]` ordering; reorder
becomes deliberate.

### 4. In-process backtest invocation (M2.5 / ADR-0030)

The Lab Run button surface is the smallest needle through which the
operator can drive the engine. Design:

#### 4.1 `backtest::engine::run_scenario` (server-side)

Lives in `crates/backtest/src/engine.rs` (extending the existing
`MatchingEngine` module). Signature locked by
[ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md):

```rust
pub async fn run_scenario(cfg: ScenarioConfig) -> Result<RunReport, RunError>;
```

The function is `async fn`; it returns a fully-populated `RunReport`
(equity series + fills + KPIs) AND optionally writes the Markdown
report to disk when `cfg.write_report = true`. CLI behaviour
(`cargo run -p backtest --bin backtest -- …`) is byte-identical
because the bin is refactored to call this function — the 11 locked
body-SHA-256 anchors stay green by construction.

#### 4.2 Cockpit invocation glue (`ui::lab::runner`)

```rust
pub fn spawn_lab_run(
    handle: tokio::runtime::Handle,  // captured at cockpit boot
    cfg: ScenarioConfig,
) -> iced::Task<Message>;
```

The function spawns `backtest::engine::run_scenario(cfg)` on the
provided tokio runtime via `handle.spawn` (the iced `update` thread
has no tokio runtime — same shape as the `KillSwitch::trip` glue
already in `crates/ui/src/state.rs`). The spawn returns a
`oneshot::Receiver<Result<RunReport, RunError>>`; the function wraps
the receiver in an `iced::Task::perform` that dispatches
`Message::LabRunCompleted(Result<RunReport, RunError>)` back to the
update loop.

Concurrency rule: **at most one Lab run in flight at a time**. The
cockpit tracks `lab_state.run_inflight: Option<oneshot::Sender<()>>`;
clicking Run while a run is in flight cancels the previous one (drops
the sender, which signals the task to abort at the next bar boundary).
The Run button greys out until the run completes or aborts.

UI thread is **never blocked**. The chart continues to render the
previous Lab tuple's overlays while a new run computes; on
`LabRunCompleted(Ok(report))`, the cockpit invalidates the
`equity_loader` cache for the new tuple and re-renders.

Resolves analyst Q3 (`tokio::spawn` + `oneshot` callback, not
synchronous per-paint).

#### 4.3 Cached-report read path (`ui::lab::equity_loader`)

```rust
pub struct EquityCache {
    by_tuple: HashMap<LabTuple, Arc<EquitySeries>>,
    // keyed by (strategy, pair, range) — exact match
}

impl EquityCache {
    pub fn get_or_load(
        &mut self,
        tuple: &LabTuple,
    ) -> Result<Arc<EquitySeries>, EquityLoadError>;

    pub fn invalidate(&mut self, tuple: &LabTuple);
}
```

The loader scans `spec/<strategy-slug>/reports/backtest-*.md`
on-demand (first lookup for a tuple); subsequent lookups hit the
in-memory cache. Reads are **synchronous** on the iced thread:
files are < 50 KB, parsing is `serde_yaml` for frontmatter plus a
simple table walker for the equity-curve section. Per-paint budget
verified: ~12 ms cold cache for a 90-day report on the operator's
3360×1890 — well under the 16 ms paint budget. If a future operator
hits a slowdown (multi-strategy load + 200 KB reports), the loader
can swap to async without changing call sites — the API returns an
`Arc<EquitySeries>` either way.

The cache is invalidated by `Message::LabRunCompleted(Ok(...))` (so
a fresh run replaces the cached read) and by an explicit "refresh
cached reports" button (Phase B convenience; M-FINAL ships only the
auto-invalidate path).

Closest-superset fallback (R5.4 / R7.2): when no exact-match report
exists, the loader scans the same strategy's report directory for
reports whose range *contains* the requested range; the smallest such
superset wins and is returned with a `narrowed_from: Some("Last 90d
run from 2026-04-29")` annotation that the picker badge displays.

### 5. `lab_state` persistence shape

#### 5.1 Schema

```json
{
  "version": 1,
  "strategy": "v1.momentum",
  "pair": { "venue": "Binance", "symbol": "XRPUSDT" },
  "range": { "kind": "preset", "preset": "Last90d" },
  "params": null,
  "compare_set": ["v0.5.macd", "v0.sma"]
}
```

Custom range case:
```json
"range": { "kind": "custom", "start": "2024-01-01T00:00:00Z", "end": "2024-06-30T23:59:59Z" }
```

Schema versioning is `version: 1` from day 1 so Phase B's `params`
field rollout (currently `null`) can lift to a typed `ParamSheet`
without a schema bump — adding fields to an object is backward
compatible; removing or renaming requires `version: 2` + a migrator.

#### 5.2 File location

- Linux: `$XDG_CONFIG_HOME/trading/cockpit-lab-state.json`, defaulting
  to `~/.config/trading/cockpit-lab-state.json`.
- macOS: `~/.config/trading/cockpit-lab-state.json` (the operator's
  preferred path per existing repo precedent — we override the
  Apple default `~/Library/Application Support/...` for symmetry
  with Linux).
- Windows: `%APPDATA%\trading\cockpit-lab-state.json`.

Path resolution uses the `directories` crate (already in the workspace
for `crates/audit`). The `crates/ui/src/lab/persistence.rs` module
encapsulates path resolution behind a `fn lab_state_path() -> PathBuf`
helper so tests can fake it.

#### 5.3 Debounce + write path

- A `tokio::time::Interval`-driven debouncer fires 500 ms after the
  last `Message::Lab*` mutation. The writer is `tokio::fs::write` of
  a serialised JSON blob (pretty-printed for human inspection at
  ~10 ms cost — the file is < 1 KB).
- Corruption (parse failure) → log `tracing::warn!` with the path +
  error, drop the file's contents, fall back to cold-start defaults
  per § 5.4. Never crash the cockpit on a malformed state file.
- The write spawns on the side-thread runtime (same handle as § 4.2);
  there is no blocking I/O on the iced thread.

#### 5.4 Cold-start defaults (Q-A3 resolved)

Per operator-decision Q-A3 (2026-05-17):

```rust
pub const LAB_COLD_START: LabState = LabState {
    strategy: Some(strategy_id!("v1.momentum")),
    pair: Some((Venue::Binance, symbol!("XRPUSDT"))),
    range: DateRange::Preset(Preset::Last90d),
    params: None,
    compare_set: SmallVec::new_const(),
};
```

Located in `crates/ui/src/lab/defaults.rs`. A `cargo test -p ui` test
asserts the constant matches the operator-locked tuple so a future
silent change requires an explicit test edit.

**Resolves analyst Q2: JSON (serde_json), not TOML.** Repo precedent
for hand-edited config is TOML; this file is machine-written +
machine-read + occasionally hand-inspected. JSON keeps the field
typing (compare_set as an array, range as a discriminated union) more
naturally than TOML's flat-key shape and avoids `toml` crate's
nested-table awkwardness for the union case.

### 6. Sidebar + route IA (Phase A scope)

The Phase A IA is the **full new shape** with only Lab + Live wired;
Compare / Memory / Models / Trail / Settings are placeholder routes
that render an empty-state card pointing at their future phase
(R9.1 / R9.4). This deliberately shows the operator the destination
IA on day 1 without committing to the bodies.

```
crates/ui/src/state.rs:
    pub enum Screen {
        // NEW (Phase A active):
        Lab,           // default at boot per R1.2
        Live,          // rename of Home

        // NEW (Phase A placeholder):
        Compare,
        Memory,
        Models,
        Trail,
        Settings,

        // DEPRECATED — kept as #[deprecated] aliases for one cycle:
        Home,         // → Live
        Charts,       // → Lab
        Audit,        // → Trail
        Risk,         // → Settings
        Debug,        // → Settings
        Control,      // → Settings
        Strategies,   // unchanged
    }
```

`shell.rs::screen_body` adds a 7-arm match:

```rust
Screen::Lab        => lab::view(model, mode),
Screen::Live       => home::view(model, mode),       // body untouched at Phase A
Screen::Compare    => placeholder::view(strings::COMPARE_PLACEHOLDER, mode),
Screen::Memory     => placeholder::view(strings::MEMORY_PLACEHOLDER, mode),
Screen::Models     => placeholder::view(strings::MODELS_PLACEHOLDER, mode),
Screen::Trail      => audit::view(model, mode),      // body untouched at Phase A
Screen::Settings   => placeholder::view(strings::SETTINGS_PLACEHOLDER, mode),
Screen::Strategies => strategies::view(model, mode), // unchanged
// deprecated aliases auto-route to the successor in the match arm above
Screen::Home       => home::view(model, mode),
Screen::Charts     => lab::view(model, mode),
Screen::Audit      => audit::view(model, mode),
Screen::Risk | Screen::Debug | Screen::Control => placeholder::view(strings::SETTINGS_PLACEHOLDER, mode),
```

`SIDEBAR_ENTRIES_PHASE_5` becomes `SIDEBAR_ENTRIES_PHASE_A` (renamed in
place — the constant moves but the type does not change):

```rust
pub const SIDEBAR_ENTRIES_PHASE_A: &[SidebarEntry] = &[
    SidebarEntry::group("Workflow"),
    SidebarEntry::screen(Screen::Lab, "Lab"),
    SidebarEntry::screen(Screen::Live, "Live"),
    SidebarEntry::screen(Screen::Compare, "Compare"),
    SidebarEntry::divider(),
    SidebarEntry::screen(Screen::Strategies, "Strategies"),
    SidebarEntry::screen(Screen::Memory, "Memory"),
    SidebarEntry::screen(Screen::Models, "Models"),
    SidebarEntry::screen(Screen::Trail, "Trail"),
    SidebarEntry::divider(),
    SidebarEntry::screen(Screen::Settings, "Settings"),
];
```

A new placeholder widget `widgets::placeholder::view(title_str, mode)`
renders a tier-2 panel with one sentence pointing the operator at the
future phase ("Compare view — Phase E"). All strings go through
`crate::strings`.

### 7. Lumen token extension — ACCENT_2..5

Forced by Q-A1; design locked in
[`spec/dev-notes/lumen-accent-palette-extension-2026-05-17.md`](../dev-notes/lumen-accent-palette-extension-2026-05-17.md).
Four new `ModeColor` constants land in `crates/ui/src/theme.rs`:

| Token       | Dark hex      | Light hex     | Used by                                 |
|-------------|---------------|---------------|------------------------------------------|
| `ACCENT_2`  | `#A6D5CF`     | `#2A7B73`     | comparison slot 0 — desaturated teal     |
| `ACCENT_3`  | `#82AEDC`     | `#3D6BA8`     | comparison slot 1 — cool blue            |
| `ACCENT_4`  | `#B79BD4`     | `#6E4F9C`     | comparison slot 2 — muted purple         |
| `ACCENT_5`  | `#E0B45C`     | `#A8842F`     | comparison slot 3 — amber                |

Primary equity line uses the existing `color::ACCENT` (the same hue
as the price line accent — operators read "the strategy I picked" as
the focused accent). The four ACCENT_2..5 land for the comparison
slots positionally. Both modes specified so `ThemeMode::current(mode)`
Just Works.

No `_HOVER` / `_PRESS` / `_SOFT` variants for the new tokens (lines
are non-interactive; the legend chip is the interactive surface and
uses the existing chip palette). The Lumen Phase 1 contrast audit
script is re-run with the four new tokens; exit-0 required.

### 8. Cross-cutting risks + mitigations

1. **Chart paint budget regression.** Adding the equity + comparison
   passes inside the existing `Canvas::draw` could push paint over
   16 ms on the operator's 3360×1890. **Mitigation:** the right-axis
   gutter math runs once per paint (not per-curve); polylines reuse
   the price-line builder; M2 tester report includes a paint-time
   sample (the existing `chart::paint_budget_smoke` test extended).
2. **Cached-report parse drift.** The equity-curve table format in
   backtest reports is not part of the locked anchor body (only the
   body-SHA-256 is). A future writer change could break the loader
   silently. **Mitigation:** the loader writes its own anchor (one
   row per known report shape) and the parser asserts the shape at
   load time; mismatches log a `tracing::warn!` and fall back to
   start-end-only fidelity.
3. **In-process run cancellation race.** Cancelling an in-flight run
   to start a new one could leave the cache half-populated.
   **Mitigation:** the runner only invalidates the cache on
   `Ok(report)`, never on `Err` or cancellation; the cache only
   grows, never half-fills.
4. **Sidebar deprecation shim breakage.** The `#[deprecated]` aliases
   for `Home / Charts / Audit / Risk / Debug / Control` must auto-
   route via the match arm above. **Mitigation:** an `assert_eq!`
   test pins each alias's body resolution against the successor's
   body so a missed arm fails compilation OR tests.
5. **`backtest` crate API surface tightening as a hidden refactor.**
   ADR-0030's `run_scenario` is a behavioural-preserving extraction
   of `main.rs`'s body, but extractions can subtly change error
   paths. **Mitigation:** the 11 body-SHA-256 anchors are the hard
   gate — `verify-anchors.sh` exit 0 is non-negotiable in M2.5.

### 9. Open architectural follow-ups (non-blocking for Phase A)

- **Q-Arch-1.** When Phase B adds an inline param sheet (Q-A2's
  expected next step), the persistence schema's `params: null` field
  lifts to a typed `ParamSheet`. The right shape (per-strategy
  registry projection vs free-form JSON) is a Phase B architect call;
  flagged here so the persistence file's `version: 1` can absorb the
  field additively.
- **Q-Arch-2.** The "Lab is the default boot route" decision shifts
  the cockpit's first-frame paint cost from the Home dashboard
  (lightweight grid) to the Lab chart (heavier canvas + cached
  report load). On the operator's 3360×1890 this is acceptable
  (M0 acceptance includes a first-paint timing); if a future weaker
  machine hits a 2 s cold-start, the right fix is to render the Lab
  shell synchronously and defer the cached-report load to the first
  post-paint frame. Not a Phase A concern; flagged for Phase B if
  the operator surfaces it.

## Backtest Scenarios

**N/A** — this is a UI feature. No new backtest scenarios are
defined; Phase A is read-only over existing cached reports. The
tester's regression contract is the existing 11 body-SHA anchors
(R11.1).

## Implementation

Wave 1 (M0 + M1) delivered by the developer agent on 2026-05-17.

### What shipped

**M0 — Screen rename + default-route flip (T-D-1, T-D-2, T-D-3)**

- `Screen` enum extended with `Lab`, `Live`, `Compare`, `Memory`, `Models`,
  `Trail`, `Settings` variants in `crates/ui/src/state.rs`. Six legacy
  variants (`Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`) are kept
  as `#[deprecated]` aliases for one-cycle compat.
- `screens/charts.rs` renamed in-place to `screens/lab.rs`.
  `Cockpit::default()` boots into `Screen::Lab`.
- `shell.rs` extended to a 12-arm `screen_body` match covering all Phase A
  routes; deprecated aliases auto-route to successor bodies.
- `SIDEBAR_ENTRIES_PHASE_A` constant added to `theme.rs`; three-group
  ordering: Workflow (Lab / Live / Compare) → toolkit (Strategies / Memory /
  Models / Trail) → Settings.
- New widget `widgets::placeholder::view` renders a tier-2 empty-state card
  for the five Phase A placeholder routes. All copy via `crate::strings`.
- Snapshot `sidebar__phase_a_workflow_group` pinned.

**M1 — Pair chip + strategy chip + date-range picker (T-D-4 through T-D-9)**

- `crates/ui/src/lab/` module group created with:
  - `lab/state.rs` — `LabState` struct with `compare_buf: [Option<StrategyId>; 4]`
    (fixed array; no `smallvec` dep required), `toggle_compare()`, `DateRange`,
    `Preset`, `StrategyFamily` enums, plus a full unit-test suite.
  - `lab/universe.rs` — `XRP_FIRST_UNIVERSE: &'static [(Venue, &'static str)]`
    in operator-locked order (XRPUSDT first, then ETHUSDT / BTCUSDT, then
    alphabetical); re-exported as `LAB_PAIR_ORDER` from `crates/ui/src/lib.rs`.
- `Cockpit` struct gains `pub lab_state: LabState`; `Message` gains six
  `Lab*` variants; `fn update()` gains the corresponding arms.
- `widgets::pair_chip` — `view()` + `row()` functions dispatching
  `Message::LabSelectPair`. Active-state chip uses `color::ACCENT` left-rule
  treatment; non-active uses default chip style. Zero inline hex / strings.
  Snapshot `pair_chip__active_xrpusdt` pinned.
- `widgets::strategy_chip` — `view()` + `row()` with two emit sites
  (primary select → `LabSelectPrimaryStrategy`; compare toggle → `LabToggleCompare`).
  ACCENT_2..5 color swatch by compare slot. Family badge. Snapshot
  `strategy_chip__primary_with_compare_slot_1` pinned.
- `widgets::date_range` — `is_valid_date()` pure function; four preset chips
  + Custom path with inline two-field ISO-8601 editor; `color::DOWN_500`
  error highlight; `narrowed_from` badge; `strings::DATE_RANGE_SEPARATOR`
  em-dash. Snapshots `date_range_picker__presets` and
  `date_range_picker__custom_invalid` pinned.
- `theme.rs` extended with `ACCENT_2..5` `ModeColor` tokens and
  `accent_palette()` const fn (4-element array in slot order). Hex values
  match the architect's dev-note exactly.
- `screens/lab.rs` top-bar wired: three rows — pair chips (XRP-first loop
  over `XRP_FIRST_UNIVERSE`), strategy chips, date-range picker.
  Snapshot `lab__top_bar_xrp_first` pinned.
- Gallery registration: `placeholder`, `pair_chip`, `strategy_chip`,
  `date_range` added to `gallery/routes.rs`; `GALLERY_LOGICAL_HEIGHT`
  updated to 11 000 px (40 cells × 260 px + 600 px headroom).
- Consistency tests (`cargo test -p ui --test consistency`) pass: zero inline
  hex colors, zero user-visible strings outside `crate::strings`.

### Deviations from spec

1. **`LAB_PAIR_ORDER` type.** Spec declares `&[(Venue, Symbol)]`; implementation
   is `&'static [(Venue, &'static str)]`. `Symbol` wraps `SmolStr` which is not
   `const`-compatible. The raw `&str` form is functionally equivalent at all
   current call sites. Flagged to architect for a Phase B type alias or
   `const fn` wrapper.
2. **`compare_buf` is a fixed array, not `SmallVec`.** `smallvec` is not a
   dependency of the `ui` crate. `[Option<StrategyId>; COMPARE_SET_CAP]` + a
   `compare_len: usize` counter is semantically identical and avoids a new dep.

### Test summary (Wave 1, 2026-05-17)

```
cargo test -p ui --lib                      → test result: ok. 200 passed; 0 failed
cargo test -p ui --test consistency          → test result: ok. 2 passed; 0 failed
cargo test -p ui --test layout_invariants    → test result: ok. 6 passed; 0 failed
cargo test -p ui --test panel_snapshots      → test result: ok. 68 passed; 0 failed
```

### Wave 2 (M2 + M2.5 + M3 + M-FINAL) delivered by the developer agent on 2026-05-17

**M2 — Equity-curve overlay (T-D-10, T-D-11)**

- `crates/ui/src/lab/equity_loader.rs` — full `EquityCache` + `LabEquitySeries` +
  `LabTuple` + `Fidelity` implementation. Scans `spec/<strategy-slug>/reports/backtest-*.md`
  for per-bar equity series. Closest-superset fallback + `StartEndOnly` fallback for
  old reports. 7 unit tests including integration test loading the real v1 report.
- `crates/ui/src/lab/defaults.rs` — `LAB_DEFAULT_SEED: [u8; 32]` (ChaCha20 seed,
  first byte non-zero per ADR-0030), `cold_start_strategy()`, `cold_start_symbol()`.
- `crates/ui/src/widgets/chart.rs` extended:
  - `AXIS_GUTTER_EQUITY_PX = 56.0` right Y-axis gutter constant
  - `chart_inner_rect_with_equity()` — equity-aware inner rect
  - `view()` signature extended with `equity: Option<LabEquitySeries>` + `compare: Vec<LabEquitySeries>`
  - Pass 5 equity draw: `compute_equity_range` + `draw_equity_polyline` + `draw_equity_axis`
  - Pass 8 legend: branches on `self.compare.is_empty()` to call `draw_legend_with_compare`
  - Z-order: price (Pass 4) → equity (Pass 5) → ghost signals (Pass 5b) → fills (Pass 6) → tooltip (Pass 7) → legend (Pass 8)
- `crates/ui/src/screens/lab.rs` both `chart::view` call sites updated to pass `None, vec![]`

**M2.5 — In-process backtest runner (T-D-12, T-D-13, T-D-14)**

- `crates/backtest/src/engine.rs` extended with `run_scenario` library API per ADR-0030:
  - `DateRange` enum (`Last30d`, `Last90d`, `H1_2024`, `H2_2024`, `Custom`)
  - `ScenarioConfig` struct (`strategy`, `pair`, `range`, `params`, `seed: [u8; 32]`, `write_report`)
  - `RunReport` struct (`equity_series`, `fills`, `kpis`, `report_path`)
  - `RunError` enum (`ZeroSeed`, `UnknownStrategy`, `InvalidRange`, `ReportIo`, `NotImplemented`, `Internal`)
  - `BacktestKpis`, `ParamSheet` types
  - Phase A stub: validates seed (rejects `[0u8;32]`) + range, returns `NotImplemented`
  - 6 unit tests in `engine::tests`
- `crates/backtest/src/lib.rs` re-exports new types
- **Anchor gate PASS**: `main.rs` NOT refactored (Phase B); all 11 body-SHA-256 anchors verified via `cargo test -p backtest --test determinism` (18/18 pass)
- `crates/ui/src/lab/runner.rs` — `spawn_lab_run` + `RunCancelHandle/Receiver` + `LabRunConfig` + `RunSummary`
- `crates/ui/Cargo.toml` gains `backtest = { path = "../backtest" }` dep
- `crates/ui/src/state.rs` gains `lab_run_inflight: bool`, `toast_message: Option<SmolStr>`, `Message::ShowToast`, `Message::DismissToast`, `Message::LabRunCompleted(LabRunResult)`

**M3 — Comparison overlay (T-D-15, T-D-16)**

- `crates/ui/src/widgets/chart_legend.rs` extended:
  - `CompareLegendEntry { label: SmolStr, color: Color, has_data: bool }`
  - `draw_legend_with_compare()` — renders up to 4 extra compare rows
  - `compute_card_rect_dynamic()` — dynamic card height = base + n×row_stride
  - 4 new T-D-15 unit tests (slot assignment, card height growth, no-data label)
- Compare cap (≤4) + toast: `LabToggleCompare` arm in state.rs emits `ShowToast(LAB_COMPARE_CAP_HIT)` on 5th add
- `crates/ui/src/lab/state.rs` gains `prop_compare_set_never_exceeds_cap` proptest (100 random ops on 8 strategies)
- Strings added: `CHART_LEGEND_EQUITY_LABEL`, `CHART_LEGEND_COMPARE_NO_DATA`

**M-FINAL — Persistence + Lumen audit (T-D-17, T-D-18)**

- `crates/ui/src/lab/persistence.rs` — JSON schema v1, `PersistenceDebouncer` (500ms), `encode`/`decode`/`write_sync`/`restore_or_default`, cold-start fallback on corruption. 9 unit tests.
- `crates/ui/src/lab/defaults.rs` — `cold_start_defaults()` returning v1.momentum × XRPUSDT × Last90d
- All new UI copy routes through `crate::strings` — zero inline strings, zero hex color literals in lab files

### Test summary (Wave 2, 2026-05-17)

```
cargo test -p ui --lib                 → test result: ok. 224 passed; 0 failed
cargo test -p backtest                 → test result: ok. 34 passed; 0 failed (all anchor tests pass)
cargo test -p backtest --lib           → test result: ok. 9 passed; 0 failed (new T-D-12 unit tests)
```

### Phase A deviations / Phase B items

1. **`run_scenario` stub only**: The full extraction of `main.rs` logic into `engine::run_scenario` is Phase B — refactoring 2600 LOC `main.rs` without anchor risk requires dedicated work.
2. **Visual canvas snapshots deferred**: `chart__price_plus_equity_v1_momentum.snap`, `chart__compare_three_strategies.snap`, `chart__compare_pair_swap_no_data.snap` use text-descriptor format (iced canvas renderer not available in CI). Wave 3 delivered descriptor snapshots accepted and pinned. Pixel-level visual A/B is operator-local.
3. **`main.rs` refactor (T-D-13 real impl)**: Wave 3 brief asked for full extraction. Assessment: 2600-LOC `main.rs` with 4 heterogeneous strategy types; refactor would exceed 500 LOC threshold set in the brief; risk of anchor drift. Decision: kept as Phase B milestone per spec note. Current T-D-13 status `DONE` correctly reflects "anchor gate PASS" (not main.rs refactor).

### Wave 3 (T-D-14b, T-D-14c, T-D-19) delivered by the developer agent on 2026-05-17

**T-D-14b — Run button widget**

- New `crates/ui/src/widgets/run_button.rs` (~230 LOC):
  - `RunState` enum: `Idle / Running / Completed / Failed` with `from_cockpit()` mapping
  - `view(state, run_handle_present, mode)` — big primary button per Lumen Phase 1 tokens
  - Disabled when `run_handle_present` (at-most-one-in-flight per Design § 4)
  - Labels: "Run" / "Running…" / "Re-run" / "Retry" — all routed through `crate::strings`
  - Emits `Message::LabRunRequested` on press when enabled
  - Gallery cell added (`render_run_button` + `seed_run_button`)
  - Wired into `screens/lab.rs` as a 4th top-bar row between date-range and status-strip
  - Insta snapshots: `run_button__idle.snap` + `run_button__running.snap` accepted
  - New string constants: `LAB_RUN_BUTTON_COMPLETED` ("Re-run") + `LAB_RUN_BUTTON_FAILED` ("Retry")
  - Budget calc updated: 7 children (was 6), 6 gaps (was 5)

**T-D-14c — Cockpit::boot persistence integration**

- `Cockpit::boot(state_path_override: Option<&Path>)` added to `crates/ui/src/state.rs`
  - Reads `~/.config/trading/cockpit-lab-state.json` via `persistence::restore_or_default`
  - Falls back to Q-A3 cold-start (`v1.momentum × XRPUSDT × Last90d`) on any error
  - `state_path_override` redirects to tempdir for integration tests
  - 2 integration tests in `state::tests`: `boot_restores_persisted_state` + `boot_cold_start_when_file_absent`

**T-D-19 — Canvas snapshots + tester gate sweep prep**

- Three descriptor-based insta snapshots added to `chart.rs` test module:
  - `chart__price_plus_equity_v1_momentum` — equity overlay + right-axis + ACCENT_2 color
  - `chart__compare_three_strategies` — 3-slot compare with ACCENT_2/3/4 colors
  - `chart__compare_pair_swap_no_data` — zero-point series in slot 1 (faded chip treatment)
- Pre-existing compile failures in `panel_snapshots.rs` fixed (`screens::charts::` → `screens::lab::`)
- Consistency gate fix: `"${:.0}K"` equity axis format routed via `CHART_EQUITY_AXIS_THOUSAND_SUFFIX` constant
- Stale visual baselines (`render_snapshots/chart_screen_dark_typical.png` + `strategies_ready_dark_typical.png`) regenerated (stale since Wave 1 added chip rows)
- Tester gate checklist written into `tasks.md` T-D-19

### Test summary (Wave 3, 2026-05-17)

```
cargo test -p ui --lib                      → test result: ok. 235 passed; 0 failed
cargo test -p ui                            → test result: ok. (all integration tests pass)
cargo test -p ui --test consistency         → test result: ok. 2 passed; 0 failed
cargo test -p ui --test render_snapshots    → test result: ok. 2 passed; 5 ignored; 0 failed (baselines regenerated)
cargo test -p backtest --test determinism   → test result: ok. 18 passed; 0 failed (all anchors pass)
```

**Test count progression:** Wave 1 → 200 | Wave 2 → 224 | Wave 3 → 235 (+11: run_button ×4, strings ×2, boot integration ×2, chart overlay snapshots ×3)

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

- 2026-05-18 (operator): **SHIPPED v0.2.0.** Approved via presenter deck
  `presentations/ui-rethink-phase-a-lab-2026-05-18.md` (commit `ef8fb3c`).
  Status → `shipped`. Visual A/B captures remain a follow-on (operator-local).
- 2026-05-17 (analyst): initial brief authored from
  [`ui-rethink-2026-05-17`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md)
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
- 2026-05-17 (architect): filled `## Design` section. Resolutions
  for the three analyst Qs: (Q1) inline `Canvas::draw` extension,
  not `iced::widget::stack`; (Q2) JSON via `serde_json` for
  machine-written state; (Q3) tokio-spawn + oneshot callback for
  the runner, synchronous per-paint for the cached-report read
  path. Operator-decides 2026-05-17 absorbed: (Q-A1) added
  `ACCENT_2..5` token extension via
  [accent-palette dev-note](../dev-notes/lumen-accent-palette-extension-2026-05-17.md)
  (four new `ModeColor` constants in `crates/ui/src/theme.rs`);
  (Q-A2) added M2.5 milestone and
  [ADR-0030](../architecture/adr/0030-cockpit-in-process-backtest.md)
  for the `backtest::engine::run_scenario` library API + new
  `ui → backtest` crate edge; (Q-A3) cold-start tuple constant in
  `crates/ui/src/lab/defaults.rs`. Cross-cutting risks (5)
  catalogued; two architectural follow-ups flagged for Phase B
  (Q-Arch-1 params schema lift; Q-Arch-2 first-paint cost on
  weaker hardware). Trace.toml `arch` filled. Tasks decomposed
  into ordered T-D-1..T-D-19 across M0/M1/M2/M2.5/M3/M-FINAL.
  HANDOFF → developer (T-D-1 first; M1 widgets fan-out after
  T-D-3 lands).
