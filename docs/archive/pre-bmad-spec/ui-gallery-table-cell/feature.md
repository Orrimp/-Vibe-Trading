---
slug: ui-gallery-table-cell
status: draft
owner: analyst
updated: 2026-05-16
predecessor: ui-gallery-bin v0.1.0-partial-terminal
---

> **Supersedes
> [`ui-gallery-bin`](../v1/ui-gallery-bin/feature.md) (terminal at
> v0.1-partial, 2026-05-16).** That feature shipped V1–V4 green
> and is operator-accepted as terminal. This brief owns V5+:
> restoring the full gallery render path.

# Widget gallery — table-cell bounds fix (`ui-gallery-table-cell`) — v0.1

## Why

V5+ of [`ui-gallery-bin`](../v1/ui-gallery-bin/feature.md) is
blocked on a `tiny-skia` `Build quad rectangle` panic in
`widget::table::Table`, bisected to `GALLERY_CELLS[7]` (the
**strategies** cell) by
[`crates/ui/tests/gallery_bisect.rs`](../../crates/ui/tests/gallery_bisect.rs).
The iced 0.14 `widget::table::Table` used by
`widgets::strategies::view` interacts badly with the fixed-height
`cell::view` container in the gallery: bumping `CELL_HEIGHT_PX`
from 260 → 500 does **not** resolve. The gallery binary's V5+
snapshot tests are written
([`crates/ui/tests/gallery_snapshots.rs`](../../crates/ui/tests/gallery_snapshots.rs))
but `#[ignore]`d pending this fix. The operator accepted
v0.1-partial of `ui-gallery-bin` as terminal (per
[`docs/dev-notes/feature-triage-2026-05-16.md`](../../docs/dev-notes/feature-triage-2026-05-16.md)
row A4) and opened this successor brief to own the V5+ work.

Cycle-2 item E (`ui-test-harness-viewport-matrix`) is downstream
of a green gallery (see
[`ui-gallery-bin/feature.md ## Why`](../v1/ui-gallery-bin/feature.md)
context) — so closing this brief unblocks the next ui-testability
cycle.

## Requirements

**R1 — Restore V5+ render path.** The gallery binary
(`cargo run -p ui --bin ui-gallery --features fixtures`) must
render the full
[`GALLERY_CELLS`](../../crates/ui/src/gallery/mod.rs) set
(including `GALLERY_CELLS[7]` and any later cells using
`widget::table::Table`) without panicking, both interactively
and under
[`iced_test::screenshot`](../../crates/ui/tests/gallery_snapshots.rs).

**R2 — Snapshot tests un-ignored and green.** The three
`gallery_snapshots` tests
(`ui_gallery_dark_floor`, `_typical`, `_operator`) currently
`#[ignore]`d in
[`crates/ui/tests/gallery_snapshots.rs`](../../crates/ui/tests/gallery_snapshots.rs)
must drop the `#[ignore]` attribute and pass against committed
baselines at `crates/ui/tests/visual-baselines/`.

**R3 — Determinism preserved (V10 inherited).** Two consecutive
runs of the snapshot tests produce byte-identical baseline PNGs
(SHA-256 match) per the v0.1-partial V10 contract.

**R4 — Anchors PASS 11/11 (V8 inherited).** No backtest body-
SHA-256 anchor moves. `bash scripts/verify_anchors.sh` exits 0.

**R5 — No regression to V1–V4.** Build / `--smoke` / widget
exhaustiveness / mod-rs parity stay green; the chrome-widget
mods inventoried by V3/V4 in v0.1-partial remain covered.

## Design

_architect fills this_

## Backtest Scenarios

_n/a — pure UI feature; no backtest scenarios. Confirmed during
analyst spawn 2026-05-16 (Wave 2a spec-hygiene)._

## Implementation

_developer fills this_

## Verification

_tester links to reports here_

## Open questions for architect

1. **Q-FIX-STRATEGY** — Three candidate fix paths surfaced in
   [`ui-gallery-bin/tasks.md ## Status as of 2026-05-15`](../v1/ui-gallery-bin/tasks.md):
   (a) special-case the strategies cell wrapper inside
   `cell::view`; (b) swap `widget::table::Table` for a
   non-table render in the gallery only; (c) fix the
   table-cell bounds upstream (broader
   `ui-iced-table-cell-bounds-fix` scope). The 2026-05-15
   backlog Changelog already queued candidate
   `ui-iced-table-panic-upstream` (0.5d) to file the panic
   upstream. Architect picks one (or sequences them).
2. **Q-RENDER-SHAPE** — If (a) or (b) is chosen, does the
   strategies cell need to render the full v1 multi-row state
   (matches the
   [`panel_snapshots__positions_v1_three_rows`](../v1/v1-cross-sectional-momentum/feature.md#ui--v1)
   shape), or is a single-row representative acceptable?
3. **Q-CELL-HEIGHT** — `CELL_HEIGHT_PX` was tested at 260 and
   500 in v0.1-partial; both panic. Is there a deterministic
   probe height the gallery should adopt, or is the panic
   independent of cell height (suggesting a width / bounds
   issue rather than vertical)?

## Hypothesis register

_seeded by analyst; architect resolves at design pass_

- **H-TC-1 — Panic is a fixed-bounds vs intrinsic-size
  conflict.** Falsifier: a minimal repro that wraps
  `widget::table::Table` in a `Container::height(Length::Fixed(N))`
  for varied N and reports the first non-panicking N.
- **H-TC-2 — Panic is triggered by the empty-state branch
  of `widgets::strategies::view`.** Falsifier: render the
  strategies cell with `fake_cockpit_v1_steady_state()` (loaded
  rows) vs `fake_cockpit_loading()` (empty) and report which
  branch panics.

## Risks

_analyst seeds; architect refines_

- **R-TC-1 — Upstream-fix path lengthens scope.** If
  Q-FIX-STRATEGY selects (c), this brief inherits the
  `ui-iced-table-panic-upstream` (0.5d) cost; cycle-2 item E
  remains blocked until upstream lands.
- **R-TC-2 — V8 anchor drift.** Any fix that touches non-UI
  crates risks moving body-SHA-256 anchors. Mitigation:
  developer must keep changes inside `crates/ui/` and confirm
  via `git diff --stat` that no other crate is touched before
  ticking `[x]` on R4.

## Effort budget

_to be set by architect at design-pass; analyst preliminary
estimate: 0.5–1.5d depending on Q-FIX-STRATEGY resolution._

## Changelog

- 2026-05-16 (analyst, Wave 2a spec-hygiene): brief opened as
  the successor to
  [`ui-gallery-bin`](../v1/ui-gallery-bin/feature.md) per
  [`docs/dev-notes/feature-triage-2026-05-16.md`](../../docs/dev-notes/feature-triage-2026-05-16.md)
  row A4. Frontmatter: `status: draft`, `owner: analyst`,
  `predecessor: ui-gallery-bin v0.1.0-partial-terminal`. R1
  carries the load-bearing requirement verbatim from the
  triage. Q-FIX-STRATEGY surfaces the three candidate fix
  paths surfaced in the predecessor's tasks.md status block.
  HANDOFF → architect (design pass + Q-FIX-STRATEGY /
  Q-RENDER-SHAPE / Q-CELL-HEIGHT resolutions before T01 of
  tasks.md lands). No `reports/`, no `anchors.toml`, no
  `trace.toml` written by this spawn — pure spec.
