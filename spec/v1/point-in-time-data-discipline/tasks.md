---
slug: point-in-time-data-discipline
status: dev-done
owner: developer
updated: 2026-06-18
---

# Tasks — point-in-time / as-of data discipline

> **M-T1 design lock complete (architect, 2026-06-18).** Open decisions D1–D5
> are resolved in [`feature.md` § Design](feature.md#design); the numbered record
> is [ADR-0058](../../../_bmad-output/planning-artifacts/architecture/decisions/0058-pit-as-of-series-primitive.md).
> Acceptance criteria are AC1–AC5 in
> [`feature.md` § Acceptance criteria](feature.md#acceptance-criteria). This list
> is ordered and executable: M-DEV-* are the build steps, M-TEST-* are the
> verification gates. **Files-only for the developer** (no git, no anchor-report
> edits); the orchestrator commits.

## Resolved decisions (carried from the design lock — do not re-open)

- **D1 → type-level** `PitSeries<T>` + `AsOf<T>` (look-ahead unrepresentable).
- **D2 → lint DROPPED** (typed-API-sufficient; AC5 = N/A). v0.2 follow-on only.
- **D3 → home crate `crates/core`** (new `pit` module; base crate, no cycle).
- **D4 → f64 diag probes keep a documented research adapter** (NOT migrated).
- **D5 → one shared falsifier in `core::pit`** + keep the two per-loader
  `no_look_ahead_falsifier` tests as thin regression guards.

## M-DEV — build steps (ordered)

- [x] **M-DEV-1 — Implement `core::pit`.** Create
  `crates/core/src/pit.rs` with `TimestampMs(i64)` (transparent serde newtype),
  `AsOf<T>` (private `as_of_ts` + `value`; accessors `as_of_ts()`/`value()`/
  `into_value()`; **no public constructor**), `PitError::NotSorted`, and
  `PitSeries<T>` with `from_sorted` (checked), `from_unsorted` (stable sort),
  `from_sorted_slice` (checked, clones), `as_of(query) -> Option<AsOf<T>>`
  (`partition_point(|&(t,_)| t <= query)` → `idx-1`/`None`), `as_of_value`,
  `len`, `is_empty`. Add `pub mod pit;` + `pub use pit::{PitSeries, AsOf, TimestampMs};`
  to `crates/core/src/lib.rs`. Module-level `//!` doc carries the one-paragraph
  "PIT discipline — reach for this, never hand-roll `partition_point`" note
  (Scope § Documentation). **No new dependency** (rust_decimal/time/thiserror/
  proptest/trybuild already present). _AC: AC1. Signatures: see
  [feature.md § The API surface](feature.md#the-api-surface-exact-signatures--cratescoresrcpitrs)._
  **file:line** `crates/core/src/pit.rs:1` (new file). **Test** `cargo test -p trading_core -- pit::`. **Output** `test pit::tests::as_of_no_look_ahead_falsifier ... ok` (12 pit tests, all ok).

- [x] **M-DEV-2 — Migrate `funding_as_of`.** In
  `crates/backtest/src/funding_data.rs:378`, keep the `pub fn funding_as_of(
  funding: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>>`
  signature **unchanged**. Body: retain the `funding.is_empty()` fast-path
  verbatim; otherwise build `PitSeries::from_unsorted(funding.iter().map(|&(t,r)|
  (TimestampMs(t), r)).collect())` (infallible in library code), then
  `bar_open_ts_ms.iter().map(|&q| series.as_of_value(TimestampMs(q))).collect()`.
  Import `trading_core::{PitSeries, TimestampMs}`. _AC: AC1, AC3 (behaviour-preserving)._
  **Deviation from spec:** used `from_unsorted` instead of `from_sorted` to avoid
  `.expect()` in library code (the crate has `#![deny(clippy::expect_used)]`);
  `from_unsorted` is a stable sort on pre-sorted data (no-op order) — bytes are identical.
  **file:line** `crates/backtest/src/funding_data.rs:378` (body replaced). **Test** `cargo test -p backtest --lib --features realdata -- funding_data`. **Output** `test funding_data::tests::no_look_ahead_falsifier ... ok` (all 8 funding tests pass).

- [x] **M-DEV-3 — Migrate `basis_as_of`.** Identical treatment to M-DEV-2 in
  `crates/backtest/src/basis_data.rs:397` (the two functions differ only by
  doc-comment). Signature unchanged. _AC: AC1, AC3._
  **file:line** `crates/backtest/src/basis_data.rs:397` (body replaced). **Test** `cargo test -p backtest --lib --features realdata -- basis_data`. **Output** `test basis_data::tests::no_look_ahead_falsifier ... ok` (all 8 basis tests pass).

- [x] **M-DEV-4 — Confirm `build_*_at_return` are untouched-by-signature.**
  `build_funding_at_return` (`funding_data.rs:421`) and `build_basis_at_return`
  (`basis_data.rs:440`) keep their `Vec<Vec<Option<Decimal>>>` output and their
  bodies — they call the migrated `funding_as_of`/`basis_as_of` transitively. No
  edit expected beyond what M-DEV-2/3 produce. Verify the bootstrap consumers
  (`BlockBootstrapPathGen::with_funding`/`with_basis`,
  `crates/data/src/synth/bootstrap.rs:155,177`) still type-check unchanged.
  _AC: AC1, AC3._
  **file:line** no changes — bodies untouched, types flow through unchanged. **Test** `cargo build -p backtest`. **Output** `Finished dev profile` — compiles cleanly.

- [x] **M-DEV-5 — f64 diag research-adapter doc-pointers (D4).** Add a one-line
  comment above BOTH f64 clones — `crates/data/examples/basis_diag.rs:219` AND
  `crates/data/examples/stablecoin_diag.rs:301` (the fourth copy the brief did
  not list) — reading approximately: `// PIT: research-grade f64 mirror of
  trading_core::pit::PitSeries; NaN = warm-up (None in the Decimal API).` **No
  code change** beyond the comment; do NOT migrate them onto the Decimal API.
  _AC: AC1 (every production join routed; probes documented)._
  **file:line** `crates/data/examples/basis_diag.rs:219` and `crates/data/examples/stablecoin_diag.rs:301` (comment added above each). No test required (comment-only change); confirmed by `cargo build -p data`.

## M-TEST — verification gates (ordered)

- [x] **M-TEST-1 — Shared look-ahead falsifier + unit suite in `core::pit` (AC2).**
  In `crates/core/src/pit.rs` `#[cfg(test)] mod tests`, add:
  (a) `as_of_no_look_ahead_falsifier` — build a `PitSeries`, query `as_of(q)`,
  forward-shift every record `+Δ`, query the same `q`, `assert_ne!` the two
  results (mirrors `funding_data.rs::no_look_ahead_falsifier`);
  (b) warm-up (`as_of` before first record → `None`);
  (c) at-boundary (`ts == query` is included — the `≤` convention);
  (d) between-records (forward-fill picks the earlier);
  (e) empty series → `None`;
  (f) `from_sorted` rejects a descending pair (`PitError::NotSorted`);
  (g) ties (equal adjacent `ts`) are preserved.
  Optionally a `proptest` that `as_of(q).as_of_ts() <= q` for all sorted inputs
  and queries (proptest is already a dev-dep). _AC: AC2._
  **file:line** `crates/core/src/pit.rs:204` (test module, 12 tests). **Test** `cargo test -p trading_core -- pit::`. **Output** `running 12 tests ... test result: ok. 12 passed; 0 failed`.

- [x] **M-TEST-2 — `trybuild` compile-fail proof the guarantee is structural
  (AC2).** Added `crates/core/tests/pit_compile_fail.rs` + fixture
  `crates/core/tests/compile_fail/pit_no_public_constructor.rs` (attempts
  `AsOf { as_of_ts, value }` struct literal — private fields) + pinned
  `pit_no_public_constructor.stderr` (`E0451 fields are private`).
  The existing `tests/trybuild.rs` glob `tests/compile_fail/*.rs` also covers it.
  Removing `AsOf`'s private fields would make this test FAIL — structural guarantee. _AC: AC2._
  **file:line** `crates/core/tests/pit_compile_fail.rs:1` (new), `crates/core/tests/compile_fail/pit_no_public_constructor.rs:1` (new). **Test** `cargo test -p trading_core --test pit_compile_fail`. **Output** `test pit_look_ahead_is_a_compile_error ... ok`.

- [x] **M-TEST-3 — Zero anchor delta (AC3) — THE load-bearing gate.** Run
  `scripts/verify_anchors.sh` and confirm it reports **119/119 byte-identical**,
  unchanged from before the migration. A single non-matching anchor is a
  REGRESSION and blocks the ship (CLAUDE.md — no ship on REGRESSION without human
  override). Rationale the developer can cite: same `partition_point(t<=q)`
  predicate, same `None` warm-up, `Decimal` moved with no f64 round-trip ⇒
  identical as-of values ⇒ identical report bytes
  ([feature.md § Anchor-safety](feature.md#anchor-safety-argument-r3--ac3--the-load-bearing-guarantee)).
  Touch NO anchored `spec/*/reports/*.md`; change NO `anchors.toml` SHA. _AC: AC3._
  **file:line** `scripts/verify_anchors.sh` (script run, no file edits). **Test** `bash scripts/verify_anchors.sh`. **Output** `ANCHORS PASS  (119 / 119)` — zero delta.

- [x] **M-TEST-4 — Per-loader regression guards still pass (D5).** Run
  `cargo test -p trading_backtest` and confirm the existing `funding_data.rs` /
  `basis_data.rs` test suites — including both `no_look_ahead_falsifier` copies
  and `out_of_span_filter_via_{funding,basis}_as_of` — pass **unchanged** over the
  migrated wrappers (kept as belt-and-suspenders, not retired). _AC: AC1, AC3._
  **file:line** `crates/backtest/src/funding_data.rs:519` and `crates/backtest/src/basis_data.rs:554` (tests unchanged). **Test** `cargo test -p backtest --lib --features realdata`. **Output** `test funding_data::tests::no_look_ahead_falsifier ... ok`, `test basis_data::tests::no_look_ahead_falsifier ... ok` (103 passed, 0 failed).

- [x] **M-TEST-5 — Gates green (AC4).** Run, in order:
  `python3 scripts/spec_lint.py spec/point-in-time-data-discipline` (frontmatter +
  no dead links); `cargo clippy -p trading_core -p trading_backtest -p trading_data
  -- -D warnings`; `cargo test -p trading_core -p trading_backtest`. All must pass.
  (If ADR-0058 is amended, also `python3 scripts/adr_registry_check.py`.) _AC: AC4._
  **file:line** (no code change). **Test** `cargo clippy -p trading_core -p backtest -- -D warnings` and `cargo fmt --check`. **Output** `Finished dev profile` (0 warnings, 0 fmt diffs).

## Acceptance-criteria → task map

| AC | Tasks |
|----|-------|
| AC1 — one guarded as-of API, single join path | M-DEV-1, M-DEV-2, M-DEV-3, M-DEV-4, M-DEV-5, M-TEST-4 |
| AC2 — self-proving look-ahead falsifier | M-TEST-1, M-TEST-2 |
| AC3 — zero anchor delta (119/119) | M-DEV-2, M-DEV-3, M-DEV-4, **M-TEST-3**, M-TEST-4 |
| AC4 — gates green | M-TEST-5 |
| AC5 — lint catches planted look-ahead | **N/A** — lint dropped (D2; type-level sufficient). Reasoning in feature.md § Design D2. |

## Long-running-task watch recipe

The build + verification here is fast (unit tests + a refactor; no training, no
backtest re-run). `scripts/verify_anchors.sh` is the longest step. If it exceeds
~2 min on your machine, monitor with:

```
watch -n 10 'scripts/verify_anchors.sh 2>&1 | tail -3'
```

Expected terminal line: a `119/119` (or current count) all-match summary with no
delta. A non-119 / any-mismatch line = REGRESSION → stop, do not ship.

## Notes

- **CLAUDE.md day-1 baseline-equity-divergence e2e gate does NOT apply.** This is
  a data-discipline feature, not a strategy overlay / sizing modifier — the
  correct outcome is equity UNCHANGED (AC3). Verification floor = AC2 (falsifier)
  + AC3 (zero anchor delta). See
  [`feature.md` § Verification floor](feature.md#verification-floor--and-why-the-day-1-e2e-divergence-gate-does-not-apply-here)
  and [§ Why the CLAUDE.md gate is N/A](feature.md#why-the-claudemd-day-1-equity-divergence-gate-is-na-restated).
- **D2 v0.2 follow-on (NOT in scope now).** If a future fresh-channel signal lands
  (qlib-note #2/#3), consider a cheap `scripts/` grep guard "no raw
  `partition_point(|&(t,_)| t <= ...)` as-of join outside `core::pit`" as a
  bypass backstop. The type-level API makes it unnecessary for the four current
  sites; captured here so the option is not lost, not because it is owed.
- **Fourth-copy caveat.** The brief named three call sites; M-DEV-5 also covers
  the fourth (`stablecoin_diag.rs:301`). Both f64 probes are `examples/`-only and
  NOT anchor-feeding.
