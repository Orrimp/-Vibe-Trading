# Orchestrator tooling — `scripts/orch_*` (2026-05-12)

Six small bash scripts + one Swift helper extracted from recurring patterns
in the chart-canvas-overhaul + ui-test-harness-bootstrap sessions. Each
script existed inline as 3-80 lines of ad-hoc bash/heredoc that I (the
orchestrator) wrote 2-4 times in those sessions. Extracting them removes
the re-implementation tax and the bug surface of slightly-different
inline variants.

These tools belong to the **orchestrator's lane** per
[`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent).
Sub-agents must NOT call them — they wrap capabilities (display server,
GPU, cursor automation, screen recording, cockpit-binary launch) that are
explicitly orchestrator-only.

## Inventory

| Script | Purpose | Replaces |
|---|---|---|
| [`scripts/orch_crop.sh`](../../scripts/orch_crop.sh) | Crop a PNG with sane `(x, y, w, h)` arg order | Bare `sips --cropOffset` (which is `(y, x)` and I got it wrong twice) |
| [`scripts/orch_probe_tcc.sh`](../../scripts/orch_probe_tcc.sh) | One-line report of macOS TCC permissions effective for this process | 3 different osascript probes I wrote ad-hoc |
| [`scripts/orch_supplement_log.sh`](../../scripts/orch_supplement_log.sh) | Append `### <title>` block with verbatim cmd output + exit code to a test-run log | ~80-line heredoc I wrote in the bootstrap session |
| [`scripts/orch_determinism_check.sh`](../../scripts/orch_determinism_check.sh) | Run `cargo test` twice + shasum-diff named outputs | Inline H1 falsifier (recurring) |
| [`scripts/orch_cockpit_on_screen.sh`](../../scripts/orch_cockpit_on_screen.sh) | Patch `cockpit.rs:158` to a non-Home `Screen::*`, build, launch in bg | 4 cycles of sed + cargo + revert |
| [`scripts/orch_cockpit_off.sh`](../../scripts/orch_cockpit_off.sh) | Kill cockpit + revert the patch + verify git clean | Manual cleanup that occasionally got skipped |
| [`scripts/orch_hover_screenshot.sh`](../../scripts/orch_hover_screenshot.sh) | Move cursor to `(x, y)` via CGEvent + screencapture | `/tmp/orch-diag/{warp,hover}.swift` + ad-hoc bash sequencing |
| [`scripts/orch_cursor_move.swift`](../../scripts/orch_cursor_move.swift) | CGWarp + CGEvent mouseMoved primitive | Above ad-hoc Swift files |

## Capability-boundary alignment

Per AGENT.md's capability map:

| Capability | Owner | Wrapped by |
|---|---|---|
| `cargo run --bin cockpit` with live window | orchestrator | `orch_cockpit_on_screen.sh` + `orch_cockpit_off.sh` |
| `screencapture` of running app | orchestrator | `orch_hover_screenshot.sh` |
| Cursor automation (CGWarp / CGEvent) | orchestrator | `orch_cursor_move.swift` + `orch_hover_screenshot.sh` |
| Sandbox-supplement log for sub-agent partials | orchestrator | `orch_supplement_log.sh` |

Sub-agents should NOT have these scripts in their allowlist. The
`.claude/settings.local.json` allowlist additions are scoped to the
orchestrator session.

## Quickstart recipes

**Capture the Charts screen with a hovered tooltip:**

```bash
scripts/orch_cockpit_on_screen.sh Charts
scripts/orch_hover_screenshot.sh 1245 575 out.png 1500
scripts/orch_cockpit_off.sh
```

**Supplement a test-runner log with sandbox-denied checks:**

```bash
LOG=spec/<slug>/reports/test-run-2026-05-12T12-43Z.log
scripts/orch_supplement_log.sh $LOG "verify_anchors" -- bash scripts/verify_anchors.sh
scripts/orch_supplement_log.sh $LOG "dense-mode grid sweep" -- env CHART_HIT_TEST_GRID=dense cargo test -p ui --test chart_hover_grid_sweep
scripts/orch_supplement_log.sh $LOG "H1 determinism" -- scripts/orch_determinism_check.sh -p ui --test visual_snapshots -- 'crates/ui/tests/visual-baselines/*.png'
```

**Probe TCC before attempting cursor automation:**

```bash
scripts/orch_probe_tcc.sh
# screen-recording=YES accessibility=BLOCKED-BY-AUTOMATION automation-system-events=YES
```

**Crop a region from a captured screenshot for review:**

```bash
scripts/orch_crop.sh capture.png 3300 250 900 500 legend-crop.png
```

## Why these aren't `.claude/skills/` or sub-agent tools

A natural question: shouldn't this be a Skill (Skill tool invocation) so
sub-agents can use it? **No** — and that's the whole point.

`AGENT.md ## Capability boundaries` is explicit: sub-agents whose tasks
require a display server, GPU, screencapture, or cursor automation must
ESCALATE to the orchestrator rather than try in their own sandbox. If we
expose these scripts to sub-agents via a Skill, we silently re-open the
"sub-agent rationalizes around a sandbox denial" failure mode that
motivated the capability-boundaries amendment.

Sub-agents calling `cargo test`, `cargo build`, `verify_anchors.sh`, etc.
are fine — those are write-allowed sandbox-runnable capabilities. The
`orch_*` set is intentionally orchestrator-only.

## Smoke-test results (2026-05-12)

All 6 scripts PASS end-to-end smoke test:

- `orch_crop.sh` — produced 900x500 PNG from a 4112x2658 source
- `orch_probe_tcc.sh` — returned `screen-recording=YES accessibility=BLOCKED-BY-AUTOMATION automation-system-events=YES`
- `orch_supplement_log.sh` — appended two named sections (one exit 0, one exit 7) with verbatim stdout+stderr
- `orch_determinism_check.sh` — ran `cargo test -p ui --test visual_snapshots` twice; `DETERMINISM PASS (byte-identical across two runs)`
- `orch_cockpit_on_screen.sh Charts` — patched, built (3s), launched (pid 78226), confirmed alive
- `orch_hover_screenshot.sh 1245 575` — captured 4112x2658 PNG with cursor warped + CursorMoved dispatched
- `orch_cockpit_off.sh` — killed cockpit, reverted `cockpit.rs`, removed `.bak`, removed state dir, `git diff --quiet` green

## Maintenance notes

- The `orch_cockpit_on_screen.sh` script patches `crates/ui/src/bin/cockpit.rs:158`. If that line moves or the sed pattern changes, this script needs updating. Defended by a sed pattern that targets `cockpit.current_screen = Screen::` rather than a hard line number.
- The `orch_hover_screenshot.sh` "jiggle" (dispatching CursorMoved twice with a 1-px offset) is defensive against iced's coalescing of repeated same-position CursorMoved events. Removed only if a future iced version drops that coalescing.
- The `orch_probe_tcc.sh` Accessibility probe currently reports `BLOCKED-BY-AUTOMATION` when Automation is denied — that's because AppleScript's `keystroke` requires both Accessibility AND Automation→System Events. A pure Accessibility probe would need a Swift binary calling `AXIsProcessTrusted()`. Defer until we need to differentiate.

## Related

- [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](ui-testing-direction-2026-05-12.md) — the strategy doc that motivated the capability-boundaries amendment these scripts complement
- [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent) — the load-bearing rule that scopes these tools to the orchestrator
- Existing project scripts (`scripts/verify_anchors.sh`, `scripts/check_no_clocks_in_ui_tests.sh`, `scripts/capture_screenshot.sh`, etc.) — same `scripts/` directory, callable from sub-agents (no capability-boundary concern)
