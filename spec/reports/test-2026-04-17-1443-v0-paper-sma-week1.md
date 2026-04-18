---
title: Test Report
feature: v0-paper-sma-week1
run_id: 2026-04-17-1443-UTC
commit: uncommitted (no commits yet on master)
agent: tester
verdict: FAIL
---

# Test Report — v0-paper-sma-week1 — 2026-04-17 14:43 UTC

## 1. Scope

- **Feature / change under test:** v0 Paper-Trading SMA Tracer Bullet — Week 1 foundation tasks T01–T20 (developer tasks T01–T12, ui-designer tasks T13–T20). Week 2 tasks (T21+) have not been started and are explicitly out of scope for this run.
- **Spec refs:** `spec/features/v0-paper-sma.md`, `spec/tasks/v0-paper-sma.md`
- **Commit SHA:** `uncommitted` — repository has no commits yet (`fatal: your current branch 'master' does not have any commits yet`)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)` / `cargo 1.94.1`
- **OS / arch:** `Darwin 25.4.0 arm64`

---

## 2. Static Analysis

| Check               | Result | Notes                                                                                       |
|---------------------|--------|---------------------------------------------------------------------------------------------|
| `cargo fmt --check` | PASS   | No diff output; exit 0.                                                                     |
| `cargo clippy`      | PASS   | 0 warnings, 0 errors. `--workspace --all-targets --all-features -- -D warnings` clean.      |
| `cargo audit`       | SKIP   | `cargo-audit` not installed. Not installing per instructions.                               |
| `cargo deny check`  | PASS\* | `advisories ok, bans ok, licenses ok, sources ok`. 1 `warning[no-license-field]` for `polars-arrow-format v0.1.0` (unlicensed transitive dep from polars). Several `warning[duplicate]` entries (base64, bitflags, rand, rand_chacha, getrandom, hashbrown — 27 total duplicate warnings). No errors. |

\* `cargo deny` exits 0 (warnings-only); the duplicate crate warnings are informational and expected with a large transitive dep graph.

---

## 3. Unit & Integration Tests

### `cargo test --workspace --all-targets`

| Crate / Target              | Passed | Failed | Ignored | Notes                             |
|-----------------------------|-------:|-------:|--------:|-----------------------------------|
| `agent` (main.rs unit)      |      7 |      0 |       0 | T12 config tests                  |
| `audit` (lib unit)          |      0 |      0 |       0 | Stub only                         |
| `audit` (ledger_integration)|      5 |      0 |       0 | T05 + T06 acceptance              |
| `backtest` (lib unit)       |      0 |      0 |       0 | Stub only                         |
| `backtest` (bin unit)       |      0 |      0 |       0 | Stub only                         |
| `core` (lib unit)           |      6 |      0 |       0 | T02/T04 order invariants          |
| `core` (trybuild_test)      |      1 |      0 |       0 | T03 compile-fail                  |
| `core` (types_test)         |     20 |      0 |       0 | T02 serde round-trips             |
| `cost` (lib unit)           |      0 |      0 |       0 | Stub only                         |
| `data` (lib unit)           |      8 |      0 |       0 | T10 FakeFeed + T11 clock-skew     |
| `exec` (lib unit)           |      0 |      0 |       0 | Stub only                         |
| `features` (lib unit)       |      0 |      0 |       0 | Stub only                         |
| `llm` (lib unit)            |      0 |      0 |       0 | Stub only                         |
| `models` (lib unit)         |      0 |      0 |       0 | Stub only                         |
| `risk` (lib unit)           |      0 |      0 |       0 | Stub only                         |
| `strategy` (lib unit)       |      0 |      0 |       0 | Stub only                         |
| `ui` (lib unit)             |     17 |      0 |       0 | T13–T20 widget + state unit tests |
| `ui` (cockpit bin unit)     |      0 |      0 |       0 | —                                 |
| `ui` (consistency.rs)       |      2 |      0 |       0 | Design-system consistency guards  |
| `ui` (panel_snapshots.rs)   |     24 |      0 |       0 | 24 insta snapshot tests           |
| **Total**                   | **90** |  **0** |     **0**|                                   |

### `cargo test --workspace --doc`

**FAIL** — `cargo test --workspace --doc` exits with error code 1.

The `core` crate's doc tests fail because `rustdoc` runs doc-test binaries in a context where the crate's own name (`core`) shadows the stdlib `core` crate, causing `thiserror`-generated `::core::fmt`, `::core::write`, `::core::convert`, `::core::option` symbols to be unresolvable. First 20 lines of error verbatim:

```
   Doc-tests core
error[E0433]: failed to resolve: could not find `write` in `core`
 --> crates/core/src/error.rs:6:39
  |
6 | #[derive(Debug, Clone, PartialEq, Eq, Error)]
  |                                       ^^^^^ could not find `write` in `core`
  |
  = note: this error originates in the derive macro `Error` (in Nightly builds, run with -Z macro-backtrace for more info)

error[E0433]: failed to resolve: could not find `write` in `core`
  --> crates/core/src/error.rs:13:39
   |
13 | #[derive(Debug, Clone, PartialEq, Eq, Error)]
   |                                       ^^^^^ could not find `write` in `core`
   |
   = note: this error originates in the derive macro `Error` (in Nightly builds, run with -Z macro-backtrace for more info)

error: aborting due to 24 previous errors
error: doctest failed, to rerun pass `-p core --doc`
```

The developer mitigated this by setting `doctest = false` in `[lib]` of `crates/core/Cargo.toml`. This prevents `cargo test -p core` from exercising doc tests, and the workspace `--all-targets` run also skips them under that flag. However `cargo test --workspace --doc` bypasses the `doctest = false` flag and runs the doc-test harness directly, exposing the underlying hazard. **This is a failing gate.**

### Reconciliation R-B: `cargo test -p audit`

Isolated run result:

```
Finished `test` profile [unoptimized + debuginfo] target(s) in 2.16s
Running unittests src/lib.rs
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured

Running tests/ledger_integration.rs
running 5 tests
test t05_account_list_returns_all_v0_accounts ... ok
test t05_cash_balance_after_buy_fill ... ok
test t05_bootstrap_is_idempotent ... ok
test t06_global_debit_credit_equality ... ok
test t06_100_fills_all_transactions_balance ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured
```

**`cargo test -p audit` is green. The developer's claim is correct.** The ui-designer's report that `cargo test -p audit` is blocked by a `core::` shadow inside `crates/audit/src/query.rs` is out of date: the developer already applied the `trading_core = { package = "core", path = "../core" }` alias to `crates/audit/Cargo.toml`, and `query.rs` imports via `trading_core::`. The ui-designer wrote their notes before the developer's alias fix was propagated to all consumer crates.

### Failing Tests

| Failure | Target | Root cause |
|---------|--------|------------|
| `cargo test --workspace --doc` (core doc tests) | `cargo test --workspace --doc` | `doctest = false` workaround does not protect against the `--doc` workspace flag; `thiserror`-generated code emits `::core::fmt` etc. which shadow-resolves to the `core` crate itself. 24 `E0433` errors. |

---

## 4. Property / Fuzz Tests

`PROPTEST_CASES=1024 cargo test --workspace property_` was run. No tests with the `property_` prefix exist in the workspace — proptest suites in `core` use names like `prop_zero_qty_rejected`, `prop_positive_qty_accepted`, `prop_exposure_cap` (no `property_` prefix). All were exercised as part of `cargo test --workspace --all-targets`.

`PROPTEST_CASES=1000 cargo test -p core` run confirmed: all 3 proptest cases pass (T04 acceptance satisfied).

| Suite | Cases | Shrunk failures | Seed |
|-------|------:|----------------:|------|
| `core::tests::order_tests::prop_zero_qty_rejected` | 1000 | 0 | default |
| `core::tests::order_tests::prop_positive_qty_accepted` | 1000 | 0 | default |
| `core::tests::order_tests::prop_exposure_cap` | 1000 | 0 | default |

---

## 5. Backtest Results

_n/a — Week 2 scope (T21+); this run validates foundation only. No backtest binary, no matching engine, no strategy, and no Parquet fixtures exist yet. Section will be populated after T25 / T_FINAL_A land._

---

## 6. Benchmarks

_n/a — Week 2 scope (T21+); this run validates foundation only. No criterion suites exist. Section will be populated after hot-path work lands._

---

## 7. Environment / Infrastructure Issues

### Reconciliation R-A: `core` crate name collision with Rust 2024

**Finding:** The `core` crate package name is a workspace-wide hazard, but it has been systematically mitigated — not fixed. Evidence:

1. **All consumer crates** (`audit`, `data`, `risk`, `exec`, `strategy`, `cost`, `features`, `models`, `llm`, `agent`, `backtest`, `ui`) use the `trading_core = { package = "core", path = "../core" }` alias in their `Cargo.toml`. Source imports use `trading_core::`. `cargo test --workspace --all-targets` is green across all 13 crates.

2. **Integration tests inside `core`** (`tests/types_test.rs`, `tests/trybuild_test.rs`) use `use core::…` directly. This works because in the integration-test context `core` resolves to the crate-under-test. This is by design and non-hazardous.

3. **Doc tests in `core`** cannot be mitigated by the alias — the `rustdoc` harness expands `thiserror` macros that emit `::core::fmt`/`::core::write`/`::core::convert`/`::core::option`, which resolve to the local `core` crate, not the stdlib. `doctest = false` in `[lib]` prevents `cargo test -p core` from failing, but `cargo test --workspace --doc` bypasses this setting and fails with 24 `E0433` errors. **This is the live failure.**

4. A future crate that imports `core` without the alias (easy to forget in Week 2 as T21–T31 expand the crate graph) will hit the same error at compile time.

**Recommendation:** Rename the crate from `package = "core"` to `package = "trading_core"` workspace-wide. This is a search-and-replace across 12 `Cargo.toml` files and 13 source trees — purely mechanical, no API change. Remove the `trading_core = { package = "core" }` indirection; the name becomes `trading_core` everywhere. This is **architect territory** because it affects the crate identity contract described in `spec/architecture.md`. Until the rename lands:

- Add `doctest = false` protection is insufficient — `cargo test --workspace --doc` still fails.
- Every new Week 2 crate must remember to use the `trading_core` alias.

**Verdict on R-A:** Real workspace hazard requiring a rename decision. Route to **architect**.

---

### T03 Acceptance Criterion Gap

The task spec says: `cargo test -p core --test trybuild`. The actual test target is named `trybuild_test` (not `trybuild`), so the documented command fails with:

```
error: no test target named `trybuild` in `core` package
help: available test targets:
    trybuild_test
    types_test
```

The test itself passes when invoked as `cargo test -p core --test trybuild_test`. However the task's acceptance criterion text is wrong. Additionally, T03 states three compile-fail scenarios (cross-currency money, negative Quantity without Result, private Order fields), but only **one** compile-fail file exists (`tests/compile_fail/money_cross_currency.rs`). The negative-Quantity and private-Order-fields scenarios are not covered. The `compile_fail_tests` harness is declared as `[x]` but covers only 1/3 of its stated scope.

### T05 Account Count Discrepancy

Task T05 acceptance criterion states "10 v0 accounts." The integration test header (`ledger_integration.rs:4`) says "all 11 v0 accounts." The feature spec chart (`spec/features/v0-paper-sma.md → Chart of accounts`) actually lists 12 accounts. The actual implementation bootstraps and tests **11** accounts. The `equity:opening_balance` account exists in the test but is not in the R3.2 bullet list in the feature spec (though it is in the chart-of-accounts code block). Minor spec drift; test passes with 11 accounts.

### T05: `sqlx-ledger` Substitution

Developer correctly documented that `sqlx-ledger` is Postgres-only and substituted hand-rolled `sqlx` + SQLite. This deviates from the architect's specified dependency. The deviation is documented in `spec/reports/dev-week1-notes-2026-04-17.md` and requires architect acknowledgment before Week 2 proceeds.

### T08: Integration Test Absent

T08 is marked `[x]` with acceptance criterion: "integration test against Binance public WS receives at least one kline and one trade within 30s; reconnect drill recovers within 5s." No such test exists in `crates/data/`. The `BinanceFeed` implementation is present and compiles, but the acceptance criterion has not been verified. Marking this `_deferred_manual_` (live venue; cannot run in sandbox). However, the task checkbox should not be `[x]` if the automated criterion has not been met — it is a handoff contract violation.

### T09: Integration Test Absent

T09 is marked `[x]` with acceptance criterion: "1-hour fixture replay emits exactly 60 bars with monotonically increasing `venue_ts`." No such test exists. `ReplayFeed` implementation is present (289 lines), but the acceptance criterion has not been verified. No 1-hour Parquet fixture exists in `data/binance/BTCUSDT/`. Marking this `_deferred_manual_` until data fixtures land, but the `[x]` checkbox is premature.

### T11 Dependency Note

T11 in the task list lists `[deps: T08, T27]`. T27 is a Week 2 task (`agent::KillSwitch`). The T11 clock-skew implementation passes its own unit tests (which inject fake timestamps and assert trip behavior via a mock kill-switch callback), but the real `agent::KillSwitch` integration is deferred. The unit tests are sufficient for Week 1 scope.

### UI Smoke (Cockpit Binary)

`cargo build -p ui --bin cockpit --features fixtures` — PASS (2.52s). Actual launch cannot be run in sandbox (no display server). Deferred as manual step per instructions.

---

## 8. Verdict

**`FAIL`**

`cargo test --workspace --doc` exits with exit code 1 due to 24 `E0433` compile errors in the `core` crate's doc-test harness. The `doctest = false` workaround in `crates/core/Cargo.toml` is insufficient when the `--doc` flag is passed at workspace level. This is a direct consequence of the `core` crate name collision (R-A). All 90 unit and integration tests pass. Both consistency tests pass. All 24 snapshot tests pass. The `cargo test -p audit` isolated run is green (developer's claim is correct; ui-designer's claim of blockage is obsolete). The static analysis gates (fmt, clippy, deny) all pass. However, two task checkboxes (`T08`, `T09`) are marked `[x]` with unmet acceptance criteria (integration tests absent), and the `core` crate rename decision needs architect input to permanently fix the doc-test failure. Until `cargo test --workspace --doc` is green the workspace does not satisfy the full test gate.

---

## 9. Routing

`HANDOFF → developer` — fix `cargo test --workspace --doc` (either rename `core` to `trading_core` as the architect decides, or add `doc_test = false` suppression at the workspace-doc level); add the 2 missing `trybuild` compile-fail cases for T03; mark T08 and T09 as `[~]` (partial) in the task list pending live/fixture integration tests.

**Other agents to loop in:**
- **architect** — `core` crate rename decision (R-A) is a workspace-identity choice that belongs in `spec/architecture.md`; the `sqlx-ledger` → raw-sqlx substitution also needs sign-off.

---

## Reconciliation Items

### R-A: `core` crate name collision — real-hazard

The alias workaround (`trading_core = { package = "core" }`) is applied consistently across all 12 consumer crates and `cargo test --workspace --all-targets` is green. However, the collision is a real workspace hazard: `cargo test --workspace --doc` fails with 24 errors, and any future crate that omits the alias will fail to compile. The `doctest = false` per-crate workaround does not protect the workspace-doc path. **Recommendation: rename `crates/core` package to `trading_core`** — architect decision required.

### R-B: `audit` test health — developer-correct

`cargo test -p audit` is fully green (5/5 integration tests pass). The ui-designer's claim that `audit` tests are blocked by a `core::` shadow is no longer accurate: the developer applied the `trading_core` alias to `crates/audit/Cargo.toml` and `query.rs` imports `trading_core::` consistently. The ui-designer's notes reflected an earlier state of the workspace that was subsequently fixed.
