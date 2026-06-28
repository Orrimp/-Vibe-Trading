---
title: Test Report — paper-mode-equity-wiring
feature: paper-mode-equity-wiring
run_id: 2026-06-12-0001-UTC
commit: abdb5dc7669d71b5e2b9c1dde208bcc0321b36df
agent: tester
verdict: PASS
---

# Test Report — paper-mode-equity-wiring — 2026-06-12

## 1. Scope

- **Feature / change under test:** Paper-mode equity wiring (ADR-0053) — unified `spawn_trading_loop(feed, …, equity_store: Option<…>, mode_label)` replaces the idle-reconciler stub; paper arm now runs the real per-bar pipeline against the live feed and persists via ADR-0052 rails; `drop(state_tx)` reconciler block deleted.
- **Spec refs:** `spec/paper-mode-equity-wiring/feature.md` (v0.2.0), `spec/paper-mode-equity-wiring/tasks.md`, `spec/architecture/adr/0053-unified-per-bar-trading-loop.md`
- **Commit SHA:** `abdb5dc7669d71b5e2b9c1dde208bcc0321b36df` (uncommitted working-tree; developer handoff with full implementation + staged spec files)
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** Darwin arm64 (Darwin 25.5.0)

## 2. Static Analysis

| Check               | Result | Notes |
|---------------------|--------|-------|
| `cargo fmt --check` | PASS   | Zero diff on all touched files (`runtime.rs`, `reconciler.rs`, `equity_store_integration.rs`, `paced_replay_late_subscriber.rs`, `live_equity_render.rs`). Known-unformatted `benches/chart_build_probe.rs` excluded per project norm. |
| `cargo clippy -p agent -p ui` | PASS (pre-existing warnings only) | 6 warnings in `bin/cockpit_live.rs` (collapsible-if), 10 warnings in `ui/src/lab/`, `ui/src/widgets/position_curve.rs`, `ui/src/live.rs` — all pre-existing, none in changed line ranges. Zero warnings attributable to the new lines in `runtime.rs`, `reconciler.rs`, or any test file. |
| `cargo audit`       | n/a    | Not run this cycle; no new dependencies added (AC8 confirmed — no new `Cargo.toml` deps). |

## 3. Unit & Integration Tests

### 3a. Full suite results

| Crate / suite | Passed | Failed | Ignored | Duration |
|---|---:|---:|---:|---:|
| `cargo test -p agent` (all suites) | all pass | 0 | 2 (doc-tests ignored) | ~2 s |
| `cargo test -p audit` | all pass | 0 | 1 (doc-test ignored) | ~0.5 s |
| `cargo test -p ui --lib` | 447 | 0 | 0 | 0.67 s |
| `cargo test -p ui --features live --lib` | 447 | 0 | 0 | 0.67 s |
| `cargo test -p ui --test live_equity_render` | 8 | 0 | 0 | 0.42 s |
| `cargo test -p ui --test panel_snapshots` | 103 | 0 | 0 | 0.30 s |
| **Total** | **all pass** | **0** | | |

### 3b. Named guard tests — individual results

| Test | Suite | Result |
|---|---|---|
| `paced_replay_late_subscriber_receives_fills_positions_pnl` | `crates/agent/tests/paced_replay_late_subscriber.rs` | PASS |
| `ac1_paper_mode_persists_one_row_per_bar` | `crates/agent/tests/equity_store_integration.rs` | PASS |
| `ac2_research_mode_writes_zero_rows` | `crates/agent/tests/equity_store_integration.rs` | PASS |
| `ac1_faked_store_tail_is_monotone` | `crates/agent/tests/equity_store_integration.rs` | PASS |
| `paper_loop_produces_moving_equity` | `crates/agent/tests/equity_store_integration.rs` | PASS |
| `paper_loop_equity_store_research_none_zero_rows` | `crates/agent/tests/equity_store_integration.rs` | PASS |
| `y_variation_gate_moving_passes_flat_fails` | `crates/ui/tests/live_equity_render.rs` | PASS |

### Failing tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — this feature is a wiring change to `crates/agent` (`runtime.rs`, `reconciler.rs`). The backtest binary never calls `runtime::run`; research replay equity math is unchanged. Anchors are byte-stable by structural independence (see AC7 / anchor gate below).

## 6. Benchmarks

_n/a_ — no hot-path changes; equity-curve and trading-loop are not criterion-benchmarked.

## 7. Builds

| Command | Result |
|---|---|
| `cargo build -p ui` | PASS (6.78 s) |
| `cargo build -p ui --features live` | PASS (0.77 s) |
| `cargo build -p ui --bin cockpit --features fixtures` | PASS (3.59 s) |

No widget changed. The fixtures-mode cockpit never runs the paper loop; it is byte-identical to pre-feature behavior (AC8 confirmed). No new external dependency in any `Cargo.toml`.

## 8. Q6 Divergence-gate ruling — headline

**Q6: INTENT SATISFIED — recorded as SATISFIED, explicitly NOT N/A.**

This is the single most important correctness statement in the feature. The `live-equity-history-durable` feature stamped N/A (genuine read-only, no decision variable). This feature introduces `registry.on_bar → risk::size_and_validate → PaperEngine::step` into paper mode — the identical "computed-but-never-applied" class as the `v3-volatility-forecaster-noop-fix` precedent (CLAUDE.md non-negotiable). The gate intent applies and is satisfied by two halves:

### Data-layer half (AC1 — `paper_loop_produces_moving_equity`)

**Test:** `crates/agent/tests/equity_store_integration.rs::paper_loop_produces_moving_equity` (line 214).

**Assertion semantics:**
- Drives 80 closed bars with a MOVING price series through `spawn_trading_loop` with `Some(store)` (paper mode).
- Asserts fills reached `bus.fills()` (AC5) — `fill_count > 0`.
- Asserts `total_equity` values across persisted rows are **NOT all equal** (assertion at line 298: `assert!(!all_equal, …)`).
- Asserts none equal `initial_capital` (line 305: `assert!(!all_initial, …)`).
- Asserts row count == bar count (line 313: `assert_eq!(rows.len(), bar_count, …)` — one writer, no double-mint, AC2).

**Would a constant equity series fail it?** YES — the `all_equal` check at line 297 (`equities.iter().all(|&e| e == first_equity)`) would be `true`, and `assert!(!all_equal)` would panic. The pre-feature bug (`drop(state_tx)` → equity = `initial_capital` forever) would produce exactly this failure. The test is a genuine sentinel.

### Render-layer half (AC6 — `y_variation_gate_moving_passes_flat_fails`)

**Test:** `crates/ui/tests/live_equity_render.rs::y_variation_gate_moving_passes_flat_fails` (line 655), constant `CURVE_Y_VAR_MIN = 30` (line 635).

**Assertion semantics — non-flat half:**
- Feeds `session_points()` (rises + dips) through the production `update` path.
- Asserts ACCENT bounding-box height `(max_y - min_y) ≥ CURVE_Y_VAR_MIN = 30` (line 669).
- Also asserts `count ≥ CURVE_DREW_MIN_ACCENT` and `x_span ≥ CURVE_X_SPAN_MIN` (belt-and-braces).

**Assertion semantics — flat contrast half (self-proving proof):**
- Feeds 8 points, all at `100_000` USDT (constant — the pre-feature bug).
- Asserts `flat_y_span < CURVE_Y_VAR_MIN = 30` (line 706). The `equity_curve.rs:178` flat-line guard renders the degenerate series as a **centered, full-width horizontal line** (~1-2 px bbox height) — PASSES `count ≥ 200` and `x_span ≥ 400` (confirmed by the test at lines 717-727), but FAILS Y-variation.
- Final relational assertion: `moving_y_span > flat_y_span` (line 731).

**The gate genuinely discriminates:** a regression back to constant equity (the bug) would fail the non-flat half's Y-variation check. Y-variation is the ONLY valid discriminator because the flat-line guard draws a full-width horizontal line that passes both existing `count` and `x_span` thresholds — this is explicitly proven by the flat-contrast half.

## 9. One-writer discipline — structural inspection

**AC4 — `drop(state_tx)` stub deleted (zero hits in paper arm):**

```
grep -n "drop(state_tx)" crates/agent/src/runtime.rs
```
Output: `651: // The idle-reconciler \`drop(state_tx)\` stub (runtime.rs:674) is`

The only occurrence is a **comment** referencing the deleted stub. Zero live `drop(state_tx)` calls remain in the paper arm. The idle-reconciler block (`runtime.rs:655-683`) is deleted and replaced by the `spawn_trading_loop` call (AC4 confirmed).

**`build_snapshot_row` visibility:**

`crates/agent/src/reconciler.rs:256`: `pub(crate) fn build_snapshot_row(snap: &PnlSnapshot, mode: &str) -> audit::EquitySnapshotRow`

`pub(crate)`, not `pub` — minimal exposure (one-writer discipline). The loop (same crate) reuses it without duplication.

**No `if mode != Research` inside the loop body:**

Inspected `crates/agent/src/runtime.rs` lines 937–1215 (the `spawn_trading_loop` body). The only mode-conditional is the `if let Some(ref store) = equity_store` branch at line 1190 — the `Some`/`None` is the gate at the CALLER (research passes `None`, paper passes `Some`). There is no `if mode != Research` or equivalent inside the loop body. The gate is structurally at the call sites.

**Two call sites — file:line citations:**

1. **Research arm** — `crates/agent/src/runtime.rs:527–539`:
   ```rust
   spawn_trading_loop(
       Arc::clone(&feed),
       …
       None, // Research mode: equity_store = None (A2 gate — never write in research)
       "research",
       …
   );
   ```

2. **Paper arm** — `crates/agent/src/runtime.rs:662–674`:
   ```rust
   spawn_trading_loop(
       paper_feed,
       …
       equity_store, // Some(store) — the loop-direct persist gate (ADR-0053 D2)
       "paper",
       …
   );
   ```

## 10. AC7 Anchor verification — explicit count

`bash scripts/verify_anchors.sh` output: **`ANCHORS PASS  (119 / 119)`**

All 119 anchors byte-identical. The backtest binary never calls `runtime::run` — anchors are structurally independent of the `spawn_trading_loop` rename by construction. Research byte-stability proven: `paced_replay_late_subscriber_receives_fills_positions_pnl` PASS, `paper_loop_equity_store_research_none_zero_rows` PASS (research with `None` → zero persisted rows), existing research integration tests PASS.

## 11. Spec-lint gate

`python3 scripts/spec_lint.py`: **`spec-lint: FAIL (71 violations in 2 categories)`**

| Category | Current | Baseline (audit-2026-06-08) | Delta |
|---|---:|---:|---:|
| dead-link | 66 | 87 | -21 |
| trace-broken-path | 5 | 7 | -2 |
| **TOTAL** | **71** | **94** | **-23** |

Counts DECREASED relative to both the `audit-2026-06-08` baseline and the prior `live-equity-history-durable` tester report (which already recorded 71 violations as the established working baseline). No new category introduced; no counts grew. Per tester protocol: pre-existing violations do not block PASS; only regressions (growing counts) block.

## 12. Pre-existing spec debt (carried from audit-2026-06-08)

All 71 violations are pre-existing, unrelated to this feature:
- **dead-link (66):** stale links in archived/historical feature specs, ADRs referencing cleaned-up paths (`v25-kronos`, `crates/forecast/src/bin`, `/tmp/orch-diag` screenshots, `v1-5b-multi-venue` report, etc.).
- **trace-broken-path (5):** `REQ-LAB-YAHOO-REALDATA-V0-1-4-001`, `REQ-VISUAL-FAIL-HTML-REPORTER-001` (2 paths), `REQ-QUEUE-STALENESS-RECONCILIATION-001`, `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001`. None are in `paper-mode-equity-wiring`.

## 13. Baseline-equity-divergence non-negotiable (CLAUDE.md)

**Intent applies. Satisfied by AC1 + AC6.** Explicit statement per CLAUDE.md and the A6 architect ruling:

This is the first feature since the `v3-volatility-forecaster-noop-fix` 2026-05-22 precedent where the gate APPLIES rather than N/A. The feature introduces a strategy + sizing + execution decision into paper mode (`registry.on_bar → risk::size_and_validate → PaperEngine::step`). The pre-feature bug (`state_tx` dropped → equity = `initial_capital` forever) is the exact "computed but never applied" pattern. The gate is satisfied:

- **AC1 (data-layer):** `paper_loop_produces_moving_equity` asserts `total_equity` values are NOT all equal and NOT all equal to `initial_capital`.
- **AC6 (render-layer):** `y_variation_gate_moving_passes_flat_fails` asserts ACCENT bounding-box height ≥ 30 px for a moving curve AND < 30 px for a flat curve, with a self-proving contrast.

Not rubber-stamped N/A. Both halves PASS.

## 14. Verification matrix per AC

| AC | Description | Test(s) | Result |
|---|---|---|---|
| AC1 | Paper loop produces moving equity (data-layer divergence) | `paper_loop_produces_moving_equity` (equity_store_integration.rs:214) | PASS |
| AC2 | One writer, one series (row count == bar count) | `paper_loop_produces_moving_equity` AC2 assertion + `paper_loop_equity_store_research_none_zero_rows` | PASS |
| AC3 | No real orders (structural — PaperEngine only) | `paper_loop_produces_moving_equity` mode label assertion; structural: `spawn_trading_loop` constructs no exchange client | PASS |
| AC4 | `drop(state_tx)` stub deleted | Structural grep: zero live occurrences in paper arm; loop-direct persist confirmed at runtime.rs:1190 | PASS |
| AC5 | Fills + positions reach bus | `paper_loop_produces_moving_equity` fill_count/pos_count assertions (lines 272, 281) | PASS |
| AC6 | Non-flat curve renders (Y-variation gate) | `y_variation_gate_moving_passes_flat_fails` (live_equity_render.rs:655), `CURVE_Y_VAR_MIN = 30` | PASS |
| AC7 | Research + backtests byte-unchanged | `paced_replay_late_subscriber_receives_fills_positions_pnl`, `paper_loop_equity_store_research_none_zero_rows`, `verify_anchors.sh` → 119/119 | PASS |
| AC8 | Fixtures smoke + no-live-feature build unchanged | `cargo build -p ui`, `cargo build -p ui --features live`, `cargo build -p ui --bin cockpit --features fixtures` all PASS; no new deps | PASS |

## 15. Known pre-existing reds (noted, not blocking)

- **Backtest montecarlo flakes:** not encountered this run. The 2 known flakes pass on re-run (not relevant to this feature).
- **`lab_run_engine::h3`:** not encountered; was fixed at `2c4a59f`. All lab tests in `cargo test -p ui --lib` pass cleanly (447/447).

## 16. Environment / Infrastructure Issues

_none_ — all suites deterministic; no flakes observed.

## 17. Verdict

**`VERDICT → PASS`**

All 8 `live_equity_render` tests pass. All 5 `equity_store_integration` tests pass. All named guard tests pass individually. 119/119 anchors byte-identical. Builds pass (no `live` feature, with `live`, with `fixtures`). No warnings in changed line ranges. The Q6 divergence gate is genuinely discriminating: `paper_loop_produces_moving_equity` would fail on a constant equity series; `y_variation_gate_moving_passes_flat_fails` would fail on a flat-to-moving regression. Pre-existing spec-lint violations are at or below the established baseline (71 ≤ 94). The CLAUDE.md baseline-equity-divergence non-negotiable is satisfied by AC1 + AC6 — not rubber-stamped N/A.

## 18. Routing

`VERDICT → PASS` — ready to merge/ship; routes to presenter per AGENT.md.

---

```toml
[handoff]
from     = "tester"
to       = "presenter"
feature  = "paper-mode-equity-wiring"
trace_refs = ["REQ-LIVE-EQUITY-PAPER-001"]
verdict  = "PASS"
priority = "normal"

[inputs]
brief     = "/tmp/brief-paper-mode-equity-wiring.md"
artifacts = [
  "spec/paper-mode-equity-wiring/feature.md",
  "spec/paper-mode-equity-wiring/tasks.md",
  "spec/architecture/adr/0053-unified-per-bar-trading-loop.md",
  "crates/agent/src/runtime.rs",
  "crates/agent/src/reconciler.rs",
  "crates/agent/tests/equity_store_integration.rs",
  "crates/ui/tests/live_equity_render.rs",
]

[outputs]
spec_files = [
  "spec/paper-mode-equity-wiring/reports/test-2026-06-12-paper-mode-equity-wiring.md",
  "spec/paper-mode-equity-wiring/feature.md",
  "spec/paper-mode-equity-wiring/tasks.md",
  "spec/trace.toml",
]
adrs_added    = []
lint_result   = "spec-lint: FAIL (71 violations in 2 categories) — pre-existing baseline, counts decreased vs audit-2026-06-08 (94). No new category, no counts grew. Does not block PASS."
anchors_result = "ANCHORS PASS (119 / 119)"

[open_questions]
items = []

[assumptions]
items = [
  "Windowed cockpit-smoke (cargo run --bin cockpit_live) is operator-verified out-of-band per MEMORY.md Cockpit Live view parked note; build confirmed green.",
  "No new external dependency introduced (AC8 confirmed by build success + no Cargo.toml diff).",
  "CURVE_Y_VAR_MIN = 30 px is calibrated from diag_accent_bounding_box empirics (healthy session ~168 px, flat ~1-2 px); threshold is stable across themes and minor layout shifts.",
]
```
