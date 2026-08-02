# Story 1.24: pwsd-fidelity-relock

Status: ready-for-dev

<!-- Created 2026-07-31 by the 1-13 code-review decision (operator: "doc now + re-lock story").
     This story owns the CODE half of the PWSD-fidelity cluster; the doc half landed in the
     1-13 review pass (deviation recorded, behavior pinned by the FP-C1.6 assert). -->

## Story

As the operator of the Honest Advisor,
I want the Monte-Carlo credibility layer's block-length estimator brought to cited-literature fidelity under a formal anchor re-lock,
so that the robustness gate's null distribution is defensible against the exact references the code cites — with the anchored history migrated honestly, never silently.

## Acceptance Criteria

1. **Window scale**: the flat-top lag window evaluates `λ(k/(2m̂))` per Politis-White 2004 (flat to lag m̂, tapering to 2m̂), replacing the shipped `λ(k/m̂)`; the m̂ search checks `ρ̂(m+1..m+K_N)` per the reference (shipped: `ρ̂(m..)`); the FP-C1.6 pin is re-baselined to the new l_ar1 with the change stated in the assert message.
2. **Auto-L target**: the cross-symbol `|r|`-mean choice is either per-symbol-standardized (or replaced with a per-symbol max/median policy) with an architect sign-off recorded in this story, or explicitly re-ratified as-is — no silent carry-forward. The absolute-ε zero-variance guard becomes scale-relative in the same pass.
3. **Volume proxy**: synthetic bars carry the correct symbol's volume at the correct bar offset (returns k spans bars k→k+1), replacing the symbol-0-at-ret_idx proxy; the slippage-consumption path (`volume_usd`) is traced and the impact (if any) on regenerated bodies is quantified in the re-lock report.
4. **Formal re-lock per ADR-0045 § D6**: the affected MC/sweep reports are regenerated under a NEW anchor namespace; the old `mc-robustness-2026-06` rows stay byte-frozen as history; an errata/decision note records old-vs-new L values per scenario and the verdict deltas (expectation per the review's direction analysis: longer L ⇒ crowns same or fewer ⇒ ship-passive strengthens — any OPPOSITE movement is a REGRESSION-class finding requiring operator escalation before ship).
5. Standing floor: anchors green under the post-re-lock set (old rows intact + new rows added); `spec_lint` PASS; the FROZEN gate (`classify_verdict`/`verdict_bands`/`compute_robustness_flag`/`rank_candidates`) byte-untouched with the identity-test obligation discharged.

## Tasks / Subtasks

- [ ] Architect pass: AC2 policy decision + the re-lock plan (namespace name, affected report inventory, compute budget).
- [ ] Dev: window/m̂/ε/volume fixes + re-baselined pins (AC1-AC3).
- [ ] Re-run + re-lock: regenerate, anchor under the new namespace, errata note (AC4).
- [ ] Review: verify old rows byte-intact, new rows complete, verdict-delta table honest (AC5).

## Dev Notes

- Origin: 1-13 review findings (Blind Hunter H1 + M3, Edge L3, plus the volume-proxy M7 routed here for anchor safety) — full evidence in `1-13-monte-carlo-bootstrap-path-generator.md` § Review Findings.
- Do-not-build register: not implicated (this is gate-credibility maintenance, not alpha-chasing; the E-1 report-only boundary is untouched — L feeds the EXISTING gate's null, no crown-eligibility change).
- The 1-13 pass pinned current behavior (`l_ar1 == 7` assert) precisely so this story's change is loud, not silent.
- Direction-of-change guard (AC4) is the story's honesty core: the re-lock must PROVE the thesis-safe direction claim, not assume it.

### References

- Trace: `REQ-PWSD-FIDELITY-RELOCK-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
