---
slug: chart-fixture-line-clipping
status: shipped
owner: operator
updated: 2026-05-20
---

# Tasks — chart-fixture-line-clipping

> Analyst pass deferred — see `feature.md` for the full diagnostic.
> Orchestrator's failed-fix attempts (2026-05-20) narrowed the scope:
> the bug is NOT a simple Length::Fill nesting issue. Needs deeper
> iced 0.14 / tiny-skia investigation.

## M0 — Analyst synthesis (pending operator promotion)

- [ ] Read the diagnostic register in `feature.md` (F-CHART-1 through
  F-CHART-3 hypotheses, plus the two falsified fix attempts).
- [ ] Reproduce the bug locally:
  `cargo test -p ui --test visual_snapshots charts_screen_dark_typical`
  + visual inspection of
  `crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`.
  Confirms: price line visible only from screen x ≈ 1290 onwards
  (bar idx ≈ 38+); upper-half of canvas + left ~10% are clipped.
- [ ] Apply the probe block documented in `feature.md ## Investigation
  evidence on disk` to confirm the clip rect coordinates.
- [ ] Investigate iced 0.14 `Canvas` widget source:
  - `iced::widget::canvas::Canvas::draw`
  - `iced::widget::canvas::Frame::new`
  - `iced_tiny_skia` compositor scissor / clip-rect handling
- [ ] Surface a falsifier per hypothesis (F-CHART-1/2/3) so the fix
  can be verified.

## M-FINAL — Tester sweep (post-fix)

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100% PASS.
- [ ] `cargo test -p ui --test visual_snapshots` 4 PASS — **with
  refreshed baselines** that show the full chart-line spanning the
  inner-rect's full width (current baselines have the bug baked in).
- [ ] `scripts/verify_anchors.sh` → ANCHORS PASS (22/22).
- [ ] `cockpit-smoke` → 0 panic lines in 8 s window.
- [ ] Operator visual verification on live cockpit: Lab screen chart
  shows price line from leftmost time-axis label to rightmost.
- [ ] Author `spec/chart-fixture-line-clipping/reports/test-final-<YYYY-MM-DD>.md`
  per the test-report template.

## Notes

- The fix MUST refresh the 3 `charts_screen_dark_*.png` baselines +
  the 2 `render_snapshots` chart baselines because the existing
  baselines were captured WITH the bug present.
- 22 anchors stay byte-identical (no strategy / audit / exec / report
  path is affected).
