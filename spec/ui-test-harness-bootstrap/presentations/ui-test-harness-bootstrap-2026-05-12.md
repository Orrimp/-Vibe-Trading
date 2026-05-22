---
slug: ui-test-harness-bootstrap
mode: release
status: shipped
audience: human-operator
updated: 2026-05-12
generated: 2026-05-12T14:00:00Z
---

# UI test harness bootstrap v0.1 — release

## TL;DR

Week-1 of the [dev-note 4-week adoption plan](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan) ships: `iced_test::screenshot` smoke test for the Charts screen at **three viewport slots** (floor 1280x720, typical 1920x1080, operator 3360x1890 @ 2.0), a **canvas hit-test grid sweep** that would have caught the original `cursor.position_in(bounds)?` early-bail bug, a **viewport-parametric helper** on `dispatch_canvas_event_for_test`, `image-compare` perceptual-diff forensics on snapshot failure, and a **`scripts/check_no_clocks_in_ui_tests.sh`** determinism gate. Evaluator (read-only, fresh context) emitted **`VERDICT → PASS`** with all 9 V-items satisfied and 11/11 anchors byte-identical. V8 (chart-canvas-overhaul V15 closure) ships **detection-only** with a partial-accept caveat: the `Cockpit.chart_tooltip = Some(..)` fixture seeds detection but does NOT render the tooltip card because `ChartProgram::State` (not `Cockpit`) owns canvas hover render state — render half deferred to `ui-test-harness-canvas-state-seeding` candidate in [backlog.md](../../backlog.md#process--tooling). Weeks 2-4 (full-widget viewport matrix, evaluator PreToolUse hooks, GHA CI) are separate features queued in backlog. This is **test infrastructure only** — zero changes to non-UI crates, zero anchor drift, 68 insta text snapshots stay byte-identical.

## What changed

- **`crates/ui/Cargo.toml:103-108`** — added `[dev-dependencies]` `iced_test = "=0.14.0"`, `image-compare = "=0.4"`, `image = "=0.25.6"` (PNG decode for visual-diff). Test-only; not reachable from production.
- **`crates/ui/src/test_support.rs`** (new module wired at [`crates/ui/src/lib.rs:43-50`](../../../crates/ui/src/lib.rs)) — `charts_screen_cockpit()` + `program_from_cockpit()` test-only factory (Q1 resolution, option (b)). Compiles under default features (H5 falsifier — no `--features fixtures` opt-in needed).
- **`crates/ui/tests/visual_snapshots.rs`** (new) — 4 `#[test] fn`s: `charts_screen_dark_floor`, `_typical`, `_operator`, and `visual_diff_helper_writes_diff_png_on_mismatch` (V9 self-test). Drives `iced_test::screenshot(&program, &theme, viewport, scale, Duration::ZERO)` and byte-compares `Screenshot.rgba` against the committed PNG baseline.
- **`crates/ui/tests/chart_hover_grid_sweep.rs`** (new) — 4 `#[test] fn`s: `cursor_grid_sweeps_every_marker_at_three_viewports`, `v15_chart_canvas_overhaul_closure_at_operator_slot` (V8), `sweep_helper_bounds_match_simulator_layout` (H3 falsifier), `marker_centroid_pixel_invariants_across_viewports` (T4023).
- **`crates/ui/tests/fixtures/mod.rs`** + **`crates/ui/tests/fixtures/visual_diff.rs`** (new) — `charts_screen_with_hovered_marker()` Q9 fixture + `matches_screenshot(...)` + `matches_rgb_buffers(...)` perceptual-diff wrapper (writes `target/visual-diff/<test>.png` on mismatch via `image_compare::rgb_hybrid_compare`).
- **`crates/ui/src/widgets/chart.rs:1043-1131`** — added three test-only helpers: `sweep_canvas_grid_for_test`, `inner_rect_for_viewport_test`, `anchor_for_first_fill_test`. Existing `dispatch_canvas_event_for_test` signature at [`chart.rs:981`](../../../crates/ui/src/widgets/chart.rs) is unchanged (R3.6 backward-compat).
- **`scripts/check_no_clocks_in_ui_tests.sh`** (new) — grep gate watching 8 files for `SystemTime::now`, `Instant::now`, `thread_rng`, `UtcOffset::current_local_offset` with `// CLOCK-OK:` whitelist marker support. Verbatim `CLOCKS PASS (8 files / 4 patterns)` on the clean tree (log:2454).
- **Three baseline PNGs** at [`crates/ui/tests/visual-baselines/`](../../../crates/ui/tests/visual-baselines/) (operator-locked Q10 slot names): `charts_screen_dark_{floor,typical,operator}.png` — committed to git.
- **`crates/ui/src/widgets/chart.rs:152`** — added one `// CLOCK-OK:` whitelist marker (preceding-line marker against an intentional doc-comment reference to `UtcOffset::current_local_offset`).

## What changed in process

This feature is the **first run under the new [AGENT.md ## Capability boundaries](../../../AGENT.md#capability-boundaries) regime AND the first run of the test-runner / evaluator split.** It is itself a meta-deliverable: the bootstrap was the trial-run for the workflow amendment.

- **Capability boundaries** were [adopted 2026-05-12](../../../AGENT.md#capability-boundaries) after the [chart-canvas-overhaul v1.10.0 retrospective](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md). The amendment establishes that sub-agents are context tools, not capability tools — `screencapture`, `osascript`, `cargo run --bin cockpit` with a live window, and "concluding the bug is X from live-app instrumentation" all belong to the orchestrator, never to a sub-agent. The dev-note ([§2 The orchestrator vs. sub-agent capability asymmetry](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#2-the-orchestrator-vs-sub-agent-capability-asymmetry)) is the proposal; the amendment is the contract.
- **Test-runner / evaluator split** ([AGENT.md ## Test-runner / evaluator split](../../../AGENT.md#test-runner--evaluator-split)) replaced the single `tester` agent for this feature. Empirical proof the pattern works:
  - **test-runner** (write-allowed) emitted [`reports/test-run-2026-05-12T12-43Z.log`](../reports/test-run-2026-05-12T12-43Z.log) — raw output only, no verdict. Honestly recorded four sandbox-denied steps (verbatim denials at log:2277-2293) instead of rationalizing around them.
  - **evaluator** (read-only, fresh context, never saw the developer diff) emitted [`reports/evaluation-2026-05-12T13-15Z.md`](../reports/evaluation-2026-05-12T13-15Z.md) — cited log section + line number for every PASS row; emitted `VERDICT → PASS` (evaluation.md:21,191-194).
- **Hypothesis-only architect** ([AGENT.md ## Architect = hypothesis only](../../../AGENT.md#architect--hypothesis-only)). The architect authored H1-H5 with explicit falsifiers in [`feature.md ## Hypothesis register`](../feature.md#hypothesis-register); the orchestrator ran the H1 twice-consecutive determinism falsifier (log:2475-2508) and the H2 baseline visual review. No architect-led "the bug is X" claim was made from live-app instrumentation — the [chart-canvas-overhaul "iced has a half-scale canvas bug" misdiagnosis](../../chart-canvas-overhaul/feature.md#diagnostic--corrected-2026-05-12-orchestrator-led) (1.5 dev-days of dead code) is the prior incident this rule prevents.
- **No sub-agent attempted a screencapture or live-cockpit launch.** Every V-item below is executable from a sub-agent's sandbox (`cargo test`, `bash scripts/*.sh`, `git diff`). The one operator-touchpoint (H2 visual baseline review) was explicitly an orchestrator-runnable gate, not a sub-agent step — and it surfaced the V8 render-gap caveat exactly as the regime intended.

The pipeline shape held cleanly: analyst → architect → developer → test-runner → evaluator → presenter, no rework loops, no capability boundary violations, no `[x]` on a task that wasn't actually done.

## Why

The [chart-canvas-overhaul v1.10.0 retrospective](../../chart-canvas-overhaul/feature.md) surfaced a workflow bug, not a chart bug: a 9-agent pipeline shipped a feature whose operator-verification step was a manual 30-second `Cmd+Shift+4`, and walking the 818-test suite confirmed **no test would have caught the tooltip-invisible-at-3360x1890 bug** ([dev-note §1](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#1-what-broke--evidence)). The two existing chart-hover tests both used fixed `(100, 50, 800x600)` canvas bounds — one viewport, no rendered pixels — and the 68 insta snapshots were all text-summary.

This bootstrap implements the [dev-note §3 Layer 1 + Layer 4 + Layer 5](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#3-ui-testing--concrete-recommended-stack) recommendations (week-1 of the 4-week plan) under the operator decisions D1-D5 from [§9](../../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator):

- **D1** — adopt all 5 TL;DR recommendations as a single block (load-bearing on each other).
- **D2** — full analyst → architect → developer pipeline, not solo dev pass.
- **D3** — macOS-only; cross-platform deferred.
- **D4** — chart-canvas-overhaul V15 acceptance defers to this feature's week-1 snapshot test (V8 below).
- **D5** — AGENT.md `## Capability boundaries` amendment is LIVE (committed 2026-05-12).

## What you can do now

| Action | Command |
|--------|---------|
| Run the visual snapshot suite (3 viewports + V9 self-test) | `cargo test -p ui --test visual_snapshots` |
| Run the canvas hit-test grid sweep (coarse, ~5k cells, sub-100ms) | `cargo test -p ui --test chart_hover_grid_sweep` |
| Run the dense grid sweep (~22k cells, sub-200ms) | `CHART_HIT_TEST_GRID=dense cargo test -p ui --test chart_hover_grid_sweep` |
| Run only the chart-canvas-overhaul V15 closure assertion | `cargo test -p ui --test chart_hover_grid_sweep v15_chart_canvas_overhaul_closure_at_operator_slot` |
| Re-verify the 11 backtest-report anchors are byte-identical | `bash scripts/verify_anchors.sh` |
| Re-verify the no-clocks-in-snapshot-path gate | `bash scripts/check_no_clocks_in_ui_tests.sh` |
| Confirm zero changes to non-UI crates | `git diff --name-only HEAD~..HEAD` |
| Open the three baseline PNGs in Preview to spot-check render | `open crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png` |

**Note on `cargo insta review`:** does **NOT** apply to the new binary baselines in v0.1. `iced_test::Snapshot` 0.14.0 exposes only `matches_image(path)` / `matches_hash(path)` — no public PNG-byte accessor — so the v0.1 helper byte-compares `Screenshot.rgba` directly and writes diff PNGs to `target/visual-diff/<test>.png` on mismatch (operator opens the triple manually: baseline / actual / diff). The 68 existing text insta snapshots are unaffected and `cargo insta review` continues to work for them. The `insta` binary-snapshot integration is deferred to week 2 — see [`feature.md ## Design ## cargo insta review integration gap`](../feature.md#cargo-insta-review-integration-gap).

**Clocks-grep behavior:** the script greps the 8 watchlist files for the 4 forbidden patterns. A match is allowed if a `// CLOCK-OK: <reason>` marker is on the same line or the preceding line. Clean tree returns `CLOCKS PASS (8 files / 4 patterns)`; injecting `std::time::SystemTime::now()` into any watchlist file returns `FAIL <file>:<line> — unwhitelisted 'SystemTime::now'` with `exit: 1` (proven by the V4 inject-and-stash falsifier — log:2511-2523).

## Live demo

Verbatim from the test-runner + orchestrator-supplement run log ([`reports/test-run-2026-05-12T12-43Z.log`](../reports/test-run-2026-05-12T12-43Z.log)):

```
$ cargo test -p ui --test visual_snapshots
   Compiling ui v0.1.0 (/Users/.../trading/crates/ui)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.21s
     Running tests/visual_snapshots.rs (target/debug/deps/visual_snapshots-eb2b8a4210a837da)

running 4 tests
test visual_diff_helper_writes_diff_png_on_mismatch ... ok
test charts_screen_dark_floor ... ok
test charts_screen_dark_typical ... ok
test charts_screen_dark_operator ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.38s
```

```
$ cargo test -p ui --test chart_hover_grid_sweep
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.68s
     Running tests/chart_hover_grid_sweep.rs (target/debug/deps/chart_hover_grid_sweep-dcd71ca222ca70e1)

running 4 tests
test sweep_helper_bounds_match_simulator_layout ... ok
test v15_chart_canvas_overhaul_closure_at_operator_slot ... ok
test marker_centroid_pixel_invariants_across_viewports ... ok
test cursor_grid_sweeps_every_marker_at_three_viewports ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994
PASS  report-sample-90d                     463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c
---
ANCHORS PASS  (11 / 11)
```

Notice the verbatim `ANCHORS PASS  (11 / 11)` line — every backtest-report body SHA is byte-identical to the locked value in [`spec/anchors.toml`](../../anchors.toml). The H1 determinism falsifier (twice-consecutive `cargo test` + shasum, log:2475-2508) returned `diff-exit: 0` on all three baselines, confirming tiny-skia CPU determinism empirically.

## Screenshots

The three committed baseline PNGs **ARE** the artifact — they are the proof of what the operator-slot Charts screen renders to today. The operator should open them locally to spot-check.

| Slot | Path | Logical viewport | Scale | Physical PNG | Bytes (first-run dev pass) |
|------|------|------------------|-------|--------------|----------------------------|
| floor | [`crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`](../../../crates/ui/tests/visual-baselines/charts_screen_dark_floor.png) | 1280 x 720 | 1.0 | 1280 x 720 | 83,456 |
| typical | [`crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`](../../../crates/ui/tests/visual-baselines/charts_screen_dark_typical.png) | 1920 x 1080 | 1.0 | 1920 x 1080 | 129,681 |
| operator | [`crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`](../../../crates/ui/tests/visual-baselines/charts_screen_dark_operator.png) | 3360 x 1890 | 2.0 | 6720 x 3780 | 796,904 |

The evaluator independently verified PNG dimensions via `file(1)` and recorded them in [`evaluation-2026-05-12T13-15Z.md ## Artifact verification`](../reports/evaluation-2026-05-12T13-15Z.md) — operator slot is exactly 2.0x the logical viewport. SHAs from the H1 falsifier run:

```
73289bdfb7b385f60afd548a9f5b2193816539216844fa523f52af75774d1651  charts_screen_dark_floor.png
a4a96ba0d5e0b86fc92ffe679aab1d2fcce69e285c3955b6006747a0f58e9fff  charts_screen_dark_typical.png
85b73747eb717003b0c8a6c7c273e2ee42f5b57bb833c9ba91fc57b69ebea60c  charts_screen_dark_operator.png
```

To open all three at once: `open crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png`.

**Caveat (logged in H2 status, [feature.md:705-727](../feature.md#hypothesis-register)):** the Q9 fixture's `Cockpit.chart_tooltip = Some(ChartTooltipView{..})` does NOT render as a visible tooltip card in these PNGs because the chart-buy-sell-emphasis v1.9.0 T2033 refactor decoupled tooltip rendering from `Cockpit` state — the canvas reads hover state from its internal `ChartProgram::State`, not from `self.tooltip`. The axes, legend, markers, status strip, and sidebar all render fully. The render gap is acknowledged and queued — see V8 row + Open decisions.

## Verification

Full V1-V9 matrix from [`evaluation-2026-05-12T13-15Z.md ## V-items`](../reports/evaluation-2026-05-12T13-15Z.md):

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | `iced_test` smoke compiles + passes (3 slot-named `#[test] fn`s green) | VERIFIED | `cargo test -p ui --test visual_snapshots` exit 0; `4 passed; 0 failed` (log:2241/2248-2256) |
| V2 | Three viewport baselines committed and bit-stable (twice-run zero-diff) | VERIFIED | H1 falsifier shasum twice-run, `diff-exit: 0` (log:2475-2508); `target/visual-diff/` shows only V9 self-test diff (log:2402-2410); `git status crates/ui/tests/visual-baselines/` shows only `Untracked files`, zero modifications (log:2387-2400) |
| V3 | Canvas hit-test grid sweeps all centroids at all three viewports | VERIFIED | `cargo test -p ui --test chart_hover_grid_sweep` exit 0; `cursor_grid_sweeps_every_marker_at_three_viewports ... ok` (log:2258-2272); dense-mode (orchestrator-run) `4 passed; finished in 0.13s` (log:2458-2470) |
| V4 | Determinism contract holds (`check_no_clocks_in_ui_tests.sh` PASS on clean tree, FAIL on SystemTime injection) | VERIFIED | `CLOCKS PASS (8 files / 4 patterns)` exit 0 (log:2454-2456); inject-and-stash `FAIL crates/ui/tests/visual_snapshots.rs:196 — unwhitelisted 'SystemTime::now'` exit 1 then revert `CLOCKS PASS` exit 0 (log:2511-2523) |
| V5 | `verify_anchors.sh` PASS 11/11 before and after | VERIFIED | 11 verbatim `PASS` lines + `ANCHORS PASS (11 / 11)` exit 0 (log:2432-2449) |
| V6 | Full workspace test suite stays green | VERIFIED | `cargo test --workspace` exit 0 (log:2239); zero `failed` occurrences in workspace block (log:282-2239); net-new targets present: `chart_hover_grid_sweep.rs` 4 passed (log:1939), `visual_snapshots.rs` 4 passed (log:2145); 68 existing `panel_snapshots` baseline green (log:2093) |
| V7 | Zero changes to non-UI crates | VERIFIED | `git diff --name-only HEAD~..HEAD` (log:2412-2422) lists only 4 files, all under `spec/**`; ui-test-harness-bootstrap working-tree additions confined to `crates/ui/**`, `scripts/`, `Cargo.toml`, `Cargo.lock` per `Untracked files` listing (log:2394-2396) |
| V8 | chart-canvas-overhaul V15 closure — named sub-test at operator slot publishes `ChartMarkerHovered(Fill(0)) + Status::Captured` | **PASS-with-H2-caveat** — detection half closed; render half deferred | `v15_chart_canvas_overhaul_closure_at_operator_slot ... ok` (log:2266, log:1935 inside workspace run); render half is the H2-caveat carve-out — operator decision **"Commit — V14 covered, V15 partial-accept"** (feature.md ## Changelog 2026-05-12 operator entry, feature.md:938-943); render-half follow-up queued as [`ui-test-harness-canvas-state-seeding`](../../backlog.md#process--tooling) candidate |
| V9 | Perceptual diff materializes on `matches_image` failure | VERIFIED | `visual_diff_helper_writes_diff_png_on_mismatch ... ok` (log:2249); `target/visual-diff/visual_diff_helper_writes_diff_png_on_mismatch.png` materialized on disk (log:2408) |

## Hypothesis register summary

From [`evaluation-2026-05-12T13-15Z.md ## Hypothesis register`](../reports/evaluation-2026-05-12T13-15Z.md):

- **H1 — tiny-skia CPU determinism holds across two runs on the same machine. → RESOLVED-UNFALSIFIED.** Orchestrator-run twice-consecutive shasum, `diff-exit: 0` across all three baselines (log:2475-2508).
- **H2 — `iced_test::screenshot(..., Duration::ZERO)` produces a fully-rendered frame on first call. → RESOLVED-WITH-CAVEAT (operator + orchestrator, 2026-05-12).** Axes / legend / markers / status strip / sidebar all render fully — `Duration::ZERO` is correct. CAVEAT: Q9 fixture's `Cockpit.chart_tooltip = Some(..)` does NOT manifest as a tooltip-card render because `ChartProgram::State` (not `Cockpit`) owns canvas hover state. Operator decision: "Commit — V14 covered, V15 partial-accept" (feature.md:938-943). Render half queued as `ui-test-harness-canvas-state-seeding` backlog candidate. **Not blocking v0.1 ship per operator lock.**
- **H3 — viewport-parametric extension to `dispatch_canvas_event_for_test` correctly recreates production canvas bounds at non-default viewports. → RESOLVED-UNFALSIFIED.** `sweep_helper_bounds_match_simulator_layout ... ok` (log:2265, 1934); helper-computed inner rect matches explicit gutter expectation byte-by-byte.
- **H4 — `image-compare`'s `rgb_hybrid_compare` produces a human-actionable diff PNG. → RESOLVED-UNFALSIFIED at the sandbox-runnable bar.** V9 test ok (log:2249) + on-disk diff PNG materialized (log:2408). Visual inspection of the diff PNG by the operator during presenter pass remains the brief's full closure path (feature.md H4 falsifier).
- **H5 — Q1 test-only factory compiles without `--features fixtures` / `--features live`. → RESOLVED-UNFALSIFIED.** `cargo build --workspace` finishes in 5.02s exit 0 (log:274-278); ui crate compiles with `--tests` under default features only (developer-pass T4011, tasks.md:73-79).

## Numbers that matter

- **Tests passing:** workspace `cargo test --workspace` exit 0 (log:2239), zero failed tests anywhere in the 1953-line workspace block (log:282-2239). The brief's expected envelope of "**818 baseline + 8 net-new** = 826" is satisfied — net-new break down as 4 in `visual_snapshots.rs` (log:2145) + 4 in `chart_hover_grid_sweep.rs` (log:1939). 68 existing `panel_snapshots` text baselines stay byte-identical green (log:2093). 6 existing `chart_tooltip_hover_fires` tests stay green (log:1966) confirming R3.6 backward-compat.
- **Anchors:** **11 / 11 PASS** verbatim (log:2447) — zero anchor drift, every backtest-report body SHA byte-identical to [`spec/anchors.toml`](../../anchors.toml).
- **New `#[test] fn`s added:** **8** total (4 visual snapshots including V9 self-test + 4 grid sweep including V8 and H3 falsifier).
- **New baseline PNGs committed:** **3** (sizes: 83,456 B / 129,681 B / 796,904 B; SHAs above).
- **Non-UI-crate changes:** **0** — `git diff --name-only HEAD~..HEAD` shows only `spec/**` paths (log:2412-2422).
- **Sub-agent capability-boundary violations:** **0** — no sub-agent attempted `screencapture`, `osascript`, or live-cockpit launch. Sandbox-denied steps were honestly recorded and escalated for orchestrator runs (log:2277-2293).
- **Sandbox-denied steps re-run by orchestrator supplement:** **4** — dense-mode grid sweep, clocks-grep clean run, anchors gate, H1 twice-run shasum — all PASS (log:2428-2526).
- **Determinism wall-clock:** visual_snapshots first run 6.38s, second consecutive run 6.31s; grid sweep coarse 0.04s, dense 0.13s; clocks-grep instant; anchors gate instant.
- **Test compile-cost:** `cargo build -p ui --tests` 25.53s (T4011 dev-pass); ui crate recompile on test run 2.21s (log:2245). No `--features fixtures` / `--features live` required.

## Open decisions

_None pending — ready to ship._

The single operator-touch point (H2 baseline visual review) has already been resolved with the **"Commit — V14 covered, V15 partial-accept"** decision logged in [`feature.md ## Changelog 2026-05-12 (operator)`](../feature.md#changelog) (feature.md:938-943). The render-half gap for V8 is queued — not blocking — as [`ui-test-harness-canvas-state-seeding`](../../backlog.md#process--tooling) in the backlog and analyst spawn happens after this v0.1 ships. Weeks 2-4 (full-widget viewport matrix, evaluator PreToolUse hooks, GHA CI + presenter integration) are separate features queued behind this one — also non-blocking.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-12 (presenter): initial draft. Cited evaluator `VERDICT → PASS` from [`reports/evaluation-2026-05-12T13-15Z.md`](../reports/evaluation-2026-05-12T13-15Z.md), embedded the verbatim `ANCHORS PASS (11 / 11)` line and the two `cargo test` runs from the test-runner log, full V1-V9 matrix with V8 marked PASS-with-H2-caveat, H1/H3/H4/H5 RESOLVED-UNFALSIFIED + H2 RESOLVED-WITH-CAVEAT. Three baseline PNG paths + dimensions + SHAs cited. Called out this feature as the first run under the new [AGENT.md ## Capability boundaries](../../../AGENT.md#capability-boundaries) regime and the first run of the test-runner / evaluator split — meta-deliverable acknowledged in `## What changed in process`. All approval boxes UN-ticked.
- 2026-05-12 (operator): `[x] Approved — ship`. Pre-tick gate PASS;
  evaluator `VERDICT → PASS`; anchors PASS 11/11; V8 PASS-with-H2-
  caveat carries forward to `ui-test-harness-canvas-state-seeding`
  backlog candidate. Status flipped `draft → shipped`. This feature
  is the first run of both the AGENT.md `## Capability boundaries`
  regime and the test-runner / evaluator split — empirical proof
  the new workflow holds under feature load.
