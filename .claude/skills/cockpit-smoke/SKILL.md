---
name: cockpit-smoke
description: Boot the fixtures-mode cockpit for a fixed window and grep stderr for first-frame render panics. Mandatory orchestrator pre-tick gate after every UI brief's PASS verdict per AGENT.md ## Process discipline rule 6. Catches the F1-class regression (zero-dim Quad panic in iced_tiny_skia::engine.rs:686) that escaped Brief A's 267-test PASS.
---

# cockpit-smoke

The visual regression gate that closes the gap behind `cockpit-render-regression v1.0.0`.

Brief A's 267-test panel-snapshot suite uses **text-summary helpers** (`tape_summary`, `strategies_summary`, etc. at `crates/ui/tests/panel_snapshots.rs:1779-2298`) — they never exercise the iced widget tree. A runtime panic at `iced_tiny_skia::engine.rs:686:14` shipped to operator approval anyway and was caught only when the operator manually ran `cargo run --bin cockpit`. This skill makes that gate machine-runnable and mandatory.

## Capability boundary

**Orchestrator-only.** Per [`AGENT.md ## Capability boundaries`](../../../AGENT.md#capability-boundaries), `cargo run --bin cockpit` with a live window cannot be executed by sub-agents — only the orchestrator. Sub-agents may cite this skill's log; they may not invoke it.

## Procedure

1. **Build first** (separate compile time from runtime so the smoke window measures cold-start, not cargo lookup).

   ```bash
   cargo build -p ui --bin cockpit --features fixtures
   ```

   Fail loud on build errors — they belong to the developer who broke the build, not this gate.

2. **Spawn cockpit in background, capture stderr, wait, kill.**

   ```bash
   LOG=spec/<slug>/reports/cockpit-smoke-$(date -u +%Y-%m-%dT%H-%MZ).log
   mkdir -p "$(dirname "$LOG")"
   (RUST_BACKTRACE=1 cargo run -p ui --bin cockpit --features fixtures > "$LOG" 2>&1 &)
   sleep 7
   pkill -f "target/debug/cockpit" 2>/dev/null
   sleep 1
   ```

   The 7s window is the hypothesis; per `spec/v1/ui-quality-gate-overhaul/feature.md ## H-A4`, the first three real runs against the post-F1 commit are MEASURED — if any exceeds 5s, bump to 10s and record the calibration in the feature's changelog.

3. **Grep stderr for any panic signature.**

   ```bash
   PANIC_COUNT=$(grep -c "panicked at\|non-unwinding panic\|fatal runtime error" "$LOG")
   if [ "$PANIC_COUNT" -gt 0 ]; then
     echo "FAIL — $PANIC_COUNT panic line(s) in $LOG"
     grep "panicked at\|non-unwinding panic" "$LOG" | head -5
     exit 1
   fi
   echo "PASS — 0 panic lines in $LOG (7s smoke window)"
   exit 0
   ```

## Exit codes

| code | meaning |
|------|---------|
| `0` | Clean run: no panic signature in stderr after the 7s window. |
| `1` | Panic detected: see the log for the first `panicked at` line + backtrace. Route `HANDOFF → developer` (UX/visual regressions route to `ui-designer` instead). |

## When to run

- **Every UI brief's PASS verdict** (mandatory). Per `AGENT.md ## Process discipline` rule 6, the orchestrator MUST run cockpit-smoke between evaluator PASS and presenter assembly. A failing smoke flips the verdict to REGRESSION and routes back to developer or ui-designer per the failure mode.
- **After any `iced` / `iced_aw` / `iced_tiny_skia` version bump** — the F1 panic was an iced_tiny_skia clamp-then-reject interaction; future renderer changes may surface similar regressions.
- **Spot-check after large UI refactors** even within a single brief — useful as a "did I break the cockpit" tripwire during multi-file edits.

## Empirical proof (operator-verifiable)

To convince yourself this gate would have caught F1:

| Commit | Expected verdict | Cite |
|--------|------------------|------|
| pre-F1 (Brief A shipped 2026-05-13) | FAIL | `/tmp/cockpit-runtime.log` (3 panic lines) |
| post-F1 (cockpit-render-regression v1.0.0) | PASS | `/tmp/cockpit-postrefactor.log` (0 panic lines) |

Both logs are already on disk from the 2026-05-14 ship pass. Run this skill against `git checkout` of either commit to reproduce.

## False-negative envelope (what this skill DOES NOT catch)

- **Silent visual regressions** (palette drift, layout shift, font fallback that doesn't panic). Those route to M1-B real-renderer snapshots.
- **Layout invariants** (zero-dim Node before draw). Those route to M1-C proptest.
- **Panics on user interaction** (click handlers, keyboard input). The 7s window renders the first frame only; no input synthesis.
- **Multi-frame regressions** (e.g. animation that crashes after t=1s). The window catches frame 0 panics; later frames may slip if the panic is throttled past 7s.

Pair this skill with M1-B and M1-C for full coverage. Defense in depth, not single-gate sufficiency.

## On failure

- Capture the panic site + first `panicked at` line + the backtrace's first widget-named frame.
- Route `HANDOFF → developer` for non-visual regressions (`unwrap`/`expect`/integer overflow).
- Route `HANDOFF → ui-designer` for visual regressions (panel disappears, color is wrong but no panic — though those route through M1-B, not this skill).
- File the log under `spec/<slug>/reports/cockpit-smoke-<ts>.log` so the evaluator's read trace can cite it.

## On success

Report a single line in the orchestrator's pre-tick gate output:

```
cockpit-smoke: PASS (0 panics, 7s window, log: spec/<slug>/reports/cockpit-smoke-<ts>.log)
```

That single line is what the presenter agent cites in the verification matrix.
