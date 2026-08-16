# Story 1-25 — architect decision: fill-correctness seam + re-lock plan

**Date:** 2026-08-16 · **For:** operator ratification · **Story:** `1-25-harness-fill-correctness-relock`
· **Runs as one program with:** `1-24-pwsd-fidelity-relock`

This discharges 1-25's first task ("Architect: fill-correctness seam decision … + re-lock plan").
It is a **decision document, not an implementation**. Nothing in the tree changed.

---

## 1. The defect, restated from source

`PaperEngine::step(&mut self, bar: &Bar, orders: Vec<Order>)` takes **one bar and many orders** and
prices every order at that bar's close. It never checks that an order's symbol matches the bar's.

The harness lanes (`scenarios/montecarlo.rs::run_path`, `bin/threshold_sweep.rs::run_cell`) iterate
**merged multi-symbol bars** and hand each order to `step` with whatever bar is currently being
processed. So an order for symbol X is filled at symbol Y's price. That is bug-log **#67**.

**This is already documented at source, with a measured instance** — `montecarlo.rs:1184-1188`:

> *"`run_path` hands every order to `engine.step(bar, …)` with the CURRENT bar, so a fill is priced at
> whatever symbol's bar is being processed (bug-log #67, owned by 1-25). With equal marks that
> mispricing is a no-op … **A first draft of this fixture used 1000/500 and came out at 105 000 — #67,
> caught by the control assertion.**"*

Two things follow, and they decide the seam:

1. **The engine's signature already carries an unstated precondition.** `(bar, orders)` is only
   meaningful if every order belongs to that bar's symbol. The precondition exists; it is simply
   undocumented and unenforced.
2. **The codebase has already had to design a test fixture around the bug** — both symbols are
   deliberately converged to 1000 so the mispricing becomes a no-op. That workaround is a standing
   trap: the next fixture author who picks unequal marks silently measures #67 instead of their
   feature, exactly as the first draft did.

---

## 2. Seam decision — **RATIFIED 2026-08-16: engine guard** (AC1 option A)

> **Operator ratified the engine guard on 2026-08-16.** The recommendation below stands as
> written; AC1's same-bytes live-path proof is now a binding deliverable, not an option.

> `PaperEngine::step` rejects (typed) any order whose `symbol` differs from `bar.symbol`.

**Why the engine, not the callers:**

| | engine guard | caller routing |
|---|---|---|
| makes the existing implicit precondition **explicit** | ✅ | ❌ leaves it implicit |
| a future harness caller can reintroduce #67 | **impossible** | ✅ silently, forever |
| retires the fixture trap above | ✅ | ❌ trap remains |
| touches the shared engine | yes — but see below | no |
| effort | one guard + one proof | per-lane routing in 2+ lanes |

**The "touches the shared engine" risk is provably nil, and AC1 already demands the proof.** Every
live/agent caller passes a single-symbol batch built from the bar it is stepping — verified at
`agent/src/runtime.rs:2280, :2310, :2385` (all `vec![ord]` or long-only orders against the current
bar). On those paths the guard's predicate is *always true*, so it is a no-op by construction. AC1
requires this be demonstrated, not asserted: a dedicated same-bytes test plus the existing suites.

**The deciding argument is this session's own lesson.** Caller routing is a *convention* — it works
until someone forgets. The engine guard is an *invariant* — it cannot be forgotten. Two fixes this
month took the structural form and are the better for it: #80 ended with `try_open_short`/
`try_cover_short` having **zero production call sites** (the seam is gone, not patched), and #86
replaced a one-directional lint with a bidirectional one. A rule nothing can violate beats a rule
everyone is told to follow.

**Explicitly rejected:** silently *deferring* a mismatched order to its own symbol's bar inside the
engine. That would make the engine reorder execution — a behaviour change disguised as a bug fix,
and impossible to prove byte-identical. Rejection is honest; the harness must then route correctly.

**Consequence for the harness:** with the guard in place, the lanes must batch per symbol. That is
the actual fix, and the guard is what makes its absence loud instead of silent.

---

## 3. Re-lock plan

**Inventory: 34 anchors — `#86`, `#87`, `#88`, `#89`, `#90`, `#91`, `#92-#99`, `#100-#107`,
`#108-#119`. That is 29 % of the 119-anchor corpus.** Two production lanes are implicated
(`run_path`, `run_cell`); BUYHOLD rows are clean throughout (pure mark-to-market, never construct an
`Order`).

**Sequencing — the order matters, because several defects interact:**

1. **Land the code fixes first, all of them, before any regeneration.** #67 (seam), #68, #69
   (exposure cap enforce-or-delete + binding test), #71 (side-aware cap), #72 (settlement cadence),
   #73 (per-settlement funding dedup — *already code-fixed 2026-08-04*), #75 (score/accrual channel
   collision), #76 (inverted residual rank + a **literal-value direction gate**).
   Regenerating before they all land produces a second contaminated corpus.
2. **Then the riders** (AC3): √8760-vs-√8575 — **and if ratified/fixed, re-sync the cockpit const in
   the same pass**, since nothing will trip otherwise; sentinel-zero pooling; negative-final Calmar
   guard; slippage-aware solvency pre-flight; FILL_SEED domain separation (**note bug-log #89: the
   `PaperEngine` seed is currently inert — domain-separating a value nothing reads buys nothing until
   it is wired**); the basis publication-lag ruling (declare `0` with corrected justification, or
   `3_600_000` and re-lock).
3. **Then regenerate under a NEW namespace** per ADR-0038/0045 §D6; old rows stay byte-frozen as
   history.
4. **Then the errata**: per-scenario old-vs-new headline numbers and **re-derived** verdicts.
5. **Then AC5's band re-examination** — state, with numbers, whether the frozen bands classify the
   *clean* surfaces as they classified the contaminated ones. Escalate if clean numbers sit near a
   band edge.

**Escalation clause (AC4, AD-19 spirit):** any flip touching the era-qualified thesis's supporting
narrative goes to the operator **before** publication. Per bug-log #75/#76 the honest current status
of the MN-basis domain is **unknown pending a correctly-signed re-run** — not "closed with finality".

---

## 4. Compute budget — **must be measured, not estimated**

There is **no recorded runtime** for a θ-surface regeneration anywhere in the corpus
(`grep` for took/elapsed/runtime/wall-clock over `evidence/*/reports/*.md` returns nothing). Any
number quoted here would be invented, so none is.

**First step — one measured cell, then multiply:**

```bash
# ⚠️ --out-dir IS MANDATORY. The binary's default out-dir is
#    evidence/v1/momentum-parameter-robustness-sweep/reports/ — an ANCHORED
#    directory. Running it without an override writes into the frozen corpus and
#    breaks the AD-2 gate. (Learned the hard way 2026-08-16: a measurement run was
#    launched on defaults and killed before it wrote. evidence/ stayed clean and
#    anchors held 119/119, but only because it was caught mid-compute.)
cargo build --release --bin param_robustness_sweep --features realdata
./target/release/param_robustness_sweep --out-dir /tmp/sweep-out/
```

**Any regeneration during this re-lock must go to a NEW namespace anyway (§3 step 3),
so `--out-dir` is load-bearing twice over — once for anchor safety while measuring,
once for the re-lock's own namespace discipline.**

Run that once, record it in this document, and the budget becomes arithmetic. Until then the honest
statement is: **34 surfaces, unknown unit cost, needs one measurement.** That measurement is itself
a small task and should be the operator's first authorisation, ahead of the full budget.

---

## 5. Decisions — BOTH ANSWERED 2026-08-16

1. **Seam: RATIFIED — engine guard.** Implementation may proceed on that basis, with AC1's
   same-bytes proof for the live/agent paths as a hard gate.
2. **Measurement run: AUTHORISED.** One θ-surface timed end-to-end (200 paths, tier1,
   momentum, 2023, vol-adjusted-return — the binary's own defaults), build time recorded
   separately from run time because only the run scales by 34. Result recorded in §4 above
   once it lands. Full-corpus authorisation remains pending that number.

### Superseded — the two decisions this document originally asked for

1. **Ratify the seam** — engine guard (recommended) vs caller routing.
2. **Authorise the measurement run** above, so the compute budget stops being a guess. Full-corpus
   authorisation can wait until the number exists.

## 6. Note on scope honesty

1-25 has accreted riders from six separate reviews (1-14, 1-15, 1-16, 1-17, 1-18, 1-20, 1-21). It is
now one story carrying **eight CRITICALs, 34 anchors and ~15 riders**, and it blocks 1-21's closure.
That is a lot for one story, and splitting the *code fixes* from the *regeneration* into two stories
is defensible — but the re-lock cannot be split from itself: a partial regeneration produces a corpus
that is neither old nor new. If a split is wanted, the line is after §3 step 2, not inside step 3.
