# Iced UI — debugging & render verification (how not to ship blind)

> **Why this exists.** The cockpit-reports-viewer equity curve shipped *empty*
> twice (2026-06-18) on green checks — because the checks were the wrong layer
> (unit tests, a no-panic `viewer` boot, and a **text** snapshot whose fixture
> hardcoded the empty state). The strong tools existed; they weren't used for the
> state the operator actually sees. Same lesson as the 2026-06 Live-dashboard saga
> ([[feedback_verify_ui_at_render_layer]]). This guide is the antidote.

## The one rule

**Verify cockpit/UI changes at the rendered-PIXEL layer, exercising the
populated/non-trivial state, with a negative control — not via unit tests, text
snapshots, or a no-panic boot.** Those are proxies; they go green while the screen
is blank.

## How the cockpit is built (the parts that matter for debugging)

- **iced 0.14**, Elm-ish: a `Program` with `Message` / `update(state, msg)` /
  `view(state) -> Element`. State you can construct in a test, then render.
- **CPU `tiny-skia` rasterizer** (NOT GPU `wgpu`) — deliberately, so renders are
  **deterministic**. That is *why* pixel snapshots work and are cheap to assert on.
- **Three binaries:** `cockpit_live` (real agent + live bus; **default features**),
  `cockpit` (`--features fixtures`, for smoke), `viewer` (offline single-report).
- **`PanelState<T>` = Loading | Ready(T) | Empty | Error** — every panel renders its
  own state independently (the KPI strip can be Ready while the curve is Empty).
- **Feature flags decide the data path.** `default = [live, yahoo, binance]`;
  `fixtures` is separate **and auto-enabled under `#[cfg(test)]`**. So `cargo test`
  runs *with* fixtures; `cockpit_live` runs *without*. A test can exercise a path the
  live binary never hits (and vice-versa) — **repro under the binary's features.**
- **`workspace_root()` = `env!("CARGO_MANIFEST_DIR")/../..`** (compile-time-baked),
  so `spec/` discovery resolves to the repo identically in tests and the binary,
  regardless of CWD. (Not a runtime env var — don't suspect it for test-vs-binary
  path drift.)

## The verification ladder (weakest → strongest)

| # | Layer | Proves | Does NOT prove |
|---|-------|--------|----------------|
| 1 | **Logic unit tests** (`cargo test -p ui`) — `update`, `PanelState`, loaders | the DATA is right (`loader → Ready`) | that anything renders |
| 2 | **Text-summary snapshots** (`panel_snapshots.rs`, `*_summary()`) | widget-tree STRUCTURE (KPI present) | pixels, draw-panics — and fixtures here often hardcode `Empty`, so the **populated path is untested** |
| 3 | **cockpit-smoke skill** (boots real `cockpit`, greps stderr 7s) | no first-frame panic on the default screen | visual correctness; non-default screens |
| 4 | **Pixel-render harnesses** ⭐ — `iced_test::Emulator::screenshot` → tiny-skia RGBA readback | **the thing actually draws** | (this is the layer to use) |
| 5 | **The live window** — `cargo run -p ui --release --bin cockpit_live` | ground truth | (orchestrator can't see it headless except via #4) |

**Layer-4 harnesses that already exist** (copy their pattern): `render_snapshots.rs`,
`visual_snapshots.rs`, `live_equity_render.rs`, `reports_populated_curve_render.rs`,
`lab_binance_render.rs`, `headless_emulator_smoke.rs`. They count hue-pixels (robust
to cosmic-text font jitter) rather than byte-comparing PNGs, and are `#[cfg(target_os
= "macos")]`-gated for pixel determinism (ADR-0057).

## The rules (each one = a way the curve shipped blind)

1. **Verify the POPULATED state, not the empty one.** A green snapshot of `Empty`
   proves nothing about `Ready`. In the same test file, assert a **positive**
   (real/Ready data → many curve px) AND a **negative control** (Empty → few px), so
   the guard provably discriminates. Pattern: `reports_populated_curve_render.rs`
   (`…draws_in_detail_pane` >1000 px + `…empty_equity_draws_no_curve` <500 px).
2. **Read the actual PNG.** When a harness or sub-agent says "N curve px / it
   renders," `Read` the saved image and *look*. A pixel count is a claim, not proof.
3. **Proxies aren't proof.** `loader → Ready` (unit), `viewer` boots no-panic, text
   snapshot green — none means "the operator's screen shows it." Only #4/#5 do.
4. **Mind the feature flag.** Repro with the SAME features the operator runs
   (`cockpit_live` = default/live), not just `cargo test` (which adds `fixtures`). A
   render that works in the test build but not the binary is a feature-path
   divergence — grep the render path for `#[cfg(feature = …)]`.
5. **Empty-by-data ≠ bug.** A panel correctly shows Empty when the data genuinely
   isn't there (e.g. a report with no companion CSV). Decide whether you're debugging
   a render bug or an absent-data condition *before* touching the renderer.
6. **Render-correct ≠ usable.** The Reports curve was render-correct yet invisible
   because the one data-bearing report was 1 of 112 in the picker. Discoverability
   (marker, auto-select, default-to-non-empty) is part of "it works" — and is itself
   render-testable.

## Debugging an iced render bug — the loop

1. Extend a **pixel-render test** to load the representative state and render to a
   PNG. Make it **RED for the current bug first** (prove it catches the failure).
2. `Read` the PNG; compare to the negative control.
3. Renders in the test but not the live binary? → feature-flag/data-path divergence;
   repro with the binary's features (and check `cfg`-gating + the boot wiring in
   `bin/cockpit_live.rs` vs `bin/cockpit.rs`).
4. `cockpit-smoke` to rule out a boot panic.
5. Interaction bug (click/select)? Drive the `Message::X` path in a test, then render
   — don't assume `view` is even reached, or that the selected index maps as expected.
6. Live-window confirmation: computer-use `request_access` → `screenshot` (the
   binaries are unbundled, so request the process name), or hand the operator a
   recipe naming the exact row/label.

## Iced-0.14 gotchas seen in this repo

- `Emulator::screenshot` `take()`s its `UserInterface::Cache` and never restores it →
  a second `screenshot()` on the same emulator panics; construct a fresh emulator per
  frame (see `benches/cockpit_render.rs`).
- Multi-thread `std::env::set_var` is UB and caused flaky time-axis rendering — the
  test-time UTC override is an `AtomicBool`, not an env var.
- A transient `cosmic-text` glyph-shaping panic can poison the shared font mutex and
  persist a `crates/ui/tests/layout_invariants.proptest-regressions` cache → `rm` it
  and re-run; not a Reports/feature regression.
- The cockpit must run in `--release` (the dev `opt-level=0` tiny-skia path is ~40×
  slower → the "1–3 s per interaction" lag). See README "Run the cockpit".
