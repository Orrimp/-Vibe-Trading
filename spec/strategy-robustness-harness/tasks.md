---
slug: strategy-robustness-harness
status: dev-done
owner: developer
updated: 2026-05-30
---

# Tasks — Strategy robustness harness (C2)

> Architect M-T1 task breakdown. Build **after C1**
> ([`monte-carlo-bootstrap-path-generator`](../monte-carlo-bootstrap-path-generator/tasks.md)) —
> C2 consumes C1's `BlockBootstrapPathGen` + `GbmPathGen`. Design clauses:
> [`feature.md` § Design](feature.md#design) (D-C2.1..D-C2.9). Contract:
> [ADR-0051](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)
> (D1 seed wiring, D2 reduction order, D3 report shape, D4 anchor unit, D5 scope).
> C2 is additive: a new bin + cell wrapper + a verbatim `compute_*` lift + the new
> reducer. **+1 anchor** under `mc-robustness-2026-06`; the 84 existing anchors
> stay byte-identical.

## M-DEV — build order

- [x] **M-DEV-1 — lift `compute_*` calculators to `backtest::stats` (R-NR.5
  behaviour-preserving)**
  - file:line: `crates/backtest/src/stats/mod.rs:40-167`
  - `bin/threshold_sweep.rs` imports from `backtest::stats` at line 233 (re-import block).
  - Test command: `cargo test -p backtest --lib -- stats`
  - Output: `test result: ok. 11 passed; 0 failed`

- [x] **M-DEV-2 — `stats::DistributionSummary` reducer (ADR-0051 D2 — the f64
  boundary)**
  - file:line: `crates/backtest/src/stats/mod.rs:172-403`
  - `MetricDistribution` + `DistributionSummary` + `reduce_samples` + `linear_percentile`.
  - ADR-0051 D2 load-bearing comment at line 321: `// ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize`.
  - Test command: `cargo test -p backtest --lib -- stats`
  - Output: `test result: ok. 11 passed; 0 failed` (includes hand-verified N=9 percentile test)

- [x] **M-DEV-3 — `scenarios::montecarlo::run_path` cell wrapper (R1.2 / R-NR.2)**
  - file:line: `crates/backtest/src/scenarios/montecarlo.rs:69-265`
  - Behaviour-preserving sibling of `run_cell`; typed to `MomentumStrategy` (NOT TCN overlay).
  - `fill_seed` held constant across paths per ADR-0051 D1.
  - Test command: `cargo test -p backtest --lib -- montecarlo`
  - Output: `test scenarios::montecarlo::tests::run_path_requires_bars_override ... ok`

- [x] **M-DEV-4 — `bin/monte_carlo.rs` driver + seed wiring (ADR-0051 D1)**
  - file:line: `crates/backtest/src/bin/monte_carlo.rs:1-960`
  - Dedicated rayon `mc_pool`, fan out over `0..N`, sort by `j` before reduction.
  - `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))` at line 150.
  - Fill-tie-break seed `FILL_SEED: u64 = 0xC0FFEE` held constant at line 186.
  - Test command: `cargo build --release -p backtest --features "candle realdata" --bin monte_carlo`
  - Output: `Finished release profile`

- [x] **M-DEV-5 — `robustness-*.md` report renderer (ADR-0051 D3) + namespace (D4)**
  - file:line: `crates/backtest/src/bin/monte_carlo.rs:237-443` (`render_report`)
  - Front-matter / body split per D3; fixed-precision formatting `{:.6}` / `{:.2}%`.
  - Namespace `mc-robustness-2026-06` added to `scripts/verify_anchors.sh` resolver at line 134.
  - Report written to `spec/strategy-robustness-harness/reports/`.
  - Test command: `bash scripts/verify_anchors.sh`
  - Output: `ANCHORS PASS  (85 / 85)`

- [x] **M-DEV-6 — the MANDATORY day-1 e2e gate (R-NR.6, CLAUDE.md non-negotiable)**
  - file:line: `crates/backtest/tests/montecarlo_e2e.rs:1-315` (6 tests)
  - (a) Divergence gate: `rn6a_divergence_gate_passes_with_distinct_seeds` (spread ≥ epsilon)
  - (a) FP-C2.1 falsifier: `fp_c2_1_degenerate_seeds_have_zero_spread` (degenerate spread ≈ 0)
  - (b) Determinism: `rn6b_two_run_byte_identity` (same seed → same formatted body)
  - Test command: `cargo test -p backtest --test montecarlo_e2e`
  - Output: `test result: ok. 6 passed; 0 failed`

- [x] **M-DEV-7 — anchored scenario run + non-regression (R3.2 / R-NR.1)**
  - file:line: `spec/strategy-robustness-harness/reports/robustness-20260530-112942-v1-momentum-2023-block-bootstrap-real-fy-mc.md`
  - Body SHA: `72fc7089c5f04885e8a2169d91c242a50e47b7820eea38b446a4dfaa2c1938c4` (two-run PASS).
  - Wall-clock: **183.5s on Apple-Silicon M-series** (N=500, rayon ~11.5 cores). Well under 10 min.
  - N=500 confirmed (Q-RH-1 = Option A, durable).
  - Test command: `bash scripts/verify_anchors.sh`
  - Output: `ANCHORS PASS  (85 / 85)`

- [x] **M-DEV-8 — clippy/fmt + spec sync**
  - `cargo fmt -p backtest --check` → zero diff
  - `cargo clippy -p backtest -- -D warnings` → 0 errors (new lib code; pre-existing test-path errors in engine/paths are baseline)
  - No `.unwrap()` in library code (only in `#[cfg(test)]` modules with explicit `#[allow]`)
  - `feature.md § Implementation` filled below
  - Test command: `cargo test -p backtest --lib && cargo test -p backtest --test montecarlo_e2e`
  - Output: `test result: ok. 58 passed; 0 failed` + `test result: ok. 6 passed; 0 failed`

## Falsification probes (developer dry-run — § Design FP-C2.*)

All four probes run and verified:

1. **FP-C2.1** DONE — `fp_c2_1_degenerate_seeds_have_zero_spread` asserts degenerate-seed spread < 1e-9.
   The inverse `rn6a_divergence_gate_passes_with_distinct_seeds` asserts real spread ≥ 0.01.
   Together: gate is falsifiable. (Integrated into `montecarlo_e2e` test suite — not a one-shot revert.)

2. **FP-C2.2** DONE — `fp_c2_2_anchor_sensitive_to_different_inputs` asserts different
   strategy inputs → different distribution summary (K3). Test in `montecarlo_e2e`.

3. **FP-C2.3** DONE — `rn6b_two_run_byte_identity` asserts same seed → byte-identical
   formatted body. Verified at full scale: two N=500 runs → SHA = `72fc7089...` both times.

4. **FP-C2.4** DONE — `fp_c2_4_generator_labels_are_distinct` asserts label strings are distinct.
   GBM smoke run produces `generator: gbm-smoke` body; block-bootstrap run produces
   `generator: block-bootstrap-real` body. GBM run is NOT under the anchor namespace.

## Notes

- **C1 is the hard dependency.** Do not start M-DEV-3/4 until C1's
  `BlockBootstrapPathGen` + `GbmPathGen` are re-exported from `crates/data`.
- **The reducer order is load-bearing** (ADR-0051 D2). Resist `par_iter().sum()` —
  it flaps the anchor on the canonical box.
- **N is a hashed input** — re-anchoring at a different N changes the SHA. N=500 confirmed.
- Determinism scope = Apple-Silicon canonical box (ADR-0051 D5); cross-platform
  parity NOT contracted. `verify_anchors.sh` runs on the canonical box.
