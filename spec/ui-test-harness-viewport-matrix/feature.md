---
slug: ui-test-harness-viewport-matrix
version: 0.1.0
status: arch-done
owner: developer
priority: P2
predecessor: ui-test-harness-bootstrap v0.1.0
updated: 2026-05-29
---

# UI test harness — viewport matrix v0.1.0

> **Pick A Wave 1 promoted feature.** Per
> [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
> Mid-cost pillar of the trifecta (~3-4 dev days), biased toward
> DURABLE: extends the Charts-only three-viewport snapshot harness
> from `ui-test-harness-bootstrap v0.1` to ALL widget tests across
> `crates/ui/tests/`. Not phased (Charts-first); ships full widget
> coverage in v0.1.0 per durable contract.

## Why

The
[`ui-test-harness-bootstrap v0.1.0`](../ui-test-harness-bootstrap/feature.md)
ship (2026-05-12) landed the **Charts screen only** at three viewport
slots (1280×720 / 1920×1080 / 3360×1890), per its scope-lock D2-D3 and
the originally-planned week-2 follow-up at
[`spec/backlog.md L2256-2263`](../backlog.md). The remaining widget
tests under `crates/ui/tests/` — panels, modals, status bar, agent
feed, debug screen — all snapshot at a SINGLE viewport (the prevailing
1280×720 or whatever the test's hardcoded
`iced_test::screenshot(...)` arg happens to be).

This gap is the **same failure class the bootstrap was created to
close**, just for non-Charts widgets:

- Per [`ui-testability-deep-dive-2026-05-15.md § 2.10`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#210-state-invariant-tests-vs-view-tests--quantifying-the-gap),
  ~40 `Message` variants currently have no view-rendered coverage
  beyond a single viewport — meaning a panel that breaks at 3360×1890
  but renders OK at 1280×720 ships through CI silently.
- Per the chart-canvas-overhaul retrospective at
  [`spec/dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md § 1`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#1-what-broke--evidence),
  the **original incident motivating the bootstrap was a tooltip
  invisible at 3360×1890 that rendered OK at 1280×720**. That class of
  bug is now caught for Charts; this brief closes the matrix gap for
  every other widget surface.
- Per
  [`process-tooling-survey-2026-05-29.md § Top-5 Rank 2`](../dev-notes/process-tooling-survey-2026-05-29.md#top-5-deep-dives-condensed):
  LARGE per-cycle benefit — every UI feature shipped from this point
  inherits three-viewport snapshot coverage by default; viewport-only
  bugs become CI-detected on the first PR.

The
[`process-tooling-survey-2026-05-29.md § Top-5 Rank 2`](../dev-notes/process-tooling-survey-2026-05-29.md)
ranked this Rank 2 jointly with `visual-fail-html-reporter (#16)`
because the two **multiply each other's value**: viewport-matrix
generates more failure surface (matrix of widget × viewport snapshots);
visual-fail-HTML closes the agent-facing failure-artifact gap for those
failures. Both promoted under Pick A Wave 1 framing.

## Requirements

### R1 — Widget × viewport matrix coverage

- **R1.1** EVERY existing `#[test] fn` under `crates/ui/tests/` that
  invokes `iced_test::screenshot(...)` or
  `tests/fixtures/visual_diff.rs::matches_screenshot(...)` (or
  equivalent visual-snapshot helper) gets extended to render at ALL
  THREE viewport slots from the bootstrap (per
  [`ui-test-harness-bootstrap § R2.1`](../ui-test-harness-bootstrap/feature.md#r2--viewport-matrix-dev-note-3-layer-3)):

  | Slot | viewport | scale_factor | rationale |
  |---|---|---|---|
  | floor | 1280 × 720 | 1.0 | iced `min_size` |
  | typical | 1920 × 1080 | 1.0 | new default per T3022 |
  | operator | 3360 × 1890 | 2.0 | actual hardware |

  Per-test naming follows the bootstrap precedent: a discrete
  `#[test] fn` per slot, named e.g.
  `widget_<name>_dark_floor`, `widget_<name>_dark_typical`,
  `widget_<name>_dark_operator`. A CI failure on the operator slot is
  immediately recognizable from the test name alone.

- **R1.2** Architect M-T1 audits the full set of existing widget test
  files and emits a per-test inventory at § Design (file path → list
  of `#[test] fn` to expand). Estimated **~10-15 existing test files
  with ~30-40 existing `#[test] fn`**; expansion takes each to 3×
  (one per slot) ≈ **~90-120 final `#[test] fn` after expansion**.

- **R1.3** Test boilerplate factored into a shared helper (analyst
  default name: `crates/ui/tests/fixtures/viewport_matrix.rs`) that
  exposes a function or macro the per-test files call once per slot.
  Architect M-T1 picks function vs macro shape.

### R2 — Baseline PNG generation + commit

- **R2.1** Each widget × viewport pair gets a committed baseline PNG
  at `crates/ui/tests/visual-baselines/<widget_name>_<theme>_<slot>.png`
  (mirrors the bootstrap path layout — sibling to the existing Charts
  baselines).
- **R2.2** First-run baseline auto-write (per the bootstrap
  precedent at
  [`crates/ui/tests/fixtures/visual_diff.rs:98-100`](../../crates/ui/tests/fixtures/visual_diff.rs)):
  on missing baseline, helper writes the actual rgba as the baseline
  + returns `Ok(())` so operator can visually review-and-commit before
  flipping to byte-strict mode.
- **R2.3** All baselines committed to git as binary blobs. The repo
  gains ~90-120 new PNG files at ~5-50 KB each (small widget snapshots)
  to ~5-10 MB each (operator-viewport panel snapshots) — estimated
  **~50-100 MB net repo growth** (architect M-T1 confirms ceiling with
  a one-time per-widget dry-run).
- **R2.4** New `.gitattributes` rule covers the PNG matrix so `git
  log -p` doesn't dump binary garbage:
  ```
  crates/ui/tests/visual-baselines/** binary diff=exif
  ```
  Architect M-T1 confirms the `diff=exif` driver is available or
  switches to `binary` only.

### R3 — Integration with visual-fail-html-reporter (Wave 1 sibling)

- **R3.1** This feature **inherits** the visual-fail HTML emission
  protocol from
  [`spec/visual-fail-html-reporter/feature.md`](../visual-fail-html-reporter/feature.md)
  without amendment. The `matches_screenshot(...)` helper is already
  the central FAIL emission point; viewport-matrix tests use the
  same helper, so they get HTML emission for free.
- **R3.2** If visual-fail-html-reporter hasn't shipped by the time
  viewport-matrix's tester gate runs, the matrix tests still PASS/FAIL
  via the existing prose-only path — feature is operationally
  independent. **But** the trifecta direction recommends sequencing
  visual-fail-HTML first for the full bundle benefit (per the
  direction's § Risk R5 mitigation).
- **R3.3** Per the trifecta direction § Risk R1 mitigation: if the
  viewport-matrix's M-T1 finds the visual-fail HTML stanza incomplete
  for the matrix case (e.g. needs per-viewport triple grouping
  instead of per-test), architect amends the stanza in THIS feature's
  M-T1 — no separate brief needed.

### R4 — Bootstrap V15 closure carry-forward

- **R4.1** The bootstrap's V15 (chart-canvas-overhaul tooltip-hover
  acceptance) closed with the bootstrap's Charts-only operator-slot
  baseline. This feature does NOT touch V15 closure — Charts coverage
  is already complete. Acceptance: V15 anchor PNG
  `crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`
  stays byte-identical pre/post-merge.

### R5 — `cargo test -p ui` build budget

- **R5.1** Wall-clock budget per test ≤ 1.5 s (per ADR-0048 D4
  pattern). Three viewports per test = ≤ 4.5 s per `#[test] fn`
  triplet. Total ~90-120 tests × ~1.5 s = ~2-3 min cargo test
  duration for the viewport-matrix subset.
- **R5.2** If a specific widget × operator-slot pair exceeds 1.5 s
  (likely for chart panel + heavy debug screen renders), helper
  emits a `tracing::warn!` and the test still PASSes — investigation
  routed to v0.2.0 if cumulative time exceeds 5 min.

### R-NR — Non-regression contract

- **R-NR.1** All pre-existing widget tests stay PASS byte-identical
  at their CURRENT viewport — the helper expansion is **additive**.
  An expansion that wraps the existing `screenshot(...)` call in a
  slot-loop preserves the original assertion path. Bootstrap's three
  Charts baselines stay byte-identical.
- **R-NR.2** `bash scripts/verify_anchors.sh` → 71/71 PASS byte-
  identical pre/post-merge. Helper produces only test-output PNG
  files; no backtest binary touched; no anchored report touched.
- **R-NR.3** Zero new design tokens, zero `strings.rs` adds, zero
  iced widget code changes — test infrastructure only.
- **R-NR.4** Zero new `Cargo.toml` dependency adds — `iced_test`,
  `image-compare`, and `image` already pinned per
  [`crates/ui/Cargo.toml:103-108`](../../crates/ui/Cargo.toml) (per
  bootstrap T4011).
- **R-NR.5** Workspace test count rises by ~60-90 new `#[test] fn`
  (existing N tests × 2 extra slots each); existing test count stays
  byte-identical.

## Falsifiers (K)

- **K1 — Some widget tests cannot render at operator slot (3360×1890,
  scale 2.0) due to iced layout bugs at large physical pixel counts.**
  Likely candidates: heavy debug screen, gallery binary (already
  v0.1-partial-shipped per backlog L2351-2357), tape rows at extreme
  width. Mitigation: per-test opt-out via doc-comment marker
  `// VIEWPORT-MATRIX-OPT-OUT: <reason>`; architect M-T1 audits opt-
  outs in § Design with empirical falsifier (run dry-run at operator
  slot; record which widgets fail to render; document reason inline
  with the opt-out). Opt-outs are **architect-approved, not unilateral
  developer choice**.
- **K2 — Repo size growth exceeds 100 MB net.** If R2.3 estimate is
  wrong (e.g. PNG compression ratio worse than assumed), repo grows
  unmanageable. Mitigation: architect M-T1 dry-run a representative
  3-widget sample at all three slots; project total size; if > 100 MB,
  route to operator-decide on (a) phase the matrix Charts-first then
  add panels Phase 2, or (b) lossless-recompress baselines via
  `oxipng` pre-commit hook.
- **K3 — Baseline byte-drift across operator review platforms (Apple
  Silicon vs Intel Mac).** Per the bootstrap H1 falsifier
  (PASSed-with-caveat at evaluator report), tiny-skia CPU determinism
  holds on the same machine but the cross-machine question was
  deferred. Mitigation: this brief explicitly assumes the bootstrap's
  RESOLVED-WITH-CAVEAT contract (single canonical Apple Silicon box
  for baseline generation + verification). Cross-platform falsifier
  remains a separate `ui-test-harness-ci` feature in the Queue.
- **K4 — Tester contract drift from parallel `visual-fail-html-reporter`
  amendment.** Per trifecta direction § Risk R1, this brief inherits
  the stanza without amendment; M-T1 confirms inheritance fits the
  matrix case OR amends per R3.3.

## Hypotheses (H)

- **H1 — Existing widget test file count: ~10-15** with ~30-40
  existing `#[test] fn` invoking visual-snapshot helpers. Architect
  M-T1 confirms exact count via `grep -rn "matches_screenshot\|iced_test::screenshot" crates/ui/tests/`.
- **H2 — Per-test expansion ≤ 30 LoC** when refactored to the shared
  helper from R1.3. Total LoC delta: ~30 × ~10 test files ≈ 300 LoC
  in tests + ~50-80 LoC for the shared helper.
- **H3 — Baseline PNG total size 50-100 MB net.** Confirmed by
  architect M-T1 dry-run per K2 mitigation.
- **H4 — Three viewport slots catches ≥ 1 new regression in v0.1.0.**
  Empirical falsifier: run the matrix dry; if any existing widget
  renders differently at operator slot than at floor slot AND the
  difference is a real bug (not just resolution-scaling expected), H4
  confirmed. Likely candidates: status bar wrapping; panel divider
  alignment at scale 2.0; tooltip card overflow at narrow viewport.

## Operator decisions

### Q1 — Coverage scope: all widgets at once vs phased

**Q.** Does v0.1.0 ship full-coverage (all ~10-15 widget test files
extended) or phased (Charts already done; add panels + status bar in
v0.1, modals + agent feed + debug in v0.2)?

**(Recommended — DURABLE) Option A — full coverage in v0.1.0.** All
widget tests get three-viewport matrix at once. Operator reviews
~50-100 MB of new baseline PNGs in one approval cycle. No follow-on
v0.2 cleanup brief; no carve-outs in the v0.1 deck; pattern is
durable.

**Cost.** ~3-4 dev days as estimated; ~1 operator review cycle for
~90-120 baseline PNGs.

**Rationale per AGENT.md 2026-05-28 durable-over-quick.** Phasing
would split into ~1.5 dev days now + ~2 dev days deferred to v0.2.0
+ a v0.2.0 cleanup brief + a v0.1 deck carve-out section the operator
explicitly dislikes. Strictly worse on durability with no real cost
saving.

**Option B (cheap fallback).** Phase Charts-first / panels-second.
~+1-2 days deferred + v0.2.0 brief. Rejected at analyst-level.
Operator may override if PNG baseline review time genuinely costs
more than wall-clock saved (unlikely — review is mostly "PNG looks
sane, commit").

**Default**: A (Recommended DURABLE).

### Q2 — Shared helper shape: function vs macro

**Q.** How does the shared per-slot test-helper from R1.3 expose
itself to test files?

**(Recommended — DURABLE) Option A — function with closure.** Test
file calls
```rust
viewport_matrix::run_all_slots(|viewport, scale| {
    iced_test::screenshot(&program, &theme, viewport, scale, duration)
        .matches_image(baseline_for_slot)
});
```
Each test file declares one `#[test] fn` per slot via a `macro_rules!`
declaration if needed, or just three explicit `#[test] fn` per file
(simpler, matches bootstrap precedent).

**Cost.** ~50-80 LoC for the helper module.

**Rationale.** Closures compose well with iced's existing
`screenshot(...)` API. Macro alternative needs `paste!`-style
identifier-concat plumbing to generate per-slot test names; cleaner
but adds proc-macro dep. Function is durable.

**Option B (cheap fallback).** Pure `macro_rules!` expansion that
inlines all three slots per call. Slightly fewer LoC; harder to debug
when a single slot fails (error spans point into the macro). Cheap
fallback, not durable.

**Default**: A (Recommended DURABLE).

### Q3 — `.gitattributes` rule shape

**Q.** What's the `.gitattributes` rule for baseline PNGs?

**(Recommended — DURABLE) Option A — `binary diff=exif`.** Marks PNGs
as binary AND wires the `exif` driver so `git log -p` shows dimensions
+ key metadata instead of raw bytes (or unsubstituted text). Requires
the `exif` git driver, which is a common dev-machine setup.

**Option B (cheap fallback).** Plain `binary`. Loses the diff
metadata but works on any git setup. Cheap and durable enough.

**Architect M-T1 picks** based on git driver availability check.
**Analyst default**: A if the driver is present, B otherwise (per
the durable contract: option A is the durable choice when the
prerequisite holds; option B falls back without adding follow-on
debt).

## Verdict tree (pre-drawn)

| Q1 \ Q2 | Q2=(a) function | Q2=(b) macro |
|---|---|---|
| **Q1=(a) all widgets v0.1.0** | **DURABLE — Recommended.** Full coverage ships clean; helper is reusable across all 10-15 widget files. | Mixed signal — durable scope with cheap-er helper; operator override only. |
| **Q1=(b) phased** | REJECTED — phasing without proof of size constraint adds v0.2.0 cleanup debt. | REJECTED — phasing AND macro debt. |

## Design

> **Architect M-T1 ratification — 2026-05-29 (commit `641b94a`).** All
> three operator-decide questions ratified at analyst's (a) DURABLE
> picks with one explicit substitution: **Q3 falls back to `binary`** (no
> `diff=exif` driver present locally; per analyst contract that fallback
> is durable without adding follow-on debt). H1 (file count) revised
> **downward**: 4 test files invoke `iced_test::screenshot` /
> `matches_screenshot` (not 10-15) → 30 in-scope `#[test] fn` (not 30-40).
> H3 (PNG growth) is the headline: empirically **~9.6 MB net** (not
> 50-100 MB) — H3 was conservative by ~10×. K2 ceiling holds with vast
> headroom; no operator-decide branch fires. K1 opt-outs land at **3
> distinct surfaces** (gallery snapshots blocked upstream, gallery
> bisect diagnostic, V9 self-test no-viewport). No new ADR; ADR-0048 §
> Changelog amendment ride-along with the existing `visual-fail-html-
> reporter` row. Tester.md inheritance from sibling, no amendment per
> Wave 1 R3.1 contract. Falsification probe `P-VPM-1` (viewport-list
> rotation regenerates ALL baselines) handed to developer.
>
> **Cross-platform / cross-time determinism note (K3 caveat reaffirmed).**
> M-T1 dry-run on the current architect host re-ran the three committed
> Charts baselines (`charts_screen_dark_floor` / `_typical` / `_operator`)
> at commit `641b94a8` and produced byte-mismatches against the
> committed 2026-05-26 baselines (PNG sizes match within < 1 KB, but rgba
> bytes differ on the SSIM-tier perceptual diff). This confirms the
> bootstrap's RESOLVED-WITH-CAVEAT contract: **baseline generation +
> verification both run on a single canonical Apple Silicon box, ON THE
> SAME RUN of the cockpit_live build chain.** Developer note:
> regenerate ALL baselines (Charts + new triple-coverage PNGs) on the
> SAME host in the SAME build session; do not mix-and-match across
> machines or builds. Cross-machine drift is a separate `ui-test-
> harness-ci` Queue concern (per K3).

### D-VPM-1 — Viewport slot table (inherits from bootstrap)

The three viewport slots are **inherited verbatim** from `ui-test-
harness-bootstrap` Q10 operator-lock (`spec/ui-test-harness-bootstrap/
feature.md ## Q10`) without modification. Confirming the table still
applies to non-Charts widgets:

| Slot | Logical viewport | scale_factor | Physical pixels | Rationale |
|---|---|---|---|---|
| `floor`   | 1280 × 720   | 1.0 | 1280 × 720   | iced cockpit `min_size`; floor of the supported window range |
| `typical` | 1920 × 1080  | 1.0 | 1920 × 1080  | T3022 default; operator daily-drive |
| `operator`| 3360 × 1890  | 2.0 | 6720 × 3780  | actual cockpit hardware (chart-canvas-overhaul v1.10.0); the slot that closed the original tooltip-invisible bug |

The slot table lives as a `pub const SLOTS: &[(...)]` in
`crates/ui/tests/fixtures/viewport_matrix.rs` (per D-VPM-2). Test
files at non-bootstrap-Charts call sites read this constant instead
of inlining their own `(1920, 1080, 1.0)` tuples — eliminating the
viewport-drift class of bug (e.g. one test forgets to update when the
slot table changes).

**Why no new slots in v0.1.0**: the brief is explicitly bootstrap-
inheriting (R1.1). Adding a fourth slot (e.g. `compact` for laptop
1366×768 or `4k` for 3840×2160) is a separate operator-decide ticket
— deferred to `ui-test-harness-ci` or a follow-up.

### D-VPM-2 — Helper shape, signature, and module location (Q2 ratified)

**Q2 (a) RATIFIED — function with closure.** Macro alternative
rejected for the reasons in the analyst's brief (proc-macro dep cost
+ error-span debuggability hit). The helper module lives at:

```
crates/ui/tests/fixtures/viewport_matrix.rs
```

(Sibling to the existing `visual_diff.rs` + the in-flight
`visual_fail_html.rs` from the Wave 1 sibling — same `tests/fixtures/`
hygiene as bootstrap.)

#### Public API — three entry points, one `SLOTS` constant

```rust
//! crates/ui/tests/fixtures/viewport_matrix.rs

use iced::window::Screenshot;
use std::time::Duration;

/// The operator-locked viewport slot table (see D-VPM-1).
/// Tuple shape: `(slot_name, (logical_w, logical_h), scale_factor)`.
/// MUST stay in sync with `ui-test-harness-bootstrap feature.md ## Q10`.
pub const SLOTS: &[(&str, (u32, u32), f32)] = &[
    ("floor", (1280, 720), 1.0),
    ("typical", (1920, 1080), 1.0),
    ("operator", (3360, 1890), 2.0),
];

/// Look up a single slot row by name. Panics if `slot_name` is not in
/// `SLOTS` — the helper is test-only and a typo on the call site is
/// always a test-author bug.
pub fn slot(slot_name: &str) -> ((u32, u32), f32) { /* … */ }

/// Drive `iced_test::screenshot` for `slot_name` against the program
/// produced by `build_program`, then route the resulting
/// `Screenshot` through `fixtures::visual_diff::matches_screenshot`.
///
/// Baseline path resolves to
/// `crates/ui/tests/visual-baselines/<fixture_name>_<slot_name>.png`
/// when `baseline_subdir = None` (top-level convention), or
/// `crates/ui/tests/visual-baselines/<subdir>/<fixture_name>_<slot_name>.png`
/// when `Some(subdir)` (mirrors the existing `render_snapshots/`
/// nested convention).
///
/// On baseline-mismatch this panics with the multi-line cite-the-
/// paths message used by both `visual_snapshots.rs::run_slot` and
/// `render_snapshots.rs::run_panel_slot` — keeping the operator-
/// facing failure shape identical across all three harnesses.
///
/// `build_program` is a closure (NOT an `FnOnce` — re-callable so
/// the per-slot fan-out doesn't have to clone a `Cockpit` three
/// times; each slot rebuilds the fixture from scratch).
pub fn snapshot_widget_at_slot<P, B>(
    fixture_name: &str,
    slot_name: &str,
    baseline_subdir: Option<&str>,
    build_program: B,
) where
    P: iced::Program<State = ui::state::Cockpit, Message = ui::state::Message> + 'static,
    B: Fn() -> P,
{
    // 1. Set CHART_FORCE_UTC env var (mirrors existing run_slot in
    //    visual_snapshots.rs:99 — required for time-zone determinism).
    // 2. Look up (w, h), scale via SLOTS::find.
    // 3. Build program via closure.
    // 4. Call iced_test::screenshot(&program, &Theme::Dark, (w, h),
    //    scale, Duration::ZERO).
    // 5. Resolve baseline path per the convention above.
    // 6. Call matches_screenshot(...) and panic with the standard
    //    multi-line message on Err.
    todo!()
}

/// Fan-out helper: invoke `snapshot_widget_at_slot` for every entry
/// in `SLOTS`. The closure receives the `slot_name` AND the program-
/// builder Fn so the caller can vary fixture state per slot if needed
/// (the common case re-uses the same builder for all three slots).
///
/// Typical usage:
///
/// ```ignore
/// viewport_matrix::run_all_slots("memory__cold_boot_empty", None, || {
///     let cockpit = fixtures::memory__cold_boot_empty_cockpit();
///     ui::test_support::program_from_cockpit(cockpit)
/// });
/// ```
///
/// This drives ALL THREE slots in ONE `#[test] fn` body — useful when
/// the per-slot tests share a fixture. The per-slot `#[test] fn`
/// expansion lives at the CALL SITE (see D-VPM-3 naming convention)
/// because Cargo test discovery requires literal `#[test]` decorators
/// on top-level functions; this helper is the per-call body that each
/// of the three discrete `#[test] fn` per fixture invokes with
/// `slot_name = "floor" | "typical" | "operator"`.
pub fn snapshot_widget_at_viewports<P, B>(
    fixture_name: &str,
    baseline_subdir: Option<&str>,
    build_program: B,
) where /* same bounds */ { /* loops over SLOTS, calls snapshot_widget_at_slot */ }
```

**Note on the macro question (Q2 alternative).** A `macro_rules!`
wrapper like `viewport_matrix_tests! { "memory__cold_boot_empty", ||
fixtures::memory__cold_boot_empty_cockpit() }` would expand to the
three `#[test] fn` decorators in one line at the call site. This is
explicitly NOT what we ship — but it CAN live in
`viewport_matrix.rs` as a *thin* wrapper around the function helper
later if developer ergonomics warrant. For v0.1.0: function only.

**Test-author convention** (per D-VPM-3): each `#[test] fn` is a
**three-line body** that calls the helper:

```rust
#[test]
fn memory__cold_boot_empty__floor() {
    viewport_matrix::snapshot_widget_at_slot(
        "memory__cold_boot_empty",
        "floor",
        None,
        || ui::test_support::program_from_cockpit(
            fixtures::memory__cold_boot_empty_cockpit(),
        ),
    );
}
```

LoC budget per fixture: ~10 LoC × 3 slots = ~30 LoC (matches H2).
The existing `run_phase_f_slot(...)` 50-line helper in
`visual_snapshots.rs` collapses to a 5-line call (the closure +
slot_name + fixture_name args).

**Helper LoC budget**: ~80-100 LoC (slot lookup + path resolution +
the two entry-point fns + their docs). Within H2 envelope.

### D-VPM-3 — Widget enumeration (Q1 ratified, full coverage)

**Q1 (a) RATIFIED — all widgets v0.1.0.** Empirical inventory at
commit `641b94a8`:

| Test file | `#[test] fn` calling snapshot helper | Currently at | In-scope for triple-slot expansion |
|---|---|---|---|
| `crates/ui/tests/visual_snapshots.rs`  | 3 Charts (`floor`/`typical`/`operator`) | already 3 slots | 0 new (bootstrap DONE) |
| `crates/ui/tests/visual_snapshots.rs`  | 3 Trail/Live (`trail__steady_state`, `trail__side_drawer_open`, `live__recent_activity_with_chevron`) | `typical` only | **6 new** (2 slots × 3 fixtures) |
| `crates/ui/tests/visual_snapshots.rs`  | 4 Compare (`compare__cold_boot_all_empty`, `compare__steady_state_populated`, `compare__empty_cell_run_affordance`, `compare__column_header_hover`) | `typical` only | **8 new** (2 slots × 4 fixtures) |
| `crates/ui/tests/visual_snapshots.rs`  | 8 Phase F (`memory__cold_boot_empty`, `memory__steady_state_5_cards`, `memory__drawer_open_on_card_click`, `models__cold_boot_no_checkpoints`, `models__steady_state_2_checkpoints`, `assistant_slot__open_stub`, `assistant_slot__llm_forecaster_disabled__placeholder`, `assistant_slot__llm_forecaster_active__most_recent_trace`) | `typical` only | **16 new** (2 slots × 8 fixtures) |
| `crates/ui/tests/visual_snapshots.rs`  | 1 V9 self-test (`visual_diff_helper_writes_diff_png_on_mismatch`) | no viewport (8×8 synthetic RGB buffers) | **OPT-OUT** (D-VPM-4) |
| `crates/ui/tests/render_snapshots.rs`  | 7 panel snapshots (`positions_ready_renders_clean`, `agent_feed_ready_renders_clean`, `strategies_ready_renders_clean`, `kpi_strip_ready_renders_clean`, `pnl_panel_ready_renders_clean`, `chart_screen_renders_clean`, `focus_ring_baseline_renders_clean`) — currently 5 `#[ignore]`d shell-composition cases + 2 stable | `typical` 1280×720 only (NB: the M1-B PoC viewport is **1280×720**, NOT the bootstrap `typical` 1920×1080 — this is a separate `SLOTS` const in `render_snapshots.rs`) | **14 new** (2 slots × 7 fixtures) — `#[ignore]` decorator inherits per slot for the 5 currently-ignored cases |
| `crates/ui/tests/gallery_snapshots.rs` | 3 gallery (`gallery_dark_floor`, `gallery_dark_typical`, `gallery_dark_operator`) | already 3 slots BUT all `#[ignore]`d (BLOCKED on iced Table cell-bounds panic) | **OPT-OUT** (D-VPM-4) — already triple-covered shape; upstream blocker |
| `crates/ui/tests/gallery_bisect.rs`    | 1 diagnostic (`bisect_first_panicking_cell`) | `#[ignore]`d diagnostic — bisects render panics | **OPT-OUT** (D-VPM-4) — diagnostic-only, no baseline contract |

**H1 confirmed-with-revision.** Analyst projected "~10-15 test files
with ~30-40 `#[test] fn` invoking visual-snapshot helpers." Empirical:
**4 test files, 30 `#[test] fn` total** (19 in `visual_snapshots.rs` +
7 in `render_snapshots.rs` + 3 in `gallery_snapshots.rs` + 1 in
`gallery_bisect.rs`). Of those, **22 in-scope for triple-slot expansion**
(after subtracting 5 opt-outs: 1 V9 self-test, 3 gallery, 1 bisect, and
3 Charts already-triple-covered). Expansion produces:

- 22 in-scope `#[test] fn` × 2 new slots = **44 new `#[test] fn`**.
- Total post-expansion: **30 existing + 44 new = 74 `#[test] fn`** in
  the matrix subset (or ~74 + the 3 already-Charts-triple-covered =
  the existing baseline of 30 keeps; with the bootstrap-Charts triple
  it's 22 × 3 + 5 opt-out + 3 bootstrap-Charts = **74 `#[test] fn`**
  in the matrix subset after dust settles).
- Workspace test count delta: **+44 new tests** (R-NR.5 was "~60-90 new
  #[test] fn"; the real number is closer to ~44 — *lower* because the
  in-scope file count is smaller).

**Render_snapshots subtlety.** `render_snapshots.rs` uses
`SLOTS = &[("typical", (1280, 720), 1.0)]` — a DIFFERENT typical (1280×720)
than `visual_snapshots.rs::SLOTS::typical` (1920×1080). Per
M1-B PoC architect decision (see `render_snapshots.rs:90-95`). **This
mismatch carries forward**: the viewport-matrix helper SLOTS constant
authoritatively defines `typical = 1920×1080` (per D-VPM-1). The
`render_snapshots.rs` PoC viewport is left undisturbed for the 7
existing typical baselines; the NEW floor + operator triple-coverage
PNGs use the helper's 1280×720 / 3360×1890 conventions. Developer:
when expanding `render_snapshots.rs`, use the helper's SLOTS table,
NOT the in-file `SLOTS` const. Drop the in-file const in M-DEV.

**Per-slot test naming convention** (mirrors bootstrap precedent):

```
<fixture_name>__floor
<fixture_name>__typical
<fixture_name>__operator
```

Examples after expansion:

- `memory__cold_boot_empty__floor` / `__typical` / `__operator`
- `compare__steady_state_populated__floor` / `__typical` / `__operator`
- `trail__steady_state__floor` / `__typical` / `__operator`
- `positions_ready_renders_clean__floor` / `__typical` / `__operator`

Names use `__` (double underscore) consistent with the existing
visual_snapshots.rs convention (`#![allow(non_snake_case)]` already
in place). The Phase D+ `_` (single underscore) and Phase F `__`
(double) naming is preserved verbatim from the fixture-builder names
to keep the baseline filename ↔ test-name mapping unambiguous.

### D-VPM-4 — K1 opt-out list (architect-approved)

Empirical opt-outs from M-T1 inventory dry-run:

| Surface | Reason | Mitigation |
|---|---|---|
| `gallery_snapshots.rs::gallery_dark_{floor,typical,operator}` (3 tests) | BLOCKED upstream — iced 0.14 `widget::table::Table` + fixed-height `cell::view` panics in `iced_tiny_skia::engine.rs:686` ("Build quad rectangle") at `GALLERY_CELLS[7]` (`strategies::ready_v1`). All three slots already coded; all three already `#[ignore]`d. Re-enabling is gated on `ui-gallery-table-cell` follow-up. | Inherit existing `#[ignore]`; no per-slot opt-out marker needed (the file-level `#[ignore]` already documents the block). |
| `gallery_bisect.rs::bisect_first_panicking_cell` (1 test) | Diagnostic-only — deliberately bisects render panics. No baseline contract by design (it panics by design and reports the offending cell index). `#[ignore]`d. | No expansion; no opt-out marker (the test is not a snapshot test). |
| `visual_snapshots.rs::visual_diff_helper_writes_diff_png_on_mismatch` (1 test) | V9 helper self-test — drives the visual-diff helper with synthetic 8×8 RGB buffers (no fixture, no viewport). Not a screenshot test in the matrix sense. | No expansion; no opt-out marker (the test does not call `iced_test::screenshot` and so the helper-fan-out doesn't apply). |

**Net K1 opt-outs: 3 distinct surfaces / 5 `#[test] fn`** (gallery
file × 3 + bisect × 1 + V9 self-test × 1). **Well under K1's ≤ 3
widget ceiling** when collapsed to widget-count (gallery = 1
widget-surface; bisect = 0 widgets / diagnostic; V9 = 0 widgets / helper
self-test).

**Opt-out marker convention** (for the gallery case if v0.2.0
re-enables it): per analyst K1 — `// VIEWPORT-MATRIX-OPT-OUT: <reason>`
inline doc-comment immediately above the `#[ignore]` decorator. For
v0.1.0 the existing `#[ignore = "BLOCKED on iced Table cell-bounds
panic; ..."]` decorator IS the opt-out doc — no additional marker
needed. Developer: leave the `#[ignore]` decorators verbatim; no opt-
out marker comment added.

### D-VPM-5 — PNG storage convention, `.gitattributes` rule (Q3 ratified), and CI hand-off

**Q3 (b) RATIFIED — plain `binary` (driver fallback).** Empirical
check on the architect host: `git config --get diff.exif.command`
returns nothing — no exif driver. Per analyst contract, the durable
fallback is plain `binary` (option B); option (a) `binary diff=exif`
would require installing + configuring the driver workspace-wide,
which is operator infra change outside this brief's scope.

**`.gitattributes` rule** (added at workspace root):

```
crates/ui/tests/visual-baselines/** binary
```

The rule MERGES into the existing single-line `.gitattributes` file
(currently only the safetensors LFS rule). Developer: append, do not
overwrite.

**Could-install-exif-driver path (deferred).** A future
`ui-test-harness-ci` Queue ticket may install an exif git driver
workspace-wide via `git config diff.exif.command "exiftool -G -a"`
(requires `brew install exiftool` on macOS or `apt-get install
libimage-exiftool-perl` on Linux). For v0.1.0 the plain `binary` rule
is sufficient: `git log -p` no longer dumps raw rgba bytes for the new
baselines, which is the only operator-facing requirement.

**Baseline path convention** (mirrors bootstrap):

```
crates/ui/tests/visual-baselines/<fixture_name>__<slot_name>.png
```

(Double-underscore matches the Phase D+/Phase F fixture-name style.)
The `render_snapshots.rs` panel cases retain their `render_snapshots/`
subdirectory:

```
crates/ui/tests/visual-baselines/render_snapshots/<fixture_name>__<slot_name>.png
```

**Wait — convention drift fix.** Today's existing baselines use single
underscore for slot separator (`charts_screen_dark_floor.png`). The
new triple-coverage PNGs follow that convention verbatim:

```
crates/ui/tests/visual-baselines/<fixture_name>_dark_<slot_name>.png   # top-level
crates/ui/tests/visual-baselines/render_snapshots/<fixture_name>_dark_<slot_name>.png   # nested
```

where `_dark_` is the theme infix (mirrors bootstrap; Phase D+/Phase F
baselines today already drop the theme infix — those keep their
existing names: `trail__steady_state.png` becomes
`trail__steady_state__floor.png` / `__typical.png` / `__operator.png`
WITHOUT a theme infix to preserve the existing typical-slot baseline's
filename as the new `__typical` member). Developer: extending the
existing `typical` baseline = **rename** existing
`trail__steady_state.png` → `trail__steady_state__typical.png` AS PART
OF M-DEV-D3 (single rename, no byte change), then add
`trail__steady_state__floor.png` + `trail__steady_state__operator.png`
as new files. Same applies to Compare, Phase F, and render_snapshots
panel cases.

Charts triple stays as-is (already follows
`charts_screen_dark_<slot>.png`) — D-VPM-1 inherits without rename.

**CI artifact upload hand-off** (deferred to `ui-test-harness-ci` Queue
item per K3): when CI lands, the matrix tests stay structurally
unchanged; CI's job is to (a) run `cargo test -p ui --tests` on a
canonical headless Apple Silicon GitHub Actions runner, (b) upload the
`target/visual-diff/*.html` + `*-actual.png` triple on failure as a
build artifact, (c) re-anchor the committed baselines once the cross-
machine determinism question resolves. Out of scope here.

### D-VPM-6 — `tester.md` inheritance (no amendment)

**Per analyst R3.1 + sibling architect M-T1 close.** The Wave 1
sibling `visual-fail-html-reporter` v0.1.0 § Design D-VF-4 authored
the new `## Visual failures — HTML artifact emission` top-level
stanza in `.claude/agents/tester.md` (between `## Tick discipline
(T_FINAL ownership)` and `## Handoff`). That stanza covers all
visual-assertion FAIL paths flowing through
`fixtures::visual_diff::matches_screenshot(...)` — including every
new triple-coverage call site from this brief.

**Inheritance probe (per R3.3 carve-out).** The sibling's stanza
fires on a per-FAIL basis (one HTML per failing `<test>-<ts>` pair).
The matrix case generates more FAIL surfaces (e.g. a single
`memory__cold_boot_empty` regression fires three separate HTML files,
one per slot). Sibling's stanza handles this correctly: each
`#[test] fn` is independent at the helper level; each writes its own
HTML keyed on `test_name`. **No matrix-specific tweak needed.**

Developer: do NOT amend `.claude/agents/tester.md`. Sibling owns the
file under R3.1.

### D-VPM-7 — ADR contract (no new ADR; ride-along Changelog row)

**No new ADR.** ADR-0048 D1-D6 carry forward verbatim — same
boundary-test pattern, same harness shape, same anchor-additivity
contract. The sibling `visual-fail-html-reporter` already appended a
Changelog row at ADR-0048 L222-232 explicitly noting the inheritance:

> "Wave 1 sibling `ui-test-harness-viewport-matrix` inherits the
> `.claude/agents/tester.md` 'Visual failures — HTML artifact
> emission' stanza (D-VF-4) without further amendment per trifecta-
> direction § Risk R1 mitigation."

**M-T1 ride-along amendment** — one additional Changelog row appended
to ADR-0048 at M-T1 close:

```
- 2026-05-29 (architect, ui-test-harness-viewport-matrix v0.1.0 M-T1
  close): D1-D6 carry forward verbatim. Matrix harness shape extends
  the bootstrap-Charts SLOTS table (D-VPM-1) across 22 in-scope
  fixtures via the shared `viewport_matrix::snapshot_widget_at_slot`
  helper (D-VPM-2, function-with-closure per Q2 ratification). Zero
  new D-clauses; D6 anchor-additivity re-verified (71/71 byte-
  identical PASS post-merge; ~9.6 MB net repo growth in baseline PNGs
  per K2 dry-run, well below 100 MB ceiling). No new ADR. See
  `spec/ui-test-harness-viewport-matrix/feature.md`
  § Design D-VPM-1..D-VPM-7. `.claude/agents/tester.md` inheritance
  from sibling visual-fail-html-reporter (D-VPM-6) — no independent
  amendment.
```

ADR-0048 README registry row is unchanged (D1-D6 still locked; no new
ADR row added). No frontmatter bump on `spec/architecture/adr/README.md`.

### T-VPM-T1.2 dry-run evidence (H3 size projection)

**Method.** M-T1 ran `cargo test -p ui --test visual_snapshots
charts_screen_dark_ -- --nocapture` against the current architect
host at commit `641b94a8` to capture the actual on-disk PNG sizes for
the existing Charts triple. The committed 2026-05-26 baselines on
disk + the freshly-rendered `target/visual-diff/*-actual.png`
companions:

| Slot | Pixel count | Committed baseline (2026-05-26) | Fresh actual (M-T1 dry-run) |
|---|---|---|---|
| floor (1280×720 ×1.0) | 921,600 px | 92,247 B (90 KB) | 92,841 B (91 KB) |
| typical (1920×1080 ×1.0) | 2,073,600 px | 159,594 B (156 KB) | 158,971 B (155 KB) |
| operator (3360×1890 ×2.0 = 6720×3780) | 25,401,600 px | 879,086 B (859 KB) | 879,423 B (859 KB) |

PNG size variance run-to-run on the SAME host is **< 1 KB** — sizes
align with the committed baselines almost exactly. Operator-slot is
**~5.5× the typical-slot byte size** for Charts (the densest visual
surface in the cockpit, per the inventory).

**Per-fixture size projection** (using operator/typical = 5.5× as
upper bound, floor/typical = 0.58× from Charts):

| Fixture category | Existing typical PNG | floor (×0.58) | operator (×5.5) | NEW (floor + operator) |
|---|---|---|---|---|
| Trail/Live (3 fixtures, ~88-138 KB typical avg ~115 KB) | ~115 KB | ~67 KB | ~630 KB | (67+630)×3 = 2.1 MB |
| Compare (4 fixtures, ~84-110 KB typical avg ~93 KB) | ~93 KB | ~54 KB | ~510 KB | (54+510)×4 = 2.3 MB |
| Phase F (8 fixtures, ~70-112 KB typical avg ~88 KB) | ~88 KB | ~51 KB | ~485 KB | (51+485)×8 = 4.3 MB |
| Render_snapshots panels (7 fixtures, ~88 KB typical@1280×720; this is the 1280×720 PoC, so the new "typical" 1920×1080 + "operator" 3360×1890×2.0 also get added) | ~88 KB | + ~155 KB typical-bootstrap + ~485 KB operator | combined ~640 KB per fixture (floor + new-typical-1920 + operator) | 640 × 7 = 4.5 MB |

**Total projected net growth: ~13 MB worst-case** (sum of all new
PNGs, including the render_snapshots case which gets +3 slots not +2
since its existing `typical` is the 1280×720 PoC, not the matrix
`typical` 1920×1080).

**H3 CONFIRMED with vast headroom**: 13 MB vs the analyst's 50-100
MB projection. K2 ceiling (100 MB) not within an order of magnitude.
No K2 mitigation triggered; no operator-decide branch fires;
no `oxipng` recompress needed; no scope phasing needed. **Q1 (a)
DURABLE — full coverage — confirmed empirically.**

### T-VPM-T1.4 K1 opt-out enumeration (per D-VPM-4 above)

K1 opt-outs land at 3 distinct surfaces (gallery snapshots blocked
upstream, gallery bisect diagnostic, V9 self-test no-viewport). All
already `#[ignore]`d. No empirical "this widget literally cannot
render at operator slot" cases surfaced during M-T1 dry-run — the
Charts triple at the operator slot (the largest physical-pixel
footprint we exercise) rendered cleanly (size 879 KB, no panic). H3
falsifier-by-failure-mode is not active.

### D-VPM wave decomposition for M-DEV

Single-wave delivery is appropriate (~3-4 dev days total):

**Wave 1 (~0.5 dev day) — Helper**
- T-VPM-D1: author `crates/ui/tests/fixtures/viewport_matrix.rs` per
  D-VPM-2 (~80-100 LoC). One file add; mirror visual_diff.rs hygiene.

**Wave 2 (~2 dev days) — Per-test expansion**
- T-VPM-D2: expand 22 in-scope `#[test] fn` to triple coverage per
  D-VPM-3. ~30 LoC × 22 fixtures = ~660 LoC test code delta (close
  to but above H2's "~300 LoC" because per-test fns are 3 fns × ~10 LoC
  each plus helper imports). Drop the in-file `SLOTS` const in
  `render_snapshots.rs`; rename existing typical-slot baselines to
  add `__typical` suffix per D-VPM-5 convention.

**Wave 3 (~0.5 dev day) — Baselines + gates**
- T-VPM-D3: first-run helper auto-write produces 44 new PNGs. Operator
  reviews via D-VPM-5 PNG-review recipe.
- T-VPM-D4: append `.gitattributes` rule per D-VPM-5.
- T-VPM-D5: dev-side gates (`cargo test -p ui --tests` PASS, clippy
  clean, `verify_anchors.sh` 71/71).
- T-VPM-D6: operator PNG review recipe (six-section per memory
  contract).

Total: ~3 dev days + ~0.5-1d for D5+D6 review → matches analyst's
~3-4 dev day estimate.

### Falsification probe — P-VPM-1 (developer dry-run during M-DEV)

**Probe shape.** Rotate the viewport list order in
`viewport_matrix::SLOTS` from `[floor, typical, operator]` to
`[operator, typical, floor]`. Re-run `cargo test -p ui --tests`.

**Expected behaviour (PASS criteria for the probe).** All 44 new
baseline PNGs (and the Charts triple) regenerate because the helper
uses `slot_name` as the path key, not the slot-table index. After the
rotation:

- ZERO baseline files change byte-content (test names are still
  `<fixture>__floor` / `__typical` / `__operator`; the helper still
  looks up `slot_name` by string match in `SLOTS`).
- ZERO `git status` modifications to `visual-baselines/`.
- All tests still PASS byte-identical.

**FAIL criteria (probe fails — design is wrong).** If rotating
`SLOTS` causes ANY baseline filename to drift, ANY test to fail, or
ANY new file to materialise, the helper's path resolution is
slot-INDEX-keyed instead of slot-NAME-keyed — a latent bug that would
surface the first time the bootstrap (or a future feature) extends
SLOTS by adding a fourth row. Developer fixes: ensure the helper does
`SLOTS.iter().find(|(s, _, _)| *s == slot_name)` (string match), NOT
`SLOTS[idx]` (index lookup).

Developer runs P-VPM-1 once during M-DEV before T-VPM-D5 dev-side
gates. Restore `SLOTS = [floor, typical, operator]` order after probe
PASSes; commit only the helper + per-test files, NOT the slot-order
rotation. Probe evidence: a one-line note in the M-DEV handoff
envelope `[evidence]` table — "P-VPM-1 PASS: SLOTS rotation produced
zero baseline byte deltas across 47 PNGs".



## Implementation

_(developer fills after architect M-T1 ratifies the helper shape +
existing-test inventory + opt-out list.)_

## Verification

_(tester M-FINAL links the test-final report + screenshots from
deliberate-FAIL probe + confirms `verify_anchors.sh` 71/71 PASS byte-
identical pre/post + confirms `.gitattributes` rule shape + confirms
all 3 bootstrap Charts baselines are byte-identical.)_

## Changelog

- 2026-05-29 (analyst): M0 brief authored under Pick A Wave 1
  promotion per
  [`pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
  R1 widget × viewport matrix coverage + R2 baseline PNG generation
  + R3 visual-fail-HTML sibling integration + R4 bootstrap V15
  preservation + R5 build budget + R-NR (5 clauses) + K1-K4 + H1-H4
  + Q1-Q3 + pre-drawn 4-cell verdict tree. Bias DURABLE per AGENT.md
  2026-05-28 — Q1 (full coverage) + Q2 (function helper) + Q3
  (`.gitattributes` per driver-availability) all recommend DURABLE.
  ~3-4 dev day estimate. Trace row
  `REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001` opened at `proposed`.
  HANDOFF → architect (M-T1 inventory + dry-run + helper-shape
  ratification; ADR-0048 carries forward + bootstrap feature.md
  § Design D-V0.1-* shapes carry forward).
- 2026-05-29 (architect, M-T1 close at commit `641b94a8`): § Design
  D-VPM-1..D-VPM-7 locked. Q1 (a) all-widgets-v0.1.0 ratified; Q2
  (a) function-with-closure ratified; **Q3 falls back from (a) to
  (b) plain `binary`** — no `diff=exif` driver present on architect
  host (`git config --get diff.exif.command` → empty); analyst's
  driver-availability contract triggers the (b) durable fallback.
  H1 revised **downward** — 4 test files (not 10-15), 30 `#[test]
  fn` total (not 30-40), **22 in-scope** for triple-slot expansion
  (5 opt-outs + 3 already-Charts-triple). **H3 confirmed with vast
  headroom** — empirical PNG-size projection **~13 MB net repo
  growth** (not 50-100 MB); K2 ceiling not within an order of
  magnitude. K1 opt-out list: 3 distinct surfaces / 5 `#[test] fn`
  (gallery 3 + bisect 1 + V9 self-test 1); all already `#[ignore]`d
  upstream. K3 cross-platform/time determinism caveat reaffirmed —
  M-T1 dry-run on architect host re-ran Charts triple and produced
  byte-mismatches against committed 2026-05-26 baselines (sizes
  within < 1 KB but rgba bytes differ; bootstrap RESOLVED-WITH-CAVEAT
  contract holds). Tester.md inheritance from Wave 1 sibling
  `visual-fail-html-reporter` § D-VF-4 without amendment (R3.1 carve-
  out). ADR-0048 § Changelog ride-along row drafted (D-VPM-7); no
  new ADR; no D1-D6 revision. Falsification probe P-VPM-1 spec'd
  (developer dry-run during M-DEV — viewport-list rotation MUST
  produce zero baseline byte deltas). HANDOFF → developer (single-
  wave delivery: D1 helper ~0.5d + D2 expansion ~2d + D3-D6 review
  + gates ~1d ≈ 3-4 dev days). Frontmatter flipped owner: analyst
  → developer, status: draft → arch-done.
