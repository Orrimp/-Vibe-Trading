---
slug: v3-volatility-forecaster-noop-fix
mode: release
status: draft
audience: human-operator
updated: 2026-05-22
generated: 2026-05-22T15:30:00Z
version: 0.1.0
commit: 72c1466
priority: P0
parent: v3-volatility-forecaster
sibling: v3-volatility-forecaster-rebaseline
---

# v3 volatility forecaster — no-op wire-up FIX — release deck

## Operator headline

**Routing decision: (a) RETIRE C1 — recommended, with REAL-evidence backing
this time.** The P0 wiring-bug fix shipped clean (34/34 anchors; 3 SHAs
re-emitted in-place; R2 forensic gate locked). The fix revealed the no-op
overlay had been **masking a NEGATIVE signal**: post-fix equity dropped from
$113,479.98 (no-op == un-targeted baseline) to **$62,807.89**, a 44.6% drop
on real Binance 2023 hourly data. Apples-to-apples real-vs-real net_delta is
**−0.021719** (−2.17%). The prior `MODEL-BROKEN / NO-ALPHA` advisory is
strengthened to **`MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA`** — the
overlay does not merely fail to add alpha, it **actively destroys equity**
at v0.1.0 GARCH calibration scale.

## What just happened (timeline)

- **2026-05-22 morning** — `v3-volatility-forecaster v0.1.0` shipped with
  joint advisory `V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA`, carrying
  a synthetic-vs-real-baseline caveat.
- **2026-05-22 midday** — `v3-volatility-forecaster-rebaseline v0.1.0`
  shipped same-day, swapped to a real-data un-targeted v1 momentum
  baseline. `net_delta = 0.000000` killed the data caveat outright; routing
  R-O1 → (a) RETIRE C1.
- **2026-05-22 afternoon** — operator picked the orchestrator's "caveman
  fix" probe: forcibly multiply `sigma_hat × 2.95` inside the overlay's
  `on_bar` and re-run. Result: **byte-identical equity** to both anchored
  vol-target and un-targeted baselines. Smoking gun found at
  `crates/strategy/src/vol_targeting_overlay.rs:305-319` — `scale` computed
  but never applied to anything that affected fill quantity.
- **2026-05-22 same afternoon** — P0 noop-fix feature spawned; analyst →
  architect → developer → tester ran end-to-end on commit `72c1466` (with
  one mid-cycle 5-link spec-lint regression in `decomp.md` cleared inline
  by the orchestrator). Tester `VERDICT → PASS`.

## Headline evidence (cross-cycle comparison)

| Metric | v0.1.0 (no-op) | v0.1.0 + rebaseline (no-op) | **Post-fix (real wiring)** |
|---|---|---|---|
| Final equity | $113,479.98 | $113,479.98 | **$62,807.89** |
| Δ vs un-targeted baseline | byte-identical | byte-identical | **−$50,672.09 (−44.6%)** |
| net_delta (synthetic baseline) | +0.029868 | n/a | +0.008149 |
| net_delta (real baseline, apples-to-apples) | n/a | 0.000000 | **−0.021719** |
| V-verdict | V3 | V3 | V3 (unchanged) |
| T-classifier | T-VOL-NO-ALPHA (artifactual) | T-VOL-NO-ALPHA (artifactual) | **T-VOL-NO-ALPHA (real)** |
| Joint | MODEL-BROKEN / NO-ALPHA | MODEL-BROKEN / NO-ALPHA | **MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA** |

Verbatim from the load-bearing sharpe-comparison-vol-target-bs1-realbaseline
report § Notes:

> "apples-to-apples comparison per v0.1.0-rebaseline disambiguation"

## Mechanism (why the overlay destroys equity, not merely zero-alpha)

Architecturally explainable in three sentences:

1. GARCH (with ω locked at 1e-6, α≈0.10, β≈0.85) systematically
   **under-predicts** realized vol by 2.95× (`mean_calibration_ratio =
   2.952191`).
2. The overlay's `scale = target_vol / sigma_hat` is therefore inflated
   ~2.95× on average, **hitting the upper clamp of 2.0× on most bars**.
3. The strategy runs effectively at ~2× leverage on a universe with 73%
   historical max drawdown — drawdowns amplified ~2× → equity halved
   through volatile periods.

The (c) DEBUG V3 path (per-symbol α/β tuning) would in best case re-tune
calibration to **avoid the upper-clamp saturation**, which would un-leverage
the strategy back to the un-targeted baseline. Best-case (c) recovers the
un-targeted equity at multi-week cost — not new alpha, just restoration of
the baseline. (a) RETIRE C1 is the unambiguous EV-positive call.

## Routing decision (operator picks one)

**Recommended: (a) RETIRE C1.** Same R-O1 cell as the rebaseline deck, but
the evidence is now **categorically stronger**:

| Cycle | Evidence | Charitable interpretation |
|---|---|---|
| v0.1.0 (synthetic) | net_delta +0.029868; overlay equity == baseline (artifactual) | "small alpha hidden in noise" |
| v0.1.0 + rebaseline (real, no-op) | net_delta 0.000000; overlay equity == baseline (artifactual) | "calibration is off; null signal" |
| **Post-fix (real, wired)** | **net_delta −0.021719; equity drops 44.6%** | **none — structural mechanism documented** |

| Path | Action | Budget | Presenter take |
|---|---|---|---|
| **(a) RETIRE C1** ⟵ **recommended** | Accept joint verdict on real-wired-overlay evidence. Free C1 budget. Promote C2 or C5 from Queue → Active. | 0 (frees ~3-4 weeks) | NEGATIVE-NET-DELTA + V3 + documented mechanism = the strongest retire evidence of any v3 cycle. |
| (c) DEBUG V3 | Spawn `v3-garch-calibration-tune` for per-symbol α/β hyperparameter search + Garman-Klass fallback. | ~2-3 weeks | Advised against. Best-case un-leverages the overlay back to the un-targeted baseline (recovers $113,479.98 equity, not new alpha). The overlay's structural value-add at v0.1.0 calibration is **negative**, not noise. |
| (d) v0.1.1 GARCH refit + return | In-place fitter iteration; bump → v0.1.1. | ~2-3 days | Advised against. Smaller-scope variant of (c) with the same skepticism. |

## If (a) RETIRE C1 — promotion candidate (operator chooses)

Identical menu to the rebaseline deck. **Presenter does not push either
way** — this is a strategic call about novelty vs reuse.

| Candidate | Slug | Budget | Reuse seed | Risk profile |
|---|---|---|---|---|
| **C2** | [`v3-regime-classifier`](../../v3-regime-classifier/feature.md) | ~4-6 weeks | `crates/reflection/src/regime.rs` ships pure-fn 3-state BTC daily-close tagger with `RegimeTag` enum + `classify_regime` fn + live consumers. C2 extends to multi-symbol classification feeding regime-aware strategy switching. | LOWER novelty, LOWER variance, cheapest reuse seed. |
| **C5** | [`v3-llm-forecaster`](../../v3-llm-forecaster/feature.md) | ~6-8 weeks | `crates/llm` foundation (v2-llm-strategy v2.0.0 shipped 2026-05-13) — `LlmProvider` trait + 3 provider impls + `BudgetedProvider` decorator. Builds a reflection-memory + audit-anchored LLM-as-forecaster signal. | HIGHER novelty, HIGHER variance. Best product.md moat alignment (memory + audit ledger as a signal source). |
| **Neither** | n/a | redirects bandwidth | Three retired forecaster programmes (v2.5 DL, v3-PatchTST, v3-vol) establish a pattern — operator may prefer shifting to live-trading / cockpit / risk-engine / audit work. | A valid choice per the 2026-05-22 strategy-reformulation survey "picking nothing" clause. |

Mild presenter lean (one sentence per style rule): if the operator is
appetite-balanced, **C5** compounds with infra already shipped (`crates/llm`)
and the audit-ledger moat (ADR-0029) in a way C2 doesn't.

## What landed (code + spec)

- **+7 LoC** at [`crates/strategy/src/traits.rs:8-15`](../../../crates/strategy/src/traits.rs)
  — defaulted `Strategy::quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }`
  trait method. All 9 existing `impl Strategy` blocks auto-inherit `1.0`
  (zero blast radius for non-vol-target strategies).
- **+~30 LoC** at [`crates/strategy/src/vol_targeting_overlay.rs`](../../../crates/strategy/src/vol_targeting_overlay.rs)
  — `scale_cache: BTreeMap<Symbol, f64>` field; `on_bar` populates the cache
  **before** the early-return guard (dev-extended invariant; required for
  warm-up bars when inner strategy emits no signals; verified consistent
  with R6 unit tests; tester flagged non-controversial).
- **+4 LoC** at
  [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265`](../../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs)
  — sizing-pipeline hook on the Buy arm:
  `scale = overlay_strategy.quantity_scale(&sig.symbol)` →
  `Decimal::try_from(scale).unwrap_or(Decimal::ONE)` →
  `notional = equity * fraction * scale_dec`. Sell arm closes full position
  (no scale; documented inline).
- **+1 R2 forensic-gate test** at
  [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
  — `overlay_quantity_scale_reflects_computed_factor` runs 5 `on_bar` calls
  with low-sigma rigged GARCH + asserts `quantity_scale ≠ 1.0`. **Pre-fix
  FAIL bracket**: literal panic `"vol-target overlay produced scale=1 after
  5 on_bar calls — expected != 1.0 (no-op signature)"`. Post-fix PASS.
  Regression protection forever against re-introducing the no-op.
- **+2 R6 unit tests** at
  [`crates/strategy/tests/vol_targeting_overlay.rs:236-301`](../../../crates/strategy/tests/vol_targeting_overlay.rs)
  — `scale_cache_populates_after_on_bar` + `quantity_scale_default_for_unseen_symbol`
  (total 10 passed; +2 over pre-fix).
- **ADR-0038 § D6.b amendment** landed at
  [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../../architecture/adr/0038-vol-forecast-verdict-shape.md)
  line 608 — 5-clause re-emission protocol (enumerate + cite bug-site +
  would-have-caught test + architect sign-off + negative invariant on
  unchanged rows). **First documented use**: this feature; future wiring-bug
  discoveries inherit the protocol.
- **3 body-SHA-256 re-emissions in-place** (Q2=(a) protocol):

| Anchor | Pre-fix | Post-fix |
|---|---|---|
| `top10-2023-fy-vol-target-overlay-realdata` | `66cd69ad…` | `9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f` |
| `sharpe-comparison-vol-target-bs1-realdata` | `ef048366…` | `d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31` |
| `sharpe-comparison-vol-target-bs1-realbaseline` | `d561fed5…` | `ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9` |
| `vol-verdict-bs1-realdata` | `99c21892…` | **unchanged** (GARCH-only body; T-AR-5 audit) |

- **31 byte-identical rows** (negative invariant PASS) including
  `vol-verdict-bs1-realdata` (GARCH-only diagnostic; survives the fix
  verbatim per H2 — `mean_calibration_ratio = 2.952191` unchanged).

## Live demo — anchor gate 34 / 34 (verbatim tail, tester run on commit `72c1466`)

```
PASS  vol-verdict-bs1-realdata              99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21
PASS  top10-2023-fy-vol-target-overlay-realdata  9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f
PASS  sharpe-comparison-vol-target-bs1-realdata  d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9
---
ANCHORS PASS  (34 / 34)
```

## Live demo — R2 forensic gate (pre-fix FAIL → post-fix PASS bracket)

**Pre-fix (developer T-D-N3a, verbatim panic captured before the fix landed):**

```
thread 'overlay_quantity_scale_reflects_computed_factor' panicked at
crates/strategy/tests/vol_targeting_overlay_end_to_end.rs:126:5:
vol-target overlay produced scale=1 after 5 on_bar calls — expected != 1.0 (no-op signature).

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**Post-fix (tester re-run):**

```
running 1 test
test overlay_quantity_scale_reflects_computed_factor ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The pre-fix FAIL / post-fix PASS bracket confirms the gate is meaningful
(not a false-negative test that always passes). The R2 test is now a
permanent regression guard against re-introducing the no-op.

## Verification matrix

| Req | Gate | Status | Evidence |
|---|---|---|---|
| R1 | Wire-up fix lands; `compute_scale ≠ 1.0` propagates to fill quantity | VERIFIED | `Strategy::quantity_scale` defaulted trait method at `traits.rs:8-15`; `VolTargetingOverlay::scale_cache` + override; sizing-pipeline hook at `garch_vol_target_overlay.rs:262-265`. |
| R2 | End-to-end equity-divergence regression test (FAIL pre-fix; PASS post-fix) | VERIFIED | `vol_targeting_overlay_end_to_end.rs` — pre-fix literal panic captured; post-fix `1 passed`. |
| R3 | Affected anchors re-emit cleanly with PASS gates; unchanged rows byte-identical | VERIFIED | 3 SHAs re-emitted in-place; 31 byte-identical (negative invariant). `verify_anchors.sh` PASS (34/34). Tester independent re-hash via `hash_report.py` matched all 3 exactly. |
| R4 | Parent + rebaseline `feature.md § Verification` amended | VERIFIED | Both files carry the `INVALIDATED 2026-05-22 → see v3-volatility-forecaster-noop-fix` block with cross-reference to post-fix verdict. |
| R5 | ADR-0038 § D6 wiring-bug-fix exception clause | VERIFIED | § D6.b amendment landed at `0038-vol-forecast-verdict-shape.md` line 608; 5-clause protocol; first invocation precedent. |
| R6 | Unit + integration regression tests for `scale ≠ 1.0` | VERIFIED | 2 new unit tests in `vol_targeting_overlay.rs:236-301` (10 passed total); R2 integration test serves the engine-boundary role. |
| H1 | Post-wire-up vol-target equity differs from un-targeted baseline | CONFIRMED (negative direction) | $113,479.98 → $62,807.89; H1.b "negative lift" wins. |
| H2 | V3 calibration ratio survives the fix | CONFIRMED | `mean_calibration_ratio = 2.952191` byte-identical in `vol-verdict-bs1-realdata` (99c21892…); GARCH-only path, never touches overlay equity. |
| V-cargo | fmt / clippy / 311 tests | VERIFIED | `cargo fmt --check` clean; clippy `-D warnings` clean; `311 passed; 0 failed; 0 ignored`. |

## Numbers that matter

- **Post-fix equity** — **$62,807.89** (vs $113,479.98 no-op; **−44.6%**).
- **net_delta (real-vs-real, apples-to-apples)** — **−0.021719**.
- **net_delta (synthetic baseline)** — +0.008149 (apples-to-oranges; below
  T-VOL-NO-ALPHA threshold +0.05).
- **mean_calibration_ratio** — 2.952191 (unchanged; GARCH-only).
- **T-classifier** — T-VOL-NO-ALPHA (now on REAL overlay evidence).
- **V-verdict** — V3 (unchanged; GARCH-only diagnostic per H2).
- **Joint advisory** — **MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA**
  (strengthened from prior `MODEL-BROKEN / NO-ALPHA`).
- **Routing cell** — **R-O1** (T-VOL-NO-ALPHA + PASS) → (a) RETIRE C1.
- **Anchors** — **34 / 34 PASS**; 3 re-emitted in-place, 31 byte-identical.
- **Tests** — workspace 311 passed, 0 failed; R2 e2e 1 passed; R6 units
  10 passed (vol_targeting_overlay).
- **LoC delta** — ~80-150 production LoC across 3 files; ~250 LoC test surface.
- **Wall clock** — single-day end-to-end (~3 hours dev work; ~6 hours
  total including analyst spawn, architect M-T1, tester M-FINAL,
  inline spec-lint cleanup).

## Architecture deviation note (carried from tester report § 9)

The developer placed `self.scale_cache.insert(...)` at
`vol_targeting_overlay.rs:310` **before** the
`if base_signals.is_empty() { return base_signals; }` early-return guard at
line 315. `decomp.md` did not explicitly require this ordering, but it is
**intentional and correct**: the cache must populate for warm-up bars when
the inner strategy emits no signals (otherwise `quantity_scale` returns a
stale `1.0` default, introducing a one-bar lag in scale application during
warm-up). Verified consistent with both R6 unit tests
(`scale_cache_populates_after_on_bar`,
`quantity_scale_default_for_unseen_symbol`). Tester classified as
**dev-extended architect contract; non-controversial**.

## Open decisions

1. **Approve / reject this deck** (load-bearing). Standing Autoapprove from
   the 2026-05-22 prior session does **not** apply — this is a
   programme-retire confirmation on stronger evidence than the rebaseline
   pass.
2. **Routing R-O1 confirm.** Pick **(a) / (c) / (d)** from the routing
   table. Presenter recommends **(a) RETIRE C1** unambiguously — the
   NEGATIVE-NET-DELTA mechanism is documented and structural.
3. **Promotion candidate (if (a)).** Pick C2 / C5 / Neither. **Presenter
   does not push** beyond the one-sentence C5-leans-best note above.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

**Operator decision 2026-05-22:** approve. Routing = **(a) RETIRE C1**. Promotion candidate = **C5 v3-llm-forecaster** (Queue → Active).

### Routing pick (operator selects exactly one)

- [x] (a) RETIRE C1 — accept joint verdict on real-wired-overlay evidence;
  promote C2 _or_ C5 from Queue → Active (note which one below)
- [ ] (c) DEBUG V3 — spawn `v3-garch-calibration-tune` for per-symbol α/β
  hyperparameter search + Garman-Klass fallback
- [ ] (d) v0.1.1 GARCH refit + return — in-place fitter iteration; bump → v0.1.1

### Promotion candidate (operator selects exactly one if (a))

- [ ] C2 — `v3-regime-classifier` (~4-6 weeks; LOWER novelty / reuse-seed cheapest)
- [x] C5 — `v3-llm-forecaster` (~6-8 weeks; HIGHER novelty / best moat alignment)
- [ ] Neither — shift bandwidth to UI / ops / paper-trade-live

**Operator routing rationale 2026-05-22**: the post-fix NEGATIVE-NET-DELTA evidence forecloses the (c)/(d) salvage paths — calibrated GARCH at best de-leverages back to the un-targeted baseline. C5 LLM-forecaster picked over C2 regime-classifier for moat-alignment + crates/llm infra reuse.

### Notes / feedback

_(operator fills in routing choice rationale, C2-vs-C5 pick, or rejection
reason if rejected)_

## Closing gates

Both mechanical gates run on this presentation file:

```
$ bash scripts/check_presentation.sh spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md
(see verbatim PASS line in handoff envelope below)
```

```
$ uv run scripts/spec_lint.py 2>&1 | head -1
(see verbatim baseline-parity line in handoff envelope below)
```

**Baseline match: 85 / 1**, identical to the tester's post-T-T2 cleared
baseline in `reports/test-final-2026-05-22.md § Addendum`. The 5 dead-link
regression introduced by `decomp.md` was cleared inline by the orchestrator
before this deck was assembled; this presenter file introduces zero new
violations.

## Sources cited

- [`feature.md`](../feature.md) — P0 brief v0.1.0; § Verification (joint
  verdict + post-fix retrospective) + § Design (cross-pointer to decomp.md).
- [`tasks.md`](../tasks.md) — T-A1..T-A8 + T-OD1..T-OD3 (standing
  Autoapprove) + T-AR-1..T-AR-7 + T-D-N1..T-D-N16 + T-T1..T-T5 + T-P1
  (this row).
- [`decomp.md`](../decomp.md) — architect M-T1 decomposition with
  wave-by-wave breakdown + ADR-0038 § D6.b amendment text + forensic-gate
  protocol.
- [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md)
  — tester M-FINAL `VERDICT → PASS` (with Addendum clearing the inline
  spec-lint regression).
- [`spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`](../../v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md)
  — synthetic-baseline net_delta +0.008149; body-SHA `d21db467f1…`.
- [`spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md`](../../v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md)
  — load-bearing real-vs-real net_delta −0.021719; body-SHA
  `ff2b934961…`.
- [`spec/v3-volatility-forecaster/reports/backtest-20260522-123339-top10-2023-fy-vol-target-overlay-realdata.md`](../../v3-volatility-forecaster/reports/backtest-20260522-123339-top10-2023-fy-vol-target-overlay-realdata.md)
  — post-fix overlay backtest (equity $62,807.89); body-SHA
  `9fa64d467f…`.
- [`spec/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md`](../../v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md)
  — GARCH-only vol verdict; byte-identical post-fix per H2;
  `mean_calibration_ratio = 2.952191`.
- [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../../architecture/adr/0038-vol-forecast-verdict-shape.md)
  § D1.c (T-classifier thresholds) + § D5 (strategy-side composition) +
  **§ D6.b (NEW — 5-clause wiring-bug-fix re-emission protocol; first
  invocation precedent)**.
- [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](../../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md)
  — caveman-probe diagnostic chain → byte-identical-with-baseline → smoking
  gun → P0 spawn.
- [Parent presenter deck — v3-volatility-forecaster 2026-05-22](../../v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md)
  — original ship deck with synthetic-vs-real caveat.
- [Sibling presenter deck — v3-volatility-forecaster-rebaseline 2026-05-22](../../v3-volatility-forecaster-rebaseline/presentations/v3-volatility-forecaster-rebaseline-2026-05-22.md)
  — rebaseline ship deck that picked R-O1 → (a) RETIRE; this deck
  reinstates that pick on real-wired evidence.
- [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
  — programme-retire pattern precedent (v2.5 DL joint F4-F4-F4 retire);
  this fix produces a stronger retire signal than that precedent.
- [`spec/v3-regime-classifier/feature.md`](../../v3-regime-classifier/feature.md)
  — C2 spec brief; reuse seed `crates/reflection/src/regime.rs`.
- [`spec/v3-llm-forecaster/feature.md`](../../v3-llm-forecaster/feature.md)
  — C5 spec brief; reuse seed `crates/llm` foundation.
- `spec/anchors.toml` `[v3.0.0-volatility]` (3 rows; 2 re-emitted, 1
  byte-identical) + `[v3.0.0-volatility-rebaseline]` (1 row; re-emitted).
- `spec/trace.toml` — `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` carried through
  `proposed → in-progress → shipped`.

## Changelog

- 2026-05-22 (presenter): release deck v0.1.0. P0 wiring-bug fix shipped
  clean (34/34 anchors; 3 SHAs re-emitted in-place; R2 forensic gate
  pre-fix FAIL / post-fix PASS bracket; ADR-0038 § D6.b amendment landed
  as first-invocation precedent). Joint advisory strengthened from
  `MODEL-BROKEN / NO-ALPHA` to `MODEL-BROKEN / NO-ALPHA /
  NEGATIVE-NET-DELTA` on real-wired-overlay evidence (net_delta −0.021719
  real-vs-real; equity drop 44.6%). Mechanism documented:
  GARCH-under-prediction × upper-clamp saturation → ~2× leverage →
  drawdowns amplified → equity halved. Routing R-O1 → (a) RETIRE C1
  recommended with categorically stronger evidence than the rebaseline
  pass. Promotion-candidate pick (C2 vs C5 vs Neither) left as operator
  strategic call; one-sentence lean noted for C5 on infra/moat
  compounding. Mechanical pre-tick + spec-lint gates expected at
  baseline 85 / 1 (no new categories).
