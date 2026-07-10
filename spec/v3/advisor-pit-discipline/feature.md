---
slug: advisor-pit-discipline
status: shipped
owner: operator
updated: 2026-07-10
version: 3.2.0
---

# P3 — Point-in-time / as-of data discipline: the look-ahead lint + explicit publication lag

Close the do-not-build register's **one named open gap** (`spec/dev-notes/do-not-build-register.md`
§ *What IS still legitimately open*) and qlib-gap #1 (`spec/dev-notes/qlib-feature-gap-2026-06-17.md`
row #2, "the one structurally-worthwhile gap"): make future-peeking **impossible by
construction** across the whole tree — not re-proven per feature by hand.

This is the **v0.2 follow-on that ADR-0058 § D5 explicitly deferred** ("a `scripts/`-level
look-ahead lint … captured as a v0.2 follow-on if a fresh data channel opens"). Two fresh
exogenous channels have since landed — **DVOL** (`v0.dvol_regime`, ADR-0072) and **macro**
(`v0.macro_riskon`, ADR-0073). Both already route through the shipped type-level
`trading_core::pit::PitSeries` primitive (ADR-0058). P3 is therefore **hardening + lint +
proof, NOT a bug fix** (see § What we found). It ships:

1. a **standalone look-ahead lint** (`scripts/check_no_raw_asof_join.sh`) that forbids raw
   `partition_point`/`binary_search` as-of joins on a time-keyed series anywhere outside the
   sanctioned `core::pit` home — the bypass backstop the type-level API cannot itself enforce;
2. an **explicit, first-class publication-lag** extension to `PitSeries` so the availability
   delay each exogenous series carries is *declared and audited*, not silently baked into how
   a loader computes its join key;
3. the **DVOL + macro retrofit onto the explicit-lag path**, proven byte-identical on the
   current corpora (the lag defaults reproduce today's implicit key exactly).

This is a **data-discipline hardening of the EXISTING pipeline**. It is NOT a new database,
NOT a PIT-data vendor integration, NOT a re-opening of the concluded active-edge search
(`spec/dev-notes/do-not-build-register.md` Group A/C). See [§ Out of scope](#out-of-scope).

---

## Why

The product's entire credibility rests on an **honest negative result** — "on the current
deep-liquidity era, active ≤ passive, measured honestly" (`spec/product.md`). A single signal
that reads a value it could not have known at decision time silently manufactures phantom
alpha and poisons that claim. The moat is *measured honesty*, and look-ahead is the most
insidious way to lose it.

ADR-0058 (shipped v1, 2026-06-18) made the **core** as-of join safe by construction: a
`PitSeries<T>` whose only query is `as_of(query) -> Option<AsOf<T>>` returning the record with
`ts <= query`, with a private-field `AsOf<T>` (no public constructor) — so *joining future
data onto a bar is UNREPRESENTABLE*. It migrated `funding_as_of` + `basis_as_of`. But it
**deliberately dropped two things**, both now due:

- **The lint (D5, D2).** "A `scripts/` grep guard would only catch a *new* hand-rolled bypass,
  captured as a v0.2 follow-on, not built now." The type-level API makes the *core join* safe,
  but nothing structurally stops a future author from writing a fresh raw
  `partition_point(|&(t,_)| t <= bar_ts)` on a new exogenous series. ADR-0058 named the exact
  trigger: **"a v0.2 follow-on if a fresh channel lands."** DVOL and macro are those channels.

- **Explicit publication lag.** The primitive keys on a bare `TimestampMs`. Today, each loader
  encodes availability *implicitly* by choosing the join key — DVOL keys on
  `day_close_ts_ms = day_open + 86_400_000 − 1` (the instant the daily close is fully observed),
  macro on `bar.close_ts` (≈ end-of-day UTC). This is *correct* but *invisible*: the lag is a
  property of the data channel, and it should be **declared once, next to the series, and
  audited** — so a future series with a genuine release lag (a monthly macro print, an on-chain
  series behind a settlement window) cannot get it silently wrong.

### The qlib gap this closes (verbatim)

> **A first-class point-in-time / as-of data discipline (table row #2).** … Today PIT-cleanliness
> is re-proven per feature by hand (leak-checks, day-1 falsifiers) … a focused as-of-join helper
> **+ a lint**, not a new database. Scope-fitting; would strengthen the moat.
> — `spec/dev-notes/qlib-feature-gap-2026-06-17.md` § Genuinely-relevant gaps, ranked, #1.

ADR-0058 delivered the *helper*. **P3 delivers the missing *lint* half of that verdict**, plus
the explicit-lag hardening the DVOL/macro channels motivate.

---

## What we found (the grounding — this determines hardening-only vs bug-fix)

**Verdict: both exogenous joins are already as-of-correct (no look-ahead) TODAY. P3 is
hardening + lint + proof, NOT a fix.** File:line evidence:

### DVOL — `v0.dvol_regime` (ADR-0072)

- The join is `crates/backtest/src/dvol_data.rs::dvol_as_of` (dvol_data.rs:375-390). Its body
  **already builds a `PitSeries` and queries `as_of_value`** (dvol_data.rs:384-389):
  ```rust
  let series = PitSeries::from_unsorted(dvol.iter().map(|&(t, r)| (TimestampMs(t), r)).collect());
  bar_open_ts_ms.iter().map(|&q| series.as_of_value(TimestampMs(q))).collect()
  ```
- The join key is `DvolRow::day_close_ts_ms = day_open_ts_ms + 86_400_000 − 1`
  (dvol_data.rs:108,113) — documented as "the instant the daily close is FULLY observed (the
  as-of join key)."
- Behaviour is pinned by three unit tests: `warm_up_before_first_dvol_is_none`,
  `bar_on_day2_sees_day1_close` (the load-bearing no-look-ahead property: a bar opening at
  day-2 midnight sees the **day-1** close, not day-2), and `forward_fill_across_intraday_bars`
  (dvol_data.rs:410-474). The docstring's own invariant: "Only the DVOL close fully observed
  at-or-before the bar's `open_ts` is used" (dvol_data.rs:367-371).
- **Publication-lag treatment: IMPLICIT-but-correct.** A daily close is not knowable until
  end-of-day, and the key `day_open + 1day − 1` places it exactly at that instant, so the first
  hourly bar that can see it is the next-day 00:00 bar. This is a de-facto next-bar availability,
  encoded in the *key*, not declared as a lag.

### Macro — `v0.macro_riskon` (ADR-0073)

- The reduction is `crates/backtest/src/macro_regime.rs::load_macro_regime_series`, which builds
  a `PitSeries<bool>` via `PitSeries::from_sorted(regime_records)` (macro_regime.rs:209). The
  arm's daily→hourly join is `regime.as_of_value(bar.open_ts)` (documented macro_regime.rs:9-13:
  "Look-ahead is structurally unrepresentable").
- The regime record is keyed by `bar.close_ts` (macro_regime.rs:135, "we key the regime record
  by `close_ts` so the as-of join `regime.as_of_value(bar.open_ts)` only sees a macro close
  AFTER it is fully observed — look-ahead-free"). The SMAs are trailing/past-only
  (macro_regime.rs:185-188) and warm-up records are *excluded* (macro_regime.rs:180-183;
  `as_of_value → None` is treated as risk-OFF/flat).
- **Publication-lag treatment: IMPLICIT-but-correct.** The three series (`^GSPC`/`DX-Y.NYB`/`^TNX`)
  are all market-observable prices/yields with **no release lag** beyond end-of-day, and
  `close_ts` ≈ EOD UTC captures that. Correct for *these three* series; the *pattern* is fragile
  for any future macro series with a genuine multi-day release lag (CPI/NFP), which is exactly
  what explicit lag guards against.

### The four raw as-of joins in the tree (what the lint must govern)

A grep for `partition_point|binary_search` across `crates/` finds exactly four sites; **both
production joins already route through `PitSeries`** — the raw predicate now survives only in
their *docstrings*:

| Site | Kind | Status |
|------|------|--------|
| `crates/backtest/src/funding_data.rs:381` | docstring only (body uses `PitSeries`, funding_data.rs:393-399) | ✅ migrated (ADR-0058) |
| `crates/backtest/src/basis_data.rs:400` | docstring only (body uses `PitSeries`) | ✅ migrated (ADR-0058) |
| `crates/data/examples/stablecoin_diag.rs:303` | **real** `partition_point` — research `f64`/NaN diag, `examples/`-only, non-anchor | allowlisted (research probe; already carries a `// PIT:` note at :299) |
| `crates/data/examples/basis_diag.rs:221` | **real** `partition_point` — research `f64`/NaN diag, `examples/`-only, non-anchor | allowlisted (research probe) |

ADR-0058 deliberately left the two `examples/` diags as `f64`/NaN research probes (§ Alternatives
"Migrate the two f64 diag probes … rejected"). The lint therefore governs **production
`crates/*/src`**, allowlists the `core::pit` home, and allowlists the two research diags via an
explicit per-line marker so any *new* production bypass is caught while these known research
sites do not produce a permanent false-positive.

### Anchor safety of the retrofit (confirmed)

The DVOL and macro arms run only on the **advisor bake-off path with `write_report = false`**
(`crates/backtest/src/engine.rs:302-316`: "Only the `v0.dvol_regime` arm reads this field; the
bake-off loop sets `write_report = false` … no anchored body written"; same for macro). No
anchored `spec/*/reports/*.md` body is produced by these arms, so **a behaviour-identical
retrofit is anchor-safe by construction** and a genuine value change (were one to exist — it
does not, see below) would surface only on non-anchored advisor paths.

---

## Design

M-T1 lock. Three deliverables — the lint, the explicit-lag helper extension, and the
byte-identical retrofit — plus their proofs. Full decision record in **ADR-0086**.

### D1 — The look-ahead lint (`scripts/check_no_raw_asof_join.sh`)

A standalone grep gate mirroring the shipped `scripts/check_no_clocks_in_ui_tests.sh` /
`check_no_secrets_in_llm_artifacts.sh` pattern (WATCHLIST + per-line allowlist marker +
self-test), **NOT** a `scripts/spec_lint.py` category (that linter is docs/spec-only and is
owned by the sibling P6a CHANGELOG-lint work — a separate script avoids the collision).

- **What it forbids:** a raw as-of join — `partition_point(|&(t, _)| t <= …)` or
  `binary_search_by(… t.cmp(&query) …)` / `binary_search_by_key(…)` on a **time-keyed** series
  — anywhere under `crates/*/src/**` (production), i.e. the "hand-rolled bypass" ADR-0058 § D1
  names as the one thing the type-level API cannot itself prevent for the Nth consumer.
- **Scope (SCANLIST):** all production library sources `crates/*/src/**/*.rs`. Test files,
  `benches/`, and `examples/` are *not* forbidden by default (a test may legitimately build a
  raw fixture; research diags are `f64`/NaN and out of the anchor path).
- **Allowlist — two mechanisms:**
  1. **The sanctioned home** `crates/core/src/pit.rs` is exempt outright — it *is* the guarded
     `partition_point` (pit.rs:174), the single implementation every consumer routes through.
  2. **Per-line escape hatch** `// PIT-OK: <reason>` on the same or preceding line — mirrors
     `// CLOCK-OK:`. For a *deliberate, reviewed* raw join outside `pit.rs` (none exist in
     production today; reserved for a future justified exception, which then also demands its
     own ADR note per § Consequences).
- **Grep precision, not a token ban.** Match the **as-of predicate shape** (a `t <= query` /
  `t <= bar` comparison inside a `partition_point`/`binary_search*` call), not the bare method
  name — `partition_point` has legitimate non-temporal uses (e.g. splitting a sorted `Vec` of
  non-timestamp keys). This is why a `cargo-deny` symbol ban is rejected (same reasoning
  `check_no_clocks` gives): a deny-ban is either false-negative or false-positive; a targeted
  grep on the predicate shape is precise.
- **Self-test / negative control (AC — day-1 gate).** The script is invocable standalone; it
  **exits 0 on the clean tree** and **exits non-zero when a planted raw as-of join is injected**
  into a scanned production file. A `--self-test` mode writes a synthetic offending fixture to a
  tempdir, runs the matcher over it, asserts a hit, then a clean fixture and asserts no hit
  (mirrors `spec_lint.py --self-test` and the `check_no_clocks` V4 AC). Wired into
  **`rust-validate`'s pre-test gate** alongside the two existing grep lints.

Why a lint *and* the type-level API (not redundant): the API makes the *core join* safe; the
lint makes the *reach for a bypass* visible and CI-blocked. ADR-0058 chose the type-level API
*over* the lint for v0.1 because there were no fresh channels; with DVOL+macro landed and more
exogenous channels plausible (the register's own § open), the bypass surface is now real and
the backstop is warranted. Together: **impossible-by-construction (type) + caught-if-attempted
(lint).**

### D2 — Explicit, first-class publication lag on `PitSeries`

Make availability delay a **declared** property of the series, not an implicit consequence of
how a loader computes its key. Additive, behaviour-preserving:

- **New constructor path** `PitSeries::from_sorted_with_lag(records, publication_lag_ms)` (and a
  `from_unsorted_with_lag` sibling) that stores a `publication_lag_ms: i64` alongside `records`.
  The existing `from_sorted`/`from_unsorted`/`from_sorted_slice` remain and are defined as
  `*_with_lag(records, 0)` — **zero-lag is the default and is byte-identical to today.**
- **Query semantics.** `as_of(query)` returns the most-recent record whose **effective
  availability time** `record.ts + publication_lag_ms <= query` (equivalently: query against
  `query − publication_lag_ms` over the raw record timestamps). With `publication_lag_ms = 0`
  this reduces **character-for-character** to the current `partition_point(|&(t,_)| t <= query)`
  — the anchor-safe invariant ADR-0058 § D3/D4 established. The proof-carrying `AsOf<T>`
  contract is unchanged (private fields, `as_of_ts()` still returns the *record's* timestamp,
  proven `≤ query − lag ≤ query`).
- **This is additive, not a rewrite.** `PitSeries<T>`'s public surface, the `partition_point`
  one-liner, and every existing test stay. `publication_lag_ms` defaults to 0; only the two
  exogenous loaders opt in (and opt in to **0** on the current channels — see D3 — because their
  key already encodes availability).

**The publication-lag table (grounded in the current key treatment):**

| Series | Channel | Current key (implicit availability) | Explicit `publication_lag_ms` (P3) | Net effect on current corpora |
|--------|---------|-------------------------------------|-----------------------------------|-------------------------------|
| DVOL daily close | `v0.dvol_regime` | `day_open + 86_400_000 − 1` (EOD close instant) | **0** — the key already places the record at the fully-observed instant | **byte-identical** |
| Macro (`^GSPC`/`DX-Y.NYB`/`^TNX`) | `v0.macro_riskon` | `bar.close_ts` ≈ EOD UTC | **0** — market-observable, no release lag beyond EOD; key already correct | **byte-identical** |
| Funding rate | (basis/carry, already migrated) | `funding_time_ms` (settlement instant) | **0** — settlement instant is the availability instant | **byte-identical** (unchanged from ADR-0058) |
| Basis | (basis-reversal, already migrated) | observation `ts` | **0** | **byte-identical** (unchanged) |
| *(future)* a monthly macro print (CPI/NFP) | *(not built)* | *(would be)* release-date key | **the actual release lag** (e.g. a print stamped to the reference-month close is not available for weeks) | N/A — pattern is now *declarable* and audited |

The point of D2 is **not** to change any current value (all lags are 0 because every current
series' key already encodes availability). It is to make the lag a **named, per-series,
audited** quantity, so the next channel *cannot* be joined without an explicit lag decision
that the design and a reviewer can see — closing the "silently baked into the key" fragility.
The lag value for each series is **recorded here** (this table) and **asserted in code** at the
loader (D3), so the two never drift.

### D3 — The retrofit (DVOL + macro route through the explicit-lag path), proven byte-identical

- `dvol_as_of` and `load_macro_regime_series` switch their `PitSeries::from_*` call to the
  `*_with_lag(records, 0)` form, with a code comment citing this feature's lag table and
  ADR-0086. Public signatures unchanged; `Decimal`/`bool` moved, never converted; empty-series
  fast-path retained. Because the lag is 0, the as-of values are **byte-identical**.
- **The byte-identity proof (the anchor question, answered).** Does rerouting change any
  produced series value on the current corpora? **No** — and it is proven, not asserted, by a
  **byte-identity test** mirroring the shipped `*_byte_identical` pattern
  (`out_of_span_filter_via_*_as_of` regression guards over the migrated funding/basis wrappers):
  a test constructs a representative `(ts, value)` series + a bar-open grid, computes the as-of
  result the **legacy raw `partition_point`** way and the **`*_with_lag(_, 0)`** way, and
  asserts `assert_eq!` element-for-element. This bites the moment the explicit-lag path diverges
  from the legacy predicate. Because DVOL/macro run `write_report = false`, this is additionally
  gated by **`scripts/verify_anchors.sh` staying 119/119** across the change (no anchored body
  can move; the developer re-runs it before AND after as the load-bearing gate).
- **If a genuine look-ahead had been found (it was not):** the divergence would surface on
  non-anchored advisor paths only (the arms write no report), the byte-identity test would
  **fail** (correctly — a real fix changes behaviour), and the design would loudly reclassify P3
  as a bug fix with its own divergence-magnitude report. We record here, explicitly, that **the
  grounding found no such leak** — both joins are already as-of-clean — so the retrofit is the
  behaviour-identical hardening path and the byte-identity test is the success condition (a
  *pass*, i.e. zero divergence, is what proves the retrofit correct — the ADR-0058 § D5 "equity
  does not move is the success condition" precedent).

### D4 — Verification floor

The **CLAUDE.md baseline-equity-divergence e2e gate is N/A** here, for the exact ADR-0058 § D5
reason: P3 introduces **no decision variable, no scale, no signal** — the correct outcome is
that equity does **not** move (zero divergence is success), so a divergence gate would assert
the opposite of the design goal. This is recorded, not skipped. The verification floor is:

1. **The lint self-test** (D1 AC) — exits 0 clean, non-zero on a planted bypass.
2. **The byte-identity test** (D3) — legacy predicate ≡ `*_with_lag(_, 0)` on DVOL + macro.
3. **The lag-0 reduction unit test** in `core::pit` — `from_sorted_with_lag(r, 0).as_of(q)` ==
   `from_sorted(r).as_of(q)` for all `q`, plus a positive-lag test proving the lag shifts
   availability (a record at `ts=1000` with `lag=500` is invisible at `query=1200`, visible at
   `query=1500`) — the explicit-lag analogue of the `as_of_no_look_ahead_falsifier`.
4. **`scripts/verify_anchors.sh` = 119/119** before AND after (anchor-safe by construction).
5. The two kept per-loader `no_look_ahead_falsifier` tests and the three DVOL as-of unit tests
   continue to pass unchanged (regression net over the retrofitted wrappers).

---

## Out of scope

- **A new PIT database / vendor feed.** Explicitly ruled out by the qlib note ("a focused
  as-of-join helper + a lint, **not** a new database") and the do-not-build register
  (§ open: "modest — a helper + lint, **not** a new database"). P3 touches no storage layer.
- **Re-opening any concluded search.** No new arm, no new signal, no ranking change. The
  FROZEN gate (`bakeoff/{robustness,rank}.rs`) is byte-untouched (do-not-build Group E).
- **Migrating the two research `f64` diag probes onto `PitSeries`.** ADR-0058 § Alternatives
  already rejected this (research `f64`/NaN, `examples/`-only, non-anchor). P3 *allowlists* them
  in the lint, it does not migrate them.
- **Changing any current series' effective join.** Every current publication lag is **0** because
  each loader's key already encodes availability; P3 makes that explicit and audited, it does
  not move a value. Anchors 119/119 by construction.
- **A `cargo-deny` / clippy-lint implementation of the guard.** Rejected for the same reason
  `check_no_clocks_in_ui_tests.sh` rejects it: a symbol ban on `partition_point` is
  false-positive (legitimate non-temporal uses) or false-negative; a predicate-shape grep is
  precise. (Recorded in ADR-0086 Alternatives.)

---

## Acceptance criteria

- **AC1 — Lint exists and self-tests.** `scripts/check_no_raw_asof_join.sh` exits 0 on the
  current clean tree and non-zero when a raw time-keyed as-of join is planted in a scanned
  production file; `--self-test` passes (synthetic offending + clean fixtures). Wired into
  `rust-validate`'s pre-test gate.
- **AC2 — Explicit lag, byte-identical default.** `PitSeries::from_sorted_with_lag(r, 0)` and
  `from_sorted(r)` produce identical `as_of` results for all queries (unit test); a positive
  lag correctly delays availability (unit test).
- **AC3 — Retrofit byte-identical.** DVOL + macro route through `*_with_lag(_, 0)`; the
  byte-identity test asserts the retrofitted as-of result equals the legacy-predicate result
  element-for-element on a representative series+grid.
- **AC4 — Anchors + spec-lint green.** `scripts/verify_anchors.sh` = **119/119** before AND
  after; `python3 scripts/spec_lint.py` = **PASS(0)**; `scripts/adr_registry_check.py
  --pre-commit` passes (ADR-0086 registered atomically).
- **AC5 — FROZEN gate untouched.** `crates/backtest/src/bakeoff/{robustness,rank}.rs`,
  `classify_verdict`, `verdict_bands` byte-diff clean; no anchored `spec/*/reports/*.md` edited;
  `ci.yml.deferred` untouched.
- **AC6 — Regression net intact.** The three DVOL as-of unit tests, the macro risk-on unit
  tests, and the two per-loader `no_look_ahead_falsifier` tests pass unchanged.

---

## Open questions (for the developer / operator; none block M-T1)

- **OQ-LINT-SCANLIST.** Should the lint additionally scan `crates/*/tests/**` with the
  allowlist marker (catching a test that accidentally *demonstrates* the anti-pattern as if it
  were production), or stay production-`src`-only? Architect lean: **production `src` only** —
  a test building a raw fixture is legitimate; over-scanning invites `// PIT-OK:` marker noise.
  Recorded for the developer to confirm against the actual test corpus.
- **OQ-LAG-STORAGE.** Store `publication_lag_ms` as a plain `i64` field on `PitSeries`, or as a
  `newtype PublicationLagMs(i64)` for symmetry with `TimestampMs`? Architect lean: **plain `i64`
  field** (the lag is an interval, not a timestamp; no join-key round-trip risk; smallest diff).
  Developer's call at implementation.

  **Resolved (developer, 2026-07-10): plain `i64` field, per the architect's lean.**
  `publication_lag_ms: i64` on `PitSeries<T>` (`crates/core/src/pit.rs:126`). No newtype — the
  smallest diff, no join-key round-trip risk.

- **Resolved OQ-LINT-SCANLIST (developer, 2026-07-10): production `src` only, per the
  architect's lean, confirmed against the actual corpus.** `scan_list()`
  (`scripts/check_no_raw_asof_join.sh`) covers `crates/*/src/*.rs` +
  `crates/*/src/**/*.rs` (see § Implementation for the pathspec bug this surfaced and fixed) and
  nothing under `tests/`/`benches/`/`examples/`. No `// PIT-OK:` marker noise was needed on the
  real tree — the sanctioned-home exemption alone (`crates/core/src/pit.rs`) sufficed; the one
  `// PIT-OK:` marker present (on `pit.rs`'s own `as_of` implementation) is belt-and-suspenders
  documentation, not a required escape (that file is exempted outright by name).

---

## Implementation

**Status: `dev-done`.** All four milestones (A–D) built and gate-proven. Handing off to the
tester to verify-and-tick and run the ship-side gates (M-TEST-5/M-TEST-6 already run once by
the developer as the load-bearing "before AND after" proof; the tester re-runs as the
independent verification pass per the standard workflow).

### Milestone A — the lint (D1)

- **`scripts/check_no_raw_asof_join.sh`** (new file, executable). Mirrors
  `check_no_clocks_in_ui_tests.sh`'s shape (`set -euo pipefail`, a per-line `// PIT-OK: <reason>`
  allowlist marker checked on the same-or-preceding line, a clear FAIL banner with remediation
  text) but matches the **as-of predicate shape** rather than a bare method name: a
  `.partition_point(` / `.binary_search_by(` / `.binary_search_by_key(` call
  (`ASOF_METHOD_RE`) on a line that ALSO contains a `<=`-comparison whose left side looks
  timestamp-shaped (`ASOF_PREDICATE_RE` — an identifier, optionally `.0`-projected, followed by
  `<=`). Both legs must hit the SAME source line, matching how every production one-liner in
  this codebase is written (confirmed against the real corpus — no multi-line raw predicate
  exists). `crates/core/src/pit.rs` is exempted by exact path match
  (`SANCTIONED_HOME`), independent of the marker mechanism.
- **A scanlist bug found and fixed during M-TEST-1.** The naive
  `git ls-files 'crates/*/src/**/*.rs'` pathspec silently EXCLUDES any file sitting directly in
  `crates/<name>/src/*.rs` (no subdirectory) — git's `**` glob requires at least one intermediate
  directory. That bug would have hidden 178 of 400 production files, including BOTH P3 retrofit
  targets (`crates/backtest/src/dvol_data.rs`, `crates/backtest/src/macro_regime.rs`) and
  `crates/core/src/pit.rs` itself. Caught live: the first negative-control plant (M-TEST-1) into
  `dvol_data.rs` produced a false PASS. Fixed by combining `'crates/*/src/*.rs'` (flat) +
  `'crates/*/src/**/*.rs'` (nested), de-duplicated (`scan_list()`, `check_no_raw_asof_join.sh`).
  Re-verified: 400 files scanned (was silently 222), negative control now correctly fails the
  gate. This is recorded here because it is a real finding, not implementation noise — a wider
  gap than the ADR/feature.md's four-site grounding survey implied, closed before it could ship.
- `--self-test` writes a synthetic offending fixture (`partition_point(|&(t,_)| t <= query)` in
  a temp `.rs`) and a clean fixture (a `PitSeries`-routed function + a legitimate NON-temporal
  `partition_point` on a `u32` needle, proving no bare-symbol false-positive) to a tempdir; both
  outcomes are asserted.
- Wired into `rust-validate`'s pre-test gate: `.claude/skills/rust-validate/SKILL.md` gained a
  new step 0 ("Pre-test grep gates") invoking both `check_no_clocks_in_ui_tests.sh` and
  `check_no_raw_asof_join.sh` before formatting/clippy. `AGENT.md`'s tooling table gained the
  corresponding row next to the two sibling `check_no_*` scripts.

### Milestone B — the lag (D2)

- `crates/core/src/pit.rs`: `PitSeries<T>` gained a `publication_lag_ms: i64` field.
  `from_sorted_with_lag(records, lag)` and `from_unsorted_with_lag(records, lag)` are the new
  primary constructors; `from_sorted`/`from_unsorted`/`from_sorted_slice` are now one-line
  delegations to their `*_with_lag(_, 0)` sibling (`from_sorted_slice` sets the field inline —
  it has no natural delegation target since it borrows rather than moves).
- `as_of(query)` computes `adjusted_query = query.saturating_sub(publication_lag_ms)` and queries
  `self.records.partition_point(|&(t, _)| t <= adjusted_query)` — the EXACT legacy one-liner,
  now against the lag-adjusted query. At `publication_lag_ms == 0`, `saturating_sub(0)` is a
  byte-identical no-op, so `adjusted_query == query` and the predicate is character-for-character
  the pre-P3 line.
- 5 new unit tests in `pit.rs`'s `#[cfg(test)] mod tests`: `lag_zero_reduction_matches_legacy_from_sorted`,
  `lag_zero_reduction_matches_legacy_from_unsorted` (the AC2 byte-identical-default proof, swept
  across warm-up/boundary/between/past-last queries), `positive_lag_delays_availability` (a
  record at `ts=1000, lag=500` is `None` at `q=1200`/`q=1499`, `Some` with `as_of_ts()==1000` at
  `q=1500`/`q=2000` — proving `as_of_ts()` still returns the record's OWN ts, not the query or
  the availability instant), `positive_lag_multi_record_forward_fill` (two records forward-fill
  independently against their own effective availability instants), and
  `lag_saturating_sub_does_not_underflow` (a defensive `i64::MAX` lag does not panic — clamps to
  warm-up rather than wrapping).

### Milestone C — the retrofit (D3)

- `crates/backtest/src/dvol_data.rs::dvol_as_of` (dvol_data.rs:384-391 after the change):
  `PitSeries::from_unsorted(...)` → `PitSeries::from_unsorted_with_lag(..., 0)`, comment citing
  ADR-0086 + the feature's lag table (DVOL lag = 0, key already EOD-close instant). Public
  signature unchanged.
- `crates/backtest/src/macro_regime.rs::load_macro_regime_series` (macro_regime.rs:209-218 after
  the change): `PitSeries::from_sorted(regime_records)` → `PitSeries::from_sorted_with_lag(regime_records, 0)`,
  same citing comment (macro lag = 0, `close_ts` ≈ EOD UTC, market-observable).
  `MacroRegimeError::PitSort` mapping unchanged.
- **Byte-identity tests** (M-TEST-3), one per site, co-located with each loader's existing tests:
  `dvol_data.rs::tests::dvol_byte_identical_legacy_vs_with_lag_zero` computes the as-of result
  the LEGACY raw `partition_point(|&(t,_)| t <= q)` way (an independent closure recomputing the
  predicate directly over the DVOL row tuples — the pre-P3 oracle) and the RETROFITTED
  `dvol_as_of(...)` way (which now internally routes through `from_unsorted_with_lag(_, 0)`) on
  a representative series (3 daily closes) + a 34-point bar-open grid spanning warm-up, exact
  boundaries, 24h of hourly forward-fill, and past-last-record, asserting element-for-element
  equality. `macro_regime.rs::tests::macro_byte_identical_legacy_vs_with_lag_zero` does the same
  at the `PitSeries<bool>` level (a synthetic 4-record series incl. a tie) since the full
  `load_macro_regime_series` requires Yahoo-corpus I/O and the retrofit only changes the
  CONSTRUCTION call, not what `regime_records` contains.
- Regression net (M-TEST-4) re-run and confirmed unchanged: DVOL's `warm_up_before_first_dvol_is_none`,
  `bar_on_day2_sees_day1_close`, `forward_fill_across_intraday_bars`, `no_look_ahead_falsifier`
  all green; macro's `risk_on_when_all_three_conditions_met` / `risk_off_when_spx_below_sma`
  green; the ADR-0058-era `basis_data.rs::no_look_ahead_falsifier` +
  `funding_data.rs::no_look_ahead_falsifier` (untouched files, confirmed no collateral damage
  from the `pit.rs` primitive change) both green.

### Milestone D — gates run (all verbatim in the developer handoff message; not re-pasted here)

`check_no_raw_asof_join.sh` clean on the real tree (400 files) + `--self-test` PASS;
`cargo test -p trading_core --lib` 110/110; `cargo test -p backtest --lib --features
realdata,yahoo,candle` 240/240 (11 pre-existing `#[ignore]`d on-machine tests unaffected);
`cargo clippy -p trading_core -p backtest --tests --features realdata,yahoo -- -D warnings`
clean; `cargo fmt --check` clean (2 cosmetic wrap findings auto-fixed via `cargo fmt`, both in
newly-added P3 code); `scripts/verify_anchors.sh` 119/119 BEFORE and AFTER; `python3
scripts/spec_lint.py` PASS(0); `scripts/adr_registry_check.py --pre-commit` exit 0 (ADR-0086 was
already registered atomically by the architect at accept-time). FROZEN-gate diff-empty confirmed
via `git status --porcelain | grep -E "bakeoff/(robustness|rank|scorecard)\.rs|spec/.*/reports/|ci\.yml\.deferred"`
(zero matches).

### Files touched

- `scripts/check_no_raw_asof_join.sh` — new.
- `crates/core/src/pit.rs` — `publication_lag_ms` field + `*_with_lag` ctors + adjusted-query
  `as_of` + 5 new unit tests.
- `crates/backtest/src/dvol_data.rs` — retrofit `dvol_as_of` + 1 byte-identity test.
- `crates/backtest/src/macro_regime.rs` — retrofit `load_macro_regime_series` + 1 byte-identity
  test.
- `.claude/skills/rust-validate/SKILL.md` — new pre-test-gate step 0.
- `AGENT.md` — new tooling-table row.
- `spec/v3/advisor-pit-discipline/{feature.md,tasks.md}` — this section + task ticks.
- `spec/trace.toml` — `REQ-V3-P3-PIT-DISCIPLINE-001` row flipped to `dev-done`.

**No CHANGELOG.md line added yet** — per ADR-0082 D2/D3, `feature.md status: shipped` (and the
CHANGELOG entry it implies) is a post-tester/post-presenter milestone; this feature is at
`dev-done`. The tester/orchestrator adds the CHANGELOG line at actual ship.

### Divergence e2e gate: confirmed N/A (not skipped)

Per feature.md § D4, no baseline-equity-divergence e2e test was written. P3 introduces no
decision variable, scale, or signal; the byte-identity tests (M-TEST-3) ARE the success
condition (zero divergence = pass), mirroring the ADR-0058 § D5 precedent this design explicitly
cites. Recorded here per the CLAUDE.md non-negotiable's own carve-out language.

## Changelog

- 2026-07-10 (architect): initial `arch-done` — P3 design lock (D1–D4), ADR-0086 accepted +
  registered, `tasks.md` Milestones A–D authored.
- 2026-07-10 (developer): `dev-done` — all four milestones built + gate-proven (see §
  Implementation). Both open questions resolved (plain `i64` lag field; production-`src`-only
  lint scanlist). One real bug found and fixed en route: the lint's naive `git ls-files` pathspec
  silently missed 178/400 production files including both retrofit targets — caught by the
  M-TEST-1 negative-control plant, fixed before ship. Handoff → tester.
