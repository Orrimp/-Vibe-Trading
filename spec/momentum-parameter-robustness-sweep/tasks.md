---
slug: momentum-parameter-robustness-sweep
status: draft
owner: analyst
updated: 2026-05-30
---

# Tasks — momentum parameter-robustness sweep (C3)

> **Stub authored by the analyst (M0 scoping).** The architect re-owns and
> re-orders this list at M-T1 into a binding build order AFTER the operator
> greenlights C3 over a pivot/C5. Tasks below are the analyst's view of the work
> implied by the brief, not a locked plan. NO code until greenlight.

## Architect (M-T1) — resolve before any build

- [ ] T-A1 — Resolve OQ-1: same-vs-independent paths across θ-cells (analyst
  leans SAME — one shared ensemble seed, θ the only varying input). _acceptance:
  the chosen seed-composition rule is written into the brief Design + the ADR
  amendment._
- [ ] T-A2 — Finalize OQ-2: lock the exact Tier-1 θ-cell list (~12-16
  hypothesis-aimed cells from §D-C3.2). _acceptance: cell list frozen in the
  brief; it IS the anchor input (R3.3)._
- [ ] T-A3 — Resolve OQ-4: ADR-0051 amendment vs new ADR-0052 for the two-axis
  seed composition + θ-surface shape (analyst recommends amendment). _acceptance:
  ADR Changelog entry OR ADR-0052 drafted; arch[] populated in the trace row._
- [ ] T-A4 — Pick the least-invasive config-injection seam that keeps `run_path`
  byte-identical (R-NR.2): refactor `run_one_path` to accept a caller-supplied
  config vs a C3-local path-runner glue. _acceptance: design names the seam; no
  edit to `scenarios::montecarlo::run_path` body._
- [ ] T-A5 — Confirm anchor shape = ONE θ-surface report under
  `mc-robustness-2026-06` (D-C3.5). _acceptance: stated in design + tasks._

## Developer (M-DEV) — build (post-greenlight, post-M-T1)

- [ ] T-D1 — Outer θ-grid enumerator + `bin/param_robustness_sweep.rs` CLI (grid
  def, N, ensemble-seed, out-dir, year). _acceptance: enumerates the locked
  Tier-1 cells; cell index bound to `g`, never completion order._
- [ ] T-D2 — Config-injection (in-memory `CrossSectionalMomentumConfig` variants
  → `MomentumStrategy::from_config` per cell). _acceptance: each cell runs its
  OWN θ; proven by T-T1._
- [ ] T-D3 — Two-axis sub-seeding per the M-T1 rule (compose ADR-0051 D1).
  _acceptance: `(g, j)`-pure seeds; FP-C3.3 two-run identity green._
- [ ] T-D4 — `ParamRobustnessVerdict` composite classifier (5-signal
  weakest-link, decision-rule §4 bands verbatim). _acceptance: unit tests at the
  band boundaries; FRAGILE/MARGINAL/ROBUST per cell._
- [ ] T-D5 — θ-surface report renderer (one report; G rows + buy-and-hold
  control row + family summary line + per-cell `→ C5` flags; sort-before-render).
  _acceptance: ADR-0051 D3 FM/body split + fixed precision; byte-identical body
  across 2 runs._
- [ ] T-D6 — Buy-and-hold passive control row over the same paths. _acceptance:
  FP-C3.4 reproduces the adversarial-review reference (p50 ≈ +1.78)._
- [ ] T-D7 — **MANDATORY day-1 gate** `tests/param_sweep_e2e.rs`: FP-C3.1
  (θ-divergence / anti-no-op, tested on BOTH real + degenerate-injection) +
  FP-C3.3 (two-run byte-identity). _acceptance: CLAUDE.md non-negotiable met;
  the gate goes RED when injection is forced to a no-op._
- [ ] T-D8 — FP-C3.5 integrity probe: assert NO "best θ is ROBUST" claim is
  emitted (family line ∈ {UNIFORM-FRAGILE, HAS-NON-FRAGILE-CELLS}; non-FRAGILE
  cells carry `→ C5`). _acceptance: the pre-registration commitment is enforced
  in code._

## Tester (M-T) — verify + anchor

- [ ] T-T1 — Run the Tier-1 sweep (N=500), score each cell, confirm the family
  verdict + (if any) the C5-flag mechanism. _acceptance: test report per the
  rust-test template; verdict read against the frozen bands._
- [ ] T-T2 — `verify_anchors.sh` → all existing anchors byte-identical; +1 new
  θ-surface anchor locked. _acceptance: 85 → 86 anchors, prior set untouched
  (R-NR.1)._
- [ ] T-T3 — Mutation/falsification check: the FP-C3.1 gate detects the
  injection no-op (revert-and-red). _acceptance: documented in the test report._

## Notes

- **Coarse-then-refine:** Tier-2 (finer grid around any non-FRAGILE cluster) is a
  SEPARATE run + SEPARATE anchor, conditional on a non-uniform-FRAGILE Tier-1.
  Skipped entirely on the expected uniform-FRAGILE outcome.
- **If-budget-tightens:** drop to N=300 / the 9-cell `lookback × k_long` core
  (NOT a weaker methodology) — see brief §0 / §D-C3.2.
- **Reuse-first:** `run_path`, `DistributionSummary`, `compute_*`,
  `BlockBootstrapPathGen` are reused verbatim — do NOT reimplement (R-NR.2).
