# Story 1.14: strategy-robustness-harness

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the distribution-summary backtest mode (Sharpe p5/p50/p95, drawdown tail, probability-of-loss) read against the frozen p5-Sharpe<0 -> FRAGILE rule,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `dev-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the distribution-summary backtest mode (Sharpe p5/p50/p95, drawdown tail, probability-of-loss) read against the frozen p5-Sharpe<0 -> FRAGILE rule.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-07-31 (burn-down 4 of 14; commits a0b986c/b58984f/2821bc6; layers: Blind 16, Edge 9, Auditor 8 — deduped to 19).
     Gates re-run THIS session: `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0 violations)`; independent leg: montecarlo_e2e 9/9, stats 27/27.
     ORCHESTRATOR-VERIFIED (read paper.rs + run_path + bakeoff/bootstrap.rs directly): the Critical is REAL and its blast radius is EXACTLY the research-harness lanes; the ADVISOR GATE IS UNAFFECTED — bakeoff/bootstrap.rs resamples log-returns from candidate equity curves and never re-executes fills. Crowns/verdicts/ship-passive stand.
     OPERATOR DECISIONS 2026-07-31: Critical cluster → NEW story 1-25 + coordinated re-lock program with 1-24; patches → apply all 15. -->

- [x] [Review][Decision] **CRITICAL cluster → DECIDED: story 1-25-harness-fill-correctness-relock (+ program with 1-24).** (a) Cross-symbol fill mispricing: `PaperEngine::step` prices every order at the stepped bar's close with no `order.symbol == bar.symbol` check [crates/backtest/src/paper.rs:118-136]; `run_path` steps cross-symbol momentum batches against the single trigger bar [crates/backtest/src/scenarios/montecarlo.rs:274+] → BTC filled at ADA's price; the C2 anchored distribution's shape (81% median MaxDD on a ≤30%-exposure book, compressed Sharpe band, P(loss) 75.2%) is execution-artifact noise; same pattern in the older anchored `threshold_sweep::run_cell` lane (which ALSO retains the pre-Bug-B unguarded Buy sizing — Auditor F7). (b) Sharpe/Sortino annualization constant √8575 vs the documented √8760 (~1.06% systematic understatement; later ratified-in-guard-test but doc-contradicted) [crates/backtest/src/stats/mod.rs:42,72]. (c) Hashed-body verdict vocabulary (WEAK/MARGINAL/ROBUST-ABOVE-1 on p50) contradicts the frozen 5-signal rule (p5<0 ⇒ FRAGILE) read at the gate layer [bin/monte_carlo.rs:359-364]. (d) Five smaller anchor-tagged items ride along: sentinel-zero pooling in degenerate-path metrics; negative-final Calmar NaN; slippage-blind solvency pre-flight (layer-3 as routine path near the boundary); FILL_SEED==default master seed domain collision (inert today); decorative portfolio-exposure cap (empty Position snapshots). ALL anchor-impacting → the fixes + regeneration + namespace re-lock + C2/C3 verdict RE-DERIVATION are story 1-25's scope, coordinated with 1-24 as ONE re-lock program. Disclosure of record: bug-log #67.
- [x] [Review][Patch] R-NR.6(a)/(b) + FP-C2.1 e2e gates exercise a synthetic stand-in (`fake_equity_curve` + test-local GBM), never the production fan-out — a `run_one_path` seed-wiring bug passes every test. Re-point at the real harness chain (small-N real fan-out: distinct seeds → spread; same seed twice → byte-identity) [crates/backtest/tests/montecarlo_e2e.rs:217-315].
- [x] [Review][Patch] `fp_c2_4_generator_labels_are_distinct` asserts two test-local string literals — vacuous (#66 class). Assert `GeneratorKind::label()` values (or a rendered body line) instead [montecarlo_e2e.rs:374].
- [x] [Review][Patch] `solvency_invariant_equity_curve_never_negative_across_paths` asserts non-negativity of fixtures that are non-negative by construction — cannot go red. Re-point at real `run_path` output or delete in favor of Gate-2 [montecarlo_e2e.rs:437].
- [x] [Review][Patch] `solvency_guard_arithmetic_unit_test` re-implements the guard inside the test (tautology; superseded by Gate-2) — delete or convert to a call into production code [montecarlo_e2e.rs:481].
- [x] [Review][Patch] `reduce_samples` NaN gate admits ±infinity → inf mean, NaN std printed into the body (release strips the debug_asserts) — reject non-finite with a typed error (current anchored inputs all finite → bodies unchanged) [crates/backtest/src/stats/mod.rs:518-536]. Add the metric NAME to `DistributionError::NanValue` while there [stats/mod.rs:427-431].
- [x] [Review][Patch] GbmSmoke bin branch inlines the anti-diagonal seed collision fixed in `data::GbmPathGen` (1-13 pass) — delegate or splitmix-mix; also separate the source-bar seed base from D1 path seeds [bin/monte_carlo.rs:834, :638].
- [x] [Review][Patch] `linear_percentile` unguarded outside its domain (empty slice → usize underflow OOB; p>100 → OOB) — release-mode guards [stats/mod.rs:581-595].
- [x] [Review][Patch] `--year` silently maps unmapped years to 8760 bars (leap-wrong, mislabeled reports) — bail on unsupported years [bin/monte_carlo.rs:407-410].
- [x] [Review][Patch] `--paths` accepts 0 (dies late with a misleading reducer error) / 1 (degenerate "distribution") / absurd N (unbounded alloc) — validate ≥ 2 + sane cap [bin/monte_carlo.rs:104-105,439].
- [x] [Review][Patch] GBM smoke reports are named `…-block-bootstrap-gbm-fy-mc` (no bootstrap in that lane) into an anchor-resolver-globbed dir — honest scenario naming [bin/monte_carlo.rs:514-518].
- [x] [Review][Patch] Solvency "layer 1" notional cap is dead code (capped buy always fails the pre-flight; no downsized buy can execute) — remove the dead layer + correct the three-layer doc (byte-neutral) [scenarios/montecarlo.rs:311-321].
- [x] [Review][Patch] Front-matter timestamp and filename read the clock twice (can straddle midnight) — single read [bin/monte_carlo.rs].
- [x] [Review][Patch] Stale "Behaviorally-preserving sibling" header on montecarlo.rs — false since the v0.1.1 solvency guard [scenarios/montecarlo.rs:1-4].
- [x] [Review][Patch] Trace row lags Gate-2: the 9th test (`solvency_guard_run_path_regression_negative_cash_prevented`) missing from the tests list; state comment still records the pre-Gate-2 gap [trace.toml REQ-STRATEGY-ROBUSTNESS-HARNESS-001].
- [x] [Review][Patch] Register entry for the `run_cell` unguarded-Buy parity question (Auditor F7): the pattern Bug-B fixed in montecarlo.rs survives in the frozen-anchored threshold-sweep lane — record in the do-not-build/known-limits context pointing at 1-25 [docs/dev-notes/do-not-build-register.md or bug-log #67].
- [x] [Review][Defer] K3/FP-C2.2 θ*-sensitivity implemented as a proxy (inverted curves, no body-hash variation); structural mitigation (param_set in hashed body) stands — proper θ*-variation e2e belongs to the 1-25 re-lock's re-verification suite. Owner: 1-25. Revisit: at 1-25 close.
- [x] [Review][Defer] Presenter deck git-history-only (ratified CLEANUP deletion; honest mapping) + R-NR.4 scoped-clippy historical disclosure — provenance notes, no action. Owner: n/a. Revisit: n/a (recorded here).

Dismissed as noise (0 substantive; solvency Bug-B itself not re-reported — fixed in-lane by b58984f and Gate-2-regression-tested).

- [x] `strategy-robustness-harness` 0.1.1 - the base feature (dev-done; leg formally closed by the 2026-07-31 code-review PASS-WITH-FINDINGS)

## Dev Notes

- Source feature folder: `spec/v1/strategy-robustness-harness/` - frontmatter status **`dev-done`** (verbatim), version `0.1.1`, updated `2026-06-17`.
- Status mapping: `dev-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/strategy-robustness-harness` (full narrative); reports under `evidence/v1/strategy-robustness-harness/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-STRATEGY-ROBUSTNESS-HARNESS-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-07-31, orchestrator)

All 15 anchor-safe patches APPLIED (dev subagent died on the session limit
after most edits; orchestrator finished the register entry + 4 clippy lints
inline) and independently verified. Literal gates THIS session: `ANCHORS PASS
(119 / 119)` · `spec-lint: PASS (0 violations)` · fresh `cargo clippy -p
backtest -- -D warnings` = 0 · montecarlo_e2e 9 → **12/12** (the R-NR.6/FP-C2
gates now drive the REAL fan-out via the new `mc_harness.rs` seam — verbatim
extraction, byte-parity argued in-module; `gbm_sym_seed_no_anti_diagonal_collision`
proves the smoke-lane seed fix; fp_c2_4 asserts real `GeneratorKind::label()`s)
· stats 27 → 31/31 (±inf reject + metric-named errors + percentile guards) ·
full crate 40 suites ok. The Critical cluster is OWNED: story 1-25 (one
re-lock program with 1-24), bug-log #67 disclosure, do-not-build register
entry against drive-by fixes. Advisor-gate independence verified in-session
(bakeoff resamples returns, never fills).
