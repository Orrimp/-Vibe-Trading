---
slug: cockpit-render-regression
status: shipped
owner: presenter
updated: 2026-05-14
version: 1.0.0
predecessor: iced-native-widgets v0.1.0 (shipped 2026-05-13)
sibling: iced-aw-cherry-pick v1.0.0 (shipped 2026-05-14 — unblocked by this ship)
---

# Cockpit render regression — diagnosis + quality-gate overhaul (v0.2.0)

> **Status:** architect draft. **No code changes, no crate adds in this
> brief.** Architect authors falsifiable hypotheses for the orchestrator
> to run (per [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only)),
> proposes a quality-gate overhaul to catch this class of bug at PR time,
> and grounds the instrumentation strategy in 2026 ecosystem state via
> WebSearch + WebFetch.
>
> The panic root cause is **localized to one of ~6 candidate widgets** in
> the iced render tree; the orchestrator runs M0 falsifiers next to pin
> the culprit and routes to developer with a tight fix scope. The
> systemic gate gap — `panel_snapshots` text-summary helpers do not
> exercise the iced renderer — is the real lesson and the M1 deliverable.

## TL;DR

`cargo run -p ui --bin cockpit --features fixtures` panics on first
frame with `Build quad rectangle` inside `iced_tiny_skia::engine::
rounded_rectangle` (renderer's all-radii-zero fast-path
`tiny_skia::Rect::from_xywh(x, y, 0.0, 0.0)`). Orchestrator-run bisect
confirmed the trigger pre-dates Brief B (B2 spinner + B3 badge call
sites bypassed → cockpit still panics on first frame), so the culprit
lives in Brief A's native-widget adoption shipped 2026-05-13
([`iced-native-widgets`](../iced-native-widgets/feature.md))
or in pre-existing `crates/ui/` code nobody has actually rendered in
cockpit mode for days. **The 267-test panel_snapshots gate missed
this entirely** because Brief A's snapshot helpers render text-summary
strings, not the iced widget tree — confirmed by the developer's
honest admission at
[`iced-aw-cherry-pick/tasks.md`](../iced-aw-cherry-pick/tasks.md)
T-M2-3 / T-M3-3 (zero snapshot bytes changed across the
`muted_body → loading_with_spinner` + `colored_cell → Badge` swaps;
the helpers route via `strings::*` regardless of the iced widget
underneath). The brief proposes (i) an M0 falsifier batch to pin
the offending widget; (ii) a `cockpit-smoke` skill that makes
`cargo run --bin cockpit --features fixtures` a mandatory pre-tick
gate; (iii) replacement of text-summary `*_summary` helpers with
real-renderer `iced_test::Simulator` flows building on the
already-pinned `iced_test = "=0.14.0"` + `image-compare = "=0.4"`
dev-deps shipped by [`ui-test-harness-bootstrap`](../ui-test-harness-bootstrap/feature.md);
(iv) `tracing` spans around the widget draw lifecycle on a
`render-debug` feature flag; (v) an opt-in `DebugRenderer` newtype
that intercepts the offending `Quad` with widget context instead
of the bare `Build quad rectangle` panic.

## Problem statement

### The panic

Reproducible on `2026-05-14T*` runs of `cargo run -p ui --bin cockpit
--features fixtures` (Darwin 25.4.0 / aarch64-apple-darwin). Full
backtrace captured at `/tmp/cockpit-panic-trace-2026-05-14.log` (130
lines). The load-bearing frames are:

```
thread 'main' panicked at iced_tiny_skia-0.14.0/src/engine.rs:686:14:
Build quad rectangle
  iced_tiny_skia::engine::rounded_rectangle      (engine.rs:686)
  iced_tiny_skia::engine::Engine::draw_quad      (engine.rs:65)
  iced_tiny_skia::Renderer::draw                 (lib.rs:119)
  iced_tiny_skia::window::compositor::present    (compositor.rs:209)
  iced_winit::run_instance::{{closure}}          (lib.rs:981)
  winit::platform_impl::macos::view::WinitView::draw_rect  (view.rs:208)
  thread caused non-unwinding panic. aborting.   (exit 134)
```

### Root cause (orchestrator-derived, not re-derived here)

- `iced_tiny_skia::engine.rs:50-63` clamps every corner radius to
  `min(width/2, height/2)`. When a `Quad`'s
  `bounds.width == 0.0 || bounds.height == 0.0`, all four radii get
  clamped to `0.0`.
- The all-zeros branch in `rounded_rectangle()`
  (`iced_tiny_skia-0.14.0/src/engine.rs:674-687`, verified via local
  cargo-registry read) then calls
  `tiny_skia::Rect::from_xywh(x, y, 0.0, 0.0).expect("Build quad rectangle")`.
- `tiny_skia::Rect` requires **positive** width and height; zero
  dimensions return `None`; the `.expect(...)` aborts.
- The panic crosses the Objective-C boundary at
  `WinitView::draw_rect` which **cannot unwind**, so the panic
  aborts the entire process (exit 134) instead of being caught by
  the iced runtime.

This matches the upstream class of "iced renderer asserts on
empty-mesh / zero-dim draw call" — see [iced-rs/iced#2774](https://github.com/iced-rs/iced/issues/2774)
(rendering only empty meshes crashes iced; fixed in #2782, but the
fix landed on the wgpu path; the tiny-skia engine still carries the
`.expect()` on the all-radii-zero branch we hit).

### What we know is NOT the trigger (orchestrator bisect, 2026-05-14)

- **`iced_aw::Spinner`** (Brief B B2). Orchestrator commented out the
  `iced_aw::Spinner::new()` call site in
  [`crates/ui/src/widgets/frame.rs:175`](../../crates/ui/src/widgets/frame.rs)
  → cockpit still panics on first frame.
- **`iced_aw::Badge`** (Brief B B3). Orchestrator commented out the
  badge construction inside `status_badge_cell` at
  [`crates/ui/src/widgets/strategies.rs:332-345`](../../crates/ui/src/widgets/strategies.rs)
  → cockpit still panics on first frame.

So the trigger is **either Brief A's native-widget adoption (table /
grid / float) or pre-existing widget code shipped earlier than
2026-05-13**. The cockpit binary itself has been gating on
`required-features = ["fixtures"]` for weeks
([`crates/ui/Cargo.toml:20`](../../crates/ui/Cargo.toml)), but
nobody has actually run it (only the unified `cockpit_live` bin gets
exercised by operators), so the regression could have been latent
for days.

### Why the existing test gate missed this

The 267 panel-snapshot tests at
[`crates/ui/tests/panel_snapshots.rs:1779-2298`](../../crates/ui/tests/panel_snapshots.rs)
do NOT render the iced widget tree. They render **text-summary
helpers** (`tape_summary`, `positions_summary`, `strategies_summary`,
…) that walk the `Cockpit` state struct and emit
`PanelState`-keyed `String` blocks. The developer-honest admission of
this gap is at
[`iced-aw-cherry-pick/tasks.md`](../iced-aw-cherry-pick/tasks.md)
T-M2-3 (`"Zero existing snapshots changed bytes. … the text-summary
helpers route via strings::*_LOADING regardless of which iced widget
wraps it"`) and T-M3-3 (`"the swap from colored_cell to Badge lives
entirely in the widget render path; the text-summary helpers don't
inspect the cell construction. Zero existing snapshots changed bytes"`).

Real-iced-renderer coverage today is **~0%** of the cockpit's render
surface. The orchestrator-only `cargo run --bin cockpit --features
fixtures` step is the actual visual gate, and it caught this — but
it ran **once, by hand, after presenter handoff** for Brief B. That
is the gate that needs to become mandatory.

## M0 — Hypothesis register (orchestrator-runnable falsifiers)

Each hypothesis posits a specific widget OR layout pattern that may
be the source of the zero-dim `Quad`. Each carries an
orchestrator-runnable falsifier (because `cargo run --bin cockpit`
with a live window is orchestrator territory per
[`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries));
the architect is forbidden from concluding "the bug is X" without an
orchestrator-run empirical test that refused to falsify (per the
chart-canvas-overhaul retrospective).

**Ordering rationale** — likelihood × falsification cost. **H1 is
the strongest candidate by a wide margin**: it is a zero-LOC
diff falsifier (just flip one constant), the constant was changed
in the past to `0.0` deliberately, and the rendered Container has
a styled background which forces the renderer to emit a fill_quad
for the right rail.

### H1 — Shell right-rail's `Length::Fixed(0.0)` Container emits a zero-width Quad

- **Statement.** [`crates/ui/src/shell.rs:37-39`](../../crates/ui/src/shell.rs)
  wraps a `Space::new()` inside
  `Container::new(...).width(Length::Fixed(RIGHT_RAIL_WIDTH_PX))
  .height(Length::Fill)`. `RIGHT_RAIL_WIDTH_PX` is defined as
  `f32 = 0.0` at
  [`crates/ui/src/theme.rs:596`](../../crates/ui/src/theme.rs)
  ("Phase 6 swaps the right-rail to a real width when the v2-LLM
  Assistant ships; Phase 2's job is just to leave the spot"). The
  outer `shell.rs:54-62` `Container` carries a styled background via
  `container::Style::background = Some(color::CANVAS.current(mode).into())`
  on the **whole shell row**, not on the `right_track`, but the
  zero-width `right_track` itself is still a rendered Container.
  In iced 0.14's tiny-skia path, a Container with a non-default
  Style (background or border) **always** emits a `fill_quad` —
  and the zero-width fast path hits the panic.
- **Why it's the strongest candidate.** This was set to `0.0`
  **deliberately** as a layout placeholder (per the file comment at
  `shell.rs:8-10`). The Brief A migration shipped 2026-05-13 may
  have indirectly upgraded which Container variant gets drawn (the
  Brief A `table.rs` adoption pushes additional fill_quads through
  every cell-bounds path; the cockpit's render order may now reach
  the right-rail Container's draw call where it previously
  short-circuited). This explains why the panic only manifests
  AFTER Brief A's adoption.
- **Falsifier.** Orchestrator changes
  [`crates/ui/src/theme.rs:596`](../../crates/ui/src/theme.rs)
  from `pub const RIGHT_RAIL_WIDTH_PX: f32 = 0.0;` to `1.0`, runs
  `cargo build -p ui --bin cockpit --features fixtures && cargo run
  -p ui --bin cockpit --features fixtures` for 7 seconds (kill via
  `pkill -f "target/debug/cockpit"`), captures stderr.
- **Expected outcome on FALSIFIED (cockpit boots, no panic in 7s).**
  Right-rail Container is the culprit. The zero-width
  `Length::Fixed(0.0)` Container with the implicit Style background
  is the load-bearing zero-dim Quad. Developer fix scope: either
  (a) gate the right_track behind `if RIGHT_RAIL_WIDTH_PX > 0.0`,
  (b) replace `Container::new(Space::new())` with `Space::new()`
  alone (no wrapper that emits a fill_quad), or (c) bump
  `RIGHT_RAIL_WIDTH_PX` to a non-zero placeholder (≥1.0). Routes
  HANDOFF → developer with surgical 1-line fix.
- **Expected outcome on UNFALSIFIED (still panics).** Right-rail not
  the trigger. Move to H2.

### H2 — `Container::new(Space::new()).width(Length::Fill).height(0)` emits zero-height Quad

- **Statement.** Two call sites use `.height(0)` with an integer
  literal (which iced 0.14 coerces to `Length::Fixed(0.0)`):
  - [`crates/ui/src/widgets/frame.rs:135`](../../crates/ui/src/widgets/frame.rs)
    inside an `active_row(...)` helper —
    `.push(Space::new().width(space::XS as f32).height(0))`.
  - [`crates/ui/src/screens/strategies.rs:263`](../../crates/ui/src/screens/strategies.rs)
    inside the chip-row layout —
    `.push(iced::widget::Space::new().width(Length::Fill).height(0))`.

  These are Space widgets (no style → no fill_quad in stock iced),
  BUT they are descendants of Containers/Rows that DO carry styles.
  When the parent computes bounds.height for layout, the zero-height
  child may force the parent's content rectangle to compute a
  zero-height fill (depending on how iced 0.14's `Row` distributes
  remaining space).
- **Falsifier.** Orchestrator changes both `.height(0)` →
  `.height(1)` and reruns the 7s cockpit smoke.
- **Expected on FALSIFIED.** One of the two `.height(0)` Space
  widgets propagates a zero-height bound to a styled ancestor.
  Developer fix: either replace `height(0)` with `height(Length::Shrink)`
  or remove the redundant Space.
- **Expected on UNFALSIFIED.** Spaces are not the trigger. Move to H3.

### H3 — Brief A's `iced::widget::table::Table` adoption emits a zero-width column Quad

- **Statement.** Brief A R1 / R2 swapped hand-rolled
  `Row::new()` headers + `Scrollable<Column>` bodies for
  `iced::widget::table::Table::new(columns, rows)` at
  [`crates/ui/src/widgets/positions.rs:122`](../../crates/ui/src/widgets/positions.rs)
  and
  [`crates/ui/src/widgets/strategies.rs:165`](../../crates/ui/src/widgets/strategies.rs).
  iced 0.14's `Table` ships with a Catalog impl that emits a
  `separator_x` + `separator_y` `fill_quad` per cell-grid boundary
  (`iced_widget-0.14.2/src/table.rs:704-714`). When the input slice
  is empty (`Cockpit.positions == PanelState::Ready(vec![])` for
  the very first frame before fixtures populate), `Table` computes a
  zero-height body region but **may still emit a header separator
  fill_quad** with zero-width because the columns haven't been
  allocated yet.
- **Falsifier.** Orchestrator inserts an early-return at
  `positions.rs:122` and `strategies.rs:165` returning
  `iced::widget::Text::new("debug: table bypassed").into()` (commented
  in, not deleted). Then rebuild + run cockpit for 7s.
- **Expected on FALSIFIED.** `Table` is the trigger. Likely the
  zero-row empty-state path. Fix: gate `Table::new` behind
  `if rows.len() > 0` and fall back to the `muted_body(POS_EMPTY)`
  path (which is already the documented empty-state branch but
  may be miswired post-Brief-A).
- **Expected on UNFALSIFIED.** Table not the trigger. Move to H4.

### H4 — Brief A's `iced::widget::grid::Grid` for kpi_strip emits zero-cell Quad

- **Statement.** Brief A R3 swapped a 6-column `Row::new()` for
  `Grid::new().columns(6)` at
  [`crates/ui/src/widgets/kpi_strip.rs:143-153`](../../crates/ui/src/widgets/kpi_strip.rs).
  The cockpit Home screen does not display the KPI strip (KPI is
  viewer-bin only — see
  [`screens/home.rs:24-42`](../../crates/ui/src/screens/home.rs)),
  but **the viewer-bin scenario selector at
  `bin/cockpit.rs:140` may auto-select a fixture preset that
  routes through the KPI strip somewhere we missed**. iced 0.14
  `Grid` defaults to `Sizing::AspectRatio(1.0)`; the architect
  pass overrode this with `.height(Length::Shrink)` — but if
  shrink yields zero (no children fit on first frame), the Grid's
  internal `fill_quad` for cell backgrounds may panic.
- **Falsifier.** Orchestrator comments out the `Grid::new()` chain
  at `kpi_strip.rs:143` and returns a `Space::new().height(Length::Fixed(80.0)).into()`
  placeholder. Rebuild + run for 7s. **Note:** Home screen
  does not include kpi_strip; if H4 is the trigger, the panic comes
  from a non-Home screen — check what `Cockpit::current_screen` is
  on cold start (likely `Home` per
  [`state.rs`](../../crates/ui/src/state.rs)). If Home, H4 is
  unlikely; expected outcome on UNFALSIFIED.
- **Expected on FALSIFIED.** Grid is the trigger.
- **Expected on UNFALSIFIED.** Grid not the trigger. Move to H5.

### H5 — Brief A's `Float`-wrapped Stack overlay emits zero-dim backdrop Quad

- **Statement.** Brief A R4 wrapped the audit-modal at
  [`crates/ui/src/widgets/journal_transaction_modal.rs:151-153`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  in `Float::new(Stack::new().push(content).push(backdrop).push(card))`.
  The `Float` is documented as "structurally inert" at default scale
  with `Float::style(|_theme| float::Style::default())` — but the
  `backdrop_layer` at
  [`journal_transaction_modal.rs:165-171`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  is a `Container::new(Space::new().width(Length::Fill).height(Length::Fill))
  .style(... background: OVERLAY ...)` wrapping a `Space`. The
  `style` closure emits a `fill_quad`; if the Stack's first-frame
  layout pass yields zero bounds for the backdrop (it shouldn't —
  Fill on both axes is well-defined), the fill_quad would hit
  the panic.
- **Critical sub-claim.** The audit modal is only rendered when
  `Cockpit.tape_audit_modal == Some(_)`. On cold start the modal
  is `None`, so the `journal_transaction_modal::view(...)` function
  is **not invoked** — callers render `content` directly. H5 should
  therefore NOT trigger on first frame. But the shell may still
  reach the `Float` code path if a fixture preset accidentally
  primes the modal state.
- **Falsifier.** Orchestrator inserts a `panic!("H5 reached")` at
  `journal_transaction_modal.rs:124` (the `pub fn view`). If
  cockpit panics there instead of in tiny-skia, the modal is being
  rendered on cold-start — and H5 is the trigger. If cockpit still
  panics with the original `Build quad rectangle`, the modal is
  not being invoked.
- **Expected on FALSIFIED ("H5 reached" replaces the tiny-skia
  panic).** The journal modal is mis-primed by fixtures. Either
  fix the fixture preset OR add an early-return in `view` when
  `state` is in a default-empty configuration.
- **Expected on UNFALSIFIED (still `Build quad rectangle`).** Modal
  not invoked; H5 ruled out. Move to H6.

### H6 — Strategies screen's empty-`Space` early-return on first frame emits zero-dim Quad

- **Statement.** [`crates/ui/src/screens/strategies.rs:143-145`](../../crates/ui/src/screens/strategies.rs)
  has an early-return:
  ```rust
  return iced::widget::Space::new()
      .width(Length::Shrink)
      .height(Length::Shrink)
      .into();
  ```
  `Space::new()` with `Shrink/Shrink` is **zero-bounds**. Stock
  iced `Space` does not have a `style()` builder, so it should not
  emit a fill_quad — but if this Space is wrapped by a `Container`
  ancestor with a Style higher up the tree, the wrapper's fill_quad
  inherits the zero bounds. The strategies screen IS reachable
  from Home if a fixture preset auto-selects it.
- **Falsifier.** Orchestrator changes the early-return to
  `iced::widget::Space::new().width(Length::Fixed(1.0)).height(Length::Fixed(1.0)).into()`,
  rebuild + run for 7s.
- **Expected on FALSIFIED.** The Shrink/Shrink Space inside a
  styled ancestor is the trigger. Fix: replace with a 1×1 Space or
  remove the early-return.
- **Expected on UNFALSIFIED.** Not the trigger. Move to H7.

### H7 — `chart.rs` canvas-internal Quad on first frame before bar data populates

- **Statement.** The chart canvas at
  [`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs)
  is a `Canvas`-backed program that draws bars, signals, and
  markers via `Frame::fill_rectangle`. On first frame, the
  `chart_buffer` is empty (`Cockpit::chart_buffer.bars().count() ==
  0`) and the chart program may still call `Frame::fill_rectangle`
  with `Size::ZERO` for one of the empty layers (volume histogram
  / drawdown band / equity curve).
- **Sub-claim.** Charts screen is NOT default; cold-start screen is
  `Home`. Charts reaches the render tree only if a fixture preset
  primes `current_screen == Charts`. If the panic is on Home,
  H7 is impossible.
- **Falsifier.** Orchestrator inserts a `tracing::error!` and an
  early-return `Frame::fill_text("debug: chart bypassed")` at the
  top of the chart's `Program::draw` impl (line to be located by
  the orchestrator via `grep -n "impl Program" crates/ui/src/widgets/chart.rs`).
  Rebuild + run for 7s.
- **Expected on FALSIFIED.** Chart canvas is the trigger. Fix:
  guard each `fill_rectangle` against zero-size.
- **Expected on UNFALSIFIED.** Chart not the trigger; move to H8.

### H8 — `focus_ring` or `kill` widget emits zero-dim quad in cold-start state

- **Statement.** Catch-all hypothesis covering the remaining
  styled-Container widgets on the cold-start render path:
  [`focus_ring.rs:78-83`](../../crates/ui/src/widgets/focus_ring.rs)
  uses `Length::Shrink` on both axes for its wrap, and
  [`kill.rs`](../../crates/ui/src/widgets/kill.rs) carries a halted
  banner that may be invisible-by-design until a kill event fires
  but still draws a zero-bounds chrome.
- **Falsifier.** Orchestrator comments out the `focus_ring::wrap(...)`
  call at
  [`screens/strategies.rs:178`](../../crates/ui/src/screens/strategies.rs)
  (returning the raw `button` instead) AND comments out the
  `kill::view(...)` call sites if present. Rebuild + run for 7s.
- **Expected on FALSIFIED.** Either focus_ring or kill is the
  trigger; bisect within H8 (one at a time).
- **Expected on UNFALSIFIED.** None of focus_ring / kill are the
  trigger. Escalate to operator: full-bisect required (revert Brief A
  commits one widget migration at a time).

### Order of execution (cheapest first)

1. **H1** — flip `RIGHT_RAIL_WIDTH_PX = 0.0 → 1.0`. **1-line edit.**
2. **H2** — flip both `.height(0)` → `.height(1)`. **2-line edit.**
3. **H5** — `panic!("H5 reached")` in `journal_transaction_modal::view`.
   **1-line edit, fastest signal.**
4. **H3** — early-return in `positions.rs:122` + `strategies.rs:165`.
   **2-line edit.**
5. **H6** — modify the Shrink Space at `strategies.rs:143`. **3-line edit.**
6. **H4** — bypass Grid in `kpi_strip.rs:143`. **1-line edit.**
7. **H7** — chart canvas bypass. **5-10-line edit, requires reading
   chart.rs Program::draw.**
8. **H8** — focus_ring + kill bypass. **Last resort, 4-line edit.**

Architect-confidence ranking: H1 is the highest-probability single
explanation (deliberate zero-constant + styled-Container ancestor +
the only "phase-6 placeholder" in the cockpit shell). Estimated 60%
probability H1 falsifies on first run. H2 is the second-likeliest
(integer-literal `.height(0)` is a known footgun).

## M0 results (orchestrator-executed 2026-05-14)

Architect's H1 60%-probability prior was **wrong by a wide margin**.
The orchestrator ran the M0 falsifier batch as a single-message
bisect rather than the proposed cheapest-first sequence — flipping
constants where cheap, then descending the screen → widget render
tree once H1 and H2 came back UNFALSIFIED. The bisect closes the
search faster than the architect's ordering would have. Recorded
here so future architect passes know the falsifier-batch shape
beats the falsifier-ladder shape when the bisect terrain is the
render tree.

### Bisect ladder (verbatim from orchestrator)

Each step: edit a single call site → `cargo build -p ui --bin
cockpit --features fixtures` → run cockpit 7s → grep
`panicked at`. Counts: 0 = clean, 3 = panic (one `panicked at`
+ one `non-unwinding panic` + one trailing `panic_handler` frame).

| Step | Patch | Panic count |
|------|-------|-------------|
| Baseline | (original code) | **3** |
| H1 | `theme.rs:596` `RIGHT_RAIL_WIDTH_PX = 0.0 → 1.0` | 3 (UNFALSIFIED) |
| H2 | `frame.rs:135` + `screens/strategies.rs:263` `.height(0) → .height(1)` | 3 (UNFALSIFIED) |
| Shell bypass | `shell.rs:34` `body = Text::new(...)` instead of `screen_body(...)` | **0** (clean — culprit is in Home descendants) |
| Home / pnl only | rest bypassed | 0 (pnl clean) |
| Home / positions only | rest bypassed | 0 (positions clean) |
| Home / strategies only | rest bypassed | **3** (culprit is in strategies widget) |
| `strategies::view` full bypass | returns `Text::new(...)` | 0 |
| `strategies::view` keeps `panel()` wrapper, body = `muted_body` | bypass `ready_body` | 0 (panel wrapper clean) |
| `ready_body` keeps `error_badges` + `footer`, bypasses `strategies_table` | `Column::new().push(Text)` | **0 — H3 CONFIRMED** |

### Verdicts

- **H1 UNFALSIFIED.** Right-rail `Length::Fixed(0.0)` Container is
  not the trigger. Patch reverted.
- **H2 UNFALSIFIED.** `.height(0)` Spaces in `frame.rs:135` +
  `screens/strategies.rs:263` are not the trigger. Patch reverted.
- **H3 CONFIRMED with its assumption falsified.** Culprit is
  [`crates/ui/src/widgets/strategies.rs:165`](../../crates/ui/src/widgets/strategies.rs)
  `table::Table::new(columns, rows.iter().cloned()).width(Length::Fill)`.
- **H4–H8 obsoleted by H3 confirmation.** Bisect ladder already
  excluded their descendants from the panic path. No re-runs needed.

### Honest divergence — H3 assumption was wrong

The original H3 statement claimed the trigger was the
**empty-state path** (zero-row `rows` slice before fixtures
populate). The orchestrator-confirmed evidence shows the cockpit
fixtures bin pre-populates `cockpit.strategies` via the
`Message::BarReceived` sequence at
[`crates/ui/src/bin/cockpit.rs:161-166`](../../crates/ui/src/bin/cockpit.rs),
so `ready_body` receives a **non-empty** `rows` on first frame.
The panic happens with populated data. The architect's proposed
defensive `if rows.len() > 0` gate would therefore not address
the bug. The M0-FIX section below abandons that gate and targets
the populated-rows path.

### Positions / Strategies asymmetry observation

[`crates/ui/src/widgets/positions.rs:122`](../../crates/ui/src/widgets/positions.rs)
calls `table::Table::new(columns, visible_iter)` with the **same
widget shape**, but positions in isolation does NOT panic
(orchestrator-verified). Only strategies' Table panics. The
difference is cell-content composition:

- **Strategies column 1** uses [`id_cell`](../../crates/ui/src/widgets/strategies.rs)
  at lines 211-253 which wraps an inner `Space` inside a
  styled `Container`:

  ```rust
  let rule = Container::new(
      iced::widget::Space::new()
          .width(Length::Fixed(2.0))
          .height(Length::Fill),    // ← suspect: Fill inside a Table cell
  )
  .width(Length::Fixed(2.0))
  .height(Length::Fill)              // ← same suspect
  .style(move |_theme| container::Style {
      background: Some(rule_color.into()),
      ..Default::default()
  });
  ```

  `rule_color` is `iced::Color::TRANSPARENT` when `!is_active`
  (per `strategies.rs:212-216`), so the inactive case still emits
  a `fill_quad` for a transparent background — the renderer
  doesn't fast-path-skip transparent fills inside a styled
  Container.

- **Positions cols 1-7** (`positions.rs:60-119`) use plain `cell`
  and `colored_cell` Text widgets only (`positions.rs:127-136`).
  No `Length::Fill` inside a Container/Space.

**Standing hypothesis (UNFALSIFIED, to be tested by M0-FIX
candidates).** Brief A's iced Table allocates row height via a
layout pass that doesn't pre-commit a non-zero height before each
cell's content gets bounds. The rule Container's
`height(Length::Fill)` resolves to 0 inside the Table cell context;
the Container's styled fill_quad is emitted with `2.0 × 0.0`
bounds; iced_tiny_skia's all-radii-zero clamp branch fires;
`tiny_skia::Rect::from_xywh(_, _, 2.0, 0.0)` returns `None`;
`.expect("Build quad rectangle")` aborts. The asymmetry with
positions corroborates the hypothesis — positions has no
`Length::Fill` inside a Table cell.

### What is NOT the fix (red herrings ruled out)

- **The Catalog adapter is not the lever.**
  [`crate::theme::iced_widget_catalogs::cockpit_table_style_fn`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  at lines 99-105 sets `separator_x` / `separator_y` to a
  `Background` (a colour), not a thickness. Wiring it via
  `iced::widget::Themer` would only change colour, not geometry.
  Confirmed by reading the adapter + `iced_widget` 0.14.2 docs
  ([docs.rs/iced_widget/0.14.2/iced_widget/table](https://docs.rs/iced_widget/0.14.2/iced_widget/table/struct.Table.html)
  — `Table` has no `.style()` builder; the `.separator_x() /
  .separator_y()` builders take `impl Into<Pixels>` thickness).
- **iced 0.14.x patch bump is dead lever.** The workspace already
  resolves `iced_widget v0.14.2` transitively through
  `iced = "=0.14.0"` (verified via `cargo tree -p ui | grep
  iced_widget`). `iced_tiny_skia` is still 0.14.0 — no published
  patch exists. The upstream master CHANGELOG ends at 0.14.0
  ([github.com/iced-rs/iced/CHANGELOG.md](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md)),
  so there is no documented post-0.14.0 fix landing on the
  registry to bump to.

## M0-FIX — H3 root-cause fix design

Five candidate fixes, ordered smallest-blast-radius first per the
prompt constraint. Each carries a falsifier the orchestrator runs.
Each is independent — a FALSIFIED early candidate stops the
ladder; UNFALSIFIED candidates exhaust their try and move down.

### Fix candidate ordering (committed)

1. **F1 — `id_cell` rule: `height(Length::Fill)` → `height(Length::Fixed(24.0))`.**
   Smallest blast radius (2-line edit, single widget). Bet: pinning
   the rule height to a fixed pixel value (~ Table row height)
   bypasses the Table layout pass's zero-height transient, the
   styled Container's fill_quad emits with `2.0 × 24.0` bounds,
   panic gone.

2. **F2 — Replace `Container::new(Space::new())` rule with `iced::widget::vertical_rule(2)`.**
   ~10-line edit at lines 217-227. Stock iced widget; its layout
   impl guards against zero-bound emissions per its source
   ([docs.iced.rs/iced_widget/rule.rs](https://docs.rs/iced_widget/0.14.2/src/iced_widget/rule.rs.html)).
   Bet: stock widget knows how to behave in Table-cell layout
   context where the custom Container+Space composition does not.
   Caveat: `vertical_rule` colour comes from the Theme's `Rule`
   palette, not from the cockpit's `ACCENT` token; if the operator
   needs the exact ACCENT pixel value, F2 needs a
   `.style(move |_t| rule::Style { color: ACCENT, .. })` chained
   onto the rule (`vertical_rule` has `.style()` per its docs).

3. **F3 — Suppress Table separator thickness: `Table::separator_x(0).separator_y(0)`.**
   1-line edit at strategies.rs:165 (chain two builder calls onto
   the existing `Table::new(...).width(Length::Fill)`). Bet: if
   the panic is not in the rule Container but in Table's
   own separator fill_quad emission, collapsing thickness to 0 px
   short-circuits the fill_quad call entirely. The
   `iced_widget::table::Table::separator_x(impl Into<Pixels>)`
   builder is documented at
   [docs.rs/iced_widget/0.14.2/iced_widget/table/struct.Table.html](https://docs.rs/iced_widget/0.14.2/iced_widget/table/struct.Table.html).
   **Risk:** if the panic IS in the rule Container, F3 will not
   fix it. F3 is therefore a falsifier that distinguishes "Table
   separator emits zero-bound fill_quad" from "id_cell rule emits
   zero-bound fill_quad."

4. **F4 — Wire the Catalog adapter via Themer (`Themer::new(table, |_| cockpit_table_style_fn())`).**
   Already-shipped Catalog adapter is unused; wiring it only sets
   separator COLOUR, not geometry. Per the standing hypothesis
   this should NOT fix the panic — wired only to confirm the
   Catalog adapter is a red herring, NOT to address the bug.
   **Recommend skipping F4 unless F1-F3 all fail**; this is a
   diagnostic falsifier, not a fix.

5. **F5 — Revert Brief A R2 strategies-table only.** Replace
   `table::Table::new(columns, rows)` with the pre-Brief-A
   hand-rolled `Row::new()` header + `Scrollable<Column>` body.
   Brief A's
   [`iced-native-widgets/feature.md`](../iced-native-widgets/feature.md)
   has the diff context. Blast radius **~80-120 LOC**, last
   resort.

### F1 — `id_cell` rule: fixed-pixel height fallback

- **Statement.** Edit
  [`crates/ui/src/widgets/strategies.rs:220`](../../crates/ui/src/widgets/strategies.rs)
  and `strategies.rs:223` — change the rule Container's
  `height(Length::Fill)` (both on the inner `Space` and on the
  outer `Container`) to `height(Length::Fixed(24.0))`. 24 px
  matches the cockpit's Table body row height (per the Lumen
  design system `text::BODY = 14 px` + `space::S = 4 px` padding
  per [`spec/design`](../design/) Lumen tokens; orchestrator
  may tune to 20 / 28 px if 24 misaligns with adjacent text).
- **Falsifier.** Orchestrator edits both `.height(Length::Fill)`
  occurrences at `strategies.rs:220` and `strategies.rs:223` to
  `.height(Length::Fixed(24.0))`, then:
  ```bash
  cargo build -p ui --bin cockpit --features fixtures && \
    (cargo run -p ui --bin cockpit --features fixtures &
     COCKPIT_PID=$!; sleep 7; kill "$COCKPIT_PID" 2>/dev/null;
     wait "$COCKPIT_PID" 2>&1) 2>/tmp/cockpit-fix-f1-stderr.log
  grep -E "panicked at|non-unwinding panic|Build quad rectangle" \
    /tmp/cockpit-fix-f1-stderr.log
  ```
- **Expected on FALSIFIED (no panic, 7s clean run).** F1 is the
  fix. Commit; HANDOFF → developer for cleanup + visual
  verification (the active-strategy rule should still render as
  a 2 px wide vertical accent stripe in column 1; if it doesn't,
  the 24 px height is mis-tuned and developer iterates).
- **Expected on UNFALSIFIED (still panics).** The rule
  Container's height is not the load-bearing zero-bound. Move to
  F2.
- **Blast radius.**
  - File-span LOC: **2** (two `Length::Fill` → `Length::Fixed(24.0)`).
  - Glue-layer LOC: **0**.
  - Affected files: `crates/ui/src/widgets/strategies.rs` only.

### F2 — Replace styled Container+Space rule with stock `vertical_rule`

- **Statement.** Replace lines 217-227 of
  [`crates/ui/src/widgets/strategies.rs`](../../crates/ui/src/widgets/strategies.rs)
  with:
  ```rust
  use iced::widget::vertical_rule;
  let rule = vertical_rule(2).style(move |_theme: &iced::Theme| {
      iced::widget::rule::Style {
          color: rule_color,
          width: 2,
          radius: iced::border::radius(0),
          fill_mode: iced::widget::rule::FillMode::Full,
      }
  });
  ```
  `vertical_rule(thickness)` is the stock iced 0.14 widget; its
  `Widget::layout` impl returns `Node::new(Size::new(thickness,
  bounds.height))` with `bounds.height` constrained by the
  parent — known to play correctly in Table cells per the
  `iced_widget` cell-layout pipeline.
- **Falsifier.** Orchestrator applies the edit above, rebuilds,
  runs cockpit 7s, greps for `panicked at` (same script as F1
  with `f1 → f2` in the log path).
- **Expected on FALSIFIED.** F2 is the fix. Commit; HANDOFF →
  developer.
- **Expected on UNFALSIFIED.** Either the rule isn't the trigger
  OR `vertical_rule` shares the same broken path (unlikely given
  its layout impl). Move to F3.
- **Blast radius.**
  - File-span LOC: **~10** (replace 11 lines with ~8).
  - Glue-layer LOC: **0** (no new dep — `vertical_rule` is in
    iced's prelude).
  - Affected files: `crates/ui/src/widgets/strategies.rs` only.
  - Visual drift: rule colour now flows through iced's
    `rule::Style` rather than `container::Style.background`; the
    `color` field defaults to the Theme's `Rule` palette unless
    overridden. The closure above pins it to `rule_color`
    explicitly — identical to current behaviour.

### F3 — Collapse Table separator thickness to zero pixels

- **Statement.** Edit
  [`crates/ui/src/widgets/strategies.rs:165`](../../crates/ui/src/widgets/strategies.rs)
  from
  ```rust
  let strategies_table = table::Table::new(columns, rows.iter().cloned())
      .width(Length::Fill);
  ```
  to
  ```rust
  let strategies_table = table::Table::new(columns, rows.iter().cloned())
      .width(Length::Fill)
      .separator_x(0)
      .separator_y(0);
  ```
  The `separator_x(0)` / `separator_y(0)` builders short-circuit
  Table's own separator fill_quad emission (zero-thickness lines
  emit no quad).
- **Falsifier.** Orchestrator applies the edit, rebuilds, runs
  cockpit 7s, greps.
- **Expected on FALSIFIED.** Table separator is the offending
  zero-bound fill_quad. F3 is the fix — but the operator must
  confirm visual acceptability (the inter-cell hairlines
  disappear). If acceptable, commit; otherwise HANDOFF →
  ui-designer to choose a different separator strategy
  (e.g. per-cell padding). Note: F3 fix and F1/F2 fix are
  **diagnostic-distinct** — they identify different root causes.
  F3 FALSIFIED implies the rule Container fix (F1/F2) is
  unnecessary; F3 UNFALSIFIED implies the panic is in the rule
  Container, not the separator.
- **Expected on UNFALSIFIED.** Separator not the trigger. Move
  to F4 (diagnostic) or F5 (revert).
- **Blast radius.**
  - File-span LOC: **2** (two `.separator_x(0).separator_y(0)`
    builder calls).
  - Glue-layer LOC: **0**.
  - Affected files: `crates/ui/src/widgets/strategies.rs` only.

### F4 — Wire Catalog adapter via Themer (DIAGNOSTIC, not a fix)

- **Statement.** Wrap the Table in
  `iced::widget::Themer::new(strategies_table, |_theme| /* set
  cockpit_table_style_fn */)`. The Catalog adapter sets separator
  COLOUR; this is **not expected to fix** the panic because the
  panic is in the all-radii-zero-bound branch of `rounded_
  rectangle`, which fires regardless of fill colour. F4 is wired
  only to falsify the "Catalog adapter fixes it" hypothesis and
  formally rule it out.
- **Recommend skipping F4 unless F1-F3 all fail.** Orchestrator
  may run F4 only if F1-F3 UNFALSIFIED — at that point the
  diagnostic value (rule out Catalog adapter as the bug) outweighs
  the test cost.
- **Blast radius (if wired).**
  - File-span LOC: **~5** (Themer wrap + Catalog adapter wire-in).
  - Glue-layer LOC: **0** (adapter already shipped at
    `crates/ui/src/theme/iced_widget_catalogs.rs:99-117`).

### F5 — Revert Brief A R2 strategies-table (last resort)

- **Statement.** Replace `table::Table::new(columns, rows)` at
  `strategies.rs:165` with the pre-Brief-A hand-rolled
  `Row::new()` header + `Scrollable<Column>` body. Source: Brief
  A's
  [`iced-native-widgets/feature.md`](../iced-native-widgets/feature.md)
  has the migration diff context — developer can `git show` the
  Brief A migration commit (introducing `table::Table::new(...)`)
  and reverse-apply only the strategies-table portion (positions
  stays migrated, since positions does not panic).
- **Falsifier.** Orchestrator applies revert via developer pass
  (this is too large for an orchestrator one-shot edit), rebuild
  + run cockpit 7s + grep.
- **Expected on FALSIFIED.** Brief A R2 strategies-table is the
  load-bearing change. Commit revert. Architect re-engages to
  decide whether to (a) ship the revert permanently or (b) wait
  for an iced patch that fixes the Table cell-layout pipeline
  and re-migrate later. **ADR required if F5 commits** —
  Brief A's stated architectural decision (T2.1 native-table
  migration) is partially undone, which is a structural
  divergence from the iced-native-widgets brief.
- **Expected on UNFALSIFIED.** Even the revert doesn't fix it →
  the bug is in code shared by both the legacy hand-rolled path
  AND the new Table path. Escalate to operator. The architect
  re-engages with a fresh hypothesis register.
- **Blast radius.**
  - File-span LOC: **~80-120** (per Brief A's own diff cite).
  - Glue-layer LOC: **0** (no Cargo.toml or feature-flag changes).
  - Affected files: `crates/ui/src/widgets/strategies.rs` only
    (positions stays on the native Table path).

### M0-FIX falsifier sequence (orchestrator runs in order, stop on first FALSIFIED)

1. **F1** (2-line edit) → if FALSIFIED, commit + HANDOFF → developer. STOP.
2. **F2** (10-line edit) → if FALSIFIED, commit + HANDOFF → developer. STOP.
3. **F3** (2-line edit) → if FALSIFIED:
   - If operator confirms visual acceptability of zero separators, commit + HANDOFF → developer. STOP.
   - If not, F3 result is informative but not the shipped fix; move to F5.
4. **F4** (diagnostic only) — run only if F1-F3 all UNFALSIFIED.
5. **F5** (80-120 LOC revert) — last resort; requires ADR.

Architect confidence ranking: **F1 highest** (60%-ish — the
`Length::Fill` inside a Table cell exactly matches the
positions/strategies asymmetry signal). **F2 second** (rest on
the same lever — replace the custom composition with the stock
widget). **F3 third** (orthogonal lever — separator vs cell rule
ambiguity; F3 result is interesting independent of fix outcome).

## M1 — Quality-gate overhaul (the systemic fix)

The text-summary `*_summary` helpers at
[`crates/ui/tests/panel_snapshots.rs:1832-2298`](../../crates/ui/tests/panel_snapshots.rs)
deliver fast determinism (267 tests in ~0.3s) but **zero iced
renderer coverage** — confirmed by the developer-honest admission at
[`iced-aw-cherry-pick/tasks.md`](../iced-aw-cherry-pick/tasks.md)
T-M2-3 / T-M3-3. The cockpit-render-regression panic is one
data-point in a class of bugs the existing harness structurally
cannot catch.

The proposal below builds on
[`ui-test-harness-bootstrap`](../ui-test-harness-bootstrap/feature.md)
(which already pinned `iced_test = "=0.14.0"` + `image-compare = "=0.4"`
in `crates/ui/Cargo.toml:116-118` as dev-dependencies). It does not
re-invent that harness; it extends it.

### M1-A — `cockpit-smoke` skill (mandatory orchestrator pre-tick gate)

**Shape.** A new `.claude/skills/cockpit-smoke/SKILL.md` that runs:

```bash
cargo run -p ui --bin cockpit --features fixtures &
COCKPIT_PID=$!
sleep 7
if kill -0 "$COCKPIT_PID" 2>/dev/null; then
    kill "$COCKPIT_PID"
    echo "COCKPIT SMOKE PASS (7s clean run)"
    exit 0
else
    wait "$COCKPIT_PID"  # capture exit code
    EXIT=$?
    echo "COCKPIT SMOKE FAIL (exited with $EXIT in <7s)"
    cat /tmp/cockpit-smoke-stderr.log
    exit 1
fi
```

The orchestrator captures stderr to `/tmp/cockpit-smoke-stderr.log`
and the skill greps for `panicked at` / `non-unwinding panic` /
`fatal runtime error`. **Mandatory after every UI brief's evaluator
PASS, before the operator approval gate.** Adds the orchestrator-only
`cargo run --bin cockpit` step the current process already does by
hand, formalized as a gate.

**Capability boundary citation.** Per [`AGENT.md ## Capability
boundaries`](../../AGENT.md#capability-boundaries), `cargo run --bin
cockpit with a live window` is orchestrator-only — this skill is
invoked by the orchestrator, not by sub-agents.

**Cost.**
- File-span LOC: **0** (no `crates/` edits).
- Glue-layer LOC: **+45** (`.claude/skills/cockpit-smoke/SKILL.md`
  ~30 lines + `AGENT.md` update to make it a pre-tick gate
  ~15 lines under `## Capability boundaries`).
- Adoption cost: **0.25 dev-day** (skill author + AGENT.md update).
- Detection coverage: catches the **current panic** (orchestrator
  has already verified this empirically). Catches the **next**
  first-frame regression of the same class (any
  `panic!()`/`unwrap()`/`expect()` reachable from the iced render
  tree). Does **not** catch silent visual regressions (those need M1-B).

### M1-B — Real-renderer snapshot tests via `iced_test::Simulator`

**Shape.** Replace the text-summary `*_summary` helpers at
`tests/panel_snapshots.rs:1832-2298` with a new
`tests/render_snapshots.rs` integration test that constructs an
actual iced widget tree per panel via
`iced_test::simulator(panel_view(&cockpit))`. The simulator walks
the widget tree (per
[docs.iced.rs/iced_test](https://docs.iced.rs/iced_test/index.html)
the API is **functional, not pixel-rendering** — it validates that
the widget tree builds, that messages route correctly, and that
`Selector` queries find expected widgets). For pixel-level
regression, use the `iced::advanced::renderer::Headless` trait
(documented at
[docs.iced.rs/iced/advanced/renderer/trait.Headless.html](https://docs.iced.rs/iced/advanced/renderer/trait.Headless.html))
to rasterize each panel to PNG, then `image_compare::gray_similarity_structure(
&Algorithm::MSSIMSimple, &baseline, &actual)`.

**Important architectural caveat.** WebFetch of
[docs.iced.rs/iced_test](https://docs.iced.rs/iced_test/index.html)
returned: *"The Simulator walks the widget tree rather than driving
the renderer to produce pixels."* So `iced_test::Simulator` alone
does NOT catch this panic — the panic is in the renderer, and the
simulator skips the renderer. **The right combination is
`Simulator` + `Headless::Renderer` rasterize**:
- `Simulator` proves the widget tree builds without panicking in
  `Widget::layout()` or `Widget::draw()` (the layout pass alone
  exercises every widget's bounds-computation logic).
- `Headless::Renderer::new()` + a 1280×720 surface rasterize proves
  the renderer doesn't panic on any of those widget draws.
- The PNG output goes through `image-compare` SSIM ≥ 0.99 vs the
  committed baseline.

The 8 `*_loading.snap` baselines + the 14 `strategies_*.snap`
baselines + the `viewer_picker_default_closed.snap` (and the
~244 other panel_snapshot tests) all become PNG triples (light /
dark / current-theme) under
`crates/ui/tests/visual-baselines/render_snapshots/` — paralleling
the existing 3 `charts_screen_dark_*.png` triples from
ui-test-harness-bootstrap.

**Methodology — bulk replacement after a survey (per
user-memory `feedback_subagent_orchestration.md`'s M0 5-grep batch
rule).** A first dev pass writes ONE proof-of-concept render-snapshot
test for `positions_ready` panel, verifies the rasterization
pipeline + SSIM gate are deterministic across two runs, then
batch-replaces the remaining ~244 text-summary tests in a
follow-up. Two-run determinism is the hard gate per
[`iced-aw-cherry-pick/feature.md ## H-arch-9`](../iced-aw-cherry-pick/feature.md#h-arch-9--iced_awspinner-deterministic-render--resolved-pass-with-caveat).

**Cost.**
- File-span LOC: **-519 LOC retired** at
  `panel_snapshots.rs:1779-2298` (`*_summary` helpers; see Brief B's
  own line cite at T-M2-3) **+800 LOC new** in
  `tests/render_snapshots.rs` (proof-of-concept + per-panel
  rasterize wrappers). Net **+281 LOC**.
- Glue-layer LOC: **+15** for any helper additions to the
  baseline-directory layout under
  `crates/ui/tests/visual-baselines/render_snapshots/<panel>/`.
- Adoption cost: **2.5 dev-days** (one for the PoC + harness, 1.5
  for the bulk migration + baseline-PNG generation, both gated by
  the operator's call on the SSIM threshold).
- Detection coverage: catches the **current panic** AND the entire
  class of "widget builds in tree-walk but panics in renderer"
  bugs. Catches **silent visual regressions** (palette drift, layout
  shift). Adds CI cost: PNG rasterize is ~50ms per panel × ~250
  panels = ~12.5s additional wall-clock per `cargo test -p ui`
  run. Per the cargo-nextest pattern referenced at
  [nexte.st/book/slow-tests](https://nexte.st/book/slow-tests.html),
  this is gated behind a 60s `slow-timeout`.

### M1-C — Property-based layout invariants via `proptest`

**Shape.** A new
`crates/ui/tests/layout_invariants.rs` integration test using
`proptest` (already in workspace dependencies — verify via
`grep -n proptest Cargo.toml`). For each widget in
[`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) that
implements `iced::advanced::Widget`, write a property test:

```rust
proptest! {
    #[test]
    fn positions_widget_layout_node_never_zero_dim(
        symbol in "[A-Z]{3,6}",
        qty_str in "-?[0-9]{0,8}\\.[0-9]{0,8}",
        // … all PositionView fields fuzzed …
    ) {
        let cockpit = Cockpit { positions: PanelState::Ready(vec![pv]), .. };
        let view = positions::view(&cockpit);
        let mut renderer = iced_test::headless_renderer();
        let node = view.as_widget().layout(&mut ..., &renderer, &Limits::new(Size::new(0.0, 0.0), Size::new(1280.0, 720.0)));
        prop_assert!(node.size().width > 0.0 || node.size().width.is_nan(),
            "positions widget produced zero-width Node for input {pv:?}");
        prop_assert!(node.size().height > 0.0 || node.size().height.is_nan(),
            "positions widget produced zero-height Node for input {pv:?}");
        // … recursively traverse `node.children()` …
    }
}
```

This is the [emilk/egui#6752 pattern](https://github.com/emilk/egui/pull/6752)
ported to iced ("Add tests for layout and visuals of most egui
widgets" — same intent, different framework). Per the
[LogRocket proptest guide](https://blog.logrocket.com/property-based-testing-in-rust-with-proptest/),
proptest's shrinker auto-minimizes the offending input — so a
falsifying case for `positions` would shrink to the smallest
`PositionView` that produces the zero-dim node, giving the
developer a tight repro.

**Coverage scope.** Start with the **6 widgets implicated in M0
hypotheses** (positions, strategies, kpi_strip, journal_transaction_modal,
chart, focus_ring). Expand to all 22 widgets in a follow-up.

**Cost.**
- File-span LOC: **+250** in `tests/layout_invariants.rs` (proof of
  concept + 6 widget properties).
- Glue-layer LOC: **+5** (workspace `Cargo.toml` `proptest`
  dev-dep promotion if not already there).
- Adoption cost: **1.5 dev-day** (PoC + 6 widget properties; the
  remaining 16 are a follow-up brief).
- Detection coverage: catches **future** zero-dim layout regressions
  AT PR TIME (proptest runs in `cargo test` matrix), shrinks the
  failing input automatically. Does NOT catch the current
  iced_tiny_skia panic directly (proptest runs after layout, not
  through the renderer) — but if the current bug IS a layout
  miscompute (i.e. `Widget::layout` returns a zero-Node for a
  reachable input), proptest would have caught it. **M1-C is
  complementary to M1-A and M1-B.**

### M1-D summary — pipeline shape after adoption

```
PR opens
  ↓
cargo test -p ui                      ← existing 267 tests pass
  ↓
cargo test -p ui --test render_snapshots  ← M1-B (PNG SSIM ≥ 0.99)
  ↓
cargo test -p ui --test layout_invariants ← M1-C (proptest, no zero-Node)
  ↓
evaluator emits VERDICT → PASS
  ↓
ORCHESTRATOR runs cockpit-smoke skill  ← M1-A (7s live run, no panic)
  ↓
presenter assembles presentation
  ↓
operator approves
```

The cockpit-smoke skill is the **last** gate because it is the
expensive one (7s wall-clock + capability boundary). The render_snapshots
+ layout_invariants tests run in parallel with the existing test
matrix.

## M2 — Instrumentation, debugging, and AI-driven approaches (research section)

The user explicitly flagged: *"Maybe more logs, maybe telemetrie oder
other approaches ai driven ui design in rust. Search a bit in the
web."* This section grounds each proposal in 2026 ecosystem state
via WebSearch + WebFetch (per [`.claude/agents/architect.md`](../../.claude/agents/architect.md)
tool list).

### M2-A — `tracing` spans around widget draw lifecycle (production-pattern)

**Shape.** Introduce a `tracing::instrument` annotation on every
widget's `Widget::draw` and `Widget::layout` impl behind a new
`render-debug` feature flag. On a debug build with
`RUST_LOG=ui::widgets=trace cargo run --bin cockpit --features
fixtures,render-debug`, the cockpit emits one span per widget draw
call with `bounds={width, height, x, y}` as a structured field.

When the renderer panics on a zero-dim Quad, the operator (or the
orchestrator) greps the trace for the last span before the panic
and immediately knows the offending widget by name.

Citations (WebSearch grounding):
- Per [tracing crate docs](https://docs.rs/tracing): *"A Span has a
  beginning and end time, may be entered and exited by the flow of
  execution, and may exist within a nested tree of similar spans"* —
  perfect fit for the widget render tree.
- Per [LogRocket — Composing an observable Rust app](https://blog.logrocket.com/composing-underpinnings-observable-rust-application/):
  *"layers are composable units that sit between the trace data
  source and the subscriber, allowing for filtering, formatting,
  and enriching data"* — a `render-debug` layer can route widget-draw
  events to a dedicated NDJSON sink, not stdout.

The CLAUDE.md *"No `println!` in library code — use `tracing`"* rule
([`CLAUDE.md ## Coding rules`](../../CLAUDE.md#coding-rules)) already
puts us on this path; M2-A makes the rule load-bearing for the
render path, not just business logic.

**Cost.**
- File-span LOC: **+1 per widget** (the `#[tracing::instrument(skip(self, renderer, theme, style, layout, cursor, viewport))]`
  attribute) × ~30 widget impls = **+30**.
- Glue-layer LOC: **+25** in
  [`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml) for the
  `render-debug` feature + the gated `tracing` dep.
- Adoption cost: **0.75 dev-day**.
- Detection coverage: doesn't PREVENT regressions but makes the
  TTL of "panic → know which widget" drop from "30 min of comment-out
  bisect" to "~5 seconds of grepping a trace log."

### M2-B — `DebugRenderer` newtype wrapping `iced_tiny_skia::Renderer`

**Shape.** A diagnostic-only `DebugRenderer` at
`crates/ui/src/widgets/debug_renderer.rs` (new file) wrapping
`iced_tiny_skia::Renderer` and intercepting `fill_quad`. Behind a
`--features render-debug` opt-in flag, the wrapper checks
`quad.bounds.width > 0.0 && quad.bounds.height > 0.0` before
delegating to the real renderer; on zero-dim, it emits a
`tracing::error!` with the full `Quad` payload and the current
widget context (pulled from a thread-local `Cell<&'static str>` set
by M2-A's instrumented `draw` calls).

This delivers the "panic WITH widget context, not bare `Build quad
rectangle`" goal the user implicitly asked for.

**Tradeoffs (per `feedback_research_brief_framing.md` — name them
honestly).**
- Adds a wrapper Renderer that diverges from upstream iced's
  Renderer impl. We are NOT forking iced (per the
  `trading_ui_library_constraints.md`
  user-memory constraint and Brief B's "iced 0.14 stays pinned"
  decision), but we ARE introducing a wrapper that intercepts a
  public trait method. This is well within iced 0.14's documented
  extension surface (the `iced::advanced::Renderer` trait is
  explicitly exposed for this purpose), but it IS a non-zero
  maintenance burden on every iced update.
- Cost is justified by detection coverage: when a future regression
  ships, the operator's first signal is "widget=X emitted zero-dim
  Quad at frame N" instead of "tiny-skia panicked, good luck."

**Cost.**
- File-span LOC: **+120** (new file `widgets/debug_renderer.rs`).
- Glue-layer LOC: **+10** (feature flag + lib.rs re-export).
- Adoption cost: **1 dev-day** (the wrapping needs to delegate every
  method of `iced::advanced::Renderer` + `iced::advanced::graphics::compositor::Default`).
- Detection coverage: catches the current panic class (zero-dim
  Quad) AND any future renderer-level regression. Does NOT catch
  visual drift (no pixel comparison).

### M2-C — `iced_test::Simulator` + LLM-as-judge for semantic visual diff

**Shape.** This is the AI-driven layer the user asked for. The
research below names the pattern, the cost, and where it does NOT
make sense for our project — per
`feedback_research_brief_framing.md`'s honesty bias, we should be
upfront about the parts where the 2026 ecosystem is not yet there.

**State of the art in 2026 (per WebSearch grounding).**

1. **LLM-guided scenario testing**
   ([arxiv 2506.05079](https://arxiv.org/html/2506.05079v4) —
   "Scenario-Guided LLM-based Mobile App GUI Testing"): the
   ScenGen framework uses 5 LLM agents (Observer / Decider /
   Executor / Verifier / Recorder) to drive UI tests from
   natural-language scenarios. The Observer extracts structured
   GUI state; the Decider routes through a scenario tree to pick
   the next action. **Not directly applicable to a desktop iced
   cockpit** — ScenGen targets mobile apps with accessibility-tree
   instrumentation that iced does not yet expose. But the
   Observer pattern translates: an LLM can read the
   `iced_test::Simulator` widget-tree dump and answer questions
   like "does this layout look right per the spec?" without
   needing pixels.
2. **AI-vision visual regression**
   ([trilogyai — AI Vision and the Future of UI Testing](https://trilogyai.substack.com/p/ai-vision-and-the-future-of-ui-testing),
   [percy.io/blog/visual-regression-testing-tools](https://percy.io/blog/visual-regression-testing-tools)):
   GPT-4o / Gemini / Claude 4.7 can compare two screenshots and
   identify "meaningful" changes (palette shift, missing widget,
   misaligned text) vs "irrelevant" ones (1-pixel anti-aliasing
   jitter, OS-rendered cursor) better than SSIM. But the
   2026 honest assessment per
   [trilogyai](https://trilogyai.substack.com/p/ai-vision-and-the-future-of-ui-testing):
   *"AI-vision prototypes driven by models like GPT-4o and Gemini
   are starting to add semantic understanding, [but] still face
   limitations, with GPT-4o and Gemini prototypes identifying
   some injected CSS bugs but missing subtle pixel shifts and
   producing inconsistent outputs."* So LLM-as-judge augments
   SSIM, does not replace it.
3. **`image-compare` SSIM** (already pinned in our dev-deps per
   [`crates/ui/Cargo.toml:117`](../../crates/ui/Cargo.toml)). Per
   [vizzly.dev/blog/honeydiff-fast-image-diffing-foundation](https://vizzly.dev/blog/honeydiff-fast-image-diffing-foundation/),
   *"SSIM (Structural Similarity Index) measures how similar
   images are from a human perception standpoint"* — the right
   first-line gate.
4. **Vibe-coding loop with screenshot feedback** (per [Anthropic —
   Best practices for Claude Code](https://code.claude.com/docs/en/best-practices)
   and [Nolan Lawson — An experiment in vibe coding](https://nolanlawson.com/2025/12/28/an-experiment-in-vibe-coding/)):
   *"Claude performs dramatically better when it can verify its
   own work, like run tests, compare screenshots, and validate
   outputs"*. The orchestrator already has `capture-screenshot`
   skill; M2-C adds an `evaluate-screenshot` skill: orchestrator
   captures a fresh PNG, ships baseline + new + a 2-line
   description to Claude 4.7, asks "is the new render semantically
   equivalent to the baseline? if not, what changed?". Output
   becomes part of the evaluator's evidence pack.

**Proposal — three-layer visual gate.**
- **Layer 1 — `iced_test::Simulator` (M1-B PoC).** Asserts widget
  tree builds without panicking. Fast (~50ms/panel), deterministic.
- **Layer 2 — `image-compare` SSIM ≥ 0.99 vs committed PNG
  baseline (M1-B full).** Catches pixel-level drift. Deterministic;
  cost ~50ms/panel + baseline maintenance.
- **Layer 3 — LLM-as-judge on the diff (M2-C).** Only invoked when
  SSIM < 0.99 falsifies. Orchestrator ships the two PNGs + a
  `spec/ui-design-principles.md` excerpt to Claude 4.7 and asks
  "is this change intentional (palette refresh, new feature) or
  a regression (missing widget, layout shift)?". This converts
  the operator's "is the screenshot correct?" call into a
  triaged "Claude flagged 3 of 12 SSIM regressions as
  human-review-required."

The AI layer is **decision-support, not automation**. We are not
auto-merging visual changes based on Claude's judgment. The operator
still ticks "approved" — but they tick faster because the LLM
already filtered out the cosmetic SSIM noise.

**Cost.**
- File-span LOC: **+40** in a new
  `.claude/skills/evaluate-screenshot/SKILL.md` (the prompt
  template + Claude API invocation).
- Glue-layer LOC: **+10** (presenter brief updates to include
  the LLM-judged column in the verification matrix).
- Adoption cost: **0.5 dev-day** for the skill + **0.5 dev-day**
  for the prompt-engineering pass.
- Detection coverage: catches **semantic** regressions (intent
  drift) that SSIM cannot. **Does NOT replace** Layer 1 / Layer 2.
- API cost: ~$0.02 per Claude 4.7 call (two PNGs + 2k token
  prompt) × ~12 panels per UI brief × ~5 UI briefs/month
  = ~$1.20/month. Trivial.

## Architectural divergences (honest)

Per user-memory `feedback_research_brief_framing.md`: name
anywhere this brief contradicts current architecture, prior thinking,
or AGENT.md guidance.

- **Custom Renderer wrapper (M2-B) is a divergence from "iced 0.14
  stable, no forks"**
  ([`iced-aw-cherry-pick`](../iced-aw-cherry-pick/feature.md)
  Out of Scope). I argue it's acceptable because (a) the wrapper
  is opt-in via `--features render-debug` (production builds skip
  it), (b) `iced::advanced::Renderer` is a documented public
  extension surface, (c) the wrapper is a strict pass-through
  except for the diagnostic intercept. But it IS a non-zero
  maintenance commitment on every iced update — named honestly.

- **Mandatory `cargo run --bin cockpit` gating (M1-A) adds
  operator wall-clock cost.** Every UI brief's pre-tick gate
  grows by 7s. Over 5 UI briefs/month that's 35s/month — trivial,
  named for completeness.

- **Replacing text-summary `*_summary` helpers (M1-B) is a
  519-line file refactor.** The bulk-migration approach (per
  user-memory `feedback_subagent_orchestration.md`'s 5-grep
  batch lemma) keeps the developer time bounded at ~2.5 dev-days,
  but it IS a significant churn. We accept it because the
  current helpers produce **zero renderer coverage** — the
  ROI is high.

- **M0 H1 deviates from the chart-canvas-overhaul precedent.** That
  brief's "iced has a half-scale canvas bug" misdiagnosis (per
  [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only))
  cost 1.5 dev-days. **The architect is not concluding "H1 IS the
  bug"** — H1 is a hypothesis with an orchestrator-runnable
  falsifier. The orchestrator runs it; if it falsifies, route to
  developer. If it doesn't, move down the list. No conclusions
  without a falsifier verdict. **2026-05-14 update:** H1 came
  back UNFALSIFIED. The architect's 60% prior was wrong; the
  ladder shape (H1 first by lowest-cost) is the right discipline
  even when the prior misfires.

- **H3 assumption-falsified — empty-state path was the wrong
  trigger model.** The original H3 statement at this brief's M0
  section claimed the trigger was the empty-`rows` early-state
  path; orchestrator confirmed `rows` is non-empty by first frame.
  The M0-FIX design above abandons the `if rows.len() > 0`
  defensive gate the original H3 proposed and targets the
  populated-rows path via the `id_cell` rule Container (F1/F2)
  or the Table separator (F3). Named honestly so future
  architect passes see that confirmed-hypothesis ≠
  confirmed-mechanism.

- **F5 (Brief A R2 partial revert) is an architectural divergence
  if committed.** Brief A's
  [`iced-native-widgets/feature.md`](../iced-native-widgets/feature.md)
  shipped 2026-05-13 with the explicit decision "adopt
  `iced::widget::table::Table` for both positions and strategies."
  F5 walks back that decision for strategies only. ADR required
  per the `spec-update` and `architect.md` skill contracts (every
  non-trivial architectural decision = ADR); ADR file path is
  pre-allocated at
  `spec/cockpit-render-regression/architecture/adr-001-brief-a-r2-partial-revert.md`
  and will only be filed if F5 is the committed fix.

- **LLM-as-judge (M2-C) introduces non-determinism into the
  evaluator's evidence pack.** Two runs of Claude 4.7 on identical
  inputs may produce slightly different prose. We mitigate by (a)
  using `temperature: 0.0` in the API call, (b) requiring the
  human operator (not Claude) to make the final approve/reject
  call. Named honestly because the project's body-SHA anchor
  discipline ([`AGENT.md ## Anchor gate`](../../AGENT.md#process-discipline-lessons-from-v0--v15a))
  is allergic to non-determinism. M2-C lives in the presenter's
  decision-support layer, NOT in the body of any report that
  gets hashed.

- **`iced_test::Simulator` does not catch the current panic
  directly.** Per
  [docs.iced.rs/iced_test](https://docs.iced.rs/iced_test/index.html)
  WebFetch: *"The Simulator walks the widget tree rather than driving
  the renderer to produce pixels."* The render-snapshot proposal
  (M1-B) requires the additional
  `iced::advanced::renderer::Headless` step
  ([docs.iced.rs/iced/advanced/renderer/trait.Headless.html](https://docs.iced.rs/iced/advanced/renderer/trait.Headless.html))
  to rasterize. Named explicitly so the developer does not assume
  `Simulator` alone is sufficient.

## Numbers that matter

| Metric | Current value | Source |
|---|---|---|
| `cargo test -p ui` test count | 267 tests | Brief B evaluator report `reports/evaluation-2026-05-14T07-13Z.md` (cited by `iced-aw-cherry-pick/feature.md` changelog) |
| Two-run determinism gate | passes (zero `*.snap.new`) | Brief B T-M_FINAL-1 + tester convention |
| Real-iced-renderer test coverage today | **~0%** | Brief B developer-honest admission at `iced-aw-cherry-pick/tasks.md` T-M2-3 / T-M3-3 |
| Cockpit-smoke gate cost | **~7s** wall-clock per UI brief | Architect estimate (first-frame panic is at frame 1; 7s gives the iced runtime a comfortable warm-up margin) |
| Real-renderer snapshot CI cost | **~12.5s** per `cargo test -p ui` (~50ms × ~250 panels) | Architect estimate per `image-compare` docs |
| **M1 total dev-days** (M1-A + M1-B + M1-C) | **~4.25 dev-days** | 0.25 + 2.5 + 1.5 |
| **M2 total dev-days** (M2-A + M2-B + M2-C) | **~2.25 dev-days** | 0.75 + 1 + 0.5 |
| **Brief total dev-days** | **~6.5 dev-days** | Sum of M1 + M2 |
| File-span LOC delta | **+~720** (M1: +281; M2: +191; M0-FIX: +2 to +120 depending on F1 vs F5) | Per per-task estimates |
| Glue-layer LOC delta | **+~110** | Per per-task estimates |
| M0-FIX preferred candidate (F1) file-span LOC | **2** | strategies.rs:220 + strategies.rs:223 `Length::Fill → Length::Fixed(24.0)` |
| M0-FIX worst-case candidate (F5 revert) file-span LOC | **~80-120** | Brief A's `iced-native-widgets/feature.md` diff scope |
| iced_widget version (transitive) | **0.14.2** | `cargo tree -p ui \| grep iced_widget` |
| iced_tiny_skia version (renderer) | **0.14.0** | Unchanged since release; no published patch |
| Upstream iced master CHANGELOG ends at | **0.14.0** (2025-12-07) | [github.com/iced-rs/iced/CHANGELOG.md](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md) |
| Anchor risk | **0** | This brief touches `crates/ui/` only — zero strategy / audit / exec / backtest paths |
| PNG-baseline impact | **0** | The 3 existing `charts_screen_dark_*.png` baselines stay byte-identical; M1-B *adds* new baselines under a sibling directory |

## Out of scope

Per user-memory `trading_ui_library_constraints.md` + Brief B's
explicit Out-of-scope, these are NOT options on the table:

- **Renderer backend switch (tiny-skia → wgpu).** Large
  architectural change with its own performance / GPU-dep
  surface. Separate brief if ever proposed. The fact that
  iced-rs/iced#2774's fix landed on wgpu but not tiny-skia is
  noted, NOT a recommendation to migrate.
- **Forking iced or iced_aw.** Pinned per Brief B's "iced 0.14
  stays pinned" architecture decision.
- **`plotters-iced`, `iced_plot`, `iced-anim` family.** Off-table
  per `trading_ui_library_constraints.md`. Do not propose any of
  these as M2 "AI-driven UI design" candidates.
- **Whole-codebase refactor of widgets.** This brief proposes
  *targeted* changes (one constant flip for M0, ~250 LOC for
  M1-B PoC, ~30 widget impls for M2-A annotation pass). No
  widget rewrites.
- **Replacing `insta` with a new snapshot framework.** `insta`
  stays for the text-summary tests we keep (e.g. layout-token
  asserts at `frame.rs:380-435` are NOT in scope for M1-B —
  those are token unit tests, not panel renders).
- **Switching `iced_aw` widget consumption.** Brief B's B1/B2/B3
  shipped 2026-05-13; this brief uses them as-is.

## Open questions for orchestrator / operator

1. **M0 falsifier execution order — confirm H1 first.** Architect
   ranks H1 at 60% probability based on the `RIGHT_RAIL_WIDTH_PX
   = 0.0` evidence. Operator: confirm this is the run order, or
   request a different starting hypothesis (e.g. operator-known
   recent change that suggests a different culprit).
2. **M1-A skill cadence — every UI brief, or only the structural
   ones?** Architect proposes "every UI brief"; operator may
   prefer "only briefs that touched `crates/ui/src/widgets/` or
   `crates/ui/src/screens/`."
3. **M1-B SSIM threshold.** Architect proposes ≥ 0.99 (per
   [vizzly.dev](https://vizzly.dev/blog/honeydiff-fast-image-diffing-foundation/)'s
   "high SSIM score indicates preserved UI structure" guidance).
   Operator: confirm or override; lower values (≥0.95) reduce
   false positives but catch fewer real regressions.
4. **M2-C — does the operator want LLM-as-judge in the loop, or
   defer to a follow-up brief?** Architect can ship M1 + M2-A +
   M2-B without M2-C; M2-C is the most exploratory of the five
   proposals and adds API cost.
5. **Cockpit fixture-preset triage.** If H5 falsifies (modal
   auto-primed by fixtures), operator may need to specify which
   default preset the cockpit boots into — current
   `bin/cockpit.rs:141-143` comment ("Operators see the most
   recent feature set when they fixtures-boot the cockpit")
   suggests the preset is implicit.
6. **Brief sequencing.** Should M0 (diagnose + fix the panic) be
   one milestone and M1+M2 (the systemic overhaul) be a separate
   brief? Architect leans "one brief — M0 finishes in ≤1 dev-day,
   M1+M2 over the following 6 dev-days, all under one
   `cockpit-render-regression` slug." Operator overrides if a
   split is preferable.

## Verification (placeholder — tester fills in)

_Tester links reports here after the developer pass lands._

## Changelog

- 2026-05-14 (presenter, v1.0.0): Frontmatter bump — `version: 0.3.0 → 1.0.0`
  reflecting "production-ready" status now that the runtime regression on the
  v0.1.0 Brief A binding is closed. `owner: developer → presenter` (presenter
  now owns until operator ticks approve in
  [`presentations/cockpit-render-regression-2026-05-14.md`](../archive/presentations-2026-Q2.tar.gz);
  on tick the orchestrator flips `status: in-progress → shipped` per AGENT.md
  process discipline rule 2). Sibling frontmatter row updated to read "unblocked
  by this ship" (Brief B's hold lifts on F1 landing). Presentation assembled
  from the evaluator's `VERDICT → PASS` at
  [`reports/evaluation-2026-05-14T17-15Z.md`](reports/evaluation-2026-05-14T17-15Z.md)
  (log body-SHA-256
  `1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`). Three
  approval boxes ship UN-TICKED — operator owns the gate. No code or
  spec-content changes in this pass beyond the frontmatter row + this
  changelog entry; the M0 / M0-FIX / M1 / M2 prose below remains the
  architect's v0.2.0 + developer's v0.3.0 surface untouched.
- 2026-05-14 (developer, v0.3.0): F1 landed. The named constant
  `crate::theme::layout::STRATEGY_RULE_HEIGHT_PX = 24.0` was added
  at `crates/ui/src/theme.rs:619` (next to the sibling
  `RIGHT_RAIL_WIDTH_PX` row-height token) with a `///`-doc
  explaining the WHY (Length::Fill inside an iced Table cell
  resolves to 0 during the first frame's layout pass, triggering
  the `iced_tiny_skia` all-radii-zero-bound panic). The rule
  Container's two `Length::Fixed(24.0)` sites in
  `crates/ui/src/widgets/strategies.rs::id_cell` (lines 228 +
  231) now reference the named constant; the orchestrator's
  `// F1 FALSIFIER 2026-05-14 — was Length::Fill` diagnostic
  comments were replaced with a tight WHY block citing the
  constant and the spec. **T-FIX-1 ticked** with the three
  honest-tick citations (file:line, cockpit smoke cmd, panic
  count grep). **T-M0-FIX-VERIFY ticked** with full quality-gate
  output (`cargo build`, `cargo test -p ui` → 267 pass / 0 fail,
  `cargo fmt --check`, `cargo clippy --no-deps` → 0 NET-NEW on
  touched files, `cargo doc --no-deps` → 0 NET-NEW warnings on
  touched files). T-FIX-2 / T-FIX-3 / T-FIX-4 / T-FIX-5 marked
  `[~]` obsoleted by F1 (M0-FIX falsifier sequence stops on
  first FALSIFIED candidate; the remaining four candidates were
  never executed and retained only for spec-history). `spec/
  trace.toml`'s REQ-COCKPIT-PANIC-001 `crates` and `tests`
  columns populated with the two touched code files and the
  existing test-suite paths. HANDOFF → tester (test-runner +
  evaluator split; cockpit smoke gate remains orchestrator-only
  per [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries) —
  developer cites the orchestrator's pre-refactor falsifier
  log `/tmp/cockpit-f1-falsifier.log`). Open question for
  orchestrator: whether T-M0-FIX-VERIFY needs a post-refactor
  re-run of the cockpit smoke against the named-constant build
  (developer's belief: not strictly required because the named
  constant compiles to the exact same `24.0` literal and only
  `cargo test -p ui` regresses on a semantic mismatch — but the
  orchestrator owns that decision per capability boundaries).
- 2026-05-14 (architect, v0.2.0): M0 results integrated.
  Orchestrator-executed bisect confirmed H1 / H2 UNFALSIFIED, H3
  CONFIRMED with its original "empty-rows" assumption FALSIFIED
  (cockpit fixtures pre-populate rows by first frame; panic
  occurs with non-empty data). H4-H8 obsoleted by H3 confirmation.
  Culprit pinned to `crates/ui/src/widgets/strategies.rs:165`
  `table::Table::new(...)` call site, with the rule Container at
  `strategies.rs:217-227` as the leading suspect for the
  zero-bound `fill_quad` (per the positions/strategies cell-content
  asymmetry — positions does not panic; only strategies has
  `Length::Fill` inside a Table cell via `id_cell`). New M0-FIX
  section with five fix candidates F1-F5 ordered smallest-blast-
  radius first: F1 (2-line `Length::Fill → Length::Fixed(24.0)`,
  preferred), F2 (10-line stock `vertical_rule` swap), F3 (2-line
  `Table::separator_x(0).separator_y(0)`, diagnostic-distinct),
  F4 (Themer wrap via Catalog adapter — diagnostic, not a fix),
  F5 (Brief A R2 partial revert — last resort, ADR required).
  Red herrings explicitly ruled out: iced 0.14.x patch bump (the
  workspace already resolves `iced_widget 0.14.2` transitively;
  no published `iced_tiny_skia` patch beyond 0.14.0), Themer-wired
  Catalog adapter (sets separator COLOUR not THICKNESS; Table has
  no `.style()` builder per
  [docs.rs/iced_widget/0.14.2/iced_widget/table](https://docs.rs/iced_widget/0.14.2/iced_widget/table/struct.Table.html)).
  Honest divergences captured: M0 ladder beat the prior, H3
  assumption was wrong, F5-if-committed requires ADR. HANDOFF →
  orchestrator (execute M0-FIX falsifiers F1 → F2 → F3 → optional F4 → F5 last resort).
- 2026-05-14 (architect): initial draft v0.1.0. M0 hypothesis
  register with 8 falsifiable hypotheses (H1-H8) ordered cheapest
  first; H1 (right-rail `Length::Fixed(0.0)`) ranked highest-
  probability at ~60%. M1 quality-gate overhaul proposes
  `cockpit-smoke` skill (mandatory pre-tick gate, orchestrator
  capability boundary), real-renderer snapshot tests building on
  `iced_test = "=0.14.0"` + `image-compare = "=0.4"` dev-deps
  already shipped by `ui-test-harness-bootstrap`, and
  `proptest`-based layout invariants. M2 instrumentation proposes
  `tracing` spans on widget draw lifecycle (per CLAUDE.md
  "tracing not println"), opt-in `DebugRenderer` newtype wrapping
  `iced_tiny_skia::Renderer` for diagnostic builds, and a
  three-layer visual gate (Simulator → SSIM → LLM-as-judge) for
  the AI-driven testing layer the user explicitly asked for.
  Architectural divergences named honestly (custom Renderer
  wrapper, mandatory operator gate cost, 519-LOC test refactor,
  non-determinism in M2-C). Six open questions for orchestrator /
  operator. HANDOFF → orchestrator (execute M0 falsifiers in
  order; route to developer once culprit pinned).
