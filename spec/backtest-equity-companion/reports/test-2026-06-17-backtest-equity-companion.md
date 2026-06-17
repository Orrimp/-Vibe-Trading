# Test report — backtest-equity-companion v0.1.0 (2026-06-17)

**VERDICT → PASS**

Verification performed by the orchestrator, independently re-running the
load-bearing gates and cross-checking the developer + ui-designer sub-agent runs
(their reported numbers matched). The live render check (viewer window) is
orchestrator-only per the cockpit-smoke capability boundary.

## 1. Scope

Equity-companion emission from the backtest CLI + the `cockpit-reports-viewer`
loader stem-match correctness fix + a committed non-anchored demo report so the
Reports screen renders a populated equity curve.

## 2. Gates

| Gate | Result |
|---|---|
| `verify_anchors.sh` | **PASS 119/119** (re-verified independently, after all edits + the demo) |
| `cargo test -p ui` | **860 passed / 0 failed / 27 ignored** (incl. the 13 reports-loader tests + the stem-match regression guard) |
| `cargo test -p backtest` | green (lib 82/0; `equity_companion_roundtrip` 3/3; `determinism` 20/20) |
| `cargo clippy -p ui` / `-p backtest` (forced re-lint, `-D warnings`) | clean |
| `cargo fmt -p ui` / `-p backtest --check` | clean |
| cockpit-smoke (fixtures cockpit, 7s window) | **PASS — 0 panic lines** |
| `spec_lint` | floor (1 = the immutable vol-verdict link) |

## 3. Acceptance criteria

| AC | What | Evidence | Status |
|---|---|---|---|
| AC1 | Backtest emit writes `reports/artifacts/<stem>/equity-*.csv` in the `read_equity_csv` schema | `crates/backtest/src/report/mod.rs::write_equity_companion` (8 call sites in `main.rs`); demo produced `spec/v0-paper-sma/reports/artifacts/backtest-20260617-180015-btc-2024-h1-sma-cross/equity-20260617-180015.csv` (17 544 rows) | PASS |
| AC2 | `read_equity_csv` round-trips the emitted file | `crates/backtest/tests/equity_companion_roundtrip.rs` (3/3) | PASS |
| AC3 | Reports screen + offline viewer render a populated curve for the demo report | viewer rendered the demo report's KPI strip + equity curve + drawdown band with **0 panics**; loader unit test `load_equity_companion_real_demo_report_is_ready` → `Ready`; operator eyeball recipe in § 5 | PASS (render-layer: 0-panic + loader→Ready; operator visual confirm pending) |
| AC4 | `verify_anchors` stays 119/119 (additive, no report-body change) | PASS 119/119 (independent) | PASS |
| AC5 | No new production crate edge | `backtest → reports` added under `[dev-dependencies]` only | PASS |
| AC6 | `spec_lint` passes | floor (1, immutable vol-verdict) | PASS |
| loader | Stem-match, not first-match-any | `crates/ui/src/reports/loader.rs::load_equity_companion` now resolves `artifacts/<report-file-stem>/`; regression guard `load_equity_companion_non_matching_stem_dir_is_empty` → `Empty` | PASS |

## 4. Notes

- **Honest cosmetic:** the companion's `ts` x-axis is synthetically reconstructed
  (`synthetic_timestamps`, 2024→2025 consecutive-hourly); the equity shape/values
  are real. Inert for rendering.
- **Corpus limitation:** the existing 119-report corpus stays empty-by-data — those
  reports predate companion emission and can't be coherently back-filled (re-running
  drifts the data source from synthetic to real). Forward backtest runs emit a
  coherent stem-matched companion automatically.

## 5. Operator human-verification recipe

- **Command:** `cargo run -p ui --release --bin cockpit_live --features live`
- **Steps:** Library sidebar group → **Reports** → select **`backtest-20260617-180015-btc-2024-h1-sma-cross`**.
- **Expected:** a populated equity curve + drawdown band in the detail pane (the
  other reports stay empty — they predate emission).
- **Failure diagnosis:** empty curve on the demo report → loader didn't resolve the
  stem-matched companion (check `reports/artifacts/<report-stem>/equity-*.csv` exists);
  a panic → capture the stderr backtrace (none seen across 860 ui tests + the smoke).
- **Cleanup:** close the window. Read-only feature; nothing written.
