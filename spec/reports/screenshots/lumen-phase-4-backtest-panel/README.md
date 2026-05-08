# Lumen Phase 4 — Backtest panel · screenshot manifest

Captured for the Phase 4 sprint review on 2026-05-06.

| File | Caption | Capture date | Captured by |
|---|---|---|---|
| `viewer-full-report.png` | Viewer bin · full backtest report (KPI strip + equity curve + drawdown band + markdown body, SMA baseline scenario) | _pending operator capture_ | _operator (sandbox lacks screen-recording permission)_ |
| `viewer-drawdown.png` | Viewer bin · drawdown-rich report (RSI reversion scenario, max-DD ~57%) | _pending operator capture_ | _operator_ |
| `cockpit-strategies-sparkline.png` | Fixtures cockpit · Strategies-detail screen with the new equity sparkline (Phase 3 deferral closure) | _pending operator capture_ | _operator_ |
| `cockpit-live-strategies-sparkline.png` | Live cockpit · Strategies-detail sparkline against the live audit ledger | _pending operator capture_ | _operator_ |

## Capture commands

```bash
# 1. Viewer · full backtest report
cargo run --release --bin viewer \
    spec/reports/backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-full-report.png
pkill -f "target/release/viewer"

# 2. Viewer · drawdown-rich report
cargo run --release --bin viewer \
    spec/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/viewer-drawdown.png
pkill -f "target/release/viewer"

# 3. Cockpit · Strategies-detail sparkline (fixtures)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" in the sidebar, then click any strategy row …
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-strategies-sparkline.png
pkill -f "target/release/cockpit"

# 4. Live cockpit · Strategies-detail sparkline
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
# … click "Strategies", then click a strategy row …
screencapture -W spec/reports/screenshots/lumen-phase-4-backtest-panel/cockpit-live-strategies-sparkline.png
pkill -f "target/release/cockpit_live"
```

Referenced from [`spec/presentations/lumen-phase-4-backtest-panel-2026-05-06.md`](../../presentations/lumen-phase-4-backtest-panel-2026-05-06.md).
