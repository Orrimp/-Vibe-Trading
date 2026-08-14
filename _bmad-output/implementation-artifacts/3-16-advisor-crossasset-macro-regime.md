# Story 3.16: advisor-crossasset-macro-regime

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-14 (burn-down 14 of 14 — the last story; run through the bmad-code-review workflow. Layers: Blind 20, Edge 22, Auditor 12 raw — 54 deduped to 24).
     VERDICT: **FAIL — the story does NOT close.** Two deliverables: the market-calendar layer IS genuinely delivered and genuinely proven inert; the `v0.macro_riskon` probe is delivered as code but is UNCONDITIONALLY inert in the product build.
     All findings anchor-impacting: NO (the arm writes no anchored body — `write_report=false`, `anchors = []`, verified). Chain CLEAR of #67/#69/#71/#72/#73 at source; only the √8575 rider is inherited → 1-25. -->

**VERDICT: FAIL.** The calendar half is real and well-built. The probe half ranks an experiment that never ran, under a label saying it did — on a product whose entire value proposition is honesty.

- [x] [Review][CRITICAL → bug-log #81] **The macro loader is NEVER COMPILED; the arm has run 100% cash in every build since it shipped.** `macro_regime.rs` is `#![cfg(feature = "yahoo")]` on **`backtest`'s** feature; `backtest` has **no `default` stanza**; and **nothing in the workspace enables `backtest/yahoo`** (zero grep hits). The near-miss that hides it: `ui` has its own `yahoo` feature enabling `data/yahoo` — a *different crate's* feature, which Cargo does not unify. So `run_bakeoff` always takes the `cfg(not(yahoo))` branch → empty `PitSeries` → `as_of_value` always `None` → `prev_on` never leaves `false` → **cash for the entire window**, while the leaderboard renders *"Macro regime (hold when SPX up, DXY down, rates calm)"*. Orchestrator-verified end to end. **Strictly worse than #78's DVOL instance**: this machine HAS the full 2021-2026 corpus and the arm is still inert — no runtime state exists in which it works.
- [x] [Review][CRITICAL] **The two "graceful degradations" of this one arm are OPPOSITE, justified by a false equivalence in a comment.** `agent/src/runtime.rs` registers `AlwaysLongStrategy` for the forward paper loop, claiming it degrades *"exactly as `run_macro_gated_buyhold_path` does with an empty regime series"*. Verified false in both directions: the bake-off path holds **100% cash**, `AlwaysLongStrategy` holds **100% coin**. So the context that *ranks* the arm and the context that *executes* it disagree about which wrong strategy to substitute, both under the same honest-sounding label — and it defeats the anti-fake gate in that very file, which bails for an *unknown* arm and waves through a *known* one wearing a substitute.
- [x] [Review][CRITICAL — #78 instance OWNED HERE] **No drop-to-ABSENCE guard.** The DVOL sibling was fixed to `continue` out of the field when its series is unavailable; the macro arm is dispatched regardless. The tree's own comment now says so: *"the macro arm's degenerate path … is owned by story 3-16"*. Closing #78's behavioural half for this arm is this review's headline deliverable, and it is the same three lines.
- [x] [Review][HIGH] **Even with the feature on, the arm is inert TODAY.** The corpus ends `2026-06` (verified on disk, all three tickers; `generated_at = 2026-06-27`), the advisor window is NOW-anchored (`assert_eq!(end_ms, NOW)`), and the 100-day warm-up plus month-file enumeration means any current lookback hits a missing month → `CacheMiss` → `None` → cash. #78's second trigger on a second arm, through a different mechanism. Note the Yahoo wrinkle handled *correctly*: weekend/holiday gaps are fine, because LOCF carrying Friday's close across Saturday is right — the defect is that **LOCF has no maximum age**, so a 7-week hole is indistinguishable from a 3-day weekend at the join.
- [x] [Review][HIGH] **The pre-registered 3-AND rule — this story's entire scientific content — has NO test that binds it.** Both "risk-on rule" unit tests re-implement the predicate *inside the test body* and never call the loader; inverting the production rule leaves both green. And S3, the file's own "look-ahead leak-check", asserts only that two different regime series produce different equity — input-sensitivity, which S1 already establishes, not a PIT property; a forward-peeking implementation passes it unchanged. (Stated fairly: **S1/S2/S4 are genuine gates** and do NOT repeat the DVOL twin's 10%-vs-100% structural-divergence defect. The failure is scope, not construction.)
- [x] [Review][HIGH] **The arm trades for free, contradicting an explicit pre-registered cost clause.** The feature spec locked: *"transition trades pay the standard taker fee … the macro arm is NOT cost-advantaged vs the always-long benchmark."* Shipped: no fee, no slippage, `total_fees: zero`, while the 18 arms it is ranked against pay 4bps a leg through `PaperEngine` (and lot-rounding since #79). ADR-0073 records no decision to drop the clause. This makes the recorded phrase *"does not beat holding **net of costs**"* **literally false** — and it is bug-log #80's shape (asymmetric friction inside a ranked comparison) on a new axis. Direction note, stated fairly: the departure *flatters* the arm, so charging costs would strengthen the null, not reverse it.
- [x] [Review][HIGH] **The conclusion is over-scoped.** *"The macro risk-on/off regime signal does not beat holding"* rests on **one coin × one 6-month window × 6 regime flips (= 3 completed round trips)**, against a benchmark that returned >+20% on that window. The recorded −0.39% implies near-permanent cash, and **time-in-market — the one statistic that would make the comparison like-for-like — is recorded nowhere**. FRAGILE discriminated nothing: `all_active_fragile` was true across 19 arms, and the benchmark is structurally exempt from both the flag and eligibility. Not an AD-11 violation (the CHANGELOG appends a rate-regime qualifier), but the required form is *"on BTCUSDT, H1-2024, 3 regime episodes"*, not a channel verdict.
- [x] [Review][HIGH — methodological, the deepest finding of the burn-down] **Pre-registration was genuine, and that is what exposes the problem: the pre-registered expected outcome is byte-identical to the signature of every failure mode.** The rule literals were locked 2026-06-26, implemented 06-27, run 06-28, and the shipped predicate matches exactly — no sweep, no tuning. But the declared expectation was *"FRAGILE / does not beat hold is the EXPECTED, valid, shippable outcome"*, and corpus-absent, feature-off and window-outside-corpus **all produce exactly that**. A probe whose expected result equals its failure signature has no discriminating power without a positive control. One exists (6 flips, trades ≠ 0) — but it lives in a commit message and **no test asserts it**. The near-miss is instructive: the first decisive attempt failed loudly with `UnknownStrategy` because the arm had never compiled; had it instead compiled-and-degraded, the null would have been "confirmed" by a wiring artifact and nothing would have caught it.
- [x] [Review][MEDIUM] **The finding has no evidence artifact at all.** `evidence/v1/advisor-crossasset-macro-regime/` does not exist. The numbers (Sharpe −0.041 / −0.39% / MaxDD 8.85% / 6 flips) exist only as prose in a commit body, the trace comment and the CHANGELOG line — unreproducible by construction (`write_report=false`, corpus gitignored). "No anchor" was allowed to mean "no artifact".
- [x] [Review][MEDIUM — governance] **Provenance: the implementing commit is titled `Save all`** — 51 files, +2053/−60, mixing the feature with a `REVISION.toml`, 14 paper-soak artifacts, a launch.json, and unrelated doc cosmetics. `review_prep.sh` could not find it (it resolved to a later `refactor(spec)` commit); the diff had to be reconstructed by hand. **It also rewrote two ANCHORED report bodies** (`success-fixed-report-sample-{7d,90d}.md`). Orchestrator-verified: the changes are **frontmatter-only** (`period_end`/`run_id`/`ledger_snapshot_sha`) and `hash_report.py` strips frontmatter before hashing, so the body-SHA survived and anchors held 119/119 — **a near-miss, not a break**. But CLAUDE.md calls those files byte-immutable, and one hunk into the body would have broken AD-2 inside an unlabeled commit. This is precisely what #77's moral warns an omnibus hides.
- [x] [Review][MEDIUM] **The T-CAL "anchor-safety invariant" — which ADR-0073 names as the diff's top risk — is a tautology.** Every T-CAL test compares the production formula against a character-for-character copy of it re-declared inside the test module; `expected_bars_for_range` has **zero test call sites**. Mutate the production function arbitrarily and all seven stay green. The actual anchor exposure is nil (verified independently: all 12 Yahoo corpus tickers end `-USD` → `Crypto24x7`), so the *change* is safe — the *proof* is not.
- [x] [Review][Record — supersession, BOTH directions fire] **Backward**: the P2 rerun (2026-07-10) already documented this exact flat-arm degradation and wrote *"should be read as 'not meaningfully evaluable this run' rather than a genuine flat-performance data point"* — a month before #78 named it, never propagated back. **And it contaminated the multi-corpus evidence**: `p2_verdict_rerun.rs` hard-codes `macro_riskon: true` for the 2021-22 corpus whose warm-up reaches a `2020/` directory that does not exist → the arm ran cash across all ten symbols of the bear regime and was printed as an *evaluated candidate*. The same harness retain-filters DVOL out and steps over its macro neighbour in the same function. **Forward**: the only non-vacuous look-ahead proof this arm has — `macro_byte_identical_legacy_vs_with_lag_zero`, a test **inside this story's own file** — was written by ADR-0086/story 3-17, is cited in *that* row, and appears **zero times** here. Backfill it.
- [x] [Review][Chain — CLEAR, verified at source, do NOT route to 1-25] `run_macro_gated_buyhold_path` is a self-contained `BTreeMap` + `Decimal` loop: it constructs **no `Order`**, never calls `PaperEngine::step`, is single-symbol and touches no funding ledger. Therefore genuinely clear of **#67** (cross-symbol fills), **#69/#71** (exposure cap), **#72/#73** (funding accrual), and **#79/#80** are N/A by construction — though note the inverse is finding F6: paying *no* friction is itself the #80 asymmetry from the other side. **One rider inherited**: the published Sharpe rides √8575 (~1.07%, monotone, ranking-invariant) → 1-25 inventory, not re-reported.

**Status: stays `review`, does NOT close.** `done` is unreachable until at minimum the feature-gate and the drop-to-ABSENCE guard land, because the shipped product ranks an experiment that never ran under a label that says it did. The trace row's `state = "tested"` is honest and stays.

- [ ] `advisor-crossasset-macro-regime` 0.1.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/advisor-crossasset-macro-regime/` - frontmatter status **`tester-done`** (verbatim), version `0.1.0`, updated `2026-06-28`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-crossasset-macro-regime**`.
- Provenance: `git log -- spec/v1/advisor-crossasset-macro-regime` (full narrative); reports under `evidence/v1/advisor-crossasset-macro-regime/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-CROSSASSET-MACRO-REGIME-001` (state=`tested`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
