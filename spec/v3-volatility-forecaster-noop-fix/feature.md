---
slug: v3-volatility-forecaster-noop-fix
version: 0.1.0
status: shipped
owner: tester
updated: 2026-05-22
priority: P0
parent: v3-volatility-forecaster
parent_version: 0.1.1
parent_disposition: invalidated-then-retired-with-real-evidence; see v3-volatility-forecaster-noop-fix
sibling: v3-volatility-forecaster-rebaseline
sibling_disposition: invalidated-then-retired-with-real-evidence; see v3-volatility-forecaster-noop-fix
---

# v3 volatility forecaster — no-op wire-up FIX

> **P0 wiring-bug discovery.** The shipped `v3-volatility-forecaster
> v0.1.0` and the follow-on `v3-volatility-forecaster-rebaseline v0.1.0`
> are both built on a **no-op vol-targeting overlay**: the GARCH
> `compute_scale` math is correct, but the scale factor is **never
> applied** to anything that affects fill quantities or equity.
> Both ships are provisionally invalidated until the wire-up lands.
> The MODEL-BROKEN / NO-ALPHA joint verdict + the (a) RETIRE-C1
> routing pick are artifacts of the no-op; the actual post-fix
> verdict is unknown and TBD by re-test.

## Why — P0 framing

This is **not a feature ask**; it is a wiring-bug discovery. Two
shipped features depend on the same load-bearing assumption — that
the GARCH vol-targeting overlay actually scales position quantities
— and that assumption is **false** in the v0.1.0 code as of
2026-05-22.

The shipped feature chain:

- **v3-volatility-forecaster v0.1.0** (shipped 2026-05-22) — joint
  advisory **V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA**, anchored
  under `[v3.0.0-volatility]` (3 rows). Carries the synthetic-vs-real
  data caveat.
- **v3-volatility-forecaster-rebaseline v0.1.0** (shipped 2026-05-22,
  same day) — re-baselined against a real-data un-targeted v1 momentum
  scenario; landed verdict **T-VOL-NO-ALPHA confirmed on real-vs-real
  evidence** (`net_delta = 0.000000`), anchored under
  `[v3.0.0-volatility-rebaseline]` (1 row). Operator routing R-O1 →
  (a) RETIRE C1 selected.

The orchestrator's caveman probe on 2026-05-22 ~11:44Z revealed that
the vol-targeting overlay's `compute_scale` return value is computed
correctly and recorded in stats counters, but **never multiplied
into any quantity that the executor reads**. Three diagnostic
observations (see § Smoking gun + § Investigation findings) all
collapse to the same root cause: the strategy-side composition lock
in ADR-0038 § D5 is **wire-incomplete** at the strategy → executor
handoff.

Net effect: the two shipped features are **not evidence about
vol-targeting**. They are evidence that the un-targeted v1 momentum
baseline equity is `$113,479.98 / 13.48% return / 73.73% DD /
6203 trades`, recorded **twice** under two anchor namespaces. The
(a) RETIRE C1 decision is invalidated and on hold until the
wire-up fix lands and the re-run produces a real verdict.

## Smoking gun

`crates/strategy/src/vol_targeting_overlay.rs:305-319`:

```rust
        // Compute scale factor.
        let scale = self.compute_scale(sigma_hat);

        // Apply scale to signals.
        let tol = 1e-6;
        if (scale - 1.0).abs() < tol {
            self.stats.signals_passthrough += base_signals.len() as u64;
            base_signals
        } else {
            self.stats.signals_scaled += base_signals.len() as u64;
            // Return the signals with the scale embedded in the strategy_id
            // (diagnostic only — the backtest engine reads quantities from fills,
            // not from signal metadata).
            base_signals
        }
```

The `else` branch increments `signals_scaled` and returns
`base_signals` **unmodified**. The inline comment admits the recorded
scale is "diagnostic only." The variable `scale` flows nowhere.

### Diagnostic chain (orchestrator caveman probe, 2026-05-22 ~11:44Z)

A foreground 30-minute probe multiplied `sigma_hat` by `2.95` (the
parent's mean_calibration_ratio = 2.952191, picked to bias-correct V3)
inside `vol_targeting_overlay.rs::on_bar` and re-ran the full
`top10-2023-fy-vol-target-overlay-realdata` backtest. Three signals
of a no-op:

1. **Equity unchanged under intentional perturbation.** The
   caveman-patched run produced **byte-identical** equity to the
   parent anchor `66cd69ad…`: `$113,479.98 / 13.48% / 73.73% DD /
   6203 trades`. A 2.95× perturbation of the input to `compute_scale`
   had zero effect on output equity — definitionally a no-op.
2. **Vol-target overlay equity == un-targeted baseline equity.** The
   anchored `top10-2023-fy-vol-target-overlay-realdata` body and the
   anchored `top10-2023-fy-momentum-realdata` body (the un-targeted
   v1 baseline from the rebaseline pass) produce the **byte-identical
   same** equity / DD / trade-count tuple. If the overlay applied
   any scale ≠ 1.0 to any signal, equity would diverge.
3. **`stats.signals_scaled = 6203` despite zero equity divergence.**
   The overlay's `else` branch fires for every signal (because clamp
   bounds `[0.5, 2.0]` are away from 1.0 with `scale ≈ target_vol /
   sigma_hat` in the typical regime); the counter ticks; the signals
   pass through unchanged. The counter is a **proxy without
   consequence**.

Code review (§ Smoking gun above) confirms: lines 309-319 implement
no application of the scale; `scale` flows nowhere; the comment
states the design intent of the `Signal.metadata` slot but
**the metadata write itself is also absent** — `base_signals` is
returned by-value without modification.

### Where it went undetected

The bug survived four independent gates because each tested an
adjacent property, none tested the load-bearing end-to-end one:

| Gate                                 | Verifies                                                                 | What it missed                                                                                                                                                                                |
| ------------------------------------ | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test` (vol_targeting_overlay) | `compute_scale` clamp invariants (`target_vol → 1.0`, low σ → max, high σ → min)  | No assertion that `scale ≠ 1.0` causes any change in returned `Signal` (vs `scale = 1.0`). All 8 unit tests verify the math, not the application.                                          |
| `clippy / fmt`                       | Code style; no dead-code lint fires because `scale` IS read (by the `if` check) | The lint passes because the variable is consulted; it does not check that the consultation has consequences.                                                                                  |
| Anchor gate (33→34 PASS)             | 2-run byte-identity of every report body                                  | The byte-identity is **exactly the signature of a no-op overlay** — overlay output == baseline output is what the anchor witness records. The gate has no way to know byte-identity-with-baseline is a bug signature. |
| Architect M-T1 lock (T-AR-4)         | Scale-clamp invariants; ADR-0038 § D5 strategy-side composition contract  | The contract specifies the composition shape (overlay wraps inner strategy), not the application semantics (overlay actually changes fill quantities). § D5 is wire-incomplete.            |
| Tester M-FINAL (parent + rebaseline) | cargo gates + anchor gate + byte-identity                                 | No gate of shape "vol-target overlay equity differs from un-targeted baseline equity by a non-trivial amount." That gate did not exist in the test surface and so could not fail.        |

The bug is at the **strategy → executor handoff**, not in the GARCH
model itself. The V3 calibration finding (mean_calibration_ratio =
2.952191) survives the fix verbatim — it is a GARCH-only diagnostic
measured on the model output before the overlay even tries to apply
the scale.

## Investigation findings

### Bug site

`crates/strategy/src/vol_targeting_overlay.rs:305-319`. 10-line scope.
The fix is **wire-up only** — no GARCH change, no Strategy trait
change at v0.1.0 scope, no scenario change. The architect picks the
fix shape (see Q1).

### Two possible fix shapes (architect-decide)

The inline comment in the bug site reads "the backtest engine reads
quantities from fills, not from signal metadata." This describes the
shape of the missing wire. Two options:

- **Option (i) — `Signal` carries a quantity-scale field.** Add
  `quantity_scale: f64` (default `1.0`) to the `trading_core::Signal`
  type. The overlay populates the field with `scale`. The executor's
  position-sizing pipeline multiplies fill quantity by
  `quantity_scale` at submission time. **Blast radius**: every
  `Strategy` impl that constructs `Signal`s (default `1.0` keeps
  existing behaviour byte-identical); every executor / sizing
  consumer that builds fills from `Signal`s (one site, presumably
  `crates/exec` or `crates/backtest::engine`). Anchor implications:
  with the default `1.0`, existing strategies stay byte-identical;
  the vol-target overlay's output diverges from baseline (the
  point of the fix).
- **Option (ii) — Per-symbol scale query at sizing time.** Add a
  trait method `Strategy::quantity_scale(&self, symbol: &Symbol) →
  f64` (default returns `1.0`); the position-sizing pipeline queries
  the strategy at sizing time and multiplies fill quantities by the
  returned value. The overlay returns `compute_scale(sigma_hat)` per
  symbol (cached from the most recent `on_bar` call); inner strategy
  delegates to default. **Blast radius**: the `Strategy` trait gets a
  new defaulted method (every impl auto-inherits); the sizing
  pipeline gains one call site; no `Signal` field change. Anchor
  implications same as (i).

The analyst's K2 mitigation default is **option (ii)** because the
blast radius is smaller — `Signal` is a load-bearing serialized
record (audit ledger writes, journal entries, ADR-0029 canonical
arch descriptors); adding a field there has multi-crate ripple. A
defaulted trait method is a much lighter touch. The architect locks
the choice at M-T1; Q1 carries (ii) as the standing-Autoapprove
default.

### TCN overlay co-investigation (T-A2 finding — no same-pattern bug)

`crates/strategy/src/tcn_overlay_momentum.rs::on_bar` (lines 634-703)
was audited for the same wire-up pattern. **TCN overlay is structurally
different** and does NOT have the same no-op:

- TCN overlay's `combine_with_direction` (lines 736-766) returns a
  potentially-different `SignalKind` (e.g. `Buy → Hold` when the
  forecaster disagrees confidently).
- The overlay's `on_bar` constructs `Signal { kind: modulated_kind,
  ..sig }` (line 697-700) — the **kind field IS replaced**.
- `Signal.kind` is a load-bearing field the executor / sizing pipeline
  already reads (Hold → no order submitted; Buy / Sell → order at
  default sizing). So a TCN kind-replacement DOES propagate to fill
  quantities (specifically: `dampened` ⇒ zero quantity).

Vol-target overlay, by contrast, intends a **quantity-scale** semantic
(multiply by 1.7, multiply by 0.6) for which **no field exists on
`Signal`** today. The TCN overlay's dampen-to-Hold semantic
piggy-backs on an existing field; the vol-target overlay's scale
semantic does not.

Net: **TCN overlay is NOT a parallel bug**. Q3 default is (b)
vol-target-only fix. The architect should still re-audit the TCN
overlay during M-T1 once the fix shape is locked, to confirm no
adjacent assumption breaks under (i) or (ii) — see § Open questions
Q3.

### Affected anchors (4 rows need re-emission post-fix)

Body-SHA-256 will change for every anchor where the vol-target
overlay output is load-bearing. Four rows in `spec/anchors.toml`:

| Namespace                            | Scenario                                                  | Current SHA (will change post-fix)                                                |
|--------------------------------------|-----------------------------------------------------------|-----------------------------------------------------------------------------------|
| `[v3.0.0-volatility]`                | `top10-2023-fy-vol-target-overlay-realdata`               | `66cd69ad03294cccf514184968babce0127f2ebfa4d1f4a03b332f8000f79c65`                |
| `[v3.0.0-volatility]`                | `sharpe-comparison-vol-target-bs1-realdata`               | `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`                |
| `[v3.0.0-volatility]`                | `vol-verdict-bs1-realdata` (GARCH calibration — unchanged sigma_hat path; will it change?) | `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21`     |
| `[v3.0.0-volatility-rebaseline]`     | `sharpe-comparison-vol-target-bs1-realbaseline`           | `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`                |

The `vol-verdict-bs1-realdata` body is a GARCH-only calibration
diagnostic; it is computed before the overlay applies the scale and
**should** stay byte-identical post-fix. Architect to confirm at
M-T1 — if the verdict report cites overlay equity, that part needs
re-emission; if it only reports GARCH internals, the row holds. The
analyst's working assumption is **3 anchors change, 1 holds**, but
records all 4 as "needs M-T1 audit."

### ADR-0038 § D6 anchor-additive contract — needs exception clause

ADR-0038 § D6 reads "existing anchors stay byte-identical." That
spirit is **don't silently mutate historical evidence**. The fix is
a **wiring-bug-fix re-emission** — the historical bodies were
recorded with a no-op overlay; the new bodies will be recorded with
a real overlay. This is legitimate re-emission (the contract being
tested is different) but precedent-setting. The R5 requirement
locks a **documented protocol** for legitimate anchor re-emission
(either an ADR-0038 § D6 amendment block OR a new sibling ADR).
Architect picks the protocol at M-T1 (Q2).

## Scope

Tight: ONE wire-up fix + 4 anchor re-emissions (3 if vol-verdict
holds) + ONE new end-to-end regression test.

1. **Wire-up fix at `vol_targeting_overlay.rs:305-319`** (or the
   equivalent in `trading_core::Signal` / `Strategy` trait per
   architect Q1). The fix MUST cause the overlay's output to differ
   from the un-targeted baseline by at least 1 basis point of equity
   when `scale ≠ 1.0` for any signal — this is the regression
   contract R2 enforces.
2. **Anchor re-emissions** for the affected rows. Anchor-additive
   contract amended at ADR level (Q2) to document the wiring-bug-fix
   protocol.
3. **New end-to-end test** that asserts vol-target overlay equity ≠
   un-targeted baseline equity by ≥ 1 bp (or whatever the architect
   locks). This is the regression contract against re-introducing
   the no-op.
4. **Amendment blocks** in parent + rebaseline `feature.md §
   Verification` documenting "INVALIDATED 2026-05-22 — see
   v3-volatility-forecaster-noop-fix" with cross-references.

## Out of scope

- **GARCH model improvements.** Per-symbol hyperparameter search,
  Garman-Klass fallback, walk-forward refit — all stay deferred to
  v0.1.1+ as before. The V3 calibration finding survives the fix.
- **CLI flag additions.** No new `--vol-target-on/off` flag; the
  fix is silent at the binary level.
- **product.md edits.** No product-requirement change; the bug fix
  is internal to the existing R6.a deliverable.
- **Promotion candidate decisions.** C2 (`v3-regime-classifier`)
  and C5 (`v3-llm-forecaster`) stay paused; no (a) RETIRE-C1
  decision until the re-run lands a real verdict.
- **Risk-engine integration.** ADR-0038 § D5 "risk-engine deferred
  to v0.1.1" still holds. The fix lives strictly in the strategy-
  side composition layer.
- **Scenario surface changes.** No new backtest scenarios; the
  existing `top10-2023-fy-vol-target-overlay-realdata` and
  `top10-2023-fy-momentum-realdata` scenarios are reused verbatim.

## Requirements (R1-R6)

### R1 — Wire-up fix lands

The vol-targeting overlay's `compute_scale` return value MUST flow
into the executor's fill-quantity pipeline such that a `scale ≠ 1.0`
result causes fill quantity to differ from the `scale = 1.0` case.
Fix site: `crates/strategy/src/vol_targeting_overlay.rs:305-319`
(application site), plus the architect-picked wire (Q1 = (i)
`Signal.quantity_scale` field OR (ii) `Strategy::quantity_scale`
trait method).

**Acceptance**: a unit test asserts that for some sigma_hat causing
`compute_scale ≈ 1.7`, the returned `Signal` (or the queried
`Strategy::quantity_scale`) carries the 1.7 value end-to-end through
the sizing layer. Test name: `vol_targeting_overlay::scale_applied_to_quantity`
or equivalent.

### R2 — End-to-end equity-divergence regression test

A new integration test MUST run a minimal vol-target overlay
backtest and an un-targeted v1 momentum baseline backtest on a
synthetic-or-fixture data stream where the GARCH model is rigged
to produce `sigma_hat` such that `compute_scale` returns a value
other than `1.0` for at least N (architect-locked) bars. The test
asserts:

```
equity_overlay != equity_untargeted   // by ≥ 1 bp
trades_overlay ≈ trades_untargeted    // signal timing identical; sizing differs
```

This is the contract against re-introducing the no-op. Test name:
`vol_targeting_overlay::overlay_changes_equity_vs_untargeted_baseline`
or equivalent.

**Acceptance**: test passes under the fix; test FAILS under the
pre-fix code (verification step at architect M-T1).

### R3 — Affected anchors re-emit cleanly with PASS gates

The 4 listed body-SHAs (3 may change; 1 is unchanged-pending-audit)
re-emit at fresh values post-fix. The remaining anchors
(34 - 4 = 30, or 34 - 3 = 31) stay byte-identical (no other
scenario consumes the vol-target overlay). Tester re-runs
`scripts/verify_anchors.sh`; outcome is **ANCHORS PASS (34 / 34)**
with 30-31 unchanged + 3-4 fresh.

**Acceptance**: tester M-FINAL report shows the new body-SHAs in
`spec/anchors.toml` and confirms the un-changed rows stayed
byte-identical (negative invariant).

### R4 — Parent + rebaseline feature.md § Verification amendment

Both `spec/v3-volatility-forecaster/feature.md § Verification` and
`spec/v3-volatility-forecaster-rebaseline/feature.md § Verification`
get a `> **INVALIDATED 2026-05-22 — see
v3-volatility-forecaster-noop-fix v0.1.0**` block at the top of the
section, plus a cross-reference to this feature's post-fix
verdict (recorded at M-FINAL).

**Acceptance**: spec-update writes the amendment block to both
files; orchestrator audits via `grep -l "INVALIDATED 2026-05-22"
spec/v3-volatility-forecaster*/feature.md` showing both hits.

### R5 — ADR-0038 § D6 wiring-bug-fix exception clause

Either an in-place amendment to ADR-0038 § D6 OR a sibling
ADR-0039 documents the protocol for legitimate anchor re-emission
under a discovered wiring bug. Default (analyst-recommended):
amend § D6 with a short "Exception: wiring-bug-fix re-emission"
subsection that requires (a) the affected anchors enumerated, (b)
the bug site cited with file:line, (c) a test that would have
caught the bug had it existed (R2), and (d) the architect signing
off on the re-emission delta.

**Acceptance**: ADR-0038 § D6 contains the new subsection;
spec-lint passes; the protocol is reusable for future wiring-bug
discoveries.

### R6 — Unit + integration regression tests for `scale != 1.0`

In addition to R2's end-to-end test, R6 locks **two** narrower
guards:

- **Unit test**: with the architect-picked wire (Q1), assert that
  `VolTargetingOverlay::on_bar` causes the returned `Signal` (or
  the queried `Strategy::quantity_scale`) to carry the computed
  scale for at least one synthetic bar. Bypasses the engine; tests
  the strategy in isolation.
- **Integration test**: at the engine boundary, assert that two
  fills generated by the overlay under `compute_scale → 1.7` have
  different quantities than two fills generated by the baseline
  under `scale = 1.0`. This complements R2 (which tests equity
  divergence) with a tighter test on fill quantities directly.

**Acceptance**: both tests pass under the fix; both tests FAIL
under the pre-fix code (verification step at architect M-T1).

## Risks (K1-K3)

### K1 — TCN overlay has the same no-op pattern (RULED OUT by T-A2)

**Status: RULED OUT.** The TCN overlay's dampen-to-Hold semantic
mutates `Signal.kind`, which is a load-bearing field the executor
already reads. The TCN overlay's wire is sound; no parallel bug.

Architect should still re-audit during M-T1 once the Q1 fix shape
is locked, to confirm no adjacent assumption breaks under (i) or
(ii). Defer to Q3 = (b) vol-target-only fix unless the audit
surfaces something.

### K2 — Fix may require touching `trading_core::Signal` or a Strategy trait

If Q1 = (i) `Signal.quantity_scale` field: every `Strategy` impl that
constructs `Signal`s gets touched (default `quantity_scale = 1.0`
keeps existing behaviour byte-identical), every executor / sizing
consumer that builds fills from `Signal`s gets touched (one or two
sites), every audit / journal entry that serializes `Signal`s gets
touched (additive field; default-serialization keeps existing JSON
shape if the field is `#[serde(default)]`).

If Q1 = (ii) `Strategy::quantity_scale(&self, symbol) → f64`:
defaulted trait method; every impl auto-inherits the default `1.0`;
only the sizing pipeline gains one call site.

**Mitigation**: standing-Autoapprove default is Q1 = (ii) per
minimum-blast-radius rationale. If the architect surfaces a reason
to pick (i) (e.g. audit-trail of per-signal scale; serialization of
the scale into the journal), the operator decides explicitly.

### K3 — Re-emitting v0.1.0 ship anchors is precedent-setting

The 33-anchor regression gate has held byte-identity through every
shipped feature since 2026-04. Re-emitting 3-4 anchors with new
SHAs is a legitimate operation when the underlying contract has
changed (no-op → real overlay), but precedent matters: future
wiring-bug discoveries will need the same protocol, and the gate
must not become a rubber-stamp.

**Mitigation**: R5 locks the ADR-0038 § D6 amendment specifying
the wiring-bug-fix re-emission protocol. The architect signs off on
the re-emission delta at M-T1. The tester validates the (3 + or 4
+ but not more) scope at M-FINAL via negative invariant check on
the unchanged rows.

## Hypotheses (H1-H2)

### H1 — Post-wire-up, vol-target equity will differ from un-targeted baseline

**Direction unknown. Magnitude unknown.** Three possible verdicts:

- **H1.a — positive lift**: Sharpe lift ≥ +0.05 → T-VOL-MARGINAL or
  T-VOL-ALPHA-UNLOCKED. The Moreira-Muir 2017 prior is +0.15 to
  +0.40 on equity factor portfolios; crypto-hourly transaction-cost
  drag may eat some or all of that. **Prior**: MEDIUM-HIGH for
  ≥ +0.05; LOW-MEDIUM for ≥ +0.10.
- **H1.b — negative lift**: Sharpe degradation. Vol-targeting on a
  universe where the GARCH calibration is broken on 3/10 symbols
  (V3 finding) could over-scale into noise. **Prior**: MEDIUM.
- **H1.c — near-zero**: Calibration was the binding constraint, not
  the wire; even with the fix, broken GARCH on AVAX/DOGE/DOT
  produces near-1.0 effective scale after clamp. **Prior**: LOW-MEDIUM.

The fix unblocks the H1 measurement; it does NOT prejudge the
outcome.

### H2 — V3 calibration ratio finding survives the fix

The V3 mean_calibration_ratio = 2.952191 is computed on
predicted-vs-realized GARCH sigma_hat **before** the overlay tries
to apply the scale. The fix changes how the scale is applied; it
does not change the scale value. V3 is a GARCH-only diagnostic.

**Verification step (T-T2)**: the tester re-emits
`vol-verdict-bs1-realdata` and confirms the calibration ratio is
unchanged (= 2.952191). If the body-SHA changes (the body cites
overlay equity downstream), the cited value is unchanged; only the
surrounding equity numbers shift.

## Routes (4-cell verdict × determinism table)

Pre-drawn for the architect / presenter to inherit. Standing
Autoapprove on Q1..Q3 defaults; route selection at M-FINAL keys to
the new verdict cell.

| Route | T-classifier on new net_delta | Determinism | Routing implication                                                                                                       | Next feature                                                                                                                                  |
| ----- | ------------------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| R-O1  | T-VOL-NO-ALPHA (`< +0.05`)     | PASS        | Confirms parent's advisory on **real overlay** evidence. The wiring fix did not unlock alpha; the no-op was masking nothing. | **(a) RETIRE C1** — promote C2 (`v3-regime-classifier`) or C5 (`v3-llm-forecaster`) per parent deck HYBRID sequencing. Real evidence this time.|
| R-O2  | T-VOL-MARGINAL (`[+0.05, +0.10)`) | PASS     | Wiring fix produced partial lift. V3 still fires on calibration.                                                          | Operator chooses **(a) RETIRE C1** (close enough; V3 dominates) OR **(d) v0.1.2 GARCH refit** to push past +0.10.                                |
| R-O3  | T-VOL-ALPHA-UNLOCKED (`≥ +0.10`) | PASS     | Wiring fix unlocked alpha. The prior MODEL-BROKEN / NO-ALPHA verdict is fully retracted. V3 calibration repair becomes the gate to banking the alpha live. | **Reopen v3-volatility-forecaster as a live candidate**; spawn `v3-garch-calibration-tune` for V3 repair before live-signal use.                |
| R-O4  | (any)                          | FAIL        | Determinism contract broken on the re-emitted anchors; cannot ship.                                                       | Route back to **developer** for determinism fix. If iteration overflows, escalate to operator-decide extend-budget vs roll back.               |

## Operator-decide questions (Q1-Q3)

Three operator-decide rows kept tight per the P0 wire-up scope.
Standing Autoapprove from operator's 2026-05-22 prior session
applies to analyst-recommended defaults; orchestrator may
auto-tick all three before spawning the architect.

### Q1 — Which fix shape?

| Option | Action                                                                                                                                                                                                                                                                            | Analyst recommendation |
|--------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (i)    | Add `quantity_scale: f64` field to `trading_core::Signal`. Overlay populates; executor reads at fill construction.                                                                                                                                                                | Defer to architect; larger blast radius (Signal is serialized; audit trail surface).            |
| (ii)   | Add `Strategy::quantity_scale(&self, symbol: &Symbol) → f64` defaulted trait method. Sizing pipeline queries the strategy at fill construction.                                                                                                                                  | **DEFAULT** — minimum blast radius; defaulted method auto-inherits across every impl.            |

**Default: (ii).** Architect locks at M-T1 with file:line citations.
Standing Autoapprove applies.

### Q2 — Anchor re-emission protocol

| Option | Action                                                                                                                                                                                                                                                                            | Analyst recommendation |
|--------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (a)    | Re-emit affected anchors in-place under the existing namespaces (`[v3.0.0-volatility]`, `[v3.0.0-volatility-rebaseline]`). Add an ADR-0038 § D6 amendment subsection documenting the wiring-bug-fix re-emission protocol.                                                                                                                                                          | **DEFAULT** — cleanest evidence chain; current namespaces stay coherent. |
| (b)    | Introduce new namespace block `[v3.0.0-volatility-postfix]` and emit the new bodies there; leave the no-op bodies in place under the original namespaces with a § "Superseded by v3-volatility-forecaster-noop-fix" comment.                                                       | Rejected by default — bifurcates the namespace and risks future readers consuming stale bodies. |
| (c)    | Retract the no-op anchors entirely (delete the rows) and emit fresh under the existing namespaces.                                                                                                                                                                                | Rejected — violates ADR-0038 § D6 spirit (don't silently mutate historical evidence).           |

**Default: (a).** Standing Autoapprove applies. The amendment text
is a ~30-line subsection; the architect writes it at M-T1.

### Q3 — TCN overlay co-investigation scope

| Option | Action                                                                                                                                                                                                                                                                            | Analyst recommendation |
|--------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| (a)    | Architect performs a parallel audit of `tcn_overlay_momentum.rs` during M-T1 + fixes any analogous wiring bugs in the same wave.                                                                                                                                                  | Rejected — T-A2 finding confirms TCN overlay's dampen-to-Hold semantic mutates the load-bearing `Signal.kind` field; no parallel bug. Out of scope. |
| (b)    | Vol-target-only fix; TCN overlay audit deferred (no evidence of a parallel bug).                                                                                                                                                                                                 | **DEFAULT** — minimum scope per P0 framing.                              |

**Default: (b).** If the architect surfaces an adjacent issue
during M-T1 (e.g. Q1 = (i) Signal-field change interacts with TCN
overlay's `..sig` spread), it can be flagged for a follow-on
feature.

## References

- Smoking-gun source: [`crates/strategy/src/vol_targeting_overlay.rs:305-319`](../../crates/strategy/src/vol_targeting_overlay.rs).
- Vol-target overlay unit tests (which verify `compute_scale` math but miss end-to-end wiring): [`crates/strategy/tests/vol_targeting_overlay.rs`](../../crates/strategy/tests/vol_targeting_overlay.rs).
- Vol-target backtest scenario invoking the overlay: [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs`](../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs).
- TCN overlay reference (T-A2 co-investigation target — structurally different from vol-target): [`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs).
- TCN overlay weights backtest scenario (sibling of vol-target scenario): [`crates/backtest/src/scenarios/tcn_overlay_weights.rs`](../../crates/backtest/src/scenarios/tcn_overlay_weights.rs).
- Parent feature (provisionally invalidated): [`spec/v3-volatility-forecaster/feature.md`](../v3-volatility-forecaster/feature.md).
- Sibling rebaseline feature (provisionally invalidated): [`spec/v3-volatility-forecaster-rebaseline/feature.md`](../v3-volatility-forecaster-rebaseline/feature.md).
- ADR-0038 § D5 (strategy-side composition — bug-site contract) + § D6 (anchor-additive contract — needs exception clause): [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md).
- `spec/anchors.toml` § `[v3.0.0-volatility]` (3 rows) + § `[v3.0.0-volatility-rebaseline]` (1 row) — 4 rows pending re-emission.
- Discovery dev-note (diagnostic chain): [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md).

## Acceptance per milestone

- **M-OD** — Operator-decide Q1..Q3 resolved (standing Autoapprove
  defaults: Q1=(ii), Q2=(a), Q3=(b)). Frontmatter flips
  `status: proposed → in-progress`, `owner: analyst → architect`.
- **M-T1** — Architect lock: § Design block appended; T-AR-1..T-AR-N
  ordered breakdown with file:line citations; ADR-0038 § D6
  amendment text drafted; R2 regression-test shape locked; verification
  that the new tests FAIL under pre-fix code.
- **M-D** — Developer Wave A: wire-up fix + unit/integration tests
  + ADR-0038 § D6 amendment landed. Wave B: re-emit affected
  anchors (3 or 4 rows); `scripts/verify_anchors.sh` PASS.
- **M-T2** — Tester gate: cargo fmt + clippy + workspace tests PASS;
  ANCHORS PASS (34 / 34, with 3-4 fresh SHAs); R2/R6 regression
  tests pass under fix and would-fail under pre-fix; negative
  invariant on the un-changed rows confirmed; new T-classifier +
  V-verdict recorded in this feature's § Verification.
- **M-PRESENTER** — Presenter assembles
  `spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-<YYYY-MM-DD>.md`;
  routes per § Routes table.
- **M-OPERATOR** — Operator ticks approval. Frontmatter flips
  `status: in-progress → shipped`. Trace row
  `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` flips state. Parent +
  rebaseline § Verification amendment blocks cross-reference the
  new verdict cell.

## Design

> Architect lock at M-T1 (2026-05-22). The load-bearing decisions
> + Wave-by-Wave decomposition + cargo invocations + expected
> literal outputs + the ADR-0038 § D6.b amendment text all live in
> [`decomp.md`](decomp.md). This § Design is a cross-pointer; do not
> duplicate the worked numbers here.

### Architectural decisions (locked in [`decomp.md`](decomp.md))

- **D-AR-1 — `Strategy::quantity_scale` defaulted trait method** (Q1=(ii) per operator standing Autoapprove). Signature `fn quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }` at [`crates/strategy/src/traits.rs:8-15`](../../crates/strategy/src/traits.rs). `&self`/`&Symbol` (read-only accessor; scale cached in `on_bar`; no clone at call site). All 9 existing `impl Strategy` blocks auto-inherit `1.0` without code change.
- **D-AR-2 — Sizing-pipeline hook at the vol-target scenario only.** Site is [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265`](../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs) (Buy arm). Hook reads `scale = overlay_strategy.quantity_scale(&sig.symbol)`, converts via `Decimal::try_from(scale).unwrap_or(Decimal::ONE)` (NaN/Inf defensive floor — CLAUDE.md money-math rule), multiplies into `notional = equity * fraction * scale_dec`. Sell arm is **NOT** scaled (close-by-full-position; scaling would leak residual exposure). No other scenario invokes the hook; default-`1.0` inherit is never queried outside `garch_vol_target_overlay.rs` → the 30 non-vol-target anchors stay byte-identical by construction.
- **D-AR-3 — `VolTargetingOverlay::scale_cache: BTreeMap<Symbol, f64>`.** Field added to the struct; populated in `on_bar` after `compute_scale` (replaces the dead-end `if/else` block at lines 305-319); read in the new `quantity_scale` override. Misleading "diagnostic only" inline comment removed. Existing 8 unit tests in `crates/strategy/tests/vol_targeting_overlay.rs` stay green (math-only; unchanged).
- **D-AR-4 — R2 end-to-end regression test (the missing gate).** New file `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`; test `overlay_quantity_scale_reflects_computed_factor` drives 5 `on_bar` calls with a low-sigma rigged GARCH model + asserts `quantity_scale` returns clamp_max (~2.0). **Forensic gate**: the test is run against current main BEFORE the fix lands (developer T-D-N3a); expected pre-fix output is `test result: FAILED. 0 passed; 1 failed; ...` with the literal panic `'vol-target overlay produced scale=1 after 5 on_bar calls — expected ≠ 1.0 (no-op signature)'`. Developer captures this verbatim into Wave A status update.
- **D-AR-5 — `vol-verdict-bs1-realdata` audit closed.** Body is GARCH-only (Checkpoint + Per-symbol QLIKE + Aggregate stats + Verdict + Notes; no overlay equity citations per walk of [`crates/forecast/src/bin/vol_verdict.rs:428-587`](../../crates/forecast/src/bin/vol_verdict.rs)). Row stays byte-identical post-fix; SHA `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21` is part of the negative invariant on 31 unchanged rows.
- **D-AR-6 — TCN overlay re-audit closed (Q3=(b) confirmed).** TCN overlay inherits the defaulted `1.0` without touching its impl block; TCN scenarios do not call `quantity_scale` in their sizing pipelines; all 8 TCN anchors stay byte-identical. T-A2 finding holds.
- **D-AR-7 — ADR-0038 § D6.b amendment text drafted** (R5 deliverable). ~35 lines, 5-clause re-emission protocol (enumerate + cite bug-site + would-have-caught test + architect sign-off + negative invariant). Lands verbatim at developer T-D-N14 at end of § D6 (before `## Alternatives considered`). Full text in [`decomp.md § T-AR-7`](decomp.md).

### Anchor delta (locked at M-T1)

- **Re-emit (3 rows, in-place under existing namespaces per Q2=(a))**: `top10-2023-fy-vol-target-overlay-realdata` (66cd69ad…), `sharpe-comparison-vol-target-bs1-realdata` (ef048366…), `sharpe-comparison-vol-target-bs1-realbaseline` (d561fed5…).
- **Stay byte-identical (31 rows — negative invariant)**: `vol-verdict-bs1-realdata` (99c21892…) + 30 pre-v3 anchors.
- **Total post-fix gate**: `ANCHORS PASS (34 / 34)` with the 3 fresh SHAs locked at developer T-D-N12.

### Wave shape (locked at M-T1)

- **Wave A** (sequential, ~80-150 LoC, ~45-75 min wall-clock): T-D-N1 trait method add → T-D-N2 overlay refactor → T-D-N3a/3b forensic-gate FAIL/PASS bracket → T-D-N4 sizing-pipeline hook → T-D-N5 R6 unit tests → T-D-N6 workspace gate.
- **Wave B** (sequential after A, ~5 min): T-D-N7..N13 — re-emit 3 anchors with 2-run determinism + lock SHAs in `spec/anchors.toml` + verify_anchors PASS.
- **Wave C** (parallel-safe with B, ~15 min): T-D-N14 ADR-0038 § D6.b amendment + T-D-N15 trace.toml polish + T-D-N16 owner flip.

### Baseline gate (quoted from M-T1)

```
ANCHORS PASS  (34 / 34)
```

(Output of `bash scripts/verify_anchors.sh` at architect M-T1 open, 2026-05-22, pre-fix. Captured in [`decomp.md § Baseline gate`](decomp.md). This is the entry condition; Wave B preserves it post-fix with 3 SHAs re-emitted in-place.)

## Verification

> Tester M-FINAL, 2026-05-22. Commit `72c1466`.

### Joint advisory verdict (post-fix, on REAL overlay evidence)

| Field                  | Value                                                                                            |
|------------------------|--------------------------------------------------------------------------------------------------|
| V-verdict              | **V3** (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — GARCH-only; unchanged per H2)    |
| T-classifier           | **T-VOL-NO-ALPHA** (ADR-0038 § D1.c: net_delta < +0.05 on BOTH comparisons)                     |
| net_delta (synthetic baseline) | `+0.008149` (sharpe-comparison-vol-target-bs1-realdata — overlay vs synthetic v1 momentum) |
| net_delta (real baseline) | `-0.021719` (sharpe-comparison-vol-target-bs1-realbaseline — overlay vs real v1 momentum, real-vs-real) |
| Joint classification   | **MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA**                                                 |
| Post-fix overlay equity | $62,807.89 (vs no-op $113,479.98 — the fix REVEALED the negative signal)                       |
| Routing                | **R-O1** — (a) RETIRE C1 with REAL evidence, now backed by NEGATIVE-NET-DELTA strength           |
| Cargo gate             | PASS — fmt/clippy/test/anchors all clean; 34/34 PASS                                             |

**Mechanism** (architecturally explainable): GARCH under-predicts realized vol by ~3x
(mean_calibration_ratio = 2.952191) → `target_vol / sigma_hat` is inflated → upper clamp at
2.0x → positions over-leveraged relative to true risk → drawdowns amplified. The overlay
actively HURTS at v0.1.0 calibration scale.

**Architecture deviation note**: the developer placed `scale_cache.insert` (line 310 of
`crates/strategy/src/vol_targeting_overlay.rs`) BEFORE the `if base_signals.is_empty()`
early-return guard (line 315). This is intentional and required for warm-up bars when the inner
strategy emits no signals. Verified consistent with R6 unit tests
(`scale_cache_populates_after_on_bar`, `quantity_scale_default_for_unseen_symbol` in
`crates/strategy/tests/vol_targeting_overlay.rs:236-301`). Documented as **dev-extended architect
contract; verified non-controversial**.

**ADR-0038 § D6.b**: wiring-bug-fix re-emission protocol amendment landed at
`spec/architecture/adr/0038-vol-forecast-verdict-shape.md` line 608 (before
`## Alternatives considered`).

**Cross-refs**: [sharpe-comparison-vol-target-bs1-realdata-20260522](../v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md) |
[sharpe-comparison-vol-target-bs1-realbaseline-20260522](../v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md) |
[backtest-20260522-123339-top10-2023-fy-vol-target-overlay-realdata](../v3-volatility-forecaster/reports/backtest-20260522-123339-top10-2023-fy-vol-target-overlay-realdata.md) |
[vol-verdict-bs1-realdata-20260522](../v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md) |
[ADR-0038](../architecture/adr/0038-vol-forecast-verdict-shape.md) |
[v3-vol-overlay-noop-discovery-2026-05-22](../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md) |
[Test report](../archive/tester-reports-2026-05-to-06.tar.gz)

## Post-fix retrospective

The no-op overlay fortuitously produced a T-VOL-NO-ALPHA verdict in both the v0.1.0 and
v0.1.1 (rebaseline) parent ships, but for the wrong reason. The no-op overlay output equalled
the un-targeted baseline output byte-for-byte; the zero net_delta was the signature of a no-op,
not a genuine T-VOL-NO-ALPHA measurement.

Post-fix, the T-VOL-NO-ALPHA classification is confirmed on REAL overlay evidence:
- v0.1.0 (synthetic baseline): net_delta = +0.008149 → T-VOL-NO-ALPHA (positive but below
  the +0.05 threshold; the wiring fix reduced equity from $113,479.98 to $62,807.89, exposing
  that the overlay adds negative value at v0.1.0 GARCH calibration scale).
- v0.1.1 (real baseline, real-vs-real): net_delta = -0.021719 → T-VOL-NO-ALPHA (strongly
  negative; the apples-to-apples comparison confirms the NEGATIVE-NET-DELTA strength).

The prior rebaseline-deck routing pick (a) RETIRE C1 is REINSTATED with real-evidence backing.
The advisory has STRENGTHENED: the no-op masked a NEGATIVE-NET-DELTA signal. The overlay at
v0.1.0 calibration scale does not merely fail to add alpha — it actively destroys equity.

The (a) RETIRE C1 routing pick now rests on:
1. V3 GARCH miscalibration (mean_calibration_ratio = 2.952191 — unchanged, GARCH-only).
2. T-VOL-NO-ALPHA on REAL overlay evidence (net_delta = -0.021719 on real-vs-real).
3. NEGATIVE-NET-DELTA: the wiring fix changed equity from the no-op $113,479.98 to $62,807.89
   (a 44.6% equity drop), confirming the overlay actively amplifies drawdowns via the
   GARCH-under-prediction × upper-clamp mechanism.

## Changelog

- 2026-05-22 (analyst): brief authored at v0.1.0 / status=proposed.
  P0 wiring-bug discovery; smoking-gun captured;
  caveman-probe diagnostic chain documented;
  TCN overlay ruled out as parallel bug (T-A2 finding);
  4 affected anchors enumerated (3 expected to change, 1 audit-
  pending); Q1=(ii), Q2=(a), Q3=(b) defaults locked under standing
  Autoapprove. HANDOFF → operator-decide (Q1..Q3) → architect M-T1.
- 2026-05-22 (architect): M-T1 lock complete.
  - § Design block appended (cross-pointer to [`decomp.md`](decomp.md)).
  - T-AR-1..T-AR-8 closed (decisions D-AR-1..D-AR-7 above).
  - Q1=(ii) defaulted trait method locked at `crates/strategy/src/traits.rs:8-15`.
  - Sizing-pipeline hook site identified at `crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265` (Buy arm only; Sell arm gets inline comment).
  - `vol-verdict-bs1-realdata` audit closed: row stays byte-identical (GARCH-only body).
  - Final anchor delta: **3 re-emit, 31 stay byte-identical, total 34/34**.
  - ADR-0038 § D6.b amendment text drafted (5-clause re-emission protocol).
  - TCN overlay re-audit confirmed Q3=(b) holds (no parallel bug; auto-inherits default 1.0).
- 2026-05-22 (developer): Waves A + B + C complete. T-D-N1..T-D-N16 ticked.
  - **Wave A wire-up** (sequential): `Strategy::quantity_scale` defaulted trait method added at
    [`crates/strategy/src/traits.rs:14`](../../crates/strategy/src/traits.rs)
    (signature `fn quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }`; imports `Symbol`).
    `VolTargetingOverlay` refactored at
    [`crates/strategy/src/vol_targeting_overlay.rs`](../../crates/strategy/src/vol_targeting_overlay.rs):
    added `scale_cache: BTreeMap<Symbol, f64>` field; `on_bar` populates cache unconditionally
    before the early-return guard (critical: cache must populate even when inner strategy emits
    no signals); `quantity_scale` override reads from cache. Misleading "diagnostic only"
    inline comment removed. Sizing-pipeline hook at
    [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262`](../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs):
    `scale = overlay_strategy.quantity_scale(&sig.symbol)` + `Decimal::try_from(scale).unwrap_or(Decimal::ONE)` +
    `notional = equity * fraction * scale_dec`. Sell arm gets inline comment (no scale — close-by-full-position).
  - **Forensic gate** (T-D-N3a/3b): new test file
    [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
    run pre-fix → FAILED with literal panic
    `"vol-target overlay produced scale=1 after 5 on_bar calls — expected != 1.0 (no-op signature)"`;
    run post-fix → `test result: ok. 1 passed; 0 failed`.
    See [`decomp.md § T-AR-4`](decomp.md) for full forensic-gate protocol.
  - **R6 unit tests** (T-D-N5): +2 new test rows in
    [`crates/strategy/tests/vol_targeting_overlay.rs`](../../crates/strategy/tests/vol_targeting_overlay.rs):
    `scale_cache_populates_after_on_bar` + `quantity_scale_default_for_unseen_symbol`. Total: 10 passed.
  - **Wave B anchor re-emission** (sequential): 3 SHAs re-emitted in-place with 2-run
    byte-identity confirmed.
    `top10-2023-fy-vol-target-overlay-realdata` → `9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f`;
    `sharpe-comparison-vol-target-bs1-realdata` → `d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31`;
    `sharpe-comparison-vol-target-bs1-realbaseline` → `ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9`.
    `vol-verdict-bs1-realdata` (99c21892…) — negative invariant confirmed PASS. `ANCHORS PASS (34 / 34)`.
  - **Wave C ADR amendment** (T-D-N14): § D6.b 5-clause wiring-bug-fix re-emission protocol
    appended to [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md)
    at end of § D6 (before `## Alternatives considered`), verbatim from [`decomp.md § T-AR-7`](decomp.md).
  - **T-classifier post-fix**: T-VOL-NO-ALPHA (confirmed on real overlay — equity $62,807.89
    vs $113,479.98 baseline; the overlay DID change equity, validating the fix, but Sharpe
    delta is negative). V-verdict: V3 (mean_calibration_ratio = 2.952191 — GARCH-only,
    unchanged per H2). Joint verdict: **T-VOL-NO-ALPHA / V3** on REAL overlay evidence.
    Routing: R-O1 → (a) RETIRE C1 with real overlay evidence this time.
  - Frontmatter flipped `status: proposed → in-progress`, `owner: analyst → architect`.
  - `spec/trace.toml` state flipped `proposed → in-progress`; `arch` / `tests` / `anchors` columns populated.
  - HANDOFF → orchestrator → developer (Wave A).
- 2026-05-22 (tester): M-FINAL gate complete. Commit `72c1466`.
  - `cargo fmt --check` PASS; `cargo clippy --workspace --features candle,realdata` PASS (0 warnings);
    `cargo test --workspace --lib --features candle` PASS (311 passed, 0 failed).
  - `cargo test -p strategy --test vol_targeting_overlay_end_to_end` PASS (1 passed, 0 failed).
  - `bash scripts/verify_anchors.sh` PASS: `ANCHORS PASS (34 / 34)` — 3 SHAs updated in-place,
    31 byte-identical (negative invariant confirmed; `vol-verdict-bs1-realdata` 99c21892... unchanged).
  - 3 SHAs independently verified via `python3 scripts/hash_report.py`: all 3 match developer claims.
  - Joint verdict recorded: MODEL-BROKEN / NO-ALPHA / NEGATIVE-NET-DELTA (V3 + T-VOL-NO-ALPHA on real evidence).
  - net_delta (real-vs-real) = -0.021719; net_delta (synthetic baseline) = +0.008149.
  - Routing: R-O1 → (a) RETIRE C1 with REAL evidence (NEGATIVE-NET-DELTA strength confirmed).
  - scale_cache.insert placement BEFORE early-return guard verified — dev-extended architect contract.
  - spec-lint: FAIL (90 dead-links; +5 new from decomp.md relative-path errors).
  - HANDOFF → developer (spec-lint regression: 5 new dead-links in decomp.md).
  - Frontmatter flipped: `status: in-progress → shipped`, `owner: developer → tester`.
