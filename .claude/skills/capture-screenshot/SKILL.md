---
name: capture-screenshot
description: Capture a UI screenshot of a running binary, OR emit a manual-capture instruction block if running in a headless sandbox. Use when the presenter agent (or ui-designer) needs a fresh screenshot for a presentation. macOS uses `screencapture` natively; Linux + headless paths emit operator instructions instead of failing.
---

# capture-screenshot

GUI screenshots are inherently brittle in agent sandboxes. This skill
prefers a manual-instruction path over fake automation.

## Inputs

- `binary` — name of the binary to screenshot, e.g. `cockpit`.
- `feature_args` — cargo args to pass, e.g. `--features fixtures`.
- `panel_name` — used for the output filename, e.g. `tape-ready`.
- `slug` — feature slug; output goes to `evidence/<slug>/reports/screenshots/`.
- `instruction_only` (default false) — skip the actual capture and
  emit only the operator instruction block.

## Procedure

1. **Resolve output path.** `evidence/<slug>/reports/screenshots/<panel_name>.png`.
   If the file already exists and the caller didn't ask for a refresh,
   return the existing path with no capture.

   **Pre-check: is this even a UI feature?** Before invoking any
   capture path, verify the feature has a UI surface. Heuristics
   (any one is sufficient):
   - `spec/<slug>/feature.md` contains a `## UI` heading, or
   - `evidence/<slug>/reports/screenshots/` directory exists (presence
     of the directory is the project convention for "UI feature").

   If neither is true, emit `n/a — non-UI feature` and return without
   capturing or emitting an operator instruction block. Caller (the
   presenter agent's screenshots step) writes
   `_n/a — non-UI feature_` into the deck. This branch was added
   2026-05-08 after the `operator-success-reports` smoke test
   surfaced that the skill defaulted to a manual-capture instruction
   for a feature that has no UI.

2. **Detect the platform** via `uname -s`:
   - `Darwin` → use `scripts/capture_screenshot.sh` (calls `screencapture`).
   - `Linux` → emit operator instruction (no portable headless GUI capture).
   - Anything else → emit operator instruction.

3. **macOS path** (`Darwin`, non-instruction-only):
   - Call `scripts/capture_screenshot.sh <binary> "<feature_args>" <output_path>`.
   - The script launches the binary in the background, sleeps 4s for the
     window to draw, runs `screencapture -W` (window-pick) or `-l` if a
     window-id is known, and kills the binary.
   - If `screencapture -W` requires user click, the script falls back to
     `screencapture -x <path>` (full screen) and prints a message that the
     full screen was captured.
   - On any failure, drop to the operator instruction path.

4. **Operator instruction path** (any non-Darwin OR sandbox-blocked):
   Emit a code block the operator copies into their terminal:

   ```
   # On your operator workstation, capture the <panel_name> screenshot:
   cargo run --release --bin <binary> <feature_args> &
   sleep 4
   screencapture -W evidence/<slug>/reports/screenshots/<panel_name>.png   # macOS
   # OR: gnome-screenshot -w -f evidence/<slug>/reports/screenshots/<panel_name>.png   # Linux GNOME
   pkill -f "target/release/<binary>"
   ```

   Then return the expected output path so the presentation can
   reference it (the presenter writes "captured manually" in the
   caption).

## Output

- A pair of values:
  - `path` — the .png path (whether captured or pending).
  - `status` — `captured` | `instruction-emitted` | `existing` | `failed-fallback-instruction`.

## Failure modes

- `screencapture` exits non-zero (e.g. user dismissed permission dialog)
  → emit instruction, do not fail the calling agent.
- Binary fails to launch → emit error + bin output + instruction.
- Output dir does not exist → mkdir -p first.

## Manifest

The presenter or ui-designer SHOULD list every screenshot referenced
in a presentation under `evidence/<slug>/reports/screenshots/README.md` so
future agents can find them without globbing. The README is small and
plain-text — caption per file plus the capture date.
