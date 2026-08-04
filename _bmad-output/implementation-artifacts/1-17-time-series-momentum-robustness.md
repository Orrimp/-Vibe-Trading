# Story 1.17: time-series-momentum-robustness

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want per-asset absolute momentum (long/flat) - the thesis-closing OHLCV test (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: per-asset absolute momentum (long/flat) - the thesis-closing OHLCV test (verdict: FRAGILE).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-03/04 (burn-down 7 of 14; commits 2e577be/dd55e70/c59998e/317256a; layers: Blind 12, Edge 12, Auditor 8 raw — deduped to 18).
     Gates THIS session: `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0 violations)`.
     Auditor verdict PASS-with-findings; BOTH anchored surfaces recomputed byte-identical at HEAD; the #68 axis probe ran on every TS swept axis and came back CLEAN (lookback + entry_threshold both EXECUTED, proven by fill-independent stat divergence in both anchored bodies).
     THE THESIS-CLOSING QUESTION, answered: #90/#91 are #67-contaminated (chain verified end-to-end) → the active-trading-thesis closure stands "DIRECTION-PRESERVED PENDING RE-LOCK", carried by: BUYHOLD fill-clean; ≥~1.06-Sharpe uniform margin across all 12 cell-years (tails 81-97% vs ~70% band); common-mode contamination across all four families; time_in_market reads position-book state independent of fills. Product-level ship-passive separately insulated by the bakeoff gate. Honest caveats: mispricing asymmetric by construction; tails are the fill-sensitive statistics.
     OPERATOR: patches = apply all 9; #69 disclosed; anchor-touching set → 1-25. -->

- [x] [Review][Decision→1-25/#69] **HIGH (the #68 probe landed one seat over): `portfolio_exposure_cap` is INERT engine-wide** — `Order::new` checks only per_symbol_exposure_cap [crates/core/src/order.rs:123-170]; the portfolio cap's sole implementation (risk::portfolio.rs:189) has zero callers; run_path sets Some(0.50) decoratively [montecarlo.rs:176-180]. Invisible in every prior family (K≤5×10% never binds); load-bearing HERE: D-TSM.2's ratified premise ("the cap throttles… no new solvency surface") is FALSE — TS emits up to 10 Buys → ~90-100% gross on high-breadth bars vs the hashed `held_constant | exposure_cap=0.50` row, with the cash pre-flight rationing the book ALPHABETICALLY (BTreeMap order), violating the per-asset-independence criterion. Verdict likely survives (p5-Sharpe scale-invariant; prob_loss/p95-maxdd are the exposure-sensitive signals). Disclosure = bug-log **#69**; 1-25 rider UPGRADED (enforce-or-delete the cap + corrected exposure description + explicit thesis re-affirmation joins AC4). Also → 1-25: anchors #90 (`c1bf9325…`)/#91 (`ff7e7dda…`) into the inventory (+√8575/PWSD riders, BUYHOLD-clean); the 1-18 horizon surfaces #92-#99 flagged for the next review's sweep; M-3 hysteresis narrative (the "band" mechanism the grid roles name does not exist — single threshold both ways); M-4 trades/turnover column absent while the anchored conclusion asserts fee-bleed (two inert columns rendered instead); tim-denominator warmup-skew legend; cross-year same-seed correlation caveat (intended SAME-paths per LOCKED constants, unstated in bodies).
- [x] [Review][Patch] The forge lane is open on the selection_mode axis: `--grid ts-tier1` + default top-K passes the direction guard and forges anchor #86's name into the frozen momentum dir; converse forges #90/#91's names; TS+carry loads the funding sidecar for behaviorally-different equity under the TS name [bin:215,:1407; sweep_harness required_direction]. Extend the pairing guard: GridKind→required (direction AND selection_mode AND score_source family).
- [x] [Review][Patch] Inert-combo validation one field over: TS×{funding_carry,basis_reversal,…} silently runs price-TS under a carry identity; nonzero entry_threshold under CrossSectionalTopK inert-but-hash-distinguishing [config.rs:377-395]. Extend the 1-16 InertDirection guard to InertScoreSource/InertThreshold.
- [x] [Review][Patch] Falsifier strengthening (#66 class — three of five vacuous vs their own revert stories): F-TSM.1 primary compares against full-capital BH so an always-long no-op passes via the sizing gap alone — re-point at the like-sized always-long control; F-TSM.3 "no-look-ahead" feeds different series and cannot fail on look-ahead — rebuild as prefix-invariance (truncate future bars, assert past decisions unchanged); F-TSM.4 headline warmup-satisfiable — assert post-warmup tim; F-TSM.2 tolerance 5_000 (~500× loose) — tighten; F-TSM.5 ignores its seed/n params and never exercises the seeded ensemble — drive a real small-N seeded TS sweep two-run identity [ts_momentum_divergence_e2e.rs].
- [x] [Review][Patch] Phantom-flag doc: `PathRunResult.time_in_market_bars` claims a `track_time_in_market` flag that does not exist; the counter increments unconditionally (non-zero for momentum/MR/carry; anchor-neutrality comes from render gating ONLY) — a reader trusting the doc walks into a four-family body-SHA change [montecarlo.rs:70-79]. Truthful doc.
- [x] [Review][Patch] `entry_threshold` is the only unvalidated numeric in the loader (500%/absurd values accepted silently; every sibling field is range-checked) [config.rs:410]. Bounds + loud reject, consistent with siblings (keep negative-θ allowed — tests use it deliberately; document).
- [x] [Review][Patch] Post-warmup score error silently force-exits (None → absent → Sell; recovery → Buy = untraced fee-paying round-trip on a data glitch) [momentum.rs TS arm]. tracing::warn on score-error exits; behavior unchanged.
- [x] [Review][Patch] anchors.toml #91 COMMENT mislabels the mode as `direction=time_series_long_flat` (the field is selection_mode; direction is momentum/identity) — comment-only, outside the hashed body [evidence/anchors.toml:784 region]. Fix the comment; run the anchors gate before AND after.
- [x] [Review][Patch] The c59998e test-pollution admission ("6 stray tcn-overlay reports a test side-effect wrote into anchored dirs — flagged for a follow-up fix") lives only in a commit message — track it: deferred-work entry with owner (tests must write to TempDir, never evidence/; a probe for strays belongs in the anchors script or CI) [deferred-work.md].
- [x] [Review][Defer] Basket-gated warmup under the per-asset TS mode (`all_warmed` requires ALL rings full — one late-listing symbol silently flattens the whole strategy forever; latent for exactly the broader-universe route the anchored conclusion recommends) [momentum.rs:240-247] — behavior change; deferred. Owner: any future universe-expansion/TS-v2 story (MUST fix before broader universes). Revisit: epic-1 retrospective.

Notes recorded: F-TSM.5's operational two-run SHA (e551aa7a) was commit-message-only; the delivered proxy matches the spec-cited fp_c3_3 pattern (deviation-in-letter, recorded). k_long=10 inert under TS but honestly documented in the hashed held_constant line (no #68-class violation).


- [ ] `time-series-momentum-robustness` 0.1.0 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/time-series-momentum-robustness/` - frontmatter status **`presenter-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/time-series-momentum-robustness` (full narrative); reports under `evidence/v1/time-series-momentum-robustness/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-TIME-SERIES-MOMENTUM-ROBUSTNESS-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-08-04, orchestrator)

All 8 patches APPLIED by the dev subagent (first complete agent run of the
burn-down — full report, all verification quoted) and independently
spot-verified by the orchestrator: full-tuple pairing guard (13 anchored
tuples pass, 5 forge combos bail naming all three axes); InertScoreSource +
InertThreshold loader guards; the falsifier suite REBUILT so each fails
under its own revert story — like-sized control gate, prefix-invariance
no-look-ahead, post-warmup tim, 0.1bp tolerance with in-code derivation,
and a REAL seeded two-run byte-identity through the production seams with
the seed proven live; phantom-flag doc truthfixed; entry_threshold bounds;
score-error-exit tracing; anchors.toml #91 comment fixed (agent) + the #90
twin (orchestrator), each under a before/after `ANCHORS PASS (119 / 119)`
double gate. Literal verification: ts_momentum_divergence_e2e 7/7 ·
strategy 386 · bin 26/26 · param_sweep 13/13 · mr 4/4 · fresh clippy zero
lines · `spec-lint: PASS (0 violations)`. Disclosures: bug-log **#69**
(inert portfolio cap — the ratified D-TSM.2 premise false; ~2× documented
gross, alphabetically rationed) with enforce-or-delete + binding-tests +
thesis re-affirmation riding 1-25 AC3/AC4; anchors #90/#91 into the #67
inventory; the thesis closure stands DIRECTION-PRESERVED PENDING RE-LOCK
per the audit's four-leg argument.
