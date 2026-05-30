---
slug: strategy-robustness-harness
status: in-progress
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

- [ ] **M-DEV-1 — lift `compute_*` calculators to `backtest::stats` (R-NR.5
  behaviour-preserving)** — _acceptance: `compute_sharpe_hourly` /
  `compute_sortino_hourly` / `compute_calmar` / `compute_max_drawdown_f64` /
  `compute_total_return` relocated VERBATIM from `bin/threshold_sweep.rs` into a
  new `crates/backtest/src/stats/mod.rs`; `bin/threshold_sweep.rs` imports them
  from `backtest::stats`. Same arithmetic, only the path changes. `cargo build -p
  backtest` clean; the `threshold_sweep` report bytes are unchanged (verify via
  the existing threshold-sweep determinism path if present)._

- [ ] **M-DEV-2 — `stats::DistributionSummary` reducer (ADR-0051 D2 — the f64
  boundary)** — _acceptance: `MetricDistribution { mean, std, p5..p95, min, max }`
  + `DistributionSummary { sharpe, sortino, calmar, max_drawdown, total_return,
  prob_loss, prob_sharpe_gt_0, prob_sharpe_gt_1, max_dd_tail_p50, max_dd_tail_p95 }`
  per § D-C2.6. Reduction is FROZEN: collect N metrics indexed by path index `j`,
  sequential left-fold mean, two-pass std, `f64::total_cmp` sort + NaN-absent
  assertion + type-7 linear percentile, integer-count probabilities. Carry the
  `// ADR-0051 D2: index-order reduction is load-bearing — do NOT parallelize`
  comment. Unit tests with a hand-checked small N (e.g. N=9, known percentiles)._

- [ ] **M-DEV-3 — `scenarios::montecarlo::run_path` cell wrapper (R1.2 / R-NR.2)** —
  _acceptance: `crates/backtest/src/scenarios/montecarlo.rs::run_path(input,
  fill_seed, strategy)` is a behaviour-preserving sibling of
  `scenarios::threshold_sweep::run_cell` (same bar-loop, same `PaperEngine::new`,
  same risk limits) typed to (or generic over) the momentum `Strategy` (NOT the
  TCN overlay) per § D-C2.3. `input.bars_override` carries this path's merged
  `Vec<Bar>`. Returns a result carrying `equity_curve: Vec<Decimal>`. Zero change
  to `PaperEngine`/`MatchingEngine`/scenario `run()`._

- [ ] **M-DEV-4 — `bin/monte_carlo.rs` driver + seed wiring (ADR-0051 D1)** —
  _acceptance: new bin mirrors `bin/threshold_sweep.rs` — dedicated rayon
  `sweep_pool`, fan out over `0..N`. `--ensemble-seed` (default `0xC0FFEE`);
  `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`
  bound to index `j` NOT completion order. Fill-tie-break seed HELD CONSTANT at
  `0xC0FFEE` across all paths (§ D-C2.4). `--generator {block-bootstrap-real |
  gbm-smoke}` (default block-bootstrap-real); `--paths N` (default 500). Per path:
  `C1::generate(universe, n_bars, path_seed_j)` → `ReplayFeed::merge_synthetic` →
  `run_path` → `compute_*` → collect `(j, metrics)`. Reduce via M-DEV-2. Source
  series = 2023-FY real Binance returns via `RealDataBarSource` (ADR-0032 revision
  pin); thread `revision_sha` into the body._

- [ ] **M-DEV-5 — `robustness-*.md` report renderer (ADR-0051 D3) + namespace
  (D4)** — _acceptance: front-matter (generated/wall_clock_s/host/pid/git_commit/
  data_revision_sha — NOT hashed) + body (master_seed, fill_seed, n_paths,
  sub_seed_rule, reduction_rule, generator, bootstrap_mode, block_length_policy,
  selected_block_length_L, source_revision_sha, param_set, per-metric table in the
  FIXED order sharpe/sortino/calmar/max_drawdown/total_return, prob-of-loss/PPSR/
  DD-tail block) — all hashed floats at `{:.6}` / `{:.2}%`. Written under
  `spec/strategy-robustness-harness/reports/`; rendered via
  `backtest::report_body_hash` contract. Add namespace `mc-robustness-2026-06` +
  the report-dir to `scripts/verify_anchors.sh` resolver (the routine additive
  extension; precedent ADR-0047 D5 / ADR-0045 D2)._

- [ ] **M-DEV-6 — the MANDATORY day-1 e2e gate (R-NR.6, CLAUDE.md non-negotiable)** —
  _acceptance: TWO e2e tests. (a) **Divergence**: single deterministic baseline-path
  backtest vs the N-path ensemble; assert `|p50_ensemble_sharpe −
  single_baseline_sharpe| ≥ epsilon` OR `(p95 − p5) ≥ epsilon` (catches the harness
  collapsing to one path — FP-C2.1). (b) **Determinism**: run the ensemble twice at
  the same `--ensemble-seed`, assert identical summary body-SHA (R3.1). Both green.
  Pattern reference `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`._

- [ ] **M-DEV-7 — anchored scenario run + non-regression (R3.2 / R-NR.1)** —
  _acceptance: run the v0.1.0 scenario — **cross-sectional momentum (v1) at its
  shipped θ* over N=500 block-bootstrap paths of 2023-FY real Binance returns** →
  one `robustness-*.md` under `mc-robustness-2026-06`. Run TWICE, confirm
  byte-identical body-SHA (the tester locks it). `bash scripts/verify_anchors.sh`
  → 84 existing anchors byte-identical pre/post + the new +1. **Report measured
  wall-clock in `§ Implementation`; if > ~10 min, fall back to N=200 and flag the
  operator** (Q-RH-1). Emit the `watch -n 10` probe per § D-C2.5._

- [ ] **M-DEV-8 — clippy/fmt + spec sync** — _acceptance: `cargo clippy -p backtest
  -- -D warnings` + `cargo fmt --check` clean; no `.unwrap()` in library code;
  fill `feature.md § Implementation` (as-built + measured wall-clock + chosen N) +
  the trace `crates`/`tests` columns (`REQ-STRATEGY-ROBUSTNESS-HARNESS-001`). The
  tester fills `anchors` after `verify_anchors.sh` PASS._

## Falsification probes (developer dry-run — § Design FP-C2.*)

Run each adversarially before declaring the gate green:

1. **FP-C2.1** force all `path_seed_j` to one constant ⇒ the M-DEV-6(a) divergence
   test MUST FAIL (spread → 0, p50 == single baseline). Proves the divergence gate
   is not itself a no-op. **Revert after.**
2. **FP-C2.2** two different momentum θ* at the same ensemble seed ⇒ DIFFERENT
   summary body-SHAs (anchor sensitivity — K3). A SHA that does not move means a
   distribution input is missing from the hashed body.
3. **FP-C2.3** same ensemble seed twice ⇒ identical body-SHA (the M-DEV-6(b) gate).
4. **FP-C2.4** `gbm-smoke` run ⇒ body says `generator: gbm-smoke` + NOT under the
   anchor namespace; `block-bootstrap-real` ⇒ `generator: block-bootstrap-real`
   (catches an accidental GBM-optimistic DD tail — K4).

## Notes

- **C1 is the hard dependency.** Do not start M-DEV-3/4 until C1's
  `BlockBootstrapPathGen` + `GbmPathGen` are re-exported from `crates/data`.
- **The reducer order is load-bearing** (ADR-0051 D2). Resist `par_iter().sum()` —
  it flaps the anchor on the canonical box.
- **N is a hashed input** — re-anchoring at a different N changes the SHA. Confirm
  N=500 (vs the N=200 fallback) with the operator after the wall-clock dry-run.
- Determinism scope = Apple-Silicon canonical box (ADR-0051 D5); cross-platform
  parity NOT contracted. `verify_anchors.sh` runs on the canonical box.
