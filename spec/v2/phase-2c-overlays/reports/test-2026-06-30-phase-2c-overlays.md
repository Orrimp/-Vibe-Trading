---
title: Test Report
feature: phase-2c-overlays
run_id: 2026-06-30-1800-UTC
commit: 9433e35e9e2e8edb846fa65d37057063b1b9b905
agent: tester
verdict: PASS
---

# Test Report — Phase 2C Overlays — 2026-06-30 18:00 UTC

Three features verified as a coherent shipping unit: P1-5 (shared vol estimator),
P1-4 (vol-overlay reposition), P1-3 (drawdown-control overlay). ADRs 0078, 0079,
0080 all registered. Sequential gate protocol applied — no parallel cargo invocations.

## 1. Scope

- **Features / changes under test:**
  - **P1-5** — `crates/strategy/src/vol_estimator.rs`: shared, stateless σ̂ estimator
    (4 public functions + 3 λ constants, 23 inline unit tests). Consumed by P1-4.
  - **P1-4** — `crates/strategy/src/vol_targeting_overlay.rs` reparameterised: `VolSource`
    enum (Ewma/Garch), `no_trade_band`, `derisk_only`, `p1_4_defaults()`, `ReturnVolCorrelation`
    struct, `PerSymbolEwmaState`. 33 new unit tests (233→266). Honest framing: risk tool,
    not Sharpe tool.
  - **P1-3** — NEW `crates/strategy/src/drawdown_control_overlay.rs`: `DrawdownControlOverlay<S>`,
    normalised CPPI cushion multiplier, static 20%-floor (D8), load-bearing HWM restart.
    12 inline unit tests + 6 e2e integration tests (mandatory divergence gate included).
- **Spec refs:**
  - `spec/v2/advisor-vol-estimator/feature.md` + `tasks.md`
  - `spec/v2/advisor-vol-overlay-reposition/feature.md` + `tasks.md`
  - `spec/v2/advisor-drawdown-control-overlay/feature.md` + `tasks.md`
  - `spec/architecture/adr/0078-vol-targeting-overlay-reposition.md` (accepted 2026-06-30)
  - ADR-0079 registered inline in `spec/architecture/adr/README.md` table row 0079 (no
    separate .md file — developer registered it inline in the README as specified in the
    feature.md; `adr_registry_check.py` exits 0)
  - `spec/architecture/adr/0080-drawdown-control-overlay.md` (accepted 2026-06-30)
  - `spec/v2/v2-architecture.md` §1 P1-3/P1-4/P1-5 + §6.0 D5/D8
- **Commit SHA:** `9433e35e9e2e8edb846fa65d37057063b1b9b905`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin arm64 (macOS 26.5.1)`

## 2. Static Analysis

| Check                                       | Result | Notes                              |
|---------------------------------------------|--------|------------------------------------|
| `cargo fmt --check`                         | PASS   | No diffs emitted                   |
| `cargo clippy -p strategy --tests -- -D warnings` | PASS | 0 warnings                   |
| `cargo clippy -p backtest --tests -- -D warnings` | PASS | 0 warnings                   |
| `cargo audit`                               | n/a    | No dependency changes in this batch|
| `cargo deny`                                | n/a    | No dependency changes in this batch|

### fmt output (Gate 9)

```
(no output; exit 0)
```

### clippy strategy output (Gate 7)

```
Blocking waiting for file lock on build directory
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 35s
EXIT:0
```

### clippy backtest output (Gate 8)

```
Blocking waiting for file lock on build directory
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 03s
EXIT:0
```

## 3. Unit & Integration Tests

### Gate 1 — `cargo build --workspace`

```
Compiling strategy v0.1.0
Compiling backtest v0.1.0
Compiling agent v0.1.0
Compiling ui v0.1.0
Compiling trader v0.1.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.99s
EXIT:0
```

### Gate 2 — `cargo test -p strategy --lib`

```
running 266 tests
... (all 266 pass) ...
test result: ok. 266 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
EXIT:0
```

New tests verified passing in this run:

| Module                         | Test count | Examples verified                                                      |
|--------------------------------|------------|------------------------------------------------------------------------|
| `vol_estimator`                | 23         | `ewma_vol_monotone_weight_property`, `har_vol_weekly_monthly_smooth_spike`, `lambda_126d_hourly_half_life`, `log_returns_known_sequence`, `realized_vol_known_sample` |
| `drawdown_control_overlay`     | 12         | `multiplier_at_zero_drawdown_is_one`, `multiplier_at_floor_drawdown_is_zero`, `floor_never_moves_even_when_hwm_doubles`, `hwm_restart_preserves_upside_in_second_drawdown`, `quantity_scale_is_always_in_zero_to_one`, `update_equity_hwm_ratchets_on_new_high`, `update_equity_no_restart_hwm_stays_fixed`, `bars_total_counter_increments`, `telemetry_reflects_current_state` |
| `vol_targeting_overlay` (P1-4) | 33 new     | `no_trade_band_suppresses_small_change`, `derisk_only_blocks_upsize`, `return_vol_correlation_positive_series`, `return_vol_correlation_negative_series`, `ewma_vol_source_computes_sigma_after_warmup`, `p1_4_defaults_sets_expected_fields`, `pearson_identity_gives_one`, `pearson_negated_gives_minus_one` |

### Gate 3 — `cargo test -p strategy --test vol_targeting_overlay_end_to_end`

```
running 1 test
test overlay_quantity_scale_reflects_computed_factor ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT:0
```

**Backward-compatibility confirmed:** `VolTargetingConfig::default()` (Garch source,
`no_trade_band=0.0`, `derisk_only=false`) is byte-identical to pre-P1-4 behaviour.
Test uses `Default::default()` — NOT `p1_4_defaults()` — so the existing assertion
(`quantity_scale ≈ 2.0` after 5 GARCH bars) passes unchanged.

### Gate 4 — `cargo test -p strategy --test drawdown_control_overlay_end_to_end -- --nocapture`

```
running 6 tests
test floor_never_moves_static_cppi_d8 ... ok
test budget_cap_invariant_quantity_scale_max_one ... ok
test quantity_scale_before_update_returns_default_one ... ok
test hwm_restart_proof_benchmark_sequence ... ok
test multiplier_at_10pct_drawdown_is_correct ... ok
test overlay_equity_diverges_from_baseline_on_drawdown_scenario ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT:0
```

**Day-1 divergence gate (LOAD-BEARING — CLAUDE.md non-negotiable):**
`overlay_equity_diverges_from_baseline_on_drawdown_scenario` PASS. The test constructs
a drawdown scenario, runs `AlwaysLongStrategy` through `DrawdownControlOverlay`, and
asserts cumulative overlaid exposure diverges from the un-overlaid baseline by ≥ 1 bp.
Red-on-revert: replacing the overlay with a passthrough (`quantity_scale` = 1.0 constant)
causes this test to FAIL — the overlay cannot ship as a silent no-op.

**HWM-restart proof:**
`hwm_restart_proof_benchmark_sequence` PASS. The test encodes a BTC-style sequence
(peak → drawdown → recovery to new high → second drawdown) and verifies that `restart_on_hwm=true`
(the default) causes M to recover to 1.0 at the new ATH before de-risking in the second
drawdown. The Hsieh 2022 BTC benchmark (Sharpe 1.52 WITH restart vs −0.04 WITHOUT) is
the research foundation for this being non-optional (ADR-0080 D3, D8).

**D8 static floor invariant:**
`floor_never_moves_static_cppi_d8` PASS. After the HWM ratchets to 2× initial, the floor
remains `initial × 0.80` (static CPPI), NOT `2×initial × 0.80` (which would be TIPP —
deferred to v0.2 per operator decision D8).

**Budget-cap invariant:**
`budget_cap_invariant_quantity_scale_max_one` PASS. `quantity_scale` returns values in
`[0, 1]` at all times, ensuring the downstream `FixedFractionSizer::budget_cap` clamp
is never bypassed (ADR-0080 D5).

### Gate 5 — `cargo test -p backtest --lib` (FROZEN gate)

```
test result: ok. 195 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out; finished in 0.65s
EXIT:0
```

FROZEN gate identity proofs (targeted):

```
running 2 tests
test bakeoff::tests::turnover_does_not_change_ranking ... ok
test bakeoff::scorecard::tests::scorecard_does_not_change_ranking ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 201 filtered out; finished in 0.00s
EXIT:0
```

Both FROZEN gate tests PASS. The three overlays are sizing modifiers only — they do not
touch `rank_candidates`, `classify_verdict`, `verdict_bands`, or the scorecard crowning
logic. The FROZEN path is byte-untouched by construction.

### Gate 6 — `cargo test -p ui --lib`

```
test result: ok. 583 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
EXIT:0
```

Sanity check PASS. The three overlays have no UI surface in this increment — no new
crate edges, no `cargo tree -p ui` changes.

| Crate       | Passed | Failed | Ignored | Duration  |
|-------------|-------:|-------:|--------:|----------:|
| `strategy`  |    266 |      0 |       0 |   0.01 s  |
| `strategy` (vol_targeting e2e) | 1 | 0 | 0 | 0.00 s |
| `strategy` (drawdown e2e) | 6 | 0 | 0 | 0.00 s |
| `backtest`  |    195 |      0 |       8 |   0.65 s  |
| `ui` (lib)  |    583 |      0 |       0 |   0.81 s  |
| **Total**   | **1051** | **0** | **8** |          |

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — No proptest or cargo-fuzz suites added in this batch. The vol estimator's
`proptest` coverage is deferred per the feature spec.

## 5. Backtest Results

_n/a_ — All three features are report-only / sizing-only on the advisor bake-off path
(`write_report=false`). No new anchored report bodies are emitted. The overlays run as
`Strategy` wrappers in the forward-paper and advisor paths; they do not participate in
the `run_scenario` anchored backtest CLI path. The FROZEN gate (Gate 5) confirms the
bake-off crowning path is byte-untouched.

The research-validated backtest numbers that motivated the architecture decisions are
documented in the ADRs and feature specs (not re-run here — they are theoretical
benchmarks from Hsieh 2022 and Harvey et al., not outputs of this codebase's
`run_scenario` CLI):

| Scenario | Sharpe | Max DD | Source |
|---|---|---|---|
| CPPI with HWM restart (BTC 2020–2022) | 1.521 | 72%→20% | Hsieh 2022 [96]; ADR-0080 Context |
| CPPI without HWM restart (same period) | −0.043 | — | Same; proves restart is load-bearing |

## 6. Benchmarks

_n/a_ — All three modules operate on the advisor path (not the hot trading loop).
`vol_estimator` functions are pure stateless computations on small slices; `DrawdownControlOverlay`
caches the multiplier in a single `Decimal` field updated once per bar. No criterion
benchmarks exist or are needed for this batch.

## 7. Key Design Points (for presenter handoff)

### Formula correction (architect-ratified, ADR-0080 §D2)

The raw research formula `M(k) = (d_max − d(k)) / (1 − d(k))` does NOT satisfy
`M(0) = 1.0` at the all-time high (it yields `d_max / 1 = 0.20` instead of 1.0), so
exposure would be capped at 20% even when the portfolio is at peak — wrong operator
contract.

The implemented formula is the **normalised** version:

```
M(k) = (d_max − d(k)) / (d_max × (1 − d(k)))
```

Boundary conditions (verified by unit tests):
- `d(k) = 0.0` → M = 1.0 (full exposure at ATH) — correct
- `d(k) = d_max` → M = 0.0 (exposure shut at floor) — correct
- `d(k) > d_max` → M = 0.0 (clamped) — guarded
- `d(k) < 0.0` → M = 1.0 (clamped; equity above HWM) — guarded

This correction is ratified in `spec/v2/v2-architecture.md` §1 P1-3 and ADR-0080 §D2.

### HWM restart is load-bearing

The `restart_on_hwm = false` configuration is available only for testing and reproduces
the Hsieh 2022 failure mode (Sharpe −0.04 on BTC). The test
`hwm_restart_proof_benchmark_sequence` verifies the default (`true`) causes correct
recovery behaviour. Setting `restart_on_hwm = false` in production is NOT recommended
(ADR-0080 D3 + Consequences section).

### Operator D8 promise: never lose more than 20%

`floor = initial_equity × 0.80` is computed once at construction and NEVER changes
(static CPPI). The `floor_never_moves_static_cppi_d8` test is the contract proof.
TIPP / ratcheting floor is deferred to v0.2 (operator decision D8, 2026-06-30).

Note: the floor guarantee is probabilistic on a gapping asset — a crypto jump larger
than the cushion between bars can still breach the floor. This is disclosed in the
feature.md "Honest operator promise" section.

### Vol-overlay honest framing (P1-4)

The overlay is repositioned from "Sharpe tool" to "risk tool" based on the research
finding that crypto's leverage effect is reversed (γ = −0.261 vs equities' +0.115,
Brini–Lenz 2024). The Sharpe-gain mechanism requires negative ρ(return, vol); on crypto
the correlation is typically positive (FOMO: vol rises after rallies). The `ReturnVolCorrelation`
struct gives the operator a per-symbol, per-window answer to whether the Sharpe mechanism
is even mechanistically present.

### Anchor safety

All three overlays run on the advisor path with `write_report=false` → no anchored
report body is ever written → 119/119 anchors unchanged by construction (verified Gate 10).

## 7a. Environment / Infrastructure Issues

Cargo artifact lock contention observed between background tasks. All gates resolved by
running sequentially in the foreground (per SEQUENTIAL CARGO ONLY protocol stated in
the task brief). No actual build failures.

_No flaky tests. No data gaps. No infra outages._

## 8. Spec-Lint Gate

Gate 11 — `python3 scripts/spec_lint.py`:

```
spec-lint: PASS (0 violations)
```

No new spec-lint violations. Pre-existing baseline violations: none (0 total).

## 9. Anchor-Verification Gate

Gate 10 — `bash scripts/verify_anchors.sh` (run before any spec/ edits):

```
ANCHORS PASS  (119 / 119)
```

All 119 anchors PASS. The three overlays do not touch any anchored report body.
`write_report=false` on the advisor path is the structural guarantee — not a
runtime-flag assertion.

## 10. ADR Registry Gate

Gate 12 — `python3 scripts/adr_registry_check.py --self-test`:

```
test_case1_missing_row ... ok
test_case2_updated_not_bumped ... ok
test_case3_status_out_of_enum ... ok
test_case4_exclude_rule ... ok
test_case5_clean ... ok
Ran 5 tests in 0.005s  OK
EXIT:0
```

Production check: `python3 scripts/adr_registry_check.py` exits 0.

ADR status:
- **ADR-0078** (`spec/architecture/adr/0078-vol-targeting-overlay-reposition.md`) — accepted, file present
- **ADR-0079** (vol estimator / P1-5) — registered as inline table row in `spec/architecture/adr/README.md` (no separate .md file; `adr_registry_check.py` exits 0; this is consistent with the feature.md annotation "ADR-0079 reserved — written/registered atomically when P1-3 drawdown overlay lands")
- **ADR-0080** (`spec/architecture/adr/0080-drawdown-control-overlay.md`) — accepted, file present

## 11. Verdict

**`PASS`**

All 12 gates pass. 1051 tests pass across `strategy`, `backtest`, and `ui` with 0
failures. The three Phase 2C overlays (P1-5 vol estimator, P1-4 vol-overlay reposition,
P1-3 drawdown-control overlay) are correct, non-regressive, and non-silent:

- The **day-1 divergence gate** (`overlay_equity_diverges_from_baseline_on_drawdown_scenario`)
  is PASS — the drawdown-control overlay demonstrably changes equity vs the un-overlaid
  baseline. This is the CLAUDE.md non-negotiable rooted in the v3-vol-overlay-noop
  precedent (2026-05-22).
- The **HWM-restart proof** (`hwm_restart_proof_benchmark_sequence`) is PASS — the
  load-bearing restart behaviour is verified to cause M to recover to 1.0 at new ATH.
- The **FROZEN gate** (`scorecard_does_not_change_ranking`, `turnover_does_not_change_ranking`)
  is PASS — crowning logic is byte-untouched by sizing-only overlays.
- Anchors 119/119 PASS — no anchored report body emitted by any of the three overlays.
- spec-lint PASS (0 violations).

The batch is shippable. Phase 2C is closed.

## 12. Routing

`VERDICT → PASS` — Phase 2C overlay layer complete; ready for Phase 2D (cost-model opt-in,
DATA-quality hardening, narration-faithfulness gate, no-alpha-gate CI).

---

```toml
[handoff]
from        = "tester"
to          = "presenter"
feature     = "phase-2c-overlays"
trace_refs  = [
  "REQ-V2-P1-5-VOL-ESTIMATOR-001",
  "REQ-V2-P1-4-VOL-OVERLAY-REPOSITION-001",
  "REQ-V2-P1-3-DRAWDOWN-OVERLAY-001",
]
verdict     = "PASS"
priority    = "high"

[inputs]
brief       = "spec/v2/advisor-vol-estimator/feature.md + spec/v2/advisor-vol-overlay-reposition/feature.md + spec/v2/advisor-drawdown-control-overlay/feature.md"
artifacts   = [
  "crates/strategy/src/vol_estimator.rs",
  "crates/strategy/src/vol_targeting_overlay.rs",
  "crates/strategy/src/drawdown_control_overlay.rs",
  "crates/strategy/tests/drawdown_control_overlay_end_to_end.rs",
  "crates/strategy/tests/vol_targeting_overlay_end_to_end.rs",
  "spec/architecture/adr/0078-vol-targeting-overlay-reposition.md",
  "spec/architecture/adr/0080-drawdown-control-overlay.md",
]

[outputs]
spec_files      = ["spec/v2/phase-2c-overlays/reports/test-2026-06-30-phase-2c-overlays.md"]
lint_result     = "spec-lint: PASS (0 violations)"
anchors_result  = "ANCHORS PASS (119 / 119)"
adr_result      = "adr_registry_check.py --self-test: OK (5/5)"

[open_questions]
items = [
  "ADR-0079 has no separate .md file — only a README table row. This is consistent with the developer's approach and exits 0 in the checker, but a future audit sweep may want to materialise it as a standalone file for consistency with ADR-0078/0080.",
]

[assumptions]
items = [
  "All three overlays are sizing-only and never emit anchored reports (write_report=false). Anchor safety is structural, not conditional.",
  "The research benchmark numbers in Section 5 (Sharpe 1.52 / −0.04) are from Hsieh 2022 and are documentation inputs to the ADR, not outputs of run_scenario — no re-run is owed.",
]
```
