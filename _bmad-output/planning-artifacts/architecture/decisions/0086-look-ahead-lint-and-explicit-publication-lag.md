---
adr: 0086
title: Look-ahead lint + explicit publication lag — the ADR-0058 v0.2 follow-on
status: accepted
date: 2026-07-10
supersedes: none
superseded-by: none
extends: 0058
---

# ADR-0086: Look-ahead lint + explicit publication lag — the ADR-0058 v0.2 follow-on

## Context

The product's credibility is an **honest negative result** ("on the current deep-liquidity era,
active ≤ passive, measured honestly" — `spec/product.md`). A single signal that reads a value it
could not have known at decision time silently manufactures phantom alpha. Look-ahead is the
most insidious way to lose the *measured-honesty* moat.

ADR-0058 (shipped v1, 2026-06-18) made the **core** as-of join safe by construction: a type-level
`trading_core::pit::PitSeries<T>` whose only query is `as_of(query) -> Option<AsOf<T>>` returning
the record with `ts <= query`, with a private-field `AsOf<T>` (no public constructor). It migrated
`funding_as_of` + `basis_as_of`. It **deliberately deferred two things**:

- **A `scripts/` look-ahead lint** — dropped for v0.1 (§ D5, § D2): "a grep guard would only catch
  a *new* hand-rolled bypass, captured as a **v0.2 follow-on if a fresh data channel opens**, not
  built now."
- **Explicit publication lag** — the primitive keys on a bare `TimestampMs`; each loader encodes
  availability *implicitly* by its choice of join key.

Two fresh exogenous channels have since landed and are exactly the trigger ADR-0058 named:
**DVOL** (`v0.dvol_regime`, ADR-0072) and **macro** (`v0.macro_riskon`, ADR-0073). Grounding both
(feature.md § What we found):

- **DVOL** — `crates/backtest/src/dvol_data.rs::dvol_as_of` (dvol_data.rs:375-390) already builds a
  `PitSeries` and queries `as_of_value`; the key is `day_close_ts_ms = day_open + 86_400_000 − 1`
  (dvol_data.rs:108,113), the fully-observed close instant. Unit tests `bar_on_day2_sees_day1_close`
  et al. pin the no-look-ahead property. **Already as-of-correct.**
- **Macro** — `crates/backtest/src/macro_regime.rs::load_macro_regime_series` builds a
  `PitSeries<bool>` (macro_regime.rs:209), keyed on `bar.close_ts` ≈ EOD UTC (macro_regime.rs:135),
  SMAs trailing/past-only, warm-up excluded. **Already as-of-correct.**

A grep for `partition_point|binary_search` across `crates/` finds four sites; **both production
joins now use `PitSeries`** (the raw predicate survives only in their docstrings), and the two real
raw joins are research `f64`/NaN diags in `crates/data/examples/{stablecoin_diag,basis_diag}.rs`
(non-anchor, `examples/`-only, deliberately left as-is by ADR-0058 § Alternatives).

So this is a **hardening + lint + proof, NOT a bug fix.** No leak was found; the retrofit is
behaviour-identical. This ADR records the design because it introduces a **new cross-cutting
data-discipline invariant** (a lint wired into the standard gate + a first-class lag concept on a
`crates/core` primitive) — the kind of durable, cross-feature contract that is ADR-worthy — and
extends ADR-0058.

The DVOL/macro arms run only on the advisor bake-off path with `write_report = false`
(`engine.rs:302-316`), so no anchored `spec/*/reports/*.md` body is produced by them; the retrofit
is **anchor-safe by construction**.

## Decision

Deliver the two deferred halves of ADR-0058 § D5 / the qlib gap #1 verdict, plus the byte-identical
retrofit of the two fresh channels.

### D1. A standalone look-ahead lint `scripts/check_no_raw_asof_join.sh`

Forbid a **raw time-keyed as-of join** — `partition_point(|&(t, _)| t <= …)` or a
`binary_search_by*` on a `t <= query` comparison — anywhere under production `crates/*/src/**`,
outside the sanctioned home `crates/core/src/pit.rs`. It mirrors the shipped
`scripts/check_no_clocks_in_ui_tests.sh` / `check_no_secrets_in_llm_artifacts.sh` pattern: a
SCANLIST, a per-line `// PIT-OK: <reason>` allowlist marker, a `--self-test` (writes a synthetic
offending fixture → assert hit; a clean fixture → assert no hit), exits non-zero on any unwhitelisted
match, and is wired into **`rust-validate`'s pre-test gate**. It is **NOT** a `scripts/spec_lint.py`
category — that linter is docs/spec-only (dead-links / frontmatter / trace / anchors) and its lint
surface is owned by the parallel P6a CHANGELOG-lint work; a separate script avoids that collision.
The matcher keys on the **as-of predicate shape**, not the bare `partition_point` symbol, so
legitimate non-temporal `partition_point` uses do not false-positive (a `cargo-deny` symbol ban is
rejected below for exactly this reason). This is the bypass backstop the type-level API cannot
itself provide for the Nth consumer: **impossible-by-construction (type) + caught-if-attempted
(lint).**

### D2. Explicit, first-class publication lag on `PitSeries`

Make availability delay a **declared** property of the series, not an implicit consequence of a
loader's key choice. Additive and behaviour-preserving:

- Store `publication_lag_ms: i64` on `PitSeries<T>`. Add `from_sorted_with_lag(records, lag)` +
  `from_unsorted_with_lag(records, lag)`; **redefine** the existing `from_sorted`/`from_unsorted`/
  `from_sorted_slice` as `*_with_lag(records, 0)`. **Zero-lag is the default and is byte-identical
  to today.**
- `as_of(query)` returns the most-recent record whose effective availability
  `record.ts + publication_lag_ms <= query`, implemented by querying the raw records against
  `query.saturating_sub(publication_lag_ms)` so the `partition_point(|&(t,_)| t <= q')` one-liner —
  and thus the anchor-safe invariant of ADR-0058 § D3/D4 — is preserved verbatim at `lag = 0`.
  `AsOf<T>` stays proof-carrying (private fields; `as_of_ts()` returns the record's ts, proven
  `≤ query − lag ≤ query`).

**Publication-lag table (grounded — all current lags are 0 because each key already encodes
availability):**

| Series | Channel | Current key | `publication_lag_ms` | Effect |
|--------|---------|-------------|----------------------|--------|
| DVOL daily close | `v0.dvol_regime` | `day_open + 86_400_000 − 1` (EOD close) | **0** | byte-identical |
| Macro `^GSPC`/`DX-Y.NYB`/`^TNX` | `v0.macro_riskon` | `bar.close_ts` ≈ EOD UTC | **0** | byte-identical |
| Funding | (basis/carry, ADR-0058) | `funding_time_ms` (settlement) | **0** | unchanged |
| Basis | (basis-reversal, ADR-0058) | observation `ts` | **0** | unchanged |
| *future* monthly macro print (CPI/NFP) | *not built* | release-date key | *the real release lag* | N/A — now *declarable* |

The value of D2 is not to move any current number (all lags are 0) but to make the lag a **named,
per-series, audited** quantity recorded in the feature's lag table and asserted at the loader, so a
future channel cannot be joined without an explicit, reviewable lag decision — closing the "silently
baked into the key" fragility.

### D3. Retrofit DVOL + macro onto the explicit-lag path, proven byte-identical

`dvol_as_of` and `load_macro_regime_series` switch their `PitSeries::from_*` calls to the
`*_with_lag(records, 0)` form (public signatures unchanged; `Decimal`/`bool` moved, not converted;
empty-series fast-path retained). Because the lag is 0, the as-of values are **byte-identical**,
proven by a **byte-identity test** mirroring the shipped `out_of_span_filter_via_*_as_of` pattern:
compute the as-of result the legacy raw-`partition_point` way and the `*_with_lag(_, 0)` way on a
representative series + bar-open grid and `assert_eq!` element-for-element. `scripts/verify_anchors.sh`
stays **119/119** (the arms write no report; no anchored body can move — the developer re-runs it
before AND after as the load-bearing gate). No anchored `spec/*/reports/*.md` is edited and no
`anchors.toml` SHA changes, so neither the ADR-0038 § D6 re-emission protocol nor the anchor-mutation
ADR rule is triggered.

### D4. Verification floor; equity-divergence gate N/A

The CLAUDE.md baseline-equity-divergence e2e gate is **N/A** here for the ADR-0058 § D5 reason: P3
introduces no decision variable, no scale, no signal — the *correct* outcome is that equity does
**not** move (zero divergence is success), so a divergence gate would assert the opposite of the
design goal. The floor is: (1) the lint self-test (D1); (2) the byte-identity test (D3); (3) the
`core::pit` lag-0-reduction + positive-lag unit tests (D2); (4) `verify_anchors.sh` 119/119 before
and after; (5) the three DVOL as-of unit tests + macro risk-on tests + the two per-loader
`no_look_ahead_falsifier` tests unchanged.

## Alternatives considered

- **A `cargo-deny` / clippy symbol ban on `partition_point`** — rejected for the same reason
  `check_no_clocks_in_ui_tests.sh` rejects a `SystemTime` ban: a bare-symbol ban is either
  false-negative or false-positive (`partition_point` has legitimate non-temporal uses). A grep on
  the *as-of predicate shape* + a per-line `// PIT-OK:` marker is precise.
- **A `scripts/spec_lint.py` category for the lint** — rejected: `spec_lint.py` is a docs/spec
  linter (dead-links, frontmatter, trace, anchors, status-drift), not a Rust-source scanner, and its
  surface is owned by the parallel P6a CHANGELOG-lint work; a standalone script matches the existing
  `check_no_*` family and avoids the collision.
- **Reject the lint again (type-level API is enough)** — rejected: ADR-0058 dropped the lint *for
  v0.1 because no fresh channels existed*; it named "a v0.2 follow-on if a fresh channel lands." DVOL
  + macro landed, and more exogenous channels are plausible (the do-not-build register's own § open),
  so the bypass surface is now real. Type + lint is defence-in-depth, not redundancy.
- **Model lag by shifting the loader's join key (status quo, no primitive change)** — rejected: it
  works for the current price/close series but keeps the lag *invisible and per-loader*, which is the
  exact fragility a future release-lagged series (CPI/NFP) would trip. A first-class, defaulted,
  additive lag on the primitive makes the decision explicit and audited at zero cost to current
  values (lag = 0 is byte-identical).
- **Migrate the two `examples/` `f64` diag probes onto `PitSeries`** — rejected (unchanged from
  ADR-0058 § Alternatives): research `f64`/NaN, `examples/`-only, non-anchor. P3 *allowlists* them in
  the lint's scope, it does not migrate them.
- **A non-zero lag for DVOL/macro (e.g. +1 bar)** — rejected: the current keys already place each
  record at its fully-observed instant, so a non-zero lag would *double-count* the delay and change
  values (breaking byte-identity + moving non-anchored advisor equity for zero correctness gain). The
  correct lag on the current channels is exactly 0.

## Consequences

If this invariant is violated — a future sidecar signal hand-rolls a raw `partition_point(t <= q)`
as-of join outside `core::pit`, or joins a release-lagged series with an unstated lag — the
moat-protecting "no look-ahead" property reverts to per-feature manual discipline and a
forward-shifted-series bug there could silently contaminate any surface it feeds. Enforcement and
mechanical checks:

- **`scripts/check_no_raw_asof_join.sh`** — the standalone grep gate + `--self-test`; wired into
  `rust-validate`'s pre-test gate; exits non-zero on any unwhitelisted production raw as-of join
  (D1 / feature.md AC1).
- **`crates/core/src/pit.rs` `#[cfg(test)]`** — the lag-0-reduction + positive-lag unit tests, atop
  the existing `as_of_no_look_ahead_falsifier` + warm-up/boundary/between/empty/unsorted/ties suite
  (D2 / AC2).
- **The DVOL + macro byte-identity test** — legacy predicate ≡ `*_with_lag(_, 0)` element-for-element
  (D3 / AC3).
- **`scripts/verify_anchors.sh`** — must stay **119/119** byte-identical across the retrofit; a single
  mismatch is a REGRESSION blocking the ship per CLAUDE.md (AC4).
- **`scripts/adr_registry_check.py --pre-commit`** — this ADR's README `## Registry` row is present
  and the README `updated:` is staged in the same commit (registered atomically per the 2026-05-29
  durable contract).

A **new** exogenous channel with a genuine publication lag (a monthly macro print, an on-chain
series behind a settlement window) MUST (a) route through `core::pit`, (b) declare its non-zero
`publication_lag_ms` in the consuming feature's lag table, and (c) — because that is a value-moving
data decision on a potentially anchor-feeding path — record it in its own ADR. This ADR makes such a
lag *declarable and enforceable*; it does not pre-authorize any specific non-zero value.

Design detail and the exact per-site plan (lint, primitive extension, retrofit, tests) live in
[`spec/v3/advisor-pit-discipline/feature.md` § Design](../../../../spec/v3/advisor-pit-discipline/feature.md#design)
and [`tasks.md`](../../../../spec/v3/advisor-pit-discipline/tasks.md) (Milestones A–D).

## Changelog
- 2026-07-10 (architect): initial accept. Delivers the two halves ADR-0058 § D5 deferred to a "v0.2
  follow-on if a fresh channel lands" (DVOL/ADR-0072 + macro/ADR-0073 are those channels): D1 a
  standalone `scripts/check_no_raw_asof_join.sh` grep lint (mirrors `check_no_clocks_in_ui_tests.sh`;
  predicate-shape match + `// PIT-OK:` marker + `--self-test`; wired into `rust-validate`; NOT a
  `spec_lint.py` category — avoids the P6a collision); D2 additive first-class `publication_lag_ms`
  on `PitSeries` (`*_with_lag` ctors, existing ctors = `_with_lag(_, 0)` = byte-identical default);
  D3 DVOL + macro retrofit onto `*_with_lag(_, 0)` proven byte-identical (both joins found ALREADY
  as-of-correct — hardening, not a fix), 119/119 anchors preserved by construction (`write_report =
  false` arms); D4 verification floor = lint self-test + byte-identity test + lag-0/positive-lag unit
  tests + zero anchor delta, equity-divergence gate N/A (no decision variable — zero divergence is
  success). extends ADR-0058. Registered atomically in `adr/README.md`.
