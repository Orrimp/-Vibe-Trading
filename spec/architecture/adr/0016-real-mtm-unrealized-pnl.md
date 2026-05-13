---
adr: 0016
title: v1+ — real mark-to-market unrealized PnL plumbing (Q1–Q8 + R10)
status: accepted
date: 2026-05-02
supersedes: none
superseded-by: none
---

# ADR-0016: v1+ — real mark-to-market unrealized PnL plumbing (Q1–Q8 + R10)

## Context

The operator success report ([ADR-0015](0015-operator-success-reports.md))
shipped with a placeholder `let unrealized: Decimal = Decimal::ZERO;`
at `crates/reports/src/lib.rs:135–150`. Closing the placeholder
requires a typed open-positions reader plus a marking convention.
Eight Q's plus one residual concern (R10, the hardcoded
`assets:position:BTC` account in `post_fill`) covered the surface.

## Decisions

### Q1 — Reader signature: snapshot vec

```rust
pub async fn open_positions_at(
    ledger: &Ledger,
    ts:     Timestamp,
) -> Result<Vec<OpenPosition>, LedgerError>;
```

Parallel to `pnl_by_strategy` / `pnl_by_symbol`. Sort key
`(symbol ASC, strategy_id ASC, None last)` for byte-identical
re-reads (R6).

### Q2 — `OpenPosition` placement: `trading_core`

New `crates/core/src/position.rs` with `OpenPosition { symbol, qty,
avg_cost_basis: Money<Usdt>, opened_at, strategy_id }`. Audit
produces; reports consumes; cockpit may consume in future.
Cross-crate placement follows the same rule as
[ADR-0012](0012-v05-broadcast-bus-extensions.md).

### Q3 — Index strategy: no new SQL index

NO new SQL index for v1+. The conditional migration
`006_open_positions_index.sql` ships only if the V8 perf gate
(<100ms for 100 fills + 5 open positions) fails. V8 PASSED at
0.287ms — the index never landed; the `006` migration slot was
reclaimed by `per-symbol-position-accounts`.

### Q4 — Anchor regression: byte-identical

Both v1+ anchors stay byte-identical. The reader returns
deterministic sorted vectors; the report rendering computes
unrealized PnL with `Decimal` math; the body hash is unchanged.

### Q5 — Fixture choice: new non-anchored fixture

Add a test-only, non-anchored fixture exercising the open-positions
reader in isolation. Keeps the V8 perf gate decoupled from the
anchor suite.

### Q6 — Mark source: open-position avg-cost-basis (architect override)

Architect override of analyst's proposal to use mid-price. The
mark-source is the open position's own `avg_cost_basis` — the
unrealized P&L is then `(current_price - avg_cost_basis) * qty`.
This avoids depending on a live mid-price snapshot for reports
written from cron at arbitrary timestamps.

### Q7 — Cost basis: weighted-average

Weighted-average across remaining qty with full close-out. When a
position is fully closed and reopened, the cost basis resets.
Closes the v0 ambiguity ("what's cost basis for a partially-closed
lot?") for v1+.

### Q8 — Long-only at v1+

Reader filters to `running_qty > 0`. Short positions raise
`LedgerError::Database` (defensive — should be unreachable in v1+).
Shorts are queued for v2+ when perps come online per
[ADR-0013](0013-v1-cross-sectional-momentum.md) Q3.

### R10 — Hardcoded BTC account: deferred

The `assets:position:BTC` hardcode at
`crates/audit/src/journal.rs:82,135` (every fill writes to the BTC
bucket regardless of symbol) is explicitly deferred to a follow-up
brief, `per-symbol-position-accounts`. Migration `006` (reclaimed
slot) seeds per-pair `assets:position:<SYMBOL>` rows; T1102 flips
the writer. Description-parse stays the primary symbol source for
legacy-row compat.

## Alternatives considered

- **Mid-price mark source.** Requires a live snapshot at arbitrary
  report timestamps. Rejected.
- **Eager SQL index at v1+.** Premature optimization; the V8 gate
  proved 0.287ms vs 100ms budget. Rejected.
- **Allow shorts at v1+.** Breaks the spot-long-only commitment
  from ADR-0013 Q3. Rejected.

## Consequences

- The `assets:position:BTC` hardcode is now the only known bug
  scheduled for follow-up. Documented in
  `spec/per-symbol-position-accounts/` as a separate feature.
- `OpenPosition` joins `FillView` / `JournalEntryView` /
  `StrategyEventView` as a cross-crate view type in `trading_core`.
- The V8 perf-gate-pass-or-add-index pattern is now the precedent
  for any future report-side query — measure first, add index only
  on failure.

## Changelog
- 2026-05-02 (architect): initial accept.
- 2026-05-13 (architect): extracted from `spec/architecture.md` §
  real-mtm-unrealized-pnl resolutions during Phase 1A Session 9.
