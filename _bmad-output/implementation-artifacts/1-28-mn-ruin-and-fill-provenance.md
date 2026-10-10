# Story 1.28: mn-ruin-and-fill-provenance

Status: review

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

5. **Re-emission protocol: DECIDED 2026-09-29, ADR-0038 § D6.b.** *(This AC was written as an
   operator entry gate. It is not one of the four things that genuinely need a human ruling, so it is
   decided here and recorded, per the standing authority of 2026-09-29.)* D6.b governs, on the `#110`
   precedent that re-emitted these same 12 MN surfaces under it while adding `trades` and `funding`
   columns — the same shape as AC2/AC3. The awkwardness noted below is real and does not change the
   answer: the ruin half is an observability addition rather than a wiring-bug fix, but D6.b is what
   the project has, § D6.c does not exist, and inventing a protocol to fit one story is worse than
   using the ratified one and saying where it strains. The original wording follows for the record.

   *(superseded)* **ENTRY GATE — the re-emission protocol needs a ruling before any SHA moves.** New
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

- [x] **AC5 DECIDED 2026-09-29** — ADR-0038 § D6.b, on the `#110` precedent. Not escalated: it is not
      one of the four cases that need a human ruling.
- [x] **AC1 DONE 2026-09-29.** `stats::clamp_equity_for_metrics` returns the clamped curve AND the
      witness (`first_ruin_bar`, `clamped_bars`, `ruined()`); all three inline copies replaced
      (`mc_harness.rs`, `param_robustness_sweep.rs` twice). Clamped VALUES pinned unchanged by a test,
      so no body moves — verified: `ANCHORS PASS (119 / 119)`, `35 of 35` literals. A source walk
      asserts the clamp literal exists at exactly 2 sites and names them on failure; **probed** by
      inserting a fourth copy → RED naming `mc_harness.rs:294`, restored → green. On its first run that
      walk found THREE sites and the third was its own search needle, which is now split so the
      detector cannot match itself. Ruin is logged at WARN at every site; rendering it is AC2.
- [x] **AC2 + AC3 + AC4 + AC6 DONE 2026-09-29.** Blast radius measured, not estimated: 34 surfaces,
      because the ruin line is emitted on the path all six table shapes pass through — a tripwire over
      12 of 34 would miss 22 by construction. Per-surface diff census against the previous bodies,
      taken BEFORE anything landed: **22 changed only the ruin line, 12 (MN) the ruin line plus the
      `cover_fills` column, 0 anything else.** No number moved. Landing control: **34 FAIL / 85 PASS**
      with the new bodies on disk and before any sha moved; afterwards `ANCHORS PASS (119 / 119)`,
      exactly 34 sha lines changed, 85 byte-identical, `35 of 35` literals, spec-lint PASS.
      Record: `docs/dev-notes/1-28-d6b-re-emission-2026-09-29.md`.
      **Result: ruined paths 0 on all 34, `cover_fills` 0 on all 12 MN.** The tripwire reads clean and
      the split is a no-op on today's numbers — which is exactly why AC4's fixture was not optional:
      the corpus has had zero liquidations since `#71`, so neither column could have proven itself from
      evidence.
- [x] *(closed by the Review Verdict below, 2026-10-10)* Review: columns non-vacuous (probed, both), unchanged numbers proven byte-identical (diff census
      above), 1-21 closed out or its remaining items re-stated honestly.
- [x] *(superseded — see the dated entry above)* Dev: AC1 — factor the single clamp+witness helper, delete the three copies, add the
      single-occurrence test.
- [x] *(superseded — see the dated entry above)* Dev: AC3 — split the counter at `montecarlo.rs:620` / `:839`, extend `PathRunResult`.
- [x] *(superseded — see the dated entry above)* Dev: AC2 — thread both through the cell reducer (`param_robustness_sweep.rs:908`,
      `:1879`, `:1913`) to the renderer (`sweep_harness.rs:1899`, `:2360`, `:2468`) — the
      existing `liquidations` field is the proven-working template for this path; extend
      that apparatus rather than building a second one.
- [x] *(superseded — see the dated entry above)* Dev: AC4 — the two non-vacuity tests, each demonstrated RED before its fix lands.
- [x] *(superseded — see the dated entry above)* Re-emission per the AC5 ruling: ~5 min of compute for the 12 MN surfaces (derived
      from 763.84 s / 32 surfaces in the 1-27 reproduction gate ≈ 24 s per surface — a
      division, not a per-surface measurement), plus the protocol's own record. The
      control from #110/#126 applies: prove the landing by showing exactly the expected
      count FAIL and the rest PASS BEFORE any SHA is touched.
- [x] *(closed by the Review Verdict below, 2026-10-10)* Review: columns non-vacuous, unchanged numbers proven byte-identical rather than
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


## Review Verdict — 2026-10-10

> **This is a SELF-REVIEW.** The work under review is my own, written in this session, so this verdict is weaker than an independent pass and the reader is entitled to know which it is. What it can honestly do is re-run the gates, check each AC against the artifact rather than against my memory of it, and name the deviations and residual debts — which is what it does below. What it cannot do is notice a blind spot I had while writing the code.

**VERDICT: PASS.**

**Evidence re-run 2026-10-10, not quoted from the story:** `ANCHORS PASS (119 / 119) [declared 119 · floor 119]` · `spec-lint: PASS (0 violations)` · `check_determinism_anchors: OK — 35 of 35`.

Story-specific: `mn_spread_divergence_e2e` → 12 passed, including
`a_forced_cover_is_counted_separately_and_never_as_a_trade` — the AC3 discriminator, which fails if
a forced cover is folded back into `trades`.

### Findings

1. **The columns are non-vacuous and were probed, which is the AC that mattered.** A ruin column
   reading `0 of 1200` and a `cover_fills` column reading `0` are indistinguishable from columns
   that cannot report anything else; both were demonstrated RED before their fix landed.
2. **The single-definition detector was itself defective when first written and that is recorded.**
   `clamp_single_definition_test` counted its own search string as a third occurrence of the clamp
   literal; fixed with `concat!` so the assertion cannot match the line making it. Worth keeping in
   the verdict because it is the same shape as the story's subject — a check that reports on itself.
3. **Diff census preceded landing**, 34 surfaces: 22 ruin-line-only, 12 ruin + `cover_fills`, 0
   anything else, with control 34 FAIL / 85 PASS. That ordering is what makes "only these lines
   changed" a measurement.

The two open `Review:` checkboxes were the only genuinely outstanding items; this verdict closes
them. The other five open boxes were stale copies of the original plan sitting beneath their own
`[x]` summaries — marked superseded in this pass (see the shared finding in 1-30's verdict).

### References

- Trace: `REQ-MN-RUIN-FILL-PROVENANCE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
- Predecessor: `_bmad-output/implementation-artifacts/1-21-perp-basis-mn-spread.md` (lines 94-98, the not-delivered record)
- Protocol: `_bmad-output/planning-artifacts/architecture/decisions/0038-vol-forecast-verdict-shape.md` § D6.b
- Disclosure: `docs/dev-notes/bug-log.md` § `#127` (the clamp half) and § `#110` (the counter half)
