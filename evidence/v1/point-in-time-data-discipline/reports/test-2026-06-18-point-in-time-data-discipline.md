# Test report — point-in-time-data-discipline v0.1.0 (2026-06-18)

**VERDICT → PASS**

Verification by the orchestrator, independently re-running the load-bearing gates
(the anchor byte-identity and the falsifier are the feature's whole point) and
cross-checking the developer's run. A behaviour-preserving refactor: the correct
outcome is equity/anchors **UNCHANGED**, so there is no new backtest anchor.

## 1. Scope

Consolidate the (4) hand-rolled as-of joins into one type-level `core::pit`
primitive (`PitSeries<T>` / `AsOf<T>`) where look-ahead is **unrepresentable**, so
the next consumer is causal by construction rather than by remembering a falsifier.

## 2. Gates (orchestrator-reverified)

| Gate | Result |
|---|---|
| `verify_anchors.sh` | **PASS 119/119** (re-run independently — the migration reproduces the as-of values byte-for-byte) |
| `cargo test -p trading_core` | **85 passed / 0 failed** (incl. 12 `pit::` tests + the unit falsifier) |
| `cargo test -p trading_core --test pit_compile_fail` | **PASS** — `pit_look_ahead_is_a_compile_error ... ok` (orchestrator re-ran) |
| `cargo test -p backtest --lib --features realdata` | **103 passed / 0 failed** (both kept `no_look_ahead_falsifier` regression guards pass) |
| `cargo clippy -p trading_core -p backtest --lib --tests -- -D warnings` | clean (0 warnings) |
| `cargo fmt --check` | clean |
| `spec_lint` | floor (1 = the immutable vol-verdict link) |

## 3. Acceptance criteria

| AC | What | Evidence | Status |
|---|---|---|---|
| AC1 | Type-level as-of API (look-ahead unrepresentable) | `crates/core/src/pit.rs` — `PitSeries<T>` (`from_sorted`/`from_unsorted`, `as_of`/`as_of_value`), `AsOf<T>` with **private fields, no public ctor / get / Index** | PASS |
| AC2 | Self-proving look-ahead falsifier | `core::pit` unit falsifier (`as_of_no_look_ahead_falsifier`) + trybuild compile-fail (`tests/compile_fail/pit_no_public_constructor.rs` tries an `AsOf{...}` struct literal → **won't compile**; I read the fixture — it's a genuine fabricate-future-record attempt, not a trivial error) | PASS |
| AC3 | Zero anchor delta (behaviour-preserving) | `verify_anchors.sh` 119/119; public `funding_as_of`/`basis_as_of` signatures byte-stable (`funding_data.rs:384`, `basis_data.rs:403`) | PASS |
| AC4 | Migrate all call sites, no new crate edge | `funding_as_of` + `basis_as_of` bodies build a `PitSeries` (sigs unchanged); `build_*_at_return` transitive; the 2 f64 `examples/` probes (`basis_diag.rs:219`, `stablecoin_diag.rs:301`) keep doc-pointer adapters; `core` gains no dep edge | PASS |
| AC5 | (lint) | **N/A** — D2 dropped the lint; the type makes look-ahead unrepresentable for the core join (a v0.2 grep guard is captured, not built) | N/A |

## 4. Notes

- **Permitted deviation (dev-flagged):** `crates/backtest` has `#![deny(clippy::expect_used)]`,
  so the infallible `PitSeries::from_unsorted` (stable sort, order-preserving on
  pre-sorted input) was used in the migrated bodies instead of `from_sorted` (which
  returns `Result` → needs `.expect()`). Bytes identical — confirmed by 119/119.
- The architect found a **fourth** copy (`stablecoin_diag.rs:301`) the analyst brief
  missed; covered (doc-pointer adapter).
- CLAUDE.md day-1 equity-divergence e2e gate is **N/A** (data-discipline refactor, no
  overlay/sizing modifier — correct outcome is equity unchanged). Floor = AC2 + AC3.

## 5. Net

The moat-hardening is structural now: a future Nth consumer of an as-of join gets a
**compile error** if it tries to read future data, instead of relying on a hand-written
falsifier. Behaviour and all 119 anchors are byte-unchanged.
