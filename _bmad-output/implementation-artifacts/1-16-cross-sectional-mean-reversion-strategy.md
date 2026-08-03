# Story 1.16: cross-sectional-mean-reversion-strategy

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the first pivot family through the robustness harness (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the first pivot family through the robustness harness (verdict: FRAGILE).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-03 (burn-down 6 of 14; commit 94cd8d4; layers: Blind 12, Edge 8, Auditor 4 raw — deduped to 13).
     Gates THIS session: `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0 violations)`; independent leg: mr_divergence_e2e 5/5.
     Auditor verdict PASS (all ACs satisfied; two architect-ratified rescopes; momentum default provably unchanged; the day-1 divergence e2e meets the bar in substance — real run_path, both arms, ≥1bp Decimal).
     #67 ROUTING: anchor #87 confirmed contaminated via the identical run_path chain (BUYHOLD clean; √8575 + PWSD riders apply) → 1-25 inventory + the bug-log blast-radius gap closed this pass.
     OPERATOR: patches = apply all 11; drift-axis disclosure = bug-log #68 + 1-25 ratify-or-fix rider. -->

- [x] [Review][Decision→1-25/#68] **HIGH (disclosure, not anchor-impacting): the drift/hold-band swept axis is behaviorally INERT** — `drift_rebalance_threshold` reaches the config hash, the report grid table, and a `#[allow(dead_code)]` field; no drift logic exists in the executed path (the only real implementation, risk::size_portfolio_target, has zero production callers; the equal-weight open/close signal scheme cannot express a hold band) — yet the anchored narrative attributes g=3's result to the "wide hold-band" lever and the g=3-vs-g=4 comparison is confounded [crates/strategy/src/cross_sectional/momentum.rs:38-39,:164; crates/backtest/src/sweep_harness.rs:65,:351]. Bytes/verdicts stand (all-FRAGILE direction-preserving under an inert axis); the INTERPRETATION is wrong. Disclosure of record = bug-log #68; implement-or-drop-the-axis = 1-25 AC3-class ratify-or-fix rider. Inert since #86; promoted to a LOCKED-grid claim here. Also routed: anchor #87 → 1-25 inventory (both hunters + auditor verified the chain; bug-log #67 blast radius now names it).
- [x] [Review][Patch] Unvalidated `--direction`×`--grid` cross-product forges the OTHER family's anchored scenario NAME (mom+mr-tier1 → #86's name over the MR grid; rev+tier1 → #87's name over momentum cells with a false LOCKED header) → anchors-gate false RED via one misinvocation [bin:194; sweep_harness.rs:268-283,:2311+]. Bail on mismatched pairs (correct pairs byte-unchanged).
- [x] [Review][Patch] MR is the only family lane with no `effective_out_dir` arm — default writes MR reports into the frozen momentum reports dir (stale-rerun → #87 false RED from the wrong directory) [bin:171-177,:1830-1857]. Add the Reversion arm.
- [x] [Review][Patch] No `deny_unknown_fields` on RawConfig: a typo'd `direction` KEY silently runs Momentum (values fail loudly, keys don't) [crates/strategy/src/cross_sectional/config.rs]. Add it (no checked-in config carries unknown keys).
- [x] [Review][Patch] `direction` silently inert for non-VolAdjustedReturn score sources / TS mode yet hash-distinguishing (two K3 identities for one behavior; "carry reversion" runs identity-direction carry with no error) [config.rs:302-368; momentum.rs:767+]. Cross-field validation: Reversion requires the inverting arm, else loud error.
- [x] [Review][Patch] Both FP-MR.5 tests are decorative (#66 class — local re-implementations, renderer never invoked); real coverage exists via 1-15's shared `fp_c3_5` gate [mr_divergence_e2e.rs:392-463]. Delete-with-pointer + add one Reversion-arm body assertion through the real `render_surface_report`.
- [x] [Review][Patch] R-MR.1(b) falsifier overclaims — it runs Momentum-vs-Momentum (a determinism/noise-floor control) and cannot detect the dropped-negation bug its text claims; the detector is (a) [mr_divergence_e2e.rs:203-246]. Comment truthfix.
- [x] [Review][Patch] Config-hash domain silently migrated for ALL pre-1-16 configs (`;direction=` appended; hash flows into strategy lifecycle events + the agent watcher) — behavior pinned, identity not; no migration note [momentum.rs:875-905; core/strategy_events.rs; agent/watcher.rs:312]. Continuity/migration note at the hash fn + a momentum-hash-continuity pin test if cheap.
- [x] [Review][Patch] Reversion emits the negated score into `SignalEvidence::momentum` whose contract says "the vol-adjusted momentum score" — sign-flipped value, no direction marker [momentum.rs::build_rebalance_signals; core/signal.rs:105-110]. Contract doc fix.
- [x] [Review][Patch] Grid doc-table units gloss bars-as-hours ("1w"/"1mo") vs real wall-minutes rebalance — the 1-bar=1-hour assumption baked into LOCKED glosses unstated [sweep_harness.rs:340-351]. State it.
- [x] [Review][Patch] Bin console family-verdict block duplicates the two literals instead of calling the extracted `family_verdict_line` (drift risk console-vs-hashed) [bin:1897-1903 vs seam:2284]. Single source.
- [x] [Review][Patch] M-DEV-2 fixture comment misdescribes the data (−5%/bar claimed; −1.5 absolute step actual) [momentum.rs:1222-1241]. Fix.
- [x] [Review][Defer] FP-MR.3 claims outrun its body (single-cell in-process determinism, not θ-loop/fold-order coverage; the fill seed is constant) — real whole-sweep identity rides 1-25's re-verification (already deferred there from 1-15). Owner: 1-25. Revisit: at 1-25 close.

Notes recorded (no action): R-MR.5 "two anchored reports"/R-MR.6 "N+2" remain in the frozen feature text vs the ratified single-report/N+1 (deviation trail explicit here); report frontmatter git_commit=parent (unhashed, standard); ≥99.5% coverage leg inherited via the byte-identical data path.


- [ ] `cross-sectional-mean-reversion-strategy` 0.1.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/cross-sectional-mean-reversion-strategy/` - frontmatter status **`tester-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/cross-sectional-mean-reversion-strategy` (full narrative); reports under `evidence/v1/cross-sectional-mean-reversion-strategy/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-XS-MEANREVERSION-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-08-03, orchestrator)

All 11 patches APPLIED (dev subagent completed the implementation incl. the
direction×grid pairing bail, MR out-dir arm, deny_unknown_fields +
inert-combo validation, decorative-test rewire, and left the continuity pin
as a placeholder; orchestrator pinned the real hash `52ba6255…` and
verified). Literal verification: strategy **273/273** (incl. the new
continuity pin + validation tests) · mr_divergence_e2e 5 → **4/4** (two
decorative tests deleted-with-pointer; one REAL Reversion-arm renderer
assertion added) · param_sweep_e2e 13/13 · bin 19 → **24/24** (pairing
tests) · fresh clippy -D warnings clean (zero error lines) · `ANCHORS PASS
(119 / 119)` · `spec-lint: PASS (0 violations)`. Disclosures of record:
bug-log **#68** (inert drift axis — the #65 lineage, one layer up) with the
implement-or-drop + per-axis divergence probe riding 1-25; anchor #87 added
to the #67 blast radius + 1-25 inventory. The anchored-name forge vector
(direction×grid) is closed by validation with the anchored identities
proven byte-unchanged for every valid pair.
