# Story 1.29: forecast-bin-reproduction-gates

Status: review

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

   > **DISCHARGED and OVERRIDDEN 2026-10-01, recorded not escalated.** All three conditions this AC
   > named are met: the operator ruled "fix `run_cell`" on 2026-09-29, the lane was repaired, and the
   > behaviour change is disclosed in `anchors.toml` beside both rows — the re-emission comment states
   > in full which of the three states is correct and why it is neither of the two previously pinned.
   >
   > The clause assigning the gates to the repair's story is overridden, because **1-30 shipped
   > without them.** It re-emitted both bodies and built no re-run gate, so the two rows were gated
   > nowhere — while this story's own omission note said they were "gated elsewhere". That note was a
   > false provenance claim of exactly the species this story filed as `#134`, written into the file
   > that files it, and found an hour later by checking rather than by reading what I had written.
   >
   > Both rows are therefore gated **here**, through the same runner (`Gate` gained a `package` field
   > so the one `backtest` bin shares it rather than growing a second apparatus — `#112` req 0). The
   > family is now **12 gated + 3 omitted = 15**, and `coverage_is_complete` enforces the total.

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
- [x] **AC2/AC3/AC4 DONE 2026-10-01** — `crates/forecast/tests/anchored_report_reproduction.rs`.
      Ten gates plus a candle-invariance measurement, all through ONE runner
      (`assert_gate_reproduces` → `run_gate`), each declaring **binary × features × profile × CWD ×
      invocation** as *fields on a struct* rather than as a convention a reader reconstructs. `""`
      for features is a declaration and carries its own `features_means` — the companion test fails
      a gate that declares a feature set without saying why.
      - **The anchor is READ from `evidence/anchors.toml` at run time, never copied into the test.**
        There is no second place for it to drift from. `anchored_sha` additionally asserts the
        duplicate `(scenario, version)` rows **agree** (twelve of these carry two rows) and that the
        gate's declared namespace is one that exists — `#120`, where a globally-pinned namespace
        reported a reproducing scenario as drifted.
      - **AC3 witnesses, with what each proves.** The clearest is `sigma_train`: `10.954250` plain
        vs `0.018016` recalibrated is the single value `--metadata-path` moves, so a gate pointed at
        the wrong overlay fails as *"you ran the other variant"* rather than as 64 opaque hex. The
        `regime_verdict` witness is `#128c`'s: a non-zero bar count is what separates a measured run
        from a swallowed refusal that zero-fills every statistic.
      - **AC4 twice over** — structurally, because `--out-dir` is the runner's and the companion test
        fails any gate whose argv contains it or names `evidence/`; and *measured*, because every run
        compares `git status` over `evidence/` **and** the checkpoint dir across its own execution.
      - The hasher is `scripts/hash_report.py` — the repo's own, the one `verify_anchors.sh` uses —
        never a re-implementation. That is the Dev Note's warning taken literally.
      - The companion `coverage_is_complete` **always runs** and is what makes the `#[ignore]`s
        honest: `GATES ∪ OMITTED == 15`, no row in both, every omission carrying a real reason. It
        earned its place on first execution by failing on two omission reasons I had written as
        *"see the bs1 row"*.
- [x] **AC6 DONE 2026-10-01 — 14 of 14 gated rows GREEN, and three of them only after a re-emission.** One serial pass,
      `--test-threads=1`, `RAYON_NUM_THREADS=8`, release, bin sources `HEAD`-clean.
      **8 passed, 3 failed, 2953 s (49 min).**

      | row | verdict | vs the prediction |
      |---|---|---|
      | `forecast-distribution-bs1-realdata` | **GREEN** | unpredicted |
      | `forecast-distribution-bs2-realdata` | **GREEN** | unpredicted |
      | `forecast-distribution-bs1-realdata-recalibrated` | **GREEN** | unpredicted |
      | `forecast-distribution-bs2-realdata-recalibrated` | **GREEN** | unpredicted |
      | `forecast-distribution-patchtst-bs1-realdata` | **GREEN** | unpredicted |
      | `vol-verdict-bs1-realdata` | **GREEN** | unpredicted |
      | `regime-verdict-bs1-realdata` | **GREEN** | § 4.4 called it the likeliest of the four
        shell-out rows to reproduce — **confirmed** |
      | `sharpe-comparison-vol-target-bs1-realdata` | **RED** `d21db467…` → `29aac8e0…` | § 4.4
        **predicted drift** — confirmed |
      | `sharpe-comparison-vol-target-bs1-realbaseline` | **RED** `ff2b9349…` → `c01f4c66…` | § 4.4
        **predicted drift** — confirmed |
      | `sharpe-comparison-regime-dispatcher-bs1-realdata` | **RED** `a9e00139…` → `d07483bc…` | § 4.4
        called this one **uncertain**, not predicted — now measured |
      | *(not a row)* `vol_verdict_is_candle_invariant` | **GREEN** | see below |

      **Seven of seven non-derived rows reproduce. All three derived rows drifted.** That split is
      the finding, and it is not a coincidence: every red is a report whose numbers are *computed
      from other anchored bodies*, and all of those sub-scenario bodies were re-emitted under
      ADR-0038 § D6.b on 2026-09-26, after these rows were locked in May. A derived row inherits
      every re-emission upstream of it and nothing was tracking that.

      Causes, at `file:line`, dated BEFORE the measurement rather than fitted to it — the three
      superseded sub-scenarios are in `evidence/anchors.toml` with their old digests kept verbatim:
      `top10-2023-1h-momentum` (`0f6f6eb8…` superseded), `top10-2023-fy-momentum-realdata`
      (`0867d232…`), `top10-2023-fy-vol-target-overlay-realdata` (`9fa64d46…`).
      `top10-2023-fy-regime-dispatcher-realdata` is **not** a cause: it is GREEN at its own anchor
      (`determinism.rs:1603-1618`), so the regime-dispatcher row's drift comes entirely through its
      momentum baseline. All three reds are D6.b re-emission candidates, **never** re-baselines
      (`#77`).

      **The one caveat this story refused to close by reading is now closed by measuring.**
      `vol_verdict_is_candle_invariant` is GREEN: `vol_verdict` produces a byte-identical body with
      and without `--features candle`. The recipes doc could only argue it from
      `features.rs:811-824` and said so; Dev Notes said *"the gate must assert it rather than assume
      it"*. It does, and the reading was right — which is worth exactly as much as it being wrong
      would have been, because now either answer would have been visible.

      **AC4 held empirically, not just structurally**: `git status` over `evidence/` and the
      checkpoint directory was clean at every check across all 49 minutes and 11 binary invocations.

      **The three reds re-emitted under § D6.b the same day — two landed, one HELD.**
      `docs/dev-notes/1-29-d6b-re-emission-2026-10-01.md` is the record for the
      `recalibrate-sigma-train` pair; the sharpe family's block is in `anchors.toml` beside the rows.

      | row | new digest | verdict | landed? |
      |---|---|---|---|
      | `sharpe-comparison-vol-target-bs1-realbaseline` | `b82f3132…` | `T-VOL-NO-ALPHA` → unchanged | **yes** |
      | `sharpe-comparison-regime-dispatcher-bs1-realdata` | `4f5d35c8…` | `T-REG-NO-ALPHA` → unchanged | **yes** |
      | `sharpe-comparison-vol-target-bs1-realdata` | `d35073f8…` | **`T-VOL-NO-ALPHA` → `T-VOL-ALPHA-UNLOCKED`** | **yes, after the ruling** |

      The third row was a standing stop condition — re-emitting it changes what the corpus claims,
      and AD-19 forbids shipping a moved verdict without an explicit override — so it was escalated
      before landing and **held** while its two siblings went in. **Operator ruled 2026-10-01: re-emit
      AND make the label's limits unmissable inside the body**, the same ruling on the same shape as
      the 2026-09-29 threshold-sweep escalation. Landed as `d35073f8…` with a **computed** qualifier
      under the T-classifier row, pinned in both directions by a unit test and defended by one of
      that gate's own witnesses — so a body printing `T-VOL-ALPHA-UNLOCKED` without its limits cannot
      pass. Details in `#137`.

      The three readings the ruling needs, all in `#137`: both Sharpes are **negative** (−0.667765
      baseline, −0.407398 overlay — the overlay loses *less*, it does not make money); the flip
      belongs to the row with the **synthetic** baseline, and the identical overlay scores −0.407398
      in both rows, so the whole +0.26 is the baseline's 0.34 of extra badness; and the
      purpose-built real-baseline comparison of that same overlay is **unchanged at NO-ALPHA**
      (Δ = −0.079). The era-qualified thesis is not contradicted by the better of the two
      comparisons the repo deliberately keeps side by side.

      **Final state, with the provenance of each green spelled out — three passes, because the
      corpus moved underneath the first one:**

      | pass | when | gates | result |
      |---|---|---|---|
      | 1 | 13:17–14:06, 2953 s | the 10 rows that existed then + the candle-invariance measurement | 7 rows GREEN, 3 sharpe rows RED |
      | 2 | 15:09–15:33, 1427 s | `recalibrate-sigma-train-bs{1,2}` (new, unblocked by `#134`), `threshold-sweep-bs{1,2}` (new) | **4 GREEN** |
      | 3 | 20:54–21:05, 327 s | the 3 sharpe rows, after their re-emissions landed | **3 GREEN** |

      **14 of 14 gated rows have a green measurement.** The 7 from pass 1 stand: none of their
      anchors was touched by any re-emission in this story, and `body_is_anchor_dir_invariant`
      separately verified the committed checkpoint directory is byte-identical after four real runs
      — which is what the two `-recalibrated` rows depend on. The fifteenth row is `#118`'s
      producer-less one and has a written disposition, not a gate.

      Pass 2 also had a false start worth keeping: the two `threshold-sweep` gates first reported
      **UNMEASURED, not red** — the AC4 check saw `M evidence/anchors.toml` appear mid-run because
      the § D6.b re-lock was landing concurrently, and refused to certify a run it could no longer
      vouch for. The refusal was correct and its message was not, since it read as an accusation of
      the binary. Both causes are now named in it, and the operational order it implies — settle the
      corpus, commit, **then** run the gates — is in the file's module doc, because the constraint is
      real and not obvious.

- [x] **AC7 RULED 2026-10-01 — (b) fix the body, re-emit, then gate. Bug-log `#134`.**
      Reading the code to write the disposition found the reason the choice is not a judgement call.
      `recalibrate_sigma_train.rs:457` renders `"- Read-only against \`{overlay_path}\` original
      safetensors."` into the hashed body — and `overlay_path` (`:628`) is the
      `.metadata.recalibrated.json` the run **wrote** three lines earlier. The body's sole read-only
      claim names the only file the run created, and calls a JSON overlay *safetensors*. Both
      anchored bodies carry it verbatim.
      - **That kills option (a).** A gate whose procedure is *"run it, it rewrites X, assert X came
        back identical"* would be certifying a body that says X is read-only — AC8's own language
        about the threshold-sweep lane, in a second lane.
      - **And the fix removes the blocker.** `#128e` blocks these rows because `--anchor-dir` (the
        *write* target) reaches the hashed body; the *read* path does not (`:506` is a hardcoded
        relative constant). A sentence naming the safetensors it actually read is stable under
        `--anchor-dir <tempdir>`, so the gate sandboxes cleanly. One change closes a false claim and
        the blocker without trading them off.
      - Print the **relative constant**, not a resolved path: `resolve_anchors_dir` falls back to an
        absolute `CARGO_MANIFEST_DIR` path, which would make the body machine-dependent — `#132`'s
        shape rather than a cure for it.
      - **DONE 2026-10-01, all of it.** The one-line fix landed with its RED-proven tripwire
        (`report_is_handed_the_read_path_not_the_write_path`, measured RED against unmodified
        `HEAD`, green after). Both rows re-emitted under § D6.b — `1d1831c6…` / `4193d2be…`, four
        `anchors.toml` rows re-locked, `ANCHORS PASS (119/119)` after. The census says what the
        re-emission did: **2 lines of 51 changed per body, zero numbers moved.**
      - The negative invariant is the measurement that unblocks them: `body_is_anchor_dir_invariant`
        ran each scenario twice into two **different** anchor-dirs and got equal digests (2312 s),
        and compared the committed checkpoint directory byte for byte across all four real runs —
        the read-only assertion `#135` had nowhere to put.
      - Both rows are now **gated**, with the runner owning `--anchor-dir` exactly as it owns
        `--out-dir`. The family is **14 gated of 15**; the one that remains is `#118`'s
        producer-less row, and `coverage_is_complete` enforces the total.
- [x] **Review — probes DONE 2026-10-01, 6/6 RED with their intended messages.** Each mutates the
      gate table, asserts the mutation actually landed (SHA-256 before vs after — a probe that
      changed nothing proves nothing), runs the companion, greps for the specific message, and
      restores. Harness: `scratchpad/probe-gates.sh`.
      - P1 drop an `OMITTED` row → family accounting fires · P2 declare a namespace that does not
        exist → `#120`'s message · P3 a gate owning `--out-dir` → AC4 · P4 argv naming `evidence/` →
        AC4 · P5 one scenario both gated and omitted → "listed twice" · P6 an omission with no
        reason → AC7's.
      - **The probe harness itself had the defect it exists to catch.** Its restore was
        `git checkout -- <file>`, which restores *HEAD* — and on the first real use it silently threw
        away the uncommitted `#135` fix in the file it was probing. Caught only because the
        no-op guard printed a before/after SHA that did not match. It now restores from a snapshot.
        Recorded in the bug-log: the guard that caught it was looking the other way.
      - The `#134`/`#135` tests were probed the same way: a deliberately absent fifth sentinel made
        the read-only guard RED with *"UNMEASURED, not a pass"*, which is what makes its green mean
        the sentinels were actually read.

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
