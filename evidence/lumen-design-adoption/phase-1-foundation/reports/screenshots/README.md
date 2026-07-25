# Lumen Phase 1 — Foundation · screenshot manifest

Captured for the Phase 1 sprint review on 2026-05-04.

| File | Caption | Capture date | Captured by |
|---|---|---|---|
| `cockpit-fixtures.png` | Fixtures bin · all panels + status bar (deterministic demo data) | _pending operator capture_ | _operator (sandbox lacks screen-recording permission)_ |
| `cockpit-live.png` | Live bin · cockpit attached to the live agent runtime | _pending operator capture_ | _operator_ |

## Capture commands

```bash
# Fixtures bin
cargo run --release --bin cockpit --features fixtures &
sleep 4
screencapture -W evidence/lumen-design-adoption/phase-1-foundation/reports/screenshots/cockpit-fixtures.png
pkill -f "target/release/cockpit"

# Live bin
cargo run --release --bin cockpit_live --features live -- \
    --config config/agent.toml &
sleep 8
screencapture -W evidence/lumen-design-adoption/phase-1-foundation/reports/screenshots/cockpit-live.png
pkill -f "target/release/cockpit_live"
```

Referenced from [`evidence/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](../../../../../docs/archive/presentations-2026-Q2.tar.gz) (phase-1-foundation section of the consolidated retrospective).
