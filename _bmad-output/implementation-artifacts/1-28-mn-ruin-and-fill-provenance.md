# Story 1.28: mn-ruin-and-fill-provenance

Status: ready-for-dev

<!-- Created 2026-09-27 by the orchestrator, from the two items story 1-21 records as
     explicitly NOT delivered (1-21 lines 94-98, bug-log #110). 1-21 shipped the honest
     interim — the body now STATES the conflation instead of hiding it — and parked the
     fix because both halves need a counter threaded through the path loop, not a
     renderer line. This story owns that threading. Same split pattern the operator
     used for 1-25 -> 1-26 (regeneration) and 1-26 -> 1-27 (bisect). -->

## Story

As the operator of the Honest Advisor,
I want a wiped-out Monte-Carlo path to be distinguishable from a merely-bad one, and a synthetic liquidation cover to be distinguishable from a real fill,
so that the two most dramatic numbers the theta-surface reports print — `p95_maxdd` and `trades` — can be read for what they actually measure instead of for two causes each.

## Context measured before writing this story (2026-09-27)

Three facts, each verified at the code or the corpus, not inferred:

1. **The ruin witness is computed and thrown away, three times.** Every equity value
   `e <= 0` is mapped to `dec!(0.000001)` before the metric calls at
   `crates/backtest/src/bin/param_robustness_sweep.rs:911-917`, again at
   `crates/backtest/src/bin/param_robustness_sweep.rs:1974-1980`, and again at
   `crates/backtest/src/mc_harness.rs:294-303`. The clamp itself is deliberate and
   correct — it exists so `compute_sharpe_hourly` cannot return NaN (ADR-0051 D2), and
   removing it is NOT in scope. The defect is that the predicate `e <= Decimal::ZERO`
   is already evaluated at all three sites and its answer is never recorded. After the
   clamp, a path that lost everything and a path that lost 99.9% are the same number.
   Three copies of one clamp is also precisely how a project ends up with three places
   to forget the same counter.

2. **`trades` counts two different events.** `crates/backtest/src/scenarios/montecarlo.rs:620`
   increments it for a real fill; `:839` increments the same counter for each synthetic
   maintenance-margin buy-to-cover leg inside the M-DEV-3 liquidation block. The report
   legend at `crates/backtest/src/sweep_harness.rs:2345-2348` already tells the reader
   this conflation exists (1-21's honest interim) — so the body currently documents a
   defect instead of not having one.

3. **Ruin is ABSENT from the live corpus — so this is a tripwire, not a correction.**
   Measured across `evidence/**/reports/*theta-surface*.md`: the live (2026-09-25,
   post-re-lock) bodies contain **zero** cells with `p95_maxdd >= 99.9%`. Cells at
   exactly `100.00%` exist only in two **superseded** 2026-06-08 mn-basisperp bodies —
   which is exactly where the clamp was hiding ruin, and is the retro-validation of the
   concern. Consequence for this story: the new columns will render `0` everywhere in
   today's corpus. That LOWERS the urgency and RAISES the value — nothing in the current
   evidence is misread today, and the guard is in place before the next surface that
   would be.

Note on what "re-emitted in place" meant in 1-26/1-21: the re-lock wrote **new filenames
with new timestamps into the same directory** (`robustness-sweep-20260925-*` beside
`robustness-sweep-20260608-*`), not over the old filenames. `scripts/verify_anchors.sh`
resolves each anchor to the NEWEST match, which is what makes the 2026-09-25 bodies the
live ones and the 2026-06-08 bodies history-on-disk. Any re-emission this story performs
follows that same shape.

## Acceptance Criteria

1. **One clamp, one witness.** The three duplicated clamp sites are replaced by a single
   shared helper that returns BOTH the clamped curve AND the ruin witness (whether any
   bar was `<= 0`, and the index of the first such bar). A test asserts the clamp literal
   `dec!(0.000001)` occurs exactly ONCE in `crates/backtest/src/` — so a fourth copy
   cannot be added without going red. The clamped VALUES are unchanged: every existing
   Sharpe / Sortino / Calmar / maxdd number stays bit-identical, and that is proven by
   re-running a surface, not asserted.

2. **Ruin count surfaced.** The per-path ruin witness reaches the cell reducer and the
   theta-surface renderer as a `ruined_paths` count (out of N). The column legend states
   plainly that `p95_maxdd` at or near 100% WITHOUT a ruin count is ambiguous, and that
   the clamp is the reason.

3. **Fill provenance separated.** `run_path`'s single `trades` counter is split: real
   fills (`montecarlo.rs:620`) stay in `trades`; synthetic maintenance-margin cover legs
   (`montecarlo.rs:839`) go to a new `liquidation_cover_fills` field on `PathRunResult`.
   Both are surfaced per cell. The `sweep_harness.rs:2345-2348` legend caveat is REPLACED
   by the derived numbers — a report that can state the split must no longer apologise
   for not being able to.

4. **Non-vacuity, proven.** Two tests that would be impossible to pass by accident: one
   constructs a path that reaches `equity <= 0` and asserts `ruined_paths >= 1` (RED
   before AC1/AC2 land); one constructs a maintenance-margin liquidation and asserts
   `liquidation_cover_fills >= 1` while `trades` EXCLUDES those legs (RED before AC3).
   A column that cannot render non-zero is indistinguishable from a column that is always
   correct — this project's governing rule, and the reason AC4 is an AC and not a task.

5. **ENTRY GATE — the re-emission protocol needs a ruling before any SHA moves.** New
   columns change the rendered body, so the affected anchored bodies must be re-emitted.
   Which protocol governs is NOT dev's call and is NOT settled: ADR-0038 § D6.b is the
   **wiring-bug-fix** re-emission protocol, and it fits AC3 cleanly (two causes summed
   into one counter is a wiring bug, and the #110 precedent re-emitted these same 12 MN
   surfaces under it) but sits awkwardly on AC2, where the clamp is deliberate, documented
   and staying. Architect + operator rule between: (a) D6.b for the whole story on the
   AC3 bug, with AC2 riding along as the #110 precedent had trades/funding ride along;
   (b) a codified § D6.c observability-additive variant; (c) split the story so only AC3
   re-emits. Blast radius to be confirmed by measurement before the ruling, not assumed:
   if the columns are MN-table-only, the expectation is 12 rows change and 107 stay
   byte-identical.

6. **Standing floor.** `bash scripts/verify_anchors.sh` prints `ANCHORS PASS (119 / 119)`
   before AND after (AD-2). The FROZEN gate files stay byte-untouched (AD-1) — neither
   clamp site is a FROZEN file, and the identity test discharges it. `scripts/spec_lint.py`
   PASS. Verdicts are unchanged **by construction** and the story says why: the frozen
   bands read five signals, and an observability column is not one of them — so any
   verdict movement in the re-emitted bodies would be a defect in this story, not a result.

## Tasks / Subtasks

- [ ] Architect: rule AC5 (which re-emission protocol) and confirm the blast radius by
      measuring which report tables gain columns. Nothing else starts before this.
- [ ] Dev: AC1 — factor the single clamp+witness helper, delete the three copies, add the
      single-occurrence test.
- [ ] Dev: AC3 — split the counter at `montecarlo.rs:620` / `:839`, extend `PathRunResult`.
- [ ] Dev: AC2 — thread both through the cell reducer (`param_robustness_sweep.rs:908`,
      `:1879`, `:1913`) to the renderer (`sweep_harness.rs:1899`, `:2360`, `:2468`) — the
      existing `liquidations` field is the proven-working template for this path; extend
      that apparatus rather than building a second one.
- [ ] Dev: AC4 — the two non-vacuity tests, each demonstrated RED before its fix lands.
- [ ] Re-emission per the AC5 ruling: ~5 min of compute for the 12 MN surfaces (derived
      from 763.84 s / 32 surfaces in the 1-27 reproduction gate ≈ 24 s per surface — a
      division, not a per-surface measurement), plus the protocol's own record. The
      control from #110/#126 applies: prove the landing by showing exactly the expected
      count FAIL and the rest PASS BEFORE any SHA is touched.
- [ ] Review: columns non-vacuous, unchanged numbers proven byte-identical rather than
      asserted, 1-21 closed out or its remaining items re-stated honestly.

## Dev Notes

- This story exists because 1-21 could not close on it. 1-21 stays at `review` until AC5
  is ruled; whether 1-21 flips to `done` with a handoff to this story (the 1-25 -> 1-26
  pattern) is the operator's call, not this story's.
- `PathRunResult.liquidations` (u64) already exists and is fully wired:
  `param_robustness_sweep.rs:908` -> `:1879` (sum) -> `:1913` -> `sweep_harness.rs:2468`
  (rendered). Verified with `scripts/callers.sh`. This is the seam to copy — the risk of a
  fresh parallel path is that it silently answers a different question.
- The `liquidations` column measured 2210 events in the old corpus and 0 after the #71 fix
  removed the absorbing state, so the column is demonstrably capable of rendering non-zero.
  The new columns must earn the same demonstration (AC4) rather than inherit its credibility.
- Do-not-build register: not implicated — gate/observability maintenance, no new alpha surface.
- Direction honesty: the expectation is that the ruin column reads 0 everywhere on today's
  corpus. That expectation is a measurement (see Context 3), and if a re-emitted body
  renders a non-zero ruin count, the thesis-adjacent reading of that surface changes and
  AC5's escalation clause applies (AD-19 spirit).

### References

- Trace: `REQ-MN-RUIN-FILL-PROVENANCE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
- Predecessor: `_bmad-output/implementation-artifacts/1-21-perp-basis-mn-spread.md` (lines 94-98, the not-delivered record)
- Protocol: `_bmad-output/planning-artifacts/architecture/decisions/0038-*.md` § D6.b
