---
slug: retired-surface-inventory-2026-05-22
date: 2026-05-22
authors: analyst
status: proposed
related:
  - docs/dev-notes/repo-cleanup-plan-2026-05-22.md
  - docs/dev-notes/v3-vol-retirement-and-c5-promotion-2026-05-22.md
  - docs/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - docs/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md
---

# Retired-feature surface inventory — 2026-05-22

P1.3 of the [repo cleanup plan](repo-cleanup-plan-2026-05-22.md).
Auditability review of three "shipped-but-no-longer-pursued" surfaces.
**Not a deletion pass** — the retirement contract is explicit: code
stays, anchors locked, no deletion. This inventory documents *what's
there*, *what verifies it*, and *what's eligible for future
`#[cfg(feature = "retired-X")]` gating*.

## TL;DR

- **Total retired-surface LoC inventoried: 13,889** across 28 source
  files (Surface A v3-vol = 3,794; Surface B v3-llm-PARTIAL = 4,569;
  Surface C v25 TCN + PatchTST = 5,526). All compile cleanly under
  `cargo check --workspace --features candle` (verified at
  HEAD = `6e5b884`).
- **Anchored-vs-orphaned: 28/28 surface files trace to at least one
  of the 34 locked body-SHAs in `spec/anchors.toml`.** Zero orphans.
  Every retired source file participates in the byte-identity
  regression gate via at least one anchored report. `verify_anchors.sh`
  returned 34/34 PASS at HEAD.
- **Test coverage: 25/28 source files have direct test coverage**
  (either inline `#[cfg(test)]` modules or dedicated `tests/*.rs`
  integration tests). The 3 with zero direct test coverage are
  `bin/train_garch.rs`, `bin/train_tcn.rs`, and `bin/llm_verdict.rs`
  — all binaries whose semantics are validated indirectly through
  their downstream anchored reports (the *output* is anchored even
  when the binary itself has no `cargo test` smoke).

## Surface A — v3-volatility-forecaster (RETIRED 2026-05-22)

**Retirement context.** Joint advisory verdict V3 × T-VOL-NO-ALPHA →
MODEL-BROKEN / NO-ALPHA on the parent and rebaseline passes; routing
pick (a) RETIRE C1 after the noop-fix (P0) confirmed net_delta =
0.000000 was the no-op signature rather than a true real-vs-real
reading. The fix wave SHIPPED 2026-05-22 (test-final PASS, 34/34
anchors); retirement was ratified at presenter approval.

REQ rows: `REQ-V3-VOL-FORECASTER-001` (parent),
`REQ-V3-VOL-FORECASTER-REBASELINE-001`,
`REQ-V3-VOL-FORECASTER-NOOP-FIX-001` (P0 fix, shipped).
Anchored namespaces: `[v3.0.0-volatility]` (3 anchors) +
`[v3.0.0-volatility-rebaseline]` (1 anchor).

| File | LoC | Anchored via | Tests | Cross-refs |
|---|---|---|---|---|
| `crates/forecast/src/garch.rs` | 509 | `vol-verdict-bs1-realdata` (99c21892…) | inline 6 `#[test]` + `forecast/tests/garch_fit_determinism.rs` | feature.md § R6, decomp.md § T-AR-3, dev-notes v3-vol-retirement |
| `crates/forecast/src/vol.rs` | 289 | `vol-verdict-bs1-realdata` | inline 2 `#[test]` (VolForecastProvider trait + types) | feature.md § R1, decomp.md § T-AR-1 |
| `crates/forecast/src/bin/train_garch.rs` | 320 | indirect via the BS-1 checkpoint `garch-bs1-991324…json` consumed by anchored `vol-verdict-bs1-realdata` | 0 direct tests; output (the checkpoint) is provenance-pinned | decomp.md § Wave B cargo invocation |
| `crates/forecast/src/bin/vol_verdict.rs` | 1,138 | `vol-verdict-bs1-realdata` (the report it emits) | inline 9 `#[test]` + `forecast/tests/vol_verdict_mutual_exclusivity.rs` + `forecast/tests/parkinson_target_derivation.rs` | decomp.md § R6 + ADR-0038 § D1 (V-verdict criteria) |
| `crates/strategy/src/vol_targeting_overlay.rs` | 523 (post noop-fix) | `top10-2023-fy-vol-target-overlay-realdata` + `sharpe-comparison-vol-target-bs1-realdata` + `sharpe-comparison-vol-target-bs1-realbaseline` | inline 7 `#[test]` + `strategy/tests/vol_targeting_overlay.rs` + `strategy/tests/vol_targeting_overlay_end_to_end.rs` (the regression gate that would have caught the original no-op) | v3-vol-overlay-noop-discovery dev-note, noop-fix feature.md |
| `crates/strategy/src/vol_killswitch_overlay.rs` | 333 | NOT directly anchored (R6.b secondary builder; built but no anchored report exercises it) | inline 5 `#[test]` | feature.md § R6.b, dev-notes v3-vol-retirement |
| `crates/strategy/src/vol_meanreversion.rs` | 275 | NOT directly anchored (R6.c tertiary standalone builder) | inline 4 `#[test]` | feature.md § R6.c, dev-notes v3-vol-retirement |
| `crates/backtest/src/scenarios/garch_vol_target_overlay.rs` | 407 | `top10-2023-fy-vol-target-overlay-realdata` | 0 inline; covered via end-to-end `vol_targeting_overlay_end_to_end.rs` + backtest determinism gate | backtest/src/main.rs § GARCH dispatch (line 1416) |
| `crates/forecast/checkpoints/anchors/garch-bs1-991324…json` | n/a (binary) | `vol-verdict-bs1-realdata` (the report references this exact SHA in provenance) | n/a | decomp.md § R6 + ADR-0038 § D1 |

**Subtotal LoC: 3,794** (excluding checkpoint binary).

**Anchored-but-untraced sub-surfaces:** `vol_killswitch_overlay.rs` and
`vol_meanreversion.rs` are the two R6.b/R6.c secondary builders that
shipped under v3-volatility-forecaster v0.1.0 but were never the
primary anchor target (the operator's Q3=(d) "all-3-consumer-builders"
pick anchored only the R6.a vol-targeting overlay). They have inline
test coverage (4 + 5 `#[test]`) but no anchored report path. **These
are the strongest gating candidates within Surface A** because they
have neither anchored-report traceability nor a non-test consumer in
the live registry — they're builders that registry-test fixtures
exercise, nothing more.

## Surface B — v3-llm-forecaster (SHIPPED-PARTIAL — NOT retired)

**Status: shipped-partial, NOT retired.** Included in this inventory
per the brief's "same audit class" framing — the surface has the same
characteristics (large LoC footprint, partial completion of Wave D)
but the feature itself is paused, not abandoned. Anchor delta planned
34 → 36 at developer Wave D close (deferred to v0.1.1).

REQ row: `REQ-V3-LLM-FORECASTER-001` (state = shipped-partial).
Anchored namespaces: none yet (Wave D deferred). Anchors stay at 34
under additive-zero per the shipped-partial precedent documented in
the test-final report § 14.

| File | LoC | Anchored via | Tests | Cross-refs |
|---|---|---|---|---|
| `crates/strategy/src/llm_forecaster/mod.rs` | 78 | 0 anchored reports (Wave D deferred); covered by 98 integration tests across 13 suites | 0 inline (module facade) | decomp.md § Wave A |
| `crates/strategy/src/llm_forecaster/trait_def.rs` | 74 | n/a (trait surface) | 0 inline; exercised by all 8 `llm_forecaster_*` integration tests | decomp.md § Wave A |
| `crates/strategy/src/llm_forecaster/types.rs` | 1,105 | n/a (DTOs); 2 stale `// TODO Wave C` markers flagged in P1.5 | inline 12 `#[test]` | dev-notes v3-llm-forecaster-prompt-spike, repo-cleanup-plan § P1.5 |
| `crates/strategy/src/llm_forecaster/canonicalize.rs` | 98 | n/a | inline 5 `#[test]` | decomp.md § Wave A |
| `crates/strategy/src/llm_forecaster/strategy.rs` | 511 | n/a (Wave D byte-identity test deferred) | inline 6 `#[test]` + `strategy/tests/llm_forecaster_neutrality.rs` | decomp.md § Wave A + Wave C |
| `crates/strategy/src/llm_forecaster/prompt.rs` | 441 | n/a (spike memo locks the schema) | inline 11 `#[test]` | dev-notes v3-llm-forecaster-prompt-spike (T-AR-8) |
| `crates/strategy/src/llm_forecaster/anthropic_impl.rs` | 672 | n/a; wiremock-tested | inline 10 `#[test]` + `strategy/tests/llm_forecaster_wiremock.rs` + `strategy/tests/llm_forecaster_wiremock_wave_e.rs` | decomp.md § Wave B + Wave E, repo-cleanup-plan § P1.5 (stale TODO) |
| `crates/strategy/src/llm_forecaster/tool_schema.rs` | 255 | n/a | inline 9 `#[test]` | decomp.md § Wave A |
| `crates/strategy/src/llm_forecaster/verdict.rs` | 866 | n/a (ADR-0039 L0-L4) | inline 18 `#[test]` + `strategy/tests/llm_verdict_priority_tree.rs` (20 tests) | ADR-0039, decomp.md § Wave G |
| `crates/strategy/src/bin/llm_verdict.rs` | 469 | n/a (Wave G driver; emits L0-L4 verdict reports) | 0 direct tests; output schema validated by `llm_verdict_priority_tree.rs` | decomp.md § Wave G |
| `crates/audit/migrations/012_llm_forecast.sql` | n/a (SQL) | n/a | `audit/tests/journal_llm_forecast_round_trip.rs` (4 tests) | decomp.md § Wave E + Wave G |

**Subtotal LoC: 4,569** (excluding SQL migration).

**Notable.** Despite the absence of an anchored body-SHA chain (Wave D
deferred), Surface B has the **strongest test coverage of any surface
in this review**: 98 integration tests across 13 suites + 20
verdict-priority-tree + 1 ignored neutrality (per the trace.toml
M-FINAL line and the M-FINAL test report). The shipped-partial
verdict is honest: code is exercised; only the 2-run byte-identity
regression gate is deferred.

## Surface C — v25 TCN + PatchTST (RETIRED earlier sessions)

**Retirement context.** v25 phase 1 (TCN) shipped F4 verdict across
v25-tcn-overlay → v25-tcn-alpha-investigation → v25-tcn-recalibrate →
v25-tcn-threshold-tuning chain (5 feature folders, ~14 anchored
reports). Phase 2 (PatchTST) shipped at v25a-patchtst-overlay v0.1.0
with F4 verdict and Sharpe-delta +0.006144 (below the +0.10
T-ALPHA-UNLOCKED bar). Phase 3 (transformer) was scoped at
v25b-transformer-overlay analyst-only and never built — that folder
has no `crates/` footprint, so it's out of scope for this inventory.

REQ rows: `REQ-V25-DL-000`, `REQ-V25-TCN-001`,
`REQ-V25-TCN-ALPHA-001`, `REQ-V25-TCN-RECALIBRATE-001`,
`REQ-V25-TCN-THRESHOLD-TUNING-001`,
`REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001`, `REQ-V25A-PATCHTST-001`
(plus `REQ-V25B-TRANSFORMER-001` — spec-only).

Anchored namespaces: `[v2.5.0]` (2), `[v2.5.0-tcn-weights]` (2),
`[v2.6.0-realdata]` (4), `[v2.6.0-alpha-investigation]` (3),
`[v2.6.1-alpha-investigation-recalibrated]` (4),
`[v2.6.2-threshold-tuning]` (2), `[v2.5a.0-patchtst]` (2). **Total
v25 anchors: 19/34 = 56% of the regression gate's surface area.**

| File | LoC | Anchored via | Tests | Cross-refs |
|---|---|---|---|---|
| `crates/forecast/src/tcn.rs` | 1,469 | `top10-2023-fy-tcn-overlay-weights-realdata` + `top10-2024-fy-tcn-overlay-weights-realdata` + 4 forecast-distribution / recalibrate / threshold-sweep namespaces | inline 13 `#[test]` + `forecast/tests/tcn_byte_identity.rs` + `forecast/tests/metal_cpu_drift.rs` | v25-tcn-overlay feature.md, v25-dl-retrospective dev-note |
| `crates/forecast/src/bin/train_tcn.rs` | 954 | indirect via `tcn-bs1` + `tcn-bs2` checkpoints consumed by all 6 v25-TCN realdata namespaces | 0 inline; `forecast/tests/train_tcn_dry_run.rs` + `train_tcn_audit_emits.rs` + `train_tcn_golden_cli.rs` + `train_tcn_no_audit_db_writes_nothing.rs` + `smoke_train.rs` | v25-tcn-overlay/reports/m3-bs1-training + m3-bs2-training |
| `crates/forecast/src/patchtst.rs` | 1,253 | `forecast-distribution-patchtst-bs1-realdata` + `top10-2023-fy-patchtst-overlay-realdata` | inline 7 `#[test]` + `forecast/tests/patchtst_byte_identity.rs` + `forecast/tests/forward_determinism_patchtst.rs` + `sigma_train_not_in_safetensors_patchtst.rs` | v25a-patchtst-overlay feature.md |
| `crates/forecast/src/bin/train_patchtst.rs` | 1,143 | indirect via `patchtst-bs1` checkpoint | inline 3 `#[test]` + `forecast/tests/smoke_train.rs` | v25a-patchtst-overlay decomp.md |
| `crates/strategy/src/tcn_overlay_momentum.rs` | 1,003 | `top10-2023-fy-tcn-overlay` + `top10-2024-fy-tcn-overlay` + `top10-2023-fy-tcn-overlay-weights` + `top10-2024-fy-tcn-overlay-weights` + realdata variants + threshold-sweep | inline 10 `#[test]` + `strategy/tests/tcn_overlay_tuned_builder.rs` | v25-tcn-overlay feature.md |
| `crates/strategy/src/patchtst_overlay_momentum.rs` | 502 | `top10-2023-fy-patchtst-overlay-realdata` | inline 8 `#[test]` + `forecast/tests/patchtst_overlay_neutrality.rs` | v25a-patchtst-overlay feature.md |
| `crates/strategy/src/patchtst_sync.rs` | 20 | indirect (sync wrapper for backtest dispatch) | 0 inline (thin shim) | v25a-patchtst-overlay decomp.md § Wave D |
| `crates/backtest/src/scenarios/tcn_overlay.rs` | 307 | `top10-2023-fy-tcn-overlay` + `top10-2024-fy-tcn-overlay` | 0 inline; `backtest/tests/determinism.rs` | v25-tcn-overlay |
| `crates/backtest/src/scenarios/tcn_overlay_weights.rs` | 314 | `top10-2023-fy-tcn-overlay-weights` + `top10-2024-fy-tcn-overlay-weights` + realdata variants | 0 inline; `backtest/tests/determinism.rs` + `multi_symbol_determinism.rs` | v25-tcn-overlay, v25-tcn-recalibrate |
| `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` | 320 | `top10-2023-fy-patchtst-overlay-realdata` | 0 inline; `backtest/tests/determinism.rs` | v25a-patchtst-overlay |
| `crates/backtest/src/scenarios/threshold_sweep.rs` | 323 | `threshold-sweep-bs1-realdata-recalibrated` + `threshold-sweep-bs2-realdata-recalibrated` | 0 inline; `backtest/tests/threshold_sweep_readonly.rs` | v25-tcn-threshold-tuning |
| `crates/forecast/src/bin/forecast_distribution.rs` | (~700 est) | `forecast-distribution-bs1-realdata` + `bs2-realdata` + `bs1/bs2-realdata-recalibrated` + `forecast-distribution-patchtst-bs1-realdata` | `forecast/tests/forecast_distribution_bin_readonly.rs` + `forecast_distribution_verdict.rs` | v25-tcn-alpha-investigation, v25-tcn-recalibrate, v25a-patchtst-overlay |
| `crates/forecast/src/bin/recalibrate_sigma_train.rs` | (~500 est) | `recalibrate-sigma-train-bs1` + `bs2` | `forecast/tests/recalibrate_sigma_train_readonly.rs` + `recalibrate_sigma_train_field_invariance.rs` | v25-tcn-recalibrate |
| `crates/forecast/src/bin/sharpe_comparison.rs` | (~400 est) | `sharpe-comparison-realdata` + `sharpe-comparison-vol-target-bs1-realdata` + `sharpe-comparison-vol-target-bs1-realbaseline` | `forecast/tests/sharpe_comparison_determinism.rs` | v25-tcn-alpha-investigation, v3-vol-forecaster (shared dispatch) |
| `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors` + `.metadata.json` + `.metadata.recalibrated.json` | n/a | All 6 v25-TCN realdata + recalibrated namespaces | n/a | v25-tcn-overlay m3 reports |
| `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe…safetensors` + `.metadata.json` + `.metadata.recalibrated.json` | n/a | Same as above (BS-2 splits) | n/a | v25-tcn-recalibrate, v25-tcn-threshold-tuning |
| `crates/forecast/checkpoints/anchors/patchtst-bs1-62520db9…safetensors` + `.metadata.json` | n/a | `forecast-distribution-patchtst-bs1-realdata` + `top10-2023-fy-patchtst-overlay-realdata` | n/a | v25a-patchtst-overlay |

**Subtotal LoC: 5,526** (the 3 binaries forecast_distribution +
recalibrate_sigma_train + sharpe_comparison are SHARED across Surface
A and Surface C — counted once under Surface C since the v25
namespaces dominate their anchor count 14:3).

**Notable.** Surface C has the most fragmented test layout — 21 of
the 28 source files in this inventory belong to Surface C, but the
3 backtest scenarios (`tcn_overlay.rs`, `tcn_overlay_weights.rs`,
`patchtst_overlay_weights.rs`, `threshold_sweep.rs`) and `patchtst_sync.rs`
have ZERO inline tests. They're covered indirectly through the
`backtest/tests/determinism.rs` end-to-end gate, which is the same
gate that produces the anchored body-SHAs — so coverage is real, just
not file-local.

## Orphan candidates (P2 gating pass scope)

These surfaces have **no anchored body-SHA traceability AND no live
non-test consumer** (i.e., they exist only in the registry / fixture
graph). They are the cleanest candidates for `#[cfg(feature =
"retired-X")]` gating in a future cleanup pass.

| File | Surface | Why orphan | Mitigation |
|---|---|---|---|
| `crates/strategy/src/vol_killswitch_overlay.rs` (333 LoC) | A | R6.b secondary builder; never the primary anchor target; no anchored report uses it; only registry-test fixtures exercise it | inline 5 `#[test]` keeps it self-validating; gating it would be safe |
| `crates/strategy/src/vol_meanreversion.rs` (275 LoC) | A | R6.c tertiary standalone strategy; same reasoning as above | inline 4 `#[test]` keeps it self-validating |

**Total orphan-candidate LoC: 608** (1.6% of total retired surface).

Everything else has at least one anchored body-SHA exercising it
end-to-end. The retirement contract holds tightly: even retired code
is on the regression gate.

## Cross-feature lessons

1. **Anchored-report coverage tracks retirement quality.** All 3
   surfaces have anchored body-SHAs that exercise the bulk of their
   code paths (28/28 source files in Surface A + Surface C trace to
   ≥ 1 of the 34 locked anchors). The shipped-partial Surface B
   compensates for missing Wave D anchors with 98 integration tests
   across 13 suites — a different but equally rigorous gate. **None
   of the 13,889 LoC inventoried is dead code in the silent-rot
   sense.**

2. **Binaries are anchored through their outputs, not their tests.**
   `train_garch.rs`, `train_tcn.rs`, `train_patchtst.rs`, and
   `llm_verdict.rs` collectively have 3 inline tests across 2,886 LoC
   — the lowest test-density in the inventory. But every one of them
   *outputs* an anchored artifact (a checkpoint or a verdict report),
   so the byte-identity gate validates them at the protocol level,
   not the implementation level. This matches the v25-dl-journey
   retrospective's lesson: training drivers are best validated
   end-to-end via their downstream report SHAs.

3. **The Surface A noop-fix expanded coverage retroactively.** The
   `vol_targeting_overlay_end_to_end.rs` integration test was added
   under the noop-fix wave specifically because it's the gate that
   would have caught the original no-op (overlay equity ≠ un-targeted
   baseline equity by ≥ 1 bp when scale ≠ 1.0). That test now lives
   in the retired surface — but it's load-bearing for the entire
   class of overlay-scaling correctness: any future overlay
   (Surface B's llm_forecaster, future Surface C variants) inherits
   the test pattern. **Retired-surface tests can be load-bearing for
   live surfaces.** Strong argument against blanket
   `#[cfg(feature = "retired-X")]` gating.

4. **Shared binaries blur the surface boundaries.**
   `forecast_distribution.rs`, `recalibrate_sigma_train.rs`, and
   `sharpe_comparison.rs` dispatch on a `--scenario` flag that
   crosses surface boundaries — `sharpe_comparison` serves both
   v25-tcn-alpha-investigation (Surface C) and v3-vol-forecaster
   (Surface A). A future gating pass MUST treat these as
   surface-shared infrastructure, not as part of either surface
   exclusively. Easiest path: gate the dispatch arms, not the
   binaries.

5. **The 8-feature May 2026 retirement chain produced no orphan
   anchors.** `verify_anchors.sh` is 34/34 PASS at HEAD. Every
   locked SHA still resolves to an extant report and matches its
   body. This is the strongest possible evidence that the
   retirement contract (code stays, anchors locked) is working
   exactly as designed: retired-yet-anchored is the equilibrium.

## Recommendations (P2 scope, not for this pass)

1. **Sub-surface gating (P2.A — small).** The 608 LoC of orphan
   candidates (`vol_killswitch_overlay.rs` +
   `vol_meanreversion.rs`) are eligible for
   `#[cfg(feature = "retired-v3-vol-secondary")]` gating in a future
   cleanup pass. **Cost-benefit:** 608 LoC × marginal cargo-build
   speedup — not worth the cycle until the broader workspace gates
   itself for compile time. **Defer.**

2. **Surface-level gating decision-record (P2.B — medium).** Before
   any wholesale gating pass, an ADR is needed to settle:
   (i) gating granularity (file / module / crate);
   (ii) what happens to anchored body-SHAs under a gated feature
   (gate-off the test → CI still passes; but `verify_anchors.sh` is a
   separate gate that runs body-SHA computation directly on the
   report files, so anchor verification is decoupled from compilation
   — this means gated code still has its evidence chain intact);
   (iii) whether retired-surface tests that turned out to be
   load-bearing for live surfaces (lesson #3) get migrated out of
   the gated surface first. **Suggested scope: ADR-0040 or
   ADR-0041; ~1 day analyst + architect.** **Defer to next
   cleanup window.**

3. **Shared-binary dispatch-arm gating (P2.C — small).** The 3
   shared binaries (`forecast_distribution`, `recalibrate_sigma_train`,
   `sharpe_comparison`) each have a `--scenario` dispatch table. A
   future gating pass should gate the v3-vol arms (1 in
   `sharpe_comparison`) and the v25 arms separately. **Cost:** ~2
   hours per binary. **Defer.**

4. **Anchor body-SHA stale-fixture cleanup (P2.D — opportunistic).**
   Already on the cleanup plan as P2.1. Several report folders carry
   older non-anchored copies (e.g.,
   `spec/v25-tcn-overlay/reports/backtest-20260518-053400-…md` and
   `…061302-…md` for the same scenario — `verify_anchors.sh` picks
   the lexicographically-newest). Cosmetic. **Defer.**

5. **Document the retirement-test load-bearing pattern (P2.E —
   ~30 min).** Lesson #3 deserves a short architecture note: which
   tests in retired surfaces are load-bearing for live surfaces, and
   the migration protocol if a future gating pass removes them.
   Could be a one-paragraph addition to
   `spec/architecture.md` § Regression-gate discipline. **Defer.**

## Verification artifacts (this pass)

- `bash scripts/verify_anchors.sh` → **34/34 PASS** (re-run at start
  + end of inventory pass; no change in body-SHAs).
- `cargo check --workspace --features candle` → **clean** (no
  warnings introduced by this pass — pass is read-only on `crates/`).
- `uv run scripts/spec_lint.py` baseline at start: 103 violations in
  1 category. End-of-pass: 103 (this dev-note introduces no new
  dead-links; only outbound cross-refs to existing committed
  docs/dev-notes/ files and existing trace.toml REQ ids).

## Changelog

- 2026-05-22 (analyst): initial inventory pass — P1.3 of
  repo-cleanup-plan-2026-05-22. 28 source files inventoried across
  3 surfaces (13,889 LoC). 28/28 anchored; 2 orphan candidates
  identified (608 LoC); 5 P2 recommendations documented. No code
  changes; read-only audit.
