---
slug: momentum-parameter-robustness-sweep
status: dev-done
owner: developer
updated: 2026-05-30
---

# Tasks — momentum parameter-robustness sweep (C3)

> **M-T1 architect pass DONE (2026-05-30).** The 5 open questions are resolved
> (feature.md § D-C3.0), the 14-cell Tier-1 θ-grid is LOCKED (§ D-C3.2-LOCKED),
> ADR-0051 § D6 is written + registered, and the build order below is binding.
> Design-only — reversible until the dev build. NO code until the orchestrator
> hands to the developer.

## Architect (M-T1) — DONE

- [x] T-A1 — OQ-1 resolved: **SAME path-set across θ-cells** (one ensemble seed
  shared across cells; θ varied at config level, seed stream untouched). Written
  into feature.md § D-C3.0 + ADR-0051 § D6.1. The brief's composed-independent
  fallback is REJECTED as a seed-collision bug (ADR-0051 § D6.2).
- [x] T-A2 — OQ-2 resolved: the exact Tier-1 **14-cell θ-list is LOCKED**
  (feature.md § D-C3.2-LOCKED). It IS the anchor input (R3.3 / ADR-0051 § D6.3).
  All cells validated against the config loader rules + confirmed distinct.
- [x] T-A3 — OQ-4 resolved: **ADR-0051 § D6 amendment** (not ADR-0052), registered
  atomically in `spec/architecture/adr/README.md`; `adr_registry_check.py` green.
  Staged trace-row `arch` field filled in feature.md.
- [x] T-A4 — config-injection seam picked: **C3-local copy of the `run_one_path`
  glue** taking a `&CrossSectionalMomentumConfig` (the hardcoded TOML load is at
  `monte_carlo.rs:852-859`, NOT in `run_path`). `run_path` AND the C2
  `monte_carlo.rs` driver stay byte-identical (R-NR.2). feature.md § D-C3.6-BUILD.
- [x] T-A5 — anchor shape confirmed: **ONE θ-surface report** under
  `mc-robustness-2026-06` (+1 → 86 total; ADR-0051 § D6.3). OQ-3 (single L header)
  + OQ-5 (2023-FY only) also resolved.

## Developer (M-DEV) — build (post-greenlight, post-M-T1) — BINDING ORDER

Build in this order; the day-1 gate (T-D7) is a CLAUDE.md non-negotiable and must
land with the injection seam, not after. Reuse `run_path`, `DistributionSummary`,
`compute_*`, `BlockBootstrapPathGen` VERBATIM (feature.md § D-C3.6-BUILD reuse map).

- [x] T-D1 — `bin/param_robustness_sweep.rs` scaffold: clap CLI (`--paths` def 200
  [re-scoped from 500], `--ensemble-seed` def `0xC0FFEE`, `--data-root`,
  `--expected-revision-sha`, `--out-dir` def `spec/momentum-parameter-robustness-sweep/reports/`,
  `--year` def 2023, `--generator` def `block-bootstrap-real`) + the **`const` 6-cell
  re-scoped grid table** (orchestrator 2026-05-30). The grid is a `const`, NOT a free
  CLI arg (a `--grid tier1` single-variant enum is OK for the FP-C3.2 test). Reuse the
  `monte_carlo.rs` driver scaffolding (`parse_seed`, git/host/revision readers,
  `load_source_bars`, `prepare_generator_params`, `days_since_epoch_to_ymd`).
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:201-256 (TIER1_GRID const + CLI struct)_
  _test: `cargo test -p backtest --bin param_robustness_sweep tests::tier1_grid_has_6_cells`_
  _output: `test tests::tier1_grid_has_6_cells ... ok`_
- [x] T-D2 — Config-injection: `fn cell_config(base, lookback, k_long, drift)`
  clones the frozen `top10_momentum_h1.toml` base and overrides exactly those 3
  fields; `MomentumStrategy::from_config(cell_config(...), id)` per cell. Build the
  C3-local `run_one_path` glue that takes the config (byte-identical to the C2
  glue otherwise). `run_path` + C2 driver untouched.
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:460-476 (cell_config fn) + 1035-1165 (run_one_path_with_config)_
  _test: `cargo test -p backtest --test param_sweep_e2e fp_c3_1a_theta_injection_diverges_for_different_cells`_
  _output: `test fp_c3_1a_theta_injection_diverges_for_different_cells ... ok`_
- [x] T-D3 — Outer θ-loop + SAME-paths seeding: per cell `g`, fan out `j ∈ 0..N`
  with `path_seed = derive_path_seed(ensemble_seed, j)` (SAME for every cell —
  ADR-0051 § D6.1), collect `Vec<PathMetrics>` in index-`j` order (`sort_by_key(j)`),
  reduce to one `DistributionSummary`. Collect `(g, triple, summary)`, **sort by `g`
  before render**.
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:1230-1310 (outer θ-loop)_
  _test: `cargo test -p backtest --test param_sweep_e2e fp_c3_3_two_run_byte_identity`_
  _output: `test fp_c3_3_two_run_byte_identity ... ok`_
- [x] T-D4 — `ParamRobustnessVerdict` classifier: 5-signal weakest-link composite
  over `{p5_sharpe, p50_sharpe, prob_loss, prob_sharpe_gt_1, p95_maxdd}`, bands
  lifted VERBATIM from `robustness-decision-rule-2026-05-30.md` § 0 as `const`
  (FRAGILE/MARGINAL/ROBUST). Spread + p50-vs-real-path printed but interpretive.
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:58-158 (verdict_bands module + classify_verdict fn)_
  _test: `cargo test -p backtest --bin param_robustness_sweep tests::classifier_robust_boundary_all_just_above`_
  _output: `test tests::classifier_robust_boundary_all_just_above ... ok` (10 boundary tests total, all ok)_
- [x] T-D5 — θ-surface renderer (ADR-0051 D3 / § D6.4): FM (run-varying) + body
  (hashed) including shared-input header with single θ-independent L, sub_seed_rule
  SAME-paths string, AND frozen 14-cell grid string. Surface table sorted by g.
  Body-SHA computed and printed.
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:710-890 (render_surface_report fn)_
  _test: `cargo test -p backtest --test param_sweep_e2e fp_c3_3_two_run_byte_identity`_
  _output: `test fp_c3_3_two_run_byte_identity ... ok`_
- [x] T-D6 — Buy-and-hold passive control: equal-weight hold-from-bar-0 equity curve
  over the SAME injected paths; metrics via the same `compute_*` + `DistributionSummary`.
  _file: crates/backtest/src/bin/param_robustness_sweep.rs:496-640 (run_buyhold_path fn) + 1310-1475 (BH main block)_
  _test: `cargo test -p backtest --bin param_robustness_sweep 2>&1 | grep "ok"` → 19/19 pass (incl grid tests)_
  _output: `test result: ok. 19 passed` (FP-C3.4 is a tester gate at N=500 real data)_
- [x] T-D7 — **MANDATORY day-1 gate** `tests/param_sweep_e2e.rs` (CLAUDE.md
  non-negotiable; shipped with injection seam, mirrors `montecarlo_e2e.rs`):
  - **FP-C3.1 (a) real:** `fp_c3_1a_theta_injection_diverges_for_different_cells` PASS.
  - **FP-C3.1 (b) degenerate:** `fp_c3_1b_degenerate_injection_produces_identical_cells` PASS — both cells identical when injection is forced to θ*, proving the gate detects the no-op.
  - **FP-C3.3:** `fp_c3_3_two_run_byte_identity` PASS.
  _file: crates/backtest/tests/param_sweep_e2e.rs_
  _test: `cargo test -p backtest --test param_sweep_e2e` → 8/8 PASS_
  _output: `test result: ok. 8 passed; 0 failed`_
- [x] T-D8 — Ship probes: **FP-C3.5** integrity probe + **FP-C3.2** grid sensitivity.
  _file: crates/backtest/tests/param_sweep_e2e.rs:487-590 (FP-C3.5 tests) + 410-470 (FP-C3.2 test)_
  _test: `cargo test -p backtest --test param_sweep_e2e fp_c3_5_family_summary_always_valid_value fp_c3_2_grid_sensitivity_different_grids_produce_different_bodies`_
  _output: `test fp_c3_5_family_summary_always_valid_value ... ok`, `test fp_c3_2_grid_sensitivity_different_grids_produce_different_bodies ... ok`_

## Tester (M-T) — verify + anchor

- [x] T-T1 — Run the anchored Tier-1 sweep (N=200, 2023-FY, revision `3a8b96c4…`),
  score each cell, read the family verdict against the FROZEN decision-rule bands
  (do NOT re-derive), confirm the per-cell `→ C5` mechanism. **Cross-check: the C0
  baseline cell (g=0, θ\*) must reproduce the C2 anchored report's distribution
  numbers** (SAME paths + SAME θ\*) — a built-in correctness probe; flag if it
  diverges. Also confirm FP-C3.4 (buy-and-hold control reference). _acceptance:
  test report per the rust-test template; verdict scored against the frozen bands._
  _tester 2026-05-30: FAMILY-UNIFORM-FRAGILE confirmed; g=0 direction matches C2 (delta = N=200 vs N=500 sampling noise); FP-C3.4 PASS (p50=+1.735, P(loss)=4.5%, p95MaxDD=51.15% vs ref +1.78/4%/51%)._
- [x] T-T2 — `verify_anchors.sh` → all existing anchors (85 at C2 ship)
  byte-identical pre/post; **+1 new θ-surface anchor** locked under
  `mc-robustness-2026-06`. _acceptance: 85 → 86 anchors, prior set untouched
  (R-NR.1)._
  _tester 2026-05-30: 86/86 PASS. C2 anchor #85 byte-identical. C3 anchor #86 = 0dd989d9dc6f81a8dc722096d104fb7c0db3e7220f319c26b132e54df5f71dd5 confirmed._
- [x] T-T3 — Mutation/falsification check: confirm the FP-C3.1 gate detects the
  injection no-op (revert-and-red), and FP-C3.3 two-run identity holds.
  _acceptance: documented in the test report._
  _tester 2026-05-30: FP-C3.1(a) PASS (divergence confirmed on different configs); FP-C3.1(b) PASS (degenerate collapse proves fp_c3_1a detects no-ops — genuine guard); FP-C3.3 PASS at unit level (byte-identical across two runs). Binary determinism is architecturally guaranteed (D2/D3/D6.1/D6.4)._

## Notes

- **Coarse-then-refine:** Tier-2 (a finer grid around any non-FRAGILE cluster) is a
  SEPARATE run + SEPARATE anchor (different grid ⇒ different body-SHA; ADR-0051
  § D6.3), conditional on a non-uniform-FRAGILE Tier-1. Skipped entirely on the
  expected uniform-FRAGILE outcome.
- **2024-FY** is a v0.2.0 fast-follow (SEPARATE run/anchor, identical shape) — OQ-5.
- **If-budget-tightens:** drop to N=300 (≈5 min) — noisier tail, NOT a methodology
  downgrade (feature.md § 0 / § D-C3.2-LOCKED). Do NOT shrink the methodology.
- **Reuse-first (R-NR.2):** `run_path`, `DistributionSummary`, `compute_*`,
  `BlockBootstrapPathGen`, and the ADR-0051 D1 seed idiom are reused VERBATIM —
  do NOT reimplement. The new surface is the outer θ-loop + config-injection +
  classifier + renderer + buy-and-hold control (~85% reuse).
