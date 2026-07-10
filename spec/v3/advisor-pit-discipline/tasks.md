---
slug: advisor-pit-discipline
status: dev-done
owner: developer
updated: 2026-07-10
---

# Tasks — P3 PIT / as-of discipline (the look-ahead lint + explicit publication lag)

Ordered for the developer. Design lock: `feature.md` § Design (D1–D4) + **ADR-0086**.

**Standing gates (every commit):** `scripts/verify_anchors.sh` → 119/119 (before AND after —
anchors keyed by NAME not filename, run it even for a "code-only" change) · `python3
scripts/spec_lint.py` → PASS(0) · `cargo fmt --check` · `cargo clippy -- -D warnings` on
touched crates. FROZEN gate `bakeoff/{robustness,rank}.rs` + `classify_verdict`/`verdict_bands`
byte-untouched; no `spec/*/reports/*` edits; `ci.yml.deferred` untouched.

**Ground truth to hold in view while editing** (verbatim, do not re-derive):
- `crates/core/src/pit.rs:112-207` — `PitSeries<T>`, `from_sorted`/`from_unsorted`/
  `from_sorted_slice`, `as_of`/`as_of_value` (the `partition_point(|&(t,_)| t <= query)` at :174).
- `crates/backtest/src/dvol_data.rs:375-390` — `dvol_as_of` (retrofit site 1).
- `crates/backtest/src/macro_regime.rs:95-210` — `load_macro_regime_series`, `from_sorted` at :209 (retrofit site 2).
- `scripts/check_no_clocks_in_ui_tests.sh` — the grep-lint pattern to mirror (WATCHLIST +
  `// CLOCK-OK:` marker + self-test AC; wired into `rust-validate`).

---

## Milestone A — The look-ahead lint (D1)

- [x] **M-DEV-1 — Write `scripts/check_no_raw_asof_join.sh`.** Mirror
  `check_no_clocks_in_ui_tests.sh`: `set -euo pipefail`; a `SCANLIST` of production sources
  (`crates/*/src/**/*.rs`, resolved via `git ls-files` or `find`); a matcher that flags the
  **as-of predicate shape** — a `partition_point(...t <= query|bar...)` or
  `binary_search_by*(...)` on a time-keyed comparison — NOT the bare method name (avoid the
  `partition_point`-on-non-temporal-keys false-positive). Exempt `crates/core/src/pit.rs`
  outright (the sanctioned home). Honour a per-line `// PIT-OK: <reason>` allowlist marker on
  the same or preceding line. Exit non-zero + print each offending `file:line` on any
  unwhitelisted match.
  File: `scripts/check_no_raw_asof_join.sh` (new file, ~200 lines). Test cmd:
  `bash scripts/check_no_raw_asof_join.sh`. Output: `PIT-JOIN LINT PASS (scanned 400 production
  src files; sanctioned home + PIT-OK markers exempt)`, exit 0. NOTE: the SCANLIST pathspec
  originally used `git ls-files 'crates/*/src/**/*.rs'` alone, which silently EXCLUDES files
  sitting directly in `crates/<name>/src/*.rs` (no subdir) — a real bug found live during
  M-TEST-1 (see that row) and fixed in this same file (`scan_list()` combines
  `'crates/*/src/*.rs'` + `'crates/*/src/**/*.rs'`, de-duplicated).
- [x] **M-DEV-2 — Add `--self-test` (the day-1 AC / negative control).** Writes a synthetic
  **offending** fixture (a raw `partition_point(|&(t,_)| t <= q)` in a temp `.rs`) → assert the
  matcher hits; then a **clean** fixture → assert no hit. Exit 0 iff both hold. Mirrors
  `spec_lint.py --self-test`.
  File: `scripts/check_no_raw_asof_join.sh:73-108` (`self_test()`). Test cmd:
  `bash scripts/check_no_raw_asof_join.sh --self-test`. Output: `SELF-TEST PASS: offending
  fixture flagged (1 hit), clean fixture silent (0 hits)`, exit 0.
- [x] **M-TEST-1 — Prove the lint on the real tree.** `bash scripts/check_no_raw_asof_join.sh`
  exits **0** on the current tree (the two production joins are already `PitSeries`; the two
  `examples/` diags are out of `src` scope OR carry `// PIT-OK:`). Then inject a raw as-of join
  into a scanned `crates/*/src` file, confirm the lint exits **non-zero**, and revert. Record
  both outcomes.
  Test cmd (clean tree): `bash scripts/check_no_raw_asof_join.sh` → `PIT-JOIN LINT PASS (scanned
  400 production src files...)`, exit 0. Negative control: appended a synthetic
  `records.partition_point(|&(t, _)| t <= query)` fn to `crates/backtest/src/dvol_data.rs`
  (backed up first, restored after — `diff` confirmed byte-identical restore), re-ran the lint →
  `FAIL  crates/backtest/src/dvol_data.rs:637 — raw as-of predicate outside core::pit`, exit 1.
  Both outcomes recorded. FIRST ATTEMPT (before the pathspec fix in M-DEV-1) produced a FALSE
  PASS on the planted violation — the file wasn't in the scanlist at all; this is what surfaced
  the M-DEV-1 bug, fixed, and the negative control re-run confirmed the fix (see M-DEV-1 note).
- [x] **M-DEV-3 — Wire into `rust-validate`'s pre-test gate.** Add the script to the
  `rust-validate` skill's pre-test lint list next to `check_no_clocks_in_ui_tests.sh` /
  `check_no_secrets_in_llm_artifacts.sh` so CI-equivalent runs enforce it.
  File: `.claude/skills/rust-validate/SKILL.md` (new step 0, "Pre-test grep gates", invokes both
  `check_no_clocks_in_ui_tests.sh` and `check_no_raw_asof_join.sh`); `AGENT.md` (new tooling-table
  row, matching the existing `check_no_secrets_in_llm_artifacts.sh`/`check_no_clocks_in_ui_tests.sh`
  rows). No test command applicable (documentation wiring); verified by reading the rendered
  SKILL.md step and the AGENT.md table row post-edit.

## Milestone B — Explicit publication lag on `PitSeries` (D2)

- [x] **M-DEV-4 — Add `publication_lag_ms` + lag-aware constructors.** In `crates/core/src/pit.rs`:
  store `publication_lag_ms: i64` on `PitSeries<T>`; add `from_sorted_with_lag(records, lag)` +
  `from_unsorted_with_lag(records, lag)`; **redefine** the existing `from_sorted`/`from_unsorted`/
  `from_sorted_slice` as `*_with_lag(records, 0)` (zero-lag default = byte-identical to today).
  Update `as_of` so the visible record satisfies `record.ts + publication_lag_ms <= query`
  (implement as querying the raw records against `query.saturating_sub(publication_lag_ms)` so
  the `partition_point(|&(t,_)| t <= q')` one-liner is preserved verbatim). `AsOf::as_of_ts()`
  still returns the **record's** ts. No public-surface removal; `serde`/derives intact.
  File: `crates/core/src/pit.rs:126` (field), `:141-143` (`from_sorted` delegates),
  `:158-171` (`from_sorted_with_lag`), `:181-183` (`from_unsorted` delegates), `:190-199`
  (`from_unsorted_with_lag`), `:212-222` (`from_sorted_slice`), `:239-253` (`as_of` with
  `saturating_sub`). Test cmd: `cargo test -p trading_core --lib pit::`. Output: `test result:
  ok. 17 passed; 0 failed; 0 ignored`.
- [x] **M-TEST-2 — lag-0 reduction + positive-lag unit tests (in `pit.rs #[cfg(test)]`).**
  (a) for a representative series, `from_sorted_with_lag(r, 0).as_of(q) == from_sorted(r).as_of(q)`
  for a sweep of `q` (byte-identical default); (b) positive lag delays availability — a record
  at `ts=1000, lag=500` is `None` at `q=1200`, `Some` at `q=1500`, and `as_of_ts()==1000` there
  (the explicit-lag analogue of `as_of_no_look_ahead_falsifier`).
  File: `crates/core/src/pit.rs` `#[cfg(test)] mod tests` — `lag_zero_reduction_matches_legacy_from_sorted`,
  `lag_zero_reduction_matches_legacy_from_unsorted`, `positive_lag_delays_availability`,
  `positive_lag_multi_record_forward_fill`, `lag_saturating_sub_does_not_underflow` (5 new
  tests, ~110 lines). Test cmd: `cargo test -p trading_core --lib pit::`. Output: `test
  pit::tests::lag_zero_reduction_matches_legacy_from_sorted ... ok`, `test
  pit::tests::lag_zero_reduction_matches_legacy_from_unsorted ... ok`, `test
  pit::tests::positive_lag_delays_availability ... ok`, `test
  pit::tests::positive_lag_multi_record_forward_fill ... ok`, `test
  pit::tests::lag_saturating_sub_does_not_underflow ... ok`; `test result: ok. 17 passed; 0
  failed`.

## Milestone C — Retrofit DVOL + macro, proven byte-identical (D3)

- [x] **M-DEV-5 — Route DVOL through the explicit-lag path.** In `dvol_as_of` (dvol_data.rs),
  switch `PitSeries::from_unsorted(...)` → `PitSeries::from_unsorted_with_lag(..., 0)` with a
  comment citing `feature.md` § lag table + ADR-0086 (DVOL lag = 0, key already EOD-close).
  Public signature unchanged.
  File: `crates/backtest/src/dvol_data.rs:384-391`. Test cmd: `cargo test -p backtest --lib
  --features realdata,yahoo dvol_data::`. Output: `test result: ok. 9 passed; 0 failed; 1
  ignored` (the 1 ignored is the pre-existing on-machine-only `real_corpus_load_smoke`,
  untouched).
- [x] **M-DEV-6 — Route macro through the explicit-lag path.** In `load_macro_regime_series`
  (macro_regime.rs), switch `PitSeries::from_sorted(regime_records)` →
  `PitSeries::from_sorted_with_lag(regime_records, 0)` with the same citing comment (macro lag =
  0, `close_ts` ≈ EOD UTC, market-observable). `MacroRegimeError::PitSort` mapping unchanged.
  File: `crates/backtest/src/macro_regime.rs:209-218`. Test cmd: `cargo test -p backtest --lib
  --features realdata,yahoo macro_regime::`. Output: `test result: ok. 7 passed; 0 failed; 0
  ignored`.
- [x] **M-TEST-3 — Byte-identity test (the anchor question).** New test (co-located with the
  DVOL as-of tests, and a macro sibling) mirroring the `out_of_span_filter_via_*_as_of` pattern:
  build a representative `(ts, value)` series + bar-open grid; compute the as-of result the
  **legacy raw `partition_point(|&(t,_)| t <= q)`** way and the **retrofitted `*_with_lag(_, 0)`**
  way; `assert_eq!` element-for-element. This is the proof that the retrofit moves no value.
  File: `crates/backtest/src/dvol_data.rs` `tests::dvol_byte_identical_legacy_vs_with_lag_zero`
  (3 daily closes + a 34-point bar-open grid); `crates/backtest/src/macro_regime.rs`
  `tests::macro_byte_identical_legacy_vs_with_lag_zero` (4 records incl. a tie + a 9-point grid,
  tested at the `PitSeries<bool>` level since the full loader needs Yahoo-corpus I/O). Test cmd:
  `cargo test -p backtest --lib --features realdata,yahoo dvol_data:: macro_regime::` (run
  separately per cargo's single-positional-filter limit). Output: `test
  dvol_data::tests::dvol_byte_identical_legacy_vs_with_lag_zero ... ok`; `test
  macro_regime::tests::macro_byte_identical_legacy_vs_with_lag_zero ... ok`.
- [x] **M-TEST-4 — Regression net unchanged.** Confirm `warm_up_before_first_dvol_is_none`,
  `bar_on_day2_sees_day1_close`, `forward_fill_across_intraday_bars`, the macro risk-on tests,
  and the two per-loader `no_look_ahead_falsifier` tests all pass unchanged.
  Test cmd: `cargo test -p backtest --lib --features realdata,yahoo,candle` (full backtest suite,
  all features). Output: `test result: ok. 240 passed; 0 failed; 11 ignored`. Individually
  confirmed: `test dvol_data::tests::warm_up_before_first_dvol_is_none ... ok`, `test
  dvol_data::tests::bar_on_day2_sees_day1_close ... ok`, `test
  dvol_data::tests::forward_fill_across_intraday_bars ... ok`, `test
  dvol_data::tests::no_look_ahead_falsifier ... ok`, `test
  macro_regime::tests::risk_on_when_all_three_conditions_met ... ok`, `test
  macro_regime::tests::risk_off_when_spx_below_sma ... ok`, `test
  basis_data::tests::no_look_ahead_falsifier ... ok`, `test
  funding_data::tests::no_look_ahead_falsifier ... ok` (the two ADR-0058-era per-loader
  falsifiers, in `basis_data.rs`/`funding_data.rs` — untouched files, confirming zero collateral
  damage from the `pit.rs` primitive change).

## Milestone D — Gates + close

- [x] **M-TEST-5 — Anchor + spec-lint proof.** `scripts/verify_anchors.sh` → **119/119** (before
  AND after the whole change); `python3 scripts/spec_lint.py` → **PASS(0)**;
  `scripts/adr_registry_check.py --pre-commit` → pass (ADR-0086 row present, README `updated:`
  staged). `cargo fmt --check` + `cargo clippy -- -D warnings` on `core` + `backtest`.
  Test cmds + outputs (run BEFORE the change as baseline, then again AFTER — both 119/119):
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)` (both runs); `python3
  scripts/spec_lint.py` → `spec-lint: PASS (0 violations)`; `python3
  scripts/adr_registry_check.py --pre-commit` → exit 0 (ADR-0086 already registered atomically by
  the architect at accept-time, confirmed via `grep -n "0086" spec/architecture/adr/README.md`);
  `cargo fmt --check -p trading_core -p backtest` → exit 0 (after `cargo fmt -p trading_core -p
  backtest` fixed 2 cosmetic wrap findings, both inside newly-added P3 code —
  `crates/core/src/pit.rs:238-243` and `:558-563`); `cargo clippy -p trading_core -p backtest
  --tests --features realdata,yahoo -- -D warnings` → `Finished` with zero warnings printed
  (exit 0). NOTE for the tester: I (developer) ran and verified this row myself with full
  citations — the tester should still independently re-run per the standard workflow.
- [x] **M-TEST-6 — FROZEN-gate + anchor-body zero-diff.** `git diff --stat` shows no change to
  `bakeoff/{robustness,rank}.rs`, `classify_verdict`, `verdict_bands`, any `spec/*/reports/*`,
  or `ci.yml.deferred`.
  Test cmd: `git status --porcelain | grep -E "bakeoff/(robustness|rank|scorecard)\.rs|spec/.*/reports/|ci\.yml\.deferred"`.
  Output: no lines printed, `grep` exit 1 (zero matches — confirmed empty). `git diff --stat`
  full change list: `.claude/skills/rust-validate/SKILL.md`, `AGENT.md`,
  `crates/backtest/src/dvol_data.rs`, `crates/backtest/src/macro_regime.rs`,
  `crates/core/src/pit.rs`, plus new untracked `scripts/check_no_raw_asof_join.sh` — none of
  these touch the FROZEN surface. NOTE for the tester: same as M-TEST-5 — independently re-run.
- [ ] **M-DEV-7 — Flip lifecycle.** On green, set `feature.md status: shipped` and the trace row
  `REQ-V3-P3-PIT-DISCIPLINE-001 state = "shipped"` (ADR-0082 single-source-of-truth); append the
  CHANGELOG line (per remediation-plan P6a discipline). Tester closes the loop with a report.
  **NOT done by the developer** — per ADR-0082 D2/D3, `state = "shipped"` is legal ONLY once
  `feature.md status: shipped`, which is a post-tester/post-presenter milestone (mirrors the
  precedent at `spec/trace.toml`'s other `dev-done` rows, e.g. the ui-designer's 2026-07-10
  `crown-credibility` handoff, which also stops at `dev-done` and leaves the shipped-flip to the
  tester/orchestrator). This developer round flipped `feature.md status: dev-done` and
  `trace.toml state = "dev-done"` instead (see the row's own comment); `HANDOFF → tester` closes
  M-DEV-7 out of scope for this round.

---

## Notes / decisions deferred to the developer

- **OQ-LINT-SCANLIST** (feature.md): production `src` only vs also `tests/`. Architect lean:
  `src` only. Confirm against the test corpus; if `tests/` is added, expect a few `// PIT-OK:`
  markers on legitimate fixtures.
- **OQ-LAG-STORAGE** (feature.md): plain `i64` field vs `PublicationLagMs` newtype. Architect
  lean: plain `i64` (interval, not a join key). Developer's call.
- **Divergence e2e gate: N/A by design** (feature.md § D4) — P3 has no decision variable; zero
  divergence (byte-identity PASS) is the success condition, per the ADR-0058 § D5 precedent.
  Record N/A with this pointer; do NOT author a divergence test that would assert the opposite
  of the goal.
