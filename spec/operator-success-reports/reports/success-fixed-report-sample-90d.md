---
period: since:2026-01-28T00:00:00Z
period_start: 2026-01-28T00:00:00.000000Z
period_end: 2026-05-10T06:04:49.211707Z
generated: 2026-05-10T06:04:49.211707Z
run_id: 7138f3f903551cc4
ledger_snapshot_sha: e8678f75865a7377aeb150a5f15f0ef5859fd15e1c45c6c3b0e2200769fffe53
seed: 0xC0FFEE
data_source: fixture:/var/folders/3d/q05sqj0x3r79f5jszgbsv0cc0000gp/T/.tmpfKyRcs/audit-90d.db
wall_clock_s: 0.071887
binary_version: 0.1.0
git_commit: n/a
agent_pid: 4649
host: unknown
reconciliation: PASS
---

## Open risks

- Rebalance rejections accumulating: threshold rebalance_rejected > 5% of trade_count (observed 3 rejected of 12 trades)
- Mean-reversion hard stops accumulating: threshold mr_stop > 10% of pair_trade_count (observed 1 hard-stops of 3 pair trades)

## Headline

Strategy return: +0.00% (+$0.00 USDT)
BTC buy-and-hold: +0.00% (+$0.00 USDT)

## Equity curve

Since inception:
`▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁`

Window since:2026-01-28T00:00:00Z:
`▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁`

## Risk metrics

| Metric | Value | Period |
|--------|-------|--------|
| Sharpe | 0.0000 | since:2026-01-28T00:00:00Z |
| Sortino | 0.0000 | since:2026-01-28T00:00:00Z |
| Calmar | 0.0000 | since:2026-01-28T00:00:00Z |
| Max drawdown | 0.00% ($0.00) | since:2026-01-28T00:00:00Z |
| Recovery time | 0 bars | since:2026-01-28T00:00:00Z |

## Strategy attribution

| strategy_id | P&L (USDT) | trade count | win rate | avg trade P&L |
|-------------|------------|-------------|----------|---------------|
| pairs_zeta | 0.00 | 3 | 0.00% | 0.00 |
| strat_alpha | 0.00 | 3 | 0.00% | 0.00 |
| strat_beta | 0.00 | 3 | 0.00% | 0.00 |
| strat_gamma | 0.00 | 3 | 0.00% | 0.00 |

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
| LLM spend | $0.00 / $135 |

## What changed

- 2026-01-30T06:00:00Z [Load] strategy_id=strat_alpha source=config/strategies/sample-90d.toml new_hash=aabbcc..
- 2026-01-30T06:00:00Z [Load] strategy_id=strat_beta source=config/strategies/sample-90d.toml new_hash=aabbcc..
- 2026-01-30T06:00:00Z [Load] strategy_id=strat_gamma source=config/strategies/sample-90d.toml new_hash=aabbcc..
- 2026-01-30T06:00:00Z [Load] strategy_id=pairs_zeta source=config/strategies/sample-90d.toml new_hash=aabbcc..
- 2026-02-27T00:00:00Z [Swap] strategy_id=strat_alpha old_hash=aabbcc.. new_hash=998877..

## Reconciliation

| Identity | Report | Ledger | Δ | Pass? |
|----------|--------|--------|---|-------|
| headline_return = realized + unrealized | 0 | 0 | 0 | PASS |
| Σ pnl_by_strategy = Σ realized | 0.00 | 0.000 | 0.000 | PASS |
| Σ pnl_by_symbol = Σ realized | 0 | 0.000 | 0.000 | PASS |
| equity_delta = realized + unrealized + fees_delta | 0 | 0 | 0 | PASS |
