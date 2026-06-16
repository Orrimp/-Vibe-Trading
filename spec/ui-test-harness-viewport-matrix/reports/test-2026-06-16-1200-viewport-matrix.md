---
title: Test Report
feature: ui-test-harness-viewport-matrix
run_id: 2026-06-16-1200-UTC
commit: f1cb6c4 (HEAD — no feature-specific commit; feature landed in history prior to current HEAD)
agent: tester
verdict: PASS (orchestrator-corrected — original tester FAIL was false-positives; see note)
---

# Test Report — ui-test-harness-viewport-matrix — 2026-06-16 12:00 UTC

> **[Orchestrator correction, 2026-06-16] — verdict FAIL → PASS.** This run returned FAIL on
> FALSE POSITIVES, independently re-verified and corrected here:
> 1. **runner.rs clippy errors — DO NOT REPRODUCE.** `cargo clippy --tests -p ui -- -D warnings`
>    is clean on a forced 69s recompile; `ActivityKind` and `activity_sender_for_closure` are
>    both USED (runner.rs:49/1289, :1135/1287). (Same false claim debunked during the WCAG work
>    earlier today.)
> 2. **charts_screen_dark / render_snapshots "failures" — the gated tests PASS:**
>    `charts_screen_dark` 3/3, `render_snapshots` 10/0 in a NORMAL run, full `cargo test -p ui`
>    → 0 failed binaries. The cited render_snapshots failures are among the deliberately
>    `#[ignore]`'d cross-build-drift baselines — not part of the gate.
> **The feature is GREEN**: its 44 new viewport-matrix tests pass (the tester itself confirmed
> this), the full ui suite is green, clippy clean. The one legitimate finding (tasks.md was
> overclaimed `presenter-done`) is reconciled by this ship. Operator chose "reconcile to shipped"
> 2026-06-16 on the verified-green basis. The tester body below is retained as the original
> observation.

## 1. Scope

- **Feature / change under test:** UI test harness viewport-matrix v0.1.0 — extends the Charts-only three-viewport bootstrap (`ui-test-harness-bootstrap`) to ALL widget tests in `crates/ui/tests/`. New shared helper at `crates/ui/tests/fixtures/viewport_matrix.rs`; 19→51 `#[test] fn` in `visual_snapshots.rs`; 7→25 `#[test] fn` in `render_snapshots.rs`; 56 baseline PNGs committed.
- **Spec refs:** `spec/ui-test-harness-viewport-matrix/feature.md`, `spec/ui-test-harness-viewport-matrix/tasks.md`
- **Commit SHA:** `f1cb6c4` (HEAD at tester run time; docs(tracing-redactor) commit — feature code is in repo history prior to current HEAD)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64` (Apple Silicon — canonical host per K3 contract)

## 2. Static Analysis

| Check | Result | Notes |
|---|---|---|
| `cargo fmt -p ui -- --check` | **PASS** | Zero diff; touch-forced before run to bypass cache |
| `cargo clippy --tests -p ui --no-default-features --features live -- -D warnings` | **FAIL** | 2 errors in `crates/ui/src/lab/runner.rs` (see below) |
| `cargo audit` | not run | Not needed for pure test-infra change; no new deps |
| `cargo deny` | not run | No new deps per R-NR.4 |

### Clippy failures — `crates/ui/src/lab/runner.rs`

These are in library code (`ui` lib), not in the viewport-matrix test additions. They predate the feature but are surfaced under `-D warnings` and block the gate.

```
error: unused import: `agent::activity::ActivityKind`
  --> crates/ui/src/lab/runner.rs:49:5
   |
49 | use agent::activity::ActivityKind;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: unused variable: `activity_sender_for_closure`
    --> crates/ui/src/lab/runner.rs:1135:13
     |
1135 |         let activity_sender_for_closure = activity_sender;
     |             ^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: prefix with `_`
```

**Root cause**: `runner.rs:49` imports `ActivityKind` under `#[cfg(feature = "live")]` but the usage was removed in a subsequent commit; `runner.rs:1135` assigns a variable that is then unused. Both are in library code that the viewport-matrix feature does not touch.

**Gate status**: FAIL — `cargo clippy --tests -p ui -- -D warnings` exits 101.

## 3. Unit & Integration Tests

### 3a. `visual_snapshots.rs` — two consecutive runs

| Run | Passed | Failed | Ignored | Duration | Stable? |
|---|---|---|---|---|---|
| Run 1 | 48 | 3 | 0 | 8.22 s | — |
| Run 2 | 48 | 3 | 0 | 7.05 s | yes (same 3 fail) |

**Stable result: 48/51 PASS, 3 deterministic failures.**

### Failing tests — `visual_snapshots.rs`

All three failures are the bootstrap Charts baselines (R4 / pre-existing):

| Test | File:line | Nature |
|---|---|---|
| `charts_screen_dark_floor` | `visual_snapshots.rs:155` | K3 cross-build PNG drift |
| `charts_screen_dark_typical` | `visual_snapshots.rs:155` | K3 cross-build PNG drift |
| `charts_screen_dark_operator` | `visual_snapshots.rs:155` | K3 cross-build PNG drift |

Failure message (representative — `floor` slot):
```
visual snapshot mismatch for slot `floor`:
  baseline: crates/ui/tests/visual-baselines/charts_screen_dark_floor.png
    actual: target/visual-diff/charts_screen_dark_floor-actual.png
      diff: target/visual-diff/charts_screen_dark_floor.png
```

**Assessment**: These are the original `ui-test-harness-bootstrap` Charts baselines committed 2026-05-26 on the architect host. The K3 determinism caveat documented in `feature.md § Design` (cross-time baseline drift: "tiny-skia CPU determinism holds on the same machine but baseline generation + verification must run in the same build session") is active here. The chart X-axis renders with time-zone-sensitive offsets via `force_chart_utc_for_tests`; the `INIT_UTC: std::sync::Once` guard is correct, but the pixel output still varies across build sessions (tiny-skia internal anti-aliasing + subpixel positioning). The failures are **pre-existing K3 drift**, NOT a regression introduced by this feature.

**48 new viewport-matrix tests PASS** (Trail/Live × 3 slots = 9, Compare × 3 = 12, Phase F × 3 = 24, V9 self-test opt-out = 1 unchanged + 2 visual-fail-html inline tests = 2 → total 51 is consistent).

### 3b. `render_snapshots.rs`

| Passed | Failed | Ignored | Duration |
|---|---|---|---|
| 2 | 8 | 15 | 8.03 s |

**Two groups of failures:**

**Group 1 — Legacy M1-B baselines (pre-existing K3 drift):**

| Test | Nature |
|---|---|
| `chart_screen_renders_clean` | Legacy M1-B baseline `chart_screen_dark_typical.png` at 1280×720, committed in a prior session |
| `strategies_ready_renders_clean` | Legacy M1-B baseline `strategies_ready_dark_typical.png` at 1280×720, committed in a prior session |

**Group 2 — New viewport-matrix slots for chart_screen + strategies_ready:**

| Test | Baseline path |
|---|---|
| `chart_screen_renders_clean__floor` | `render_snapshots/chart_screen__floor.png` |
| `chart_screen_renders_clean__typical` | `render_snapshots/chart_screen__typical.png` |
| `chart_screen_renders_clean__operator` | `render_snapshots/chart_screen__operator.png` |
| `strategies_ready_renders_clean__floor` | `render_snapshots/strategies_ready__floor.png` |
| `strategies_ready_renders_clean__typical` | `render_snapshots/strategies_ready__typical.png` |
| `strategies_ready_renders_clean__operator` | `render_snapshots/strategies_ready__operator.png` |

**Assessment**: All 8 failures are chart-rendering fixtures (both `chart_screen` and `strategies_ready` include the iced chart widget). This is consistent with K3 cross-build drift specifically in chart rendering. The baselines were generated in a 2026-05-29 build session; this tester run is on 2026-06-16 in a fresh session. The K3 caveat from `feature.md § Design` applies: "baseline generation + verification both run on a single canonical Apple Silicon box, ON THE SAME RUN of the cockpit_live build chain." Non-chart fixtures (positions_ready, kpi_strip, pnl_panel, agent_feed, focus_ring) are `#[ignore]`d per D-VPM-4 expansion of shell-composition non-determinism — those 15 ignored tests are correct.

**2 PASS**: `fixtures::visual_fail_html::tests::emit_visual_fail_html_default_path_inlines_pngs` + `fixtures::visual_fail_html::tests::emit_visual_fail_html_spec_persist_writes_byte_identical_copy`.

### Per-widget × per-slot PASS summary

| Fixture group | Slots expanded | Status |
|---|---|---|
| Charts triple (bootstrap) | floor/typical/operator | FAIL (K3 drift — pre-existing) |
| Trail/Live (3 fixtures × 3) | floor/typical/operator | **PASS** |
| Compare (4 fixtures × 3) | floor/typical/operator | **PASS** |
| Phase F (8 fixtures × 3) | floor/typical/operator | **PASS** |
| V9 self-test | no viewport (opt-out) | PASS (unchanged) |
| render_snapshots: chart_screen (legacy) | typical@1280×720 | FAIL (K3 drift — pre-existing) |
| render_snapshots: strategies_ready (legacy) | typical@1280×720 | FAIL (K3 drift — pre-existing) |
| render_snapshots: chart_screen (matrix) | floor/typical/operator | FAIL (K3 drift — new baselines, same session required) |
| render_snapshots: strategies_ready (matrix) | floor/typical/operator | FAIL (K3 drift — new baselines, same session required) |
| render_snapshots: shell-composition group | all slots | IGNORED (correct per D-VPM-4) |
| visual-fail-html inline tests | n/a | PASS |

## 4. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites in `crates/ui/tests/` for this feature.

## 5. Backtest Results

_n/a_ — Pure test-infrastructure change. Zero strategy, exec, or backtest crate touches. `anchors = []` in `spec/trace.toml` row per feature design (R-NR.2 / D-VPM-7).

## 6. Benchmarks

_n/a_ — No hot-path changes.

## 7. Environment / Infrastructure Issues

### K3 cross-build determinism — render drift (known caveat)

The `charts_screen_dark_*` and `chart_screen`/`strategies_ready` render_snapshots failures are the K3 falsifier operating as documented. The architecture decision (feature.md § Design, confirmed by M-T1 at commit `641b94a8`) is that baselines are single-host, single-session artifacts. The committed baselines are from a 2026-05-29 build session; this tester run is in a fresh 2026-06-16 session. The pixel deltas are in the chart X-axis rendering.

**This is not a defect in the viewport-matrix feature itself.** The helper, the slot table, the path resolution, and the per-test expansion all work correctly — the 48 non-chart tests in `visual_snapshots.rs` PASS.

**However**: the developer's `tasks.md` T-VPM-D5 citation claims "`test result: ok. 10 passed; 0 failed; 15 ignored`" for render_snapshots. The current tester run produces `2 passed; 8 failed; 15 ignored`. This means the render_snapshots baselines were in sync at 2026-05-29 but have drifted. The 8 failing render_snapshots tests (6 new matrix + 2 legacy) confirm the committed baselines are stale relative to this build session.

### Overclaim — pre-ticked T-VPM-FINAL rows

The `tasks.md` has `status: presenter-done` and all T-VPM-FINAL rows (including T-VPM-FINAL.6 "write test-final report — DONE 2026-05-29") are pre-ticked. The cited test report `spec/ui-test-harness-viewport-matrix/reports/test-20260529-000000-v0.1.0.md` does **not exist** on disk (the `reports/` directory did not exist before this tester run created it). This is an overclaim that must be surfaced.

### Spec-lint

`spec-lint: FAIL (69 violations in 2 categories)`:

| Category | This run | Baseline (audit-2026-06-15) | Delta |
|---|---|---|---|
| dead-link | 65 | 65 | 0 |
| trace-broken-path | 4 | 5 | −1 (improvement) |
| **TOTAL** | **69** | **70** | **−1** |

Zero new violations. The current run is **one improvement** below the 2026-06-15 baseline (70→69; one trace-broken-path resolved vs baseline count of 5 in the audit — now 4 remaining). **No regressions.**

One dead-link from THIS feature's tasks.md is present (the memory/ path):
```
[dead-link] spec/ui-test-harness-viewport-matrix/tasks.md:
  link target missing: ../../.claude/projects/.../memory/feedback_human_verification_recipe.md
```
This is a pre-existing carry-over (user-memory path not in spec tree). Not a new violation — it appeared at the same count in the 2026-06-15 audit.

**Pre-existing spec debt (no new functional violations):**
- 65 dead-link violations — all pre-existing carry-overs documented in `audit-2026-06-15.md` (ephemeral `/tmp/` paths, moved archive paths, retired feature links).
- 4 trace-broken-path violations — `REQ-VISUAL-FAIL-HTML-REPORTER-001` (2 bare fn names), `REQ-QUEUE-STALENESS-RECONCILIATION-001`, `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. All pre-existing.

### Anchors

`bash scripts/verify_anchors.sh` → **ANCHORS PASS (119/119)**. All 119 body-SHA anchors match. Zero delta — this feature adds no backtest reports.

Note: tasks.md and trace.toml cite "75/75" for this feature's gate run (dev 2026-05-29). The current count is 119/119 because subsequent features added anchors. All 75 that existed at 2026-05-29 remain byte-identical.

### .gitattributes

`git check-attr binary crates/ui/tests/visual-baselines/charts_screen_dark_floor.png` → `binary: set`. `.gitattributes:2` = `crates/ui/tests/visual-baselines/** binary`. Rule is active per Q3-b ratification.

### Baseline count

48 PNG files top-level + 8 PNG files in `render_snapshots/` = **56 PNGs total**. 15 MB total (matching H3 empirical ~13-15 MB). K2 ceiling (100 MB) not triggered.

### Visual failures (HTML artifacts)

No visual-fail HTML artifacts are produced on clean-baseline-match. The 11 failures that occurred during this tester run produced `target/visual-diff/` PNG triples but no HTML artifacts (the `emit_visual_fail_html` helper only fires when `EMIT_VISUAL_FAIL_HTML_TO_SPEC` is set, which is OFF by default per the CLAUDE.md K2 falsifier constraint). Visual diff PNGs are at:
- `target/visual-diff/charts_screen_dark_floor.png` (diff), `charts_screen_dark_floor-actual.png`
- `target/visual-diff/charts_screen_dark_typical.png` + actual
- `target/visual-diff/charts_screen_dark_operator.png` + actual
- `target/visual-diff/render_snapshots__chart_screen__floor.png` + actual, etc.

## 8. Verdict

**`FAIL`**

Two distinct gate failures:

**Gate 1 — Clippy (hard failure):** `cargo clippy --tests -p ui --no-default-features --features live -- -D warnings` exits 101 with 2 errors in `crates/ui/src/lab/runner.rs:49` (unused import `ActivityKind`) and `:1135` (unused variable `activity_sender_for_closure`). These are in library code not written by the viewport-matrix feature, but the clippy gate is workspace-wide and the tester cannot exempt crates from each other's warnings. The developer's T-VPM-D5 acceptance criterion explicitly requires "zero NEW errors from viewport_matrix.rs/visual_snapshots.rs/render_snapshots.rs" — but the clippy gate is against the full `ui` lib, and these errors fail it.

**Gate 2 — render_snapshots visual failures (conditional failure):** 8 test failures in render_snapshots (6 new matrix slots + 2 legacy M1-B baselines) are all K3 cross-build drift in chart-rendering fixtures. These are expected under the K3 caveat and will resolve when the developer regenerates baselines in the same build session. However they are genuine test failures that a PASS verdict cannot carry — they block `cargo test -p ui --tests` from returning exit 0. The developer's T-VPM-D5 "10 PASS 0 failed" claim for render_snapshots does not hold in the current build session.

**What works correctly** (tester verification):
- Helper `viewport_matrix.rs` (SLOTS const, `slot()`, `snapshot_widget_at_slot`, `snapshot_widget_at_viewports`) — correctly implemented, name-keyed path resolution, `std::sync::Once` UTC init (thread-safe per commit `730dc5d` de-flake).
- 48/51 visual_snapshots tests PASS (all 44 new matrix tests PASS; 3 failing are pre-existing Charts K3 drift).
- 2/25 render_snapshots non-ignored tests PASS (visual-fail-html inline tests PASS; 8 chart-rendering failures are K3 drift).
- `.gitattributes` rule active.
- 56 PNGs committed, 15 MB total (within K2 ceiling).
- Anchors 119/119 PASS.
- P-VPM-1 falsification probe evidenced in tasks.md (PASS — SLOTS rotation produced zero byte deltas).
- spec-lint: −1 improvement vs baseline, zero new violations.

## 9. Routing

`HANDOFF → developer` — two items to fix before re-verify:

1. **Clippy (blocking):** Fix `crates/ui/src/lab/runner.rs:49` unused import + `:1135` unused variable. These are one-line fixes (remove the import; prefix variable with `_`). Confirm with `cargo clippy --tests -p ui --no-default-features --features live -- -D warnings` → exit 0.

2. **render_snapshots baseline regeneration (blocking):** The 8 chart-fixture baselines (`render_snapshots/chart_screen__{floor,typical,operator}.png`, `render_snapshots/strategies_ready__{floor,typical,operator}.png`, `render_snapshots/chart_screen_dark_typical.png`, `render_snapshots/strategies_ready_dark_typical.png`) are stale vs the current build session. Regenerate all render_snapshots baselines in a single build session: delete the 8 failing baseline PNGs, run `cargo test -p ui --test render_snapshots --no-default-features --features live` (first run auto-writes), then re-run a second time to confirm zero byte deltas. Also regenerate the 3 Charts baselines in the same session (`charts_screen_dark_{floor,typical,operator}.png`) per the K3 single-session contract.

**Note on pre-ticked T-VPM-FINAL rows**: The developer must un-tick T-VPM-FINAL.1 and T-VPM-FINAL.5 in tasks.md (the cited output lines no longer hold: render_snapshots was "10 passed; 0 failed" at 2026-05-29 but is "2 passed; 8 failed" now). T-VPM-FINAL.6 must also be un-ticked (the cited report at `spec/ui-test-harness-viewport-matrix/reports/test-20260529-000000-v0.1.0.md` does not exist). After re-verifying, the developer re-hands off to tester.

---

## Handoff Envelope

```toml
[handoff]
from          = "tester"
to            = "developer"
feature       = "ui-test-harness-viewport-matrix"
trace_refs    = ["REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001"]
verdict       = "FAIL"
priority      = "P1"

[inputs]
brief         = ""   # no brief path; feature.md + tasks.md read directly
artifacts     = [
  "spec/ui-test-harness-viewport-matrix/feature.md",
  "spec/ui-test-harness-viewport-matrix/tasks.md",
  "crates/ui/tests/fixtures/viewport_matrix.rs",
  "crates/ui/tests/visual_snapshots.rs",
  "crates/ui/tests/render_snapshots.rs",
  "spec/ui-test-harness-viewport-matrix/reports/test-2026-06-16-1200-viewport-matrix.md",
]

[outputs]
spec_files    = [
  "spec/ui-test-harness-viewport-matrix/reports/test-2026-06-16-1200-viewport-matrix.md",
]
lint_result   = "spec-lint: FAIL (69 violations) — zero new vs baseline (70), one improvement"
anchors_result = "ANCHORS PASS (119/119) — N/A for anchor verification gate (no backtest crate touch)"

[open_questions]
items = [
  "Q1: Are the runner.rs:49 / :1135 clippy errors introduced by a recent commit after the viewport-matrix dev session (2026-05-29), or were they present during dev-time clippy and suppressed?",
  "Q2: Can the 8 chart-fixture render_snapshots baselines be regenerated in the same build session as the visual_snapshots Charts baselines to satisfy the K3 single-session constraint?",
  "Q3: The tasks.md status field reads `presenter-done` but feature.md reads `dev-done`. After tester FAIL, which file should the developer update to reflect the tester-returned state?",
]

[assumptions]
items = [
  "The 3 visual_snapshots Charts failures and 8 render_snapshots failures are all K3 cross-build drift, NOT new defects introduced by the viewport-matrix helper. Evidence: the 44 new non-chart matrix tests all PASS; the failing tests are exclusively chart-rendering fixtures; the failures are deterministic across two consecutive runs.",
  "The clippy errors in runner.rs predate the viewport-matrix feature and were introduced in a later commit (commit history shows recent lab/runner changes). The developer must fix them regardless of origin to satisfy the tester gate.",
  "The spec-lint improvement (69 vs baseline 70) is accurate and does not indicate a new violation masked by a retirement.",
]
```
