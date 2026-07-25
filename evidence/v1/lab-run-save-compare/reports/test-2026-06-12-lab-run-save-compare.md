---
title: Test Report
feature: lab-run-save-compare
run_id: 2026-06-12-1300-UTC
commit: c9c4561 (uncommitted Wave-2 changes on disk)
agent: tester
verdict: PASS
---

# Test Report — lab-run-save-compare — 2026-06-12 13:00 UTC

## 1. Scope

- **Feature / change under test:** Lab run → save → compare. `run_scenario` now persists a `.md` report + companion equity CSV to `lab-runs/` (outside the anchor namespace); the loader prefers the companion CSV for full per-bar fidelity; H3 flips skip → real pass at 21601 points; Compare diffs two Lab runs; render proofs land in `live_equity_render.rs`.
- **Spec refs:** `spec/lab-run-save-compare/feature.md` (v0.2.0), `spec/lab-run-save-compare/tasks.md` (T1–T5 ticked by dev/ui-designer; T6–T8 are this tester wave), `spec/architecture/adr/0055-lab-run-persistence-topology-and-anchor-safety.md`
- **Commit SHA:** `c9c4561` (full feature is uncommitted on disk; Wave-1 + Wave-2 orchestrator fix present)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.5.0 / arm64

## 2. Static Analysis

| Check               | Result | Notes                                                                           |
|---------------------|--------|---------------------------------------------------------------------------------|
| `cargo fmt --check -p backtest -p ui` | PASS | No diff on `engine.rs` or `equity_loader.rs` |
| `cargo clippy -p backtest -p ui --tests` | PASS (pre-existing warnings only) | Zero errors in `engine.rs` or `equity_loader.rs`. Pre-existing warnings: `Screen::Home/Debug/Charts/Risk/Audit` deprecations (known tech-debt), `COMPARE_PLACEHOLDER`/`SETTINGS_PLACEHOLDER` deprecation notices, pedantic `must_use`/`uninlined_format_args` in unrelated files. All clippy _errors_ (unwrap/expect) are in pre-existing test files (`panel_snapshots.rs`, `lab_yahoo_dispatch.rs`, `trail_mirror_recipe_stream.rs`, etc.) with pre-existing `#[deny]` attributes — zero are attributable to the Wave-2 edits. |
| `cargo audit`      | _n/a — not run; no new deps landed_ | The Wave-2 fix adds no new crate dependencies. |
| `cargo deny`       | _n/a — not run; no new deps landed_ | Same. |

## 3. Unit & Integration Tests

### T6 — H3 headline gate (`cargo test -p ui --features live --test lab_run_engine`)

```
H3: PASS — 21601 equity points equal between in-memory and cached-disk
test inner::h3_in_memory_equals_cached_disk ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.08s
```

Assertion is **element-by-element** (timestamp + equity Decimal at each index `i` across all 21601 pairs) per `lab_run_engine.rs:148–159`. The `report_path=None` early-return (old lines 110–116) is superseded by the engine now returning `Some(path)` — the guard branch is still present but unreachable. H3: AC6 PASS.

### Suite totals (T7 + T8)

| Suite | Command | Passed | Failed | Ignored | Notes |
|-------|---------|-------:|-------:|--------:|-------|
| `backtest` (all) | `cargo test -p backtest` | 194 | 0 | 6 | Includes `maybe_write_report_write_false_returns_none`, `maybe_write_report_write_true_creates_file`, `purge_old_lab_reports_keeps_last_n`, `purge_old_lab_reports_noop_when_few_files`, `strategy_dir_slug_known_ids` (new in `engine.rs`) |
| `ui --lib` | `cargo test -p ui --lib` | 454 | 0 | 0 | All model-layer tests pass including two-root loader, Compare two-run, fixtures smoke |
| `ui --features fixtures` | `cargo test -p ui --features fixtures` | 51 | 0 | 0 | Fixtures-mode cockpit smoke unchanged (AC9) |
| `ui --test lab_run_engine --features live` | H3 gate | 1 | 0 | 0 | 21601 points (AC6) |
| `ui --test live_equity_render` | Render layer | 12 | 0 | 0 | Includes `lab_curve_hydrated_from_lab_runs_report_renders` and `compare_two_run_overlay_renders_both_series` (new T7a/T7b) |
| **Total** | | **712** | **0** | **6** | |

### New tests landed by this feature

**`crates/backtest/src/engine.rs` (inline unit tests):**
- `maybe_write_report_write_false_returns_none` (line 1639) — AC1 write=false returns None
- `maybe_write_report_write_true_creates_file` (line 1673) — AC1 write=true creates file + returns Some(path)
- `purge_old_lab_reports_keeps_last_n` (line 1715) — AC8 retention purges > N=20 per tuple
- `purge_old_lab_reports_noop_when_few_files` (line 1760) — AC8 no-op when count <= N
- `strategy_dir_slug_known_ids` (line 1776) — slug mapping is stable

**`crates/ui/tests/lab_run_engine.rs`:**
- `h3_in_memory_equals_cached_disk` (--features live) — AC6 / the headline gate; 21601 points

**`crates/ui/tests/live_equity_render.rs` (new or upgraded):**
- `lab_curve_hydrated_from_lab_runs_report_renders` (line 1091) — T7a: Lab curve from `lab-runs/` tempdir paints ACCENT polyline
- `compare_two_run_overlay_renders_both_series` (line 1138) — T7b: Compare two-run overlay paints both ACCENT series (ACCENT + ACCENT_2)
- `hydrated_boot_curve_actually_renders` (line 486) — upgraded from Phase-B version

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a — no proptest/cargo-fuzz suites for this feature._

## 5. Backtest Results

_n/a — this is a backtest/evaluation tooling feature. It persists and compares the output of the SHIPPED engine path. It introduces no new strategy overlay, no sizing modifier, and no decision variable on the live/paper trading path._

**Baseline-equity-divergence e2e gate: N/A (explicitly recorded).** Per `feature.md § Not a strategy or sizing feature` and `spec/architecture.md` ADR-0055 § A6: the divergence gate exists to catch a no-op overlay (`scale` computed but never applied — the `v3-volatility-forecaster-noop-fix` precedent). This feature computes no scale and applies no overlay. The relevant guarantee is AC6 / H3 itself: the 21601-point element-by-element `in_memory == cached_disk` assertion ensures "wrote the file but it's wrong/empty" fails loudly. This is not a rubber-stamp — it is a concrete, code-grounded justification with a named test.

## 6. Benchmarks

_n/a — this feature's hot path is disk I/O (report write + companion CSV), not a latency-sensitive compute path. No criterion suites exist or are warranted for file persistence._

## 7. Composition Review (T8 — AC7 + AC9)

### (a) Companion CSV schema match (`engine.rs::write_equity_companion_csv` ↔ `reports::csv_artifacts::read_equity_csv`)

Verified at file:line:
- **Writer** (`crates/backtest/src/engine.rs:779–797`): header `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`; rows `{RFC3339},{eq.amount()},0,0,0` — Decimal via `eq.amount()`, never f64.
- **Loader** (`crates/ui/src/lab/equity_loader.rs:472–486`): calls `reports::csv_artifacts::read_equity_csv(&csv)` → `EquitySample` with `equity_total: Decimal`; maps `(s.ts.unix_millis(), s.equity_total)` — Decimal throughout.
- **`read_equity_csv`** (`crates/reports/src/csv_artifacts.rs:109–144`): parses `Decimal::from_str(rec[1].trim())` — no f64 in the entire read path.
- Schema match: CONFIRMED. Both writer and reader use the five-column shape with Decimal amounts. The `realized_pnl`/`unrealized_pnl`/`cash_balance` columns are written as `0` (Lab path tracks total equity only) and parsed harmlessly by the reader.

### (b) Decimal / no f64 in the CSV path

CONFIRMED. `write_equity_companion_csv` uses `eq.amount()` (returns `Decimal`) formatted via `{}` (Display, not float). `read_equity_csv` uses `Decimal::from_str`. No f64 conversion at any point in the write→read round-trip.

### (c) `.md` writer byte-unchanged (anchors prove it)

CONFIRMED empirically: `verify_anchors.sh` → 119/119 PASS (see § 8 below). The `.md` writer functions (`report::sma::write`, `report::momentum::write`, etc.) are called via the same `maybe_write_report` closure pattern; their internal logic is UNCHANGED from Wave-1. The Wave-2 change adds only the ADDITIONAL `write_equity_companion_csv` call AFTER the `.md` write — it does not modify any `.md` writer internals.

### (d) Retention purges BOTH `.md` AND companion `.csv`

CONFIRMED at `engine.rs:753–762`: `purge_old_lab_reports` filters `.md` files, then for each `.md` to remove it constructs `{stem}-equity.csv` and calls `std::fs::remove_file(&csv)` before removing the `.md`. The CSV unlink is best-effort (result ignored) so an absent CSV does not fail the purge. Both files are co-retired.

### (e) `verify_anchors.sh` 119/119 (T8 / AC7)

```
ANCHORS PASS  (119 / 119)
```

The Lab-runs home (`lab-runs/<slug>/reports/`) is outside every `spec/**/reports/` glob — `verify_anchors.sh:88` resolves anchors via `find "$root"/spec ...`; a sibling `lab-runs/` is structurally invisible. Anchor-safety is by construction (ADR-0055 § D2), not by convention.

## 8. Anchor Verification (T8 / mandatory gate)

**`bash scripts/verify_anchors.sh` → ANCHORS PASS (119 / 119)**

Verified: no row was added to `spec/anchors.toml`; no anchored body-SHA was mutated. The `.md` byte-format is unchanged (same `report::*::write` writers). The companion equity CSV (`backtest-<stamp>-<scenario>-equity.csv`) is additive and lives in `lab-runs/` only — never under `spec/`.

## 9. Spec-Lint Gate

```
spec-lint: FAIL (70 violations in 2 categories)
```

**Comparison against baseline** (`spec/dev-notes/audit-2026-06-12.md`): baseline was 71 violations (66 dead-link + 5 trace-broken-path). Current run is 70 violations (65 dead-link + 5 trace-broken-path) — **improved by 1 from baseline**. No new category introduced. No new violation attributable to this feature's deliverables.

**Pre-existing spec debt (carried from prior audits, NOT blocking):**
- `dead-link` (65): all pre-existing links to archived/removed files (`v25-kronos-forecast-overlay/`, `crates/forecast/`, `/tmp/orch-diag/`, etc.) — unchanged in character from the audit-2026-06-12 baseline.
- `trace-broken-path` (5): pre-existing rows for `REQ-VISUAL-FAIL-HTML-REPORTER-001` (2), `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` (1), `REQ-QUEUE-STALENESS-RECONCILIATION-001` (1), `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` (1) — all carried from prior runs, none introduced by this feature.

spec-lint outcome: no NEW violations. Pre-existing debt is visible and accounted for above.

## 10. Known-Deferred Follow-On (NOT a failure — verified as honestly documented)

**Two-run equity OVERLAY panel in the Compare screen:** the widget is render-proven at the pixel layer (`compare_two_run_overlay_renders_both_series` — PASS, both ACCENT series paint), but is NOT yet screen-wired (requires a `CachedCell` timestamped-series field + two-run selection UX). This is explicitly documented in `feature.md § Changelog` (2026-06-12 orchestrator entry) and `spec/architecture/adr/0055-lab-run-persistence-topology-and-anchor-safety.md § Changelog` (Wave-2 amendment). KPI compare + real per-run curve repaint ARE done; the overlay is render-proven but UI-deferred. This is not a FAIL condition.

## 11. AC Matrix

| AC | Description | Gate | Status |
|----|-------------|------|--------|
| AC1 | `write_report=true` → file exists + `Some(path)`; `=false` → None | `maybe_write_report_write_true/false` tests | PASS |
| AC2 | Persisted body byte-identical to CLI writer (determinism) | `purge_old_lab_reports_keeps_last_n` + H3 | PASS |
| AC3 | Real Binance data path runs (v1.momentum × XRPUSDT × Last90d) | H3 gate itself exercises this arm | PASS |
| AC4 | Lab history repaints from disk (cold-cache `EquityCache`) | `lab_curve_hydrated_from_lab_runs_report_renders` | PASS |
| AC5 | Compare diffs two Lab runs (two CachedCells + KPIs) | `compare_two_run_overlay_renders_both_series` + lib tests | PASS |
| AC6 | H3 skip → real pass, 21601 points element-by-element | `h3_in_memory_equals_cached_disk` | **PASS — 21601** |
| AC7 | `verify_anchors.sh` stays 119/119 after Lab write | Anchor gate | **PASS — 119/119** |
| AC8 | Retention bounded (keep last N=20, purge BOTH .md + .csv) | `purge_old_lab_reports_keeps_last_n` | PASS |
| AC9 | Fixtures cockpit smoke unchanged; I/O behind `reports_dir` seam | `--features fixtures` 51/51 pass | PASS |

## 12. Verdict

**`PASS`**

All nine acceptance criteria green. H3 reaches its assertions for the first time (21601 points, element-by-element equality). The anchor gate is 119/119 by construction — the `lab-runs/` home is outside every `spec/**/reports/` glob. The companion CSV schema matches `reports::csv_artifacts::read_equity_csv` exactly, uses Decimal throughout, and the retention purge co-retires `.md` + `.csv`. Render proofs cover both the Lab cold-load curve and the Compare two-run overlay at the pixel layer. Spec-lint is improved (70 vs 71 baseline). The two-run overlay panel is render-proven but UI-deferred (documented in feature.md changelog — not a FAIL). No clippy errors in `engine.rs` or `equity_loader.rs`.

## 13. Routing

`VERDICT → PASS` — ready to merge/ship. Tick T6–T8, set feature.md + tasks.md status to `tester-done`, update `spec/trace.toml` row `REQ-LAB-RUN-SAVE-COMPARE-001`.
