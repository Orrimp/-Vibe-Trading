---
slug: audit-tick-consumer-envelope
mode: release
status: approved
audience: human-operator
updated: 2026-05-20
generated: 2026-05-20T14:00:00Z
---

# Audit tick consumer envelope — release

## TL;DR

- Process-tooling broadcast envelope is live: every relevant audit-journal
  write now also publishes a typed `AuditTick` on an in-memory
  `tokio::sync::broadcast` channel. Future consumers subscribe to that
  channel instead of patching each writer.
- **Default behaviour is unchanged.** The tee is opt-in at `Ledger`
  construction (`Ledger::open_with_tick_bus`); the existing
  `Ledger::open` path is bit-identical to pre-feature.
- **22 / 22 body-SHA-256 anchors PASS** — the additive read-side tee did
  not perturb a single rendered backtest or report byte.
- The reflection-side consumer (`ReflectionAuditTickConsumer`) ships as an
  **observation-only stub** gated by `[reflection]
  audit_tick_consumer_enabled = false` — production lesson writing still
  flows through the existing mpsc tap.
- Q1..Q5 were operator-decided on 2026-05-20 to the analyst defaults
  via "Autoapprove all".

## What changed

- **New module `crates/audit/src/tick.rs`** — defines `AuditTick<E, C>`,
  `AuditContext { run_id, posted_at, agent_pid }`, and the
  `#[non_exhaustive]` `AuditEvent` enum (8 variants: `Fill`,
  `StrategySignal`, `StrategyEvent`, `ForecastEmitted`,
  `KillSwitchTripped`, `FeedReconnect`, `UptimeIntervalOpened`,
  `UptimeIntervalClosed`). Plus a thin `AuditTickStream` consumer
  newtype with async `next()` and a blocking-iter adaptor.
- **New opt-in constructor `Ledger::open_with_tick_bus(path,
  capacity)`** plus `with_run_id(uuid)` / test-only `with_pid(pid)`
  builders. Default `Ledger::open` produces no tick bus → no tee → zero
  behaviour change by construction.
- **Post-commit tee in 6 in-scope `journal::*` writers** (`post_fill`,
  `post_strategy_signal`, `kill_switch_tripped`, `strategy_event` —
  which transitively covers `feed_reconnect`, `rebalance_rejected`,
  `mean_reversion_stop`, `pair_short_observation` — plus
  `open_uptime_interval` and `close_uptime_interval`). When no bus is
  attached the tee is a single-branch noop; when attached it is a
  non-blocking `Sender::send` after the SQL commit.
- **Feature-gated `forecast → audit` edge** — `TcnForecaster::with_ledger`
  + two `ForecastEmitted` emit sites land behind the
  `forecast/audit-tick` Cargo feature. Compile-time chain is wired
  (`agent → strategy → forecast`); runtime wiring is parked as a
  follow-up design item (see Open decisions §1).
- **New `crates/reflection/src/audit_tick_consumer.rs` stub** that
  subscribes, logs, and counts variants — does not write any
  `LessonCard`s yet. The existing `ReflectionWriter` (mpsc tap) is
  untouched and remains the v2.x production path.
- **Config additions** (serde-defaulted, backward-compatible): `[audit]
  tick_bus_capacity = 1024` and `[reflection]
  audit_tick_consumer_enabled = false`.

No UI surface, no schema migration, no new external crate dependency
(no `barter-rs` — confirmed by `grep -rn 'barter' Cargo.toml crates/` →
0 hits).

## Why

The audit journal previously used **per-consumer write taps** — every
new consumer (reflection, the future Lab Trail screen, v2.6 bake-off,
v3 success-reports) required its own mutation of writers in
`crates/audit`, `crates/exec`, and `crates/agent`. As the consumer
count grows, that surface stops stabilising. The envelope (canonical
design in [ADR-0031](../../architecture/adr/0031-audit-tick-consumer-envelope.md))
inverts the pattern: producers tee once, consumers subscribe.
Implementation is additive — the double-entry ledger and SQLite schema
are untouched. See [feature.md](../feature.md) "Why" for the full
rationale.

## What you can do now

| Action | Command |
|--------|---------|
| Run the journal-side variant-coverage suite (proves all 6 tees fire) | `cargo test -p audit --test tick_variant_coverage` |
| Run the lag/backpressure suite (H3 — slow consumer drops, producer never blocks) | `cargo test -p audit --test tick_lag_drop --release` |
| Run the per-run-id distinctness suite (K4) | `cargo test -p audit --test tick_run_id` |
| Run the reflection-side stub end-to-end | `cargo test -p reflection --test audit_tick_consumer_stub` |
| Verify backtest/report anchors are byte-identical | `bash scripts/verify_anchors.sh` |
| Enable the reflection observation stub in a side experiment | edit `config/agent.toml` → `[reflection] audit_tick_consumer_enabled = true`; restart agent |

## Live demo

This is a backend-only feature with no UI surface and no operator-facing
CLI. Ground-truth evidence is the new test surface + the anchor gate.
Both run as part of M-FINAL and were re-verified at presentation time.

### Anchor gate (proves: tee is byte-neutral over rendered reports)

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
PASS  top10-2023-fy-tcn-overlay             01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5
PASS  top10-2024-fy-tcn-overlay             e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163
PASS  top10-2023-fy-tcn-overlay-weights     7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4
PASS  top10-2024-fy-tcn-overlay-weights     23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b
PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
PASS  top10-2024-fy-tcn-overlay-realdata    fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3
...
ANCHORS PASS  (22 / 22)
```

Interpretation: all 22 body-SHA-256 anchors are unchanged. The
post-commit broadcast tee does not alter any rendered backtest or
report byte — H2 holds.

### New test surface (proves: the envelope works end to end)

```
$ cargo test -p audit --test tick_variant_coverage
running 7 tests
test post_fill_emits_fill_variant ... ok
test post_strategy_signal_emits_strategy_signal_variant ... ok
test post_strategy_signal_hold_emits_no_tick ... ok
test kill_switch_tripped_emits_kill_switch_variant ... ok
test strategy_event_emits_strategy_event_variant ... ok
test open_uptime_interval_emits_uptime_opened_variant ... ok
test close_uptime_interval_emits_uptime_closed_variant ... ok
test result: ok. 7 passed; 0 failed; 0 ignored

$ cargo test -p audit --test tick_serde_roundtrip
test result: ok. 8 passed; 0 failed; 0 ignored
                # one roundtrip per AuditEvent variant

$ cargo test -p audit --test tick_run_id
test result: ok. 2 passed; 0 failed; 0 ignored

$ cargo test -p reflection --test audit_tick_consumer_stub
test result: ok. 2 passed; 0 failed; 0 ignored
```

Note: `cockpit-smoke` was intentionally skipped — this feature touches
no UI code (verified via `git diff` against `crates/ui/`). Full
artifact: `presentations/artifacts/audit-tick-consumer-envelope-2026-05-20/live-evidence.txt`.

## Screenshots

_n/a — backend-only feature. No iced widgets, no chart, no cockpit
panel touched by this brief. The UI consumer (Lab Trail) ships in its
own follow-up brief per Q4._

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 (R1, H5) | `AuditTick` / `AuditContext` / `AuditEvent` shape + 256B size budget | VERIFIED | `cargo test -p audit --test tick_event_size` → `audit_event_size_within_budget ... ok` |
| V2 (R2) | Post-commit tee in 6 in-scope writers; per-writer variant fires once | VERIFIED | `cargo test -p audit --test tick_variant_coverage` → 7 passed (6 variant tests + 1 Hold fast-return) |
| V3 (R3, H3, K1) | Lagging consumer observes `Lagged(_)`; producer per-send p99 ≤ 10µs | VERIFIED | `cargo test -p audit --test tick_lag_drop --release` → 2 passed |
| V4 (R4) | Reflection observation stub subscribes, processes a fill tick, terminates on sender drop | VERIFIED | `cargo test -p reflection --test audit_tick_consumer_stub` → 2 passed |
| V5 (R5.1, H2) | All 22 body-SHA-256 anchors byte-identical (non-regression) | VERIFIED | `scripts/verify_anchors.sh` → `ANCHORS PASS (22 / 22)` |
| V6 (K4) | Per-`Ledger`-clone `run_id` distinctness across concurrent backtests | VERIFIED | `cargo test -p audit --test tick_run_id` → 2 passed (`base_ledger_run_id_is_nil`, `with_run_id_stamps_distinct_ids_per_clone`) |
| V7 (R1.1, R1.3) | All 8 `AuditEvent` variants serde-roundtrip cleanly under `#[non_exhaustive]` | VERIFIED | `cargo test -p audit --test tick_serde_roundtrip` → 8 passed |
| V8 (R1.4, NR#6) | No `barter-rs` Cargo dep introduced; shape only | VERIFIED | tester: `grep -rn 'barter' Cargo.toml crates/` → 0 Cargo hits; 1 doc-only hit in `tick.rs:29` |
| V9 (gate) | `cargo fmt --check` exit 0 | VERIFIED | `test-final-2026-05-20.md` §2 |
| V10 (gate) | `cargo clippy --workspace -- -D warnings` exit 0 | VERIFIED | `test-final-2026-05-20.md` §2 |
| V11 (gate) | `cargo test --workspace` 100% PASS (excluding pre-existing failures) | VERIFIED | 1422 / 1 / 5 — the 1 failure (`no_inline_user_visible_strings_in_widgets`) pre-dates this feature by 13 commits (`f5fec84`); `git diff` shows `audit-tick` touched no UI |
| V12 (gate) | Spec-lint feature contribution = 0 | VERIFIED | 87 violations / 2 categories — identical to baseline carry-forward; `grep "audit-tick"` against lint output → empty |

All 12 verification rows are VERIFIED. Test report:
[reports/test-final-2026-05-20.md](../reports/test-final-2026-05-20.md)
verdict = `PASS`.

## Architecture changes

- **[ADR-0031](../../architecture/adr/0031-audit-tick-consumer-envelope.md)**
  status flipped `proposed` → `accepted` at M-T1; new `refined-by` /
  `decomposed-by` cross-links added to feature.md and decomp.md.
- **`spec/architecture/01-data-flow.md`** edge table gained two rows:
  - `reflection → audit (via AuditTick stream)` — read-only, in-memory
  - `forecast → audit (audit-tick feature-gated)` — read-only, in-memory
- **New public surface in `crates/audit`**: `tick::AuditTick<E, C>`,
  `tick::AuditContext`, `tick::AuditEvent` (`#[non_exhaustive]`),
  `tick::AuditTickStream`, `tick::emit_public`, `Ledger::open_with_tick_bus`,
  `Ledger::with_run_id`. The existing `Ledger::open` / `Ledger::in_memory`
  / `Ledger::pool` surface is preserved bit-identically.
- **Trace row `REQ-AUDIT-TICK-001`** advanced `proposed → accepted →
  tested` across the M-T1 / M-FINAL gates; `arch[]`, `tests[]`,
  `anchors[]` populated.

Import-direction invariant from `01-data-flow.md` upheld:
`crates/audit::tick` imports nothing from sibling crates; payloads use
`trading_core` types only (verified at compile time).

## Numbers that matter

- **New tests:** 22 cases across 6 new test files
  (`tick_event_size` × 1, `tick_variant_coverage` × 7, `tick_lag_drop`
  × 2, `tick_run_id` × 2, `tick_serde_roundtrip` × 8,
  `audit_tick_consumer_stub` × 2).
- **Workspace test totals:** 1422 passed / 1 failed / 5 ignored. The
  single failure is the pre-existing
  `no_inline_user_visible_strings_in_widgets` from commit `f5fec84`
  (chart-x-axis-local-time, 13 commits prior); `audit-tick` touched no
  UI code.
- **Anchors:** 22 / 22 PASS (byte-identical).
- **Spec-lint:** 87 violations in 2 categories. **Feature contribution
  = 0.** Baseline is carry-forward; no regression introduced.
- **Variant coverage:** 8 `AuditEvent` variants × 8 serde roundtrip
  tests; 6 in-scope writers × 1 variant-coverage assertion each (plus
  1 Hold-fast-return).
- **Channel sizing:** default `tick_bus_capacity = 1024` (≈5 s
  headroom at peak 200 ticks/s; ≈200 KB per receiver). Matches
  `agent::bus::EventBus::fills_tx`.
- **Event size:** `size_of::<AuditEvent>() ≤ 256 B` (H5 enforced via
  `static_assertions::const_assert!`; `Fill` and `StrategySignal`
  variants both boxed to satisfy budget — Signal also exceeded the
  unboxed budget, per developer's H5-boxing note).
- **Bench (informational, not gated):** `tick_send_latency` criterion
  bench at `crates/audit/benches/tick_send_latency.rs` — numbers
  produced for H1, not used as a gate per decomp §7.
- **New external deps:** 0. `tokio::broadcast`, `serde`, `uuid`,
  `time`, `smol_str`, `rust_decimal`, `metrics`, `static_assertions`
  were all already in the workspace.

## Known deviations / carry-forward debt

This is **not** introduced by `audit-tick-consumer-envelope`; documented
for visibility.

### Deviations explicitly accepted in this brief

1. **T-D-12** — `audit` dep in `crates/forecast/Cargo.toml` kept
   **required** (not optional as the decomp originally specified). The
   `train_tcn` bin already uses `audit` unconditionally; making it
   optional would break the existing build. The `audit-tick = []`
   feature flag still gates all `TcnForecaster` ledger fields and emit
   calls at compile time — the goal of compile-time gating is met.
2. **T-D-14** — `TcnForecaster::with_ledger()` runtime wiring is
   architecturally blocked: `TcnForecaster` is constructed inside
   `crates/strategy` from TOML config, not in `agent/src/main.rs`. The
   compile-time feature chain (`agent → strategy → forecast/audit-tick`)
   **is** wired; the runtime wiring needs a strategy-side config
   surface that accepts an optional `Ledger` handle. Recorded as a
   future architect design item — see Open decisions §1.
3. **H5 boxing** — both `AuditEvent::Fill { fill: Box<Fill>, ... }`
   and `AuditEvent::StrategySignal { signal: Box<Signal>, ... }` were
   boxed to satisfy the 256B size budget. The spec mentioned "likely
   Fill"; Signal also exceeded.

### Pre-existing carry-forward (not this feature)

1. **`cargo clippy --all-targets` 4 errors** (`doc_markdown`) in
   `crates/audit/src/bootstrap.rs:115` and
   `crates/audit/src/journal.rs:2292,2327`. All four originate in
   commit `2112d69` (cockpit-training-control Wave D, 2026-05-19) —
   confirmed by `git blame`. The M-FINAL clippy gate
   (`--workspace -- -D warnings`, no `--all-targets`) is GREEN.
   Cosmetic; routed to developer separately by the tester.
2. **`no_inline_user_visible_strings_in_widgets`** test failure at
   `crates/ui/src/widgets/chart.rs:190` (`"UI_CHART_FORCE_UTC"` not
   routed via `ui::strings`). Introduced in commit `f5fec84`
   (chart-x-axis-local-time), 13 commits prior. `git diff
   f5fec84..ea07934 -- crates/ui/` is empty — `audit-tick` touched no
   UI. Routed to developer separately by the tester.
3. **Spec-lint 87 violations** — 81 dead-link + 6 trace-broken-path
   — all carry-forward from earlier sprints; feature contribution = 0.

## Open decisions

1. **When should the architect close T-D-14 (strategy crate accepts
   optional `Ledger` handle via its config, so live builds can thread
   the handle into `TcnForecaster::with_ledger`)?** Two reasonable
   answers:
   - (a) **Before the first downstream consumer brief lands** —
     keeps the `forecast → audit` edge truly active before another
     consumer relies on it.
   - (b) **Defer until Lab Trail (Phase D) actually subscribes** —
     no consumer currently reads `ForecastEmitted`, so the runtime
     wiring is dead code until then. Lower risk, less yak-shaving.
   The presenter's read of the decomp + risk register leans toward
   (b) — there is no current consumer, and bolting on runtime wiring
   for a stub edge invites a future refactor. The operator's call.

No other decisions are pending; Q1..Q5 were resolved 2026-05-20 to
analyst defaults via "Autoapprove all".

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

Operator-approved 2026-05-20 via "Autoapprove all" directive. Open
question on T-D-14 strategy-crate Ledger handle resolved to **option
(b) DEFER** per presenter's recommendation — Lab Trail (Phase D) is
the first downstream consumer that would need ForecastEmitted at
runtime; closing T-D-14 now would land dead code. Feature ships at
v0.1.0.

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-20 (presenter): initial release deck; tester PASS verdict
  on commit `ea07934`; anchors 22/22; 1 open decision (T-D-14 timing)
  surfaced for operator.
