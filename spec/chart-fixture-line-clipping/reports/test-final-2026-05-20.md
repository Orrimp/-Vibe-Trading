---
title: Test Report — chart-fixture-line-clipping v1.0.0
feature: chart-fixture-line-clipping
run_id: 2026-05-20-0722-UTC
commit: working-tree-on-top-of-53a827c
agent: orchestrator-direct
verdict: PASS
---

# Test Report — chart-fixture-line-clipping v1.0.0 — 2026-05-20 07:22 UTC

## 1. Scope

- **Feature / change under test:** `chart-fixture-line-clipping v1.0.0` — fix the
  pre-existing chart canvas rendering bug where the price line only renders in
  the bottom-right ~quarter of the canvas widget bounds.
- **Spec refs:** `spec/chart-fixture-line-clipping/feature.md`,
  `spec/chart-fixture-line-clipping/tasks.md`.
- **Rust toolchain:** stable, edition 2024.
- **OS / arch:** Darwin 25.4.0 arm64.

## 2. Root Cause

iced 0.14.0 `tiny_skia` backend transformation-order bug in
`Renderer::draw_primitives` and `Layer::push_*`. Fixed upstream Jan 28, 2026 by
commit [`76b32d4906`](https://github.com/iced-rs/iced/commit/76b32d4906)
*"Fix transformation of `canvas` primitives in `tiny_skia`"* — but the fix is
post-0.14.0 and there is no 0.14.x patch release on crates.io as of today.

## 3. Fix Applied

Backported the upstream patch via vendored `iced_tiny_skia` + workspace
`[patch.crates-io]`:

- `vendor/iced_tiny_skia/src/layer.rs:271-275` — `Item::Cached` arm:
  `vec![*bounds * *transformation]` → `vec![*bounds]`
- `vendor/iced_tiny_skia/src/lib.rs:135-138` — drop the duplicate
  `group.transformation()` multiplier on the clip-bounds intersection.
- `vendor/iced_tiny_skia/src/lib.rs:148-149,185-186` — swap transformation
  order from `group.transformation() * Transformation::scale(scale_factor)` to
  `Transformation::scale(scale_factor) * group.transformation()` in both the
  primitive draw pass and the text draw pass.
- `Cargo.toml:99-104` — `[patch.crates-io]` directive:
  `iced_tiny_skia = { path = "vendor/iced_tiny_skia" }`.

## 4. Validation Matrix

| Check                                          | Result | Notes                                              |
|------------------------------------------------|--------|----------------------------------------------------|
| `cargo fmt --check`                            | PASS   | Zero diffs.                                        |
| `cargo clippy --workspace -- -D warnings`      | PASS   | Clean.                                             |
| `cargo test --workspace --lib`                 | PASS   | 279 tests passed.                                  |
| `cargo test -p ui --test visual_snapshots`     | PASS   | 4 PASS — chart line now spans full canvas.         |
| `cargo test -p ui --test render_snapshots`     | PASS   | 2 PASS + 5 ignored — chart_screen line correct.    |
| `scripts/verify_anchors.sh`                    | PASS   | 22 / 22 byte-identical.                            |
| `cockpit-smoke` (orchestrator, 8 s window)     | PASS   | 0 panic lines. Log:                                |
|                                                |        | `reports/cockpit-smoke-2026-05-20T07-22Z.log`      |
| `cargo test -p ui --doc`                       | PASS   | 1 ignored (axis.rs `pub(crate)` example).          |

## 5. Visual Baselines Refreshed

- `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`
- `crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`
- `crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`
- `crates/ui/tests/visual-baselines/render_snapshots/chart_screen_dark_typical.png`
- `crates/ui/tests/visual-baselines/render_snapshots/strategies_ready_dark_typical.png`

Two-run determinism verified on both `visual_snapshots` and `render_snapshots`.

## 6. Investigation Trail (preserved in feature.md)

- 5 initial hypothesis seeds (H-CHART-1 through H-CHART-5) — falsified.
- 2 failed fix attempts (outer Container wrap removal; Length::Fixed canvas
  height) — bug persists.
- Empirical probes (red rect fill, corner dots, inner-rect outline,
  bisected RED/GREEN half-path) — narrowed to "geometry rendered at correct
  positions but only the bottom-right portion is composited."
- Web research located the upstream iced fix.

## 7. Non-Regression Contract

- 22/22 body-SHA-256 anchors stay byte-identical (no strategy / audit / exec
  / report path touched).
- `cockpit-smoke` 0 panics.
- All 5 chart baselines refreshed; chart line spans full inner-rect width.

## 8. Verdict

**VERDICT → PASS**

## 9. Routing

Orchestrator commits + pushes; backlog updated; bug closed.

## 10. Out-of-band followups

- **Vendored iced_tiny_skia retirement.** When iced ships a 0.14.x patch
  release containing commit 76b32d4906, OR when we upgrade to a newer
  iced minor, drop `vendor/iced_tiny_skia/` and the `[patch.crates-io]`
  directive.
