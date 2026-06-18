---
adr: 0058
title: Point-in-time as-of-join primitive — type-level PitSeries in crates/core
status: accepted
date: 2026-06-18
supersedes: none
superseded-by: none
extends: 0041
---

# ADR-0058: Point-in-time as-of-join primitive — type-level `PitSeries` in `crates/core`

## Context

The project enforces point-in-time (PIT) cleanliness — "no signal reads future
data" — *by hand*, feature by feature. The sidecar as-of join (funding / basis
onto bars) is the one genuinely-manual look-ahead seam (the price path is already
PIT-clean by construction via streaming `Strategy::on_bar` + past-only
`RingBuffer`). That seam is **hand-rolled and duplicated four times**, each a
copy of `partition_point(|&(t, _)| t <= bar_ts)` → rightmost-at-or-before,
`None` on warm-up:

- `funding_as_of` — `crates/backtest/src/funding_data.rs:378` (production, `Decimal`)
- `basis_as_of` — `crates/backtest/src/basis_data.rs:397` (production, `Decimal`)
- `funding_as_of` f64 clone — `crates/data/examples/basis_diag.rs:219` (research)
- `funding_as_of` f64 clone — `crates/data/examples/stablecoin_diag.rs:301`
  (research; the on-chain spike's probe — **a fourth copy the originating brief
  did not enumerate**, which is precisely the "duplication invites an uncaught
  copy" risk, already realized)

The two production copies feed the carry/basis signals and the block-bootstrap,
which feed **anchored** backtest report surfaces (`anchors.toml`:
`v1-carry-*`, `v1-basis-reversal-*`, `v2-mn-*`, plus funding/basis-fed bootstrap
distributions). The credibility of the program's terminal "active ≤ passive,
honestly measured" negative result (product.md, 2026-06-08) rests on **no
reported number being contaminated by look-ahead**. Today that is defended per
feature by a remembered `no_look_ahead_falsifier`. The risk: a fifth, Nth
consumer copies the pattern and *nothing structural* forces the copy to be causal.

This is a data-discipline **hardening + dedup** (no known leak; the four
hand-checks pass). Because it introduces a **new `crates/core` domain primitive**
and performs a **cross-crate migration that touches the anchor-feeding path**, the
design is recorded here. Why now: the qlib feature-gap analysis (2026-06-17)
named this the one structurally-worthwhile gap.

## Decision

Introduce a **type-level** as-of primitive in `crates/core` and route every
production sidecar as-of join through it, behaviour-preserving (byte-identical
as-of values), so look-ahead becomes **unrepresentable** rather than
falsifier-policed.

### D1. Type-level `PitSeries<T>` + `AsOf<T>`, not a runtime-guarded function

The primitive is `PitSeries<T>` (a sorted, timestamped series) whose only query
methods are `as_of(query) -> Option<AsOf<T>>` and `as_of_value(query) ->
Option<T>`, both returning only the most-recent record with `ts <= query`
(`None` on warm-up). `AsOf<T>` carries a **private** `as_of_ts` (proven `<= query`)
and a **private** `value`, with accessors `as_of_ts()` / `value()` /
`into_value()` and **no public constructor**. There is no `get(i)`, no `Index`,
no future-returning iterator, no `records()` accessor — so there is **no API
surface that yields a record at `ts > query`**, and a caller cannot fabricate an
`AsOf` whose timestamp post-dates its query. Look-ahead is a compile-time
impossibility, not a debug-assert. A runtime-guarded free function
(`as_of_join` + `debug_assert!`) was rejected (see Alternatives): the guarantee
would be debug-time-only and a hand-rolled `partition_point` could still bypass
it. The full signatures are fixed in
[`spec/point-in-time-data-discipline/feature.md` § The API surface](../../point-in-time-data-discipline/feature.md#the-api-surface-exact-signatures--cratescoresrcpitrs).

### D2. Home crate is `crates/core` (new `pit` module); no new dependency edge

`PitSeries`/`AsOf`/`TimestampMs` live in a new `crates/core/src/pit.rs`, exported
as `pub use pit::{PitSeries, AsOf, TimestampMs};`. `core` is the workspace **base
crate** — `backtest` and `data` depend on it; it depends on neither (verified: no
cycle). It already carries `rust_decimal`, `time`, `thiserror`, `proptest`,
`trybuild`, so the primitive and its compile-fail test land with **zero new
dependency edge**. This is the ADR-0041 layering rule applied (code lives in the
crate whose layer it serves; a crate gains no edge that violates the hierarchy):
the as-of join is a domain primitive alongside `Bar` / `Timestamp` / `FundingObs`,
so it belongs in `core`. A `crates/data` home was rejected — it would force
`backtest` to depend on `data` (a new, wrong-direction edge) to reach the
primitive.

### D3. The join key is `TimestampMs(i64)`, not `core::Timestamp`

The as-of key is a transparent `i64` ms-since-epoch newtype `TimestampMs`, **not**
`core::Timestamp`. The production loaders join on raw `i64` ms
(`bar_open_ts_ms`, `funding_time_ms`); keying on `Timestamp` would round-trip
through `Timestamp::unix_millis()` (`i128 → i64` truncation) and risk an anchor
delta. `TimestampMs` derives `Ord` as the plain `i64` ordering, so the query
predicate `|&(t, _)| t <= query` is preserved **character-for-character** against
the legacy `|&(t, _)| t <= bar_ts`.

### D4. Behaviour-preserving migration — byte-identical as-of values (anchor-safe)

The two production functions keep their **public signatures unchanged**
(`pub fn funding_as_of(&[(i64, Decimal)], &[i64]) -> Vec<Option<Decimal>>`,
ditto `basis_as_of`); only their bodies change to build a `PitSeries` once and
`map` queries through `as_of_value`. The `build_*_at_return` wrappers and their
`Vec<Vec<Option<Decimal>>>` output — the materialized array
`BlockBootstrapPathGen::with_funding`/`with_basis` consume — are **unchanged in
type and shape**. As-of values are byte-identical for three independently-checkable
reasons: (i) the same `partition_point(t <= q)` predicate and the same `idx-1` /
`None` off-by-one; (ii) the empty-series fast-path retained verbatim; (iii)
`Decimal` is **moved**, never converted — **no `f64` round-trip**, no rescale.
Therefore every anchored carry/basis/bootstrap surface stays byte-identical and
**`scripts/verify_anchors.sh` remains 119/119** (the developer re-verifies as the
load-bearing gate). No anchored `spec/*/reports/*.md` is edited and no
`anchors.toml` SHA changes, so neither the ADR-0038 § D6 re-emission protocol nor
the `spec/anchors.toml` anchor-mutation-requires-an-ADR rule is triggered by this
change.

### D5. Verification floor — self-proving falsifier + zero anchor delta; equity-divergence gate N/A

The day-1 gate is **the self-proving look-ahead falsifier**, lifted to
`core::pit`: a unit test that queries causally, forward-shifts the series, and
asserts the two results differ (`assert_ne!`), plus a **`trybuild` compile-fail**
fixture asserting no API returns a record at `ts > query` (private `AsOf` fields;
no `AsOf::new`) — so *removing the guard makes a test fail*. The two existing
per-loader `no_look_ahead_falsifier` tests are **kept** as thin regression guards
over the migrated wrappers (D5 belt-and-suspenders; zero anchor cost). The
CLAUDE.md "day-1 baseline-equity-divergence e2e test for every strategy overlay /
sizing modifier" gate is **N/A** here: this feature introduces no decision
variable, no scale, no signal — the *correct* outcome is that equity does **not**
move (zero anchor delta is the success condition), so the divergence gate would
assert the opposite of the design goal. The `scripts/`-level look-ahead lint is
**dropped** — the type-level API makes the core join safe by construction; a grep
guard would only catch a *new* hand-rolled bypass, captured as a v0.2 follow-on,
not built now.

## Alternatives considered

- **Runtime-guard free function (`as_of_join` + `debug_assert!` sorted/causal)** —
  rejected: the guarantee is debug-time-only and a hand-rolled `partition_point`
  bypasses it, committing the project to maintaining a `scripts/` lint as the
  backstop (ongoing carry-cost). Type-level makes look-ahead unrepresentable for
  the Nth consumer with no lint to maintain.
- **Home in `crates/data` (next to the loaders)** — rejected: would force
  `backtest` to add a `data` dependency edge (wrong direction, new edge) to reach
  a primitive that is, semantically, a `core` domain type. Violates the ADR-0041
  layering rule.
- **Key on `core::Timestamp`** — rejected: an `i128 → i64` round-trip on the join
  key risks an anchor delta and adds friction for zero benefit; the loaders speak
  `i64` ms natively.
- **Migrate the two f64 diag probes onto `PitSeries<Decimal>`** — rejected: they
  are research-grade `f64` with `NaN`-not-`None` warm-up, `examples/`-only, not
  anchor-feeding; forcing them onto the Decimal API changes their sentinel and
  adds `f64↔Decimal` friction for no production value. They keep a one-line
  doc-pointer to the canonical API instead.
- **Retire the two per-loader `no_look_ahead_falsifier` tests** — rejected: they
  now exercise the migrated wrappers at zero anchor cost; keeping them is a cheap
  regression net that pins the wrapper still routes through the guard.
- **Ship a `scripts/` look-ahead lint now** — rejected for v0.1: redundant with
  the type-level guarantee for the four current sites; captured as a v0.2
  follow-on if a fresh data channel opens.

## Consequences

If this rule is violated — a future sidecar signal hand-rolls a
`partition_point(t <= q)` as-of join instead of reaching for `core::pit` — the
moat-protecting "no look-ahead" property reverts to per-feature manual discipline
for that consumer, and a forward-shifted-series bug there would silently
contaminate any anchored surface it feeds. Enforcement and mechanical checks:

- **`crates/core/src/pit.rs` `#[cfg(test)]`** — the shared
  `as_of_no_look_ahead_falsifier` + the warm-up / boundary / between / empty /
  unsorted-reject / ties unit suite (AC2).
- **`crates/core/tests/pit_compile_fail.rs`** (`trybuild`) — proves the no-future
  guarantee is structural; a regression that exposes an `AsOf` field or adds a
  future-returning accessor makes it fail (AC2).
- **`scripts/verify_anchors.sh`** — must stay **119/119** byte-identical across
  the migration; a single mismatch is a REGRESSION blocking the ship per CLAUDE.md
  (AC3).
- **`crates/backtest` `funding_data.rs` / `basis_data.rs` test suites** — the two
  kept `no_look_ahead_falsifier` + `out_of_span_filter_via_*_as_of` regression
  guards over the migrated wrappers (D5).
- **v0.2 follow-on (not yet built)** — an optional `scripts/` grep guard "no raw
  `partition_point(|&(t,_)| t <= ...)` as-of join outside `core::pit`" as the
  bypass backstop if a fresh channel lands.

Migration scope and the exact per-site plan (all four copies, the tests touched,
and the bootstrap-input invariance) live in
[`spec/point-in-time-data-discipline/feature.md` § Design](../../point-in-time-data-discipline/feature.md#design)
and [`tasks.md`](../../point-in-time-data-discipline/tasks.md) (5 M-DEV + 5 M-TEST).

## Changelog
- 2026-06-18 (architect): initial accept. Locked D1 type-level `PitSeries`/`AsOf`
  in `crates/core::pit`; D2 home = core (no new edge, ADR-0041-consistent); D3
  key = `TimestampMs(i64)` (anchor-safe, no `Timestamp` round-trip); D4
  byte-identical migration of `funding_as_of`/`basis_as_of` + transitive
  `build_*_at_return`, 119/119 anchors preserved; D5 verification floor = shared
  falsifier (unit + trybuild compile-fail) + zero anchor delta, equity-divergence
  gate N/A, lint dropped. Flagged a fourth as-of copy (`stablecoin_diag.rs:301`)
  the originating brief did not enumerate. extends ADR-0041 (layering invariant).
