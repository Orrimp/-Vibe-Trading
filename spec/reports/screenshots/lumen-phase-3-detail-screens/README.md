# Lumen Phase 3 — Detail screens · screenshot manifest

Captured for the Phase 3 sprint review on 2026-05-05.

| File | Caption | Capture date | Captured by |
|---|---|---|---|
| `cockpit-strategies.png` | Fixtures bin · Strategies-detail screen (per-strategy params + signal-event history) | _pending operator capture_ | _operator (sandbox lacks screen-recording permission)_ |
| `cockpit-risk.png` | Fixtures bin · Risk / Limits screen (per-venue exposure cells + kill-threshold proximity gauge with tri-band colour ramp) | _pending operator capture_ | _operator_ |
| `cockpit-audit.png` | Fixtures bin · Audit / Journal screen (filter chip-row + 250-row pagination) | _pending operator capture_ | _operator_ |
| `cockpit-live-six-entries.png` | Live bin · sidebar with all 6 entries (Home / Debug / Strategies / Risk / Audit / Charts) | _pending operator capture_ | _operator_ |

## Capture commands

```bash
# 1. Strategies-detail (click "Strategies" in sidebar before capture)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-strategies.png
pkill -f "target/release/cockpit"

# 2. Risk / Limits screen
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Risk" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-risk.png
pkill -f "target/release/cockpit"

# 3. Audit / Journal screen
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Audit" in the sidebar …
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-audit.png
pkill -f "target/release/cockpit"

# 4. Live bin (any screen — sidebar shows all 6 entries)
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
screencapture -W spec/reports/screenshots/lumen-phase-3-detail-screens/cockpit-live-six-entries.png
pkill -f "target/release/cockpit_live"
```

Referenced from [`spec/presentations/lumen-phase-3-detail-screens-2026-05-05.md`](../../presentations/lumen-phase-3-detail-screens-2026-05-05.md).
