---
title: Test Report
feature: audit-tick-consumer-envelope
run_id: 2026-05-20-T1200-UTC
commit: ea07934
agent: tester
verdict: PASS
---

# Test Report — audit-tick-consumer-envelope — 2026-05-20

## 1. Scope

- **Feature / change under test:** `audit-tick-consumer-envelope` v0.1.0 — broadcast tee over audit journal (T-D-1..T-D-25 M-FINAL gate)
- **Spec refs:** `spec/audit-tick-consumer-envelope/feature.md`, `spec/audit-tick-consumer-envelope/tasks.md`
- **Commit SHA:** `ea07934`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** Darwin arm64

## 2. Static Analysis

| Check                                              | Result    | Notes                                                                 |
|----------------------------------------------------|-----------|-----------------------------------------------------------------------|
| `cargo fmt --check`                                | **PASS**  | Exit 0; no format diffs                                               |
| `cargo clippy --workspace -- -D warnings`          | **PASS**  | Exit 0; no lib warnings                                               |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **FAIL (pre-existing)** | 4 `doc_markdown` errors in `crates/audit/src/bootstrap.rs:115` (1) and `crates/audit/src/journal.rs:2292,2327` (3). All 4 introduced in commit `2112d69` (cockpit-training-control Wave D, 2026-05-19), NOT by this feature. Zero audit-tick-consumer-envelope source files in the error set. Developer gate ran `--workspace -- -D warnings` (no `--all-targets`) per decomp §10; the skill spec prescribes `--all-targets`. Recorded as pre-existing debt (see §9). |
| `cargo audit`                                      | N/A       | `cargo-audit` not installed; skipped per skill policy                 |
| `cargo deny`                                       | N/A       | Not in scope for this feature                                         |
| `grep -rn 'barter' Cargo.toml crates/`             | **PASS**  | One hit — a doc comment in `crates/audit/src/tick.rs:29` ("Mirrors the barter-rs shape — no crate dep"). Zero `barter` Cargo.toml entries. R1.4 confirmed. |

## 3. Unit & Integration Tests

### Full workspace (`cargo test --workspace`)

| Crate / Suite                                  | Passed | Failed | Ignored | Notes                                   |
|------------------------------------------------|-------:|-------:|--------:|-----------------------------------------|
| `agent` (lib + integration)                    |     65 |      0 |       0 | All 50 unit + 15 integration pass       |
| `audit` (lib)                                  |     36 |      0 |       0 |                                         |
| `audit` (integration tests, existing)          |    110 |      0 |       0 | All pre-existing journal/ledger tests pass |
| `audit` (new tick_* tests)                     |     20 |      0 |       0 | tick_event_size(1)+variant_coverage(7)+lag_drop(2)+run_id(2)+serde_roundtrip(8) |
| `backtest`                                     |     38 |      0 |       1 | 1 ignored (requires config/strategies at cwd) |
| `cost`                                         |     14 |      0 |       0 |                                         |
| `data`                                         |     65 |      0 |       4 | 3 ignored (live WS); 1 ignored (real parquet) |
| `exec`                                         |      9 |      0 |       0 |                                         |
| `features`                                     |     55 |      0 |       0 |                                         |
| `forecast`                                     |     79 |      0 |       0 |                                         |
| `reflection` (lib + new test)                  |      6 |      0 |       0 | includes audit_tick_consumer_stub (2)   |
| `risk`                                         |     24 |      0 |       0 |                                         |
| `strategy`                                     |     83 |      0 |       0 |                                         |
| `ui` (lib + most integration)                  |    792 |      0 |       0 |                                         |
| `ui --test consistency`                        |      1 |      1 |       0 | **PRE-EXISTING** — `no_inline_user_visible_strings_in_widgets` FAIL from `chart.rs:190` `"UI_CHART_FORCE_UTC"` literal; introduced in commit `f5fec84` (`ship(chart-x-axis-local-time): v1.11.0`) which precedes audit-tick by 13 commits. `ea07934` did not touch `chart.rs` (verified via `git diff`). |
| **Total**                                      | **1422** |  **1** |     **5** | 1 failure is pre-existing (non-feature) |

### Failing Tests

**`no_inline_user_visible_strings_in_widgets`** (`crates/ui/tests/consistency.rs:216`)

Pre-existing regression from `f5fec84` (`ship(chart-x-axis-local-time)`). The string literal `"UI_CHART_FORCE_UTC"` at `crates/ui/src/widgets/chart.rs:190` was not routed via `ui::strings`. The `audit-tick-consumer-envelope` feature did not touch any UI code; this is confirmed by `git diff f5fec84..ea07934 -- crates/ui/` returning empty output. This failure existed on `main` before the audit-tick developer began work.

## 4. Property / Fuzz Tests

_n/a_ — `proptest` suites present in `crates/features` all pass (included in crate totals above). No fuzz targets introduced or in scope.

## 5. Backtest Results

_n/a_ — This feature is backend process-tooling (additive broadcast tee). No strategy logic was modified. The 22 body-SHA anchors verify backtest regression independently (see §7).

## 6. Benchmarks

_n/a_ — `crates/audit/benches/tick_send_latency.rs` was added (T-D-23, optional) to measure `Sender::send` p99 with 0..16 subscribers. Not gated at M-FINAL per decomp §7. No baseline to diff against.

## 7. Anchor Verification (verify-anchors — MANDATORY)

`scripts/verify_anchors.sh` run: **ANCHORS PASS (22 / 22)**

All 22 body-SHA-256 anchors byte-identical:

| Scenario | SHA (first 8 chars) | Result |
|---|---|---|
| btc-2023-1m-sma-cross | fc2e3b4a | PASS |
| btc-2023-1m-sma-baseline-refresh | fc2e3b4a | PASS |
| btc-2023-1m-macd-trend | ef9c5e48 | PASS |
| btc-2023-1m-rsi-reversion | bc56d20d | PASS |
| btc-2023-1m-bbands-mean-revert | d8a08a23 | PASS |
| top10-2023-1h-momentum | 3b60ef07 | PASS |
| top10-2024-h1-momentum | 1f33534f | PASS |
| pairs-2023-zscore-mr | 90591a0e | PASS |
| pairs-2024-h1-zscore-mr | 14f50a59 | PASS |
| report-sample-7d | 520b1f29 | PASS |
| report-sample-90d | c656414e | PASS |
| top10-2023-fy-tcn-overlay | 01d02584 | PASS |
| top10-2024-fy-tcn-overlay | e24c85ac | PASS |
| top10-2023-fy-tcn-overlay-weights | 7cb1357c | PASS |
| top10-2024-fy-tcn-overlay-weights | 23c24dae | PASS |
| top10-2023-fy-tcn-overlay-realdata | 8fa47f49 | PASS |
| top10-2024-fy-tcn-overlay-realdata | fd8191df | PASS |
| top10-2023-fy-tcn-overlay-weights-realdata | 552d7df2 | PASS |
| top10-2024-fy-tcn-overlay-weights-realdata | 2a65c4347 | PASS |
| forecast-distribution-bs1-realdata | ef73cb8d | PASS |
| forecast-distribution-bs2-realdata | d7cd08e6 | PASS |
| sharpe-comparison-realdata | 17d2e96c | PASS |

R5.1 satisfied: additive read-side feature left all 22 backtest report bodies byte-identical.

## 8. Per-Feature New Test Verification

Each new test file run individually:

| Test command | Result | Tests |
|---|---|---|
| `cargo test -p audit --test tick_event_size` | **PASS** | 1 passed (`audit_event_size_within_budget`) |
| `cargo test -p audit --test tick_variant_coverage` | **PASS** | 7 passed (all 6 non-delegating writers + Hold fast-return) |
| `cargo test -p audit --test tick_lag_drop --release` | **PASS** | 2 passed (`producer_never_blocks`, `slow_consumer_sees_lagged_error`) |
| `cargo test -p audit --test tick_run_id` | **PASS** | 2 passed (`base_ledger_run_id_is_nil`, `with_run_id_stamps_distinct_ids`) |
| `cargo test -p audit --test tick_serde_roundtrip` | **PASS** | 8 passed (one per AuditEvent variant) |
| `cargo test -p reflection --test audit_tick_consumer_stub` | **PASS** | 2 passed (`stub_terminates_immediately_when_no_ticks`, `stub_receives_fill_tick`) |
| **Total new tests** | **PASS** | **22 passed; 0 failed** |

Note: `tick_variant_coverage` emits one compile-time warning (`unused import: Decision`) — not a clippy failure, no `#[deny]` in scope.

## 9. Spec-Lint Gate

```
spec-lint: FAIL (87 violations in 2 categories)
```

| Category | This run | Previous baseline (cockpit-training-control tester 2026-05-19) | Delta | New from audit-tick? |
|---|---|---|---|---|
| dead-link | 81 | 730 | -649 (improvement) | 0 |
| trace-broken-path | 6 | 6 | 0 | 0 |
| **TOTAL** | **87** | **736** | **-649** | **0** |

Feature contribution = **0** violations. The 87 remaining are all pre-existing. Zero hits from `spec/audit-tick-consumer-envelope/` in lint output (verified: `grep "audit-tick"` against lint output returned empty). The large dead-link decrease (-649) is from prior spec-hygiene work in commits `9c4e58b` and `c014760` which archved old feature links.

**Pre-existing spec debt (carry-forward):**
- 81 dead-links in `spec/architecture/adr/`, `spec/chart-*`, `spec/dev-notes/`, `spec/iced-*`, `spec/journal-transactions-metadata/`, `spec/lumen-design-adoption/`, `spec/v0-paper-sma/`, `spec/v05-*`, `spec/v1-*`, `spec/v15a-*`, `spec/v2-*`, `spec/v25-*` — all pre-existing.
- 6 trace-broken-path: `REQ-V25A-PATCHTST-001`, `REQ-V25B-TRANSFORMER-001`, `REQ-V26-BAKEOFF-001` — pre-existing from unrealised future feature anchors.

Per tester gate rules: pre-existing baseline violations do NOT block PASS; feature contribution = 0 satisfies the gate.

## 10. Pre-existing Debt (carry-forward from prior sprints)

These items are NOT introduced by `audit-tick-consumer-envelope`. Documented for visibility per tester gate rules:

1. **`cargo clippy --all-targets` failures (4 violations):** `doc_markdown` in `crates/audit/src/bootstrap.rs:115` and `crates/audit/src/journal.rs:2292,2327` — introduced in commit `2112d69` (cockpit-training-control Wave D). Developer gate ran without `--all-targets`. Routing: `HANDOFF → developer` for a follow-up cosmetic fix.
2. **`no_inline_user_visible_strings_in_widgets` test failure:** `crates/ui/src/widgets/chart.rs:190` `"UI_CHART_FORCE_UTC"` — introduced in commit `f5fec84` (`ship(chart-x-axis-local-time)`). Routing: should have been caught by chart-x-axis tester; carry-forward.

## 11. Open Questions (developer-flagged, non-blocking)

1. **T-D-14 deviation:** `TcnForecaster::with_ledger()` wiring is architecturally blocked — forecasters are constructed inside `crates/strategy` from TOML config; `Ledger` never reaches the forecaster at runtime. Feature chain is wired at compile time. Runtime wiring requires an architect design item (strategy crate accepting optional `Ledger` handle via config). OPEN for follow-up.
2. **T-D-12 deviation:** `audit` dep in `crates/forecast/Cargo.toml` kept required (not optional) because `train_tcn` bin uses it unconditionally. Only the `audit-tick = []` feature flag was added. Confirmed not a build regression.

## 12. Environment / Infrastructure Issues

_none_ — all standard toolchain, no external dependencies accessed, SQLite in-memory tests only.

## 13. Verdict

**`PASS`**

All 6 new test files pass (22 new test cases). The 22 body-SHA-256 anchors are byte-identical (22/22 PASS). `cargo fmt --check` exit 0. `cargo clippy --workspace -- -D warnings` exit 0. Zero `barter` Cargo dependencies. Spec-lint feature contribution = 0. The one workspace test failure (`no_inline_user_visible_strings_in_widgets`) and 4 clippy `--all-targets` errors are pre-existing regressions from prior sprints (commits `f5fec84` and `2112d69` respectively), confirmed by `git blame` and `git diff`. Neither touches `audit-tick-consumer-envelope` files.

## 14. Routing

`VERDICT → PASS` — ready for presenter (sprint-review deck for operator approval).

Pre-existing debt routing (parallel, non-blocking for this verdict):
- `HANDOFF → developer` for doc_markdown clippy fix in `bootstrap.rs` + `journal.rs` (cosmetic, no urgency).
- `HANDOFF → developer` for `no_inline_user_visible_strings_in_widgets` in `chart.rs:190` (route `"UI_CHART_FORCE_UTC"` through `ui::strings`).
