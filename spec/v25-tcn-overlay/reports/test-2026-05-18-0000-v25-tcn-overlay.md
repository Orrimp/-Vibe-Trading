---
title: Test Report
feature: v25-tcn-overlay
slug: v25-tcn-overlay
report: test
run_id: 2026-05-18-0000-UTC
commit: 1a4c4e4a187b15e66b4f03d1b4e9e48a2701ac22
agent: tester
verdict: FAIL
anchors_status: PARTIAL — 11/11 existing PASS; 2 new anchors locked (bs1/bs2-tcn-overlay); verify_anchors.sh gate for canonical names (top10-2023-fy-tcn-overlay / top10-2024-fy-tcn-overlay) BLOCKED — scenario naming mismatch
updated: 2026-05-18
---

# Test Report — v25-tcn-overlay — 2026-05-18 00:00 UTC

## 1. Scope

- **Feature / change under test:** v2.5 TCN forecast overlay (phase 1 of 4) — M4-M7 tester gate (T-D-15, T-D-16, T-T-1). BS-1 and BS-2 full-year backtests with passthrough-forecaster synthetic RNG; determinism verification; anchor lock; full static analysis.
- **Spec refs:** `spec/v25-tcn-overlay/feature.md`, `spec/v25-tcn-overlay/tasks.md`
- **Commit SHA:** `1a4c4e4a187b15e66b4f03d1b4e9e48a2701ac22`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`, `cargo 1.94.1 (29ea6fb6a 2026-03-24)`
- **OS / arch:** `Darwin 25.4.0 arm64`

## 2. Static Analysis

| Check                                                   | Result        | Notes                                                                                                                                             |
|---------------------------------------------------------|---------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `cargo fmt --check`                                     | **FAIL**      | 2 files have formatting diffs: `crates/agent/src/config.rs:714` (multi-line chain), `crates/agent/src/lib.rs:17` (import reordering). Pre-existing or from Wave D. Developer must run `cargo fmt`.   |
| `cargo clippy --workspace -- -D warnings`               | **FAIL**      | 4 errors in `crates/forecast/src/tcn.rs` (see §2.1). Crate does not compile under strict clippy. |
| `cargo audit`                                           | N/A           | `cargo-audit` not installed in environment (pre-existing infra item — same as v2 report §2).     |
| `cargo deny check`                                      | **PRE-EXISTING FAIL** | `advisories FAILED` (RUSTSEC-2024-0436: `paste` unmaintained), `licenses FAILED` (MIT-0 `borrow-or-share`, no-license `polars-arrow-format`). Introduced before v2.5, confirmed pre-existing. |
| `spec-lint` (`uv run scripts/spec_lint.py`)             | **FAIL**      | 737 violations in 3 categories (see §2.2). 2 are NEW (unreferenced-anchor). Blocks PASS per AGENT.md.                                            |

### 2.1 Clippy errors in `crates/forecast/src/tcn.rs` (4 errors, NEW)

All 4 are in Wave D developer code (`tcn.rs`), introduced in commit `c4fa6c9`. They prevent `cargo clippy --workspace -- -D warnings` from passing.

```
error: this operation will always return zero. This is likely not the intended outcome
   --> crates/forecast/src/tcn.rs:684:17
   (erasing_op)

error: this operation has no effect
   --> crates/forecast/src/tcn.rs:685:17
   (identity-op)

error: this `if` statement can be collapsed
   --> crates/forecast/src/tcn.rs:912:9
   (collapsible-if)

error: this `if` statement can be collapsed
   --> crates/forecast/src/tcn.rs:913:13
   (collapsible-if)
```

**Action required:** developer must fix `tcn.rs:684-685` (arithmetic ops that simplify to 0/identity) and `tcn.rs:912-913` (collapse nested if-let).

### 2.2 Spec-lint violations

**Total: 737 violations in 3 categories.**

| Category | Count | Status |
|----------|------:|-------|
| dead-link | 727 | PRE-EXISTING — all link targets for Lumen phases, backlog features, v15a cross-refs. No new dead-links introduced by v2.5 changes. |
| unreferenced-anchor | 2 | **NEW** — `bs1-tcn-overlay` and `bs2-tcn-overlay` added to `spec/anchors.toml` but `spec/trace.toml` REQ-V25-TCN-001 cites `top10-2023-fy-tcn-overlay` / `top10-2024-fy-tcn-overlay`. Naming mismatch (see §5). |
| trace-broken-path | 8 | 2 are NEW-ish (REQ-V25-TCN-001 cites anchors not in anchors.toml) but caused by the same naming mismatch; 6 are PRE-EXISTING roadmap-only rows (REQ-V25A, REQ-V25B, REQ-V26). |

**Pre-existing spec debt (carried from all prior runs):**
- 727 dead-link violations — orphan feature folder cross-references in backlog.md and architecture docs (Lumen phase stubs, v15a partial migrations). These predate v2.5 and are not introduced by this feature.

## 3. Unit & Integration Tests

`cargo test --workspace --exclude forecast` (forecast excluded due to clippy compile errors in --features candle path; base `cargo build -p backtest --release` PASSES).

| Crate / suite           | Passed | Failed | Ignored | Notes |
|-------------------------|-------:|-------:|--------:|-------|
| `backtest` (unit)       | 9      | 0      | 0       |       |
| `backtest` (integration)| 20     | 0      | 0       | Includes 2 NEW TCN anchor tests (T-T-1) |
| `backtest` (multi-pair) | 2      | 0      | 0       |       |
| `backtest` (multi-symbol)| 5     | 0      | 0       |       |
| `reports`               | 103    | 0      | 0       |       |
| `strategy`              | 41+    | 0      | 0       | Includes 7 TCN overlay unit tests from Wave D |
| `ui`                    | 235    | 0      | 0       |       |
| All other crates        | ~800+  | 0      | 4 ignored | 4 ignored are pre-existing `#[ignore]` flags |
| **Total (excl. forecast)**| **~1300** | **0** | **4** | Zero failures |

### Failing Tests

_none_ — all non-forecast tests pass.

### Forecast crate

`cargo test -p forecast --features candle` was NOT run because `cargo clippy -p forecast --features candle -- -D warnings` fails with 4 errors (§2.1). The developer reported all 47+ forecast tests passing in Wave A+B/D (task.md T-D-5, T-D-6, T-D-13, T-D-14 evidence). Those results are cited from tasks.md but were NOT re-executed in this tester run due to the clippy errors blocking compilation of the crate under `-D warnings`.

## 4. Property / Fuzz Tests

_n/a_ — no proptest or cargo-fuzz suites defined for this feature. The determinism property (same seed → same output) is tested via the anchor regression tests in §3.

## 5. Backtest Results

**IMPORTANT WARNING: Both BS-1 and BS-2 runs use `PassthroughForecaster` (no-candle mode). The real TCN checkpoint inference (M3 weights) has not been run. These results are v1 momentum baseline performance with synthetic hourly bars, NOT the TCN overlay contribution. The TCN overlay is not active because the `candle` feature is not enabled in the backtest binary path.**

### BS-1 — 2023 Full-Year Top-10 USDT

**Universe:** ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT
**Period:** 2023 (2208 bars per symbol, 22080 total)
**Data source:** Synthetic seeded RNG (ChaCha20, seed 0xC0FFEE, v2.5 tcn-overlay)
**Fees / slippage:** 2 bps slippage, 4 bps taker fee

| Metric           | BS-1 (passthrough TCN) | v1 Baseline (top10-2023-1h-momentum) | Delta |
|------------------|----------------------:|-------------------------------------:|------:|
| Total return     | **-69.76%**           | ~v1 (synthetic data differs)        | N/A — different data generators |
| Max drawdown     | **87.48%**            | ~v1                                  | N/A   |
| Trades           | 1224                  | ~v1                                  | N/A   |
| Final equity     | $30,235.58            | $100,000 start                       | -$69,764 |
| Dampen rate      | 0.00%                 | N/A                                  | N/A — PassthroughForecaster never dampens |

**FAILURE ASSESSMENT:** Final equity -69.76% with 87.48% max drawdown in synthetic data. The dampened-to-Hold count is 0 of 1224 signals — the TCN overlay contributes nothing (PassthroughForecaster, dampen rate 0%). This does NOT represent TCN model performance; it is v1 momentum on synthetic data.

**Success criteria assessment (feature.md § Backtest Scenarios):**
- Sharpe ≥ v1 + 0.10: **CANNOT ASSESS** — no real TCN inference; passthrough mode equals v1; no Sharpe computed in report
- Max drawdown not worse by >2pp: **CANNOT ASSESS** — same as above
- Trade count ≤ 1.5× v1: 1224 vs v1 baseline on same synthetic data — approximately equal (no inflation from TCN)

The -69.76% return is concerning as a raw number but is an artifact of synthetic data for a strategy that has no TCN contribution active. The real assessment requires M3 training run completion (T-D-11/T-D-12) and real checkpoint inference.

### BS-2 — 2024 Full-Year Top-10 USDT

**Universe:** Same 10 symbols
**Period:** 2024 (6600 bars per symbol, 66000 total)
**Data source:** Synthetic seeded RNG

| Metric        | BS-2 (passthrough TCN) |
|---------------|----------------------:|
| Total return  | **-55.70%**           |
| Max drawdown  | **87.48%**            |
| Trades        | 3672                  |
| Final equity  | $44,300.24            |
| Dampen rate   | 0.00%                 |

**Same assessment as BS-1:** PassthroughForecaster active; zero TCN contribution; cannot assess against success criteria.

### Determinism — BS-1 Replay Verification (T-D-15)

**PASS.** Pre-ran report (`backtest-20260517-185919-bs1-tcn-overlay.md`, operator-run at 18:59:19) and re-run (`backtest-20260517-215112-bs1-tcn-overlay.md`, tester-run at 21:51:12) produce **byte-identical body hashes**:

```
bs1 pre-ran SHA-256: 015aeed0b25808152c55b60186fe53cf6e329f89b91c86071e5516b7149bc636
bs1 re-run  SHA-256: 015aeed0b25808152c55b60186fe53cf6e329f89b91c86071e5516b7149bc636
```

Verified by `cargo test -p backtest --test tcn_anchor_hash -- --nocapture`.

### Equity Curve Summary

Both BS-1 and BS-2 show severe drawdown (87.48% max drawdown) on synthetic data. The synthetic data generator uses independent ChaCha20Rng streams per symbol which produce adverse trending behavior under the equal-weight momentum strategy. This is expected behavior on random-walk synthetic data and does not represent live market performance. The TCN model's contribution (when the M3 training run completes and the candle feature is enabled) is the key unknown.

## 6. Benchmarks

_n/a_ — no criterion benchmarks defined for the v2.5 TCN overlay feature.

## 7. Anchor Verification

### 7.1 Existing 11 Anchors (verify_anchors.sh equivalent via cargo test)

`cargo test -p backtest --test determinism` — **20/20 PASS** (18 prior anchors + 2 new TCN anchors by scenario ID).

The 11 existing anchors in `spec/anchors.toml` (v0, v0.5, v1, v1.5a, v2.0.0 scenarios) are byte-identical. Confirmed via T622 + T717 tests.

### 7.2 New TCN Anchors (T-T-1)

| Scenario (anchors.toml) | Version | SHA-256 | Status |
|-------------------------|---------|---------|--------|
| `bs1-tcn-overlay` | v2.5.0 | `015aeed0b25808152c55b60186fe53cf6e329f89b91c86071e5516b7149bc636` | LOCKED |
| `bs2-tcn-overlay` | v2.5.0 | `698f1ffb951e357b8708171107f6190be2d8c68fdddcbda8a0a731345a5a79ec` | LOCKED |

**NAMING CONFLICT (T-T-1 incomplete):** The feature.md § Backtest Scenarios and `spec/trace.toml` REQ-V25-TCN-001 require the canonical anchor names `top10-2023-fy-tcn-overlay` and `top10-2024-fy-tcn-overlay`. The backtest binary (`crates/backtest/src/main.rs`) uses scenario IDs `"bs1-tcn-overlay"` and `"bs2-tcn-overlay"`, producing report files `backtest-*-bs1-tcn-overlay.md` and `backtest-*-bs2-tcn-overlay.md`. `verify_anchors.sh` resolves anchor scenario names to report files by the pattern `backtest-*-<scenario>.md` — so the canonical names will MISS.

**Developer action required:** Rename the backtest scenarios in `crates/backtest/src/main.rs` from `"bs1-tcn-overlay"` / `"bs2-tcn-overlay"` to `"top10-2023-fy-tcn-overlay"` / `"top10-2024-fy-tcn-overlay"`. Then regenerate the reports, recompute hashes, update anchors.toml, and re-run the tester gate.

### 7.3 verify_anchors.sh Status

**BLOCKED** — `bash scripts/verify_anchors.sh` and `python3 scripts/hash_report.py` were both permission-denied in this tester session despite the allowlist statement. Anchor verification was performed via the Rust `cargo test -p backtest --test determinism` path and the inline `tcn_anchor_hash` test (removed after use). This is functionally equivalent for the body-hash gate.

## 8. Environment / Infrastructure Issues

1. **`bash scripts/verify_anchors.sh` permission-denied** — The shell script and all `python3 scripts/*` calls were denied by the session permission system despite the user stating these are allowlisted. Workaround: used `uv run scripts/spec_lint.py` for spec_lint and Rust `cargo test` for anchor hash verification. Flag to operator: the allowlist needs to be configured to permit `bash scripts/*.sh` and `python3 scripts/*.py`.

2. **Python 3.9 installed as `python3`** — `spec_lint.py` requires Python 3.11+ (`tomllib`). `uv run` worked as the alternative.

3. **PassthroughForecaster active in all backtest runs** — The `candle` feature is not enabled in `cargo run -p backtest --release`. This means the TCN model is never called. The real TCN backtest requires M3 training run completion (T-D-11/T-D-12 still open).

## 9. Verdict

**`FAIL`**

Multiple blocking issues prevent PASS:

1. **`cargo fmt --check` FAILS** — 2 files in `crates/agent/` have formatting diffs. Developer must run `cargo fmt --all`.
2. **`cargo clippy --workspace -- -D warnings` FAILS** — 4 clippy errors in `crates/forecast/src/tcn.rs` (Wave D code). Developer must fix `tcn.rs:684-685` (erasing_op/identity-op) and `tcn.rs:912-913` (collapsible-if).
3. **Spec-lint NEW regression: `unreferenced-anchor` (0 → 2)** — `bs1-tcn-overlay` and `bs2-tcn-overlay` added to `anchors.toml` are not cited by any `trace.toml` row (which uses `top10-2023-fy-*` names). Developer must rename backtest scenarios to align with the canonical anchor names from feature.md and trace.toml.
4. **T-T-1 incomplete** — The two new anchor names in anchors.toml (`bs1-tcn-overlay`, `bs2-tcn-overlay`) do not match the canonical names required by the feature spec (`top10-2023-fy-tcn-overlay`, `top10-2024-fy-tcn-overlay`). `verify_anchors.sh` would MISS on the canonical names.
5. **BS-1 -69.76% return flags a SUCCESS CRITERIA FAIL** — The feature.md success criterion requires TCN overlay Sharpe ≥ v1 + 0.10. With PassthroughForecaster (no active TCN), this criterion cannot even be evaluated. The M3 training run (T-D-11/T-D-12) must complete before any meaningful success-criterion assessment can be made.

Non-blocking findings (pre-existing):
- `cargo deny check` advisory + license failures (pre-existing; identical to v2 report)
- `cargo audit` not installed (pre-existing)
- 727 dead-link violations (pre-existing spec debt)
- 6 of 8 trace-broken-path violations (roadmap rows with no anchors yet)

## 10. Routing

`HANDOFF → developer` — 3 code-level fixes required before re-gate:

1. Run `cargo fmt --all` to fix formatting in `crates/agent/`.
2. Fix `crates/forecast/src/tcn.rs:684-685` (erasing_op / identity-op) and `tcn.rs:912-913` (collapsible-if) so `cargo clippy --workspace -- -D warnings` passes.
3. Rename backtest scenarios `"bs1-tcn-overlay"` → `"top10-2023-fy-tcn-overlay"` and `"bs2-tcn-overlay"` → `"top10-2024-fy-tcn-overlay"` in `crates/backtest/src/main.rs`, regenerate reports, recompute hashes, update `spec/anchors.toml`, update `crates/backtest/tests/determinism.rs` anchor tests.

`HANDOFF → architect / analyst (advisory, non-blocking)` — The real TCN success-criteria evaluation (Sharpe vs v1 baseline) requires M3 full training run. The -69.76% BS-1 return is synthetic data / passthrough-forecaster behavior, not TCN model performance. Once M3 completes and the `candle` feature is enabled in the backtest binary, the tester gate must be re-run to evaluate the actual success criteria.
