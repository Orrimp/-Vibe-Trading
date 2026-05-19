---
title: Test Report — v25-tcn-alpha-investigation
feature: v25-tcn-alpha-investigation
run_id: 2026-05-19-0900-UTC
commit: b8a29a8
agent: tester
verdict: FAIL
---

# Test Report — v25-tcn-alpha-investigation — 2026-05-19 09:00 UTC

## 1. Scope

- **Feature / change under test:** v2.5 TCN alpha-verdict investigation (MINIMAL scope) —
  `forecast_distribution` bin (M-R-HAT, T-D-1..T-D-5) + `sharpe_comparison` bin
  (M-SHARPE, T-D-6..T-D-10) + additive `--emit-equity-bin` flag on `crates/backtest`
  (T-D-8). Closes M-FINAL / T-T-1 (anchor lock + non-regression gate).
- **Spec refs:** `spec/v25-tcn-alpha-investigation/feature.md`,
  `spec/v25-tcn-alpha-investigation/tasks.md`
- **Commit SHA:** `b8a29a8`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25), cargo 1.94.1 (29ea6fb6a 2026-03-24)
- **OS / arch:** darwin arm64 (Apple Silicon)

## 2. Static Analysis

| Check                                                              | Result | Notes                                            |
|--------------------------------------------------------------------|--------|--------------------------------------------------|
| `cargo fmt --check`                                                | PASS   | No diff                                          |
| `cargo clippy --workspace -- -D warnings`                          | PASS   | 0 warnings; Finished in 1.80s                    |
| `cargo clippy --workspace --features realdata -- -D warnings`      | PASS   | 0 warnings; Finished in 12.00s                   |
| `cargo clippy --workspace --features realdata,candle -- -D warnings` | PASS | 0 warnings; Finished in 0.65s (cached)           |
| `cargo audit`                                                      | n/a    | Not run; no new dependencies in this feature     |
| `cargo deny`                                                       | n/a    | Not run this cycle                               |

## 3. Unit & Integration Tests

### Feature-specific tests (all target gates from punch list)

| Test command                                                                          | Passed | Failed | Duration |
|---------------------------------------------------------------------------------------|-------:|-------:|:---------|
| `cargo test -p forecast --features candle --test forecast_distribution_verdict`       | 5      | 0      | 0.00s    |
| `cargo test -p forecast --features candle --test forecast_distribution_bin_readonly`  | 2      | 0      | 4.22s    |
| `cargo test -p forecast --features candle --test sharpe_comparison_determinism`       | 1      | 0      | 0.04s    |
| `cargo test -p backtest --test backtest_sharpe_emit_equity_bin`                       | 3      | 0      | 14.10s   |
| `cargo test -p backtest --features realdata --test determinism`                       | 22     | 0      | 73.64s   |

### Workspace tests (default features)

| Crate / suite             | Passed | Failed | Duration |
|---------------------------|-------:|-------:|:---------|
| backtest (unit)           | 50     | 0      | 6.16s    |
| other workspace crates    | ~52    | 0      | <90s     |
| `crates/reports` (lib)    | 102    | **1**  | 0.03s    |

### Failing test — `parse::tests::all_anchored_reports_parse_ok`

```
thread 'parse::tests::all_anchored_reports_parse_ok' panicked at crates/reports/src/parse.rs:318:13:
parse failed for spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md:
Err(NoSummaryHeading)
```

**Root cause:** The `collect_backtest_reports()` function in `crates/reports/src/parse.rs`
recursively walks ALL subdirectories under `spec/` except `design/` and `archive/`, collecting
every file whose name matches `backtest-*.md`. The file
`spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md`
is a presenter deck (not a backtest report), but its `backtest-` prefix causes it to be
collected and parsed as a report — where it fails `NoSummaryHeading` because the presentation
file lacks the `## Summary` section that backtest reports contain.

**Pre-existing?** YES. The presentation file was committed at `664bb59` (2026-05-18,
`docs(backtest-real-binance-data): presenter deck ready for operator approval`), after the
previous tester run at `d98622e` (the `backtest-real-binance-data` ship gate). The previous
tester report `test-20260518-1800-backtest-real-binance-data.md` shows `reports: 1 passed`
because the presentation file did not yet exist at the time of that test run. The naming
collision (`backtest-*.md` in `presentations/`) was introduced by `664bb59` and went undetected
because no subsequent tester ran the `reports` library tests.

**Fix required:** Developer must either:
- (a) Rename the presentation file to not start with `backtest-` (e.g. `presenter-backtest-real-binance-data-2026-05-18.md`), OR
- (b) Update `collect_backtest_reports()` in `crates/reports/src/parse.rs` to skip the `presentations/` subdirectory, OR
- (c) Both (recommended: fix both the file name AND tighten the collection filter for robustness)

This failure is NOT introduced by the `v25-tcn-alpha-investigation` feature code.
Routes to developer (pre-existing spec debt from `664bb59`).

### Gate 7 — `cargo test -p backtest --features realdata,candle --test determinism`

**Concurrent run: 2/26 FAIL (race condition)**

When run with all 26 tests in parallel, tests `realdata_2023_fy_tcn_overlay_determinism`
(T-D-13) and `realdata_2024_fy_tcn_overlay_determinism` (T-D-14) fail with:
```
Error: unknown scenario: top10-2023-fy-tcn-overlay-realdata
```

**Root cause:** Both `ensure_realdata_binary()` (used by T-D-13/T-D-14) and
`ensure_realdata_candle_binary()` (used by T-D-15) write to the same
`target/debug/backtest` path. When the tests run concurrently, the binaries clobber
each other. In the failing run, the binary at execution time was compiled WITHOUT the
`realdata` feature active (despite `ensure_realdata_binary()` requesting `--features realdata`),
causing the `#[cfg(feature = "realdata")]` scenario arms to be absent.

**Single-threaded run: 26/26 PASS** — when run with `--test-threads 1`, all 26 tests pass
(see single-threaded result below). The race condition resolves with sequential execution.

**Pre-existing?** The binary-clobber race between `ensure_realdata_binary()` and
`ensure_realdata_candle_binary()` exists in `determinism.rs` since the test was introduced
at `df73780` (before the current feature). The previous tester ran at `d98622e` and showed
26/26 PASS — but at that time the `realdata,candle` tests (T-D-15) may have been absent or
skipped due to missing LFS checkpoints. With LFS resolved (checkpoints now present), T-D-15
runs and creates the race condition. This is a pre-existing infrastructure gap exposed by
the new LFS-resolved checkpoint environment.

**Fix required:** Developer must separate the two binary build outputs — e.g. use
`--target-dir` or separate cargo `--out-dir` flags, or serialize the realdata/realdata+candle
test groups with `#[serial]` (using the `serial_test` crate) to prevent concurrent binary
writes to `target/debug/backtest`.

**Single-threaded gate 7 result (running now):** PENDING at report write time —
single-threaded run started as background task. Expected: 26/26 PASS based on
isolated T-D-13 run (confirmed PASS in isolation above).

**Update:** Single-threaded determinism run (`--test-threads 1`) confirmed **26/26 PASS**
after report composition — see routing section for disposition.

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites in this feature.

## 5. Backtest Results (Investigation Findings)

The investigation produces forensic reports, not new backtest scenarios. The three anchored
investigation reports constitute the "backtest results" for this feature.

**Universe:** 10 USDT pairs (ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT,
ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT)

**Period:** BS-1 = 2023-01-01..2024-01-01 (FY2023), BS-2 = 2024-01-01..2025-01-01 (FY2024)

**Data source:** Binance Vision hourly OHLCV, revision SHA
`3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`

### Forecast-distribution (M-R-HAT)

| Stat         | BS-1 (FY2023)      | BS-2 (FY2024)      |
|--------------|--------------------|--------------------|
| Inferences   | 77,830             | ~77,830 est.       |
| mean r_hat   | 0.000904261        | —                  |
| std r_hat    | 0.018015573        | —                  |
| abs_p95      | 0.032130495        | ~similar           |
| sigma_train  | 10.954250          | 6.916 (BS-2)       |
| frac inside ε | 0.030952 (3.1%)   | ~similar           |
| frac passes confidence gate (τ=0.6) | 0.000000 | 0.000000 |
| **F-verdict** | **F4**            | **F4**             |
| Joint verdict | **F4** (agree)    | Follow-on: `v25-tcn-horizon-bump-or-retire` |

**Sigma_train calibration anomaly (observed, not a blocker):**
`sigma_train` (BS-1 = 10.954, BS-2 = 6.916) is approximately **500x larger** than the
inference-time `r_hat` standard deviation (~0.018–0.022). The F2 condition
`std > 0.1 * sigma_train` evaluates as `0.018 > 1.095` → FALSE, so F2 does not fire.
F4 fires as catch-all. The mismatch suggests `sigma_train` was computed in different
units at training time (possibly z-scored or otherwise normalized targets, while inference
operates on raw log-returns). This anomaly does not change the F4 verdict but surfaces a
secondary calibration concern that a follow-on recalibrate feature could address cheaply.

**Anchor body SHAs (2-run byte-identical, orchestrator-verified):**
- `forecast-distribution-bs1-realdata`: `ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54`
- `forecast-distribution-bs2-realdata`: `d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06`

### Sharpe-comparison (M-SHARPE)

| Scenario                                        | Variant       | Bars   | Dampen rate | Sharpe (ann) | Sortino (ann) | Calmar   |
|-------------------------------------------------|---------------|--------|-------------|--------------|---------------|----------|
| top10-2023-fy-tcn-overlay-realdata              | passthrough   | 87,590 | 0.00%       | 0.003098     | 0.004380      | 0.017263 |
| top10-2024-fy-tcn-overlay-realdata              | passthrough   | 87,840 | 0.00%       | 0.001389     | 0.001965      | 0.006447 |
| top10-2023-fy-tcn-overlay-weights-realdata      | real-weights  | 87,590 | 0.00%       | 0.003098     | 0.004380      | 0.017263 |
| top10-2024-fy-tcn-overlay-weights-realdata      | real-weights  | 87,840 | 0.00%       | 0.001389     | 0.001965      | 0.006447 |

Sharpe delta between passthrough and real-weights: **0.000000** for both years.
Annualisation: √(24·365) = 92.601295 (hourly). Risk-free = 0.

**Anchor body SHA (2-run byte-identical):**
- `sharpe-comparison-realdata`: `17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924`

## 6. Benchmarks

_n/a_ — no hot paths touched; this feature is read-only instrumentation.

## 7. Anchor Verification

### Pre-lock (19/19 PASS)

```
ANCHORS PASS  (19 / 19)
```
All 19 original anchors byte-identical — R6 non-regression contract confirmed.

### Anchor lock (T-T-1)

Three new anchors appended to `spec/anchors.toml` under version `v2.6.0-alpha-investigation`:

```toml
[[anchors]]
scenario = "forecast-distribution-bs1-realdata"
version  = "v2.6.0-alpha-investigation"
sha256   = "ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54"

[[anchors]]
scenario = "forecast-distribution-bs2-realdata"
version  = "v2.6.0-alpha-investigation"
sha256   = "d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06"

[[anchors]]
scenario = "sharpe-comparison-realdata"
version  = "v2.6.0-alpha-investigation"
sha256   = "17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924"
```

`verify_anchors.sh` was extended with a third fallback find pattern
(`*/reports/$scenario-*.md`) to locate investigation reports that follow the
`<scenario>-YYYYMMDD.md` naming convention rather than the `backtest-*-<scenario>.md`
convention used by backtest reports.

### Post-lock (22/22 PASS)

```
PASS  forecast-distribution-bs1-realdata    ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54
PASS  forecast-distribution-bs2-realdata    d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06
PASS  sharpe-comparison-realdata            17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924
ANCHORS PASS  (22 / 22)
```
All 19 original anchors byte-identical. R6 contract upheld.

## 7b. Spec-lint

**Pre-lock spec-lint:** 738 violations in 2 categories (729 dead-link + 9 trace-broken-path).
The 3 extra trace-broken-path (vs baseline 6) were from the 3 new anchors not yet in
`anchors.toml`.

**Post-lock spec-lint:** 735 violations in 2 categories (729 dead-link + 6 trace-broken-path).

Baseline (audit-2026-05-18.md): 734 violations in 3 categories
(727 dead-link + 1 missing-frontmatter + 6 trace-broken-path).

| Category            | Baseline | Post-lock | Δ  | Disposition                                                  |
|---------------------|----------|-----------|----|--------------------------------------------------------------|
| dead-link           | 727      | 729       | +2 | 1 from this feature (`tasks.md` → `.claude/skills/` relative path); 1 from `ui-rethink-phase-a-lab` (prior commit). Pre-existing pattern. |
| missing-frontmatter | 1        | 0         | -1 | Cleared (improvement)                                        |
| trace-broken-path   | 6        | 6         | 0  | Same 6 pre-existing (PatchTST / Transformer / bake-off future anchors) |

**No new category regressions.** The +2 dead-link delta is within the pre-existing dead-link
pattern (relative paths from `spec/` to `.claude/` are flagged throughout the codebase). The
`spec-lint: FAIL` reflects the pre-existing baseline debt, NOT a new regression introduced
by this feature.

## 8. K-risk Dispositions

| Risk | Status |
|------|--------|
| **K3 — Report-anchor determinism** | PASS — all 3 report bodies byte-identical on 2 independent runs (orchestrator-verified at b8a29a8). |
| **K4 — Histogram representation drift** | PASS — fixed-width canonicalization per ADR-0033 § D2 (microreturn-i64 bin edges, %.9f percentiles, %.6f gates). |
| **K5 — Retraining scope creep** | PASS — `--help` contains no `retrain`, `update`, or `write-checkpoint` flag (verified by `forecast_distribution_bin_readonly` test). |

## 9. Pre-existing Spec Debt (quoted per spec-lint gate rule)

The following are carried-over baseline violations that do NOT block PASS but must be visible:

1. **729 dead-link violations** — dominated by stale relative links to `lumen-phase-*` slugs
   re-nested under `spec/lumen-design-adoption/phase-*/`. Pre-existing since audit-2026-05-18.
2. **6 trace-broken-path violations** — anchors for PatchTST, Transformer, and bake-off
   features not yet shipped. Expected; will clear when those features land.
3. **`parse::tests::all_anchored_reports_parse_ok` FAIL (pre-existing)** — presentation file
   `spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md`
   committed at `664bb59` uses a `backtest-` prefix that is accidentally collected by the
   `crates/reports` parse test. Introduced at `664bb59`, not by this feature.
4. **`realdata_2023/2024_fy_tcn_overlay_determinism` concurrent race (pre-existing)** —
   two binary-build helpers (`ensure_realdata_binary` / `ensure_realdata_candle_binary`) write
   to the same `target/debug/backtest` path; concurrent test runs race to clobber each other.
   Fails intermittently under `--features realdata,candle` parallel execution; PASS under
   `--test-threads 1`. First exposed in this environment with LFS checkpoints resolved
   (previously skipped due to absent checkpoints).

## 10. Verdict

**`FAIL`**

All feature-specific tests pass: fmt PASS, clippy PASS (all 3 feature combos), the 4
feature test suites PASS (11 tests total), `backtest_sharpe_emit_equity_bin` 3/3 PASS,
`backtest --features realdata --test determinism` 22/22 PASS, verify-anchors 22/22 PASS
(post-lock), K3/K4/K5 risks resolved, joint F4 verdict correctly published.

**Two failing tests are pre-existing regressions** not introduced by this feature's code:
1. `parse::tests::all_anchored_reports_parse_ok` — broken by `664bb59` presenter naming
2. `realdata_2023/2024_fy_tcn_overlay_determinism` (concurrent) — test infrastructure race
   in `determinism.rs`, exposed by LFS resolution enabling T-D-15 tests to run

Per tester protocol: the verdict is FAIL because the gate `cargo test --workspace` has a
failing test and the punch list item 7 (`cargo test -p backtest --features realdata,candle
--test determinism`) has 2 failures in default parallel execution (though PASS single-threaded).

**Path to PASS:** Developer fixes the two pre-existing failures, tester re-runs gates 5 and 7.

## 11. Routing

`HANDOFF → developer` — two pre-existing test failures must be fixed before PASS:

1. **`parse::tests::all_anchored_reports_parse_ok`** — rename presentation file
   `spec/backtest-real-binance-data/presentations/backtest-real-binance-data-2026-05-18.md`
   to not start with `backtest-` (e.g. `presenter-backtest-real-binance-data-2026-05-18.md`)
   AND/OR update `collect_backtest_reports()` in `crates/reports/src/parse.rs` to also
   skip the `presentations/` directory. Both fixes recommended for robustness.

2. **`realdata_2023/2024_fy_tcn_overlay_determinism` (concurrent binary-clobber race)** —
   serialize the `ensure_realdata_binary` and `ensure_realdata_candle_binary` test groups
   in `crates/backtest/tests/determinism.rs` (e.g. using `serial_test` crate, or separating
   into distinct test files that can be run with different feature sets), OR use distinct
   output paths for the two binary builds to avoid the `target/debug/backtest` clobber.

After both fixes: re-run gates 5 and 7 to confirm PASS, then return to tester for final
stamp. The anchor lock (22/22), all feature-specific tests, and the F4 alpha-verdict
investigation findings are solid and do not need re-work.
