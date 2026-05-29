---
slug: ui-test-harness-viewport-matrix
version: 0.1.0
status: draft
owner: analyst
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

_(architect M-T1 fills D-VPM-1 through D-VPM-N here. Expected
moderate-skip — no new ADR needed; ADR-0048 carries forward for
visual-snapshot helper shape; bootstrap feature.md § Design carries
forward for slot table + viewport-matrix concept. Architect M-T1
audits existing widget test files per R1.2 + dry-runs baseline PNG
count per K2.)_

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
