# Story 1.26: harness-relock-regeneration

Status: in-progress

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

   > **GATE MET 2026-09-25** — verified at source, not assumed. `#69` was wired on 2026-08-23
   > (`723ca742`): `scripts/callers.sh size_portfolio_target` reports a production caller at
   > `montecarlo.rs:520` inside `run_path`, where the bug-log census found zero, and the binding
   > test this AC requires is green.
   >
   > `#68` took the **other** branch of its own "implement-or-drop". The two were one defect —
   > `size_portfolio_target` implements BOTH controls — so wiring it for `#69` made the drift axis
   > live as a side effect (`montecarlo.rs:238` says so). Its binding test varies only
   > `drift_rebalance_threshold` 0.001 vs 0.90 and is green. This AC's letter says "dropped";
   > executing that would have deleted a working dimension, so it went back to the operator, who
   > **ruled 2026-09-25 that implemented-and-binding-tested satisfies the gate**. The axis stays.
   >
   > What still rides the regeneration (AC4): the anchored narratives for `#86`/`#87` attribute
   > cell results to a hold band that was INERT when those surfaces ran. The new rows are the
   > first where the third axis is real.
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
7. **Un-ignore the four drift gates (bug-log #93).** `determinism.rs`'s `t717_*` / `tt1_*`
   `*_anchor_hash_unchanged` tests re-run a scenario and compare its body-SHA to a pin. They are the
   **only** gate that can observe code-vs-evidence drift — `verify_anchors.sh` hashes committed
   bodies and never re-runs, which is why it reported 119/119 green while the code stopped
   reproducing the evidence (`0f6f6eb8…` pinned vs `b655e5e7…` produced). They currently carry
   `#[ignore]`. This story re-derives those pins from the regenerated surfaces and **removes the
   attribute in the same commit** — the flip must be visible in the diff that re-prices the anchors.
   Do NOT re-baseline them any other way: re-pinning a truthful gate to current output is #77.
8. Standing floor: anchors green (old + new rows); spec-lint PASS; advisor-gate independence
   re-proven (`bakeoff/bootstrap.rs` resamples returns — assert its inputs/outputs unchanged).

## Tasks / Subtasks

- [x] **AC1 entry gate SATISFIED 2026-08-23.** #68 and #69 are both WIRED (not dropped — the "drop
  the drift axis" ruling was withdrawn 2026-08-19 on a false premise, then superseded by ADR-0089
  D1/D7). `run_path` builds a signed target vector per rebalance boundary and calls
  `size_portfolio_target`; the gross cap and the drift band both bind, RED-proven in
  `crates/backtest/tests/portfolio_controls_bind.rs`. Landed in `723ca74`.
  **Three things this story inherits from that commit:**
  - **bug-log #94 was fixed in the same pass** (the sizer sized resizes to the target, not the delta).
    Regenerating before it would have produced a corpus with a −74 %-equity artefact baked in.
  - **ADR-0089's "turnover falls" is CORRECTED to "direction unknown".** The old code could not resize
    a held leg at all, so the band bounds NEW behaviour. AC4's errata must REPORT the turnover
    direction from measurement; do not carry the old claim forward as an expectation.
  - **The four `#[ignore]`d determinism pins are NOT part of this movement.** Re-measured after D1:
    byte-identical to their 2026-08-22 values. They cover `scenarios/momentum.rs` and
    `scenarios/tcn_overlay.rs` — lanes outside `run_path` (bug-log #95). Their drift is pre-existing
    and separately caused; re-deriving them is a distinct exercise from re-pricing the 34 surfaces.
- [x] **bug-log #95 recorded as deliberately out of scope.** None of the eight lanes produces an
  inventory anchor, so it does not block. The errata states explicitly that it makes **no**
  engine-wide claim for the exposure cap — only that it binds on `run_path`, the lane behind all 34
  surfaces.
- [x] **Compute window scheduled and run** — `RAYON_NUM_THREADS=8`, `nice -n 19`, one terminal.
- [x] **All 34 regenerated**, 34/34 `ok`, 0 failures, into `evidence/v2/harness-relock/reports/`
  with `--out-dir` explicit (AC3). **Measured wall clock: 74.4 min**, against a Dev-Notes budget of
  "10.3 h floor, 15–20 h realistic". The estimate extrapolated from one long-only momentum surface;
  two surfaces account for 83 % of the run and the other 32 finish in 12 minutes together.
- [x] **AC4 errata + verdict re-derivation** — [`evidence/v2/harness-relock/ERRATA.md`](../../evidence/v2/harness-relock/ERRATA.md).
  156 cells re-priced, 312 verdicts independently re-derived against the frozen bands with **0
  mismatches**. 20 cells FRAGILE → MARGINAL, 0 in any other direction, 6 surfaces change family
  verdict. **Turnover measured: it RISES (+5.5…+7.6 %), correcting ADR-0089's "turnover falls"** —
  on the 9 of 34 surfaces that publish a `trades` column; the errata claims nothing for the other 25.
- [x] **AC5 band re-examination, with numbers** — ERRATA § 5. The bands classify the clean corpus
  differently in kind: `p95_maxdd` goes from tripping **149/156** cells to **0/156**, `p5_sharpe`
  becomes the dominant binding signal, and the old corpus had **no** cell anywhere decided by a
  single signal against **24** in the clean one. Escalated: at N=200 paths `P(Sharpe>1)` resolves to
  0.005 and the band sits at 0.350 — one cell is FRAGILE *solely* on that signal at 0.345, i.e. by
  **one resampled path**. The gate stays byte-frozen (AD-1); no threshold is proposed.
- [x] **AC8 standing floor verified** — anchors 119/119, spec-lint PASS, advisor-gate independence
  17/17, FDR identity 2/2, manifest gate 2/2, FROZEN files byte-untouched. ERRATA § 8.
- [x] **Bodies proven anchor-grade before proposing to anchor them** — 4 surfaces across 3 slugs
  re-run independently, all 4 body-SHAs byte-identical; 0 of 34 bodies carry run-varying metadata,
  so anchoring cannot reproduce bug-log #106.
- [ ] **BLOCKED on operator — verdict-narrative ruling.** ERRATA § 6. The thesis is NOT in question
  (0 of 156 cells beat their own buy-and-hold control, in either corpus; nothing reached ROBUST).
  README and CHANGELOG need **no** change — their claims name `perp-basis-mn-spread` (all 12 held)
  and `momentum-parameter-robustness-sweep` (held). The flip lands on three slugs' verdict lines.
- [ ] **BLOCKED on operator/architect — bug-log #108.** AC3's "new namespace" is what ADR-0038
  § D6.b rejects by name, for a reason that is live here. `anchors.toml` deliberately untouched
  pending the ruling, so AC8's "new rows" leg is open. ERRATA § 6b.
- [ ] **AC7 unsatisfiable as written — bug-log #109.** Re-measured after the re-lock: all four gates
  still RED with hashes *unchanged* from 2026-08-22/23. They cover scenarios that are not among the
  34. Needs an AC amendment, not a commit.
- [ ] Review: old rows intact, new rows complete, verdict-delta table honest.
- [ ] AC6 — unblock story 1-21, once § 6 is ruled.

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
