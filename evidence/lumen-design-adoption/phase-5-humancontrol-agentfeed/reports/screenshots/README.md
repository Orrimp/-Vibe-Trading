# Lumen Phase 5 — HumanControl + AgentFeed · screenshot manifest

Captured for the Phase 5 sprint review on 2026-05-07.

| File | Caption | Capture date | Captured by |
|---|---|---|---|
| `cockpit-control.png` | Cockpit · Control screen (HumanControl panel: execution-mode segments + limits + kill bottom action) | _pending operator capture_ | _operator (sandbox lacks screen-recording permission)_ |
| `cockpit-strategies-pause-override.png` | Cockpit · Strategies-detail with pause buttons + override-veto buttons (surfaced fixture veto) | _pending operator capture_ | _operator_ |
| `cockpit-override-modal.png` | Cockpit · Override-risk-veto modal with `OVERRIDE` typed-confirm phrase | _pending operator capture_ | _operator_ |
| `cockpit-focus-ring.png` | Cockpit · Focus-ring halo on a focused destructive control (TD-1 closure) | _pending operator capture_ | _operator_ |

## Capture commands

```bash
# 1. Control screen (HumanControl panel)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Control" in the sidebar …
screencapture -W evidence/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/screenshots/cockpit-control.png
pkill -f "target/release/cockpit"

# 2. Strategies-detail with pause + override-veto
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" → click any strategy row …
screencapture -W evidence/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/screenshots/cockpit-strategies-pause-override.png
pkill -f "target/release/cockpit"

# 3. Override-risk-veto modal
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … click "Strategies" → click "Override" on a surfaced veto …
screencapture -W evidence/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/screenshots/cockpit-override-modal.png
pkill -f "target/release/cockpit"

# 4. Focus-ring halo (TD-1 closure)
cargo run --release --bin cockpit --features fixtures &
sleep 4
# … Tab to a destructive control (kill / override / pause); halo renders …
screencapture -W evidence/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/screenshots/cockpit-focus-ring.png
pkill -f "target/release/cockpit"
```

Referenced from [`evidence/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](../../../../../docs/archive/presentations-2026-Q2.tar.gz) (Phase 5 section of the consolidated retrospective).
