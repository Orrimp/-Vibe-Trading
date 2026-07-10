---
slug: advisor-pit-discipline
status: arch-done
owner: architect
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

- [ ] **M-DEV-1 — Write `scripts/check_no_raw_asof_join.sh`.** Mirror
  `check_no_clocks_in_ui_tests.sh`: `set -euo pipefail`; a `SCANLIST` of production sources
  (`crates/*/src/**/*.rs`, resolved via `git ls-files` or `find`); a matcher that flags the
  **as-of predicate shape** — a `partition_point(...t <= query|bar...)` or
  `binary_search_by*(...)` on a time-keyed comparison — NOT the bare method name (avoid the
  `partition_point`-on-non-temporal-keys false-positive). Exempt `crates/core/src/pit.rs`
  outright (the sanctioned home). Honour a per-line `// PIT-OK: <reason>` allowlist marker on
  the same or preceding line. Exit non-zero + print each offending `file:line` on any
  unwhitelisted match.
- [ ] **M-DEV-2 — Add `--self-test` (the day-1 AC / negative control).** Writes a synthetic
  **offending** fixture (a raw `partition_point(|&(t,_)| t <= q)` in a temp `.rs`) → assert the
  matcher hits; then a **clean** fixture → assert no hit. Exit 0 iff both hold. Mirrors
  `spec_lint.py --self-test`.
- [ ] **M-TEST-1 — Prove the lint on the real tree.** `bash scripts/check_no_raw_asof_join.sh`
  exits **0** on the current tree (the two production joins are already `PitSeries`; the two
  `examples/` diags are out of `src` scope OR carry `// PIT-OK:`). Then inject a raw as-of join
  into a scanned `crates/*/src` file, confirm the lint exits **non-zero**, and revert. Record
  both outcomes.
- [ ] **M-DEV-3 — Wire into `rust-validate`'s pre-test gate.** Add the script to the
  `rust-validate` skill's pre-test lint list next to `check_no_clocks_in_ui_tests.sh` /
  `check_no_secrets_in_llm_artifacts.sh` so CI-equivalent runs enforce it.

## Milestone B — Explicit publication lag on `PitSeries` (D2)

- [ ] **M-DEV-4 — Add `publication_lag_ms` + lag-aware constructors.** In `crates/core/src/pit.rs`:
  store `publication_lag_ms: i64` on `PitSeries<T>`; add `from_sorted_with_lag(records, lag)` +
  `from_unsorted_with_lag(records, lag)`; **redefine** the existing `from_sorted`/`from_unsorted`/
  `from_sorted_slice` as `*_with_lag(records, 0)` (zero-lag default = byte-identical to today).
  Update `as_of` so the visible record satisfies `record.ts + publication_lag_ms <= query`
  (implement as querying the raw records against `query.saturating_sub(publication_lag_ms)` so
  the `partition_point(|&(t,_)| t <= q')` one-liner is preserved verbatim). `AsOf::as_of_ts()`
  still returns the **record's** ts. No public-surface removal; `serde`/derives intact.
- [ ] **M-TEST-2 — lag-0 reduction + positive-lag unit tests (in `pit.rs #[cfg(test)]`).**
  (a) for a representative series, `from_sorted_with_lag(r, 0).as_of(q) == from_sorted(r).as_of(q)`
  for a sweep of `q` (byte-identical default); (b) positive lag delays availability — a record
  at `ts=1000, lag=500` is `None` at `q=1200`, `Some` at `q=1500`, and `as_of_ts()==1000` there
  (the explicit-lag analogue of `as_of_no_look_ahead_falsifier`).

## Milestone C — Retrofit DVOL + macro, proven byte-identical (D3)

- [ ] **M-DEV-5 — Route DVOL through the explicit-lag path.** In `dvol_as_of` (dvol_data.rs),
  switch `PitSeries::from_unsorted(...)` → `PitSeries::from_unsorted_with_lag(..., 0)` with a
  comment citing `feature.md` § lag table + ADR-0086 (DVOL lag = 0, key already EOD-close).
  Public signature unchanged.
- [ ] **M-DEV-6 — Route macro through the explicit-lag path.** In `load_macro_regime_series`
  (macro_regime.rs), switch `PitSeries::from_sorted(regime_records)` →
  `PitSeries::from_sorted_with_lag(regime_records, 0)` with the same citing comment (macro lag =
  0, `close_ts` ≈ EOD UTC, market-observable). `MacroRegimeError::PitSort` mapping unchanged.
- [ ] **M-TEST-3 — Byte-identity test (the anchor question).** New test (co-located with the
  DVOL as-of tests, and a macro sibling) mirroring the `out_of_span_filter_via_*_as_of` pattern:
  build a representative `(ts, value)` series + bar-open grid; compute the as-of result the
  **legacy raw `partition_point(|&(t,_)| t <= q)`** way and the **retrofitted `*_with_lag(_, 0)`**
  way; `assert_eq!` element-for-element. This is the proof that the retrofit moves no value.
- [ ] **M-TEST-4 — Regression net unchanged.** Confirm `warm_up_before_first_dvol_is_none`,
  `bar_on_day2_sees_day1_close`, `forward_fill_across_intraday_bars`, the macro risk-on tests,
  and the two per-loader `no_look_ahead_falsifier` tests all pass unchanged.

## Milestone D — Gates + close

- [ ] **M-TEST-5 — Anchor + spec-lint proof.** `scripts/verify_anchors.sh` → **119/119** (before
  AND after the whole change); `python3 scripts/spec_lint.py` → **PASS(0)**;
  `scripts/adr_registry_check.py --pre-commit` → pass (ADR-0086 row present, README `updated:`
  staged). `cargo fmt --check` + `cargo clippy -- -D warnings` on `core` + `backtest`.
- [ ] **M-TEST-6 — FROZEN-gate + anchor-body zero-diff.** `git diff --stat` shows no change to
  `bakeoff/{robustness,rank}.rs`, `classify_verdict`, `verdict_bands`, any `spec/*/reports/*`,
  or `ci.yml.deferred`.
- [ ] **M-DEV-7 — Flip lifecycle.** On green, set `feature.md status: shipped` and the trace row
  `REQ-V3-P3-PIT-DISCIPLINE-001 state = "shipped"` (ADR-0082 single-source-of-truth); append the
  CHANGELOG line (per remediation-plan P6a discipline). Tester closes the loop with a report.

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
