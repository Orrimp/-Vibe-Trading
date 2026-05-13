---
slug: iced-native-widgets
status: shipped
owner: shipped
updated: 2026-05-13
version: 0.1.0
predecessor: iced-ecosystem-evaluation v0.2.0
refinement_pass: 2026-05-13 (orchestrator T-M0-J through T-M0-N grep batch)
---

# iced native widgets — Brief A (v0.1.0)

> **Status:** research + scoping brief. **No code changes, no crate adds.**
> Analyst scopes the four migrations greenlit by Brief A of the
> [`iced-ecosystem-evaluation`](../iced-ecosystem-evaluation/feature.md) v0.2.0
> architect synthesis. Architect picks up this brief and converts the
> hypothesis register into orchestrator-runnable falsifiers (per
> [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only)).

## Why

**Operator's approval (2026-05-13, Q-O3):** "Adoption order A → B → C → D
unchanged" — i.e. native iced 0.14 widgets first, then `iced_aw` cherry-pick,
then `iced_dialog` (gated), then `plotters-iced2` (gated). This brief IS
Brief A.

The cockpit's hand-rolled widget surface stands at **22 widgets / ~5.2 kLOC**
([`crates/ui/src/widgets/`](../../crates/ui/src/widgets/)). Brief A
targets the four largest, lowest-risk migrations whose target primitives
already compile in the workspace lockfile under our current
`iced = "=0.14.0"` feature set `["tiny-skia", "thread-pool", "advanced",
"canvas"]` (per [`iced-ecosystem-evaluation` H-arch-0
RESOLVED-FALSIFIED-partial](../iced-ecosystem-evaluation/feature.md#hypothesis-register-architect-2026-05-13)
build-artifact evidence — `table.rs`, `grid.rs`, `float.rs` all present in
the compiled `iced_widget-0.14.2` `.d` manifest).

Brief A's payload: retire ~900-1100 LOC of hand-rolled `Row`/`Column`/`Stack`
glue across four widgets by routing their layout through iced's first-party
`table`, `grid`, and `float` primitives. **Zero new direct or transitive
crates.**

### What this brief does NOT do

Locked exclusions, inherited from the [`iced-ecosystem-evaluation` v0.2.0
architect synthesis](../iced-ecosystem-evaluation/feature.md#design--architect-synthesis):

- **`agent_feed.rs` migration (A3)** — HELD per architect Q1. Falsifier
  evidence (H-arch-7) is consistent with native `table` being eager-only
  (no row virtualization API). At ≥500 visible rows steady-state,
  `agent_feed` keeps its `Scrollable<Column>` glue. Revisit when iced
  ships `table` with lazy children.
- **`chart_tooltip.rs` migration** — HELD per architect Q3 / H-arch-2
  RESOLVED-UNFALSIFIED. The tooltip is canvas-internal
  (`chart_tooltip::draw_tooltip` is a free function called from
  `ChartProgram::draw` at [`crates/ui/src/widgets/chart.rs:468`](../../crates/ui/src/widgets/chart.rs);
  no `impl Widget` exists in [`chart_tooltip.rs`](../../crates/ui/src/widgets/chart_tooltip.rs)).
  Native `float` operates on widget-tree elements; porting requires lifting
  tooltip out of canvas first, which is a separate brief.
- **In-cockpit `markdown` viewer (M5)** — operator-gated Q-O2 = ADOPT but
  scoped to a separate brief slot. Brief A does NOT enable the `markdown`
  iced feature flag (currently OFF — declared but inactive per
  fingerprint JSON evidence). M5 owns the 1-line Cargo.toml edit.
- **`iced_aw` cherry-pick (Brief B)** — separate brief; not in scope here.
- **Adoption of `pin`, `sensor`, `Animation` API** — Brief A is layout
  primitives only (table/grid/float). `pin`/`sensor` have no current
  cockpit fit; `Animation` API depends on operator's Q-O1 ratification.

## Predecessor + decisions inherited

From [`spec/iced-ecosystem-evaluation/feature.md`](../iced-ecosystem-evaluation/feature.md)
v0.2.0 (architect synthesis + M0 falsifier pass + operator Q-O1/Q-O2/Q-O3
resolution, all 2026-05-13):

| Decision | Source | Resolution | Impact on Brief A |
|---|---|---|---|
| Q1 — native `table` for tabular surfaces | architect synthesis | ADOPT for `positions.rs` + `strategies.rs`; HOLD `agent_feed.rs` | R1, R2 in scope; agent_feed out |
| Q2 — native `grid` for KPI strip | architect synthesis | ADOPT for `kpi_strip.rs` | R3 in scope |
| Q3 — native `float` for overlays | architect synthesis | ADOPT for `journal_transaction_modal.rs`; DEFER for `chart_tooltip.rs` | R4 in scope; chart_tooltip out |
| Q-O3 — brief ordering | operator | A → B → C → D unchanged | This IS Brief A |
| H-arch-0 — native widgets reachable under current iced features | M0 falsifier sub-agent | **RESOLVED-FALSIFIED-partial.** `table`/`grid`/`float`/`pin` REACHABLE; `markdown` requires feature flag | R1-R4 unblocked; M5 deferred |
| H-arch-2 — `chart_tooltip` is canvas-internal | M0 falsifier sub-agent | **RESOLVED-UNFALSIFIED-confirm.** `draw_tooltip` is a free function called from `ChartProgram::draw`; no `impl Widget` | `chart_tooltip` out of Brief A |
| H-arch-7 — native `table` lacks virtualization | M0 falsifier sub-agent | **RESOLVED-UNFALSIFIED-partial.** Indirect evidence consistent; `lazy` is a sibling feature-gated module. Orchestrator-direct grep of `iced_widget-0.14.2/src/table.rs` (per task brief) confirms public API is `Table::new(items: Vec<T>)` with `pub fn table()`, `pub fn column()`, `pub struct Table`, `pub struct Column`, `pub struct Style`, `pub trait Catalog`, padding/separator methods — and ZERO matches for `with_offset|virtual|lazy|skip|row_budget|visible_range|from_iter|Lazy|with_capacity_hint`. | `agent_feed` stays HELD; R1/R2 unaffected (bounded-row surfaces) |

## Requirements

One requirement per migration target. Each Rn captures **current shape** +
**data dependency** + **state semantics** so the architect's hypothesis
pass has a concrete pre-port reference.

### R1 — Positions table migration

**Target:** [`crates/ui/src/widgets/positions.rs`](../../crates/ui/src/widgets/positions.rs)
(100 LOC).

**Current shape:** [`positions.rs:38-46`](../../crates/ui/src/widgets/positions.rs)
hand-rolls a 7-column `Row::new()` header (`SYMBOL`, `QTY`, `COST`, `MARK`,
`PNL`, `PNL_PCT`, `EXPOSURE`) over a `Scrollable<Column>` of
per-position rows. Each row at [`positions.rs:65-74`](../../crates/ui/src/widgets/positions.rs)
is a 7-cell `Row::new().push(...).push(...)` wrapped in
[`active_row(...)`](../../crates/ui/src/widgets/frame.rs) for the
2 px left-rule chrome.

> **Brief discrepancy note:** the task brief lists 6 columns (symbol, qty,
> cost_basis, last_mark, pnl, pnl_pct); the actual file has **7 columns**
> with `EXPOSURE` as column 7. The migration must preserve all 7.

**Data shape:** consumes `&[trading_core::PositionView]`. Each `PositionView`
carries `symbol`, `base_qty: Decimal`, `cost_basis: Money`,
`last_mark: NotNan<f64>`, `pnl: Money`, `pnl_pct: Decimal`,
`exposure_pct: Decimal`. Render order is **iteration order of the input
slice** — no in-widget sort; sort happens upstream in
`crate::state` if at all.

**State semantics** ([`positions.rs:20-25`](../../crates/ui/src/widgets/positions.rs)):
`PanelState<Vec<PositionView>>` four-variant — `Loading` → muted body
("Loading positions…"); `Empty` → muted body ("No open positions");
`Error(e)` → red `POS_ERROR_PREFIX + e`; `Ready(visible)` → table (with
zero-qty positions filtered out at [`positions.rs:31-33`](../../crates/ui/src/widgets/positions.rs)
per T17 acceptance; if the visible slice is empty, the body falls back to
the `Empty` copy).

**Per-cell coloring:** PNL and PNL_PCT cells use `color_for_delta(...)`
(positive/negative sentiment); other cells use `color::FG_1`. Migration
must route per-cell color through `iced::widget::table`'s style/catalog
hook OR keep the colored-cell helper in the column's body closure.

**Acceptance gate:** the migration is byte-equivalent under
`cargo test -p ui --test panel_snapshots positions` after a one-shot
`cargo insta review` refresh of the 6 `positions_*.snap` baselines (or
stays byte-identical, ideally).

### R2 — Strategies table migration

**Target:** [`crates/ui/src/widgets/strategies.rs`](../../crates/ui/src/widgets/strategies.rs)
(344 LOC). **Only the table-glue portion migrates** — the per-row error
badge, the recent-events footer, and the pause-button surface stay
hand-rolled.

**Current shape:** [`strategies.rs:63-70`](../../crates/ui/src/widgets/strategies.rs)
hand-rolls a 6-column `Row::new()` header (`ID`, `HASH`, `STATUS`,
`LAST_EVENT`, `SIGNALS_60S`, `POSITION`) over a `Scrollable<Column>` of
per-strategy rows. Each row at [`strategies.rs:134-141`](../../crates/ui/src/widgets/strategies.rs)
is a 6-cell `Row::new()` wrapped in a `Button` for click-dispatch
(`Message::SelectStrategy(r.id.clone())`), then wrapped in `active_row(...)`
for the 2 px ACCENT left-rule on the selected row. **Per-row error
badge** at [`strategies.rs:77-79`](../../crates/ui/src/widgets/strategies.rs)
is rendered as a sibling row beneath the main row.

**Data shape:** consumes `&[StrategyRow]` and
`&VecDeque<StrategyEventView>` (the footer recent-events log). `StrategyRow`
carries `id: StrategyId`, `short_hash: String`, `status: StrategyStatus`
(enum: `Ready` / `Loading` / `Error(SmolStr)`), `last_event:
Option<StrategyEventView>`, `signals_60s: u32`, `has_position: bool`.
Iteration order is upstream-fixed (no in-widget sort).

**State semantics** ([`strategies.rs:45-54`](../../crates/ui/src/widgets/strategies.rs)):
identical `PanelState` four-variant pattern as R1.

**Per-row interactivity (PRESERVE):**
- Whole-row click → `Message::SelectStrategy(...)` (R5.2 / Q11b compound dispatch).
- Hover background color → `color::PANEL_SUNKEN`.
- 2 px ACCENT left-rule when `selected_strategy == Some(&r.id)`.
- Per-row error badge rendered as a follow-up row when `status ==
  StrategyStatus::Error(...)`.

**Migration constraint (refinement pass 2026-05-13):** native `table` exposes
NO row-click hook (T-M0-M / H-arch-A5b RESOLVED-CONFIRM: zero matches in
`table.rs` for `row_decorator|after_row|tail|on_row|row_overlay`). The
migration **commits** to wrapping column 1's body lambda in
`Button::new(cell_id).on_press(Message::SelectStrategy(r.id.clone()))` (Q5
resolution). No fallback branch — this is the path. Columns 2-6 stay plain
`Element`; mouse-event bubbling delivers click-through. Selected-row 2 px
ACCENT left-rule routes through the Table `Catalog` impl (see Q3-sub below)
OR the per-cell content's own border helper, depending on Catalog choice.

**Out-of-scope wrt. R2:** the `pause_button(...)` widget at
[`strategies.rs:278-316`](../../crates/ui/src/widgets/strategies.rs) and
the `event_kind_label(...)` helper at
[`strategies.rs:244-262`](../../crates/ui/src/widgets/strategies.rs) stay
unchanged; they are not table-layout concerns.

### R3 — KPI strip grid migration

**Target:** [`crates/ui/src/widgets/kpi_strip.rs`](../../crates/ui/src/widgets/kpi_strip.rs)
(264 LOC).

**Current shape:** [`kpi_strip.rs:123-132`](../../crates/ui/src/widgets/kpi_strip.rs)
hand-rolls a 6-card `Row::new().spacing(space::M).push(...).push(...).push(...).push(...).push(...).push(...)`
layout, with each cell as a `Container::new(Column { label, value })` at
`Length::FillPortion(1)` width.

> **Brief discrepancy note:** the task brief states "4-cell layout"; the
> actual file ships **6 cards** — Total Return, CAGR, Sharpe, Max DD, Win
> Rate, Trades (per [`kpi_strip.rs:81-130`](../../crates/ui/src/widgets/kpi_strip.rs)
> + the unavailable-strip mirror at
> [`kpi_strip.rs:138-149`](../../crates/ui/src/widgets/kpi_strip.rs)).
> Confirmed by the on-file doc comment: "Six metric cards (Total return /
> CAGR / Sharpe / Max DD / Win rate / Trades) in one Row." The migration
> targets a **6-column grid**.

**Data shape:** consumes `&PanelState<BacktestMetrics>` from
`trading_core::BacktestMetrics` — fields `total_return_pct: Decimal`,
`cagr_pct: Decimal`, `cagr_present: bool`, `sharpe: Decimal`,
`sharpe_present: bool`, `max_drawdown_pct: Decimal`, `win_rate_pct:
Decimal`, `win_rate_present: bool`, `trades: u64`. The `*_present` flag
pattern distinguishes "0.0 because absent" from "0.0 because zero" (e.g.
CAGR + Win rate frequently absent in the RSI sample at
[`kpi_strip.rs:192-207`](../../crates/ui/src/widgets/kpi_strip.rs)).

**State semantics:** `Loading` / `Empty` / `Error(_)` all collapse to a
single `unavailable_strip(...)` at [`kpi_strip.rs:135-155`](../../crates/ui/src/widgets/kpi_strip.rs)
— six dash-placeholder cards stacked above a
`VIEWER_METRICS_UNAVAILABLE` muted body. `Ready` additionally collapses
to the unavailable strip when `is_all_absent(m)` (every flag false + every
numeric zero).

**Per-cell content:** each card is a 2-line `label` over `value` where:
- Label: `text::SMALL` + `color::FG_3`.
- Value: `text::H1` (24 px) + sentiment-coloured (Total Return /
  Max DD via `format_pct_sentiment` / `format_pct_max_dd`; CAGR / Sharpe /
  Win Rate / Trades neutral `color::FG_1`; absent values render `—` in
  `color::FG_3`).

**Migration goal:** retire the `Row::new().spacing(...).push(...).push(...).push(...).push(...).push(...).push(...).width(Length::Fill)`
glue + the unavailable-strip mirror loop at
[`kpi_strip.rs:146-149`](../../crates/ui/src/widgets/kpi_strip.rs) in
favor of `iced::widget::grid` with implicit 6-column alignment. The
`card(...)` helper at [`kpi_strip.rs:159-178`](../../crates/ui/src/widgets/kpi_strip.rs)
stays — it carries the label/value composition + sentiment color routing.
The outer Tier-1 PANEL chrome `Container` at
[`kpi_strip.rs:52-65`](../../crates/ui/src/widgets/kpi_strip.rs) stays.

**Wire shape (refinement pass 2026-05-13, T-M0-K / H-arch-A3 RESOLVED-UNFALSIFIED):**
`Grid::new().columns(6).spacing(space::M).push(total_return).push(cagr).push(sharpe).push(max_dd).push(win_rate).push(trades).width(Length::Fill)`.
Confirmed Grid API surface: `Grid::new()`, `Grid::with_capacity(n)`,
`Grid::columns(n)`, `Grid::fluid(max_width)`, `Grid::spacing(px)`,
`Grid::width(px)`, `Grid::height(Sizing)`, `Grid::push(child)`,
`Grid::push_maybe(opt)`, `Grid::extend(iter)`. The 6-card layout maps to
`Grid::columns(6).push(...) × 6` — clean fit, no width-hint per cell.

**Theming (Q3-sub):** Grid has NO `Style` struct, NO `Catalog` trait, NO
`style()` method, NO `class()` method. Visual chrome stays in the outer
PANEL `Container` (already styled via existing closure pattern); per-card
visuals stay in the `card(...)` helper. No grid-level theming required —
the outer container + per-cell content carry all the surface tokens. Some
spacing / separator drift vs hand-rolled `Row` may be visible at snapshot
refresh time; bounded shape-only diff is acceptable per V3B.

### R4 — Journal-transaction modal `float` migration

**Target:** [`crates/ui/src/widgets/journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
(571 LOC). **Only the overlay-positioning portion migrates** — the
typed-confirm chrome, focus-ring integration, journal-row click-through,
metadata block, and entries-table sub-blocks stay hand-rolled.

**Current shape:** [`journal_transaction_modal.rs:99-111`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
hand-rolls a 3-layer `Stack` overlay — bottom = cockpit `content`,
middle = backdrop `MouseArea<Container<Space>>` with
`color::OVERLAY` background + `on_press(close_msg)` click-outside dispatch
([`journal_transaction_modal.rs:115-131`](../../crates/ui/src/widgets/journal_transaction_modal.rs)),
top = centered modal card (`Length::Fixed(480.0)` width, `space::XL`
padding, `BORDER_STRONG` rim,
[`journal_transaction_modal.rs:133-176`](../../crates/ui/src/widgets/journal_transaction_modal.rs)).
The architect's design (Q1 of the predecessor brief) named
`iced::widget::Stack` as the chosen overlay primitive at the time — Brief
A swaps to native `float`.

**Data shape:** consumes `&JournalModalState` (`tx_id: SmolStr`, `entries:
PanelState<JournalTransactionView>`). Triggered by
`Message::TapeRowClicked(transaction_id)` at
[`crates/ui/src/state.rs:1324`](../../crates/ui/src/state.rs); the update
handler at [`state.rs:1324-1335`](../../crates/ui/src/state.rs) sets
`model.tape_audit_modal = Some(JournalModalState { tx_id, entries:
PanelState::Loading })` and kicks off the async fetch. Modal closes via
`Message::TapeAuditModalClosed` at [`state.rs:1036-1041`](../../crates/ui/src/state.rs)
+ [`state.rs:1336`](../../crates/ui/src/state.rs).

**Close affordances (PRESERVE — three paths per R4):**
- **Escape key** — keyboard subscription dispatches
  `Message::TapeAuditModalClosed` (T1206 wiring; lives in the cockpit's
  subscription path, not in the widget itself).
- **Click-outside** — backdrop's `MouseArea::on_press(close_msg)`.
- **Explicit Close button** — header's `Button::new("Close")
  .on_press(close_msg)` at
  [`journal_transaction_modal.rs:193-225`](../../crates/ui/src/widgets/journal_transaction_modal.rs).

**Focus-ring integration (PRESERVE):** the Close button at
[`journal_transaction_modal.rs:193-225`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
renders a `theme::focus::ring` shadow on hover (T1504) as a
best-effort focus indicator. Per the inline NOTE comment, iced 0.14's
`button::Status` lacks a `Focused` variant, so true keyboard focus is
deferred; this constraint does NOT block Brief A — `float` migration
inherits the same hover-only behavior.

**State semantics** ([`journal_transaction_modal.rs:331-348`](../../crates/ui/src/widgets/journal_transaction_modal.rs)):
`Loading` / `Empty` / `Error(_)` render centered-message bodies inside the
card; `Ready(view)` renders the metadata block + 4-column
`Account|Debit|Credit|Currency` entries table.

**Migration goal:** swap the centered-card-via-Stack-+-center_x/y composition
for `iced::widget::float::Float` as the **positioning** primitive. Refinement
pass 2026-05-13 (T-M0-M / T-M0-N) locks down `Float` semantics:
- `Float::style(impl Fn(&Theme) -> Style + 'a)` — closure-routed, fits the
  cockpit pattern.
- **`Float` is positioning-only**: ZERO matches in `float.rs` for
  `on_dismiss|on_close|on_outside_click|Background|backdrop|focus_trap|keyboard::|on_key|Escape|key_press|subscription`.
  No dismiss callback. No keyboard participation. No focus trap. No
  backdrop hook.

**Three-close-path wiring (refinement pass):**
- **Escape key** — stays in `state.rs` keyboard subscription (Q7 default;
  H-arch-A7b CONFIRMED via T-M0-N). `Subscription::with_state` /
  `keyboard::on_key_press` dispatches `Message::TapeAuditModalClosed` from
  the application root. Path unchanged by Brief A.
- **Click-outside** — KEEP the existing hand-rolled `backdrop_layer` at
  [`journal_transaction_modal.rs:118-131`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (the `MouseArea::new(Space).on_press(close_msg)` sibling). `Float`
  cannot dismiss; only `MouseArea` can. The backdrop is now a **sibling
  layer composed into `Float`'s base element** (e.g. `Stack::new().push(content).push(backdrop_layer(close_msg))`)
  with `Float::new(stack, card)` providing centering on top.
- **Explicit Close button** — header's `Button::on_press(close_msg)`
  unchanged at [`journal_transaction_modal.rs:193-225`](../../crates/ui/src/widgets/journal_transaction_modal.rs).

**Focus-trap status:** NOT provided by `Float`. Brief A inherits the iced
0.14 button-focus limitation; Tab can escape the modal. Out-of-scope to
fix here (no regression vs current hand-rolled).

**Brief C (`iced_dialog`) trigger reaffirmed:** the predecessor brief's
H-arch-6 / H-arch-11 path remains the route to native dismiss + focus
trap. Brief A ships `Float` for positioning only and reports the
finding (no `on_dismiss` in v0.14 `float`).

## Acceptance criteria

Each `Vn` block aligns with `Rn`. The five sub-criteria (`VnA`-`VnE`) are
identical in shape across requirements — the architect/dev pass repeats
them per-widget.

### V1 — Positions table migration

- **V1A — Compile + tests.** `cargo build -p ui` succeeds; `cargo test
  -p ui` and `cargo test -p ui --test panel_snapshots positions` both
  pass with the refreshed baselines.
- **V1B — Snapshot diff is shape-only.** The 6 `positions_*.snap`
  baselines refresh via `cargo insta review` after the migration; the
  diff is bounded to layout-glue shape changes (no content drift in
  rendered text, no numeric or sentiment-color drift). Two consecutive
  `cargo test -p ui` runs against the new baselines are byte-stable
  (determinism gate).
- **V1C — PNG visual baselines unaffected.** The 3 PNG baselines at
  [`crates/ui/tests/visual-baselines/`](../../crates/ui/tests/visual-baselines/)
  render the Charts screen, not positions; expected `cmp -s` PASS against
  pre-migration bytes. The `visual_snapshots.rs` test at
  [`crates/ui/tests/visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs)
  stays green.
- **V1D — Anchor verify PASS.** `scripts/verify_anchors.sh` exits zero;
  the 11 body-SHA-256 anchors in [`spec/anchors.toml`](../anchors.toml)
  stay 11/11 (Brief A touches no report-generation paths).
- **V1E — Docs warning-clean.** `cargo doc -p ui --no-deps` emits zero
  warnings on the new `positions::view` surface.

### V2 — Strategies table migration

Same five sub-criteria as V1, applied to `strategies.rs`:
- **V2A** — `cargo test -p ui --test panel_snapshots strategies` PASS.
- **V2B** — 8 strategies-related `.snap` baselines refresh shape-only
  (5 `strategies_*` + 3 `strategies_screen__sparkline_present` /
  `*sma_crossover_default` / `*empty_state` that include the
  table-glue surface — pause-button + override-modal snapshots stay
  byte-identical since their widgets are not touched). Two-run determinism
  gate.
- **V2C** — PNG baselines unaffected (Charts-only).
- **V2D** — Anchor verify PASS.
- **V2E** — Docs warning-clean.

### V3 — KPI strip grid migration

- **V3A** — `cargo test -p ui --lib widgets::kpi_strip` PASS; the
  in-file `kpi_strip__sample_report` + `kpi_strip__metrics_unavailable`
  insta snapshots at [`crates/ui/src/widgets/snapshots/`](../../crates/ui/src/widgets/snapshots/)
  refresh shape-only.
- **V3B** — Snapshot diff is layout-glue only. The viewer-bin's
  `viewer__full_view__sample_report.snap` panel snapshot at
  [`crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap`](../../crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap)
  also refreshes (it embeds kpi_strip's rendered output). Two-run
  determinism gate.
- **V3C** — PNG baselines unaffected (kpi_strip lives on viewer-bin, not
  Charts screen).
- **V3D** — Anchor verify PASS.
- **V3E** — Docs warning-clean.

### V4 — Journal-transaction modal `float` migration

- **V4A** — `cargo test -p ui` PASS (the four `*_renders_without_panic`
  smoke tests at [`journal_transaction_modal.rs:519-562`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  + any `tape_row_click_opens_modal.rs` integration test stay green).
- **V4B** — The 3 expected modal-related `.snap` baselines refresh
  shape-only (audit-modal loading / empty / error / ready variants —
  enumerate during architect pass). Two-run determinism gate.
  Critically, the three close paths (Escape / click-outside / Close
  button) must each still funnel to `Message::TapeAuditModalClosed` and
  this is asserted by [`crates/ui/tests/tape_row_click_opens_modal.rs`](../../crates/ui/tests/tape_row_click_opens_modal.rs)
  + the modal-keyboard subscription path in `state.rs`.
- **V4C** — PNG baselines unaffected (modal is overlay-on-cockpit, not
  Charts screen; PNG baselines do not currently snapshot a modal-open
  cockpit).
- **V4D** — Anchor verify PASS.
- **V4E** — Docs warning-clean.

## Non-regression contract

Brief A's outer envelope, lifted from the predecessor brief's
[`## Snapshot-impact summary`](../iced-ecosystem-evaluation/feature.md#snapshot-impact-summary)
+ this brief's per-Vn breakdown:

| Surface | Pre-migration | Post-migration target | Failure mode |
|---|---|---|---|
| Workspace tests (`cargo test`) | 1203+ green | 1203+ green (no test added or removed by Brief A) | Any net-new failure routes back to developer |
| Panel `.snap` baselines | 68 total (positions ×6, strategies ×14, audit-modal-related ×4, kpi-via-viewer ×1) | ~20 refresh shape-only; remaining ~48 byte-identical | Content drift in any non-target snapshot → STOP, route to developer (over-broad change) |
| PNG visual baselines | 3 (Charts screen only) | 3 byte-identical | Any PNG diff → STOP, route to ui-designer (Charts surface untouched by Brief A by construction) |
| Anchor verify | 11/11 PASS | 11/11 PASS | Single FAIL → STOP, route to developer (Brief A touches no report-generation path) |
| Widget surface untouched | 18 of 22 widgets | 18 of 22 widgets | Diff in any of the 18 → STOP, scope leak |
| Direct crate deps | `iced = "=0.14.0"`, `iced_test = "=0.14.0"` (dev) | Unchanged | Any add → STOP, route to architect (Brief A is zero-new-dep by construction) |
| Transitive crate count | 34 | 34 | Any new transitive → STOP, scope leak |
| iced Cargo.toml features | `tiny-skia, thread-pool, advanced, canvas` | Unchanged | Any feature add → STOP (markdown feature flag is M5's job, not Brief A's) |

The four migrations are **independently shippable** — a partial Brief A
(e.g. only R1 + R3 land, R2 + R4 deferred) is acceptable per Q2 below.

## Hypothesis register

Analyst seeds; architect ratifies + adds falsifiers per
[`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only).
Each H carries a measurable falsifier the orchestrator runs (NOT a live
display server / GPU / window — `cargo doc`, `cargo build`, `cargo test`,
`grep` only).

### H1 — Native `table` API surface matches the `positions.rs` 7-column shape

- **Statement:** `iced::widget::table::Table` v0.14 accepts a
  `Vec<&PositionView>` (or equivalent owned slice) plus N `Column`
  definitions where each `Column` maps the value `T` → an
  `iced::Element<Message>`. The 7 positions columns (`symbol`, `qty`,
  `cost_basis`, `last_mark`, `pnl`, `pnl_pct`, `exposure`) each
  expressible as a `Column::new(header_str, |p: &PositionView| -> Element
  {...})` lambda. The PNL / PNL_PCT per-cell sentiment color flows
  through the lambda's return value (since the lambda emits a `Text`
  with the chosen color), so no per-cell catalog hook is required.
- **Falsifier:** Orchestrator-direct grep of
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/table.rs`
  for `pub fn column`, `pub struct Column`, `where T:`,
  `Fn(&T) -> Element`. The brief from the orchestrator already confirms
  `pub fn table()`, `pub fn column()`, `pub struct Table`, `pub struct
  Column`, `pub struct Style`, `pub trait Catalog`, padding/separator
  methods exist. **Open question for architect:** does the lambda
  signature accept `&T` or `&mut T` or owned `T`, and does it accept a
  borrow lifetime compatible with our `&'a [PositionView]` source? If the
  lambda requires `'static T`, the migration needs an owned-clone path
  (cost: per-render `Vec<PositionView>` allocation; acceptable at
  bounded row counts) — see Q4.
- **Status:** unresolved. **Default expectation: PASS** based on the
  predecessor brief's architect read of the API surface.

### H2 — Native `grid` accepts the kpi_strip's 6-cell layout

- **Statement:** `iced::widget::grid::Grid` v0.14 exposes a column-count
  parameter (literally `Grid::with_columns(6)` or equivalent) and
  accepts heterogeneous cell contents (each cell is an `Element`,
  not a typed `T`). The kpi_strip's 6 cards (each a `Container<Column
  { label, value }>`) wire in as 6 cell elements in a single 6-column
  grid. Implicit alignment removes the
  `Length::FillPortion(1)`-on-every-container glue at
  [`kpi_strip.rs:175`](../../crates/ui/src/widgets/kpi_strip.rs).
- **Falsifier:** Orchestrator-direct grep of
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/grid.rs`
  for `pub fn columns`, `with_columns`, `column_count`, `pub fn cell`.
  If the API is row-major-element-list with an explicit column count,
  H2 PASS. If the API requires per-cell `(row, col)` coordinates, the
  migration buys less than expected (re-scope R3 to "skip; hand-roll
  `Row` is already 8 lines").
- **Status:** unresolved. **Default expectation: PASS** based on
  predecessor brief's architect cost analysis.

### H3 — Native `float` exposes `on_dismiss` + backdrop hook

- **Statement:** `iced::widget::float::Float` (or equivalent v0.14
  surface name) takes a base element + a floating overlay element + an
  `on_dismiss: Msg` builder method, where dismissal fires when the user
  clicks outside the overlay OR presses Escape (if `float` integrates
  with iced's keyboard subscription) OR the overlay's own close-button
  emits `close_msg`. The backdrop style accepts a `Background` or
  `Color` argument so our `color::OVERLAY` token routes in.
- **Falsifier:** Orchestrator-direct grep of
  `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/float.rs`
  for `on_dismiss`, `on_close`, `Background`, `focus`, `Escape`,
  `keyboard`. The predecessor brief's H-arch-6 covered this; status was
  unresolved. If `on_dismiss` is absent, R4 either (a) keeps the
  hand-rolled backdrop `MouseArea` and uses `float` only for centered
  positioning, OR (b) defers to Brief C (`iced_dialog`) per H-arch-11.
- **Status:** unresolved. **Default expectation: PARTIAL PASS** — float
  likely positions but may not dismiss-on-Escape natively; that path
  stays in the cockpit's keyboard subscription regardless.

### H4 — Theme/`Catalog` interop with `theme::ModeColor` ramp

- **Statement:** `iced::widget::table::Style` + `pub trait Catalog`
  (orchestrator-confirmed in the brief) integrate with our
  `theme::Catalog` impls — i.e. our `color::PANEL` / `color::FG_1` /
  `color::ACCENT` tokens flow into native table/grid/float styling
  without an adapter layer. The cockpit's existing `theme::ModeColor`
  ramp (declared in `crates/ui/src/theme/mod.rs`) is already a
  `iced::theme`-compatible surface per the Lumen Phase 1-5 ship-record.
- **Falsifier:** Orchestrator runs `cargo doc -p iced --no-deps
  --features "tiny-skia,thread-pool,advanced,canvas"` and inspects
  `iced::widget::table::Catalog`'s required methods. If the trait's
  associated types or `Style`-builder signatures differ from our
  existing `theme::Catalog` impl shape, a thin adapter wrapper is
  required (architect estimates ~30 LOC per widget; tolerable).
- **Status:** unresolved.

### H5 — Row click in native `table` dispatches `Message::SelectStrategy`

- **Statement (R2-specific):** `iced::widget::table::Table` v0.14 exposes
  either (a) an `on_row_click: Fn(&T) -> Msg` builder method, OR (b) the
  ability to wrap each row's content in a `Button` inside the `Column`'s
  body lambda. The latter is strictly possible (lambdas return
  `Element`, and `Button::new(...).on_press(...).into()` is an
  `Element`); the question is whether the per-cell wrapping breaks
  table layout alignment (column widths derived from cell sizes).
- **Falsifier:** Orchestrator-direct grep
  `iced_widget-0.14.2/src/table.rs` for `on_press`, `on_click`,
  `on_row_click`, `Callback`. If absent, fallback to per-cell `Button`
  wrap inside each Column lambda. Architect runs a 50-LOC spike to
  confirm column-width derivation handles `Button`-wrapped cells without
  alignment loss.
- **Status:** unresolved. **Default expectation: NO on_row_click hook**
  — table widgets in immediate-mode UIs typically don't ship row-click
  callbacks; we'll need the Button-per-cell fallback.

## Open questions for architect

Be specific. Architect picks which to falsify before dev fan-out; some are
operator-orthogonal (architect-decide), some may need predecessor brief
re-reading.

- **Q1 — Snapshot strategy: regenerate ALL panel_snapshots in one pass
  or per-widget?** The predecessor brief's snapshot-impact summary
  estimated ~20 panel snapshots refresh across Brief A. **Recommended:
  per-widget refresh** — gives one bisectable git commit per migration,
  makes a partial Brief A (R1+R3 only, R2+R4 deferred) shippable. The
  alternative (one giant refresh commit) collapses bisection. Architect
  ratifies.

- **Q2 — Test ordering: inter-dep among R1/R2/R3/R4?** Analyst's read:
  **NONE** — the four widgets do not share types, do not co-render in
  any panel snapshot (positions and strategies render in different
  panels per the Phase 4 layout; kpi_strip is viewer-bin-only;
  journal-modal is overlay-on-cockpit). Dev fan-out per widget (R1, R2,
  R3, R4 as parallel sub-agents) is safe. **Confirm or override.**

- **Q3 — Style/theme integration: do iced 0.14 native widgets accept
  the cockpit's `theme::Catalog` impls or do we need style wrappers?**
  Maps to H4 above. Architect runs the falsifier; if H4 falsifies, dev
  task list grows by 1-2 adapter tasks per widget (R1: positions table
  catalog adapter; R3: kpi grid catalog adapter; R4: float backdrop
  style adapter).

- **Q4 — Does `iced::widget::table::Table` borrow or own its rows?**
  Maps to H1's open question. `positions.rs::view()` currently consumes
  `&Cockpit` (borrowed source slice). If `Table::new(items: Vec<T>)`
  requires owned `T`, the migration adds a `.cloned().collect()` step
  per render — cost is bounded (positions ≤symbol-count rows, strategies
  ≤20 rows per predecessor brief Q1 rationale). Architect confirms via
  direct `table.rs` grep for `Vec<T>` vs `&[T]` constructors.

- **Q5 — Row click dispatch in R2: `on_row_click` vs `Button`-per-cell
  vs `Button`-wrapping-Row?** Maps to H5 above. Architect runs the
  falsifier; if no `on_row_click` hook exists, dev's R2 migration adds
  a layer to wrap each row body in `Button::new(row_content).on_press(...)`
  inside the cell lambda — but this loses the table's column-width
  alignment unless `Button` content participates in `Table`'s width
  derivation (which is the architect's empirical concern).

- **Q6 — Per-row error badge in R2: rendered as a separate
  `Table::row` or as a wrapped pair in a single cell?** At
  [`strategies.rs:77-79`](../../crates/ui/src/widgets/strategies.rs) the
  error-badge row is `push`ed as a sibling row in the outer `Column`
  beneath the main `Row`. Native `table` may not accept a heterogeneous
  row stream (every row maps to the same `Column` set). Options:
  - Option A — error-badge stays as a follow-up `Column` row INSIDE the
    table cell's lambda (loses error-row's horizontal bleed across all 6
    columns).
  - Option B — error-badge migrates to a footer-style row outside the
    table (loses its visual coupling to the parent strategy row).
  - Option C — table renders only the main rows; error-badges render in
    a sibling `Column<error_badges>` below the table (loses row-level
    proximity entirely).
  - Option D — wait for iced to expose row-expansion / per-row tail
    rendering (predecessor brief did not surface such an API).
  Architect picks; if no option preserves all three constraints
  (alignment + bleed + proximity), R2 falls back to hand-roll and
  Brief A ships R1 + R3 + R4 only.

- **Q7 — `float` keyboard-Escape integration in R4: does iced 0.14's
  `float` participate in the keyboard subscription pipeline, or does
  the Escape handler stay in `state.rs`'s subscription?** Maps to H3.
  Default expectation: subscription stays in `state.rs`; `float` only
  handles click-outside dismissal.

## Notes / out of scope

Explicit non-goals for Brief A:

- **`agent_feed.rs` migration** — HELD per architect Q1 / H-arch-7. See
  [predecessor brief Q1 resolution](../iced-ecosystem-evaluation/feature.md#design--architect-synthesis).
- **`chart_tooltip.rs` migration** — HELD per architect Q3 / H-arch-2.
  Canvas-internal dispatch; lifting tooltip out of canvas is a separate
  brief.
- **In-cockpit `markdown` viewer (M5)** — operator-approved (Q-O2 =
  ADOPT) but scoped to a separate brief slot between A and B. Requires
  enabling the `markdown` iced feature flag (currently OFF — declared
  but inactive per M0 falsifier fingerprint JSON evidence).
- **`iced_aw` cherry-pick (Brief B)** — separate brief.
- **`iced_dialog` chrome (Brief C)** — gated on H-arch-6 / H3
  falsification.
- **`plotters-iced2` SPIKE (Brief D)** — research-only, post-Brief-A.
- **Performance benchmarks** — no `criterion` runs from Brief A.
  Positions / strategies / kpi / journal-modal are bounded-row /
  bounded-cell surfaces; perf is not a Brief A concern.
- **Anchors changes** — Brief A touches no report-generation path; the
  11 anchors stay 11/11.
- **Cargo.toml edits** — Brief A is zero-new-dep AND zero-feature-flag
  by construction.

## Design — architect synthesis

> **Architect pass, 2026-05-13.** Resolves Q1-Q7 against the analyst's
> brief; authors falsifiable hypotheses for every load-bearing call (per
> [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only));
> ratifies the 4-lane fan-out; lays down `tasks.md` with concrete T-tasks
> per the [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)
> tester / evaluator split (this feature ships after 2026-05-12, so the new
> split applies). **No code changes, no crate adds.**

### Sandbox capability surface (load-bearing for falsifier evidence)

Architect's sub-agent sandbox blocks `Read` and `Bash` against
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/`
(verified: `Read` and `find/grep/ls` all return "Permission denied" on
that path). Substituted: build-artifact evidence from
[`target/debug/deps/iced_widget-e0d51c2a6d696d3b.d`](../../target/debug/deps/iced_widget-e0d51c2a6d696d3b.d)
(.d manifest — confirms `table.rs`, `grid.rs`, `float.rs` are all
compiled into the present iced_widget-0.14.2 build under
`features=["advanced","canvas"]`; M0 sub-agent's predecessor result
inherited verbatim) + the orchestrator-confirmed `table.rs` API surface
inherited from the analyst brief
([`## Predecessor + decisions inherited` H-arch-7 row](#predecessor--decisions-inherited)):
`pub fn table()`, `pub fn column()`, `pub struct Table`,
`pub struct Column`, `pub struct Style`, `pub trait Catalog`, padding/
separator methods present; ZERO matches for `with_offset|virtual|lazy|skip|row_budget|visible_range|from_iter|Lazy|with_capacity_hint`;
`Table::new(items: Vec<T>)` is the constructor signature. **Open
orchestrator-direct grep requests** are surfaced inline at each Q-resolution
below (Q3 / Q4 / Q5 / Q6 / Q7 all flagged); the orchestrator may resolve
them before dev fan-out OR may spawn the developer with conditional
branches in the T-tasks.

### Theme-integration ground truth (load-bearing for Q3 / H-arch-A4)

Architect grepped `crates/ui/src/theme.rs` (1300 LOC) for `Catalog`,
`StyleFn`, `theme::Style`, `Theme::`, `style_fn` — **zero matches**. The
cockpit's theme surface is a **per-token-color module** (`color::FG_1`,
`color::PANEL`, `color::ACCENT`, `color::OVERLAY`, etc., each a
`ModeColor` struct with `.current(ThemeMode)` returning an `iced::Color`).
Widget styling is **closure-based** at every call site — e.g.
[`journal_transaction_modal.rs:125`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
`.style(|_theme: &iced::Theme| container::Style { background: Some(color::OVERLAY.current(ThemeMode::Dark).into()), .. })`.
The cockpit **does NOT** impl `theme::Catalog` traits anywhere — it routes
tokens directly into per-widget `Style` structs via closures. This makes
Q3 / H-arch-A4 a **non-issue**: native `iced::widget::table`,
`iced::widget::grid`, `iced::widget::float` all accept the same closure
pattern (per iced 0.14's `.style(|theme: &Theme| Style {...})` convention
already used across the cockpit's 22 widgets). No adapter wrappers needed.

### Q-resolutions (Q1-Q7)

- **Q1 (snapshot regen strategy) — RESOLVE: per-widget refresh, one
  bisectable commit per migration.** Confirms analyst recommendation.
  Rationale: bisectability beats one-pass review burden at this scope
  (~20 snapshots across 4 widgets). A partial Brief A (e.g. R1 + R3
  land, R2 + R4 deferred) stays shippable; one-shot refresh collapses
  bisection AND blocks partial shipping. Each developer lane runs
  `cargo insta accept` scoped to ITS widget's snapshot files only.

- **Q2 (4-lane dev fan-out safety) — RESOLVE: 4 parallel lanes
  (R1=positions, R2=strategies, R3=kpi_strip, R4=journal_modal),
  one sub-agent per widget.** Confirms analyst recommendation.
  Inter-dep audit:
  - R1 + R2 share no types beyond `Element` / `PanelState` /
    `Message` (positions consumes `PositionView`, strategies consumes
    `StrategyRow`); no shared snapshot file.
  - R3 lives in viewer-bin (`ViewerMessage`), zero cockpit `Message`
    overlap with R1/R2/R4.
  - R4 is overlay-on-cockpit (consumes `Message`); shares no widget
    source file with R1/R2/R3.
  - No file edited by ≥2 lanes; `Cargo.toml` is read-only across all 4.
  - **Operator pre-condition lifted by architect:** Q3 / H-arch-A4
    falsifies positively (theme is closure-based, no adapter needed);
    the analyst's "Q3 is a pre-condition for ALL four migrations"
    note in [tasks.md ## Notes](tasks.md#notes) is therefore RELAXED
    — the 4 lanes spawn in parallel.

- **Q3 (theme `Catalog` interop) — RESOLVE: NO adapter wrappers needed.**
  Maps to H-arch-A4. The cockpit's `theme.rs` (1300 LOC) carries
  ZERO `impl Catalog for ...` blocks (architect-verified by direct
  grep). Styling is exclusively via per-call-site `.style(|theme:
  &Theme| Style { ... })` closures that route `color::*.current(mode)`
  into the appropriate `Style` field. Native `table::Style` / `grid::Style` /
  `float::Style` (each declared `pub struct Style` per the
  orchestrator-confirmed `table.rs` grep + parallel expectation for
  `grid.rs` / `float.rs`) accept the same closure shape — `.style(|theme|
  table::Style { ... })`. **No additional T-tasks for Catalog adapter
  work.** Falsifier (H-arch-A4 below) gates the assumption that
  `grid::Style` and `float::Style` exist with closure-compatible builder
  signatures; if EITHER falsifies, dev's R3 or R4 T-task list grows by
  +1 (adapter-write task, ≤30 LOC estimate per widget per analyst H4).

- **Q3-sub (refinement pass 2026-05-13, T-M0-K) — RESOLVED PARTIAL.** The
  closure-routing assumption holds for **`Float` only**: `Float::style(impl Fn(&Theme) -> Style + 'a)`
  is confirmed. **`Table`** has `pub struct Style { separator_x: Background, separator_y: Background }`
  + `pub trait Catalog` + `pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>`
  BUT **no `pub fn style()` method on the Table builder.** Visible Table
  builders: `width`, `padding`, `padding_x`, `padding_y`, `separator`,
  `separator_x`, `separator_y`. The canonical theming path is
  `impl iced::widget::table::Catalog for iced::Theme`. **`Grid`** has NO
  `Style` struct, NO `Catalog` trait, NO `style()`, NO `class()` —
  defaults only.

  **Architect decision (Q3-sub): adopt option (b) — `impl iced::widget::table::Catalog for iced::Theme`
  in a new `crates/ui/src/theme/iced_widget_catalogs.rs` module.** Rationale:
  centralized theme-routing module future-proofs us for any further native
  iced widgets adopting the Catalog pattern (Brief B `iced_aw` cherry-pick
  may surface more). The closure-everywhere lemma in main `theme.rs` is
  preserved; the `Catalog` impl is a thin adapter (~30 LOC) that calls
  `Style { separator_x: color::DIVIDER.current(mode).into(), separator_y: color::DIVIDER.current(mode).into() }`
  and routes through StyleFn. Grid takes defaults; visual chrome stays
  in the outer PANEL container.

  **Cost:** +1 T-task to M2 (or shared M0): write the Table Catalog impl
  (~30 LOC) in `crates/ui/src/theme/iced_widget_catalogs.rs`. Lane 1 (R1
  positions) and Lane 2 (R2 strategies) both depend on it (~serial gate
  if architect picks Lane 1 to own it; Lane 2 reads). Alternative
  consideration v0.1 quick-win: option (a) accept Table defaults — but
  separator/padding drift vs hand-rolled is guaranteed in snapshot diffs.
  Picking (b) bounds drift to a single deliberate Style.

- **Q4 (`Table::new` ownership) — RESOLVE: owned `Vec<T>` constructor;
  dev cooks a `.iter().cloned().collect::<Vec<_>>()` per render.**
  Inherited from analyst brief's orchestrator-confirmed `table.rs`
  grep: `Table::new(items: Vec<T>)`. Cost is bounded:
  - R1 positions: filtered `Vec<&PositionView>` already allocated per
    render at [`positions.rs:31-33`](../../crates/ui/src/widgets/positions.rs)
    — `Vec::iter().cloned().collect()` adds one shallow clone per
    visible position (≤symbol-count rows, typically <20).
  - R2 strategies: `&[StrategyRow]` consumed once per render — same
    pattern (≤20 strategies per predecessor brief Q1 rationale).
  `PositionView` and `StrategyRow` are `Clone` (both end-to-end
  derive-`Clone` per their definition sites). Migration adds one
  allocation per render per table; perf is not a Brief A concern (per
  [`## Notes / out of scope`](#notes--out-of-scope)). **Falsifier
  H-arch-A2 below confirms `Clone` is implementable for the borrow
  pattern; if `Table::new` actually requires `'static T`, an owned-copy
  path is the same shape — no T-task change.**

- **Q5 (row-click dispatch in R2) — RESOLVE: Button-per-row inside
  column 1's body lambda; this IS the path (no fallback).** Maps to
  H-arch-A5 + H-arch-A5b. T-M0-L (orchestrator grep batch 2026-05-13)
  CONFIRMED zero row-click hooks in `table.rs` (`row_decorator|after_row|tail|on_row|row_overlay`
  all zero matches). The migration **commits** to:
  - Column 1's body lambda emits `Button::new(cell_id).on_press(Message::SelectStrategy(r.id.clone()))`.
  - Columns 2-6 stay plain `Element` cells. Mouse-event bubbling on
    the underlying Button-wrapped cell handles click-through.

  Snapshot diff in the M2 lane will reveal any column-edge misalignment;
  if observed, the lane escalates to architect (NOT a free fallback).
  H-arch-A5 keeps its "two-run determinism + snapshot-shape" falsifier
  but the "Option B fallback" branch is removed from M2 — it was an
  artifact of the un-resolved H-arch-A5b grep. The conditional task
  T2.3 (Option B fallback wrap-all-6-cells) is dropped from tasks.md.

- **Q6 (R2 per-row error badge) — RESOLVE: Option C (sibling
  `Column<error_badges>` below the table). COMMITTED — no Option-A
  branch.** Maps to H-arch-A5b. T-M0-L (orchestrator grep batch
  2026-05-13) CONFIRMED zero matches for `row_decorator|after_row|tail|on_row|row_overlay`
  in `table.rs`. Option A (badge inside cell lambda) is mechanically
  impossible — there is no per-row tail API. Option B (footer-style row)
  loses visual coupling. Option D defers indefinitely. **Option C IS the
  ship path.** The error-badge moves to a sibling `Column<error_badges>`
  below the `Table` within the same outer `Column::new().spacing(space::XS)`
  parent. Proximity tradeoff accepted per:
  (a) error states are rare (per `StrategyStatus::Error(_)`);
  (b) sibling-Column preserves left-edge alignment with the table's
  first column;
  (c) snapshot baseline refreshes shape-only — proximity diff is bounded.

- **Q7 (R4 `float` keyboard-Escape integration) — RESOLVE: Escape
  stays in `state.rs` subscription path. CONFIRMED (no branch).**
  Maps to H-arch-A7 + H-arch-A7b. T-M0-N (orchestrator grep batch
  2026-05-13) CONFIRMED zero matches for
  `on_dismiss|on_close|on_outside_click|Background|backdrop|focus_trap|keyboard::|on_key|Escape|key_press|subscription`
  in `float.rs`. `Float` is **positioning-only**. The cockpit's
  existing Escape wiring at `state.rs` (T1206) stays in place. R4's
  click-outside dismissal **must** keep the hand-rolled `MouseArea`
  backdrop at [`journal_transaction_modal.rs:118-131`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (architect-verified: `MouseArea::new(Space).on_press(close_msg)` at
  line 130). R4 wires `Float::new(stack_with_backdrop, card)` where
  `stack_with_backdrop = Stack::new().push(content).push(backdrop_layer(close_msg.clone()))`.
  The conditional T4.2 fallback in tasks.md collapses to a single
  committed shape — see tasks.md M4 updates.

### Hypothesis register update (architect, 2026-05-13)

Per [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only),
each H carries an orchestrator-runnable falsifier (NO display server /
GPU / live window — `cargo doc`, `cargo build`, `cargo test`, `grep`
only). Numbering: `H-arch-A1` through `H-arch-A7` (the `A` prefix
distinguishes Brief-A architect hypotheses from analyst seeds `H1-H5`
and predecessor `H-arch-0` through `H-arch-11`).

**H-arch-A1 (architect, 2026-05-13; absorbs analyst H1 + predecessor
H-arch-1) — Snapshot diffs across R1/R2/R3/R4 migrations are
byte-stable across two consecutive `cargo test -p ui` runs after the
one-shot `cargo insta accept` baseline refresh.**
- *Statement:* The four migrations produce equivalent rendered output
  modulo expected one-shot baseline refresh. Two-run determinism
  verifies native widgets carry no internal non-determinism (no
  wall-clock reads, no `HashMap` iteration drift in cell ordering).
- *Falsifier:* Dev lane runs the widget's migration → `cargo insta
  accept` scoped to that widget's `.snap` files → `cargo test -p ui
  --test panel_snapshots <widget>` TWICE in succession → `git diff
  --quiet` the snapshot files between runs. If any byte differs →
  FALSIFIED, STOP this lane, escalate to architect (likely upstream
  iced determinism issue).
- *Status:* unresolved per-lane (one resolution per R1/R2/R3/R4).

**H-arch-A2 (architect, 2026-05-13; resolves Q4 borrow vs own) —
`iced::widget::table::Table::new` takes `IntoIterator<Item = T>` where
`T: Clone`, accepting our `Vec<PositionView>` / `Vec<StrategyRow>`; both
end-to-end `derive(Clone)`.**
- *Statement (refinement pass 2026-05-13, T-M0-J):* Actual signature is
  more flexible than initially framed:
  ```rust
  pub fn new<'b, T>(
      columns: impl IntoIterator<Item = Column<'a, 'b, T, Message, Theme, Renderer>>,
      rows: impl IntoIterator<Item = T>,
  ) -> Self where T: Clone,
  ```
  `IntoIterator<Item = T>` is more permissive than `Vec<T>` — dev may
  pass `Vec<T>`, `[T; N]`, or any iterator yielding owned `T`. The
  `T: Clone` bound stands. Architect-confirmed (refinement pass) that
  `PositionView` is `derive(Debug, Clone, Serialize, Deserialize)` at
  [`crates/core/src/views.rs:98-99`](../../crates/core/src/views.rs)
  and `StrategyRow` is `derive(Debug, Clone)` at
  [`crates/ui/src/state.rs:535-536`](../../crates/ui/src/state.rs).
  Both fit the bound without further derives.
- *Falsifier:* `cargo build -p ui` after migration succeeds (compile-
  time verification of the `IntoIterator<Item = T>` + `T: Clone` bound).
  Two-run determinism via H-arch-A1.
- *Status:* **REFINED (orchestrator T-M0-J, 2026-05-13).** Constructor
  signature confirmed; `T: Clone` bound confirmed; both target types
  satisfy `Clone`. Migration shape locked: dev calls
  `Table::new(columns_vec, rows_iter)` where `rows_iter` may be either
  `rows.iter().cloned()` (yields owned `T`) or
  `rows.iter().cloned().collect::<Vec<_>>()` (extra alloc, no
  correctness diff). Lane developer picks the cheaper form.

**H-arch-A3 (architect, 2026-05-13; absorbs analyst H2) — Native
`iced::widget::grid::Grid` v0.14 exposes a 6-cell row-major layout
API (column-count parameter + Element-list rows) sufficient for
kpi_strip.rs's 6-card layout.**
- *Statement (refinement pass 2026-05-13, T-M0-K):* Grid API surface
  confirmed in full:
  - Constructors: `Grid::new()`, `Grid::with_capacity(n)`,
    `Grid::with_children(...)`, `Grid::from_vec(Vec<Element>)`.
  - Layout: `Grid::columns(n: usize)` (fixed column count), `Grid::fluid(max_width)` (responsive).
  - Sizing: `Grid::spacing(pixels)`, `Grid::width(pixels)`, `Grid::height(Sizing)`.
  - Population: `Grid::push(child)`, `Grid::push_maybe(opt_child)`, `Grid::extend(iter)`.
  Each cell is an `Element` — accepts heterogeneous content. The
  kpi_strip 6-card layout maps to `Grid::new().columns(6).push(card1).push(card2)...push(card6)`
  — clean fit.
- *Falsifier (RESOLVED-UNFALSIFIED):* T-M0-K grep confirmed the API
  surface above. R3 ships against `Grid::new().columns(6)` shape.
  Two-run determinism via H-arch-A1 still applies.
- *Status:* **RESOLVED-UNFALSIFIED (orchestrator T-M0-K, 2026-05-13).**

**H-arch-A4 (architect, 2026-05-13; resolves Q3) — Native
`table::Style`, `grid::Style`, `float::Style` all accept the
cockpit's closure-style `.style(|theme: &Theme| Style { ... })`
pattern; no `Catalog`-impl adapter wrapper is required.**
- *Statement (refinement pass 2026-05-13, T-M0-K — PARTIAL FALSIFICATION):*
  Closure-routing holds for ONE of three widgets:
  - **`Float`** — ✅ `Float::style(impl Fn(&Theme) -> Style + 'a)` confirmed.
    Closure-routing fits the cockpit pattern.
  - **`Table`** — ⚠️ has `pub struct Style { separator_x: Background, separator_y: Background }`
    + `pub trait Catalog` + `pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>`
    BUT **NO `pub fn style()` method on the Table builder.** Visible
    builders: `width`, `padding`, `padding_x`, `padding_y`, `separator`,
    `separator_x`, `separator_y`. The canonical theming path is
    `impl iced::widget::table::Catalog for iced::Theme`. **Architect
    decision (option b):** write the Catalog impl in
    `crates/ui/src/theme/iced_widget_catalogs.rs`.
  - **`Grid`** — ❌ has NO `Style` struct, NO `Catalog`, NO `style()`,
    NO `class()`. Inherits container-style spacing only. **Architect
    decision:** accept defaults; visual chrome stays in the outer PANEL
    `Container`.
- *Status:* **RESOLVED-PARTIAL-FALSIFIED (orchestrator T-M0-K, 2026-05-13).**
  - Float: PASS, closure-style as predicted.
  - Table: FALSIFIED in closure-style; PASS via Catalog. Adds +1 T-task
    to M2 (Table Catalog adapter, ~30 LOC).
  - Grid: FALSIFIED across all three of `style`/`StyleFn`/`Style`; PASS
    by accepting defaults. No T-task addition; bounded visual drift
    accepted at snapshot refresh.

**H-arch-A5 (architect, 2026-05-13; absorbs analyst H5; resolves Q5)
— Native `table` does not expose `on_row_click`; `Button`-wrapping
column 1's body lambda (current cockpit pattern, preserved) routes
`Message::SelectStrategy` without breaking `Table`'s column-width
derivation.**
- *Statement:* No `on_row_click` callback in `table.rs` (inherited
  grep evidence). The migration places the `Button::new(cell_content).on_press(Message::SelectStrategy(r.id.clone()))`
  inside column 1's body lambda; columns 2-6 stay plain `Element`
  cells. `Button`'s width participates in iced's standard
  width-derivation (same as any other Element).
- *Falsifier:* Dev lane's R2 migration runs the cargo test snapshot
  suite. If column widths visibly mis-align (snapshot diff shows
  column edges shifted by >0 px), falsified → fall back to
  **Option-B** (wrap ALL 6 cells in parallel Buttons that emit the
  same `Message::SelectStrategy`). Cost: +1 T-task (R2.fallback);
  zero scope leak.
- *Status:* unresolved; resolves at end of R2 lane.

**H-arch-A5b (architect, 2026-05-13; resolves Q6) — Native `table`
does NOT expose `row_decorator` / `after_row` / `tail` / per-row
follow-up API; sibling `Column<error_badges>` below the table is
the only preserves-row-bleed option.**
- *Falsifier (RESOLVED-CONFIRM):* T-M0-L (orchestrator grep batch
  2026-05-13) — ZERO matches in `table.rs` for `row_decorator|after_row|tail|on_row|row_overlay`.
  Button-in-column-1 is the only row-click path; sibling
  `Column<error_badges>` is the only error-badge layout. Q5/Q6
  resolutions both lock to the single committed shape (no fallback
  branches in M2).
- *Status:* **RESOLVED-CONFIRM (orchestrator T-M0-L, 2026-05-13).**

**H-arch-A6 (architect, 2026-05-13; corrects orchestrator's earlier
4-card framing) — `kpi_strip.rs` ships SIX cards (not 4): Total
Return / CAGR / Sharpe / Max DD / Win Rate / Trades, per file
[`kpi_strip.rs:81-130`](../../crates/ui/src/widgets/kpi_strip.rs)
and the `unavailable_strip` mirror at
[`kpi_strip.rs:138-149`](../../crates/ui/src/widgets/kpi_strip.rs).
Native `grid`'s 6-column template absorbs this without per-card
width hints.**
- *Falsifier:* Orchestrator-direct read of `kpi_strip.rs:123-132`
  (this Brief's primary widget file, already in our access scope):
  `grep -n "\.push(" kpi_strip.rs | wc -l` against the in-`view()`
  cells (expect 6 `.push(card_*)` lines in the main strip and 6
  iterations in `unavailable_strip`). VERIFIED on architect read of
  [`kpi_strip.rs:123-132`](../../crates/ui/src/widgets/kpi_strip.rs)
  — six `.push(...)` calls (total_return, cagr, sharpe, max_dd,
  win_rate, trades) and a 6-element `labels[]` loop at line 138-149.
  **PASS (architect-confirmed pre-dev).**
- *Status:* **RESOLVED-UNFALSIFIED (architect, 2026-05-13).**
  H-arch-A6 is a confirm-only inheritance correction, not a blocker.

**H-arch-A7 (architect, 2026-05-13; absorbs analyst H3; resolves Q7)
— Native `iced::widget::float::Float` is **positioning-only** in
v0.14: no `on_dismiss`, no `on_close`, no `on_outside_click`, no
`Background` / backdrop hook, no `focus_trap`. Brief A keeps the
hand-rolled `MouseArea` backdrop for click-outside dismissal AND
keeps Escape in `state.rs` subscription.**
- *Statement (refinement pass 2026-05-13, T-M0-M — FALSIFIED):* T-M0-M
  orchestrator grep batch — ZERO matches in `float.rs` for
  `on_dismiss|on_close|on_outside_click|Background|backdrop|focus_trap`.
  Native `float` is the positioning primitive only (centers the modal
  card). The three close paths fan out as:
  - **Escape** — `state.rs` keyboard subscription (T1206 path) →
    `Message::TapeAuditModalClosed`. Unchanged by Brief A.
  - **Click-outside** — KEEP `backdrop_layer` (`MouseArea::new(Space).on_press(close_msg)`
    at [`journal_transaction_modal.rs:130`](../../crates/ui/src/widgets/journal_transaction_modal.rs),
    architect-verified refinement pass) as a sibling layer inside
    `Float`'s base element. Compose as `Float::new(Stack::new().push(content).push(backdrop_layer(close_msg)), card)`.
  - **Close button** — header `Button::on_press(close_msg)` unchanged.
  Focus trap not provided; Tab-escape from modal accepted per current
  iced 0.14 limitation (no regression vs hand-rolled).
- *Status:* **RESOLVED-FALSIFIED (orchestrator T-M0-M, 2026-05-13).**
  R4 ships with `Float` for positioning only. M4 task list collapses
  to a single committed shape: T4.1 + T4.3 retain; T4.2 fallback
  removed — replaced by an explicit T-task that documents the
  hand-rolled backdrop sibling composition.

**H-arch-A7b (architect, 2026-05-13; resolves Q7-keyboard-side)
— Native `float` does NOT participate in iced's keyboard
subscription pipeline; the Escape handler stays in `state.rs`'s
subscription.**
- *Falsifier (RESOLVED-FALSIFIED):* T-M0-N (orchestrator grep batch
  2026-05-13) — ZERO matches in `float.rs` for
  `keyboard::|on_key|Escape|key_press|subscription`. `Float` has no
  keyboard participation whatsoever. Escape path stays in `state.rs`
  subscription (T1206) — confirmed as the only viable wiring.
- *Status:* **RESOLVED-FALSIFIED (orchestrator T-M0-N, 2026-05-13).**
  Falsified in the strict sense (`float` was hypothesized to *possibly*
  participate; it does not), which **confirms** the architectural
  decision: subscription owns Escape. No T-task changes required.

### Adoption order within Brief A

**4-lane fan-out, all four sub-agents spawn in parallel.** Confirms
analyst recommendation (Q2). Lane assignments:

| Lane | Widget | Sub-agent | Files touched | T-tasks |
|---|---|---|---|---|
| Lane 1 | R1 — positions table | developer-1 | [`crates/ui/src/widgets/positions.rs`](../../crates/ui/src/widgets/positions.rs), [`crates/ui/src/widgets/snapshots/`](../../crates/ui/src/widgets/snapshots/), [`crates/ui/tests/snapshots/panel_snapshots__positions_*.snap`](../../crates/ui/tests/snapshots/) | M1 |
| Lane 2 | R2 — strategies table | developer-2 | [`crates/ui/src/widgets/strategies.rs`](../../crates/ui/src/widgets/strategies.rs), [`crates/ui/src/widgets/snapshots/`](../../crates/ui/src/widgets/snapshots/), [`crates/ui/tests/snapshots/panel_snapshots__strategies_*.snap`](../../crates/ui/tests/snapshots/) | M2 |
| Lane 3 | R3 — kpi_strip grid | developer-3 | [`crates/ui/src/widgets/kpi_strip.rs`](../../crates/ui/src/widgets/kpi_strip.rs), [`crates/ui/src/widgets/snapshots/`](../../crates/ui/src/widgets/snapshots/), [`crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap`](../../crates/ui/tests/snapshots/) | M3 |
| Lane 4 | R4 — journal_modal float | developer-4 | [`crates/ui/src/widgets/journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs), modal-related `.snap` baselines, [`crates/ui/tests/tape_row_click_opens_modal.rs`](../../crates/ui/tests/tape_row_click_opens_modal.rs) | M4 |

**Sequencing pre-condition (relaxed):** the analyst's
[`tasks.md ## Notes` "Q3 is a pre-condition for ALL four migrations"
clause](tasks.md#notes) is **RELAXED** by Q3 / H-arch-A4
resolution above (theme is closure-based, no adapter ladder). The
4 lanes spawn truly in parallel — no inter-lane handoff required.

**Falsifier-first opportunity (orchestrator-side):** if the
orchestrator has shell access to `~/.cargo/registry/`, running
H-arch-A3 / H-arch-A4 / H-arch-A5b / H-arch-A7 / H-arch-A7b greps
**before** dev fan-out lets each lane carry a pre-resolved Q-set
(removes branching from the dev T-tasks). Architect surfaces this
as an opportunity, not a requirement — the dev T-tasks below carry
explicit fallback branches per Q-resolution so the lanes are
self-contained regardless.

**Refinement pass 2026-05-13:** the orchestrator DID run the 5-grep
batch (T-M0-J through T-M0-N). Falsifier evidence is in. M2/M3/M4
conditional branches collapse to single committed shapes per the
hypothesis register updates and Q3-sub/Q5/Q6/Q7 lock-downs. See the
2026-05-13 refinement-pass changelog entry below.

### Milestone shape (M0 / M1 / M2 / M3 / M4 / M_FINAL)

- **M0** — Architect's design pass (this section + Q-resolutions +
  hypothesis register). Most M0 ticks are recorded here; orchestrator-
  side falsifier passes (H-arch-A3 / A4 / A5b / A7 / A7b) are
  orchestrator-owned and tick at orchestrator-confirm time.
- **M1** — R1 positions table migration (Lane 1).
- **M2** — R2 strategies table migration (Lane 2).
- **M3** — R3 kpi_strip grid migration (Lane 3).
- **M4** — R4 journal_modal float migration (Lane 4).
- **M_FINAL** — per [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split)
  (feature ships after 2026-05-12 so the split applies):
  - **M_FINAL_TEST_RUN** (test-runner; write-allowed) — runs
    `rust-build` + `rust-test` + `rust-validate` + `verify_anchors`
    against the merged 4-lane branch; dumps to
    `spec/iced-native-widgets/reports/test-run-<ts>.log`. No verdict.
  - **M_FINAL_EVAL** (evaluator; read-only, fresh context) — reads
    the run log + all 4 lane diffs + cited snapshot artifacts;
    emits `spec/iced-native-widgets/reports/evaluation-<ts>.md`
    with `VERDICT → PASS / FAIL / REGRESSION`. PASS routes to
    presenter; FAIL routes back to the named lane's developer;
    REGRESSION routes to architect (this section).

### Operator-input questions surfaced

**Zero.** All Q1-Q7 are architect-decide per the brief's
"resolve each in the new `## Design` section" instruction. The
orchestrator-direct grep requests flagged inline at Q3/Q5/Q6/Q7
(falsifying H-arch-A3/A4/A5b/A7/A7b) are **orchestrator-routed**,
not operator-routed — they are infrastructure / sandbox boundary
questions, not product / design choices.

## Implementation

_Architect-decide. See [`tasks.md`](tasks.md) for the M0-M4 +
M_FINAL_* enumerated T-tasks per the 4-lane fan-out plan in
[`## Design — architect synthesis ## Adoption order within Brief A`](#adoption-order-within-brief-a)._

## Verification

_tester links to reports here once any of R1-R4 ships. The brief itself
has no test — its verification is architect approval of the candidate
mappings + the hypothesis register's falsifier outcomes._

## Changelog

- 2026-05-13 (architect, refinement pass): Absorbed orchestrator's
  T-M0-J through T-M0-N grep evidence batch against
  `iced_widget-0.14.2/src/{table,grid,float}.rs`. Hypothesis register
  flips: **H-arch-A2 REFINED** (`Table::new` accepts `IntoIterator<Item = T>`,
  more permissive than initial `Vec<T>` framing; `T: Clone` bound
  confirmed; `PositionView` + `StrategyRow` both `derive(Clone)`
  end-to-end). **H-arch-A3 RESOLVED-UNFALSIFIED** (Grid API surface
  confirmed: `Grid::new().columns(6).push(...)` shape fits kpi_strip
  cleanly). **H-arch-A4 RESOLVED-PARTIAL-FALSIFIED** (Float = closure-style
  PASS; Table = no closure-style `.style()` builder, requires `impl Catalog`;
  Grid = no theme surface at all). **H-arch-A5b RESOLVED-CONFIRM** (zero
  row-click hooks in `table.rs`; Button-in-column-1 is the only path).
  **H-arch-A7 RESOLVED-FALSIFIED** (Float is positioning-only — no
  `on_dismiss`, no backdrop, no focus_trap; hand-rolled `MouseArea`
  backdrop stays). **H-arch-A7b RESOLVED-FALSIFIED** (Float has zero
  keyboard participation; `state.rs` subscription owns Escape).
  Q3-sub architect decision: **option (b)** — `impl iced::widget::table::Catalog for iced::Theme`
  in a new `crates/ui/src/theme/iced_widget_catalogs.rs` module (~30 LOC,
  future-proofs for Brief B). Q5/Q6/Q7 resolutions collapse to single
  committed shapes (no fallback branches). R2 commits to
  Button-in-column-1 + sibling `Column<error_badges>` below table. R3
  commits to `Grid::new().columns(6)` with defaults theming (visual
  chrome stays in outer PANEL container). R4 commits to
  `Float::new(Stack::new().push(content).push(backdrop_layer(close_msg)), card)`
  with MouseArea backdrop sibling preserved at
  [`journal_transaction_modal.rs:118-131`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  + Escape in `state.rs` subscription. M0 ticks T-M0-J through T-M0-N
  via tasks.md companion edit. M2 fallback branch T2.3 removed; M4
  fallback branch T4.2 collapsed to a committed task; M2 gains a Catalog
  adapter T-task; M3 gains a defaults-theming confirmation note.
  No code changes. No crate adds.
  HANDOFF → orchestrator (4-lane developer fan-out: M1/M2/M3/M4).
- 2026-05-13 (architect, design pass): Added [`## Design — architect
  synthesis`](#design--architect-synthesis) section. Resolved Q1-Q7
  (all architect-decide; zero operator-routed). Q1 = per-widget
  snapshot refresh (bisectable). Q2 = 4-lane parallel fan-out
  (R1/R2/R3/R4 independent). Q3 = no Catalog adapter needed (cockpit
  theme.rs is 100% closure-routed, zero `impl Catalog` blocks; native
  table/grid/float accept the same `.style(|theme| Style { ... })`
  pattern). Q4 = `Table::new(items: Vec<T>)` per inherited orchestrator
  grep; bounded-Clone cost acceptable. Q5 = Button-per-row in column-1
  body lambda (preserves current cockpit pattern). Q6 = sibling
  `Column<error_badges>` below table (Option C); preserves row-bleed.
  Q7 = Escape stays in `state.rs` subscription; `float` does
  positioning only. Authored H-arch-A1 through H-arch-A7 (+A5b, +A7b
  sub-hypotheses), each with orchestrator-runnable falsifier (grep /
  cargo build / two-run determinism). H-arch-A6 RESOLVED-UNFALSIFIED
  inline (kpi_strip = 6 cards, architect-confirmed from file read).
  Surfaced 5 orchestrator-direct grep requests (H-arch-A3 / A4 /
  A5b / A7 / A7b) against `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/{grid,table,float}.rs`
  (sub-agent sandbox blocks `Read` and `find/grep/ls` on that path;
  build-artifact `.d`-manifest substitute confirms file presence
  but cannot inspect API surface). Populated [`tasks.md`](tasks.md)
  with M0-M4 + M_FINAL_TEST_RUN + M_FINAL_EVAL milestones; T-tasks
  enumerated per lane with file:line acceptance criteria. Frontmatter
  `owner: analyst → architect`. No code changes. No crate adds. No
  Cargo.toml edits (markdown feature flag remains M5's territory per
  operator-locked scope).
  HANDOFF → orchestrator (developer fan-out: 4 parallel lanes M1/M2/M3/M4).
- 2026-05-13 (analyst, initial draft): Scoped Brief A of
  [`iced-ecosystem-evaluation` v0.2.0](../iced-ecosystem-evaluation/feature.md).
  Mapped four target widgets (positions, strategies, kpi_strip,
  journal_transaction_modal) to native iced 0.14 `table` / `grid` /
  `float` primitives. Authored R1-R4 (one per target) capturing
  current LOC / data shape / state semantics / interactivity
  constraints. Authored V1-V4 acceptance criteria (each as 5
  sub-criteria VnA-VnE: compile+tests / snapshot-shape-only /
  PNG-byte-identical / anchors / docs warning-clean). Authored 5
  hypotheses (H1-H5) with orchestrator-runnable falsifiers (cargo
  doc + grep / table.rs source grep — no live display server).
  7 open questions for architect (Q1-Q7). Inherited locked decisions
  from predecessor brief (Q1/Q2/Q3 architect picks; H-arch-0/2/7 M0
  falsifier results). Flagged two on-file discrepancies vs. the task
  brief: (a) positions has 7 columns, not 6 (EXPOSURE is column 7);
  (b) kpi_strip has 6 cards, not 4 (Total Return / CAGR / Sharpe /
  Max DD / Win Rate / Trades). Both confirmed against
  [`positions.rs:38-46`](../../crates/ui/src/widgets/positions.rs) and
  [`kpi_strip.rs:81-130`](../../crates/ui/src/widgets/kpi_strip.rs).
  No code changes. No crate adds.
  HANDOFF → orchestrator (architect spawns next per canonical workflow).
