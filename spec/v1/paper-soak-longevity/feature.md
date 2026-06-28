---
slug: paper-soak-longevity
status: shipped
owner: operator
version: 0.1.0
updated: 2026-06-19
---

# Reflection-loop paper-wiring + paper-soak longevity evidence

## Why
The product's §success-metrics terminal criterion is an *observed* continuous-paper
run with lessons demonstrably accumulating. The operator-chosen soak to produce that
evidence revealed a real defect: the reflection writer was **never wired into the
paper trading loop**, so paper mode accumulated zero lesson cards — the moat #2
differentiator (persistent reflection memory) was broken in the paper path. This
feature closes that defect and ships the longevity evidence artifact.

## Requirements / Acceptance (all met)
- **AC1** — `reflection_writer` threaded `RunHandles → run() → spawn_trading_loop`;
  a lesson card is written on each closed trade. Proven:
  [`crates/agent/tests/reflection_wiring_regression.rs`](../../../crates/agent/tests/reflection_wiring_regression.rs)
  (`lesson_card_is_written_on_position_close` + the `no_lesson_card_without_writer`
  negative control — red without the wiring).
- **AC2** — accurate regime tags: the BTC-daily-close seed is loaded via
  `tokio::task::spawn_blocking` **off the async hot path** (polars/BLAS no longer
  stalls the live paper startup), so `classify_regime` returns Bull/Bear/Chop
  accurately. Proven: `seeded_btc_closes_yields_bull_regime_not_chop`. The
  `unwrap_or(Chop)` fallback is now only the genuine <7d-data path.
- **AC3** — soak evidence: durable fills, equity movement, restart-continuity, kill
  switch, and an accumulated lesson card — see
  [reports/longevity-2026-06-19.md](reports/longevity-2026-06-19.md).
- **AC4** — gates green: `agent`+`reflection`+`ui` full suite, `clippy -D` (forced),
  `fmt`, `verify_anchors` 119/119, cockpit render harnesses.
- **AC5** — operator runbook for the real-time soak:
  [runbook-realtime-soak.md](runbook-realtime-soak.md).

## Design / Implementation
Seed load runs once on a blocking thread before `spawn_trading_loop`; the loop
writes a lesson card (fire-and-forget) on each position close. `data` dep gains the
`yahoo` feature to read the BTC daily-close cache (`data/yahoo/BTC-USD`). See the
final addendum in the longevity report for the three problems fixed in the
completion pass.

## Scope boundary
- The literal **90-day continuous real-time uptime** is an operator deployment soak
  (the runbook), not an in-session deliverable. The in-session soak proves the
  mechanism; the 90-day figure is projected.
- `cockpit_live` passes `reflection_writer = None` **deliberately** — the cockpit
  *reads* the reflection DB (Memory screen); lesson-card *generation* is the headless
  `trading` bin's paper loop. So cockpit paper mode does not itself write lessons.
  (Parity is a possible follow-up.)

## Verification
[reports/longevity-2026-06-19.md](reports/longevity-2026-06-19.md) — evidence artifact.

## Changelog
- 2026-06-19 (operator): shipped — reflection writer wired into the paper loop
  (moat #2), accurate off-hot-path regime seed, soak longevity evidence + runbook.
