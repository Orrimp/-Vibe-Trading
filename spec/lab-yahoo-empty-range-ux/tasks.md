---
slug: lab-yahoo-empty-range-ux
status: arch-done
owner: developer
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

## M-T1 — Architect (design pass) — DONE

- [x] **M-T1.1** Q1 resolved → **(a) split**. Typed signal lives in the
      **data crate** (`YahooError::NoDataForRange { ticker, start_label,
      end_label }`, emitted only on HTTP-200 + 0 quotes in
      `fetch_and_cache`); the typed transport decode lives in the **ui
      crate** (`runner::preload_notice::classify`). Both layers needed:
      the mock path bypasses the data crate, so the runner chokepoint
      classifies zero-bar success at the `preload_result` match.
      See feature.md § D-ER-1.
- [x] **M-T1.2** Q2 resolved → **(a) clamp**. `end_ms.min(now_ms)` in
      `range_to_ms_pair` (`runner.rs:323`), proven no-op for
      `H1_2024`/`H2_2024` (K3). See § D-ER-2.
- [x] **M-T1.3** Q3 resolved → **(a) distinct notice**. New
      `last_run_notice: Option<SmolStr>` field (NOT a severity flag on
      `last_run_error`), rendered muted `FG_2` sibling branch at
      `screens/lab.rs`. Existing red `⚠` branch byte-identical. See
      § D-ER-3.
- [x] **M-T1.4** ADR decision → **ADR-0040 § Changelog amendment, NO new
      ADR**. `NoDataForRange` is additive to D5; clamp refines D6/D7.
      ADR-0050 (rt.spawn) UNTOUCHED — classification is on the preload
      *result*, not the spawn glue. README registry/frontmatter bumped
      atomically with the amendment. See § D-ER-5.
- [x] **M-T1.5** Test harness confirmed → reuse `LabYahooBarSource` +
      `MockLabYahooBarSource` (ADR-0048, `runner.rs:222`). Add a sibling
      `transport_err()` mock variant. No new infra. See § D-ER-4.
- [x] **M-T1.6** `## Design` (D-ER-1..6 + P-ER-1) authored in feature.md;
      ADR-0040 + README updated. `arch` column entries cited in HANDOFF
      (orchestrator flips trace.toml). Task split is small → inline below,
      no separate `decomp.md`.
- [x] **M-T1.7** HANDOFF → developer (‖ ui-designer, Q3=(a) → M-DEV.9).

## M-DEV — Developer (‖ ui-designer)

> All rows reference feature.md § D-ER-N (the design lock) + exact file:line
> seams. Suggested commit order: data-crate variant → runner classifier +
> clamp → state/render wiring → tests. Build between each.

### Data crate (`crates/data`)

- [ ] **M-DEV.1** (D-ER-1 step 1, R1/R-NR.5) Add `YahooError::NoDataForRange
      { ticker, start_label, end_label }` to the enum in
      `crates/data/src/yahoo.rs` (after `MissingData`, ~line 167). Display:
      `"no Yahoo data for {ticker} in {start_label}..{end_label}"`. Purely
      additive — do NOT alter any existing variant's `#[error(...)]` string.
- [ ] **M-DEV.2** (D-ER-1 step 1) In `fetch_and_cache`
      (`yahoo.rs:364`, `#[cfg(feature = "yahoo-online")]`), after
      `let quotes = response.quotes()?;` (line 387), early-return
      `NoDataForRange` when `quotes.is_empty()` (before `quotes_to_bars`).
      K1-correct by construction — `classify_yfa_error` already mapped
      transport/429 before `.quotes()`.

### Runner (`crates/ui/src/lab/runner.rs`)

- [ ] **M-DEV.3** (D-ER-1 step 2, H2) Add `pub mod preload_notice` to
      `runner.rs` with `const NO_DATA_TAG: &str = "\u{1}NODATA\u{1}"`,
      `fn no_data_message(ticker, start_label, end_label) -> SmolStr`
      (formats `strings::LAB_YAHOO_NO_DATA_NOTICE`, prefixes the tag), and
      `enum RunMessageKind { Notice(SmolStr), Error(SmolStr) }` +
      `fn classify(raw: &str) -> RunMessageKind` (strip-tag → Notice, else
      Error). `LabRunResult` type stays UNCHANGED.
- [ ] **M-DEV.4** (D-ER-1 step 2, K1) In `fetch_with_backoff`
      (`runner.rs:462`), add a NON-retry early-out: match
      `Err(YahooError::NoDataForRange { .. })` in the `Ok(result)` arm
      (line 508) and return it immediately (do NOT consume the retry
      budget). Then in `preload_yahoo_bars` (`runner.rs:363`), in the
      `fetch_with_backoff` `Err(e)` arm (line 417): if `e` is
      `NoDataForRange` OR the post-fetch `load_cached` re-check yields
      `CacheMiss`/`MissingData{actual:0}` AFTER a successful fetch, build
      the tagged message via `preload_notice::no_data_message(...)` using
      the resolved window labels — replacing the generic "Check network"
      string for THAT case only.
- [ ] **M-DEV.5** (D-ER-1 step 2 — mock path) Factor a single helper
      `empty_bars_to_notice_or_pass(...)` and call it from BOTH
      `preload_result` match arms in `spawn_lab_run` (mock arm
      `runner.rs:812`, production arm `runner.rs:999`): when
      `Ok((bars, _sha))` has `bars.is_empty()`, return the tagged no-data
      `Err` instead of feeding an empty `bars_override` to the engine.
      (Caution #2 in § Developer cautions — both arms or they diverge.)
- [ ] **M-DEV.6** (D-ER-2, R4/K3) In `range_to_ms_pair` (`runner.rs:323`)
      apply `let end_ms = end_ms.min(now_ms);` on the returned pair.
      `now_ms` already computed at line 326. `start_ms` NEVER clamped.

### State + render (`crates/ui`)

- [ ] **M-DEV.7** (D-ER-3, R2/R-NR.4) Add `pub const
      LAB_YAHOO_NO_DATA_NOTICE` to `crates/ui/src/strings.rs` (template
      with `{ticker}` + `{window}`; NO variant name, NO "check network").
- [ ] **M-DEV.8** (D-ER-3 seam 1) Add `last_run_notice: Option<SmolStr>`
      to `crates/ui/src/lab/state.rs` (after `last_run_error`, line 205).
      Init `None` in the three constructors (`state.rs:292, 355, 395`) and
      in `LabState::clone` (Caution #3 — `None`, not serialized,
      schema stays `version: 1`).
- [ ] **M-DEV.9** (D-ER-3 seam 2) In `crates/ui/src/state.rs`:
      (a) `LabRunRequested` arm (line 2142) — clear `last_run_notice = None`
      beside `last_run_error`. (b) `LabRunCompleted` arm (lines 2151-2154)
      — replace flat `Err(msg) => Some(msg.clone())` with
      `preload_notice::classify(raw)` routing to `last_run_notice` (Notice)
      vs `last_run_error` (Error), each clearing the other.
- [ ] **M-DEV.10** (D-ER-3 seam 3, R-NR.2) In
      `crates/ui/src/screens/lab.rs`, add a sibling `last_run_notice`
      render branch immediately after the existing red branch (line 479):
      `ⓘ {notice}` at `color::FG_2` (muted), `text::SMALL`. Leave the
      `last_run_error` red `DOWN_500` branch (lines 474-479) BYTE-IDENTICAL.
      Confirm `last_run_ok` derivation (line 384) treats no-data as a clean
      terminal (notice ⇒ `error.is_none()` ⇒ Run button NOT `Failed`, R3).
- [ ] **M-DEV.11** (Caution #4, optional log-cleanliness) At
      `cockpit_live.rs:1086` activity-handle fail path, strip the tag for
      the log message via the same `classify().msg()`. Do not over-scope.

### Tests (`crates/ui/tests`, `crates/data`)

- [ ] **M-DEV.12** (D-ER-4 T1 — REQUIRED GATE / K2) New
      `crates/ui/tests/lab_yahoo_empty_range_classification.rs`
      (`--features live`). Case A: mock `Ok((vec![], sha))` → assert
      `last_run_notice.is_some()` + `last_run_error.is_none()` + notice
      contains no `CacheMiss`/`MissingData`/`Check network` + names window.
      Case B: new `MockLabYahooBarSource::transport_err()` (untagged
      `Err`) → assert `last_run_error.is_some()` + `last_run_notice
      .is_none()`. Different surfaces.
- [ ] **M-DEV.13** (D-ER-4 T2 — K1 at data boundary) Unit test in
      `crates/data` (`--features yahoo-online`): `classify_yfa_error`
      maps transport/429 → `Http`/`RateLimited`, NOT `NoDataForRange`.
- [ ] **M-DEV.14** (D-ER-4 T3 + T4 — H3 + K3) New
      `crates/ui/tests/lab_yahoo_range_clamp.rs` (`--features yahoo`):
      future `Custom` end clamped to `now`; `Last30d` end `<= now`;
      `H1_2024`/`H2_2024` return their exact literal pairs (byte-identical).
- [ ] **M-DEV.15** (D-ER-4 T5 — R3) Terminal-state test (reuse
      `lab_stop_button_gating.rs` pattern): no-data `LabRunCompleted(Err)`
      → `lab_run_inflight == false` + `run_progress.is_none()`.
- [ ] **M-DEV.16** (classify unit) Direct unit test of
      `preload_notice::classify`: tagged → `Notice(stripped)`; untagged →
      `Error(verbatim)`; empty → `Error`.
- [ ] **M-DEV.17** (P-ER-1 falsifier — for the tester to run) Document in
      the T1 test file how to run the misroute falsifier (temporarily drop
      `NO_DATA_TAG` from `no_data_message` → Case A must FAIL). Tester
      executes P-ER-1 in M-FINAL.
- [ ] **M-DEV.18** `rust-build` + `rust-validate` (clippy `-D warnings`,
      fmt). Build the `live`, `yahoo`, and `yahoo-online` feature sets.
- [ ] **M-DEV.19** Populate `crates` + `tests` columns of the trace row
      (orchestrator owns `arch`/state; developer may fill `crates`/`tests`
      per workflow).
- [ ] **M-DEV.20** (ui-designer, Q3=(a)) Confirm `FG_2` reads as
      "info, not alarm" against the run-button row. No new design token
      expected (R-NR.4) — `FG_2` is an existing Lumen neutral. If `FG_2`
      is insufficient, propose an existing info token; do NOT mint new.
- [ ] **M-DEV.21** HANDOFF → tester.

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
- 2026-05-30 (architect, M-T1): M-T1.1–M-T1.7 marked done (Q1=(a) split,
  Q2=(a) clamp, Q3=(a) notice field, ADR-0040 § Changelog amendment).
  M-DEV expanded to 21 developer-ready rows tied to feature.md § D-ER-1..6
  + exact file:line seams (data variant + emission, runner classifier +
  non-retry + clamp + dual mock-arm helper, state field + clone carve-out
  + classifier routing, muted-FG_2 render branch, 5 test files incl. the
  K2 required gate + P-ER-1 falsifier). Frontmatter `in-progress`→
  `arch-done`, owner `analyst`→`developer`. HANDOFF → developer ‖
  ui-designer (M-DEV.20).
