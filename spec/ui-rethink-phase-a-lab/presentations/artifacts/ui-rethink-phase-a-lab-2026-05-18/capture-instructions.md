# Visual A/B capture instructions — ui-rethink-phase-a-lab v0.2.0

This artifact is the operator-local capture set the presenter agent
references from `ui-rethink-phase-a-lab-2026-05-18.md`. The presenter
sub-agent runs in a sandbox without display access; per AGENT.md
§ Capability boundaries, capturing live cockpit screenshots is an
operator-local action.

The canonical capture set is mirrored from Appendix B of the final
tester report (`reports/test-2026-05-18-0628-ui-rethink-phase-a-lab.md`).

## Run

```bash
cargo run -p ui --bin cockpit --features fixtures
```

## Captures (3360x1890 Retina, save into `reports/screenshots/`)

| Filename                                | Frame to capture                                                                  |
|-----------------------------------------|-----------------------------------------------------------------------------------|
| `lab-cold-start-defaults.png`           | First boot — top bar shows `v1.momentum x XRPUSDT x Last 90d` (Q-A3 cold-start)   |
| `lab-buy-sell-markers.png`              | Lab default view — candles + buy/sell triangle markers (R2.1 layer 1)             |
| `lab-equity-overlay.png`                | Equity-curve overlay toggled on — ACCENT line on right gutter (R2.2 layer 2)      |
| `lab-compare-three-strategies.png`      | Compare overlay with 3 picks (ACCENT_2 / ACCENT_3 / ACCENT_4) (R2.3 layer 3)      |
| `lab-compare-four-strategies.png`       | Compare overlay full 4-strategy palette (operator-stress for ACCENT_5)            |
| `lab-pair-chip-palette.png`             | Pair chip row open — XRP-first ordering visible (R3.2)                            |
| `lab-persistence-restart.png`           | After quit-and-relaunch — restored tuple (non-default) visible in top bar (R6)    |

## Persistence smoke (Q-A3 manual gate)

1. Launch cockpit; change strategy to any non-default; close window.
2. Re-launch — restored strategy shows in top bar.
3. Delete `~/.config/trading/cockpit-lab-state.json`; re-launch.
4. Confirm cold-start defaults: `v1.momentum x XRPUSDT x Last 90d`.

## After capture

Save the PNGs under
`spec/ui-rethink-phase-a-lab/reports/screenshots/`. Add a one-line
caption + date per file to
`spec/ui-rethink-phase-a-lab/reports/screenshots/README.md`. The
presentation references the relative paths; once the captures land
the presentation does not need to be re-written.
