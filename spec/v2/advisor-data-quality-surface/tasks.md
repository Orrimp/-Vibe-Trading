---
slug: advisor-data-quality-surface
status: dev-done
owner: developer
updated: 2026-07-06
---

# Tasks — P1-7 DATA-Stage Trust/Quality Surface

## Completed by developer (2026-07-06)

- [x] T1 — Add `DataQualityView` / `VenueTrust` / `DataQualityWarning` types +
      `DataQualityView::for_symbol` constructor.
  - file: `crates/ui/src/leaderboard/state.rs:143` (new `DataQualityView`
    struct, `VenueTrust` enum, `DataQualityWarning` enum, `for_symbol` impl —
    inserted before the P1-2 `TailSummaryView` section)
  - test: `cargo test -p ui --lib leaderboard::state`
  - output: `test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 554 filtered out; finished in 0.00s`
    (includes `data_quality_for_symbol_btcusdt_is_high_reconcilable_no_warnings`,
    `data_quality_for_symbol_is_stable_across_the_universe`,
    `venue_trust_badge_labels_are_distinct_and_non_empty`,
    `data_quality_warning_copy_is_plain_language_and_distinct`)

- [x] T2 — Wire `BakeoffReportMirror.data_quality: DataQualityView` (always
      populated, not `Option` — every bake-off runs on a known symbol) +
      populate in `from_report` from `report.request.symbol`.
  - file: `crates/ui/src/leaderboard/state.rs` (`BakeoffReportMirror` struct
    field + `from_report` populating line, immediately before the `coin:`
    field so both are seeded from the same `report.request.symbol`)
  - test: `cargo test -p ui --lib leaderboard::state::tests::bakeoff_report_mirror_from_report_populates_data_quality`
  - output: `test leaderboard::state::tests::bakeoff_report_mirror_from_report_populates_data_quality ... ok`
    (part of the 34/34 run cited under T1)

- [x] T3 — Update all 7 `BakeoffReportMirror { .. }` struct-literal
      construction sites (6 in `fixtures.rs`, 1 in `state.rs` test module)
      with `data_quality: DataQualityView::for_symbol("BTCUSDT")`.
  - file: `crates/ui/src/fixtures.rs` (6 sites: `fake_bakeoff_report_mirror`
    line ~1608, `fake_bakeoff_report_mirror_with_shorts` line ~1799,
    `fake_bakeoff_report_mirror_five_arm` line ~2052,
    `fake_bakeoff_report_mirror_benchmark_wins` line ~2119,
    `fake_bakeoff_report_mirror_benchmark_wins_full` line ~2284,
    `fake_bakeoff_report_mirror_with_ensembles` line ~2454)
  - file: `crates/ui/src/leaderboard/state.rs` (`ready_mirror()` test helper)
  - note: `fake_bakeoff_report_mirror_with_signal_library` needed NO change —
    it builds from `base = fake_bakeoff_report_mirror()` and mutates fields,
    so `data_quality` is inherited from the base call.
  - test: `cargo build -p ui --lib`
  - output: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 5m 35s`
    (compile-clean is the acceptance bar — a missing field on any of the 7
    sites is a hard compile error, not a silent gap; the build DID surface and
    require fixing one clippy `doc_lazy_continuation` issue unrelated to this
    task, see T4 note)

- [x] T4 — Render panel: `data_quality_block()` in
      `crates/ui/src/screens/leaderboard.rs`, wired into `ready_pane` ABOVE
      the recommendation + table (DATA → ANALYSIS → SUGGEST spine).
  - file: `crates/ui/src/screens/leaderboard.rs:781` (`data_quality_block` fn,
    inserted before `scorecard_block`; `ready_pane` (line ~448) now pushes
    `data_quality_block(&report.data_quality, mode)` first)
  - test: `cargo test -p ui --test leaderboard_data_quality_render --features fixtures`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.39s`
    — PNGs read + eyeballed: `/tmp/leaderboard_data_quality_render.png` shows
    a "Data quality" panel with Venue/Provenance/Trust level/Survival bias
    rows + the informational note, positioned directly under the "Ranking
    strategies for €200 in BTCUSDT" caption and ABOVE the "Recommendation"
    block (confirms the DATA → ANALYSIS → SUGGEST render order);
    `/tmp/leaderboard_data_quality_warnings_render.png` shows all three
    warning lines (Thin liquidity / Wash-trading suspicion / Pump-and-dump)
    in amber `WARN_500`; `/tmp/leaderboard_data_quality_no_warnings_render.png`
    (the negative control) is otherwise pixel-identical minus the Warnings
    row — confirms the conditional row.

- [x] T5 — 18 new `LEADERBOARD_DATA_QUALITY_*` string constants in
      `crate::strings`, registered in `strings::all()`.
  - file: `crates/ui/src/strings.rs:2960` (new section before the P0-1
    scorecard section); `all()` registry entries added at the same location
    the constants live (before `LEADERBOARD_SCORECARD_TITLE`'s entry)
  - test: `cargo test -p ui --lib strings::tests`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 586 filtered out; finished in 0.00s`
    (`all_keys_unique` + `all_values_non_empty`, both green — proves the 18
    new constants are unique-keyed and non-empty)

- [x] T6 — `cargo build -p ui --lib` clean after `cargo clean -p ui`.
  - test: `cargo build -p ui --lib`
  - output: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 5m 35s`
  - note: the FIRST attempt at this build hung — `ps` showed the `agent`
    crate's `rustc` process asleep at 0.0% CPU for 9+ minutes, holding a
    stale 0-byte incremental-compilation lock
    (`target/debug/incremental/agent-*/s-*.lock`). This IS the
    "recurring target/ cache corruption" the brief warned about — `cargo
    clean -p ui` does not clean a transitive dependency's (`agent`'s)
    incremental cache, so the pre-existing corruption there survived the
    ui-only clean. Fix: killed the stuck `rustc`/`cargo` processes, ran
    `rm -rf target/debug/incremental/agent-*`, retried — the retry compiled
    `agent` then `ui` cleanly in 5m35s (see output above). Documented here
    so a future session recognizes the same symptom (`ps` shows 0.0% CPU +
    `S` state on a `rustc` process for minutes with no forward progress →
    stale incremental lock, not a real compile bottleneck).

- [x] T7 — Render-snapshot test (CLAUDE.md non-negotiable): populated
      `DataQualityView` paints a visible panel + a Warnings-row negative
      control.
  - file: `crates/ui/tests/leaderboard_data_quality_render.rs` (new file —
    `data_quality_block_paints_a_substantial_panel` +
    `data_quality_panel_present_with_warnings`)
  - test: `cargo test -p ui --test leaderboard_data_quality_render --features fixtures`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.39s`
    (same run cited under T4; PNGs eyeballed and described there)

- [x] T8 — Create `spec/v2/advisor-data-quality-surface/feature.md` +
      `tasks.md` (this file) — the piece MISSING from the prior reverted
      attempt.
  - file: `spec/v2/advisor-data-quality-surface/feature.md`
  - file: `spec/v2/advisor-data-quality-surface/tasks.md` (this file)
  - test: `python3 scripts/spec_lint.py`
  - output: `spec-lint: PASS (0 violations)`

- [x] T9 — Add `REQ-V2-P1-7-DATA-QUALITY-SURFACE-001` row to
      `spec/trace.toml`.
  - file: `spec/trace.toml:3366` (new `[[req]]` block appended)
  - test: `python3 scripts/spec_lint.py`
  - output: `spec-lint: PASS (0 violations)`

## For the tester to verify

- [ ] T_FINAL_1 — `cargo test -p ui --lib` clean (all tests including the new
      `DataQualityView`/`VenueTrust`/`DataQualityWarning` unit tests pass).
  - test cmd: `cargo test -p ui --lib`
  - developer's own run: `test result: ok. 588 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s`
    (583 pre-existing + 5 new `DataQualityView` unit tests) — tester to
    independently re-run.
- [ ] T_FINAL_2 — `cargo test -p ui --test leaderboard_data_quality_render --features fixtures` PASS
      (both guards: populated-panel foreground floor + Warnings-row negative
      control).
  - test cmd: `cargo test -p ui --test leaderboard_data_quality_render --features fixtures`
  - developer's own run: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.39s`
    — tester to independently re-run AND re-eyeball
    `/tmp/leaderboard_data_quality_render.png` +
    `/tmp/leaderboard_data_quality_warnings_render.png` +
    `/tmp/leaderboard_data_quality_no_warnings_render.png`.
- [ ] T_FINAL_3 — `cargo clippy -p ui --tests --features fixtures -- -D warnings` clean.
  - test cmd: `cargo clippy -p ui --tests --features fixtures -- -D warnings`
  - developer's own run: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 3m 25s`
    (0 warnings/errors; fixed one `doc_lazy_continuation` violation en route —
    an un-indented `+`-prefixed doc-comment continuation line in
    `LEADERBOARD_DATA_QUALITY_SURVIVAL_LABEL`'s doc comment, reworded to avoid
    the leading `+`). Confirms the pre-existing `field_reassign_with_default`
    allow in `promote_swept_config.rs` was NOT disturbed (this feature never
    touches that file).
- [ ] T_FINAL_4 — `cargo fmt --check` clean.
  - test cmd: `cargo fmt --check`
  - developer's own run: exit code 0 (no diff output) — one `cargo fmt` pass
    was needed first (3 files: `strings.rs`, `leaderboard/mod.rs`,
    `tests/leaderboard_data_quality_render.rs`), then `--check` was clean.
- [ ] T_FINAL_5 — `bash scripts/verify_anchors.sh` 119/119 BEFORE and AFTER
      (display-only DTO on the advisor bake-off path — anchor-safe by
      construction, no anchored CLI path touches `BakeoffReportMirror`).
  - test cmd: `bash scripts/verify_anchors.sh`
  - developer's own run (AFTER all code changes): `ANCHORS PASS  (119 / 119)`
    — BEFORE was not separately captured (git baseline was clean at session
    start per the pre-flight `git status` check), but per T-FINAL prior
    features (P1-5/P1-6) the same display-only/no-anchored-CLI-path
    reasoning applies; tester should run the BEFORE/AFTER pair against
    `main` explicitly if a stricter proof is wanted.
- [ ] T_FINAL_6 — `python3 scripts/spec_lint.py` PASS.
  - test cmd: `python3 scripts/spec_lint.py`
  - developer's own run: `spec-lint: PASS (0 violations)`
- [ ] T_FINAL_7 — Sanity-check (not a re-run requirement, no rank-path touch):
      the FROZEN-gate identity proofs `scorecard_does_not_change_ranking` +
      `turnover_does_not_change_ranking` stay green — this feature never
      touches `crates/backtest`.
  - test cmd: `cargo test -p backtest --lib does_not_change_ranking`
  - developer's own run: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 201 filtered out; finished in 0.00s`
    (`bakeoff::tests::turnover_does_not_change_ranking` +
    `bakeoff::scorecard::tests::scorecard_does_not_change_ranking`, both `ok`)

## Deviations from the brief

- The brief's VERIFY section names a single populated + negative-control
  render test in the pattern of `leaderboard_scorecard_render.rs`. Because
  `BakeoffReportMirror.data_quality` is NOT `Option` (per the brief's own
  field-set instruction — every bake-off has a known symbol, so there is no
  degenerate state to model the way `scorecard: Option<ScorecardView>` does),
  the render test cannot construct a "same fixture with the block entirely
  removed" negative control the way the scorecard test does. Instead the test
  file supplies TWO guards: (1) a whole-frame + top-band foreground-floor
  smoke check on the populated panel, and (2) a true present/absent negative
  control on the ONE conditionally-rendered sub-element the DTO has — the
  `Warnings` row (gated on `!warnings.is_empty()`). This is documented inline
  in the test file's module doc comment and confirmed visually: the eyeballed
  PNGs (T4/T7) show the Warnings row is the only pixel difference between the
  with/without frames. Flagging for the tester/architect in case a stronger
  whole-panel negative control is wanted in a follow-up (e.g. adding a
  `#[cfg(test)]`-only variant of `ready_pane` that can suppress the panel,
  mirroring how the scorecard/tail blocks are suppressed via `None`).

## Build-environment note (for future sessions)

A stuck `rustc` process (0.0% CPU, `S` state, holding a stale
`target/debug/incremental/<crate>-*/s-*.lock`) blocked the FIRST `cargo build
-p ui --lib` attempt for 9+ minutes with zero forward progress. `cargo clean
-p ui` only cleans `ui`'s own incremental cache, not a transitive
dependency's (here `agent`'s) — so pre-existing corruption in a dependency's
incremental cache survives a `-p`-scoped clean. Diagnosis: `ps -o
pid,stat,time,etime,%cpu -p <pid>` showing near-zero accumulated CPU time
over many minutes of wall-clock elapsed time, combined with `lsof -p <pid> |
grep lock` showing an open `.lock` file in `target/debug/incremental/`. Fix:
kill the stuck process, `rm -rf target/debug/incremental/<crate>-*` for the
specific crate the lock names, retry.
