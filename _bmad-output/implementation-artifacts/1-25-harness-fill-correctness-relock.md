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
5. **Band re-examination (product-review finding 2, 2026-08-04):** `classify_verdict` /
   `verdict_bands` thresholds were calibrated against the very surfaces this story
   regenerates. After the re-derivation, the story must state explicitly — with numbers —
   whether the frozen bands would classify the CLEAN surfaces the same way they classified
   the contaminated ones. The gate stays byte-frozen either way (AD-1); the deliverable is
   the ANSWER, recorded in the errata, plus an operator escalation if the clean numbers sit
   near a band edge. A frozen gate whose calibration is never re-examined is an assumption,
   not a guarantee.
6. Standing floor: anchors green (old + new rows); spec-lint PASS; advisor-gate independence re-proven (bakeoff/bootstrap.rs resamples returns — assert its inputs/outputs unchanged by this story).

## Tasks / Subtasks

- [ ] Architect: fill-correctness seam decision (engine guard vs caller routing) + re-lock plan (namespace, report inventory incl. threshold-sweep lanes, compute budget) — coordinate with 1-24 as one program.
- [ ] Dev: fixes per AC1-AC3 + the same-bytes live-path proof.
- [ ] Re-run + re-lock + errata + verdict re-derivation (AC4).
- [ ] Review: old rows intact, new rows complete, verdict-delta table honest, advisor-gate independence proof (AC5).

## Dev Notes

- Origin: 1-14 review Critical (Blind Hunter; orchestrator-verified at paper.rs:118-136 + montecarlo.rs:274+ + run_cell) — full evidence in `1-14-strategy-robustness-harness.md` § Review Findings; disclosure bug-log #67.
- **Advisor gate PROVEN unaffected** (verified 2026-07-31): `bakeoff/bootstrap.rs` resamples log-returns from candidate equity curves — no fill re-execution. Crowns/verdicts/ship-passive rest on it, not on the harness lanes.
- Blast radius: research-evidence class only (C2 monte_carlo lane, C3 threshold-sweep lanes and their anchored namespaces).
- **Inventory extension (1-15 review, 2026-07-31):** anchor #86 (`v1-momentum-theta-surface-…`, SHA `0dd989d9…`) confirmed contaminated via run_path→PaperEngine on all 6 active cells (BUYHOLD row clean — pure mark-to-market; this bin never calls run_cell, so the contamination has TWO routes: run_path for C2/C3-sweep, run_cell for the older threshold lanes). NEW riders for AC3: (i) cross-frequency Sharpe comparison — θ-cell curves per MERGED bar vs BUYHOLD hourly, both annualized hourly (~√10 deflation on the cell axis of the same table); (ii) BUYHOLD frictionless-vs-fee'd-cells asymmetry (behavioral half); (iii) per-cell trade counts into the θ-surface table (the hashed conclusion asserts a turnover mechanism its body can't evidence); (iv) the `{:.6}` format-placeholder prose + hard-coded `held_constant` line (body hygiene at regeneration). FP-C3.3's full-sweep two-run identity + the p50-vs-real-path interpretive field ride the re-verification suite. **Extension (1-18 review, 2026-08-04) — the heaviest yet:** anchors **#92-#99** (8 horizon θ-surfaces) join the inventory via the same run_path chain. THREE new items, two of them CRITICAL and one a claim-qualification already issued:
  - **bug-log #72 (CRITICAL):** the bootstrap's cosmetic 1h timestamp ladder makes funding settlement fire every 8 BARS, so carry-4h harvested ~1/4 and carry-daily ~1/24 of the true funding (per-path g=0: 15,490 → 3,039 → 267). Anchors #96-#99 measured a throttled mechanism, and their hashed bodies assert a "native settlement cadence" they never ran. **The carry × coarse-horizon leg of the thesis closure is UNRESOLVED, not direction-preserved** — errata already issued at `evidence/v1/horizon-retest-robustness/reports/ERRATA-2026-08-04.md` (operator-ratified escalation under AC4, 2026-08-04). The fix must make generated paths carry their true cadence (or every time-derived rule take it explicitly), then RE-DERIVE the carry-coarse verdicts.
  - **bug-log #71 (CRITICAL):** `Order::new`'s exposure cap is side-blind and rejects position-CLOSING sells silently (no else arm, no warn, no counter) — the strategy's held_symbols diverges from the engine's book and every later decision runs off a false flat. Blast radius is larger than #67's: it changes WHICH ORDERS EXIST. AC3 gains: the cap must be evaluated against RESULTING exposure with the side considered, plus a binding test.
  - **H4 (anchor-impacting):** daily carry samples only the 00:00-UTC settlement — the 08:00 and 16:00 rates never enter the score ring (bucket opens are midnight-aligned), so the daily carry score is a biased subsample, not a daily mean; at 4h each rate is forward-filled into two buckets, halving the documented "L settlements" memory. Grid docs say settlements; the code counts bars.
  - Body-hygiene riders for the regeneration: the `rebalance_minutes=60` row printed unqualified on 4h/daily surfaces; the ragged `| horizon |` column padding (fixing the alignment would break four anchors — leave until re-lock); the anchors.toml "every cell has p5 < 0" justification, which is false for #97 g4 and #99 g4 (verdicts still correct via weakest-link).
**Extension (1-17 review, 2026-08-04):** anchors #90/#91 (TS 2023+2024) join the inventory — identical chain, BUYHOLD clean, same riders; #92-#99 (1-18 horizon surfaces) expected at the 1-18 review. **The decorative-cap rider is UPGRADED to bug-log #69**: portfolio_exposure_cap inert engine-wide, D-TSM.2's safety premise false, TS surfaces ran ~90-100% gross vs the hashed 0.50 claim with alphabetical cash rationing — AC3 now includes enforce-or-delete + a BINDING test for every declared risk limit; AC4's re-derivation includes the corrected exposure description + an EXPLICIT re-affirmation of the active-trading-thesis closure (which stands direction-preserved-pending-re-lock per the 1-17 audit). Narrative corrections at regeneration also cover: the hysteresis/"band" mechanism (single threshold both ways — no band exists), the missing trades/turnover column vs the fee-bleed conclusion, the tim warmup-skew legend, and the cross-year same-seed correlation caveat. **Extension (1-16 review, 2026-08-03):** anchor #87 (MR θ-surface, SHA `a708112e…`) joins #86 — identical chain, BUYHOLD clean, same riders; plus the #68 drift-axis ratify-or-fix (implement the hold band or drop the axis + correct the narrative at regeneration; a drift-only cell pair becomes a mandatory per-axis divergence probe in the re-verification suite).
- Do-not-build register: not implicated (gate-credibility maintenance; no new alpha surface).
- Direction honesty: Blind's assessment — cleaning the noise plausibly shows MORE fragility, not less — is a hypothesis, not a result; AC4's escalation clause is the guard.

### References

- Trace: `REQ-HARNESS-FILL-CORRECTNESS-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
