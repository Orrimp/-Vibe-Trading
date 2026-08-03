# Story 1.15: momentum-parameter-robustness-sweep

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the distribution-per-theta sweep over the momentum family (verdict: FAMILY-UNIFORM-FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the distribution-per-theta sweep over the momentum family (verdict: FAMILY-UNIFORM-FRAGILE).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-07-31 (burn-down 5 of 14; commit f83c106; layers: Blind 15, Edge 8, Auditor 7 raw — deduped to 17).
     Gates THIS session: `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0 violations)`; independent leg: param_sweep_e2e 8/8, bin 19/19.
     Seed conformance: ADR-0051 §D6.1 EXACT (rejected D6.2 mix absent from production; its only occurrence is the negative test proving the collision).
     #67 ROUTING CONFIRMED: anchor #86's FAMILY-UNIFORM-FRAGILE consumed the corrupted fill path via run_path→PaperEngine on ALL 6 active cells (this bin never calls run_cell); BUYHOLD row clean (pure mark-to-market). √8575 + FILL_SEED riders consumed. → anchor #86 added to story 1-25's re-derivation inventory.
     OPERATOR: patches = apply all 12; anchor-impacting set routed to the standing 1-25 program (inventory extended, not silently). -->

- [x] [Review][Decision→1-25] **Anchor-impacting set routed to the standing 1-25 program (per the 1-14 routing decision):** H1 cross-frequency Sharpe comparison (θ-cells per MERGED bar ≈10 samples/hour vs BUYHOLD hourly, both annualized hourly → cell Sharpe deflated ~√10 in the same anchored table; family verdict unaffected — equity-level signals carry it) [bin:1085-1102 vs :506-583]; BUYHOLD frictionless while cells pay ~6 bps (behavioral half; comment truthfix is patched now) [buyhold.rs]; per-cell trade counts absent from the table while the hashed conclusion asserts a turnover mechanism [bin:806-818, dead total_trades :471]; literal `{:.6}` format-placeholder prose frozen in the hashed body [bin:733]; `held_constant` line is hard-coded prose not read from config [bin:723]; + anchor #86's contamination re-derivation itself.
- [x] [Review][Patch] FP-C3.2 grid-sensitivity gate hashes two hand-built strings — proves SHA-256, not the renderer; remove-the-grid-from-body passes it (#66 class) [param_sweep_e2e.rs:418-482]. Re-point through the production renderer/grid_def (seam extraction per the 1-14 mc_harness precedent).
- [x] [Review][Patch] FP-C3.5 anti-cherry-pick pair asserts local re-implementations; `render_surface_report` has ZERO test coverage — a "best θ is ROBUST" renderer edit passes every gate [param_sweep_e2e.rs:484-573; renderer bin:624-829]. Real renderer-output asserts via the seam.
- [x] [Review][Patch] θ-injection seam ungated: e2e builds its own configs, never the bin's `cell_config`/`per_cell_cfg` wiring — a no-op collapse to 6 identical rows passes all gates (the vol-overlay-noop class); e2e's 2-symbol universe also leaves k_long inert [param_sweep_e2e.rs:240-267; bin:483-494,:1209]. Gate the REAL injection path; make k_long distinguishable in the fixture.
- [x] [Review][Patch] `--grid two-cell` emits the SAME scenario identity as the anchored tier-1 → shadows anchor #86's report as latest → false anchors-RED [bin:384-387,:1397-1404]. Encode the grid kind in non-tier1 scenario names (tier-1 name byte-unchanged → anchor-safe).
- [x] [Review][Patch] Phantom g=0 probe: role text claims "MUST reproduce C2 anchor numbers" — arithmetically impossible at N=200 vs C2's N=500, and no code compares anything [bin:186,:225 region]. Truthful text (disclosed direction-match, manual) + drop the claim from the module doc.
- [x] [Review][Patch] e2e per-symbol seeds reproduce the additive anti-diagonal collision in the harness's own fixture (path j's sym-B ≡ path j+1's sym-A) [param_sweep_e2e.rs:135-144]. Splitmix idiom (3rd instance of the class — 1-13 data crate, 1-14 monte_carlo bin, now here).
- [x] [Review][Patch] GbmSmoke lane hygiene: burns 87,600 unused source bars with a literal 0xC0FFEE ignoring --ensemble-seed; scenario named "block-bootstrap-gbm" (no bootstrap); frozen §4.1 declares gbm output VOID but only a Notes line flags it [bin:844-856,:1397]. Skip the unused generation, honest gbm-lane naming, explicit VOID banner (non-anchored lane).
- [x] [Review][Patch] No minimum-N guard vs the ratified N≥200 floor — `--paths 3` renders a full verdict-bearing report silently [bin:354-356]. Warn (non-breaking) below the floor.
- [x] [Review][Patch] BUYHOLD rayon closures panic via .expect after ~20 min of cell compute; cells use the contextual Result path [bin:1305-1310 vs :1246-1248]. Result path + context.
- [x] [Review][Patch] Family verdict computed twice (renderer flag + main recompute) — future desync risk between console and hashed line [bin:740-746 vs :1464-1471]. Single source.
- [x] [Review][Patch] BUYHOLD comment claims "no fees after bar 0" — there is no fee at bar 0 either; + stale N=500×G=14 throughput comments [buyhold.rs; bin:2567,2621,3451 at HEAD]. Truthful comments (behavioral question stays 1-25).
- [x] [Review][Patch] Tester-report provenance + FP-C3.3 binary-leg honesty: record in Dev Notes that the report lives in git only (CLEANUP 1405042) and the full-sweep two-run identity leg was never completed (unit-level leg is the spec-permitted form) [story Dev Notes].
- [x] [Review][Defer] FP-C3.3 full-sweep two-run byte-identity: run it as part of 1-25's re-verification suite (it re-runs the sweep anyway). Owner: 1-25. Revisit: at 1-25 close.
- [x] [Review][Defer] D-C3.6 "p50-vs-real-path" interpretive field referenced in the body Notes but never computed — body edit = re-lock; fold into 1-25's regeneration. Owner: 1-25. Revisit: at 1-25 close.

Dismissed as noise (2): lookback-units suspicion (bar-count semantics correct on hourly bars); L=204 probe identity (legitimate — L derives from the source series, D6.1.4).


- [ ] `momentum-parameter-robustness-sweep` 0.1.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/momentum-parameter-robustness-sweep/` - frontmatter status **`tester-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/momentum-parameter-robustness-sweep` (full narrative); reports under `evidence/v1/momentum-parameter-robustness-sweep/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).
- Provenance addendum (review 1-15 patch 12, 2026-08-03): the v0.1.0 TESTER REPORT exists in git history only (`git show f83c106` era; deleted by the ratified CLEANUP-PLAN commit `1405042`). FP-C3.3's BINARY-level two-run full-sweep identity leg was started at delivery and never recorded complete — the unit-level leg is the spec-permitted form that shipped; the full-sweep leg rides story 1-25's re-verification suite.

### References

- Trace: `REQ-MOMENTUM-PARAMETER-ROBUSTNESS-SWEEP-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-08-03, orchestrator)

All 12 patches APPLIED (dev subagent died on the session limit after ~9 —
incl. the full sweep_harness.rs seam extraction, the M2 discriminator with
the tier-1-name-byte-unchanged proof, min-N warn, L2 comment truthfix;
orchestrator finished: both production GbmSmoke sym-seed sites →
`derive_gbm_sym_seed` splitmix idiom, BUYHOLD closures → contextual Result,
e2e seed helper de-collided, the 5 seam-driven gates written, story
provenance note, + 30 seam lint conversions proven byte-neutral). Literal
verification: param_sweep_e2e 8 → **13/13** (fp_c3_2/fp_c3_5 now assert REAL
renderer output; θ-injection through the REAL cell_config on both axes; seed
invariant via the production fn; tier-1 scenario identity asserted
byte-identical to the anchored literal) · bin 19/19 · fresh clippy -D
warnings = 0 error lines · `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0
violations)`. The anchor-impacting set (cross-frequency Sharpe axis, BH
friction, trades-in-table, body-hygiene items, anchor #86 re-derivation)
rides story 1-25 per the standing routing decision — its inventory was
extended visibly in this pass. External check same window: the weekly
spec-auditor (audit-2026-08-03) independently found all gates green and the
review program honest.
