---
slug: v3-volatility-forecaster-rebaseline
mode: release
status: draft
audience: human-operator
updated: 2026-05-22
generated: 2026-05-22T12:15:00Z
version: 0.1.0
commit: 596baeb641adbb047d7951b692a0ad4e2d17c949
predecessor: v3-volatility-forecaster v0.1.0 (shipped 2026-05-22 with MODEL-BROKEN / NO-ALPHA advisory + data caveat)
parent: v3-volatility-forecaster
---

# v3 volatility forecaster — re-baseline pass — release deck

## Operator headline

The (b) re-baseline pick paid off. **One day** of additive scope swapped the
synthetic v1 momentum baseline for a real-Binance one, and the apples-to-apples
comparison **kills the data caveat outright**: `net_delta = 0.000000`. The
parent's joint advisory **`V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA`** now
stands without footnote on real-vs-real evidence. Anchor gate **34 / 34 PASS**;
parent anchor `ef048366ac5…` byte-identical (architect's T-AR-2 immutability
correction held). Routing R-O1 fires deterministically → **(a) RETIRE C1**.

**Recommended pick:** **(a) RETIRE C1** + promote C2 _or_ C5 from Queue → Active.
Presenter advises against (c) DEBUG V3 and (d) v0.1.1 refit (see § Routing
decision for the reasoning). The C2-vs-C5 promotion pick is the operator's
genuine strategic call — presenter does not push either way.

## Headline evidence

- **Pre-lock anchor gate held:** parent `sharpe-comparison-vol-target-bs1-realdata`
  body-SHA `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`
  byte-identical after Wave A + B + C. Architect's T-AR-2 correction (NEW
  `ScenarioFamily::VolTargetRebaseline` enum variant instead of a hard-coded
  swap that would have mutated the parent body) preserved ADR-0038 § D6
  anchor-additive contract end-to-end.
- **net_delta moved by −0.029868** (parent synthetic-baseline +0.029868 → this
  pass real-baseline 0.000000). The synthetic comparison was **favorable** to
  the overlay; the real comparison is **brutal**. Both Sharpe sides land
  identically at 0.003098 — the GARCH vol-targeting overlay contributes zero
  incremental alpha over the un-targeted v1 momentum baseline on real Binance
  2023 hourly data.
- Verbatim from the new sharpe-comparison report § Notes:
  > "apples-to-apples comparison per v0.1.0-rebaseline disambiguation"

## Routing decision (operator picks one)

**One decision. Variable budget implications. This is the load-bearing call —
presenter does not autoapprove.**

| Path | Action | Budget | Presenter take |
|------|--------|--------|----------------|
| **(a) RETIRE C1** ⟵ **recommended** | Accept joint verdict at face value on real-vs-real evidence. Free C1 budget. Promote C2 (regime-classifier) or C5 (LLM-forecaster) from Queue → Active. | 0 (frees ~3-4 weeks) | net_delta = 0.000000 + V3 + caveat ruled out is decisive. Mirrors the v2.5 DL retire pattern (joint F4-F4-F4 → retire; see `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`). |
| **(c) DEBUG V3** | Spawn `v3-garch-calibration-tune` for per-symbol α/β hyperparameter search + Garman-Klass fallback for non-convergent symbols. | ~2-3 weeks | Advised against. Calibration repair could move ratio 2.95 → ~1.0 on AVAX/DOGE/DOT, but the **overlay-structure** finding (zero alpha at v2 baseline+overlay scale) is harder to refute than the calibration finding. Two conditions both look unlikely. |
| **(d) v0.1.1 GARCH refit + return** | Same fitter; iterate hyperparameters in place; bump version → v0.1.1. | ~2-3 days | Advised against. Smaller-scoped variant of (c) with the same skepticism applied. Single iteration unlikely to flip a zero net_delta. |

### Decision tree

```
Operator-decide (post-rebaseline):
  └── do you believe per-symbol GARCH calibration repair would flip a
      net_delta of 0.000000 to a banked alpha?
       ├── NO → (a) RETIRE C1 → promote C2 or C5 from Queue → Active
       └── YES, worth multi-week budget → (c) DEBUG V3
       └── YES, but only single-iteration budget → (d) v0.1.1 GARCH refit
```

### Why presenter recommends (a)

Two independent failure modes now confirmed on apples-to-apples evidence:
**V3** (GARCH model-broken; calibration ratio 2.952191) AND **T-VOL-NO-ALPHA**
(zero net delta). Per ADR-0038 § D1.b: any V1/V2/V3 collapses to MODEL-BROKEN
regardless of T-classifier — V3 ALONE is sufficient grounds for retire. Adding
NO-ALPHA on top removes the last ambiguity. The v2.5 DL programme retire (joint
F4-F4-F4 across 3 model checkpoints / 2 model families / 2 horizons) was a
weaker evidence base than this rebaseline's net_delta = 0.000000, and the
operator chose retire there. **Same pattern.**

## If (a) RETIRE C1 — promotion candidate pick (operator chooses)

Both candidates have full analyst spec briefs as of 2026-05-22. **Presenter
does not push either way — this is a strategic call about novelty vs reuse.**

| Candidate | Slug | Budget | Reuse seed | Risk profile |
|-----------|------|--------|------------|--------------|
| **C2** | [`v3-regime-classifier`](../../v3-regime-classifier/feature.md) | ~4-6 weeks | `crates/reflection/src/regime.rs` already ships a pure-fn 3-state BTC daily-close tagger (Bull/Bear/Chop) with `RegimeTag` enum + `classify_regime` fn + 5+ live consumers and stable byte-identity invariants. C2 extends to multi-symbol classification feeding regime-aware strategy switching. | LOWER novelty, LOWER variance. Closer to a proven idea; cheapest-seed candidate from the survey. |
| **C5** | [`v3-llm-forecaster`](../../v3-llm-forecaster/feature.md) | ~6-8 weeks | `crates/llm` foundation (v2-llm-strategy v2.0.0 shipped 2026-05-13) — `LlmProvider` trait + 3 provider impls + `BudgetedProvider` decorator. C5 builds a reflection-memory + audit-anchored LLM-as-forecaster signal. | HIGHER novelty, HIGHER variance. **Best product.md moat alignment** — survey rated C5 as the (2)+(4) differentiator compounder (persistent memory + audit ledger as a signal source). Information-theoretically independent of DL-on-OHLCV. |

The C1+C2+C5 HYBRID sequencing from the 2026-05-22 strategy-reformulation
survey assigned ~6-8 weeks total budget across the three. C1 consumed ~1 day
(re-baseline included). The remaining ~5-7 weeks comfortably funds either C2
alone, C5 alone, or C2 then C5 sequentially if the operator wants to bank C2
first and consume C5 as a follow-on. The operator may also opt to fund
**neither** and shift bandwidth to UI / ops / paper-trade-live — that's a
valid choice per the survey's "picking nothing" clause.

## What landed

- **1 new scenario** `top10-2023-fy-momentum-realdata` in `Scenario::from_name`
  ([`crates/backtest/src/main.rs:769`](../../../crates/backtest/src/main.rs)).
  By-value-equal to the parent vol-target-realdata arm on every non-strategy
  field per architect T-AR-1 lock.
- **1 new `ScenarioFamily::VolTargetRebaseline`** enum variant + out-dir
  dispatch arm + new dispatch arm at
  [`crates/forecast/src/bin/sharpe_comparison.rs:59`](../../../crates/forecast/src/bin/sharpe_comparison.rs) +
  sibling `render_vol_target_rebaseline` module (~250 LoC; no shared-extract,
  duplication is advisory strings only). Parent `VolTarget` arm byte-identical.
- **1 new anchor** under NEW `[v3.0.0-volatility-rebaseline]` namespace block in
  `spec/anchors.toml`:
  - `sharpe-comparison-vol-target-bs1-realbaseline` →
    `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`
- **1 architecture deviation** flagged: `MomentumScenarioInput` gained
  `bars_override: Option<Vec<Bar>>` + `data_revision_sha: Option<String>` fields
  ([`cli_types.rs:44-66`](../../../crates/backtest/src/cli_types.rs)); `momentum::run`
  uses `bars_override` when provided
  ([`scenarios/momentum.rs:200-242`](../../../crates/backtest/src/scenarios/momentum.rs));
  `is_momentum` dispatch extended for `RealData`; `report::momentum::write`
  emits `data_revision_sha` in frontmatter. ~100 LoC across 4 files vs decomp's
  ~25 LoC estimate. **Additive-only**; synthetic path byte-identical; all 33
  parent anchors verified byte-identical. Non-blocking — flagged for next audit
  pass.
- **3 new render tests** ship in the developer's diff:
  `render_vol_target_rebaseline::tests::t_classifier_thresholds`,
  `…::render_contains_required_sections`,
  `…::render_is_deterministic`.
- **ADR-0038 § D6 anchor-additive contract held** through Wave B (the critical
  architect correction at T-AR-2).
- **Budget:** ~1 day end-to-end (architect M-T1 + dev Waves A+B+C + tester
  M-FINAL); ~2 hours of dev wall-clock per the developer's row-level invocation
  log. The (b) re-baseline pick was the highest-EV play available at the parent
  deck and it paid.

## Live demo — anchor gate 34 / 34 (verbatim tail, tester run)

```
PASS  vol-verdict-bs1-realdata              99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21
PASS  top10-2023-fy-vol-target-overlay-realdata  66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65
PASS  sharpe-comparison-vol-target-bs1-realdata  ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1
PASS  sharpe-comparison-vol-target-bs1-realbaseline  d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8
---
ANCHORS PASS  (34 / 34)
```

Parent anchor `ef048366ac5…` byte-identical post-Wave-B — the load-bearing
confirmation that T-AR-2's NEW-enum-variant approach (vs the brief's
hard-coded-swap default) correctly preserved ADR-0038 § D6.

## Verification matrix

| V-id | Gate | Status | Evidence |
|------|------|--------|----------|
| V-R1 | REAL-data baseline scenario | VERIFIED | New scenario `top10-2023-fy-momentum-realdata` registers with `data_source: ScenarioDataSource::RealData`; report frontmatter shows `data_source: real (Binance Vision via data/binance/, v3.0.0-volatility-rebaseline)`. |
| V-R2 | Baseline provenance pinned per ADR-0032 | VERIFIED | `data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` matches parent vol-target-realdata exactly. |
| V-R3 | T-classifier re-evaluated on new net_delta | VERIFIED | net_delta = 0.000000 < +0.05 → T-VOL-NO-ALPHA per ADR-0038 § D1.c. V-verdict V3 carried forward verbatim (H-rebase-2 confirmed). |
| V-R4 | Anchor-additive through report emission | VERIFIED | `ANCHORS PASS (34 / 34)`; +1 row under NEW `[v3.0.0-volatility-rebaseline]` namespace; existing 3 `[v3.0.0-volatility]` anchors byte-identical. |
| V-R5 | 2-run byte-identity determinism | VERIFIED | Tester recomputed body-SHA `d561fed564…` matches developer's 2-run claim; tester-independent verification. |
| V-cargo | fmt / clippy / 311 tests | VERIFIED | `cargo fmt --check` clean; clippy `-D warnings` clean; `311 passed; 0 failed`. |
| V-H1 | H-rebase-1 (real-vs-real reveals net_delta movement) | CONFIRMED | Parent net_delta 0.029868 → this pass 0.000000; delta movement −0.029868; T-classifier unchanged. |
| V-H2 | H-rebase-2 (V3 survives baseline swap) | CONFIRMED | V3 carries forward byte-identical; calibration ratio = 2.952191 is a GARCH-only diagnostic on the unchanged vol-verdict-bs1-realdata report. |

## Numbers that matter

- **net_delta** — **0.000000** (real-vs-real). Parent was +0.029868
  (synthetic-vs-real); delta moved by **−0.029868**.
- **Sharpe baseline** — **0.003098** (top10-2023-fy-momentum-realdata).
- **Sharpe overlay** — **0.003098** (top10-2023-fy-vol-target-overlay-realdata).
- **T-classifier** — **T-VOL-NO-ALPHA** (< +0.05 threshold).
- **V-verdict** — **V3** unchanged (mean_calibration_ratio = 2.952191 outside
  [0.7, 1.4]).
- **Joint advisory** — **MODEL-BROKEN / NO-ALPHA**, **no caveat**.
- **Total return / max drawdown** — **+13.48% / 73.73%** both columns
  (identical to 4-decimal precision; co-incident equity curves).
- **Trades** — 6203 both columns.
- **Anchors** — 33 (pre-lock) → **34 PASS** (post-lock).
- **Parent anchor** `ef048366ac5…` — byte-identical (anchor-additive contract
  held; architect T-AR-2 correction validated).
- **Tests** — **311 passed / 0 failed / 0 ignored**.
- **Routing cell** — **R-O1** (T-VOL-NO-ALPHA + PASS) → (a) RETIRE C1.

## Open decisions

1. **Routing R-O1 confirm (load-bearing).** Pick **(a) / (c) / (d)** from the
   routing table above. Presenter recommends (a). Standing Autoapprove from the
   2026-05-22 prior session does **not** apply — this is a strategic
   programme-retire call, presenter explicitly does not autoapprove.
2. **C2-vs-C5 promotion (if (a)).** Pick C2 (`v3-regime-classifier`) or C5
   (`v3-llm-forecaster`) to move from Queue → Active. **Presenter does not
   recommend either way** — genuine operator strategic call (novelty risk
   appetite vs reuse-seed cheapness).

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Routing pick (operator selects exactly one)

- [ ] (a) RETIRE C1 — accept joint verdict on real-vs-real evidence; promote
  C2 _or_ C5 from Queue → Active (note which one below)
- [ ] (c) DEBUG V3 — spawn `v3-garch-calibration-tune` for per-symbol α/β
  hyperparameter search + Garman-Klass fallback
- [ ] (d) v0.1.1 GARCH refit + return — in-place fitter iteration; bump version
  → v0.1.1

### Promotion candidate (operator selects exactly one if (a))

- [ ] C2 — `v3-regime-classifier` (~4-6 weeks; LOWER novelty / reuse-seed
  cheapest)
- [ ] C5 — `v3-llm-forecaster` (~6-8 weeks; HIGHER novelty / best moat
  alignment)
- [ ] Neither — shift bandwidth to UI / ops / paper-trade-live

### Notes / feedback

_(operator fills in routing choice rationale, C2-vs-C5 pick, or rejection
reason if rejected)_

## Closing gates

Both mechanical gates run on this presentation file:

```
$ bash scripts/check_presentation.sh spec/v3-volatility-forecaster-rebaseline/presentations/v3-volatility-forecaster-rebaseline-2026-05-22.md
PRESENTATION CHECK PASS  (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/spec/v3-volatility-forecaster-rebaseline/presentations/v3-volatility-forecaster-rebaseline-2026-05-22.md — approval block UN-ticked)
```

```
$ python3.14 scripts/spec_lint.py 2>&1 | head -1
spec-lint: FAIL (85 violations in 1 categories)
```

**Baseline match: 85 / 1**, identical to the tester's recorded post-T-T2
baseline in `reports/test-final-2026-05-22.md § 10`. **No new categories or
count growth introduced by this presentation file.** The 85 pre-existing
dead-links are all in files unrelated to this feature (ADR-0027 Kronos
cross-refs, chart-canvas-overhaul /tmp screenshot refs, journal-transactions-metadata
report path refs, v0-paper-sma screenshot README, lumen-design-adoption phase-5
feature refs, live-cockpit-unified feature refs, v3-llm-forecaster crate path
refs, v2-llm-strategy/v2-llm-strategy/tasks.md self-ref). Zero violations in
any `spec/v3-volatility-forecaster-rebaseline/` file per tester T-T2.

## Sources cited

- [`feature.md`](../feature.md) — feature brief v0.1.0 § Verification + §
  Routes + § Implementation (architect T-AR-2 critical correction recorded).
- [`tasks.md`](../tasks.md) — T-A1..T-A3 + T-OD1..T-OD3 (standing Autoapprove)
  + T-AR-1..T-AR-4 + T-D-N1..T-D-N12 + T-T-1..T-T-4 + T-P1 (this row).
- [`decomp.md`](../decomp.md) — architect M-T1 decomposition + T-AR-2
  anchor-immutability correction + § 6 anchor-block verbatim shape.
- [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md) —
  tester M-FINAL `VERDICT → PASS`; 311/0/0 tests; 34/34 anchors; spec-lint
  85/1 (no new categories).
- [`reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md`](../reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md)
  — load-bearing evidence file (body-SHA
  `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`).
- [`reports/backtest-20260522-095222-top10-2023-fy-momentum-realdata.md`](../reports/backtest-20260522-095222-top10-2023-fy-momentum-realdata.md)
  — new real-data un-targeted v1 momentum baseline backtest
  (data_revision_sha `3a8b96c43f…`).
- [Parent presenter deck — v3-volatility-forecaster 2026-05-22](../../v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md)
  — operator's (b) routing pick that enabled this disambiguation.
- [Parent feature brief — v3-volatility-forecaster v0.1.0](../../v3-volatility-forecaster/feature.md)
  — joint advisory disposition (`shipped-with-MODEL-BROKEN-NO-ALPHA-advisory`).
- [Parent sharpe-comparison (anchored byte-immutable through this pass)](../../v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md)
  — synthetic-vs-real baseline; body-SHA `ef048366ac5…`.
- [ADR-0038 § D1.b + § D1.c + § D6](../../architecture/adr/0038-vol-forecast-verdict-shape.md)
  — V-verdict shape + T-classifier thresholds + anchor-additive contract.
- [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../../dev-notes/strategy-reformulation-survey-2026-05-22.md)
  § Candidate 2 / 5 — C2 (regime-classifier) + C5 (LLM-forecaster) survey-time
  EV ranking; load-bearing for the promotion decision.
- [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
  — v2.5 DL programme retire pattern; precedent for routing (a).
- [`spec/v3-regime-classifier/feature.md`](../../v3-regime-classifier/feature.md)
  — C2 spec-only design brief; reuse seed at `crates/reflection/src/regime.rs`.
- [`spec/v3-llm-forecaster/feature.md`](../../v3-llm-forecaster/feature.md) —
  C5 spec-only design brief; reuse seed at `crates/llm` foundation.
- `spec/anchors.toml [v3.0.0-volatility-rebaseline]` — 1 new anchor row.
- `spec/trace.toml` — `REQ-V3-VOL-FORECASTER-REBASELINE-001` carried through
  `proposed → in-progress → shipped`.

## Changelog

- 2026-05-22 (presenter): release deck v0.1.0. Re-baseline pass ships clean
  code + decisive disambiguation: net_delta = 0.000000 on apples-to-apples
  real-vs-real evidence kills the parent's synthetic-vs-real data caveat
  outright. Joint advisory `V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA`
  now stands without footnote. Anchor gate 34/34 PASS with parent anchor
  `ef048366ac5…` byte-identical (architect T-AR-2 NEW-enum-variant correction
  validated). Routing R-O1 → (a) RETIRE C1 recommended; presenter advises
  against (c) DEBUG V3 and (d) v0.1.1 refit on the grounds that overlay-scale
  zero alpha is harder to refute than calibration ratio. Promotion-candidate
  pick (C2 v3-regime-classifier vs C5 v3-llm-forecaster) left as genuine
  operator strategic call — presenter does not push either way. Mechanical
  pre-tick + spec-lint gates expected at baseline 85 / 1 (no new categories).
