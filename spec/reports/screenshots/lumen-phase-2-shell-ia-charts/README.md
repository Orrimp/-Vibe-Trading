# Lumen Phase 2 — Shell IA + Charts · screenshot manifest

Captured for the Phase 2 sprint review on 2026-05-05.

| File | Caption | Capture date | Captured by |
|---|---|---|---|
| `cockpit-home.png` | Fixtures bin · Home screen (default landing — PnL / Positions / Strategies / Tape under the new sidebar shell) | _pending operator capture_ | _operator (sandbox lacks screen-recording permission)_ |
| `cockpit-debug.png` | Fixtures bin · Debug screen (kill switch · latency · per-venue market health · server time · version · placeholder logs) | _pending operator capture_ | _operator_ |
| `cockpit-charts-with-markers.png` | Fixtures bin · Charts screen with a symbol selected (line series + buy/sell triangle markers from the audit ledger) | _pending operator capture_ | _operator_ |
| `cockpit-live-home.png` | Live bin · Home screen (real venues, live agent attached) | _pending operator capture_ | _operator_ |

## Capture commands

```bash
# 1. Home (default landing)
cargo run --release --bin cockpit --features fixtures &
sleep 4
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-home.png
pkill -f "target/release/cockpit"

# 2. Debug screen (click "Debug" in the sidebar before capture)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Debug" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-debug.png
pkill -f "target/release/cockpit"

# 3. Charts with markers (click "Charts" → click any symbol chip)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Charts", then click a symbol chip …
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-charts-with-markers.png
pkill -f "target/release/cockpit"

# 4. Live bin
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
screencapture -W spec/reports/screenshots/lumen-phase-2-shell-ia-charts/cockpit-live-home.png
pkill -f "target/release/cockpit_live"
```

Referenced from [`spec/presentations/lumen-phase-2-shell-ia-charts-2026-05-05.md`](../../presentations/lumen-phase-2-shell-ia-charts-2026-05-05.md).
