# Story 2.63: lab-recipe-test-harness-v0.3-extension

Status: ready-for-dev

<!-- Analyst-drafted 2026-07-29 (Mary). Operator-DECIDED build: PRD §13 Q2 answer, 2026-07-27
     ("schedule"). Promotes the standing backlog row (backlog.md § Deferred by decision, tracked
     as this story slug) from backlog to ready-for-dev. TEST-INFRA, not product feature surface. -->

## Story

As the operator of the Honest Advisor,
I want the v0.3 cross-surface extension of the lab recipe/subscription test harness — the ADR-0048 pattern applied to every Recipe added since v0.2.0, with every recipe test asserting EXECUTION,
so that the v0.2 durable contract ("harness coverage is every UI Recipe/Subscription"; H4: future Recipes inherit the pattern) holds again on today's code, and no recipe guard can pass vacuously (the bug-log #66 class: a test that exists and passes is not a test that runs).

## Acceptance Criteria

1. **Fresh R1-style inventory:** re-derive the per-surface vulnerability map (mirror v0.2's R1 table) over today's `impl Recipe`/subscription set in `crates/ui/` + `crates/agent/`; it MUST cover at minimum the four post-v0.2 recipes verified uncovered 2026-07-29 — `ForwardPlanRecipe` (`crates/ui/src/live.rs:889`, zero test references), `NarrationOutcomeRecipe` (`:963`, zero), `SweepProgressRecipe` (`:1128`, zero), `BakeoffProgressRecipe` (`:1052`, partial via `tests/bakeoff_progress_relay.rs`) — and classify each as S1 (boundary `stream_impl` test) and/or S2 (gating/inclusion test) needed, with K3 well-isolated exclusions reasoned in writing.
2. **Pattern-(d) coverage lands per the inventory:** each in-scope recipe gets its S1/S2 tests via the per-Recipe-specific mock pattern (ADR-0048 D1-D6 carry forward; the extracted `forward_plan_stream_impl`/`narration_outcome_stream_impl` seams already exist — reuse; new production seams are API-additive extractions only, shape-identical to v0.2's Wave C/D precedent).
3. **Non-vacuity — every recipe test asserts EXECUTION (bug-log #66 precedent, 2026-07-27 hardening theme):** no skip-as-pass anywhere in the harness suite (new AND the surviving v0.1/v0.2 files): fixture/corpus paths resolve from the workspace root (`CARGO_MANIFEST_DIR`-derived, never cwd-relative), skip is legal ONLY on genuine probe-absence, any `Err` with the fixture present FAILS loudly, and each test proves it ran (event/count assertions that cannot be satisfied by an early return). A sweep records the audit result per existing harness file.
4. **Falsification proof (v0.2 Q2=(a) durable protocol):** every new test file carries the T-T4 probe line in its module doc-comment and the tester report records one FAIL → restore → PASS cycle per recipe — no probe, no PASS ("prove it or it's theater").
5. **Batch-inclusion + floor:** `build_subscription_batch_descriptor` (`crates/ui/src/live.rs:1361-1387`, the Wave C seam) gains/asserts variants for the in-scope recipes' inclusion gating; zero production behavior change beyond API-additive seams; `verify_anchors` 119/119 before AND after; `python3 scripts/spec_lint.py` PASS; clippy clean; build-time inflation ≤ 10% (K4 carry-over).

## Tasks / Subtasks

- [ ] Inventory pass (AC 1) — output as a table in this story's Dev Agent Record before coding.
- [ ] Wave-decomposed implementation per inventory (mirror v0.2's Wave A-D shape; one tester verdict per wave).
- [ ] #66 non-vacuity sweep over the existing harness suite (AC 3) — fix or document every skip path.
- [ ] ADR-0048 § Changelog row for v0.3.0 (no new ADR expected — D-V0.2.0-4 precedent) unless a genuinely new pattern class emerges (K1 route: back to analyst).
- [ ] Gates: anchors 119/119, spec-lint, clippy, fmt; T-T4 probe evidence in the tester report.

## Dev Notes

- **What v0.2 covered (so v0.3 doesn't re-litigate):** v0.2.0 shipped the FULL Q1=(a) scope 2026-05-30 — all four waves (TrainingLog S1+S2, ActivityAuditAggregator select-arm survival, SubscriptionBatchDescriptor seam + ServerTime/ToastDismiss, TrailMirror S2 + Activity S1 + TrainingPoller S1+S2). Nothing was deferred FROM v0.2; "v0.3.0+" was reserved (brief § Hypotheses H4 + falsifier K1 routing) for exactly this: applying the contract to recipes that did not exist yet. The four advisor-era recipes above (F5 forward-plan, F9 narration, F1/F2 bakeoff progress, F8 sweep progress) are that accrued surface.
- **Motivating precedent (cite in every skip-path fix): bug-log #66** — the ui real-data guard tests were vacuous since day 1 (cwd-relative corpus root; any-`Err`→skip), the anchored report's "ran for real" claim held only for the backtest twins, and reviving ONE chain surfaced three real product bugs within the hour. Moral, per the entry: skip paths need positive proof of execution. That is AC 3's bar, suite-wide.
- **Do-not-build register check (mandatory): PASS.** Test-infra only — no alpha surface (Group A untouched), no scope change (B), no data feeds (C), no execution machinery (D), no gate/anchor edit (E; pure test additions, the v0.2 D6 anchor-additivity precedent held byte-identical). The backlog row was explicitly re-confirmed "still wanted" at the v3 close-out (2026-07-09) and CHANGELOG § Deferred carries it as the one genuinely-open forward build item.
- Feature-completeness note: this story does NOT reopen feature development; it is the hardening/infra lane the 2026-07-27 theme names. `#[cfg(feature = "live")]` gating where a recipe is live-only (K4 carry-over).

### References

- Trace: `REQ-LAB-RECIPE-HARNESS-V0-3-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))
- Decision record: PRD §13 Q2 (operator answer 2026-07-27); `_bmad-output/planning-artifacts/backlog.md` § Deferred by decision; ADR-0048 (+ v0.2.0 Changelog row); predecessor stories `2-41-lab-recipe-test-harness`, `2-42-lab-recipe-test-harness-v0-2-0-cross-surface-extension`; `docs/dev-notes/bug-log.md` #66.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
