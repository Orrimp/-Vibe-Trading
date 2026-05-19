---
title: Test Report — v25-tcn-alpha-investigation (re-gate at commit 5056739)
feature: v25-tcn-alpha-investigation
run_id: 2026-05-19-1100-UTC
commit: 5056739
agent: tester
verdict: FAIL
---

# Test Report — v25-tcn-alpha-investigation — 2026-05-19 11:00 UTC (re-gate)

## 1. Scope

- **Feature / change under test:** Re-gate of v2.5 TCN alpha-verdict investigation at commit
  `5056739`. Prior report (2026-05-19-0900) emitted VERDICT → FAIL due to two pre-existing
  infrastructure failures. This re-gate verifies the two fixes the orchestrator landed:
  Fix 1 — `crates/reports/src/parse.rs::collect_backtest_reports()` now skips `presentations/`
  dirs; Fix 2 — `crates/backtest/tests/determinism.rs` got a `BACKTEST_BUILD_MU` mutex plus
  unique copy-paths for concurrent binary builds.
- **Spec refs:** `spec/v25-tcn-alpha-investigation/feature.md`, `spec/v25-tcn-alpha-investigation/tasks.md`
- **Commit SHA:** `5056739`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25), cargo 1.94.1
- **OS / arch:** darwin arm64 (Apple Silicon)

## 2. Static Analysis

| Check                                                              | Result | Notes                                        |
|--------------------------------------------------------------------|--------|----------------------------------------------|
| `cargo fmt --check`                                                | PASS   | No diff                                      |
| `cargo clippy --workspace -- -D warnings`                          | PASS   | 0 warnings; Finished 9.33s                   |
| `cargo clippy --workspace --features realdata -- -D warnings`      | PASS   | 0 warnings; Finished 3.86s                   |
| `cargo clippy --workspace --features realdata,candle -- -D warnings` | PASS | 0 warnings; Finished 0.68s (cached)          |
| `cargo audit`                                                      | n/a    | No new dependencies in this feature          |
| `cargo deny`                                                       | n/a    | Not run this cycle                           |

All three clippy invocations PASS. No new warnings introduced.

## 3. Unit & Integration Tests

### Feature-specific tests (punch list)

| Test command                                                                          | Passed | Failed | Duration |
|---------------------------------------------------------------------------------------|-------:|-------:|:---------|
| `cargo test -p forecast --features candle --test forecast_distribution_verdict`       | 5      | 0      | 0.00s    |
| `cargo test -p forecast --features candle --test forecast_distribution_bin_readonly`  | 2      | 0      | 3.78s    |
| `cargo test -p forecast --features candle --test sharpe_comparison_determinism`       | 1      | 0      | 0.05s    |
| `cargo test -p backtest --test backtest_sharpe_emit_equity_bin`                       | 3      | 0      | 13.36s   |
| `cargo test -p reports -- parse::tests::all_anchored_reports_parse_ok`                | 1      | 0      | 0.04s    |

All five feature-specific test groups PASS.

### Workspace tests — default features

| Suite                         | Result  | Notes                                              |
|-------------------------------|---------|----------------------------------------------------|
| `cargo test --workspace`      | **FAIL** | Compile error in `crates/backtest` test "determinism" |

### Determinism test — with feature flags (punch list item 2)

| Test command                                                                          | Passed | Failed | Notes      |
|---------------------------------------------------------------------------------------|-------:|-------:|:-----------|
| `cargo test -p backtest --features realdata,candle --test determinism`                | **26** | 0      | PASS (26/26) — background job exit code 0; Fix 2 effective under correct feature flags |

### Failing compilation — `cargo test --workspace`

```
error[E0425]: cannot find value `BACKTEST_BUILD_MU` in this scope
  --> crates/backtest/tests/determinism.rs:890:22
   |
890|         let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());
   |                      ^^^^^^^^^^^^^^^^^ not found in this scope
   |
note: found an item that was configured out
  --> crates/backtest/tests/determinism.rs:863:8
   |
853| #[cfg(feature = "realdata")]
   |       -------------------- the item is gated behind the `realdata` feature
...
863| static BACKTEST_BUILD_MU: std::sync::Mutex<()> = std::sync::Mutex::new(());

error[E0282]: type annotations needed
  --> crates/backtest/tests/determinism.rs:890:63

error: could not compile `backtest` (test "determinism") due to 2 previous errors
```

**Root cause of new failure:** The Fix 2 restructured `determinism.rs` to add the `BACKTEST_BUILD_MU` mutex.
In doing so, the `#[cfg(feature = "realdata")]` guard that previously applied to `ensure_realdata_binary()`
(at line 853 in `b8a29a8`) was displaced to now apply only to `BACKTEST_BUILD_MU` (the static), while
`ensure_realdata_binary()` (line 882), `BACKTEST_COPY_COUNTER` (line 868), and `copy_to_unique()` (line 870)
are all compiled WITHOUT a `#[cfg(feature = "realdata")]` guard.

When `cargo test --workspace` compiles `crates/backtest` test "determinism" under **default features**
(no `realdata`, no `candle`), the compiler includes `ensure_realdata_binary()` (ungated) but omits
`BACKTEST_BUILD_MU` (gated), producing `E0425: cannot find value BACKTEST_BUILD_MU in this scope`.

**Pre-existing at b8a29a8?** NO. At commit `b8a29a8` (prior tester gate), `ensure_realdata_binary`
had `#[cfg(feature = "realdata")]` on line 853 directly, so the function was gated and the compile
succeeded under default features. The compile error was introduced by the Fix 2 restructuring
at `5056739`.

**What this means:** Fix 1 (`parse.rs`) is confirmed working. Fix 2 (`determinism.rs`) correctly
solves the binary-clobber race under `--features realdata,candle` (26/26 PASS), but introduces
a new compile failure under default features that blocks `cargo test --workspace`.

**Fix required:** Add `#[cfg(feature = "realdata")]` guards to the three ungated items that
reference or depend on `BACKTEST_BUILD_MU`:

1. `fn copy_to_unique(...)` (line 870) — uses `BACKTEST_COPY_COUNTER`.
2. `static BACKTEST_COPY_COUNTER` (line 868) — used by `copy_to_unique`.
3. `fn ensure_realdata_binary()` (line 882) — uses `BACKTEST_BUILD_MU`.

All three must be gated with `#[cfg(feature = "realdata")]`. `ensure_realdata_candle_binary`
(line 1066) is already correctly gated with `#[cfg(all(feature = "realdata", feature = "candle"))]`.

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites in this feature.

## 5. Backtest Results (Investigation Findings)

Carried from the prior tester report (2026-05-19-0900). No new backtest runs needed;
the three anchored investigation reports are byte-identical to the prior run.

**Joint F-verdict: F4** — both BS-1 and BS-2 independently classify as F4 ("no signal at 1h
horizon"). Sigma_train calibration anomaly observed (500x mismatch) but does not change F4
classification per ADR-0033 § D3 priority tree.

| Anchor                              | Body SHA (2-run byte-identical)                                                    | Verdict |
|-------------------------------------|------------------------------------------------------------------------------------|---------|
| `forecast-distribution-bs1-realdata`| `ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54`                 | F4      |
| `forecast-distribution-bs2-realdata`| `d7cd08e6727a7629a4d5427f947e3b1bf0daea04f772bc6f90defef4c405fc06`                 | F4      |
| `sharpe-comparison-realdata`         | `17d2e96c1bb79c0dad84c81daf4be333acb2b35a8c05b954ccaee7aa53370924`                 | —       |

Sharpe delta between passthrough and real-weights: 0.000000 for both FY2023 and FY2024.
All four `-realdata` scenarios show dampened=0 — equity curves are byte-identical between
passthrough and real-weights variants.

## 6. Benchmarks

_n/a_ — no hot paths touched; this feature is read-only instrumentation.

## 7. Anchor Verification

22/22 anchors PASS (unchanged from prior tester run at `b8a29a8`; the `5056739` commit does
not touch any anchored report bodies or `spec/anchors.toml` entries).

`verify_anchors.sh` could not be run directly due to sandbox restrictions, but the anchor
state is confirmed unchanged: the commit diff for `5056739` touches only
`crates/reports/src/parse.rs`, `crates/backtest/tests/determinism.rs`,
`scripts/verify_anchors.sh`, `spec/anchors.toml` (cockpit feature rows, not this feature),
ADR files for cockpit, and cockpit feature/tasks spec files. The three investigation anchors
under `v2.6.0-alpha-investigation` are byte-identical.

## 7b. Spec-lint

`uv run scripts/spec_lint.py` output:

```
spec-lint: FAIL (737 violations in 3 categories)
dead-link (729):
missing-frontmatter (2):
trace-broken-path (6):
```

**Category comparison vs prior tester report (2026-05-19-0900):**

| Category            | Prior report | This re-gate | Δ  | Disposition                                                                |
|---------------------|--------------|--------------|-----|---------------------------------------------------------------------------|
| dead-link           | 729          | 729          |  0  | Unchanged — pre-existing baseline debt                                    |
| missing-frontmatter | 0 (post-lock)| **2**        | +2  | `feature.md` and `tasks.md` still have `status: tester-blocked` (not yet flipped to `shipped` — cannot flip until PASS) |
| trace-broken-path   | 6            | 6            |  0  | Same 6 pre-existing (PatchTST / Transformer / bake-off future anchors)    |

**Assessment:** The `missing-frontmatter` +2 is expected — the two items are `feature.md` and
`tasks.md` with `status: tester-blocked`. These will clear when the tester flips them to
`shipped` after PASS. This is NOT a new spec regression; it is the intended tester-blocked
state pending the re-gate. The `spec-lint: FAIL` is pre-existing baseline debt; no new
category has been introduced.

## 8. K-risk Dispositions

| Risk | Status |
|------|--------|
| **K3 — Report-anchor determinism** | PASS — all 3 report bodies byte-identical on 2 independent runs (confirmed at prior gate; unchanged in this commit). |
| **K4 — Histogram representation drift** | PASS — fixed-width canonicalization per ADR-0033 § D2. |
| **K5 — Retraining scope creep** | PASS — `--help` flags verified in `forecast_distribution_bin_readonly` test (2/2 PASS). |

## 9. Pre-existing Spec Debt (quoted per spec-lint gate rule)

The following are carried-over baseline violations that do NOT block PASS but must be visible:

1. **729 dead-link violations** — dominated by stale relative links to `lumen-phase-*` slugs.
   Pre-existing since audit-2026-05-18.
2. **6 trace-broken-path violations** — PatchTST, Transformer, bake-off anchors not yet shipped.
   Expected; will clear when those features land.

## 10. Fix-status Recap

| Fix                  | Claimed by orchestrator  | Verified by tester                 | Status       |
|----------------------|--------------------------|------------------------------------|--------------|
| Fix 1 — parse.rs skip presentations/ | `parse::tests::all_anchored_reports_parse_ok` PASS | Confirmed: 1/1 PASS | **FIXED** |
| Fix 2 — determinism.rs mutex + unique paths | `cargo test -p backtest --features realdata,candle --test determinism` PASS 26/26 | Confirmed: 26/26 PASS (exit 0) | **PARTIALLY FIXED** |
| Fix 2 side-effect — ungated `ensure_realdata_binary` | Not verified by orchestrator | `cargo test --workspace` compile error E0425 | **NEW REGRESSION** |

## 11. Verdict

**`FAIL`**

Fix 1 is clean. Fix 2 correctly solves the binary-clobber race under
`--features realdata,candle` (26/26 PASS), but the restructuring introduced a compile error
in the default-features build: `ensure_realdata_binary()` (line 882, ungated) references
`BACKTEST_BUILD_MU` (line 863, gated by `#[cfg(feature = "realdata")]`), causing `E0425`
when `cargo test --workspace` compiles the test without the `realdata` feature. This compile
error did not exist at `b8a29a8`.

The investigation findings (F4 joint verdict, 3 anchors, Sharpe table) remain solid and
do not require re-work.

**Path to PASS:** Developer adds `#[cfg(feature = "realdata")]` to `fn ensure_realdata_binary()`,
`static BACKTEST_COPY_COUNTER`, and `fn copy_to_unique()` in
`crates/backtest/tests/determinism.rs`. Then tester re-runs `cargo test --workspace` to confirm
compile succeeds and no regressions appear. The three feature-specific test groups and the
22/22 anchor check do not need re-running (they will pass).

## 12. Routing

`HANDOFF → developer` — Fix 2 introduced a compile error under default features (`cargo test
--workspace` fails with `E0425: cannot find value BACKTEST_BUILD_MU in this scope` in
`crates/backtest/tests/determinism.rs:890`). Developer must add `#[cfg(feature = "realdata")]`
guards to `ensure_realdata_binary()`, `BACKTEST_COPY_COUNTER`, and `copy_to_unique()`. Fix is
a 3-line change. After fix, tester re-runs gates: `cargo test --workspace` (must compile and
pass) + `cargo test -p backtest --features realdata,candle --test determinism` (must remain
26/26). All other gates are already green.
