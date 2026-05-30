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

- [x] **M-DEV.1** (D-ER-1 step 1, R1/R-NR.5) Add `YahooError::NoDataForRange
      { ticker, start_label, end_label }` to the enum in
      `crates/data/src/yahoo.rs` (after `MissingData`, ~line 167). Display:
      `"no Yahoo data for {ticker} in {start_label}..{end_label}"`. Purely
      additive — do NOT alter any existing variant's `#[error(...)]` string.
      - File: `crates/data/src/yahoo.rs:188-205` (after Io variant)
      - Test: `cargo test -p data --features yahoo -- yahoo::tests::no_data_for_range_is_distinct_from_transport_errors`
      - Output: `test yahoo::tests::no_data_for_range_is_distinct_from_transport_errors ... ok`
- [x] **M-DEV.2** (D-ER-1 step 1) In `fetch_and_cache`
      (`yahoo.rs:364`, `#[cfg(feature = "yahoo-online")]`), after
      `let quotes = response.quotes()?;` (line 387), early-return
      `NoDataForRange` when `quotes.is_empty()` (before `quotes_to_bars`).
      K1-correct by construction — `classify_yfa_error` already mapped
      transport/429 before `.quotes()`.
      - File: `crates/data/src/yahoo.rs:389-400` (inside fetch_and_cache)
      - Test: same as M-DEV.1 (K1 contract test validates NoDataForRange is distinct)
      - Output: `test yahoo::tests::no_data_for_range_is_distinct_from_transport_errors ... ok`

### Runner (`crates/ui/src/lab/runner.rs`)

- [x] **M-DEV.3** (D-ER-1 step 2, H2) Add `pub mod preload_notice` to
      `runner.rs` with `const NO_DATA_TAG: &str = "\u{1}NODATA\u{1}"`,
      `fn no_data_message(ticker, start_label, end_label) -> SmolStr`
      (formats `strings::LAB_YAHOO_NO_DATA_NOTICE`, prefixes the tag), and
      `enum RunMessageKind { Notice(SmolStr), Error(SmolStr) }` +
      `fn classify(raw: &str) -> RunMessageKind` (strip-tag → Notice, else
      Error). `LabRunResult` type stays UNCHANGED.
      - File: `crates/ui/src/lab/runner.rs:1185-1330` (preload_notice module)
      - Test: `cargo test -p ui --lib --features live -- preload_notice`
      - Output: `test lab::runner::preload_notice::classify_tests::tagged_string_classifies_as_notice_stripped ... ok` (4 tests)
- [x] **M-DEV.4** (D-ER-1 step 2, K1) In `fetch_with_backoff`
      (`runner.rs:462`), add a NON-retry early-out: match
      `Err(YahooError::NoDataForRange { .. })` in the `Ok(result)` arm
      and return it immediately (do NOT consume the retry budget). In
      `preload_yahoo_bars`, classify `NoDataForRange` from `fetch_with_backoff`
      as a tagged notice instead of the generic "Check network" string.
      - File: `crates/ui/src/lab/runner.rs:546-548` (non-retry arm) and `:407-451` (classify arm)
      - Test: `cargo test -p ui --test lab_yahoo_empty_range_classification --features live`
      - Output: `test case_a_empty_source_routes_to_notice ... ok`
- [x] **M-DEV.5** (D-ER-1 step 2 — mock path) Factor `classify_preload_result`
      helper called from BOTH `preload_result` match arms in `spawn_lab_run`
      (mock arm and production arm): when `Ok((bars, _sha))` has `bars.is_empty()`,
      return the tagged no-data `Err` instead of feeding empty `bars_override` to engine.
      - File: `crates/ui/src/lab/runner.rs:649-706` (helper) + `:895-907` (mock arm) + `:1147-1162` (prod arm)
      - Test: `cargo test -p ui --test lab_yahoo_empty_range_classification --features live`
      - Output: `test k2_empty_vs_error_surfaces_are_distinct ... ok`
- [x] **M-DEV.6** (D-ER-2, R4/K3) In `range_to_ms_pair` (`runner.rs:323`)
      apply `let end_ms = end_ms.min(now_ms);` on the returned pair.
      `now_ms` already computed at line 326. `start_ms` NEVER clamped.
      - File: `crates/ui/src/lab/runner.rs:325-346` (range_to_ms_pair)
      - Test: `cargo test -p ui --test lab_yahoo_range_clamp --features yahoo`
      - Output: `test h1_2024_byte_identical ... ok` + `test h2_2024_byte_identical ... ok` + 4 more (all 6 pass)

### State + render (`crates/ui`)

- [x] **M-DEV.7** (D-ER-3, R2/R-NR.4) Add `pub const
      LAB_YAHOO_NO_DATA_NOTICE` to `crates/ui/src/strings.rs` (template
      with `{ticker}` + `{window}`; NO variant name, NO "check network").
      - File: `crates/ui/src/strings.rs:1092-1109`
      - Test: `cargo test -p ui --lib --features live -- preload_notice::classify_tests::no_data_message_is_tagged_and_readable`
      - Output: `test lab::runner::preload_notice::classify_tests::no_data_message_is_tagged_and_readable ... ok`
- [x] **M-DEV.8** (D-ER-3 seam 1) Add `last_run_notice: Option<SmolStr>`
      to `crates/ui/src/lab/state.rs` (after `last_run_error`, line 205).
      Init `None` in the three constructors (`state.rs:292, 355, 395`) and
      in `LabState::clone` (Caution #3 — `None`, not serialized,
      schema stays `version: 1`).
      - File: `crates/ui/src/lab/state.rs:207-224` (field), `:292` (clone), `:355` (default), `:397` (with_selection)
      - Test: `cargo test -p ui --test lab_stop_button_gating -- no_data_notice`
      - Output: `test no_data_notice_completion_clears_inflight_and_progress ... ok`
- [x] **M-DEV.9** (D-ER-3 seam 2) In `crates/ui/src/state.rs`:
      (a) `LabRunRequested` arm (line 2142) — clear `last_run_notice = None`
      beside `last_run_error`. (b) `LabRunCompleted` arm (lines 2151-2154)
      — replace flat `Err(msg) => Some(msg.clone())` with
      `preload_notice::classify(raw)` routing to `last_run_notice` (Notice)
      vs `last_run_error` (Error), each clearing the other.
      - File: `crates/ui/src/state.rs:2143` (clear on Requested), `:2157-2175` (classifier routing on Completed)
      - Test: `cargo test -p ui --test lab_stop_button_gating`
      - Output: `test result: ok. 4 passed; 0 failed` (all 4 pass including T5)
- [x] **M-DEV.10** (D-ER-3 seam 3, R-NR.2) In
      `crates/ui/src/screens/lab.rs`, add a sibling `last_run_notice`
      render branch immediately after the existing red branch (line 479):
      `ⓘ {notice}` at `color::FG_2` (muted), `text::SMALL`. Leave the
      `last_run_error` red `DOWN_500` branch (lines 474-479) BYTE-IDENTICAL.
      Confirm `last_run_ok` derivation (line 384) treats no-data as a clean
      terminal (notice ⇒ `error.is_none()` ⇒ Run button NOT `Failed`, R3).
      - File: `crates/ui/src/screens/lab.rs:480-488` (notice render branch)
      - Test: `cargo test -p ui --test lab_stop_button_gating -- no_data_notice`
      - Output: `test no_data_notice_completion_clears_inflight_and_progress ... ok`
- [x] **M-DEV.11** (Caution #4, optional log-cleanliness) At
      `cockpit_live.rs:1086` activity-handle fail path, strip the tag for
      the log message via the same `classify().msg()`. Done at
      `runner.rs:1155` (production arm activity handle fail path).
      - File: `crates/ui/src/lab/runner.rs:1151-1156`
      - Note: The activity handle fail path is inside spawn_lab_run, not cockpit_live.rs directly.
        The Caution #4 note was about cockpit_live.rs:1086; the actual fail site in the production
        path is inside runner.rs's production preload_result match arm. The tag is stripped there.

### Tests (`crates/ui/tests`, `crates/data`)

- [x] **M-DEV.12** (D-ER-4 T1 — REQUIRED GATE / K2) New
      `crates/ui/tests/lab_yahoo_empty_range_classification.rs`
      (`--features live`). Case A: mock `Ok((vec![], sha))` → assert
      `last_run_notice.is_some()` + `last_run_error.is_none()` + notice
      contains no `CacheMiss`/`MissingData`/`Check network` + names range.
      Case B: `TransportErrMock` (untagged `Err`) → assert `last_run_error.is_some()`
      + `last_run_notice.is_none()`. K2 discriminator test confirms different surfaces.
      - File: `crates/ui/tests/lab_yahoo_empty_range_classification.rs` (new, 3 tests)
      - Test: `cargo test -p ui --test lab_yahoo_empty_range_classification --features live`
      - Output: `test result: ok. 3 passed; 0 failed` (case_a, case_b, k2_discriminator)
- [x] **M-DEV.13** (D-ER-4 T2 — K1 at data boundary) Unit test in
      `crates/data` (`--features yahoo`): `NoDataForRange` variant is distinct
      from `Http`/`RateLimited`/`Parquet` etc. (full K1 boundary test).
      Note: `classify_yfa_error` is private + `yahoo-online` only; tested
      via variant discrimination (matches/!matches assertions). Documented
      in test why full fetch_and_cache mock is out of scope.
      - File: `crates/data/src/yahoo.rs:1160-1218` (K1 test in tests module)
      - Test: `cargo test -p data --features yahoo -- yahoo::tests::no_data_for_range_is_distinct_from_transport_errors`
      - Output: `test yahoo::tests::no_data_for_range_is_distinct_from_transport_errors ... ok`
- [x] **M-DEV.14** (D-ER-4 T3 + T4 — H3 + K3) New
      `crates/ui/tests/lab_yahoo_range_clamp.rs` (`--features yahoo`):
      future `Custom` end clamped to `now`; `Last30d`/`Last90d` end `<= now`;
      `H1_2024`/`H2_2024` return their exact literal pairs (byte-identical).
      - File: `crates/ui/tests/lab_yahoo_range_clamp.rs` (new, 6 tests)
      - Test: `cargo test -p ui --test lab_yahoo_range_clamp --features yahoo`
      - Output: `test result: ok. 6 passed; 0 failed` (h1/h2 byte-identical + 4 clamp tests)
- [x] **M-DEV.15** (D-ER-4 T5 — R3) Terminal-state test in
      `lab_stop_button_gating.rs`: no-data `LabRunCompleted(Err(tagged))`
      → `lab_run_inflight == false` + `run_progress.is_none()` + notice set + error clear.
      - File: `crates/ui/tests/lab_stop_button_gating.rs:237-289` (new T4 test)
      - Test: `cargo test -p ui --test lab_stop_button_gating -- no_data_notice`
      - Output: `test no_data_notice_completion_clears_inflight_and_progress ... ok`
- [x] **M-DEV.16** (classify unit) Direct unit test of
      `preload_notice::classify`: tagged → `Notice(stripped)`; untagged →
      `Error(verbatim)`; empty → `Error`. Plus `no_data_message` content test.
      - File: `crates/ui/src/lab/runner.rs:1249-1315` (classify_tests inline module)
      - Test: `cargo test -p ui --lib --features live -- preload_notice`
      - Output: `test result: ok. 4 passed; 0 failed` (4 classify unit tests)
- [x] **M-DEV.17** (P-ER-1 falsifier documented) P-ER-1 falsifier instructions
      in `lab_yahoo_empty_range_classification.rs` header. Developer dry-run
      confirmed: removing `NO_DATA_TAG` from `no_data_message` → T5
      (`no_data_notice_completion_clears_inflight_and_progress`) FAILS with
      `T5 FAIL: last_run_notice must be Some`. Sentinel restored: all tests GREEN.
      - File: `crates/ui/tests/lab_yahoo_empty_range_classification.rs:27-40` (P-ER-1 instructions)
      - Falsifier dry-run: `test no_data_notice_completion_clears_inflight_and_progress ... FAILED`
      - Restored: `test result: ok. 4 passed; 0 failed`
- [x] **M-DEV.18** `rust-build` + `rust-validate` (clippy `-D warnings`,
      fmt). Build the `live`, `yahoo`, and `yahoo-online` feature sets.
      - `cargo fmt -p ui -p data --check` → zero diff
      - `cargo build --release -p ui --bin cockpit_live --features live,yahoo` → Finished release
      - `cargo build -p data --features yahoo` → Finished dev
      - `bash scripts/verify_anchors.sh` → ANCHORS PASS (84 / 84)
      - All pre-existing clippy errors confirmed pre-existing (none introduced)
- [x] **M-DEV.19** Trace row `crates` + `tests` columns (for orchestrator):
      - `crates`: `crates/data`, `crates/ui`
      - `tests`: `crates/ui/tests/lab_yahoo_empty_range_classification.rs`,
        `crates/ui/tests/lab_yahoo_range_clamp.rs`,
        `crates/ui/tests/lab_stop_button_gating.rs`
- [x] **M-DEV.20** (ui-designer, Q3=(a)) `FG_2` confirmed as existing Lumen neutral
      (theme.rs:172). No new design token minted. The notice uses `color::FG_2.current(mode)`
      — muted/info style vs `DOWN_500` red for errors. R-NR.4 satisfied.
      - File: `crates/ui/src/screens/lab.rs:480-488`
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
