---
slug: lab-yahoo-empty-range-ux
status: in-progress
owner: analyst
updated: 2026-05-30
version: 0.1.0
---

# Tasks — lab-yahoo-empty-range-ux v0.1.0

Workflow: **analyst → architect → (developer ‖ ui-designer) → tester → presenter → operator**.
Small feature (~1–2 dev-days). The developer/ui-designer split is small —
ui-designer's surface is one notice token + one string (Q3=(a)); it can
fold into developer if Q3=(b) is chosen.

Trace row: `REQ-LAB-YAHOO-EMPTY-RANGE-UX-001` (state `proposed`).

---

## M0 — Analyst (DONE)

- [x] **M0.1** Read the empty-Yahoo-response path READ-ONLY
      (`runner.rs` `preload_yahoo_bars`/`range_to_ms_pair`/`fetch_with_backoff`;
      `data/src/yahoo.rs` `YahooError`/`load_cached`/`fetch_and_cache`;
      `screens/lab.rs` error render; Bug #64 code-map dev-note).
- [x] **M0.2** Author `feature.md` — R1–R4 + R-NR, K1–K4, H1–H3, Q1–Q3
      (durable-biased), 4-cell verdict tree.
- [x] **M0.3** Author `tasks.md` (this file).
- [x] **M0.4** Open trace row `REQ-LAB-YAHOO-EMPTY-RANGE-UX-001`
      (state `proposed`).
- [x] **M0.5** Promote to backlog Active (small, operator-motivated).
- [x] **M0.6** HANDOFF → architect (envelope + prose line).

## M-T1 — Architect (design pass)

- [ ] **M-T1.1** Resolve Q1 (classification mechanism). If Q1=(a),
      decide WHERE the new signal lives: a new
      `YahooError::NoDataForRange` variant in `data` crate, OR a
      `LoadedBars{loaded_count:0}` success that `preload_yahoo_bars`
      classifies. Lock the boundary (data-crate vs ui-crate).
- [ ] **M-T1.2** Resolve Q2 (preset guard). If Q2=(a), specify the clamp
      contract in `range_to_ms_pair` (clamp ONLY when `end_ms > now_ms`;
      past ranges byte-identical per K3). If Q2=(b)/(c), note the deferral.
- [ ] **M-T1.3** Resolve Q3 (message surface). If Q3=(a), specify the
      `last_run_notice` field vs the `last_run_error`+flag shape, and the
      notice style token (hand to ui-designer). If Q3=(b), specify the
      corrected `last_run_error` copy only.
- [ ] **M-T1.4** Confirm whether an ADR amendment is needed. Likely a
      one-line ADR-0040 (Yahoo cache) Changelog amendment for the
      classification, NOT a new ADR. Confirm no ADR-0050 (rt.spawn) touch
      — this feature does not alter the spawn path.
- [ ] **M-T1.5** Confirm the test harness: reuse `LabYahooBarSource` +
      `MockLabYahooBarSource` (ADR-0048) for the empty-source classification
      test (K2). No new test infra.
- [ ] **M-T1.6** Write `architecture.md` / ADR delta + populate the
      `arch` column of the trace row. Author `decomp.md` if the task split
      warrants it (likely inline in tasks.md given the small size).
- [ ] **M-T1.7** HANDOFF → developer (‖ ui-designer if Q3=(a)).

## M-DEV — Developer (‖ ui-designer)

- [ ] **M-DEV.1** Implement R1 — classification at the preload boundary
      per the M-T1 Q1 decision. Empty-but-successful fetch → no-data
      outcome; transport/parse/429 → existing error.
- [ ] **M-DEV.2** Implement R2 — render the no-data message naming the
      ticker + resolved window (post-`range_to_ms_pair`), with NO internal
      variant name and NO "check network" hint. Add the new string
      constant in `strings.rs` (R-NR.4).
- [ ] **M-DEV.3** Implement R3 — assert the empty path resolves to a
      terminal state (no spinner hang). Reuse existing termination.
- [ ] **M-DEV.4** Implement R4 per Q2 — clamp/warn/none.
- [ ] **M-DEV.5** Implement Q3 surface (notice field+style OR corrected
      error copy).
- [ ] **M-DEV.6** Tests:
      - **K2 classification test** — mock empty source → no-data notice;
        mock transport error → red error; assert different surfaces.
      - **H3 future-dating test** — `Last30d`/`Last90d` under a pinned
        future clock flagged future-dated (and clamped if Q2=(a)).
      - **K3 clamp non-regression test** (Q2=(a) only) — a non-future
        range byte-identical pre/post.
      - **R-NR.2 error-path test** — genuine error still red `⚠`.
- [ ] **M-DEV.7** `rust-build` + `rust-validate` (clippy -D warnings, fmt).
- [ ] **M-DEV.8** Populate `crates` + `tests` columns of the trace row.
- [ ] **M-DEV.9** (ui-designer, Q3=(a) only) Confirm the notice token in
      the Lumen design system; no new design token if an existing
      muted/info color suffices (R-NR.4).
- [ ] **M-DEV.10** HANDOFF → tester.

## M-FINAL — Tester

- [ ] **M-FINAL.1** Run the full suite + the new K2/H3/K3/R-NR.2 tests.
- [ ] **M-FINAL.2** Verify anchors byte-identical (R-NR.1 — synthetic
      path untouched; expect ZERO anchor delta).
- [ ] **M-FINAL.3** Walk the 4-cell verdict tree; emit VERDICT.
- [ ] **M-FINAL.4** Operator visual-verify recipe (self-contained:
      Command / Steps / Timing / Expected-result / Failure-diagnosis /
      Cleanup) — pick `Last 30d` on a Yahoo source under the 2026 clock,
      click Run, confirm the no-data notice (not a red error / not a hang).
- [ ] **M-FINAL.5** Write `reports/test-final-<date>-lab-yahoo-empty-range-ux-v0.1.0.md`.
      Populate `anchors` column of the trace row. Flip state → `verified`.
- [ ] **M-FINAL.6** HANDOFF → presenter (only on PASS).

## M-PRESENT — Presenter

- [ ] **M-PRESENT.1** Assemble
      `presentations/lab-yahoo-empty-range-ux-<date>.md`. Frame as the
      discharge of the Bug #64 attempt-3 deck FYI #2 carry-forward.
- [ ] **M-PRESENT.2** Include a before/after of the operator message
      (generic red error → clear no-data notice) and the preset-guard
      behaviour (Q2 outcome).
- [ ] **M-PRESENT.3** Operator approval → flip state → `shipped`.

---

## Sequencing / parallelism notes

- **No conflict** with the 3 in-flight Pick C architects or the
  lab-recipe Wave C dev — disjoint file scopes (Pick C is Python scripts
  under `scripts/`; this is `crates/data` + `crates/ui` Yahoo/Lab paths).
- The lab-recipe Wave C dev touches `runner.rs` test harness; coordinate
  the merge order if both land near-simultaneously, but the production
  empty-classification path is a distinct code region from the recipe
  harness.
- Developer ‖ ui-designer parallel only if Q3=(a); otherwise single
  developer lane.

## Changelog

- 2026-05-30 (analyst): M0 task list authored. M0.1–M0.6 marked done.
  M-T1 architect pass next (resolve Q1/Q2/Q3 + ADR-0040 amendment
  decision). HANDOFF → architect.
