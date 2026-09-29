# Story 1.29: forecast-bin-reproduction-gates

Status: ready-for-dev

<!-- Created 2026-09-27 by the orchestrator. #126 closed the backtest/param_robustness_sweep/
     monte_carlo family: every scenario those three binaries can produce now has a re-run
     gate. The 15 that remain are a different producer family — six binaries across
     crates/forecast and crates/backtest — and deriving their reproduction conditions
     surfaced five defects, filed as bug-log #128. This story owns closing the family. -->

## Story

As the operator of the Honest Advisor,
I want every anchored scenario the forecast/report binaries can produce to have a gate that re-runs it and compares against the ANCHOR,
so that `verify_anchors.sh` reporting 119/119 stops meaning "the committed bodies still hash to what we wrote down" for a fifth of the corpus and starts meaning "the code still produces them".

## What is actually ungated, derived mechanically

The member list is **derived from the criterion, not written down** (`#126`'s lesson): every scenario
in `evidence/anchors.toml` that no test file and no run manifest names. That yields exactly **15**,
which matches the independent count in `docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md`.

Three of the 15 have no literal occurrence anywhere in the tree because their names are built with
`format!("…-{label}-…")`. A grep-only inventory would have declared them producer-less — the exact
trap `#126` was filed for, caught this time before it cost anything.

| count | rows | state |
|---|---|---|
| 10 | forecast-distribution ×5, vol-verdict, regime-verdict, sharpe-comparison ×3 | measurable today |
| 2 | threshold-sweep-bs1 / -bs2 -realdata-recalibrated | measurable, but **HELD** on `#129` (below) |
| 2 | recalibrate-sigma-train-bs1 / -bs2 | **blocked** — the anchored SHA encodes an input PATH (`#128e`) |
| 1 | sharpe-comparison-realdata | **no producer at HEAD** (`#118`) — operator disposition, not gateable |

Full per-row recipes, with the required CWD, feature set, inputs, out-dir flag and filename shape for
each: `docs/dev-notes/forecast-bin-repro-recipes-2026-09-27.md`.

### The two threshold-sweep rows are HELD, not merely scheduled (`#129`)

`run_cell` — `crates/backtest/src/scenarios/threshold_sweep.rs:56`, the lane that produces those two
rows — never received either half of the `#67` fix, and its own module doc has said so since v0.1.1
(`scenarios/montecarlo.rs:5-9`). Worse than unfixed: both its engine calls are written
`&& let Ok(fills) = engine.step(bar, vec![ord]).await` with **no else arm** (`:228`, `:269`), so once
the `#67` guard began returning `Err` for a cross-symbol order, that order started **vanishing** —
no log, no counter, no fill. The fix did not leave this lane alone; it changed its behaviour, on a lane
carrying two frozen anchors, undisclosed.

A reproduction gate on those two rows would pin a body produced by that lane as correct-by-definition.
**This story must not gate them until `#129` is ruled on.** Converting an undisclosed behaviour change
into a defended invariant is the most expensive version of the mistake this whole gate programme exists
to stop. The other 10 rows are unaffected and proceed.

Cost: **≈ 79 min total**, taken from each anchored report's own `wall_clock_s` front-matter rather
than estimated — from `vol_verdict` at 0.7 s to `recalibrate_sigma_train --scenario bs2` at 619.8 s.
Every required checkpoint and corpus file is present on this machine (verified by `ls`).

## Acceptance Criteria

1. **The two landmines are disarmed BEFORE any gate runs.** `crates/backtest/src/bin/threshold_sweep.rs`
   defaults `--out-dir` into the anchored corpus (`:152`) while hardcoding its filename's date
   (`:1045`) — so a no-argument run overwrites a byte-immutable body in place rather than merely
   planting a newer file. Fix the default (or make the flag required); this changes no report body, so
   it is anchor-neutral and provable as such. And `parse_gate_survivors` (`:246-251`) must stop
   returning nine silent zeros on a failed read of its hardcoded dated predecessor path (`:106`,
   `:109`) — a read failure is an error, not "graceful degradation". Both per `#128a` / `#128b`.

2. **A gate per measurable row, all through ONE path.** 12 gates that re-run the scenario and compare
   the body-SHA against the ANCHOR — never against a second run of themselves. One shared helper, one
   build profile, one invocation shape. `#124` is the precedent for why this is an AC and not a
   detail: a nine-gate family that used two different runners with two unstated conditions produced a
   false drift report within two hours of that condition being documented.

3. **Every gate DECLARES its condition and asserts a witness BEFORE the SHA.** CWD × feature set ×
   build profile × invocation × input corpus, with the anchor namespace declared inline (the
   `// anchor-ns:` convention `scripts/check_determinism_anchors.py` already reads). Passing "no
   flags" counts as a declaration and is stated. Each gate asserts a human-legible line from the
   report itself — its `Data source`, or its feature-dependent path marker — so a changed condition
   fails as a mismatch rather than as fake drift.

4. **No gate may write into `evidence/`.** `--out-dir` to a tempdir, without exception, because
   `verify_anchors.sh` resolves each anchor to the NEWEST match — a red run must not be able to flip
   the corpus gate as a side effect of having been run. `#128a` makes this lethal rather than merely
   untidy for one of the six bins.

5. **A refusal must not be reported as a result.** Three of the six bins declare no
   `required-features`; of those only `sharpe_comparison` bails. `regime_verdict` substitutes an empty
   report and emits V-REG-1 — "EM convergence failure" (`:197`) — with exit 0, so a wrong-feature
   build asserts that EM failed to converge when the binary never ran (`#128c`). Either it refuses, or
   the emitted report names the real cause. A gate built on top of a binary that misdiagnoses its own
   preconditions inherits the misdiagnosis.

6. **Drift is expected; a red is a measurement, not a failure of this story.** Three rows are
   *predicted* to drift because their backtest inputs were re-emitted after they were locked. That
   prediction is dated inference, not measurement. Each actual red is triaged as a D6.b re-emission
   candidate with its cause named at a `file:line` before any SHA moves — never re-baselined, which
   is `#77`.

7. **The two blocked rows get a disposition in writing, not a silent omission.**
   `recalibrate_sigma_train.rs:455-459` writes the resolved overlay path into the hashed body, so
   `--anchor-dir` cannot be redirected without changing the digest (`#128e`). Options to rule between:
   assert against the committed checkpoint directory (`git diff --exit-code
   crates/forecast/checkpoints/anchors/`) as the condition, or move the path to the front-matter —
   which is itself a body change and therefore a D6.b re-emission. A hashed body should carry what the
   run measured, not where it looked. `sharpe-comparison-realdata` is `#118` and is the operator's,
   not this story's.

8. **The held rows stay held.** No gate is written for
   `threshold-sweep-bs{1,2}-realdata-recalibrated` until `#129` is ruled. If the ruling is "fix
   `run_cell`", those two rows' gates belong to that fix's story, not to this one; if it is "narrow
   1-25's AC2", the silent-drop behaviour change must be disclosed in the two anchors' comments before
   any gate defends their bodies.

9. **Standing floor.** `ANCHORS PASS (119 / 119)` before AND after (AD-2) — and note the gate now
   prints a *measured* pass count with a declared floor and cross-check (`#127`), so a row that stops
   being parsed fails loudly. FROZEN gate files byte-untouched (AD-1). `spec-lint` PASS. Verification
   scoped to the touched crates — never a workspace-wide `clippy --all-targets`.

## Tasks / Subtasks

- [x] **AC1 DONE 2026-09-29/30.** `#128a`: the `--out-dir` default no longer points into the corpus
      (landed 2026-09-27). `#128b`: `parse_gate_survivors` no longer returns nine silent zeros —
      and enumerating its failure paths while fixing it found a THIRD nobody had listed: besides an
      unreadable file and a missing section heading, a **partial parse** left the tail zero-filled.
      The probe prints the defect in one line: `[69085, 60339, 51964, 44375, 0, 0, 0, 0, 0]` — four
      measured values and five fabricated ones, identical once rendered. All three are errors carrying
      their own diagnosis, pinned by four unit tests including a non-vacuity test that the happy path
      still parses. Body-neutral, measured not assumed: a full bs1 run re-emitted `924a51bb…`,
      byte-identical to the anchored digest.
- [ ] *(superseded)* AC1 first, alone, and committed before anything is run: the out-dir default, the required-flag
      decision, and the silent-zeros parser. Nothing else may execute while a no-argument run of
      `threshold_sweep` can destroy an anchored body.
- [x] **AC5 DECIDED + half DONE 2026-09-30** (bug-log `#128c`). Decision, recorded not escalated:
      **diagnose where the report can name the condition, refuse where it cannot.**
      - `sharpe_comparison` already bails on a refused child — unchanged.
      - `regime_verdict` had ONE line covering two cases. Child FAILED: harmless, V-REG-1 fires and its
        evidence names the real condition (more honest than the audit's summary — read the code, not
        the verdict name). Child SUCCEEDED and wrote nothing: it computed a verdict from fabricated
        zeros on a run that reported success. That now bails with the reason.
      - `vol_verdict`: no change; it takes no such substitution.
      Deferred with its reason: V-REG-1 is still NAMED for EM convergence while it tests *"the backtest
      completed"* — the table row already labels itself honestly, so the gap is the headline. Renaming
      is body-affecting and rides this story's own re-emission of `regime-verdict-bs1-realdata`.
- [ ] AC2/AC3/AC4: the shared runner, then the 12 gates on top of it.
- [ ] AC6: run them. Triage each red to a cause at a `file:line`; batch the D6.b re-emissions rather
      than re-locking one row at a time.
- [ ] AC7: architect + operator ruling on the two blocked rows.
- [ ] Review: every gate probed by deliberate breakage (and the probe itself verified to have changed
      something — two "probes" in the 1-27 pass were no-ops reporting OK on unmodified files).

## Dev Notes

- The apparatus to extend, not re-implement: `crates/backtest/tests/determinism.rs`
  (`run_realdata_scenario_try`, `assert_reproduces_with_flags`, `ensure_realdata_release_binary`, the
  `// anchor-ns:` declarations) and `crates/backtest/tests/theta_surface_reproduction.rs` (the
  manifest-driven shape plus its always-running companion test that fails when the manifest goes
  stale). I re-implemented an existing measurement harness twice during the re-lock and each copy
  silently answered a different question; that is the failure this note exists to prevent.
- `#128d`: the `candle` feature does not control whether candle is linked — `backtest/Cargo.toml:63`
  enables `strategy/forecast` unconditionally, so `:38`'s `candle = ["strategy/forecast"]` only flips
  `#[cfg]` blocks. Do not reason about what is in the build from the flag.
- One unresolved caveat inherited from the derivation, explicitly NOT closed: `vol_verdict` compiles
  both with and without `candle` and takes different code paths, and a code-reading argument says the
  numbers are identical because only the container differs. That is a reading, not a measurement. The
  gate must assert it rather than assume it.
- Do-not-build register: not implicated — gate coverage, no new alpha surface.
- After this story the corpus splits cleanly into "has a re-run gate" and "has a written disposition
  for why it cannot". That, not 119/119, is the honest statement of what the anchor system proves.

### References

- Trace: `REQ-FORECAST-BIN-REPRODUCTION-GATES-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
- Recipes: `docs/dev-notes/forecast-bin-repro-recipes-2026-09-27.md`
- Coverage audit: `docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md`
- Disclosures: `docs/dev-notes/bug-log.md` § `#129` (the held lane — read this before touching the two threshold-sweep rows), § `#128` (this family's five defects), § `#127` (the corpus gate's own pass count), § `#118` (the producer-less row)
