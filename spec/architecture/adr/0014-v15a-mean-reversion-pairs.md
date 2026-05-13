---
adr: 0014
title: v1.5a — mean-reversion pairs strategy architectural resolutions (Q1–Q10)
status: accepted
date: 2026-04-30
supersedes: none
superseded-by: none
---

# ADR-0014: v1.5a — mean-reversion pairs strategy architectural resolutions (Q1–Q10)

## Context

v1.5a introduces a cointegration-style mean-reversion pairs strategy
on the existing v1 Binance USDT universe. Ten architect-questions
spanned scope (single brief vs split), modelling (hedge ratio,
spot-only formulation), accounting (per-pair P&L), data (USDC pairs,
L2, funding), risk (portfolio exposure cap, hard-stop ledger
surface), composition (per-symbol cap), and ingest semantics (pair
bar sync). All ten resolutions preserve the v0 trait shape ([ADR-0005](0005-v0-strategy-trait-no-hotload.md))
and extend the v0.5 / v1 audit surfaces non-destructively.

Captured as one ADR rather than ten because each Q's answer
constrains the next; splitting would fragment the cross-pair
reasoning.

## Decisions

### Q1 — Single brief vs split (confirm split)

v1.5a ships pairs on the Binance USDT universe only. The sibling
brief `v1.5b-multi-venue` (queued) covers Coinbase + Kraken adapters
and USDC pairs. Splitting keeps the strategy-edge claim (pairs work
on this universe) independent of the venue-diversity claim.

### Q2 — Hedge ratio: fixed β = 1.0 with per-pair TOML override

v1.5a uses fixed β = 1.0 per pair, overridable in the per-pair TOML.
Rolling-OLS β adds a regression dependency, a calibration-window
choice, and a "what to do when the window degenerates" failure mode.
None of those is justified until live metrics demand it. Reopen the
question if a pair shows persistent regime-shift behaviour in v1.5a
live-paper.

### Q3 — Spot-only formulation: C (observation-only short leg)

v1.5a ships formulation C: the spread / z-score machinery computes
signals for both legs of a pair, but **executes only the long leg**.
The short leg's notional is recorded as an `observed_short` field in
`strategy_events` for ledger-side analysis, but no real order is
placed. This keeps the strategy in spot-long-only territory ([ADR-0013](0013-v1-cross-sectional-momentum.md) Q3)
while preserving the analytical edge of the pair structure.

### Q4 — `pnl_by_pair` shape: compose, no schema change

A new `audit::query::pnl_by_pair(pair_membership, since, until)`
reader composes existing `pnl_by_symbol` results with the
pair-membership map captured at strategy-load time. No new audit-DB
schema, no migration. The pair-membership map is a runtime construct
held by the strategy instance; the reader takes it as an argument.
The operator success report calls this reader once per pair-strategy
load.

### Q5 — USDC pairs: blocked on v1.5b multi-venue

v1.5a ships USDT-only pairs. USDC pairs require Coinbase + Kraken
adapters (Binance USDC liquidity is too thin for this strategy) and
are gated on v1.5b's multi-venue ingest landing.

### Q6 — L2 / funding-rate ingest: stay deferred

v1.5a does not consume L2 books or funding rates. The v1 funding
poller (observation-only, [ADR-0013](0013-v1-cross-sectional-momentum.md) Q2)
stays as-is; the pairs strategy does not need it. L2 stays deferred
to v2+.

### Q7 — `portfolio_exposure_cap` shape: reuse v1's single field

v1.5a reuses `RiskLimits.portfolio_exposure_cap: Option<Decimal>`
(added in v1 R5.5). The default is bumped to accommodate pair
positions (which open simultaneously); the field shape doesn't
change. No new risk type; no new config column.

### Q8 — Hard-stop / short-observation ledger surface: two new `strategy_events` kinds

Extend `strategy_events.kind` ([ADR-0008](0008-v05-strategy-event-journal-schema.md))
with two new variants:

- `pair_hard_stop_tripped` — when a pair-position hits its hard
  stop (per-pair `max_drawdown_pct`); emitted alongside the
  close-orders.
- `pair_short_observed` — written when formulation C records the
  observed-but-unexecuted short leg. Per Q3, this is the analytical
  record.

No schema migration. Both variants reuse the existing
`error_code` / `error_summary` columns (semantically: "trip code" /
"observation summary"). Reader: `strategy_events_since` with a
`kind` filter — same pattern as Q6 of v1.

### Q9 — Per-symbol cap composition: strategy emits desired vector; risk clamps

The strategy emits its desired `Vec<ProposedOrder>` and
`risk::size_portfolio_target` clamps per-symbol (the existing v0
risk function). The strategy is unaware of the cap — it proposes
the ideal action; risk reconciles to limits. The clamp emits a
`rebalance_rejected` event ([ADR-0013](0013-v1-cross-sectional-momentum.md) Q6)
when it had to scale a leg down to zero, naming the cap.

This composition rule (strategy proposes, risk disposes) extends to
all multi-symbol strategies going forward. v1.5a is the first
strategy with simultaneous-leg execution, so the rule got formalized
here.

### Q10 — Pair-bar synchronization: wait-for-sync with max-staleness clamp

The strategy waits for both legs of a pair to arrive at the same
`venue_ts` before computing the spread and deciding. If one leg is
stale (no bar within `max_staleness_ms`, default 90 000 ms), the
strategy emits a `pair_bar_stale` strategy-event and skips the
decision for that minute. This prevents acting on torn snapshots
where one leg has updated and the other hasn't.

Implementation lives in the strategy's `on_bar` accumulator — not
in the registry. The registry stays bar-by-bar; the strategy buffers
internally. This is consistent with [ADR-0013](0013-v1-cross-sectional-momentum.md) Q5
(strategy-side filtering, not registry-side).

## Alternatives considered (cross-Q highlights)

- **Rolling-OLS β at v1.5a** (Q2). Adds regression dependency and
  calibration choices. Rejected pending live evidence.
- **Execute the short leg via perps in v1.5a** (Q3). Brings perps
  online ahead of the v2 plan. Rejected; perp infrastructure is its
  own scope.
- **New `pair_pnl` audit table** (Q4). Forks the schema for a
  read-side concern. Rejected in favour of composition.
- **Strategy clamps its own per-symbol exposure** (Q9). Forces
  every strategy to know all other strategies' positions; couples
  what should be orthogonal. Rejected.
- **Strategy emits whichever leg arrives first** (Q10). Acts on
  torn data. Rejected.

## Consequences

- The pairs strategy is the first to use simultaneous-leg
  execution. The "strategy proposes, risk disposes" rule is now
  formalized for all multi-leg / multi-symbol strategies.
- `strategy_events` now carries five kinds in production
  (`Load` / `Swap` / `Unload` / `Reject` from v0.5 + the v1.5a
  additions `pair_hard_stop_tripped`, `pair_short_observed`,
  `pair_bar_stale`, plus `rebalance_rejected` from
  [ADR-0013](0013-v1-cross-sectional-momentum.md) Q6). The table is
  becoming the canonical non-monetary event log; this is the
  expected outcome of [ADR-0008](0008-v05-strategy-event-journal-schema.md)'s
  sibling-table pattern.
- The wait-for-sync logic in Q10 sets a precedent for any future
  multi-symbol strategy that needs aligned bars. If a third such
  strategy lands, consider extracting a `PairSyncBuffer` utility
  from the v1.5a impl rather than copying the logic.
- Anchor count grew from 7 → 9 (the two v1.5a pairs scenarios:
  `pairs-2023-zscore-mr` and `pairs-2024-h1-zscore-mr`). Both
  locked at v1.5a ship time.

## Changelog
- 2026-04-30 (architect): initial accept. Ten interconnected
  decisions captured as a single ADR.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  v1.5a — mean-reversion pairs resolutions during Phase 1A
  Session 8. Intra-block `#v05--strategy-event-journal-schema`
  anchors rewritten to cite ADR-0008.
