# Runbook — the hardening burn-down loop

A `/loop` prompt for finishing the one thing this project still owes. **Draft for operator review
(2026-10-10); not yet run.** Edit it in place and it is what the loop executes.

## The goal, in the project's own words

`README.md` § Status:

> **FEATURE-COMPLETE (2026-07-09) · UNDER REVIEW HARDENING (since 2026-07-26)**
> Its FEATURE LIST is complete; its PRODUCT STATE is mid-hardening.
> The active-trading-thesis closure currently reads *direction-preserved pending re-lock*.

So the goal is **not** to build anything. It is to retire that last clause: get the hardening
burn-down to the point where the thesis closure is **re-locked** rather than pending, and the
project's claims are provable rather than asserted. That is a terminal goal, and the loop should
end when it is reached.

## What stands between here and there

Measured from `sprint-status.yaml` on 2026-10-10 — 151 done, 10 retired, and these open:

| story | status | bearing on the goal |
|---|---|---|
| `1-24-pwsd-fidelity-relock` | **ready-for-dev** | **the critical path.** The last open CRITICAL re-lock; the thesis clause is pending on it |
| `1-25-harness-fill-correctness-relock` | review | the other CRITICAL re-lock, dev-complete |
| `1-28-mn-ruin-and-fill-provenance` | review | |
| `1-29-forecast-bin-reproduction-gates` | review | |
| `1-30-threshold-sweep-drift-bisect` | review | |
| `3-15-advisor-options-impliedvol-probe` | review | |
| `3-16-advisor-crossasset-macro-regime` | review | |
| `2-1-lumen-design-adoption` | in-progress | UI; rendered-pixel proof required (AD-10) |
| `6-9-cockpit-cross-platform` | in-progress | run-2 Linux/Windows shakeout, fix-forward |

**Explicitly OUT of scope for this loop** — do not start these, and do not treat them as blocking:
`2-63-lab-recipe-test-harness-v0-3-extension` (deferred by decision; infra, not required for
feature-completeness), `3-18-advisor-reflection-decision-loop` (gated on an operator green-light),
`3-19-advisor-eur-live-rate`, and anything in
[`docs/dev-notes/do-not-build-register.md`](../dev-notes/do-not-build-register.md). No new features:
the product is feature-complete and that was analyst-verified as an honest null.

## The loop prompt

Everything below the line is what gets pasted after `/loop`.

---

Goal: finish the hardening burn-down so `README.md` § Status can stop saying the
active-trading-thesis closure is *"direction-preserved pending re-lock"*. Work the items below;
decide and proceed; record every decision in the artifact it belongs to.

**Order, and why.** `1-24-pwsd-fidelity-relock` first and alone — it is `ready-for-dev`, it is the
last open CRITICAL re-lock, and the thesis clause is pending on it. Then the `review` queue, closest
to shipped first. `2-1` and `6-9` last; they are `in-progress` and neither gates the thesis.

**Each iteration:** read `sprint-status.yaml` and `docs/dev-notes/bug-log.md` for current state
rather than assuming it, pick the one item that most advances the goal, do it to completion, run the
gates, commit. Prefer finishing one item over touching three.

**Non-negotiables that bite in a loop** (the full set is `CLAUDE.md`; these are the ones a long
unattended run will walk into):

- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (119 / 119)` **before and after** any change
  touching `evidence/` (AD-2). Anchored report bodies are byte-immutable; a body change is an
  ADR-0038 § D6.b re-emission with its five steps, never a re-pin (`#77`).
- The FROZEN robustness gate stays byte-untouched: `classify_verdict` / `verdict_bands` /
  `compute_robustness_flag` / `rank_candidates` (AD-1).
- Money math is `rust_decimal::Decimal`, never `f64` (AD-9). UI ships only on rendered-pixel proof
  (AD-10). Any strategy overlay or sizing modifier needs its day-1 baseline-divergence e2e (AD-16).
- Scope verification to the crates you touched. **Never** a workspace-wide `clippy --all-targets` —
  one measured run took 52 minutes and grew `target/` to 155 GB.
- `spec-lint` PASS, and the ADR-0082 triad moves together when a story flips to `done`: story
  `Status:` ↔ `trace.toml` `state=` ↔ CHANGELOG line, in the same pass.
- The ship-passive thesis is **ERA-QUALIFIED (2023+)**. Never state the universal form.
- Work directly on `main`. Commit with `--no-gpg-sign`. Do not read or commit the three untracked
  audit notes in `docs/dev-notes/` — they are not mine.

**Running the gates costs real time, so plan for it.** The reproduction gates are ~76 min of
measured compute; a single θ-surface pass is ~73 min. Run one load-sensitive measurement at a time,
chain the next behind it rather than idling, and emit a copy-pasteable `watch -n N` block for
anything over two minutes. **A gate run takes the corpus out of play**: `git commit` and any
`evidence/` edit during one makes it report UNMEASURED, so settle the corpus, commit, *then* run.

**House rules earned the hard way — apply them, do not re-derive them:**

- A gate that cannot fail is indistinguishable from one that passes. Make the success path
  observable; a skip is UNMEASURED, never a pass; and **probe every gate you add** by deliberate
  breakage, verifying the probe actually changed something before trusting the red.
- Derive a member list from the **criterion**, never from the symptoms you happened to see
  (`#126`, and `#119`'s fix list which missed four siblings).
- Never re-implement a measurement harness; extend the one that exists (`#133`, `#136`).
- Put a correction where the reader lands, not where the register looks tidy (`#117`, `#138`).
- A deferral must name **what happens if its carrier never arrives** (`#128c` waited for a
  re-emission that could not come; `#133`'s got lucky the next day).
- Restore a probed file from a snapshot, never `git checkout --` — it discards uncommitted work.

**Stop and ask me — one short, concrete question, after writing the decision to disk — only for:**

1. pushing or publishing outward (attempt a push once; if the 1Password SSH agent refuses, say so
   with the exact error and the unlock step, and **do not retry in a loop**);
2. deleting or overwriting evidence;
3. shipping on a `REGRESSION` verdict (AD-19), or any verdict that **moves**;
4. changing what the project publicly claims — the era-qualified thesis, the do-not-build register,
   or `README.md` § Status.

Everything else: decide, write down why, keep going. Never author an operator-ruling acceptance
criterion outside those four.

**A `review`-status story is dev-complete, so review it rather than re-implementing it.** Use
`bmad-code-review`. Where the review is of my own work, **say so in the verdict** — it is weaker
than an independent pass and the reader is entitled to know which it was.

**The loop is done when** `1-24` is closed, every `review` story is either closed or has a written
verdict, and `README.md` § Status has been updated to say what is true then — item 4 above, so bring
me the proposed wording rather than landing it. Stop the loop at that point instead of finding more
work; say what is left and that it is out of scope.

---

## Queue survey, 2026-10-10 — "review" conflates two different states

Checked rather than assumed, because this repo's status lines lag its code. The seven `review`
stories are not seven stories awaiting a verdict:

| story | verdict on file | what it actually needs |
|---|---|---|
| `3-15-advisor-options-impliedvol-probe` | **FAIL**, 2026-08-12, 16 findings all triaged | a **disposition**, not a review |
| `3-16-advisor-crossasset-macro-regime` | **FAIL**, 2026-08-14, findings all triaged | a **disposition**, not a review |
| `1-24-pwsd-fidelity-relock` | none | a verdict (my work — label the self-review) |
| `1-25-harness-fill-correctness-relock` | none | a verdict |
| `1-28-mn-ruin-and-fill-provenance` | none | a verdict (my work) |
| `1-29-forecast-bin-reproduction-gates` | none | a verdict (my work) |
| `1-30-threshold-sweep-drift-bisect` | none | a verdict (my work) |

**Two stories have sat at `review` for two months holding FAIL verdicts.** That is not a queue of
pending reviews; it is `review` meaning two incompatible things — *"awaiting a verdict"* and
*"failed its verdict and awaiting a decision"* — with nothing in the status to tell them apart.
Same shape as bug-log `#138`: a status line that cannot distinguish the states it is asked to
carry. The lint cannot catch it either, since both are legitimately `review`.

**CORRECTION 2026-10-11.** I characterised both FAILs as *"the product ranks arms that never ran,
under labels saying they did"* — here and twice to the operator — and then checked. It is half
right, and the half that is wrong matters:

- **`v0.macro_riskon` is ABSENT** from the ranked field, verified in T7.1's 19-arm leaderboard.
  That is `#81`'s honesty half working exactly as designed: the arm is dropped rather than shown
  as a cash stub. **The product is honest here**; what remains open is the story, not the behaviour.
- **`v0.dvol_regime` ran and traded 17 times** in that same leaderboard. Not inert on a machine
  that has the corpus.
- The real exposure is **conditional**: `/data/*` is gitignored (`.gitignore:23`), so on a fresh
  clone or CI box the DVOL parquets are absent, `unwrap_or_default()` yields an empty series, and
  the arm stays in the ranked field as a 100%-cash stub under an honest-sounding label. That is
  `#78`, and the fix has an in-repo precedent: do for DVOL what `#81` already did for macro.

Keeping the original framing below, superseded, because overstating a finding is the same class of
error as the findings themselves.

**Superseded framing:** the two FAILs are the same root finding, and it is not a small one: **the
product ranks arms that never ran, under labels saying they did.** `3-15`'s DVOL arm holds cash through warm-up and in a
calm window never trades (`#78`); `3-16`'s macro arm is **never compiled** — `macro_regime.rs` is
gated on `backtest/yahoo` and nothing in the workspace enables it, so it has run 100% cash in every
build since it shipped (`#81`), while the leaderboard renders *"Macro regime (hold when SPX up, DXY
down, rates calm)"*.

So their disposition is **not** this loop's to take: it changes what the product claims on an
honesty-first product, which is stop condition 4. `#81`'s capability half is already recorded as
BLOCKED by design, so the choice is about the **labels and the ranked field**, not about building
the arms.

## THE GOAL IS NOT REACHABLE AS WRITTEN — 1-25, not 1-24, is the blocker (2026-10-10)

This runbook named `1-24` the critical path on the reading that it is *"the last open CRITICAL
re-lock"* and that README § Status's *"direction-preserved pending re-lock"* clause is pending on
it. **Half right, and the wrong half is load-bearing.** The README names **two** CRITICAL re-lock
stories — 1-24 **and** 1-25 — and the clause is pending on both.

1-24 is now done. **1-25 is NOT CLOSABLE**, by its own independent audit at
`docs/dev-notes/1-25-ac-closure-audit-2026-09-27.md`: *"One AC is self-documented as unmet in the
source at HEAD, and five of seven riders are unmet."* AC2 unmet, AC6 discharged only on substituted
evidence after its cited gate turned out not to measure the claim.

And its inventory is not a punch list — it is seven CRITICAL bug-log entries across anchors
#86–#119, several of which invalidate what their surfaces are taken to show:

- `#75` — the `mn-basis` arm **never ran**: `run_path` overwrote the score map with the accrual map,
  so four anchored surfaces are duplicate funding runs.
- `#76` — the residual arm ranks the basis axis **inverted** vs its own spec, so no anchored MN
  surface ever tested the basis in its documented direction.
- `#72` — carry-4h and carry-daily harvested ~1/4 and ~1/24 of the true funding; those bodies assert
  a settlement cadence they never ran.
- `#73` — funding accrued per symbol-bar rather than per settlement, so every carry lane over-accrued
  by ~10× on the 10-symbol universe.

Closing that is a re-lock of a further ~34 anchored surfaces plus the code fixes behind it —
comparable to or larger than everything this loop accomplished on 1-24. **It is a programme, not an
iteration**, and it is the operator's call whether to start it.

**So the loop stopped here rather than grinding on.** Its own instruction is *"stop there rather
than finding more work"*, and a goal that is out of reach is a better reason to stop than a goal
that is met. What the loop can hand over — an honest README wording that reflects the true state
rather than retiring the clause — is below.

## Operational note — long runs must be resumable (learned 2026-10-10)

The 1-24 regeneration is 75–80 min, and a tracked background job was **killed at ~10.5 min**
having completed 14 of 35 surfaces, taking its whole process group with it. Earlier runs in the
same session completed at 49 and 39 min on an identical timeout, so the cap is not something to
predict — and `setsid nohup … &` from inside a tool call does not escape it either; that was
tried and the child did not survive.

What works is making the kill cheap rather than preventing it:

- **Skip work already done.** A surface whose `.md` exists is skipped on resume, so a kill costs
  only the in-flight surface. Relaunching is then idempotent and needs no bookkeeping.
- **Cheapest work first.** The two expensive θ-surfaces (~24 and ~37 min) cannot fit in any
  single capped job, so they run LAST — a kill during the tail keeps all 33 cheap surfaces.
- **Never write into `evidence/` from a long run.** Already the rule for AC4, and a partial run
  makes it sharper: a job that dies halfway through landing bodies leaves the corpus in a state
  nobody chose.

A job that cannot be made resumable belongs in the operator's own terminal with a `watch` recipe,
not in a capped background slot.

## Before you start it

Two things the loop cannot do, worth knowing in advance:

1. **It cannot clear the stop conditions.** A moved verdict parks it, exactly as bug-log `#137`
   parked the 2026-10-01 run until you ruled on it. Expect to be asked a handful of times.
2. **It cannot fix the 1Password SSH agent.** That blocked the push twice on 2026-10-01. The loop
   will keep committing locally and the queue will grow behind a locked agent — `ssh-add -l` listing
   keys while the push still fails means *locked*, not *down*.

Invocation once the prompt above reads the way you want it:

```bash
/loop
```

with the body above as the prompt — no interval, so the pacing is chosen per iteration from what the
run is actually waiting on (a finished measurement, a CI result, nothing).

## Proposed README § Status wording — for operator sign-off, NOT landed

Changing README § Status is stop condition 4, so this is a draft handed over rather than applied.
It does **not** retire the "pending re-lock" clause, because that would be false; it scopes it to
what is actually outstanding.

> **Current wording:** *"The active-trading-thesis closure currently reads direction-preserved
> pending re-lock."*

> **Proposed:** *"The active-trading-thesis closure reads **direction-preserved, pending the 1-25
> harness-fill re-lock.** Of the two CRITICAL re-lock stories the burn-down opened, **1-24
> (PWSD fidelity) closed 2026-10-10** — its 35 anchored surfaces were regenerated under a corrected
> Politis–White estimator, and the re-derivation moved two of them adversely (one family verdict
> off UNIFORM-FRAGILE), disclosed in
> `docs/dev-notes/1-24-pwsd-relock-errata-2026-10-10.md` (a path relative to the repo root, where
> this wording is destined — deliberately not a live link from here, since the lint resolves links
> against the file they sit in and this is a draft for `README.md`).
> **1-25 remains open and is the binding one:** its own closure audit finds one acceptance criterion
> unmet in the source at HEAD and five of seven riders unmet, over an inventory of seven CRITICAL
> defects spanning anchors #86–#119 — including an arm that never ran (`#75`), an arm ranked
> inverted against its own spec (`#76`), and funding mis-accrued by ~10× on the carry lanes
> (`#72`/`#73`). Until those surfaces are re-derived from clean arithmetic, the direction of the
> closure is preserved and its magnitudes are not."*

Three things that wording does deliberately:

1. **It does not claim the re-lock is finished.** Retiring the clause was this loop's goal and the
   evidence does not support it. Writing the goal into the README anyway is the exact failure the
   whole burn-down exists to correct.
2. **It separates direction from magnitude.** That is the distinction 1-25's own AC4 draws, and it
   is what "direction-preserved" has always meant — the honest claim is narrower than the sentence
   it replaces, not broader.
3. **It names the defects rather than gesturing at them.** A reader who wants to know what "pending
   re-lock" is pending *on* can now find out without reading four stories.
