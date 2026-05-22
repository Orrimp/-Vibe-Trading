---
slug: ui-test-harness-bootstrap
version: 0.1.0
status: shipped
owner: operator
predecessor: chart-canvas-overhaul v1.10.0
updated: 2026-05-12
---

# UI test harness bootstrap (v0.1)

> First feature under the new
> [AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)
> regime (adopted 2026-05-12). Implements **week 1 only** of the 4-week
> adoption plan in
> [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md).
> Weeks 2-4 are separate features queued in the backlog after this ships.
> The week-1 snapshot test ALSO closes V15 of
> [`spec/chart-canvas-overhaul/feature.md`](../chart-canvas-overhaul/feature.md)
> (operator decision D4 in the dev-note's §9 — defer manual V15 capture
> to this feature's harness).

## Why

The chart-canvas-overhaul v1.10.0 retrospective surfaced a workflow bug,
not a chart bug: a 9-agent pipeline (analyst → architect → developer →
ui-designer → tester → re-spec → re-arch → re-dev → re-ui) shipped a
feature whose operator-verification step was a manual 30-second
`Cmd+Shift+4`. Two specific failures motivate this feature:

1. **The audit's single most damning finding** (per
   [dev-note §1](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#1-what-broke--evidence)):
   walking the 818-test suite, **no test would have caught the
   tooltip-invisible-at-3360×1890 bug**.
   [`crates/ui/tests/chart_tooltip_hover_fires.rs`](../../crates/ui/tests/chart_tooltip_hover_fires.rs)
   exercises hover-event detection at fixed canvas bounds `(100, 50,
   800×600)` — one viewport, no rendered pixels.
   [`crates/ui/tests/chart_tooltip_integration.rs`](../../crates/ui/tests/chart_tooltip_integration.rs)
   asserts `cockpit.chart_tooltip.is_some()` — one state assertion, no
   rendered pixels. The 68 existing insta snapshots are all
   text-summary; the 11 anchors are all backtest-report SHA. The
   evidence table on the dev-note's §1:

   | Failure | Evidence |
   |---|---|
   | Tester PASS verdict on 1280×720 capture, operator sees broken UI at 3360×1890 | prior `chart-buy-sell-emphasis` cycle |
   | Architect "iced canvas-scale bug" misdiagnosis | retracted in [`chart-canvas-overhaul/feature.md ## Diagnostic — CORRECTED`](../chart-canvas-overhaul/feature.md#diagnostic--corrected-2026-05-12-orchestrator-led) |
   | Multi-cycle (M6 → M6.2 → M7) on the same complaint | 1.5 dev-days of dead code on a perceptual misread |
   | 818 tests green, 0 cover the broken behavior | local audit confirms |

2. The
   [chart-canvas-overhaul V15 tooltip-hover acceptance](../chart-canvas-overhaul/feature.md#verification-v-items)
   currently requires a manual `screencapture` from the operator. Per
   the dev-note's §8 resolution path and operator decision **D4**, V15
   is deferred to **this feature's week-1 snapshot test** — the first
   `iced_test` snapshot test we write IS a chart-hover assertion at
   3360×1890, replacing the manual capture.

This brief authors the bootstrap that makes layered visual testing
possible. **It is test infrastructure only — no product surface
changes, no anchor changes, no non-UI crate changes.**

## Scope locked

Per operator decision block in
[dev-note §9](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator):

- **D1** — adopt all 5 TL;DR recommendations as a **single block**
  (they are load-bearing on each other). v0.1 lands the foundation
  (Layers 1 + 4 from §3); weeks 2–4 land the rest.
- **D2** — **full analyst → architect → developer pipeline**, NOT solo
  dev pass. This feature IS the bootstrap; the new pipeline shape must
  work for it to count as proven.
- **D3** — **macOS-only**. Windows/Linux is a separate future feature
  queued as `cockpit-cross-platform` candidate.
- **D4** — chart-canvas-overhaul V15 acceptance defers to this
  feature's week-1 snapshot test (no further manual capture).
- **D5** — AGENT.md `## Capability boundaries` amendment is LIVE
  (committed 2026-05-12). Individual `.claude/agents/*.md` files
  update later when the boundaries prove out.

### In scope (week 1 only)

- `iced_test::Simulator::snapshot()` integration as a new
  `crates/ui/tests/visual_snapshots.rs` test file
  ([iced_test 0.14 docs](https://docs.rs/iced_test/0.14.0/iced_test/))
- One snapshot test for the **Charts screen** at **three viewport
  sizes** — the dev-note's §3 Layer 3 viewport matrix:

  | Slot | viewport | scale_factor | rationale |
  |---|---|---|---|
  | floor | 1280 × 720 | 1.0 | `min_size` |
  | typical | 1920 × 1080 | 1.0 | new default per T3022 |
  | operator | 3360 × 1890 | 2.0 | actual hardware |

- `insta` binary snapshot integration so `cargo insta review` opens
  PNG diffs ([insta binary snapshots](https://insta.rs/docs/snapshot-types/)).
- Extension of `dispatch_canvas_event_for_test` at
  [`crates/ui/src/widgets/chart.rs:981`](../../crates/ui/src/widgets/chart.rs)
  so a unit test can sweep a cursor grid and assert hover detection at
  every marker centroid in each viewport. This is the test that would
  have caught the original `cursor.position_in(bounds)?` early-bail
  bug AND closes V15 of chart-canvas-overhaul.
- Confirmation that tiny-skia CPU determinism holds: PNG hashes match
  across two consecutive runs on the same machine (H1 falsifier
  below), and the test suite stays bit-stable when CI lands in week 4.

### Out of scope (separate features, separate analyst spawns)

- Viewport-matrix parameterization across ALL widget tests — **week 2
  feature**.
- Evaluator subagent + PreToolUse hooks — **week 3 feature**.
- GitHub Actions CI workflow + presenter integration — **week 4
  feature**.

### Explicit non-goals

- No feature surface changes (test infrastructure only).
- No `spec/anchors.toml` changes — the 11 backtest-report anchors stay
  byte-identical (bootstrap touches `crates/ui/` only).
- No `Cargo.toml` changes outside adding `iced_test` (already a
  workspace member of iced 0.14) and `image-compare` if needed.
- No changes to non-UI crates.
- No `dssim-core` (AGPL/commercial dual-license); only `image-compare`
  (MIT/Apache) is allowed if a perceptual-diff dependency is needed.

## Requirements

Functional requirements grouped by the dev-note's §3 layer that lands
them. Each `R*.x` is verifiable by a `V*` below.

### R1 — `iced_test` smoke (dev-note §3 Layer 1)

- **R1.1** A new test target `crates/ui/tests/visual_snapshots.rs`
  exists and is reachable from `cargo test -p ui --test
  visual_snapshots`.
- **R1.2** The test target drives the Charts screen via the
  free function
  [`iced_test::screenshot(&program, &theme, viewport, scale_factor, duration)`](https://docs.rs/iced_test/0.14.0/iced_test/fn.screenshot.html)
  — confirmed canonical by architect's M0 doc-audit (see Q2
  resolution). `iced_test::Simulator` is **not** the path: its
  `Snapshot` type exposes only `matches_image(path)` / `matches_hash(path)`
  and no public raw-byte accessor, so it cannot feed `insta`.
  `iced_test::screenshot` accepts an explicit `viewport: impl
  Into<Size>` + `scale_factor: f32`, which the dev-note's Layer 3
  matrix needs.
- **R1.3** The test asserts via `screenshot(...).matches_image(<baseline_path>)`
  rather than `insta::assert_binary_snapshot!`. First run auto-writes
  the baseline; subsequent runs byte-compare against it. **Q4 is
  resolved against insta integration in v0.1** — see Design §
  `cargo insta` gap.
- **R1.4** Baseline PNGs live under `crates/ui/tests/visual-baselines/`
  (sibling to the existing `crates/ui/tests/snapshots/` text-baseline
  tree). They are committed to git. See Q3 resolution.
- **R1.5** The Charts-screen test uses a deterministic fixture
  authored in `crates/ui/tests/fixtures/mod.rs` (new test-only module —
  see Q6 resolution) that bakes a `Cockpit.chart_tooltip =
  Some(ChartTooltipView{...})` state so the hovered-marker tooltip
  card renders without live cursor input (operator-locked Q9). The
  fixture lives alongside the text-snapshot fixture but does NOT
  reuse `panel_snapshots::charts_screen_with_counters_and_chart`'s
  exact scene — see Q9 lock.

### R2 — Viewport matrix (dev-note §3 Layer 3)

- **R2.1** The Charts-screen snapshot test runs at all three viewport
  slots (floor / typical / operator) per the operator-locked slot
  table (Q10). Baselines live at:
  - `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`
    (1280 × 720, scale_factor = 1.0)
  - `crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`
    (1920 × 1080, scale_factor = 1.0)
  - `crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`
    (3360 × 1890, scale_factor = 2.0)
- **R2.2** Each viewport slot is a discrete `#[test] fn` named for
  the slot (e.g. `charts_screen_dark_floor`,
  `charts_screen_dark_typical`, `charts_screen_dark_operator`) so a
  CI failure on the operator slot is immediately recognizable from
  the test-name alone. See Q3 resolution.
- **R2.3** Slot → (viewport, scale_factor) mapping is declared once
  as a `const SLOTS: &[(&str, (u32, u32), f32)]` table at the top of
  `visual_snapshots.rs`; each `#[test] fn` looks up its row by
  slot-name. Adding a fourth slot is one table row + one `#[test]
  fn` stub.
- **R2.4** All three baselines are committed to the repo. The
  developer pass writes them via the first `matches_image` invocation;
  the tester verifies bit-stability on a fresh checkout (V2 + H1
  falsifier).

### R3 — Canvas hit-test grid (dev-note §3 Layer 4 — the load-bearing one)

- **R3.1** A new test target
  `crates/ui/tests/chart_hover_grid_sweep.rs` (sibling to
  `chart_tooltip_hover_fires.rs` — Q5 resolution) iterates the
  cursor across a grid of cursor positions inside the canvas at
  each viewport size.
- **R3.2** For every marker centroid produced by the production
  `anchor_for_ts` math, the test asserts that
  `dispatch_canvas_event_for_test` returns
  `Some(Message::ChartMarkerHovered(...))` when the cursor is at the
  centroid, AND `None` (or `ChartMarkerHoverEnded`) when the cursor
  is at a known empty cell of the grid.
- **R3.3** The grid sweep runs at all three viewport sizes from R2.1
  (canvas bounds are computed from the viewport size — see Design).
  The test is the one we needed but never had: it would have caught
  the original `cursor.position_in(bounds)?` early-bail bug AND any
  future gutter-math regression that shifts centroids by a few
  pixels.
- **R3.4** The test does **not** require any rendering — it operates
  at the `dispatch_canvas_event_for_test` helper level. This is what
  makes it the cheap complement to R1/R2 and what runs sub-second
  even at 22k cells.
- **R3.5** Grid resolution is opt-in: a default `cargo test`
  invocation runs a coarse 32 logical-px grid for all three
  viewports; the dense sweep from feature.md's Q6 strawman (16 /
  16 / 24 px-per-cell, ~22k cells total) runs only when
  `CHART_HIT_TEST_GRID=dense` is set (Q5 resolution — see Design).
  This keeps CI fast on every PR while preserving the cheap-on-demand
  fine sweep for regression-bisect work.
- **R3.6** The viewport-parametric extension to
  `dispatch_canvas_event_for_test` (per dev-note §3 Layer 4)
  accepts a viewport `(w, h, scale)` and returns the canvas bounds
  the production `chart::view` would have computed at that
  viewport. Existing `chart_tooltip_hover_fires.rs` tests use the
  legacy single-bounds entry point unchanged (backward-compat
  guarantee).

### R6 — Perceptual diff for failure forensics (dev-note §3 Layer 5 — operator-locked Q8)

- **R6.1** When a `matches_image` assertion fails (post-baseline),
  the harness invokes
  [`image_compare::rgb_hybrid_compare`](https://docs.rs/image-compare)
  on the baseline vs. actual buffers and writes a visual-diff PNG
  to `target/visual-diff/<test_name>.png`. Operator opens the
  triple (baseline / actual / diff) to triage.
- **R6.2** `image-compare` is a `[dev-dependencies]` of `crates/ui`
  only — never reachable from production code. Licensed
  MIT/Apache; the AGPL `dssim-core` is explicitly excluded per the
  dev-note's §3 Layer 5 license guidance.
- **R6.3** The diff-write happens **inside the failing test's
  panic handler** (via a small helper module
  `crates/ui/tests/fixtures/visual_diff.rs`) so the integration
  cost is one helper call wrapping `matches_image` — see Design.
- **R6.4** The diff is **forensic only** — the assertion still
  fails on any byte mismatch. R6 does NOT loosen the determinism
  contract; it only makes the post-failure operator review faster.

### R4 — Determinism contract (cross-cutting)

- **R4.1** Two consecutive `cargo test -p ui --test visual_snapshots`
  runs on the same machine produce byte-identical PNGs (H1
  falsifier — see Hypothesis register).
- **R4.2** No `SystemTime::now()` / `Instant::now()` / wall-clock /
  pid / host / non-`UtcOffset::UTC` time-zone calls are reachable
  from the snapshot path. The existing `cfg(test)` override at
  `crates/ui/src/widgets/chart.rs:125-160` (`local_offset_or_utc()`)
  is the canonical pattern; new test paths reuse it.
- **R4.3** All RNGs (if any are reachable) use seeded
  `ChaCha20Rng::from_seed(...)` per the project's existing
  determinism non-negotiables in
  [AGENT.md ## Process discipline #5](../../AGENT.md#process-discipline-lessons-from-v0--v15a).

### R5 — Non-regression (cross-cutting)

- **R5.1** `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11/11)`
  before and after the developer pass; the 11 backtest-report SHA
  anchors stay byte-identical.
- **R5.2** All existing 818 tests stay green: `cargo test
  --workspace` produces the same pass count post-bootstrap (modulo
  the net-new visual_snapshots + grid tests this feature adds).
- **R5.3** Zero changes to non-UI crates. `git diff --name-only
  HEAD~..HEAD` post-developer-pass shows only files under
  `crates/ui/`, `Cargo.toml` (workspace deps for `iced_test` +
  optional `image-compare`), `Cargo.lock`, and `spec/`.

## Design (architect — 2026-05-12)

### Dependency changes

```toml
# crates/ui/Cargo.toml — [dev-dependencies] additions
iced_test     = "=0.14.0"   # ships with iced 0.14 — no version skew risk
image-compare = "=0.4"      # operator-locked Q8; MIT/Apache; rgb_hybrid_compare
```

`iced_test` 0.14 is pinned to match the workspace-pinned
`iced = "=0.14.0"` in [`Cargo.toml`](../../Cargo.toml). No workspace
`Cargo.toml` change is needed; both deps are test-only and confined to
`crates/ui/Cargo.toml [dev-dependencies]`. `Cargo.lock` will pick up
the two new entries + their transitive deps — `Cargo.lock` is part of
the in-scope diff (R5.3 already permits it).

### File / module layout

```
crates/ui/
├── Cargo.toml                                  # +iced_test, +image-compare
└── tests/
    ├── visual_snapshots.rs                     # NEW — three #[test] fn (R1, R2)
    ├── chart_hover_grid_sweep.rs               # NEW — grid sweep (R3, Q5)
    ├── fixtures/
    │   ├── mod.rs                              # NEW — shared test fixture builders
    │   └── visual_diff.rs                      # NEW — image-compare wrapper (R6)
    └── visual-baselines/                       # NEW — committed PNG baselines
        ├── charts_screen_dark_floor.png
        ├── charts_screen_dark_typical.png
        └── charts_screen_dark_operator.png
crates/ui/src/widgets/chart.rs                  # extend dispatch_canvas_event_for_test (R3.6)
scripts/check_no_clocks_in_ui_tests.sh          # NEW — V4 grep gate (Q7)
```

### Q1-Q7 resolutions (architect-decide)

- **Q1 (cockpit factory) → option (b) — thin test-only factory.**
  Add `pub fn for_charts_screen_test_program() -> impl iced_program::Program<Message = Message, Theme = iced::Theme>`
  at `crates/ui/src/lib.rs` (or sibling test-only module gated by
  `#[cfg(feature = "fixtures")]`). It wraps the existing
  `iced::application(App::boot, App::update, App::view)` call from
  [`src/bin/cockpit.rs:118`](../../crates/ui/src/bin/cockpit.rs)
  but seeds `App::boot` with a fixture-loaded `Cockpit` already
  navigated to the Charts screen. **Rationale:** option (a)
  (drive-via-`click(selector)`) is brittle to sidebar / navigation
  drift; option (c) (raw `widgets::canvas_chart`) skips the gutter /
  axes / legend chrome that was the very regression operator
  complained about. (b) hits the structural middle.
- **Q2 (iced_test PNG accessor) → use the free function
  `iced_test::screenshot(&program, &theme, viewport, scale_factor,
  duration)` + `Snapshot::matches_image(path)`.** *Rationale:* the
  docs-confirmed
  [`Snapshot`](https://docs.rs/iced_test/0.14.0/iced_test/simulator/struct.Snapshot.html)
  type exposes ONLY `matches_image(impl AsRef<Path>) → Result<bool, Error>`
  and `matches_hash(impl AsRef<Path>)` — there is no public
  `png()` / `into_image()` / `as_image()` accessor. The dev-note's
  "wire to `insta::assert_binary_snapshot!`" plan rests on a
  method that does not exist in 0.14.0. `iced_test::screenshot`
  returns a snapshot type with the same `matches_image` API and
  takes explicit viewport + scale_factor — exactly the surface
  the Layer 3 matrix needs. Q4 (insta integration) is **deferred to
  week 2** when we either PR upstream or read `Screenshot::rgba`
  via the renderer if iced_test gains the accessor.
- **Q3 (PNG baseline directory) → `crates/ui/tests/visual-baselines/`
  flat layout.** *Rationale:* `iced_test::Snapshot::matches_image`
  takes an absolute or relative file path of the test author's
  choosing — it has NO `cargo insta`-style convention to honour.
  A flat `visual-baselines/` sibling to the existing `snapshots/`
  text-baseline tree keeps PNGs out of the insta tooling's discovery
  path (preventing false-positive `cargo insta pending` noise) and
  makes the baseline set greppable as one folder.
- **Q4 (viewport-matrix parameterization) → three discrete `#[test]
  fn` per slot.** *Rationale:* analyst's strawman (a). Each
  `matches_image` call needs its own failing test for the operator
  to immediately recognize the broken viewport from the test-name;
  `#[rstest]` adds a dependency and a macro indirection for three
  rows; a single-test `for (w,h,s) in MATRIX` loop fails on the
  first slot and never reports the others.
- **Q5 (grid-sweep test placement + opt-in convention) → new sibling
  file `chart_hover_grid_sweep.rs` + env-var-gated dense mode.**
  *Rationale:* placement-wise, separating the grid sweep from
  `chart_tooltip_hover_fires.rs` keeps the existing focused-hover
  file under its current ~480-line scope. Density-wise, `cargo test`
  default runs the coarse 32-px sweep (~5k cells total — sub-200ms);
  `CHART_HIT_TEST_GRID=dense cargo test -p ui --test
  chart_hover_grid_sweep` runs the 16/16/24 px strawman (~22k cells
  total — under a few seconds). Env-var-gated chosen over `#[ignore]`
  because `#[ignore]` hides the dense-mode coverage from
  `cargo test --workspace`'s green-tally — and we want the run-count
  signal that `cargo test` always evaluated the cheap form.
- **Q6 (fixture authoring location) → new
  `crates/ui/tests/fixtures/mod.rs` test-only module.**
  *Rationale:* extending `crates/ui/src/fixtures.rs` would expand the
  production-reachable fixture surface (currently 1096 lines covering
  cockpit-state builders) for a test-only need; integration tests can
  share a sibling `mod fixtures;` import via `#[path]` or via a
  `tests/fixtures/mod.rs` Cargo convention. This also isolates the
  hovered-marker fixture (Q9 operator-lock) from any future
  production-fixture refactor.
- **Q7 (V4 form for "no clock reads on snapshot path") → shell
  script `scripts/check_no_clocks_in_ui_tests.sh` run from
  `rust-validate`.** *Rationale:* a cargo-deny ban on
  `std::time::SystemTime` is too coarse (the existing
  `local_offset_or_utc` test override at
  [`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs)
  intentionally uses `time` for the `#[cfg(test)]` override). A
  shell-grep allow-list is precise: forbid `SystemTime::now`,
  `Instant::now`, `thread_rng`, `UtcOffset::current_local_offset`
  reachable from `crates/ui/src/widgets/chart.rs` (rendering path)
  and `crates/ui/src/screens/lab.rs` (Charts-screen path), with
  an explicit whitelist comment marker (`// CLOCK-OK:`) for
  intentional uses. Compile-fail (`trybuild`) is over-engineering.

### Q8/Q9/Q10 — operator locks (recorded for non-revisit)

- **Q8 — `image-compare` IN week 1 (richer scope).** Operator
  chose the half-day cost of integrating `image-compare` from
  bootstrap rather than carrying determinism risk into week 2. The
  perceptual-diff PNG to `target/visual-diff/<test>.png` materializes
  on every failing `matches_image` call so the operator's triage
  view is always present, never a "rerun with --feature diff"
  follow-up. R6 above is the encoded contract; T-NNNN tasks in
  M1 wire the helper.
- **Q9 — richer fixture with `chart_tooltip = Some(view)` baked
  into the scene.** Operator chose to make the V15 closure
  (chart-canvas-overhaul tooltip-hover acceptance) renderable from
  a snapshot without needing live cursor input. The fixture diverges
  from
  [`charts_screen_with_counters_and_chart`](../../crates/ui/tests/panel_snapshots.rs)
  — that text snapshot's scene has no hovered tooltip. The new
  fixture (`fixtures::charts_screen_with_hovered_marker`) seeds the
  same bar series + fills + signals + position-mirror data but ALSO
  pre-populates `Cockpit.chart_tooltip = Some(ChartTooltipView {
  ... })` against the first fill marker. The text-snapshot fixture
  stays unchanged; the binary-snapshot fixture is a superset.
- **Q10 — slot names for baselines:** `floor` (1280×720 @1.0),
  `typical` (1920×1080 @1.0), `operator` (3360×1890 @2.0). Slot
  names (not dimension-strings) are the source of truth — the
  test names are `charts_screen_dark_floor`,
  `_typical`, `_operator`. The slot→(viewport, scale) map is the
  `const SLOTS` table in `visual_snapshots.rs` (R2.3).

### Viewport-matrix mechanism

```rust
// crates/ui/tests/visual_snapshots.rs (sketch)
const SLOTS: &[(&str, (u32, u32), f32)] = &[
    ("floor",    (1280, 720),  1.0),
    ("typical",  (1920, 1080), 1.0),
    ("operator", (3360, 1890), 2.0),
];

fn run_slot(slot_name: &str) {
    let (_, (w, h), scale) = SLOTS.iter().find(|(s, _, _)| *s == slot_name).unwrap();
    let program = ui::for_charts_screen_test_program();
    let theme = iced::Theme::Dark;
    let viewport = iced::Size::new(*w as f32, *h as f32);
    let snap = iced_test::screenshot(&program, &theme, viewport, *scale, Duration::from_millis(0));
    let baseline = format!("tests/visual-baselines/charts_screen_dark_{slot_name}.png");
    fixtures::visual_diff::matches_image_with_diff(snap, &baseline, slot_name)
        .expect("snapshot mismatch — see target/visual-diff/");
}

#[test] fn charts_screen_dark_floor()    { run_slot("floor"); }
#[test] fn charts_screen_dark_typical()  { run_slot("typical"); }
#[test] fn charts_screen_dark_operator() { run_slot("operator"); }
```

The `matches_image_with_diff` helper wraps
`Snapshot::matches_image(path)` and, on `Ok(false)`, decodes both
baseline and actual PNGs into `image::RgbImage`, runs
`image_compare::rgb_hybrid_compare`, writes
`target/visual-diff/charts_screen_dark_<slot>.png`, then panics with
a message that cites both the baseline path and the diff path. On
`Err(_)` (baseline doesn't exist) the helper auto-creates the
baseline silently — matching iced_test's first-run semantics.

### Canvas hit-test grid-sweep design

The viewport-parametric extension to `dispatch_canvas_event_for_test`
adds a sibling helper at
[`crates/ui/src/widgets/chart.rs:981`](../../crates/ui/src/widgets/chart.rs):

```rust
/// Test-only — viewport-parametric wrapper for grid-sweep tests.
/// Computes the canvas bounds the production `chart::view` would
/// produce at the given viewport, then sweeps `cursor_positions`
/// through `dispatch_canvas_event_for_test`.
#[doc(hidden)]
#[must_use]
pub fn sweep_canvas_grid_for_test(
    bars: Vec<Bar>,
    markers: Vec<FillView>,
    signals: Vec<SignalView>,
    viewport: (u32, u32),
    scale_factor: f32,
    cursor_positions: Vec<Point>,
) -> Vec<(Point, Option<Message>, iced::event::Status)> { ... }
```

The existing `dispatch_canvas_event_for_test` stays exactly as it is
(backward-compat — R3.6). The new wrapper computes `Rectangle` bounds
from `(viewport, scale_factor)` using the same gutter math the
production `widgets::canvas_chart::inner_rect_with_gutters` uses
(referenced from
[`chart_tooltip_hover_fires.rs:GUTTER_PX`](../../crates/ui/tests/chart_tooltip_hover_fires.rs)),
then loops `dispatch_canvas_event_for_test` for each cursor position.

The grid-sweep test
(`crates/ui/tests/chart_hover_grid_sweep.rs::cursor_grid_sweeps_every_marker_at_three_viewports`)
walks the `SLOTS` table, builds a `Vec<Point>` of `(step, step)` grid
positions for each viewport, calls `sweep_canvas_grid_for_test`, and
partitions results: every cursor position within ±`HIT_RECT_HALF_PX`
of an `anchor_for_ts(...)` centroid MUST produce
`Some(Message::ChartMarkerHovered(..))`; every other position MUST
produce `None` or `Some(Message::ChartMarkerHoverEnded)`.

V8 (chart-canvas-overhaul V15 closure) is the
specific assertion that at the `operator` slot
(3360×1890 @2.0), a cursor at the first fill marker's centroid
publishes `ChartMarkerHovered(Fill(0))` with `Status::Captured` —
named `v15_chart_canvas_overhaul_closure_at_operator_slot` for the
tester's verify-anchors row.

### `cargo insta review` integration gap

The dev-note's §3 Layer 2 plan ("extend insta to binary snapshots")
rests on extracting raw PNG bytes from `iced_test::Snapshot`. The
0.14.0 surface confirmed by M0 audit
([Snapshot docs](https://docs.rs/iced_test/0.14.0/iced_test/simulator/struct.Snapshot.html))
exposes no public raw-byte accessor — only `matches_image(path)` /
`matches_hash(path)`. v0.1 therefore uses `matches_image` directly;
the `cargo insta review` workflow does NOT see the PNG baselines.

Operator-facing failure flow in v0.1:
1. `cargo test -p ui --test visual_snapshots` fails.
2. Helper has written `target/visual-diff/charts_screen_dark_<slot>.png`
   alongside an `actual.png` copy.
3. Operator opens the triple manually (Finder / `open`); decides
   accept-or-reject.
4. To accept: delete the old baseline and rerun (iced_test
   auto-rewrites it on the next pass).

The `cargo insta review` shortcut returns in week 2 once either
(a) iced_test exposes a byte accessor (PR candidate from this team),
or (b) we switch to extracting bytes via the renderer-internal
`Screenshot::rgba` field via a thin `iced_test`-internal adapter we
maintain.

### Fixture authoring strategy

`crates/ui/tests/fixtures/mod.rs` (new, test-only) hosts:

- `charts_screen_with_hovered_marker() -> Cockpit` — the operator-locked
  Q9 fixture. Constructs the same scene as
  `panel_snapshots::charts_screen_with_counters_and_chart` (3 bars,
  3 fills, 2 signals, position mirror) but ALSO pre-populates
  `cockpit.chart_tooltip = Some(ChartTooltipView { ... })` against
  the leftmost fill's `(symbol, side, price, qty, fee, fee_tier,
  venue_ts)` so the rendered Charts screen shows the tooltip card.
  Mirrors the field shape at
  [`state.rs:599`](../../crates/ui/src/state.rs) (`pub struct
  ChartTooltipView`).
- `charts_screen_program() -> impl Program` — wraps the
  Q1-resolution factory call so the visual_snapshots tests and any
  future iced_test-driven tests share a single program-build helper.

`crates/ui/tests/fixtures/visual_diff.rs` (new, test-only) hosts:

- `matches_image_with_diff(snap: Snapshot, baseline_path: &str,
  test_name: &str) -> Result<(), Error>` — the helper described under
  R6.3. Decodes baseline + actual into `image::RgbImage`, runs
  `image_compare::rgb_hybrid_compare`, writes the diff PNG, panics
  with a multi-line cite-the-paths message.

### Determinism contract (R4 details)

The existing `local_offset_or_utc()` test override at
[`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs) is the
canonical pattern. No new clock calls are reachable from
`for_charts_screen_test_program`'s state path because the fixture
hard-codes `Timestamp::new(OffsetDateTime::from_unix_timestamp(...))`
just like
[`chart_tooltip_hover_fires::fixed_ts`](../../crates/ui/tests/chart_tooltip_hover_fires.rs).
The `check_no_clocks_in_ui_tests.sh` script (Q7 resolution) enforces
this with a grep + an explicit `// CLOCK-OK:` whitelist marker.

## Acceptance criteria (V-items)

Per the new [AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)
rule: **no acceptance criterion may require a sub-agent to capture a
screenshot or run the cockpit binary**. Every V-item below is either a
`cargo test` invocation, a `scripts/verify_anchors.sh` invocation, or
an artifact the orchestrator generates from inside its own shell.

- **V1 — `iced_test` smoke compiles + passes.** `cargo test -p ui
  --test visual_snapshots` exits 0 with the three slot-named
  `#[test] fn`s green. (Closes R1.1–R1.5.) Sandbox-safe: pure
  `cargo test`.
- **V2 — Three viewport baselines committed and bit-stable.** Running
  `cargo test -p ui --test visual_snapshots` twice consecutively on
  the same machine produces zero diff under `target/visual-diff/`
  after the second run AND `git status crates/ui/tests/visual-baselines/`
  shows zero modifications after the second run. (Closes R2.1–R2.4
  + R4.1 — falsifier for H1.) Sandbox-safe: pure `cargo test` + `git
  status`.
- **V3 — Canvas hit-test grid sweeps all centroids at all three
  viewports.** `cargo test -p ui --test chart_hover_grid_sweep
  cursor_grid_sweeps_every_marker_at_three_viewports` exits 0; on
  every viewport, every marker centroid produced by `anchor_for_ts`
  fires a `Hovered` message and every empty grid cell does not.
  (Closes R3.1–R3.6.) Sandbox-safe: pure `cargo test`.
- **V4 — Determinism contract holds.** `bash
  scripts/check_no_clocks_in_ui_tests.sh` exits 0 on the clean tree
  AND exits non-zero when a `SystemTime::now()` is temporarily
  introduced into `crates/ui/src/widgets/chart.rs` (proven by the
  developer's tick-proof experiment — git-stash-able). (Closes
  R4.1–R4.3.) Sandbox-safe: pure shell grep.
- **V5 — `verify_anchors.sh` PASS 11/11 before and after.** The
  tester runs `bash scripts/verify_anchors.sh` and the report cites
  the `ANCHORS PASS (11/11)` line. (Closes R5.1.) Sandbox-safe:
  pure shell.
- **V6 — Full workspace test suite stays green.** `cargo test
  --workspace` produces ≥ 818 passing tests + the net-new ones this
  feature adds (V1 + V3 + any sub-tests), with zero failures.
  (Closes R5.2.) Sandbox-safe: pure `cargo test`.
- **V7 — Zero changes to non-UI crates.** `git diff --name-only
  HEAD~..HEAD` against the pre-feature commit shows file changes
  only under `crates/ui/`, root `Cargo.toml`, `Cargo.lock`,
  `scripts/check_no_clocks_in_ui_tests.sh`, and `spec/`. The tester
  pastes the diff list into the report. (Closes R5.3.) Sandbox-safe:
  pure git.
- **V8 — chart-canvas-overhaul V15 closure.** A named sub-test
  `v15_chart_canvas_overhaul_closure_at_operator_slot` in
  `chart_hover_grid_sweep.rs` exits 0; at the operator slot
  (3360×1890 @2.0), a cursor at the first fill marker's centroid
  publishes `ChartMarkerHovered(Fill(0))` with `Status::Captured`.
  This is the assertion that V15 of chart-canvas-overhaul originally
  required a manual screencapture for (per dev-note §8 + operator
  D4). Sandbox-safe: pure `cargo test`.
- **V9 — Perceptual diff materializes on `matches_image` failure.**
  A targeted test
  `visual_diff_helper_writes_diff_png_on_mismatch` in
  `crates/ui/tests/fixtures/visual_diff.rs` (run as a
  `#[test]` inside a `#[cfg(test)] mod tests`) deliberately compares
  two different `image::RgbImage` buffers, asserts the helper panics
  AND that `target/visual-diff/<test_name>.png` exists post-panic
  (caught via `std::panic::catch_unwind`). (Closes R6.1–R6.4.)
  Sandbox-safe: pure `cargo test` — no live cockpit, no screencapture.

**Critical workflow rule** (per
[AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)):
all V-items above are executable from a sub-agent's sandbox. None
require a display server, `screencapture`, `osascript`, or a live
cockpit window. If during architect / developer work a Q surfaces
that genuinely needs the operator to look at a rendered window, it
escalates as an **operator-input Q** in this brief — the orchestrator
does the screencap.

## Non-regression contract

- 11 anchors stay byte-identical
  ([`spec/anchors.toml`](../anchors.toml)).
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11/11)` before
  and after.
- Existing 818 tests stay green: `cargo test --workspace` passes
  with at least the prior pass count.
- Zero changes to non-UI crates: only `crates/ui/`, root
  `Cargo.toml` (workspace deps), `Cargo.lock`, and `spec/` may
  change.
- No changes to `crates/strategy/`, `crates/audit/`, `crates/exec/`,
  `crates/backtest/`, `crates/reports/`, `crates/risk/`,
  `crates/core/`, `crates/agent/`, `crates/data/`, or any other
  non-UI crate.

## Hypothesis register

Per the new [AGENT.md ## Capability boundaries](../../AGENT.md#architect--hypothesis-only)
rule: architects author hypotheses with **explicit falsifiers**; the
orchestrator runs the empirical test that refuses to falsify.
Hypotheses without orchestrator-run falsification are first-class spec
artifacts (not blockers, just unresolved).

The analyst seeds the register with one hypothesis identifiable at
brief time:

- **H1 — tiny-skia CPU determinism holds across two runs on the same
  machine.**
  - *Statement:* `iced_test::Simulator::snapshot()` invoked twice in
    a row in the same `cargo test` process (same fixture, same theme,
    same viewport, same scale_factor) produces byte-identical PNG
    output. The
    [tiny-skia README](https://github.com/linebender/tiny-skia)
    documents "expected to match Skia pixel-for-pixel" and we already
    pin `iced_tiny_skia` in
    [`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml).
  - *Falsifier:* the orchestrator runs `cargo test -p ui --test
    visual_snapshots` twice consecutively against a checked-in
    baseline. If the second run produces a non-empty `cargo insta
    pending` diff → tiny-skia determinism assumption is wrong; STOP
    and re-scope toward `matches_hash`-with-perceptual-tolerance
    (dev-note §3 Layer 5) instead of strict-byte
    `assert_binary_snapshot!`.
  - *Status:* unresolved — falsified by V2.

Architects authoring the Design section append additional hypotheses
here. The orchestrator does not need to falsify all of them before
the developer pass starts — falsification is part of the developer +
tester cycle, not a precondition for the architect handoff.

- **H2 (architect, 2026-05-12) — `iced_test::screenshot(&program, &theme, viewport, scale_factor, Duration::ZERO)`
  produces a fully-rendered frame on first call (no Subscription
  pump required).**
  - *Statement:* The cockpit's `App::view` is pure
    function-of-state — there's no `iced::Subscription` pulling
    deferred messages that the renderer needs to consume before
    the first frame is "settled". Driving the screenshot with
    `Duration::ZERO` should therefore produce a frame that visually
    matches running the same `App::view(state)` under a real iced
    runtime.
  - *Falsifier:* the orchestrator runs the floor-slot test once
    and operator visually compares the resulting baseline PNG
    against a manual `cargo run --bin cockpit` capture. If the
    snapshot shows pre-`Task::done` placeholder UI (e.g. a loading
    spinner where data should be), H2 is falsified and the
    Design's `Duration::ZERO` becomes `Duration::from_millis(50)`
    or however long the longest `App::boot` task takes to resolve.
  - *Status:* **RESOLVED-WITH-CAVEAT (orchestrator + operator,
    2026-05-12).** Operator visually reviewed all three baselines
    at
    [`crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png`](../../crates/ui/tests/visual-baselines/).
    Axes (left price gutter USD + bottom time gutter HH:MM),
    legend top-right (5-row Buy/Sell/Buy-signal/Sell-signal/Price),
    markers, status strip, sidebar Charts-active all render fully
    — H2's "fully-rendered frame on first call" assertion holds
    for these surfaces, so **`Duration::ZERO` is the correct
    setting**.  CAVEAT: the Q9 fixture's `Cockpit.chart_tooltip =
    Some(ChartTooltipView{..})` does NOT manifest as a visible
    tooltip card in the rendered PNGs because the chart-buy-sell-
    emphasis v1.9.0 T2033 refactor decoupled tooltip rendering
    from Cockpit state — the canvas reads hover state from its
    own internal `ChartProgram::State`, not from `self.tooltip`.
    That gap means **V8 (chart-canvas-overhaul V15 closure) is
    PARTIAL** in v0.1: the **detection** half is covered by
    T4022/T4023 grid-sweep tests; the **render** half awaits a
    canvas-state-seeding extension queued as week-2 follow-up
    (`ui-test-harness-canvas-state-seeding` candidate in
    [`spec/backlog.md`](../backlog.md#process--tooling)). Operator
    decision **"Commit — V14 covered, V15 partial-accept"** logged
    2026-05-12; the baselines are committed at the current state.

- **H3 (architect, 2026-05-12) — The viewport-parametric extension
  to `dispatch_canvas_event_for_test` correctly recreates production
  canvas bounds at non-default viewports.**
  - *Statement:* The production
    [`widgets::canvas_chart::inner_rect_with_gutters`](../../crates/ui/src/widgets/chart.rs)
    math is purely a function of the canvas's `Rectangle` bounds —
    given `(viewport_w, viewport_h, scale_factor)`, the new
    `sweep_canvas_grid_for_test` helper can compute the exact
    `Rectangle` the production widget would see at that viewport.
    No iced layout-engine round-trip is needed.
  - *Falsifier:* a sanity sub-test
    `sweep_helper_bounds_match_simulator_layout` runs both
    `sweep_canvas_grid_for_test` AND `iced_test::screenshot` against
    the same viewport, then queries the chart-canvas widget's
    bounds via an iced_test selector (text- or position-based) and
    compares to the helper's computed bounds. If the two differ by
    >1 px, H3 is falsified and the grid-sweep test must instead
    drive bounds via the simulator's accessibility layout pass.
  - *Status:* unresolved — falsifier added as a sub-task in M2.

- **H4 (architect, 2026-05-12) — `image-compare`'s
  `rgb_hybrid_compare` produces a human-actionable diff PNG when
  fed two near-identical PNGs (sub-pixel antialiasing drift).**
  - *Statement:* On the failure cases that motivate R6 (text
    antialiasing drift from a macOS minor-version bump,
    sub-pixel gutter math regression), `image-compare`'s hybrid
    SSIM+RMS diff PNG visually highlights the changed region —
    not a uniform grey field.
  - *Falsifier:* the operator runs the developer's V9 test
    (`visual_diff_helper_writes_diff_png_on_mismatch`) which
    compares two deliberately-shifted-by-2px text fixtures, then
    visually inspects the resulting diff PNG. If the diff is
    indistinguishable from full-grey (no localized red/yellow
    high-delta region), H4 is falsified and we either tune the
    `image-compare` algorithm parameters or fall back to
    `twenty-twenty` (dev-note §3 Layer 5 alternative).
  - *Status:* unresolved — falsified by V9 + operator inspection
    during presenter pass.

- **H5 (architect, 2026-05-12) — The Q1 test-only factory
  `for_charts_screen_test_program` can be authored without
  pulling the `agent` / `audit` / `tokio` deps in the `live`
  feature gate.**
  - *Statement:* `iced_test` runs the cockpit's
    `App::boot / update / view` triple. None of those touch the
    live broadcast bus when the seeded `Cockpit` state already
    has `PanelState::Ready(...)` for every panel. The factory
    therefore lives in the default-features build and does NOT
    require `--features live` / `--features fixtures` at
    compile time.
  - *Falsifier:* the developer's M1 `cargo build -p ui --tests`
    succeeds with default features only. If it requires
    `--features fixtures` to compile, H5 is falsified and the
    factory either moves under `#[cfg(any(test, feature =
    "fixtures"))]` or the `tests/` integration tests add a
    `required-features = ["fixtures"]` cargo-target gate.
  - *Status:* unresolved — falsified by T4011 in tasks.md.

## Open questions

> **2026-05-12 architect update — all 10 Qs resolved.** Q1–Q7
> resolved by architect-decide with rationale in
> `## Design ## Q1-Q7 resolutions`. Q8–Q10 operator-locked
> upstream of the architect spawn; encoded in `## Design ##
> Q8/Q9/Q10 — operator locks`. Original Q text preserved below
> for audit / re-litigation reference; **do not re-route to
> operator** — orchestrator goes straight to developer spawn.

- **Q1 (architect-decide, blocking-on-arch-spawn) — Cockpit factory for
  the Charts screen.** `iced_test::Simulator::new(program)` needs a
  `Program` instance. Options: (a) drive the full `Cockpit` app and
  navigate via `Simulator::click(selector)` to the Charts screen; (b)
  expose a thin test-only factory `Cockpit::for_charts_screen_test()`
  that hardcodes the screen state; (c) construct
  `widgets::canvas_chart` directly without the surrounding shell.
  *Recommended:* (b) — (a) is brittle to sidebar drift, (c) skips
  the shell-frame regression that operator complained about (gutter,
  axes, legend chrome).
- **Q2 (architect-decide, low-risk) — `iced_test::Snapshot::png()`
  surface.** The dev-note assumes a `Snapshot::png()` accessor; the
  actual iced_test 0.14 API may expose pixel bytes via a different
  method name (e.g. `into_image()`, `as_png()`, or via a
  `tiny_skia::Pixmap` extractor). Architect verifies from
  [docs.rs/iced_test/0.14.0](https://docs.rs/iced_test/0.14.0/iced_test/)
  during their design pass and locks the exact call shape in the
  task body.
- **Q3 (architect-decide, low-risk) — PNG baseline directory
  convention.** Existing text snapshots live at
  `crates/ui/tests/snapshots/<test_target>__<test_name>.snap`. Options:
  (a) PNG baselines colocate at the same path with a `.png` extension
  via `insta::assert_binary_snapshot!`; (b) PNG baselines live under a
  new `crates/ui/tests/snapshots/visual/` subfolder for grep-ability.
  *Recommended:* (a) — `cargo insta review` already handles
  per-test discovery, splitting into subfolders adds friction.
- **Q4 (architect-decide, low-risk) — Viewport matrix
  parameterisation.** Options: (a) one `#[test] fn` per slot, with
  shared helper; (b) `#[rstest]` parametrisation; (c) a manual
  `for (w, h, scale) in MATRIX` loop inside one `#[test] fn`.
  *Recommended:* (a) — each slot is a separate
  `assert_binary_snapshot!` invocation; one `#[test] fn` per slot
  keeps the `cargo insta review` flow per-baseline and per-failure;
  `#[rstest]` is over-engineering for three rows.
- **Q5 (architect-decide, low-risk) — Grid-sweep test placement.**
  Should the canvas hit-test grid (R3) live (a) as new `#[test] fn`s
  in the existing
  [`crates/ui/tests/chart_tooltip_hover_fires.rs`](../../crates/ui/tests/chart_tooltip_hover_fires.rs)
  alongside the existing hover-event tests, or (b) as a new sibling
  `crates/ui/tests/chart_hover_grid_sweep.rs`? *Recommended:* (b) —
  grid sweep is a new test class (parameterised over viewports);
  keeping it sibling avoids bloating the existing focused file.
- **Q6 (architect-decide, medium-risk) — Grid resolution.** R3.5
  punts grid density. Tighter = catches smaller pixel shifts; looser
  = faster test. Strawman: floor 16 px/cell (80×45 grid = 3600
  cells), typical 16 px/cell (120×67 = 8040 cells), operator
  24 logical px/cell (140×79 = 11060 cells). The dispatch helper
  is cheap (no rendering), so even ~22k cells per run is sub-second.
  Architect ratifies or tightens.
- **Q7 (architect-decide, low-risk) — V4 form for "no clock reads on
  snapshot path".** Options: (a) a `cargo test` that uses the
  `trybuild`-style compile-fail or `cargo-deny` ban on
  `std::time::SystemTime`; (b) a shell script
  `scripts/check_no_clocks_in_ui_tests.sh` run as part of
  `rust-validate`; (c) a documented manual grep step in the
  task description and trust the developer. *Recommended:* (b) —
  a small shell script the tester runs alongside `verify_anchors.sh`.
- **Q8 (operator-input, blocking-on-operator) — Is `image-compare`
  needed in week 1?** The dev-note's Layer 5 (perceptual diff)
  motivates `image-compare` for fuzzy regions but explicitly says
  "most snapshots use exact byte comparison via `matches_hash`". H1
  predicts byte-identical PNGs hold for our cockpit. *Recommended:*
  defer `image-compare` to week 2 unless H1 falsifies. Operator may
  pre-emptively approve pulling it in week 1 if they want
  belt-and-braces tolerance for text antialiasing drift across
  macOS minor versions.
- **Q9 (operator-input, blocking-on-architect-spawn) — Charts-screen
  fixture parity with text snapshots.** The existing
  [`crates/ui/tests/panel_snapshots.rs`](../../crates/ui/tests/panel_snapshots.rs)
  has `charts_screen__chip_row_active_btc` /
  `charts_screen__chip_row_active_eth` /
  `charts_screen_with_counters_and_chart` baselines. *Recommended:*
  the v0.1 visual snapshot uses the SAME fixture as
  `charts_screen_with_counters_and_chart` so the text + binary
  baselines describe the same scene. Operator confirms or asks for
  a richer fixture (e.g. with a hovered marker).
- **Q10 (operator-input, deferred-OK) — Naming convention for the
  three viewport baselines.** Strawman:
  `charts_screen_dark_1280x720_1x.png`,
  `charts_screen_dark_1920x1080_1x.png`,
  `charts_screen_dark_3360x1890_2x.png`. Operator may prefer slot
  names (`floor`, `typical`, `operator`) — readability vs.
  self-documentation tradeoff.

## Predecessors / related

- [`spec/chart-canvas-overhaul/feature.md`](../chart-canvas-overhaul/feature.md)
  — the retrospective that motivates this feature. V15 closes here
  via V8 above.
- [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)
  — the strategy doc. This feature implements §3 Layers 1 + 4 only;
  Layers 2 (`insta` binary snapshots) lands in support of L1 here;
  Layers 3 (full viewport matrix across all widgets) and 5
  (`image-compare`) defer to weeks 2 / 4.
- [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)
  — the workflow amendment whose first-real-feature test is this
  one. The pipeline (analyst → architect → developer → tester) must
  work cleanly without a single sub-agent attempting a screencapture
  or live cockpit launch — that is the bootstrap's load-bearing
  proof.
- Workspace `iced` pin in
  [`Cargo.toml:69`](../../Cargo.toml) — `iced = { version =
  "=0.14.0", default-features = false, features = ["tiny-skia",
  "thread-pool", "advanced", "canvas"] }`. `iced_test` is a member
  of the iced 0.14 workspace; pulling it in is a new dev-dependency
  in `crates/ui/Cargo.toml`, not a workspace `iced` version bump.

## Changelog

- 2026-05-12 (analyst): initial draft. Scope locked to week-1 only
  of the [dev-note §6 adoption plan](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  10 open questions for the architect / operator. H1 seeded in
  Hypothesis register. Non-regression contract: 11 anchors stay
  byte-identical, 818 tests stay green, zero non-UI-crate changes.
  HANDOFF → architect (Design section + tasks.md fleshing).
- 2026-05-12 (architect): Design section authored. Q1–Q7 resolved
  architect-decide (rationale inline). Q8/Q9/Q10 operator-locks
  encoded. Critical API correction: `iced_test::Snapshot` 0.14.0
  exposes only `matches_image(path)` / `matches_hash(path)` — no
  public PNG-byte accessor — so v0.1 uses `iced_test::screenshot`
  (viewport+scale-controlled free function) + `matches_image`
  directly. Insta binary-snapshot integration deferred to week 2.
  R6 (perceptual diff via `image-compare`) added per Q8 lock.
  V9 added for R6 self-test. H2–H5 architect-authored with
  explicit falsifiers. Open Questions section frozen — all
  resolved. HANDOFF → orchestrator (developer can spawn sequential).
- 2026-05-12 (developer): M1 (T4011–T4015) + M2 (T4021–T4023) + M3
  partials landed. **Second architect-API correction:**
  `iced_test::screenshot` returns `iced::window::Screenshot { rgba,
  size, scale_factor }` directly — no `Snapshot` indirection.
  Visual-diff helper byte-compares `Screenshot.rgba` against
  baseline-PNG-decoded RGBA. New files: `test_support.rs`,
  `tests/fixtures/{mod.rs,visual_diff.rs}`, `tests/visual_snapshots.rs`,
  `tests/chart_hover_grid_sweep.rs`, three baseline PNGs at
  `tests/visual-baselines/`, `scripts/check_no_clocks_in_ui_tests.sh`.
  Anchors PASS 11/11. Frontmatter `owner: architect → developer`.
- 2026-05-12 (orchestrator): H1 (determinism) + T4022 dense-mode
  (0.11s) + T4031 clocks-grep clean + V4 inject-and-stash all
  PASS verbatim. Two consecutive `cargo test -p ui --test
  visual_snapshots` runs produced byte-identical SHAs on all three
  baselines — tiny-skia CPU determinism confirmed empirically.
- 2026-05-12 (operator): **H2 RESOLVED-WITH-CAVEAT**.  Baselines
  committed. V14 (axes + legend + layout) closed by all three
  PNGs.  V8 / chart-canvas-overhaul V15: detection covered by
  T4022/T4023 grid sweep; render gap acknowledged; canvas-state-
  seeding queued as week-2 follow-up. Operator decision: "Commit —
  V14 covered, V15 partial-accept".
- 2026-05-12 (operator): **SHIPPED.** Operator approval recorded
  in [`presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](presentations/ui-test-harness-bootstrap-2026-05-12.md#approval)
  as `[x] Approved — ship`. Evaluator emitted `VERDICT → PASS`
  (`reports/evaluation-2026-05-12T13-15Z.md`). Anchors PASS 11/11.
  Frontmatter flipped `in-progress → shipped`; `owner: developer →
  shipped`. First run of both the AGENT.md `## Capability
  boundaries` regime and the test-runner / evaluator split —
  empirical proof the new workflow holds.
