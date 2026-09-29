# Story 1.30: threshold-sweep-drift-bisect

Status: review

<!-- 2026-09-29: AC1, AC2, AC4, AC6 and AC7 are DELIVERED (both bisects run, re-lock landed,
     citations moved). AC3 and the AC5 remainder are NOT: the lane still installs no tracing
     subscriber and renders no order/fill counts, so the repair's two halves — per-symbol routing
     and the Bug-B solvency pre-flight — still cannot be separated by measurement. Status is
     in-progress rather than review for exactly that reason.

     Created 2026-09-28 by the operator's ruling on bug-log #129: bisect first, then re-lock.
     Same shape as 1-27, which bisected the four #[ignore]d determinism gates to 11acd126 and
     re-emitted 15 rows. The difference is stated in "Method" below and it is not cosmetic. -->

## Story

As the operator of the Honest Advisor,
I want the threshold-sweep lane's divergence traced to the commits that caused it before its anchors are re-pinned,
so that the re-emission records a cause instead of a new number, and so that a verdict flip from `T-MARGINAL` to `T-ALPHA-UNLOCKED` reaches a reader as an explained change rather than as a discovery.

## What is already measured (2026-09-27, `docs/dev-notes/129-threshold-sweep-measurement-2026-09-27.md`)

Six distinct body digests for two scenarios across three states:

| state | bs1 | bs2 |
|---|---|---|
| ANCHOR — locked 2026-05-21 at `447c0432` | `551cc2ab…` | `755bc380…` |
| PRE-FIX — HEAD with `run_cell` untouched | `1dc4855f…` | `811f0c01…` |
| POST-FIX — HEAD with the `#129` repair | `04564535…` | `f7008296…` |

**PRE-FIX ≠ ANCHOR**, so the lane stopped reproducing before any of today's work, while
`verify_anchors.sh` reported 119/119 throughout. That is the third such family after 1-26's 34
θ-surfaces and 1-27's 15 rows, and it was outside both inventories.

The verdict had already flipped at PRE-FIX: `max_cell_sharpe − v1_sharpe` went `0.0152` → `1.8490`,
crossing the `+0.10` line at `bin/threshold_sweep.rs:228-235`. Of that growth, **38 % is the CONTROL
getting worse** (v1 Sharpe `+0.003098` → `−0.693194`), not the strategy improving. The FROZEN gate is
uninvolved — `T-*` is this one report's vocabulary over a 9×5 grid, no bootstrap, no DSR, no
multiple-testing correction, maximum taken over 45 cells.

Each run costs **~5 min** (measured: 286–295 s wall clock, release, `--features candle,realdata`).

## Method — this is NOT 1-27's bisect, and assuming it is will produce a wrong answer

1-27 had one cause: the gates were red because `11acd126` made the engine correct. Here **54 commits**
touch the files feeding this lane between `447c0432` and HEAD
(`git log 447c0432..HEAD -- scenarios/threshold_sweep.rs bin/threshold_sweep.rs paper.rs engine.rs
tcn_overlay_momentum.rs stats/`), and several are known to move money arithmetic on their own:
`11acd126` (`#67` engine guard — mechanistically predicted to hit this lane, since the guard sits
beneath it and `run_cell` never received the routing half), `b3332d35` (the forward loop now pays what
the bake-off pays), `83378c59` (short legs now pay what long legs pay), `b1d96e72` (funding accrual
wrong twice).

So a plain `git bisect` answers "which commit FIRST broke byte-reproduction" — which is worth knowing
and is **not** the same as "what moved the numbers". Expect a sequence, and say so. The deliverable is
a **per-commit attribution list**, not a single SHA, with each entry naming its mechanism at a
`file:line`. Bug-log `#125` is the standing warning: a drift that resembles `#67`'s signature may have
a different cause, and folding it in destroys the evidence.

## Acceptance Criteria

1. **First divergence found.** The earliest commit at which bs1 stops reproducing `551cc2ab…`,
   identified by bisect. ~6 steps over 54 candidates × ~5 min per run plus builds. Every run writes to
   a tempdir via `--out-dir`; never into `evidence/`.

2. **Every subsequent mover attributed.** For each later commit that changes the body again, its
   mechanism named at a `file:line`. A commit that changes the digest without an identified mechanism
   is recorded as *unattributed* — that is an honest entry, and a guess dressed as attribution is not.

3. **The `run_cell` repair is separated from the rest.** `POST-FIX ≠ PRE-FIX` is measured but its cause
   is not: the repair has two behavioural halves (per-symbol routing, Bug-B solvency pre-flight) and
   the measurement could not separate them, because the bin installs no tracing subscriber (the run
   emitted **zero** log lines at any level) and the report renders no order or trade counts. AC5 fixes
   that; this AC then uses it.

4. **The verdict flip is stated in the errata, not discovered by a reader.** With the composition
   spelled out: how much of the delta is the best cell rising and how much is the control falling. AD-19
   escalation applies before publication — the ruling to re-lock was given while the flip was still
   unknown, and the re-emission does not land until the operator has seen this number.

5. **Instrumentation riders, without which the next measurement is as blind as this one.** Install a
   tracing subscriber in `bin/threshold_sweep.rs` (or state in the file that it has none and why), and
   render per-cell order/fill counts so the report can answer what its own arithmetic did. `#110` added
   a `trades` column to the MN surfaces for exactly this reason; this family never got one.

6. **Re-emission under ADR-0038 § D6.b**, with the landing control this project uses: exactly the
   expected rows RED before any sha moves, the rest byte-identical afterwards, and the superseded
   bodies kept on disk and linked.

7. **The citations move in the same pass.** The old verdict is quoted in `trace.toml:372` (live), in
   **accepted ADR-0036:45**, and inside
   `evidence/v1/v25-tcn-horizon-bump-or-retire/reports/test-final-2026-05-21.md:75` — an `evidence/`
   body that cannot be edited to match without breaking its own bytes. The ADR needs a supersession
   note rather than a silent edit; the frozen body's citation needs an explicit statement that it
   records the 2026-05 reading. This is the `#110` knot and it is an architect/operator call, not
   dev's.

8. **Standing floor.** `ANCHORS PASS (119 / 119)` before and after (AD-2); FROZEN files byte-untouched
   (AD-1) — confirmed uninvolved by grep, and the identity test discharges it; `spec-lint` PASS;
   `check_determinism_anchors` still `35 of 35 … across 5 file(s)`. Verification scoped to the touched
   crates.

## Tasks / Subtasks

- [x] **AC1 DONE 2026-09-28 — first divergence is `13955206`, a documentation reorg** (bug-log `#132`).
      766 commits, no path restriction, GOOD `42e084e0` (verified to reproduce `551cc2ab…` before the
      bisect started — and note the anchored body's own `git_commit: 447c0432` names a commit at which
      the producing binary did not yet exist, so that stamp records the repo pointer, not the code that
      ran). Mechanism: the body embeds the predecessor's PATH at line 98, so `spec/` → `spec/v1/` moved
      the digest with zero numbers changed. `#128e` in a second producer.
- [x] **AC2 DONE 2026-09-29 — the arithmetic step is `11acd126`, the `#67` fix itself.** Eleven steps
      on v1 Sharpe, two `SKIP-build` commits handled as untestable. `+0.003098` → `−0.328302` exactly
      there. Mechanism confirmed from the other end: `run_cell` never got the routing half, so once the
      engine guard returned `Err`, its else-less `let Ok(fills)` chain dropped cross-symbol orders in
      silence. Same commit 1-27 bisected to for its 15 rows, reached independently here.
      **Consequence for AC6: the re-lock pins the POST-FIX bodies (`04564535…`, `f7008296…`).** The
      anchored body is `#67`-contaminated (mispriced fills) and the pre-fix state is drop-contaminated;
      only today's code is correct.
- [x] *(superseded)* **AC2 — bisect the ARITHMETIC step on a NUMBER, not on the body SHA.** The digest is polluted by
      provenance strings and the BMAD migration renames that path again, which would yield another
      pure-path step. Use v1 Sharpe (`+0.003098` at the anchor → `−0.328302` pre-fix): one value,
      present in every version of the body, demonstrably moved. Range: `13955206`..HEAD.
- [x] Do not start the bisect while commits are unpushed. A bisect moves the working tree across four
      months of history; the five commits sitting unpushed on 2026-09-28 (the push is blocked on the
      operator's SSH agent) are the reason this is written down rather than already run.
- [ ] AC1: bisect bs1 against `551cc2ab…`. bs2 as the confirmation, not as a second bisect.
- [ ] AC2: attribute each mover.
- [ ] AC5 before AC3 — the instrumentation is what makes AC3 answerable at all.
- [x] **AC4/AC5(part)/AC6/AC7 DONE 2026-09-29.** Escalated with the composition; operator ruled
      re-emit AND qualify in-body. The report now renders a qualifier COMPUTED FROM THE RUN (limits of
      the T-classifier, the delta's composition, and a negative-baseline sentence when it applies) —
      which caught an orchestrator misreading on its first render: Heatmap A is the Sharpe *delta*, so
      the figures filed as "best cell Sharpe" were deltas. Corrected in the bug-log.
      Re-emitted under D6.b: 4 rows re-pinned (2 scenarios x 2 namespaces), landing control proved
      **4 FAIL / 115 PASS before any sha moved**, exactly 4 sha256 lines changed afterwards and 115
      byte-identical, `ANCHORS PASS (119 / 119)`. Citations moved in the same pass: `trace.toml`
      REQ-V25-TCN-THRESHOLD-TUNING-001 and ADR-0036 (which is `proposed`, not accepted as I first
      recorded) both corrected with the contamination named; the frozen
      `test-final-2026-05-21.md:75` cannot be edited without breaking its own bytes, so the anchor
      comment is the correction of record for it.
      Also fixed here, body-neutral: the report filename no longer hardcodes `-20260521` (the sibling
      of `#130`, and the reason a default run could overwrite an anchored body).
- [x] **AC3 + AC5 ANSWERED 2026-09-29 — and the premise under them was wrong.** This binary DOES
      install a subscriber (`bin/threshold_sweep.rs:700`); `EnvFilter::from_default_env()` admits
      ERROR only when `RUST_LOG` is unset, which is why the 2026-09-27 run showed nothing. So AC5's
      logging half needed no build at all. What it needed was a counter, because the repaired code
      cannot log a refusal — it never hands a foreign bar to the engine, so that branch is unreachable
      by construction. Measured over 47 cells: `cross_symbol_routed` **148 719 of 183 751 fills
      (80.9 %)**, `solvency_skips` **6** (zero in 44 of 47 cells). **AC3's answer: the routing half
      moved the result; the solvency guard contributed 0.003 %.** Body proven neutral — the diagnostic
      run re-emitted bs1 at `924a51bb…`, byte-identical to the re-locked row.
      Per-cell rendering deliberately NOT added; reasoning recorded in the bug-log.

## Dev Notes

- The lane: `crates/backtest/src/scenarios/threshold_sweep.rs::run_cell`, called from
  `bin/threshold_sweep.rs:783/:845/:937`. It was repaired on 2026-09-27 (`3c104317`) with the ratified
  `#67` seam plus the Bug-B pre-flight it never received; that repair is landed and is not in question.
- Two body-hygiene riders belong to the same re-emission if it happens: `#130` (the `generated:` field
  is welded to `2026-05-21`, only the time is live) and `#128b` (`parse_gate_survivors` returns nine
  silent zeros on a failed read of a hardcoded dated corpus path).
- Do-not-build register: not implicated.
- Direction honesty: `T-ALPHA-UNLOCKED` is a label this report computes, not a finding about the
  market. Nothing in this story may be summarised as an edge having been found, and the era-qualified
  thesis is untouched by a point estimate that never faced the robustness gate.

### References

- Trace: `REQ-THRESHOLD-SWEEP-DRIFT-BISECT-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
- Measurement: `docs/dev-notes/129-threshold-sweep-measurement-2026-09-27.md`
- Disclosures: `docs/dev-notes/bug-log.md` § `#129`, § `#128`, § `#130`
- Precedent: `_bmad-output/implementation-artifacts/1-27-determinism-drift-bisect.md`
