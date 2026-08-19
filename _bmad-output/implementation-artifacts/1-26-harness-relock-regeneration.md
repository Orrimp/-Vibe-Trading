# Story 1.26: harness-relock-regeneration

Status: ready-for-dev

<!-- Created 2026-08-19 by the operator's AC4 split ruling on 1-25. 1-25 keeps the
     CODE deliverable (the eight CRITICALs); this story owns the REGENERATION —
     AC4 (re-lock + errata + verdict re-derivation) and AC5 (band re-examination).
     The split line falls BEFORE regeneration per the architect plan §6, so a
     partial corpus can never be produced. Disclosure of record: bug-log #67. -->

## Story

As the operator of the Honest Advisor,
I want the 34 contaminated anchored surfaces regenerated under a formal re-lock once every
harness code fix has landed,
so that the C2/C3 research verdicts rest on real execution arithmetic — with the old rows frozen
as history, the migration honest, and the verdict re-derivation loud.

## Acceptance Criteria

1. **Entry gate — ALL code fixes landed first.** #67, #75, #71, #76 (done, 1-25), #72/#73 (code
   halves 2026-08-04), plus the two 2026-08-19 rulings: **#69 wired** (`size_portfolio_target`
   actually called, binding test present) and **#68 dropped** (drift axis removed from the grid and
   from every surface that presents it as an explored dimension). Regenerating before all of these
   land produces a second contaminated corpus — that is the whole reason for the split.
2. **Inventory: 34 anchors** — `#86`, `#87`, `#88`, `#89`, `#90`, `#91`, `#92-#99`, `#100-#107`,
   `#108-#119` = **29 % of the 119-anchor corpus**. Two lanes (`run_path`, `run_cell`). BUYHOLD rows
   are clean throughout (pure mark-to-market, never construct an `Order`).
3. **`--out-dir` is MANDATORY on every invocation.** `param_robustness_sweep`'s default out-dir points
   **into the anchored corpus** (`evidence/v1/momentum-parameter-robustness-sweep/reports/`). A
   measurement run was launched on defaults 2026-08-16 and killed mid-compute before it wrote;
   `evidence/` stayed clean and anchors held only because the sweep is slow. Regeneration goes to a
   NEW namespace per ADR-0038/0045 § D6; old rows stay byte-frozen.
4. **Errata**: per-scenario old-vs-new headline numbers and **re-derived** verdicts. The C2 FRAGILE
   and sweep FAMILY-UNIFORM-FRAGILE conclusions re-stated from clean arithmetic — whichever way they
   land. Any flip touching the era-qualified thesis's supporting narrative **escalates to the operator
   BEFORE publication** (AD-19 spirit).
5. **Band re-examination (AC5 carried from 1-25).** State explicitly, with numbers, whether the frozen
   `classify_verdict` / `verdict_bands` thresholds classify the CLEAN surfaces as they classified the
   contaminated ones. The gate stays byte-frozen either way (AD-1); the deliverable is the ANSWER,
   plus escalation if clean numbers sit near a band edge. A frozen gate whose calibration is never
   re-examined is an assumption, not a guarantee.
6. **Unblocks 1-21.** Its triad is deliberately unflipped pending a correctly-signed MN re-run. The
   re-derived verdicts — not the 1-21 review — decide what the market-neutral basis spread shows.
7. Standing floor: anchors green (old + new rows); spec-lint PASS; advisor-gate independence
   re-proven (`bakeoff/bootstrap.rs` resamples returns — assert its inputs/outputs unchanged).

## Tasks / Subtasks

- [ ] Confirm the AC1 entry gate: #69 wired and #68 dropped, both verified.
- [ ] Schedule the compute window (see Dev Notes — measured, not estimated).
- [ ] Regenerate all 34 surfaces to a NEW namespace with `--out-dir` set explicitly.
- [ ] Errata + verdict re-derivation (AC4), escalating any thesis-touching flip first.
- [ ] AC5 band re-examination, with numbers.
- [ ] Review: old rows intact, new rows complete, verdict-delta table honest.

## Dev Notes

- **Compute budget — MEASURED 2026-08-16, not estimated.** One θ-surface = **1087.11 s (18.1 min)**,
  exit 0; release build 8.65 s (one-time); 13 482 s user ⇒ **~12.4× parallelism already in use**.
  **34 surfaces ⇒ ≈10.3 h sequential.**
- **Read 10.3 h as a FLOOR.** The measured lane is long-only momentum — the cheap end. The MN family
  (#108-#119) runs *"~2× the order traffic of any prior lane: 6 legs plus buy-to-covers"*; the
  basis-reversal family (#100-#107) is the heaviest Sell-traffic lane at **60 k–318 k trades / 200
  paths**. And there is **no parallelism headroom** — one surface already saturates ~12.4 cores, so
  this is close to a serial budget. **Plan a multi-day window; 15–20 h is the realistic figure.**
- **Riders that must ride this regeneration** (carried from 1-25 AC3): √8760-vs-√8575 — and if
  ratified or fixed, **re-sync the cockpit const in the same pass**, since nothing else will trip;
  sentinel-zero pooling; negative-final Calmar guard; slippage-aware solvency pre-flight; the basis
  publication-lag ruling (declare `0` with corrected justification, or `3_600_000`). Note bug-log
  **#89**: `PaperEngine`'s seed is inert, so the FILL_SEED domain-separation rider buys nothing until
  the value is actually consumed.
- **Disk**: `target/debug` reached 155 GB during 1-25 and filled the box. `[profile.dev]`/`[profile.test]`
  now cap it (`debug = "line-tables-only"`, `split-debuginfo = "unpacked"`); `release` — which this
  story uses — is untouched.
- Architect plan: [`docs/dev-notes/1-25-architect-seam-and-relock-plan-2026-08-16.md`](../../docs/dev-notes/1-25-architect-seam-and-relock-plan-2026-08-16.md)

### References

- Predecessor: `1-25-harness-fill-correctness-relock` (the CODE half).
- Blocks: `1-21-perp-basis-mn-spread` (AC6).
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1.
