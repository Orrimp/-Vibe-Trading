# Story 1.13: monte-carlo-bootstrap-path-generator

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the stationary-block-bootstrap path generator (Politis-White auto block length) that resamples real returns preserving fat tails and volatility clustering,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the stationary-block-bootstrap path generator (Politis-White auto block length) that resamples real returns preserving fat tails and volatility clustering.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-07-31 (burn-down 3 of 14; commit 7d326ca; layers: Blind Hunter 14, Edge Case Hunter 11, Acceptance Auditor 8 — deduped to 19).
     Gates re-run THIS session: `ANCHORS PASS  (119 / 119)` · `spec-lint: PASS (0 violations)`; independent leg: synth suite 28/28 green.
     Auditor verdict PASS (all ACs satisfied or deviation-ratified at the implementing commit; ADR-0051 §D6 seed discipline verified verbatim in all three consumers). Blind Hunter confirmed the PR-1994 core, PPW-2009 D_SB constant, and shared-index wiring CORRECT — the findings are around that sound core.
     OPERATOR DECISIONS 2026-07-31: PWSD cluster = "doc now + re-lock story" (see 1-24); patches = apply all. -->

- [x] [Review][Decision] **PWSD-fidelity cluster → DECIDED: doc-now + re-lock story 1-24.** The flat-top window runs λ(k/m̂) where PW-2004 + both reference impls use λ(k/(2m̂)) → auto-L systematically short → MC null slightly too benign (empirically thesis-safe: longer L ⇒ fewer crowns ⇒ ship-passive strengthens); the m̂ search checks ρ̂(m..) vs the literature's ρ̂(m+1..); Auto runs PWSD on the cross-symbol mean of |r| (highest-vol symbol dominates; b_opt derived for a different series than the one resampled); the zero-variance guard uses absolute ε (scale-dependent); the volume proxy writes symbol-0's volume at the wrong bar on every symbol (slippage consumes volume_usd → potential anchored-body divergence on regeneration). L flows into hashed anchored bodies (`mc-robustness-2026-06` namespace) → the CODE fixes require an ADR-0045 §D6-class namespace re-lock + verdict re-run = story 1-24. TODAY (this pass): reconcile the three-way doc contradiction to the one shipped truth, record the deviation-from-literature explicitly [crates/data/src/synth/block_length.rs:22-24 vs :127-130 vs code :155], pin current behavior via the FP-C1.6 assert.
- [x] [Review][Patch] Universe↔source pairing is positional with a silent symbol-0 fallback — names never cross-checked; longer universe silently reuses returns_by_sym[0]; the funding/basis extensions stack two more arrays on the same unenforced contract [crates/data/src/synth/bootstrap.rs:316-325]. Add symbol-name + arity validation → typed SynthError (behavior-neutral for all shipped consumers, which build universe from the same source).
- [x] [Review][Patch] GBM anti-diagonal seed collision: sym_seed = path_seed + sym_i·GOLDEN_GAMMA while path_seed_j = master + j·GOLDEN_GAMMA → seed(j,i) collides whenever i+j matches (ETH-on-path-0 ≡ BTC-on-path-1) [crates/data/src/synth/gbm.rs:145]. Fix with a distinct per-symbol mixing (e.g. splitmix64 over (path_seed, sym_i)); ANCHOR-SAFE — no GBM-derived body is anchored (verified).
- [x] [Review][Patch] GbmParams unvalidated: inverted/NaN clamp bounds PANIC (f64::clamp asserts); NaN vol/drift silently flatlines to price_lo [crates/data/src/synth/gbm.rs:162,170]. Validate at construction → typed error.
- [x] [Review][Patch] Source-price sanity: zero/negative/NaN closes produce inf/NaN log-returns silently clamped into flat garbage; decimal_to_f64 unwrap_or(1.0) rewrites unparseable prices [crates/data/src/synth/bootstrap.rs:226-233,390,472-475]. Validate finite-positive at new() → typed error; guard pub politis_white_block_length against non-finite input.
- [x] [Review][Patch] Non-positive start_price silently clamped (bootstrap 1e-6 / GBM 0.01) → plausible-shaped garbage, no error [bootstrap.rs:327; gbm.rs:147-152]. Typed error.
- [x] [Review][Patch] Fixed(L) unbounded: L ≫ n_returns → p≈0 → circular-replay "ensemble" with zero dispersion masquerading as MC evidence; Fixed(0) silently promoted [bootstrap.rs:242,264-284]. Error when Fixed(L) > n_returns (or 0), loud not silent.
- [x] [Review][Patch] 2-bar source (single return) → seed-INDEPENDENT dispersion-zero ensemble (random_range(0..1) always 0); raise minimum source length so divergence is structurally possible [bootstrap.rs:120,270-280].
- [x] [Review][Patch] Unbounded n_bars → time-crate overflow panic / alloc abort (~70M bars hits year-9999) [bootstrap.rs:486-495; gbm.rs timestamps]. Sane upper bound → typed error.
- [x] [Review][Patch] FP-C1.4 moment tolerances near-vacuous (mean floor ~158 standard errors; var floor 7× actual) — the #66 pattern on the story's headline statistical guarantee [bootstrap.rs:686-698]. Tighten to meaningful standard-error multiples (must stay green on the shipped fixture).
- [x] [Review][Patch] No test drives BlockLengthPolicy::Auto through generate() on a DEPENDENT series — a regression hard-wiring Auto→L=1 (the hunted silent-IID degradation) passes the whole suite [bootstrap.rs:881-890]. Add the AR-series Auto e2e asserting selected_block_length > 1.
- [x] [Review][Patch] FP-C1.5 fixture contradicts itself: both "correlated" series share identical RNG seeds (source corr exactly 1.0, "idiosyncratic noise" common to both), comment describes structure that doesn't exist, dead `let _ = 0_u64` [bootstrap.rs:719-723,768]. Make the fixture honest (real idiosyncratic noise) while keeping its proven mutation-catching property.
- [x] [Review][Patch] FP-C1.6 claims to pin the canonical fixture but only PRINTS l_ar1 — the advertised algorithm-change tripwire doesn't exist (it would have caught the window-scale issue) [block_length.rs:228-237]. Assert the pinned value (pins CURRENT behavior intentionally, pending 1-24) + fix the stale "expected: 5" comment (actual 7).
- [x] [Review][Patch] Doc honesty set: trait `# Errors` promises ragged-universe errors generate() cannot raise [mod.rs:127-135]; "byte-identical" contract omits the per-platform qualifier (libm 1-ulp) [mod.rs:7-8]; story records the ratified R2.3 SHA-plumbing deviation + the deleted tester report's git-show provenance (report was never anchored; deleted by CLEANUP 1405042) [story Dev Notes].
- [x] [Review][Patch] Trace-row title records the pre-ratification contract ("GBM lifted (behaviour-preserving)", "revision SHA" input) — neither shipped (D-C1.5 ratified a NEW GBM impl; no SHA param exists) [_bmad-output/planning-artifacts/trace.toml:2617]. Rewrite title truthfully, PRIOR-preserved.
- [x] [Review][Defer] Resampler fabricates constant ±10bp wicks (real high/low discarded) — fine for today's fill logic, latent poison for any intrabar consumer [bootstrap.rs:410-411] — deferred, design-note class. Owner: architect (any future intrabar-fill story must revisit). Revisit: epic-1 retrospective.
- [x] [Review][Defer] GBM 3-copy dedup carve-out (D-C1.5 v0.2.0) still unbuilt and unowned (momentum.rs / main.rs / determinism.rs copies independent) — deferred. Owner: dev (refactor-when-touched). Revisit: epic-1 retrospective.

Dismissed as noise (2): K_N = max(5, ceil(log10 N)) matches the PPW reference code (not the paper text — no practical effect); Ĝ/ĝ naming swapped vs literature but algebraically correct.

- [ ] `monte-carlo-bootstrap-path-generator` 0.1.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/monte-carlo-bootstrap-path-generator/` - frontmatter status **`tester-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/monte-carlo-bootstrap-path-generator` (full narrative); reports under `evidence/v1/monte-carlo-bootstrap-path-generator/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-MC-BOOTSTRAP-PATH-GENERATOR-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-07-31, orchestrator)

Decision (operator): PWSD cluster = doc-now + re-lock story → **story 1-24
created, wired (board ready-for-dev, trace REQ-PWSD-FIDELITY-RELOCK-001
scoped)**; the deviation is now recorded in block_length.rs's module doc and
the FP-C1.6 assert pins the shipped behaviour bit-exactly (AR(1) φ=0.6 → L=7;
iid → L=1) so 1-24's change will be loud. All 14 anchor-safe patches APPLIED
(dev subagent completed edits; verification by the orchestrator after an app
restart): validation set (universe symbol/arity, finite-positive prices,
GbmParams, start_price, Fixed(L) bound, n_bars cap, ≥3-bar minimum, pub PWSD
non-finite guard), GBM anti-diagonal seed fix (splitmix-mixed, collision test
added), FP-C1.4 honest tolerances, Auto-through-generate dependent-series
test, FP-C1.5 honest fixture, FP-C1.6 assert-not-print, doc set (trait
Errors, per-platform byte-identity, R2.3 ratified-deviation + tester-report
git-provenance), trace-row title truthified. Verification (literal): synth
suite 28 → **43/43 green**; all 21 data suites ok; fresh `cargo clippy -p
data -- -D warnings` = 0; `cargo check -p backtest` clean; bakeoff bootstrap
10/10; `ANCHORS PASS (119 / 119)`; `spec-lint: PASS (0 violations)`.
Auditor verdict PASS; Blind confirmed the PR-1994 core + PPW-2009 constant
CORRECT — the review's residue was around, not in, the statistical core.
