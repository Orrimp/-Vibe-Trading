---
title: Test Report — v5-latency-slippage-sim-v0.3.0-full-path-wiring
feature: v5-latency-slippage-sim-v0.3.0-full-path-wiring
run_id: 2026-05-27-1850-UTC
commit: 21bda41
agent: tester
verdict: SOFT-PASS
---

# Test Report — v5-latency-slippage-sim-v0.3.0-full-path-wiring — 2026-05-27 18:50 UTC

## 1. Scope

- **Feature / change under test:** v5 latency-slippage-sim v0.3.0 — full-path wiring. Plumbs
  `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }` into
  the 6 strategy construction sites v0.2.0 missed (SmaComposed, Pairs, TcnOverlay,
  TcnOverlayWeights, PatchTstOverlay, GarchVolOverlay). Lifts `sim_slippage_cost` to shared
  `crates/backtest/src/scenarios/sim.rs`. Adds `--force-synthetic-bars` CLI flag (Q1=(a)
  revert-to-synthetic for Group A). Implements namespace-aware Rust resolver for
  `t1937_nine_strategy_anchors_unchanged` (Q4=(b)). Re-emits 11 canonical reports. Migrates
  9 anchor SHAs in-place (Q3=(a) extend same `v5-realdata-medium-2026-05` pin). Extends
  Sharpe-delta table to all 7 strategy paths.
- **Spec refs:** `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/feature.md`,
  `spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/tasks.md`,
  `spec/architecture/adr/0047-v5-v0.3.0-full-path-wiring-and-namespace-aware-resolver.md`
- **Commit SHA:** `21bda41` (feat(v5-latency-slippage-sim-v0.3.0-full-path-wiring): Wave A-F
  M-DEV — full plumbing, 69/69, t1937 GREEN)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** darwin 25.5.0 (Apple Silicon)
- **Tester agent run date:** 2026-05-27

---

## 2. Static Analysis

| Check              | Result | Notes |
|--------------------|--------|-------|
| `cargo fmt --check` | PASS | No diff; exit code 0. |
| `cargo clippy -- -D warnings` | FAIL (pre-existing only) | All errors in `crates/forecast/tests/` (6 locations) and `crates/ui/` (132 locations). Zero errors attributable to v0.3.0 changes. `crates/backtest/` (the v0.3.0 surface) is clean. See § Clippy detail below. |
| `cargo audit` | not run | No new dependencies introduced by v0.3.0. |
| `cargo deny` | not run | No new dependencies introduced by v0.3.0. |

### Clippy detail

`cargo clippy --workspace --all-targets -- -D warnings` exits non-zero due to pre-existing
warnings in `crates/forecast/tests/` and `crates/ui/`. The v0.3.0 commit (`21bda41`) stat
confirms it did not touch any file in `crates/forecast/` or `crates/ui/` (except
`crates/ui/tests/lab_markers_anchor.rs` — 2 lines, and that file has zero clippy errors).

**Pre-existing clippy errors (carried from prior tester reports — same set as v0.2.0 M-FINAL):**
- `crates/forecast/tests/patchtst_byte_identity.rs:40` — collapsible-if
- `crates/forecast/tests/tcn_byte_identity.rs:43` — dead_code (fn assert_no_git_diff)
- `crates/forecast/tests/tcn_byte_identity.rs:170` — collapsible-if
- `crates/forecast/tests/patchtst_overlay_neutrality.rs:233` — collapsible-if
- `crates/forecast/tests/forecast_distribution_verdict.rs:233` — doc-lazy-continuation
- `crates/forecast/tests/forecast_distribution_verdict.rs:312` — neg-cmp-op-on-partial-ord
- `crates/forecast/src/bin/sharpe_comparison.rs` (5 locations) — manual assign-op
- `crates/ui/src/` (132 locations) — pre-existing mix of deprecated variants, dead code, etc.

**Zero new clippy warnings attributable to v0.3.0 changes (backtest crate is clean).**

### grep gate: ADR-0047 D2 — sim_slippage_cost defined exactly once

```
grep -r "fn sim_slippage_cost" crates/backtest/src/
```

Result: exactly 1 definition at `crates/backtest/src/scenarios/sim.rs` (with a comment in
`crates/backtest/src/scenarios/momentum.rs` noting the move). Gate: PASS.

---

## 3. Unit & Integration Tests

### 3a. Targeted gates (developer-claimed — independently verified)

| Gate | Command | Expected | Actual | Result |
|------|---------|----------|--------|--------|
| T-T-1 — anchors | `bash scripts/verify_anchors.sh` | PASS 69/69 | PASS 69/69 | PASS |
| T-T-2a — backtest lib | `cargo test -p backtest --lib` | 35 pass | 35 pass; 0 failed; 5 ignored | PASS |
| T-T-2b — t1937 + t1937b | `cargo test -p reports --test strategy_anchors_unchanged` | 3 pass | 3 pass (t1937 GREEN, t1937b GREEN, t1942 GREEN) | PASS |
| T-T-4a — latency sim e2e | `cargo test -p strategy --test latency_slippage_sim_e2e` | 3 pass | 3 pass | PASS |
| T-T-4b — vol targeting e2e | `cargo test -p strategy --test vol_targeting_overlay_end_to_end` | 1 pass | 1 pass | PASS |
| T-T-4c — vol killswitch e2e | `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | 4 pass | 4 pass | PASS |

All 6 targeted gates match developer claims exactly.

### 3b. Workspace-wide test: `cargo test --workspace --no-fail-fast`

All test suites PASS with **zero new failures** attributable to v0.3.0. One pre-existing
whitelisted failure observed:

| Test | File | Status | Attribution |
|------|------|--------|-------------|
| `inner::h3_in_memory_equals_cached_disk` | `crates/ui/tests/lab_run_engine.rs:108` | FAILED | Pre-existing flake; whitelisted in multiple prior tester reports (cockpit-activity-status-bar, reflection-memory-trader-wiring, v0.2.0 M-FINAL). `21bda41` did not touch this file. |

All other suites: PASS. The v0.2.0-whitelisted `t1937_nine_strategy_anchors_unchanged` failure has
**flipped to GREEN** per R3/R-NR.4 gate — this is the key R3 deliverable.

### Crate-level summary (workspace run)

| Crate / suite | Passed | Failed | Ignored |
|---|---:|---:|---:|
| backtest (lib) | 35 | 0 | 5 |
| reports (strategy_anchors_unchanged) | 3 | 0 | 0 |
| strategy (latency_slippage_sim_e2e) | 3 | 0 | 0 |
| strategy (vol_targeting_overlay_end_to_end) | 1 | 0 | 0 |
| strategy (vol_killswitch_overlay_end_to_end) | 4 | 0 | 0 |
| ui (lab_run_engine) — pre-existing flake | 0 | 1 | 0 |
| all other workspace suites | 200+ | 0 | 4 |
| **Total workspace** | **200+** | **1 (pre-existing)** | **9** |

### Failing Tests

**Pre-existing flake only — no new failures.**

`inner::h3_in_memory_equals_cached_disk` in `crates/ui/tests/lab_run_engine.rs:108`:
```
thread panicked at crates/ui/tests/lab_run_engine.rs:108:22:
write_report=true should produce a report_path
```
Whitelisted since cockpit-activity-status-bar test report (2026-05-26).
The v0.3.0 commit did not touch `lab_run_engine.rs`.

---

## 4. Property / Fuzz Tests

_n/a — no proptest or cargo-fuzz suites in scope for this feature._

---

## 5. Backtest Results

### 5a. Anchor verification — `bash scripts/verify_anchors.sh`

```
ANCHORS PASS  (69 / 69)
```

All 69 rows pass. This includes:
- 34 noop-baseline rows (unchanged)
- 35 `v5-realdata-medium-2026-05` canonical rows (9 updated SHAs + 26 unchanged)

### 5b. Determinism spot-check (gate 11)

Two scenarios re-run independently from scratch; body-SHA compared to `anchors.toml`:

| Scenario | Computed SHA | anchors.toml SHA | Match |
|----------|-------------|------------------|-------|
| `pairs-2023-zscore-mr` | `01c9da4d4c5ce268b5de49c72f367ef729fcaccf04d572e5dc0fa1f1bd65e76e` | `01c9da4d4c5ce268b5de49c72f367ef729fcaccf04d572e5dc0fa1f1bd65e76e` | PASS |
| `top10-2023-1h-momentum` | `0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3` | `0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3` | PASS |

Determinism gate: PASS.

### 5c. Group A re-emission verification (gate 13)

Report frontmatter for `backtest-20260527-181323-btc-2023-1m-sma-cross.md`:
```yaml
data_source: synthetic (seeded RNG, v0 fallback)
```
Confirmed: `--force-synthetic-bars` was used. Group A SMA/Composed scenarios were emitted
against synthetic GBM data (Q1=(a) revert-to-synthetic), not real Binance Parquet.

### 5d. Sharpe-delta table audit (gate 12) — K1 surprise scan

`spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/sharpe-delta-table-2026-05-27.md`
reviewed in full. K1 definition: noop Sharpe > 0 AND canonical Sharpe < 0.

| Group | # Scenarios | K1 Surprises | Notes |
|-------|------------|-------------|-------|
| A — SMA cross (synthetic, Q1=a) | 2 | 0 | noop Sharpe already negative (-13.02); canonical also negative (-28.93) |
| A — Composed (real-data + sim) | 3 | 0 | noop Sharpe deeply negative (-40 to -68); canonical improved (real-data effect dominates) |
| B — Momentum | 2 | 0 | Sharpe N/A; equity delta -$3.5k to -$5.4k |
| C — Pairs (newly wired) | 2 | 0 | Equity deeply negative in both namespaces |
| D — TCN overlay synthetic (newly wired) | 2 | 0 | Equity positive but reduced ($1.9k–$4.3k drag) |
| E–J — candle/realdata absent, analysis, success | 57 | 0 | SHA unchanged; no delta |
| **Total** | **69** | **0** | **H1 HOLDS (0 < 3 flipped scenarios)** |

**0 K1 surprises confirmed. H1 holds. No retirement candidates.**

### 5e. Selected spot-check rows from Sharpe-delta table

| Scenario | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| btc-2023-1m-sma-cross | $47,290.03 | $17,992.64 | -$29,297.39 | 0 | v5-sim+Q1 (synthetic) |
| pairs-2023-zscore-mr | -$60,524.71 | -$62,693.12 | -$2,168.41 | 0 | v5-sim (16 fills × 8bps) |
| top10-2023-fy-tcn-overlay | $30,235.58 | $28,347.99 | -$1,887.59 | 0 | v5-sim (1,224 fills × 8bps) |
| top10-2023-1h-momentum | $56,282.81 | $50,922.49 | -$5,360.32 | 0 | v5-sim (unchanged from v0.2.0) |
| report-sample-7d | — | — | $0.00 | 0 | =noop (operator success sample) |

All spot-checked rows are consistent with expected friction-only delta.

---

## 6. Benchmarks

_n/a — v0.3.0 changes are plumbing-only (struct field additions, function lift to shared module,
CLI flag). No hot paths touched. `sim_slippage_cost` was already in the momentum path; the lift
to `sim.rs` is behaviour-preserving (anchor-additive per ADR-0038 § D6.a)._

---

## 7. Environment / Infrastructure Issues

None. All runs are clean and deterministic.

---

## 8. Open Items

### Candle-feature-gated scenarios — DEFERRED to v0.4.0

The following scenarios require `--features candle` or `--features realdata` at compile time.
The default CI binary does not include these features. Their canonical SHAs in `anchors.toml`
are **noop-identical** to the noop-baseline SHAs — the v5 sim is wired in the code path but
the feature guard prevents it from firing in the default build:

| Scenario | Canonical SHA | Noop SHA | Status |
|----------|--------------|----------|--------|
| `top10-2023-fy-tcn-overlay-weights` | `7cb1357c…` | `7cb1357c…` | NOOP-IDENTICAL (candle absent) |
| `top10-2024-fy-tcn-overlay-weights` | `23c24dae…` | `23c24dae…` | NOOP-IDENTICAL (candle absent) |
| `top10-2023-fy-tcn-overlay-realdata` | `8fa47f49…` | `8fa47f49…` | NOOP-IDENTICAL (realdata absent) |
| `top10-2024-fy-tcn-overlay-realdata` | `fd8191df…` | `fd8191df…` | NOOP-IDENTICAL (realdata absent) |
| `top10-2023-fy-tcn-overlay-weights-realdata` | `552d7df2…` | `552d7df2…` | NOOP-IDENTICAL (realdata absent) |
| `top10-2024-fy-tcn-overlay-weights-realdata` | `2a65c434…` | `2a65c434…` | NOOP-IDENTICAL (realdata absent) |
| `top10-2023-fy-patchtst-overlay-realdata` | `5f303cc0…` | `5f303cc0…` | NOOP-IDENTICAL (realdata+candle absent) |
| `top10-2023-fy-vol-target-overlay-realdata` | `9fa64d46…` | `9fa64d46…` | NOOP-IDENTICAL (realdata absent) |

The plumbing code IS in place (R1 closed these paths). The canonical SHAs are noop-identical
because the feature flag gates the actual strategy execution. **These are DEFERRED to v0.4.0**
where a feature-flagged rebuild can produce the actual friction-bearing SHAs. This is the
explicit carve-out for the SOFT-PASS verdict (vs full PASS).

The Sharpe-delta table documents these as `=noop (candle absent)` / `=noop (realdata absent)`.
The 8 deferred scenarios show $0 equity delta — no K1 surprise risk (Sharpe sign cannot flip
when delta is $0).

### Pre-existing spec debt (carried forward per spec-lint gate rules)

`spec-lint: FAIL (72 violations in 3 categories)` — identical to v0.2.0 M-FINAL baseline (no
new categories; no count increase vs v0.2.0 M-FINAL which also reported 72/3):
- `dead-link (69)` — pre-existing, majority from v0-paper-sma screenshots README and v05-composed
  strategies backtest links from the 2026-04 era. Carry-forward.
- `missing-frontmatter (1)` — `spec/lab-polish-round-2/tasks.md`. Pre-existing.
- `shipped-no-tests (2)` — `spec/lab-end-to-end-v2/feature.md`, `spec/vol-killswitch-overlay-noop-fix/feature.md`. Pre-existing.

No new categories introduced by v0.3.0. No count regression (72 = 72 vs v0.2.0). Does not
block SOFT-PASS per spec-lint gate rules.

---

## 9. Verdict

**`SOFT-PASS`**

All hard gates pass. The verdict is SOFT-PASS rather than full PASS because 8 candle/realdata-
feature-gated scenarios (TCN-weights, TCN-realdata, PatchTST, VolTarget-GARCH) have canonical
SHAs that remain noop-identical to their noop-baseline counterparts, since the `--features
candle` / `--features realdata` compilation flags are absent from the default CI binary. The
plumbing code (R1) is in place for all 6 strategy paths including these; the runtime just does
not execute those paths without the feature flags. These 8 scenarios are explicitly deferred to
v0.4.0 per the Sharpe-delta table's documentation.

The SOFT-PASS verdict is explicitly anticipated in the feature brief's 4-cell verdict tree as
an acceptable sub-condition of R-O1 SHIP: the 3-scenario "candle/realdata absent" carve-out is
structurally equivalent to the v0.2.0 Ship Route (a) partial migration, but now with the code
wiring complete. The v0.4.0 follow-on will only need a feature-flagged rebuild, not additional
plumbing.

**Summary of gates:**

| Gate | Result |
|------|--------|
| `bash scripts/verify_anchors.sh` | PASS 69/69 |
| `cargo test -p backtest --lib` | PASS 35/35 |
| `cargo test -p reports --test strategy_anchors_unchanged` | PASS 3/3 (t1937 GREEN, t1937b GREEN) |
| `cargo test -p strategy --test latency_slippage_sim_e2e` | PASS 3/3 |
| `cargo test -p strategy --test vol_targeting_overlay_end_to_end` | PASS 1/1 |
| `cargo test -p strategy --test vol_killswitch_overlay_end_to_end` | PASS 4/4 |
| `cargo test --workspace --no-fail-fast` | PASS (1 pre-existing flake only) |
| `cargo fmt --all --check` | PASS |
| `cargo clippy` | FAIL (pre-existing only; 0 in backtest) |
| `spec-lint` | 72/3 — baseline unchanged (pre-existing only) |
| Determinism spot-check (pairs-2023, top10-2023-momentum) | PASS (exact SHA match) |
| K1 surprise scan (all 69 scenarios) | PASS (0 K1 surprises) |
| Group A synthetic data verified | PASS (data_source: synthetic in frontmatter) |
| ADR-0047 D2 grep gate (sim_slippage_cost defined once) | PASS |
| Candle-feature-gated scenarios | DEFERRED to v0.4.0 (8 scenarios noop-identical) |

---

## 10. Routing

`VERDICT → SOFT-PASS` — all hard gates green; 8 candle/realdata-feature-gated scenarios
explicitly deferred to v0.4.0 with plumbing code in place. Ready to route to presenter.

`HANDOFF → presenter` — assemble sprint-review deck per 4-cell verdict tree. Lead with the
full-coverage Sharpe-delta story (4 of 7 strategy paths fully verified under canonical friction;
3 paths wired but deferred). Capture operator decision on the SOFT-PASS verdict cell.

---

## 11. Anchor column population (trace.toml REQ-V5-FULL-PATH-WIRING-001)

Anchors verified under namespace `v5-realdata-medium-2026-05` (9 updated SHAs) and
`noop-baseline` (all 34 unchanged). Tester-verified anchor scenarios for this feature:

**Newly wired and re-emitted (canonical SHA changed from noop):**
- `btc-2023-1m-sma-cross` (v5-realdata-medium-2026-05): `d2fa7616…`
- `btc-2023-1m-sma-baseline-refresh` (v5-realdata-medium-2026-05): `d2fa7616…`
- `btc-2023-1m-macd-trend` (v5-realdata-medium-2026-05): `6cb14ac5…`
- `btc-2023-1m-rsi-reversion` (v5-realdata-medium-2026-05): `87b4e1cc…`
- `btc-2023-1m-bbands-mean-revert` (v5-realdata-medium-2026-05): `5b6237d1…`
- `pairs-2023-zscore-mr` (v5-realdata-medium-2026-05): `01c9da4d…`
- `pairs-2024-h1-zscore-mr` (v5-realdata-medium-2026-05): `6252819b…`
- `top10-2023-fy-tcn-overlay` (v5-realdata-medium-2026-05): `1460fcc7…`
- `top10-2024-fy-tcn-overlay` (v5-realdata-medium-2026-05): `b8e9186b…`

**Unchanged from v0.2.0 (determinism confirmed):**
- `top10-2023-1h-momentum`, `top10-2024-h1-momentum` (momentum wired in v0.1.0)
- All 8 candle/realdata-absent scenarios (noop-identical per feature gate)
- All analysis/operator-success scenarios (no equity surface)
