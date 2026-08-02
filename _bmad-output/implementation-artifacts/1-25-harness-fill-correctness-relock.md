# Story 1.25: harness-fill-correctness-relock

Status: ready-for-dev

<!-- Created 2026-07-31 by the 1-14 code-review decision (operator: new critical story + program).
     CRITICAL priority. Runs as ONE re-lock program with 1-24-pwsd-fidelity-relock (same
     namespace ceremony, one re-run budget, one errata). Disclosure of record: bug-log #67. -->

## Story

As the operator of the Honest Advisor,
I want the research-harness lanes' fill arithmetic corrected (orders priced at their own symbol's bar, never the trigger bar's) and the affected anchored evidence formally regenerated under a re-lock,
so that the C2/C3 research verdicts rest on real execution arithmetic — with the migration honest, the old rows frozen as history, and the verdict re-derivation loud.

## Acceptance Criteria

1. **Fill-symbol correctness**: a batch stepped through the harness lanes prices each order at ITS symbol's bar for that timestamp (or defers the order to that symbol's bar); `PaperEngine::step` either enforces `order.symbol == bar.symbol` (typed reject) or the harness callers route per-symbol — architect chooses the seam with the LIVE cockpit paths regression-proven byte-identical (the Lab/advisor flows step single-symbol batches; prove no behavior change there via the existing suites + a dedicated same-bytes test).
2. **Both lanes**: `scenarios/montecarlo.rs::run_path` AND `threshold_sweep.rs::run_cell` fixed (run_cell additionally gains the Bug-B solvency guard it never received); the FROZEN gate files stay byte-untouched (identity test discharged).
3. **The riders land in the same regeneration**: √8760 annualization (or a formal ratification of √8575 with the doc corrected to match), hashed-body verdict vocabulary aligned to the frozen 5-signal rule, sentinel-zero pooling policy, negative-final Calmar guard, slippage-aware solvency pre-flight, FILL_SEED domain separation, real portfolio-exposure-cap enforcement decision (enforce or delete the decorative limit) — each either fixed or explicitly ratified-as-is in the story record.
4. **Formal re-lock per the ADR-0038/0045 §D6 family**: affected reports regenerated under a NEW namespace; old rows byte-frozen; an errata/decision note records per-scenario old-vs-new headline numbers and the RE-DERIVED verdicts. The C2 FRAGILE and the sweep FAMILY-UNIFORM-FRAGILE conclusions are re-stated from clean arithmetic — whichever way they land. Any flip that would touch the era-qualified thesis's supporting narrative escalates to the operator BEFORE publication (AD-19 spirit).
5. Standing floor: anchors green (old + new rows); spec-lint PASS; advisor-gate independence re-proven (bakeoff/bootstrap.rs resamples returns — assert its inputs/outputs unchanged by this story).

## Tasks / Subtasks

- [ ] Architect: fill-correctness seam decision (engine guard vs caller routing) + re-lock plan (namespace, report inventory incl. threshold-sweep lanes, compute budget) — coordinate with 1-24 as one program.
- [ ] Dev: fixes per AC1-AC3 + the same-bytes live-path proof.
- [ ] Re-run + re-lock + errata + verdict re-derivation (AC4).
- [ ] Review: old rows intact, new rows complete, verdict-delta table honest, advisor-gate independence proof (AC5).

## Dev Notes

- Origin: 1-14 review Critical (Blind Hunter; orchestrator-verified at paper.rs:118-136 + montecarlo.rs:274+ + run_cell) — full evidence in `1-14-strategy-robustness-harness.md` § Review Findings; disclosure bug-log #67.
- **Advisor gate PROVEN unaffected** (verified 2026-07-31): `bakeoff/bootstrap.rs` resamples log-returns from candidate equity curves — no fill re-execution. Crowns/verdicts/ship-passive rest on it, not on the harness lanes.
- Blast radius: research-evidence class only (C2 monte_carlo lane, C3 threshold-sweep lanes and their anchored namespaces).
- Do-not-build register: not implicated (gate-credibility maintenance; no new alpha surface).
- Direction honesty: Blind's assessment — cleaning the noise plausibly shows MORE fragility, not less — is a hypothesis, not a result; AC4's escalation clause is the guard.

### References

- Trace: `REQ-HARNESS-FILL-CORRECTNESS-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
