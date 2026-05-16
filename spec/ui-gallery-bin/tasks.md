---
slug: ui-gallery-bin
status: shipped
owner: orchestrator
updated: 2026-05-16
---

> **All 39 open task boxes below are `[deferred to
> ui-gallery-table-cell]` as of 2026-05-16** per
> [`spec/dev-notes/feature-triage-2026-05-16.md`](../dev-notes/feature-triage-2026-05-16.md)
> row A4 (Wave 2a spec-hygiene). v0.1-partial is operator-accepted
> as terminal for this feature; V5+ work continues in
> [`spec/ui-gallery-table-cell/tasks.md`](../ui-gallery-table-cell/tasks.md).
> The boxes below are preserved verbatim for traceability — do
> **not** tick them in this folder.

# Tasks — widget gallery binary (`ui-gallery-bin`) v0.1

> ## Status as of 2026-05-15 (orchestrator-led recovery)
>
> Developer agent halted on missing Bash permission after writing
> ~885 LOC unverified. Orchestrator took over the verification loop
> (commits `6fbeaff`, this follow-up).
>
> **Done (V-items green):**
> - V1 — build (`cargo build -p ui --bin ui-gallery --features fixtures`)
> - V2 — `--smoke` exits cleanly (`ui-gallery --smoke OK`)
> - V3 — every widget in `EXPECTED_WIDGETS` has at least one gallery cell
> - V4 — every `pub mod` in `widgets/mod.rs` is listed in `EXPECTED_WIDGETS`
> - All 271 ui tests pass, 0 regressions
>
> **Blocked (V5+):**
> - V5/V6/V7/V10 — snapshot tests at three viewports. Test file
>   [`crates/ui/tests/gallery_snapshots.rs`](../../crates/ui/tests/gallery_snapshots.rs)
>   is written but `#[ignore]`d. Root cause:
>   [`crates/ui/tests/gallery_bisect.rs`](../../crates/ui/tests/gallery_bisect.rs)
>   pinpoints `GALLERY_CELLS[7]` (`strategies :: ready_v1`) as the
>   first cell that triggers a tiny-skia "Build quad rectangle"
>   panic. The iced 0.14 `widget::table::Table` used by
>   `widgets::strategies::view` interacts badly with the fixed-height
>   `cell::view` container — bumping `CELL_HEIGHT_PX` 260 → 500 does
>   NOT resolve. Needs a follow-up feature to either swap strategies
>   for a non-table render in the gallery, or special-case the
>   strategies cell wrapper. Suggested slug:
>   `ui-gallery-table-cell` (or fold into a broader
>   `ui-iced-table-cell-bounds-fix`).
> - V8 (anchors PASS gate), V9 (workspace tests + clippy +
>   fmt) — green for everything that landed; full V8 verdict gated
>   on tester pass.
> - T21 (README mention) — not done. Bin docstring at
>   [`crates/ui/src/bin/ui_gallery.rs:7-18`](../../crates/ui/src/bin/ui_gallery.rs)
>   has the `cargo run` recipe inline; README follow-up trivial.
>
> **Design-pass deviation (developer-found):**
> - `GalleryCell::render` signature changed from `fn(&Cockpit) ->
>   Element<'static, Message>` to `fn(&Cockpit) -> Element<'_,
>   Message>` because the original signature is structurally
>   incompatible with iced widgets that return borrowed Elements.
>   `cell::view` leaks the seeded cockpit via `Box::leak`
>   (test-only binary; bounded leak per render). Documented in
>   [`design.md ## Changelog`](design.md#changelog).
>
> The original M0..M5 task table below is preserved for traceability
> but is no longer the authoritative status.

> **Status:** analyst initial draft (2026-05-15). Architect Design
> pass owes Q-ARCH-1..6 resolutions
> ([`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect))
> + H-GAL-2 spike before T01 lands. T-tasks below are concrete,
> file-scoped, and ready for the developer once architect's M0
> closes.
>
> Honest-tick discipline (per
> [`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
> rule 1): developer MUST NOT tick `[x]` without citing
> (a) file:line of change, (b) test command, (c) test-output line —
> same convention as the
> [`iced-native-widgets/tasks.md`](../iced-native-widgets/tasks.md)
> Lane-1..4 commits. Tester (test-runner + evaluator split per
> [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split))
> owns the M_FINAL_* ticks.
>
> Effort budget: **3.0 dev-days total** per
> [`dev-note §5.1 row C`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#51-idea-table)
> (Effort M, ROI **High**, Risk Low). Task estimates below sum to 3.0d.

## M0 — Architect design pass + falsifier spike

Architect-owned. The orchestrator runs the H-GAL-2 spike before T01
because the entire design rests on it.

- [ ] **T-M0-A** *(architect)* — Resolve **Q-ARCH-1** (`--smoke` flag
  CLI parse). _Strawman: `clap::Parser` derive — matches `viewer`
  bin._ Cite resolution at
  [`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect).
- [ ] **T-M0-B** *(architect)* — Resolve **Q-ARCH-2** (mod-rs parse
  mechanism for exhaustiveness test). _Strawman: `include_str!` +
  naive `pub mod (\w+);` regex. Architect picks build.rs if
  `#[cfg]`-gated declarations exist._
- [ ] **T-M0-C** *(architect)* — Resolve **Q-ARCH-3** (mega-canvas
  height for operator slot). Falsifier: H-GAL-2 spike (see T-M0-S
  below). _Architect commits to scrollable-or-column shape after
  spike result lands._
- [ ] **T-M0-D** *(architect)* — Resolve **Q-ARCH-4** (cell
  render-failure handling). _Strawman: no panic-catching; rely on
  V3 exhaustiveness + V6 snapshot to catch regressions._
- [ ] **T-M0-E** *(architect)* — Resolve **Q-ARCH-5** (no
  `insta`-integration work in this brief; defer to cycle-1 item F).
  _Strawman: confirm. Reuse `tests/fixtures/visual_diff.rs` unchanged._
- [ ] **T-M0-F** *(architect)* — Resolve **Q-ARCH-6** (`README.md`
  add a `cargo run --bin ui-gallery` row). _Strawman: yes; one-line
  addition; added to T07 scope._
- [ ] **T-M0-S** *(orchestrator)* — Run **H-GAL-2 spike**:
  `iced_test::screenshot((1280, 720), 1.0, ...)` against a trivial
  `scrollable(column![...24×Container::new(text("cell N"))::height(200)...])`
  program. Report PNG dimensions back. If PNG height = 720 px,
  H-GAL-2 falsified — architect picks `column!`-no-scrollable for
  T02. If PNG height ≈ 4800 px, H-GAL-2 RESOLVED-UNFALSIFIED —
  architect keeps the scrollable design. Cite finding in
  [`feature.md ## Hypothesis register`](feature.md#hypothesis-register).
  _Effort: 0.0d (orchestrator overhead, not in 3.0d developer budget)._
- [ ] **T-M0-G** *(orchestrator)* — Run **H-GAL-3 falsifier**:
  `grep -rn 'cfg(feature = "live")' crates/ui/src/widgets/`.
  Expected empty. If non-empty, H-GAL-3 falsified — architect
  decides per-widget `#[cfg]` strategy for the gallery. _Effort:
  0.0d (orchestrator overhead)._

## M1 — Add the bin target + module skeleton (0.5d)

Target files: [`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml),
[`crates/ui/src/lib.rs`](../../crates/ui/src/lib.rs),
new `crates/ui/src/bin/ui_gallery.rs`, new
`crates/ui/src/gallery.rs`. Goal: the bin compiles and runs
`--smoke` to first-frame, even with `GALLERY_CELLS = &[]` placeholder.

- [ ] **T01** — Add `[[bin]] name = "ui-gallery", path =
  "src/bin/ui_gallery.rs", required-features = ["fixtures"]` to
  `crates/ui/Cargo.toml`. Mirror the existing `[[bin]]
  name = "cockpit"` stanza at
  [`Cargo.toml:17-20`](../../crates/ui/Cargo.toml). _Acceptance:
  `cargo build -p ui --bin ui-gallery --features fixtures` exits 0
  (V1). Estimate: 0.1d._
- [ ] **T02** — Author `crates/ui/src/gallery.rs` skeleton:
  `pub struct GalleryCell`, `pub const GALLERY_CELLS: &[GalleryCell]`
  (initially empty or one placeholder), `pub const
  EXPECTED_WIDGETS: &[&str]` from
  [`feature.md ## Design / exhaustiveness test`](feature.md#exhaustiveness-test),
  and `pub fn view(model: &Cockpit) -> Element<'_, Message>`
  rendering a `scrollable(column!(...))` of cells (or `column!`
  per T-M0-C resolution). Module-level rustdoc cites the
  Q-GALLERY-SCOPE lock and the dev-note §3.3 origin. _Acceptance:
  `cargo build -p ui --features fixtures --lib` resolves the new
  module. Estimate: 0.2d._
- [ ] **T03** — Author `crates/ui/src/bin/ui_gallery.rs` entry
  point: `clap`-parsed `--smoke` flag (per Q-ARCH-1), iced
  `Application::run`-style boot, single-window with the
  `gallery::view` Element. `--smoke` returns `Ok(())` after
  fixture-load + first-frame render without entering the iced
  event loop. _Acceptance: V2 — `cargo run -p ui --bin ui-gallery
  --features fixtures -- --smoke` exits 0 within 5 seconds, no
  panic. Estimate: 0.2d._

## M2 — Wire the 24-cell matrix (1.0d)

Target file: `crates/ui/src/gallery.rs`. Goal: every
`GalleryCell.render` closure calls the production
`crates/ui/src/widgets/*::view(...)` function with a
`fixtures.rs`-seeded `Cockpit`. Matrix per
[`feature.md ## Design / route table`](feature.md#route-table).

- [ ] **T04** — Wire cells 1–4 (positions): `loading`, `empty`,
  `ready_v1_three`, `ready_negative_pnl`. Fixtures: `fake_positions`,
  `fake_v1_three_symbol_portfolio`, `fake_pnl_negative`,
  `PanelState::Loading` / `Ready(vec![])` constructed inline (or
  via small `fake_*` extensions in `fixtures.rs`). _Acceptance:
  `cargo build -p ui --features fixtures --bin ui-gallery` succeeds;
  V3 exhaustiveness lists `positions` as covered. Estimate: 0.15d._
- [ ] **T05** — Wire cells 5–6 (pnl): `positive`, `negative`. Direct
  reuse of `fake_pnl_positive` / `fake_pnl_negative`. _Estimate: 0.05d._
- [ ] **T06** — Wire cells 7–10 (strategies): `loading`,
  `ready_v1_with_events`, `with_error_row`, `with_one_veto`.
  Fixtures: `fake_cockpit_v1_steady_state`,
  `fake_cockpit_with_strategies`, `fake_cockpit_with_one_veto`. For
  `with_error_row` the developer extends `fixtures.rs` with a
  `fake_strategy_row_error_in_v1_set()` helper (≤ 20 LOC; per
  H-GAL-4 falsifier budget). _Estimate: 0.15d._
- [ ] **T07** — Wire cells 11–12 (chart): `charts_screen_hovered`,
  `charts_screen_empty`. The `hovered` cell reuses
  [`charts_screen_with_hovered_marker`](../ui-test-harness-bootstrap/feature.md#fixture-authoring-strategy)
  via the bootstrap's existing fixture import path. The `empty`
  cell uses `fake_cockpit_ready()` with `bars = vec![]`. _Note:
  per [`feature.md ## Risks H2 caveat`](feature.md#risks) the
  hovered cell's tooltip card may not render inside the inline
  canvas; this is acknowledged and the standalone `chart_tooltip`
  cells (T11) compensate. Estimate: 0.15d._
- [ ] **T08** — Wire cells 13–14 (latency): `healthy`, `degraded`.
  Fixtures: `fake_market_health` (current shape) + a
  `fake_market_health_degraded()` helper extension. _Estimate: 0.1d._
- [ ] **T09** — Wire cells 15–17 (human_control): `auto-mode`,
  `paused`, `killed`. Direct `AgentMode` mutation on
  `fake_cockpit_ready()`. _Estimate: 0.1d._
- [ ] **T10** — Wire cells 18–19 (agent_feed): `empty`,
  `with_three_fills`. Direct reuse of
  `fake_cockpit_ready_with_three_fills`. _Estimate: 0.05d._
- [ ] **T11** — Wire cells 20–24 (num, volume_histogram,
  chart_tooltip): `num/format showcase`,
  `volume_histogram/mixed_bins`, `volume_histogram/empty`,
  `chart_tooltip/fill_tooltip`, `chart_tooltip/signal_tooltip`. New
  `fixtures.rs` helpers: `fake_volume_bins() -> Vec<VolumeBin>`,
  `fake_signal_view(n: i64) -> SignalView` (≤ 30 LOC each; per
  H-GAL-4 falsifier budget — total fixtures-additions must stay
  ≤ 80 LOC). The `num/format` cell renders a static
  `Column<Text>` of formatted outputs (no Cockpit needed) — see
  [`feature.md ## Design / route table note on num.rs`](feature.md#route-table).
  _Estimate: 0.25d._

## M3 — Add the chrome-widget single-cells (0.25d)

Target file: `crates/ui/src/gallery.rs`. Goal: V3 exhaustiveness
test passes. Each of the 12 chrome-widget modules listed in
[`feature.md ## Design / exhaustiveness test`](feature.md#exhaustiveness-test)
(`chart_legend`, `drawdown_band`, `equity_curve`, `focus_ring`,
`frame`, `journal_transaction_modal`, `kill`, `kpi_strip`,
`override_risk_veto`, `sidebar_nav`, `sparkline`, `status_bar`)
gets one representative cell.

- [ ] **T12** — Wire single-cells for `kpi_strip`, `status_bar`,
  `sidebar_nav`. Reuse `fake_cockpit_v1_steady_state()` for all
  three; each widget's `view(model)` reads the parts it needs.
  _Estimate: 0.1d._
- [ ] **T13** — Wire single-cells for `kill`, `human_control`
  (already covered by T09 — confirm), `override_risk_veto`,
  `journal_transaction_modal`. Reuse `fake_cockpit_with_one_veto`
  + `fake_journal_rows(3)`. _Estimate: 0.1d._
- [ ] **T14** — Wire single-cells for `chart_legend`,
  `drawdown_band`, `equity_curve`, `focus_ring`, `frame`,
  `sparkline`. Most are chart-chrome widgets; reuse `fake_cockpit_ready`
  + the bootstrap's chart fixtures. _Note: architect may demote
  some to "covered via parent cell" in M0; rebalance estimate
  downward in that case. Estimate: 0.05d._

## M4 — Exhaustiveness + snapshot tests (0.75d)

Target files: `crates/ui/src/gallery.rs` (unit tests in
`#[cfg(test)] mod tests`), new
`crates/ui/tests/gallery_snapshots.rs`. Goal: V3, V4, V6 green.

- [ ] **T15** — Unit test
  `gallery::tests::every_expected_widget_has_at_least_one_gallery_cell`
  (per
  [`feature.md ## Design / exhaustiveness test`](feature.md#exhaustiveness-test)).
  _Acceptance: V3 — `cargo test -p ui --features fixtures
  gallery::tests::every_expected_widget_has_at_least_one_gallery_cell`
  exits 0. Estimate: 0.15d._
- [ ] **T16** — Unit test
  `gallery::tests::every_widget_mod_is_listed_in_expected_widgets`
  (per Q-ARCH-2 resolution — `include_str!` + regex parse). Strawman
  regex: `r"^pub(?:\(crate\))? mod (\w+);"` matched per line of
  `widgets/mod.rs`. _Acceptance: V4 — same `cargo test` form for
  the second test name. Estimate: 0.15d._
- [ ] **T17** — Author `crates/ui/tests/gallery_snapshots.rs` with
  the same shape as
  [`tests/visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs):
  `const SLOTS` mirroring the bootstrap's
  [`floor / typical / operator`](../ui-test-harness-bootstrap/feature.md#r2--viewport-matrix-dev-note-3-layer-3)
  table; three `#[test] fn`s (`ui_gallery_dark_floor`,
  `_typical`, `_operator`) driving `iced_test::screenshot(...)`
  + the bootstrap's `matches_screenshot` helper at
  [`tests/fixtures/visual_diff.rs`](../../crates/ui/tests/fixtures/visual_diff.rs)
  unchanged. Baselines: `tests/visual-baselines/ui_gallery_dark_*.png`.
  _Acceptance: V6 — `cargo test -p ui --features fixtures --test
  gallery_snapshots` exits 0; three baseline PNGs land under
  `tests/visual-baselines/`. Estimate: 0.3d._
- [ ] **T18** — Verify V10 (snapshot determinism): the developer
  runs `cargo test -p ui --features fixtures --test
  gallery_snapshots` twice consecutively; `shasum -a 256
  tests/visual-baselines/ui_gallery_dark_*.png` produces identical
  hashes on both runs. _Acceptance: V10 — two hash runs match;
  `git status tests/visual-baselines/` shows zero modifications
  between runs. Estimate: 0.05d (the test itself runs in seconds;
  the budget is the hash + git-status spot-check)._
- [ ] **T19** — Operator-slot PNG size check (Risk
  mitigation per
  [`feature.md ## Risks`](feature.md#risks)). Developer reports
  `ls -lh tests/visual-baselines/ui_gallery_dark_operator.png`.
  If > 10 MB, escalates to architect for the gallery-split design
  (six baselines instead of three). If ≤ 10 MB, proceed. _Acceptance:
  size noted in M4 verification. Estimate: 0.05d (filesystem check)._
- [ ] **T20** — Anchors PASS gate (V8). Developer runs `bash
  scripts/verify_anchors.sh`; expects `ANCHORS PASS (11/11)`. _The
  feature touches zero non-UI crates, so the gate is a sanity
  check, not an expected failure. Estimate: 0.05d._

## M5 — README + presenter-deck artifact (0.5d)

Target files: [`README.md`](../../README.md) (one-line addition),
new `spec/ui-gallery-bin/presentations/ui-gallery-bin-2026-05-XX.md`
(presenter authors; deferred to post-PASS). Goal: V7, V9 close;
operator-facing discoverability lands.

- [ ] **T21** — Add a `cargo run -p ui --bin ui-gallery --features
  fixtures` invocation row to
  [`README.md`](../../README.md) under whatever section currently
  lists `cargo run --bin cockpit`. Cross-link to
  [`feature.md`](feature.md). Per Q-ARCH-6 resolution. _Acceptance:
  V9 — `git diff --name-only HEAD~..HEAD` includes `README.md`.
  Estimate: 0.05d._
- [ ] **T22** — Manual smoke + presenter-deck screenshot. The
  **orchestrator** (not a sub-agent — per
  [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries))
  runs `cargo run -p ui --bin ui-gallery --features fixtures`,
  captures the window at 1920×1080 via `screencapture`, embeds the
  PNG in
  `spec/ui-gallery-bin/presentations/ui-gallery-bin-2026-05-XX.md`
  for the operator approval pass. _Note: per
  [`AGENT.md`](../../AGENT.md) the presenter runs after `VERDICT →
  PASS`; this T22 is a placeholder for the deck-authoring step,
  not a sub-agent deliverable. Estimate: 0.15d (presenter overhead;
  not counted in developer budget, but tracked here for
  cycle-completeness)._
- [ ] **T23** — Workspace-test green gate (V7). Developer / tester
  runs `cargo test --workspace --features fixtures`; expects zero
  failures and ≥ (prior pass count + V3 + V4 + V6 sub-tests) green.
  _Acceptance: V7. Estimate: 0.1d (test runtime; verification only)._
- [ ] **T24** — `cargo fmt` + `cargo clippy --workspace --features
  fixtures -- -D warnings` clean. Per
  [`CLAUDE.md ## Coding rules`](../../CLAUDE.md#coding-rules).
  _Estimate: 0.05d._
- [ ] **T25** — Tick this tasks.md via `spec-update`. Per
  [`AGENT.md ## Process discipline rule 1`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
  honest-tick discipline: every prior `[x]` cites
  (a) file:line of change, (b) test command, (c) test-output line.
  _Estimate: 0.15d._

## M_FINAL_TEST_RUN — test-runner pass

Test-runner spawn (per
[`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split)).
Owns the V1..V10 verification matrix tick. Emits
`spec/ui-gallery-bin/reports/test-<ts>.md` per the
[`rust-test`](../../.claude/skills/rust-test/templates/test-report.md)
template.

- [ ] **T-FINAL-TEST-1** — V1, V2, V5 (build + smoke + tiny-skia
  backend gate). Single `cargo build -v` + `cargo run -- --smoke`
  invocation. _Test-runner owns; cite verbatim output._
- [ ] **T-FINAL-TEST-2** — V3, V4 (exhaustiveness tests). Single
  `cargo test -p ui --features fixtures gallery::tests` invocation.
- [ ] **T-FINAL-TEST-3** — V6, V10 (gallery_snapshots first-run +
  second-run determinism). Two consecutive `cargo test ... --test
  gallery_snapshots` runs; SHA-compare the three baselines between
  runs.
- [ ] **T-FINAL-TEST-4** — V7 (workspace green) + V8 (anchors PASS
  11/11). `cargo test --workspace --features fixtures` + `bash
  scripts/verify_anchors.sh`.
- [ ] **T-FINAL-TEST-5** — V9 (file-list gate). `git diff
  --name-only` against the pre-feature commit; verify only
  in-scope paths changed.

## M_FINAL_EVAL — evaluator pass

Evaluator spawn (per
[`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split)).
Reads the test-runner's report + cites the verification matrix +
emits `VERDICT → PASS | FAIL | REGRESSION` at
`spec/ui-gallery-bin/reports/evaluation-<ts>.md`.

- [ ] **T-FINAL-EVAL-1** — Read test-runner report; cross-check
  V1..V10 ticks against
  [`feature.md ## Acceptance / verification (V-items)`](feature.md#acceptance--verification-v-items).
  Emit verdict.

## Notes

- **Effort sum (developer-owned T01..T25):** 0.1 + 0.2 + 0.2 + 0.15
  + 0.05 + 0.15 + 0.15 + 0.1 + 0.1 + 0.05 + 0.25 + 0.1 + 0.1 + 0.05
  + 0.15 + 0.15 + 0.3 + 0.05 + 0.05 + 0.05 + 0.05 + 0.15 + 0.1 +
  0.05 + 0.15 = **3.0 dev-days**, matching the
  [`dev-note §5.1 row C`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#51-idea-table)
  budget exactly.
- **Architect M0 tasks (T-M0-A..G) are not in the 3.0d budget** —
  architect-pass overhead per the
  [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)
  separation.
- **Test-runner + evaluator M_FINAL tasks are not in the 3.0d budget**
  — same reason; these are the tester pair's overhead per the
  test-runner/evaluator split.
- **No new external crates** expected (per feature.md ## Dependencies).
  If T03 forces `clap` to be re-imported under the bin's compile
  scope, the workspace `clap` dep is already wired
  ([`Cargo.toml:65`](../../crates/ui/Cargo.toml)).
- **Q-ARCH-3 contingency:** if H-GAL-2 spike (T-M0-S) falsifies the
  scrollable approach, T02 / T17 baseline-size estimates shift —
  operator-slot PNG could grow 3-5x (no scrollable means full-content
  intrinsic height). T19 catches the breach; T-M0-C is the resolution
  point.
- **Q-GALLERY-SCOPE drift gate:** any developer PR that adds a
  local fixture-builder inside `gallery.rs` (rather than extending
  `fixtures.rs`) violates the Q-GALLERY-SCOPE lock. Evaluator
  enforces in T-FINAL-EVAL-1 via `grep -n 'fn fake_\|fn synth' crates/ui/src/gallery.rs`
  (expected empty).
