---
period: since:2026-04-21T00:00:00Z
period_start: 2026-04-21T00:00:00.000000Z
period_end: 2026-05-12T21:35:36.312258Z
generated: 2026-05-12T21:35:36.312258Z
run_id: 7d3d495368a0e601
ledger_snapshot_sha: 9d32fa27b8056977b69963a9a1ca98066eb375471fd66489553ff7dc40d7f233
seed: 0xC0FFEE
data_source: fixture:/var/folders/3d/q05sqj0x3r79f5jszgbsv0cc0000gp/T/.tmpdfpmqw/audit-7d.db
wall_clock_s: 0.028091
binary_version: 0.1.0
git_commit: n/a
agent_pid: 9691
host: unknown
reconciliation: PASS
---

## Open risks

- Rebalance rejections accumulating: threshold rebalance_rejected > 5% of trade_count (observed 1 rejected of 6 trades)

## Headline

Strategy return: +0.00% (+$0.00 USDT)
BTC buy-and-hold: +0.00% (+$0.00 USDT)

## Equity curve

Since inception:
`▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁`

Window since:2026-04-21T00:00:00Z:
`▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁`

## Risk metrics

| Metric | Value | Period |
|--------|-------|--------|
| Sharpe | 0.0000 | since:2026-04-21T00:00:00Z |
| Sortino | 0.0000 | since:2026-04-21T00:00:00Z |
| Calmar | 0.0000 | since:2026-04-21T00:00:00Z |
| Max drawdown | 0.00% ($0.00) | since:2026-04-21T00:00:00Z |
| Recovery time | 0 bars | since:2026-04-21T00:00:00Z |

## Strategy attribution

| strategy_id | P&L (USDT) | trade count | win rate | avg trade P&L |
|-------------|------------|-------------|----------|---------------|
| strat_alpha | 0.00 | 3 | 0.00% | 0.00 |
| strat_beta | 0.00 | 3 | 0.00% | 0.00 |
| strat_gamma | (no activity) | (no activity) | (no activity) | (no activity) |

## Memory highlights

_no closed trades yet — lesson cards will appear after the first closed trade._

## System health

| Metric | Value |
|--------|-------|
| Uptime | 100% |
| Kill-switch trips | 0 |
| Clock-skew events | 0 |
| Feed reconnects | 0 |
| Funding poll success | n/a |
| LLM spend | $0.00 / $200 |
| Cache hit ratio | 0.0% |

## What changed

- 2026-04-21T01:00:00Z [Load] strategy_id=strat_alpha source=config/strategies/sample-7d.toml new_hash=aabbcc..
- 2026-04-21T01:00:00Z [Load] strategy_id=strat_beta source=config/strategies/sample-7d.toml new_hash=aabbcc..
- 2026-04-21T01:00:00Z [Load] strategy_id=strat_gamma source=config/strategies/sample-7d.toml new_hash=aabbcc..

## Reconciliation

| Identity | Report | Ledger | Δ | Pass? |
|----------|--------|--------|---|-------|
| headline_return = realized + unrealized | 0 | 0 | 0 | PASS |
| Σ pnl_by_strategy = Σ realized | 0.00 | 0.00 | 0.00 | PASS |
| Σ pnl_by_symbol = Σ realized | 0 | 0.00 | 0.00 | PASS |
| equity_delta = realized + unrealized + fees_delta | 0 | 0 | 0 | PASS |
