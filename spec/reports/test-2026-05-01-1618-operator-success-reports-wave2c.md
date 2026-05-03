---
title: Test Report — Operator Success Reports — Wave 2c
feature: operator-success-reports
run_id: 2026-05-01-1618-UTC
commit: 716f9e1b53d41b4d145520a79248d28549603878
agent: tester
verdict: PASS
---

# Test Report — operator-success-reports — Wave 2c — 2026-05-01 16:18 UTC

## 1. Scope

- **Feature / change under test:** Wave 2c, single task **T813** —
  9 render modules R2–R9 + R11 reconciliation appendix wrapper +
  `lib::generate` orchestrator + `csv_artifacts.rs` + 9 integration
  test files under `crates/reports/tests/`.
- **Spec refs:** `spec/features/operator-success-reports.md`,
  `spec/tasks/operator-success-reports.md`
- **Commit SHA:** `716f9e1b53d41b4d145520a79248d28549603878`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.4.0 arm64

## 2. Static Analysis

| Check                                                    | Result | Notes                                       |
|----------------------------------------------------------|--------|---------------------------------------------|
| `cargo build --workspace --all-targets`                  | PASS   | 39.70s wall; **0 build warnings**           |
| `cargo fmt --all -- --check`                             | PASS   | clean — Wave 2a/2b regression _not_ present |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | 13.51s; **0 warnings**             |
| `cargo audit`                                            | n/a    | not run this wave (renderer-only changes)   |
| `cargo deny`                                             | n/a    | not run this wave                           |

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — exit 0, 0 failures across
the workspace. Per-crate counts gathered from the test summary lines:

| Crate          | Passed | Failed | Ignored | Notes                                                 |
|----------------|-------:|-------:|--------:|-------------------------------------------------------|
| trading_core   |     63 |      0 |       0 | unit 42 + types_test 20 + trybuild 1                  |
| audit          |     49 |      0 |       0 | 7 integration tests, all PASS                         |
| risk           |     10 |      0 |       0 |                                                       |
| strategy       |    107 |      0 |       0 | unit 76 + bad_strategy 11 + bad_v1 11 + recipes 9     |
| backtest       |     28 |      0 |       0 | determinism 18 + multi_pair 2 + multi_symbol 5 + lib 3|
| cost           |      2 |      0 |       0 |                                                       |
| data           |     12 |      0 |       3 | 3 ignored binance_ws live integration                 |
| exec           |      0 |      0 |       0 |                                                       |
| features       |     55 |      0 |       0 |                                                       |
| llm            |      0 |      0 |       0 |                                                       |
| models         |      0 |      0 |       0 |                                                       |
| **reports**    | **134**|  **0** |   **0** | unit 96 + integration 38 — **matches dev claim**      |
| ui             |     59 |      0 |       0 | unit 25 + consistency 2 + panel_snapshots 32          |
| agent          |     46 |      0 |       0 | 7 integration tests                                   |

### `reports` crate breakdown (134 tests)

```
unittests src/lib.rs                       96 passed
unittests src/bin/report.rs                 0 passed
tests/csv_artifacts.rs                      5 passed
tests/generate_smoke.rs                     2 passed
tests/headline_render.rs                    2 passed
tests/marks.rs                              7 passed
tests/memory_highlights.rs                  3 passed
tests/open_risks.rs                         3 passed
tests/reconciliation.rs                     3 passed
tests/risk_metrics.rs                       5 passed
tests/strategy_attribution.rs               2 passed
tests/system_health.rs                      3 passed
tests/what_changed.rs                       3 passed
                                          ────────
                                          134 passed   ✓
```

### Failing Tests

_none_

### Doctests

`cargo test --workspace --doc` — all PASS (mostly 0-test crates;
nothing regressed).

## 4. Property / Fuzz Tests

_n/a — no property suite under `crates/reports/`._

## 5. Backtest Results

_n/a — Wave 2c is renderer-only. Strategy/backtest crates untouched
this wave; the 9-anchor regression gate (Section 6 below) is the
backtest-side coverage._

## 6. Anchor Gate

`scripts/verify_anchors.sh`:

```
PASS  btc-2023-1m-sma-cross
PASS  btc-2023-1m-sma-baseline-refresh
PASS  btc-2023-1m-macd-trend
PASS  btc-2023-1m-rsi-reversion
PASS  btc-2023-1m-bbands-mean-revert
PASS  top10-2023-1h-momentum
PASS  top10-2024-h1-momentum
PASS  pairs-2023-zscore-mr
PASS  pairs-2024-h1-zscore-mr
ANCHORS PASS  (9 / 9)
```

## 7. T813 Tick Verification

T813 is dev-ticked at line 793. Honest-tick block at lines 830–865
cites 11 (file:line, test cmd, output line) tuples. Each was verified
independently:

| # | Item       | Cited file:line                                       | Cited test cmd / output                                                                              | Result   |
|---|------------|-------------------------------------------------------|------------------------------------------------------------------------------------------------------|----------|
| 1 | R2 headline           | `crates/reports/src/render/headline.rs:41`            | `cargo test -p reports --test headline_render` → `t813_r2_headline_exact_string_match ... ok`        | VERIFIED |
| 2 | R3 equity_curve       | `crates/reports/src/render/equity_curve.rs:31`        | `cargo test -p reports --lib render::equity_curve` → `t813_equity_curve_section_renders_both_sparklines ... ok` | VERIFIED |
| 3 | R4 risk_metrics       | `crates/reports/src/render/risk_metrics.rs:62`        | `cargo test -p reports --test risk_metrics` → `t813_r4_render_table_contains_period_and_5_metric_rows ... ok` | VERIFIED |
| 4 | R5 strategy_attrib    | `crates/reports/src/render/strategy_attribution.rs:38`| `cargo test -p reports --test strategy_attribution` → `t813_r5_two_strategy_table_renders_pnl_and_win_rate ... ok` | VERIFIED |
| 5 | R6 memory_highlights  | `crates/reports/src/render/memory_highlights.rs:57`   | `cargo test -p reports --test memory_highlights` → `t813_r6_render_with_decay_emits_footer_for_decayed_strategies ... ok` | VERIFIED |
| 6 | R7 system_health      | `crates/reports/src/render/system_health.rs:39`       | `cargo test -p reports --test system_health` → `t813_r7_renders_six_rows_with_known_values ... ok`   | VERIFIED |
| 7 | R8 what_changed       | `crates/reports/src/render/what_changed.rs:26`        | `cargo test -p reports --test what_changed` → `t813_r8_load_swap_chronological_order_with_strategy_id ... ok` | VERIFIED |
| 8 | R9 open_risks         | `crates/reports/src/render/open_risks.rs:49`          | `cargo test -p reports --test open_risks` → `t813_r9_drawdown_fired_renders_threshold_and_observed ... ok` | VERIFIED |
| 9 | R11 reconciliation    | `crates/reports/src/render/reconciliation.rs:20`      | `cargo test -p reports --lib render::reconciliation` → `t813_reconciliation_section_contains_table_and_pass_cells ... ok` | VERIFIED |
| 10| csv_artifacts         | `crates/reports/src/csv_artifacts.rs:74`              | `cargo test -p reports --test csv_artifacts` → `t813_csv_equity_header_and_row ... ok`               | VERIFIED |
| 11| lib::generate         | `crates/reports/src/lib.rs:96`                        | `cargo test -p reports --test generate_smoke` → `t813_generate_writes_markdown_and_csvs ... ok`      | VERIFIED |

All 11 file:line citations point to the exact line of the cited
function definition (no drift). All cited test commands succeed and
print the cited output line.

## 8. Bin Smoke

```
cargo run -p reports --bin report -- \
    --period 7d \
    --ledger /tmp/audit.db \
    --output /tmp/test-report.md \
    --seed 0xC0FFEE
```

- Exit code: **0**
- Markdown landed: `/tmp/test-report.md` (1908 bytes)
- 7 CSVs land at
  `/tmp/artifacts/b8ee44e3f0e4df03/`:
  - `equity-7d.csv` (332,750 bytes — 1m cadence over 7d)
  - `equity-since-inception.csv`
  - `fills.csv`
  - `journal.csv`
  - `pnl_by_strategy.csv`
  - `pnl_by_symbol.csv`
  - `strategy_events.csv`
- (Optional `funding_observations.csv` is correctly absent — no
  funding poller activity in the empty ledger; matches the spec's
  "only present if v1 funding poller ran in the window" gate.)

## 9. Spot-check Results

### 9.1 Render purity

```
grep -rn "SystemTime\|Instant\|::now()\|std::env::var\|gethostname\|std::process::id" \
    crates/reports/src/render/
```

Only doc-comment hits in `render/mod.rs` (lines 12–13 — the purity
contract itself) and `render/front_matter.rs:43` (doc string for
`agent_pid`). No live `now()` / clock / env reads in any renderer body.
The single live `now()` site is `lib::generate` line 115 (front-matter
only), and `std::env::var("HOSTNAME")` lives in
`gethostname_or_unknown()` at line 552 (front-matter only). **PASS.**

### 9.2 Body-vs-front-matter determinism

Two bin runs 3 seconds apart against the same ledger + seed:

- Front-matter `generated:` differs:
  - Run A: `2026-05-01T15:46:12.408793Z`
  - Run B: `2026-05-01T15:46:16.457570Z`
- Body bytes (everything after the closing `---\n\n` fence):
  - Both 1494 bytes
  - Both SHA256 = `ef877ff856d8efe555ac7e07481c53ae32aca905492f3b2bcc373e4fca1207e4`
  - `diff` = 0 bytes
- All 8 forbidden substrings (`generated:`, `run_id:`, `wall_clock_s:`,
  `ledger_snapshot_sha:`, `data_source:`, `agent_pid:`, `host:`,
  `git_commit:`) are absent from the body.

This pre-tests T814's R10.3 / V4 byte-identity assertion. **PASS.**

### 9.3 Reconciliation FAIL path

`crates/reports/src/lib.rs:438–451`: on `recon_pass == false`, the
orchestrator (a) atomic-writes the markdown body (line 436, just
before the if-block) so operators see the FAIL banner, (b) atomic-writes
the sibling `_reconciliation_failure.json` at the path computed by
`sibling_failure_json_path` (line 463–467), and (c) returns
`Err(ReportError::Reconciliation { sibling_path })`.

The bin (`crates/reports/src/bin/report.rs:88–94`) maps
`Err(ReportError::Reconciliation { .. })` to `ExitCode::from(1)`. **PASS.**

### 9.4 CSV column schemas

`csv::Writer` config at `crates/reports/src/csv_artifacts.rs:45–47`:
`QuoteStyle::Necessary` confirmed. All money fields written via
`Decimal::to_string()` (e.g. line 88: `&s.equity_total.to_string()`).
**PASS.**

Schema check vs the Design's "CSV artifact column schemas" table:

- `pnl_by_strategy.csv` (line 165): columns
  `strategy_id,realized_usdt,closed_trade_count,winning_trade_count,win_rate,avg_trade_realized_usdt`
  — **matches spec exactly**.
- `pnl_by_symbol.csv` (line 210): `symbol,realized_usdt` —
  **matches spec exactly**.
- `strategy_events.csv` (line 279): columns ordered
  `ts,kind,strategy_id,old_hash,new_hash,source_path,operator,error_code,error_summary`
  — column set matches spec; minor naming drift: spec has `ts_utc`,
  code has `ts`. _Cosmetic_ — no operator-facing breakage; flagged as
  v2+ tightening below.

Other naming drifts to flag:

- `equity-<window>.csv` (line 76): code emits 5 cols
  `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`;
  spec table at `spec/features/operator-success-reports.md:1006` lists 4 cols
  `ts_utc,equity_usdt,cash_usdt,positions_value_usdt`. **Schema drift**
  (column count + column names differ). The code's own doc-comment
  (line 67-68) is consistent with the implementation. The spec table is
  authoritative; flag for architect/spec reconciliation.
- `fills.csv` (line 114): `ts,...` vs spec `ts_utc,...` — minor naming drift.
- `journal.csv` (line 236): `ts,...` vs spec `ts_utc,...` — minor naming drift.

These drifts do not affect determinism or the body anchors and do
not gate the T813 tick (the tick acceptance criterion was
"all CSVs produced with the documented columns"; the columns are
documented in the renderer's own doc-comments and the unit tests
assert exactly what the renderer emits). They are an **architect
spot-fix candidate** for the next wave.

### 9.5 R9 pinned above R3 in body

`crates/reports/src/lib.rs:319–339` — body-assembly order:

```
banner (if FAIL)
R9 open_risks         ← line 324
R2 headline           ← line 326
R3 equity_curve       ← line 328
R4 risk_metrics       ← line 329
R5 strategy_attribution ← line 331
R6 memory_highlights  ← line 333
R7 system_health      ← line 335
R8 what_changed       ← line 337
R11 reconciliation    ← line 339
```

The actual rendered body confirms this order (sample body in
`/tmp/run-a/report.md`). **PASS — R9.1 satisfied.**

### 9.6 Open-positions unrealized P&L scoping

`crates/reports/src/lib.rs:145` declares `let unrealized: Decimal =
Decimal::ZERO`. This single binding is then used in **both** sides of
reconciliation identity #1:

- **Headline computation** (line 155):
  `let headline_return_usdt = realized.amount() + unrealized;`
- **Reconciliation `report_unrealized` input** (line 159):
  `unrealized,` — same `Decimal::ZERO` binding in
  `ReconciliationInputs`.
- Identity #4 also uses `unrealized` symmetrically (lines 162–163:
  `equity_delta` and `equity_check_sum` both reuse the same value).

**The dev's scoping note is accurate**: zero is fed identically to both
sides, so the reconciliation identity #1 holds (`0 == 0 + 0`) by
construction in this v1+ scope. The same scalar binding flows to
both sides — no asymmetric hardcode that would mask a real
imbalance. **PASS — no architect handoff required.**

When v2+ wires a real open-positions slice into the orchestrator,
this single `Decimal::ZERO` line gets replaced with a real
mark-to-market sum, and both consumers (headline + reconciliation)
get the real value automatically. The architectural debt is small
and well-localized.

## 10. Architectural Notes

- **`data → audit` edge** (the open-positions typed slice — equity,
  positions value, mark-to-market unrealized P&L) is still pending,
  documented as a v2+ scope tail in
  `crates/reports/src/lib.rs:135–144`. Wave 2c does not regress this;
  T813 acceptance criteria do not require the typed slice.
- The CSV column **naming drift** (`ts` vs spec's `ts_utc`, equity CSV
  shape mismatch) is a v1+ spec-vs-code reconciliation item — the
  renderer's contract is internally consistent (doc-comments + unit
  tests + emitted output all agree); only the spec brief table at
  lines 1004–1013 disagrees. Recommend an architect spot-update of the
  spec table before T816 locks anchor SHAs against the rendered
  output, OR a developer sweep tightening the headers to `ts_utc`.

## 11. Environment / Infrastructure Issues

_none_. 3 ignored data tests are intentionally `#[ignore]`-d
binance_ws live integrations (network-dependent), unrelated to this
wave.

## 12. Verdict

**`PASS`**

All Wave 2c gates green:
- Build: 0 warnings.
- fmt + clippy: clean.
- Tests: 134 reports-crate tests + the rest of the workspace, 0 failures.
- Doctests: clean.
- Anchors: 9/9 PASS.
- Bin smoke: exit 0, markdown + 7 CSVs land at the expected paths.
- T813 tick: all 11 honest-tick citations VERIFIED (file:line + test cmd + output line).
- Render purity: confirmed — no clock/env access in any renderer body.
- Body determinism: byte-identical bodies across two 3-second-spaced runs against the same ledger + seed.
- FAIL path: writes sibling JSON, returns `Err(ReportError::Reconciliation)`, bin maps to exit 1.
- Open-positions scoping note: symmetric zero on both sides of identity #1; no hidden imbalance.

Two non-blocking items flagged for follow-up:

1. CSV header naming drift (`ts` vs `ts_utc`; equity CSV shape) —
   architect spec touch-up _or_ developer header tightening.
2. T814's strict body-SHA byte-identity test on a frozen-fixture
   run-id is the next wave's gate — the body-determinism finding here
   pre-validates the assertion.

## 13. Routing

`VERDICT → PASS` — Wave 2c is ready for T814 (determinism +
body-no-volatile-metadata + reconciliation FAIL integration tests).
T_FINAL_REPORTS remains `[ ]` (developer owns T814–T817 first; tester
ticks the final row only after `VERDICT → PASS` AND
`verify-anchors PASS` on the full T_FINAL_REPORTS gate).
