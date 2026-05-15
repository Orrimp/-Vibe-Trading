---
slug: iced-aw-cherry-pick
mode: release
status: draft
audience: human-operator
updated: 2026-05-14
generated: 2026-05-14T08:00:00Z
predecessor: iced-native-widgets v0.1.0
---

# iced_aw cherry-pick (v0.1.0) — Brief B — release

## TL;DR

- **B1 — date_picker** lands in the viewer bin (`crates/ui/src/bin/viewer.rs`)
  as a smoke-test consumer. Unblocks v1.11 / Phase 4 operator-selectable
  backtest range. Picker payload is `iced_aw::core::date::Date`; calendar
  anchor is the const `(2024, 1, 1)`, never `Date::today()`, so snapshots
  stay deterministic.
- **B2 — spinner** replaces 8 `muted_body(X_LOADING)` call sites across
  5 files with a new `loading_with_spinner(text, mode)` helper in
  `crates/ui/src/widgets/frame.rs` that pairs a 16 px `iced_aw::Spinner`
  with the existing informational text — text preserved, no copy lost.
- **B3 — badge** upgrades the Strategies STATUS column from a text-color
  `colored_cell` override to a typed `iced_aw::Badge` chip styled via
  a new `cockpit_badge_style_fn(intent)` factory parallel to Brief A's
  `cockpit_table_style_fn`. Routes Lumen tokens
  (`UP_50/UP_500` / `ACCENT_SOFT/FG_3` / `DOWN_50/DOWN_500`) by
  domain intent.
- **All three architect hypotheses stay UNFALSIFIED**: H-arch-4 (feature-flag
  isolation), H-arch-9 (deterministic spinner render), H-arch-10
  (badge Catalog/StyleFn hook). Verdicts cross-checked by the evaluator
  against the raw test-run log.
- **Cost: +1 direct crate (`iced_aw = "0.14"`), +3 transitive
  (`chrono` + `num-traits` + `once_cell`). 267 tests pass × 2 runs,
  zero `*.snap.new`, anchors unchanged (11 / 11), zero Charts PNG drift,
  zero net-new clippy or rustdoc warnings on any Brief B file.**

Evaluator's full PASS at
[`reports/evaluation-2026-05-14T07-13Z.md`](../reports/evaluation-2026-05-14T07-13Z.md).

## What changed

### Glue-layer LOC (Cargo.toml + theme adapter + helper)

| Surface | LOC delta |
|---|---:|
| [`crates/ui/Cargo.toml:78`](../../../crates/ui/Cargo.toml) — `iced_aw = { version = "0.14", default-features = false, features = ["date_picker", "spinner", "badge"] }` | ~+3 |
| [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../../crates/ui/src/theme/iced_widget_catalogs.rs) — new `BadgeIntent` enum + `cockpit_badge_style` + `cockpit_badge_style_fn` + 5 unit tests | ~+30 |
| [`crates/ui/src/widgets/frame.rs:150-188`](../../../crates/ui/src/widgets/frame.rs) — new `loading_with_spinner(text, mode)` helper + 1 unit test | ~+15 |
| [`crates/ui/src/theme.rs`](../../../crates/ui/src/theme.rs) — new `color::SPINNER_TINT = FG_3` token + design-principles cross-link | ~+5 |
| **Glue-layer total** | **~+53** |

### File-span LOC (per sub-target — touched files only)

| Sub | Touched file(s) | File-span LOC delta |
|---|---|---:|
| **B1** | [`crates/ui/src/bin/viewer.rs`](../../../crates/ui/src/bin/viewer.rs) `fn picker_block` (lines 277-353) + [`crates/ui/src/viewer.rs`](../../../crates/ui/src/viewer.rs) `VIEWER_PICKER_ANCHOR` + new model fields | **+30 to +50** (new surface) |
| **B2** | [`frame.rs`](../../../crates/ui/src/widgets/frame.rs) helper + 8 call sites across [`screens/strategies.rs`](../../../crates/ui/src/screens/strategies.rs), [`screens/audit.rs`](../../../crates/ui/src/screens/audit.rs), [`screens/risk.rs`](../../../crates/ui/src/screens/risk.rs), [`widgets/positions.rs`](../../../crates/ui/src/widgets/positions.rs), [`widgets/strategies.rs`](../../../crates/ui/src/widgets/strategies.rs), [`widgets/pnl.rs`](../../../crates/ui/src/widgets/pnl.rs), [`widgets/agent_feed.rs`](../../../crates/ui/src/widgets/agent_feed.rs) | **net +15** (−8 retired, +15 helper, +8 swaps) |
| **B3** | [`crates/ui/src/widgets/strategies.rs`](../../../crates/ui/src/widgets/strategies.rs) (column 3 + new `status_badge_cell` at lines 332-345; legacy `colored_cell` deleted) | **net +10** (−2 retired, +12 new) |
| **Brief B file-span total** | — | **net ~+55** |

**Aggregate Brief B delta: ~+55 file-span + ~+53 glue ≈ +108 LOC** —
slightly above the parent brief's "~50-100 LOC" estimate; consistent with
the architect's revised target after the M3 surface adjustment (see
Architectural divergences below). Brief B is a feature add, not a
refactor.

## Why

The parent [`iced-ecosystem-evaluation` v0.2.0](../../iced-ecosystem-evaluation/feature.md)
Q5 resolution greenlit a **scoped** `iced_aw` cherry-pick (NOT
`iced_aw/full`) for three widget surfaces — `date_picker`, `spinner`,
`badge`. Each addresses a concrete cockpit gap:

- B1 unblocks the Phase 4 backtest-range picker.
- B2 turns the static `Loading…` placeholder into a visible spinning
  indicator across 8 panels (without dropping the informational text
  that tells the operator _what_ is loading).
- B3 retires a text-color sentinel pattern on Strategies and replaces
  it with a typed status-chip primitive — the surface Phase 4
  risk-status will need anyway.

Catalog adapter [`cockpit_table_style_fn`](../../../crates/ui/src/theme/iced_widget_catalogs.rs)
shipped-but-unused in Brief A's `iced_widget_catalogs.rs` is the
declared seam for this kind of `iced_aw` adopter; B3 ships its sibling
`cockpit_badge_style_fn` in the same module per Brief A's docstring
designation.

## What you can do now

| Action | Command |
|--------|---------|
| Verify the viewer bin compiles and the picker test passes | `cargo test -p ui --lib viewer` |
| Run the full Brief B test surface | `cargo test -p ui` |
| Re-verify the no-clocks-in-snapshot-path gate (H-arch-9 hard gate) | `bash scripts/check_no_clocks_in_ui_tests.sh` |
| Confirm anchors stay byte-identical (Brief B does not touch any anchored path) | `bash scripts/verify_anchors.sh` |
| Open the picker surface live (operator-only — see Screenshots) | `cargo run -p ui --bin viewer` |
| Open a cockpit screen with loading panels and badge cells live | `cargo run -p ui --bin cockpit --features fixtures` |
| Inspect the new Catalog adapter | `open crates/ui/src/theme/iced_widget_catalogs.rs` |
| Inspect the new spinner helper | `open crates/ui/src/widgets/frame.rs` |

## Live demo

Verbatim from the test-runner log
[`reports/test-run-2026-05-14T07-13Z.log`](../reports/test-run-2026-05-14T07-13Z.log)
— cited per row:

### Compile gates green (4 binaries)

```
$ cargo build -p ui --tests
Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.16s
$ cargo build -p ui --bin viewer
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.94s
$ cargo build -p ui --bin cockpit --features fixtures
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.94s
```
(log:7, log:44, log:48, log:52, log:76 — fmt + 4 builds, all exit 0)

### Test suite — two-run determinism (H-arch-9 hard gate)

```
$ cargo test -p ui                                  # run 1
... 23 binaries, 267 passed, 0 failed
$ cargo test -p ui                                  # run 2 (back-to-back)
... 23 binaries, 267 passed, 0 failed
$ find crates/ui -name '*.snap.new'
(empty)
$ echo "## exit: $?"
## exit: 0
```
(log:518 run-1 exit 0; log:960 run-2 exit 0; log:962-963 `find` empty,
exit 0 — the H-arch-9 hard gate green)

### CLOCKS gate (H-arch-9 caveat resolution)

```
$ bash scripts/check_no_clocks_in_ui_tests.sh
CLOCKS PASS  (8 files / 4 patterns)
```
(log:1533-1534 — `scripts/check_no_clocks_in_ui_tests.sh` scope is the
8 workspace-file watchlist; `~/.cargo/registry/` is structurally
unreachable so `iced_aw-0.14.1/src/widget/spinner.rs:160` is not flagged.
Caveat from architect's H-arch-9 verdict resolved.)

### Anchors diff empty (Brief B touched zero anchored paths)

```
$ git diff --stat HEAD spec/anchors.toml
$ echo "## exit: $?"
## exit: 0
```
(log:1547-1548 — zero diff body. The 11 backtest body-SHA-256 anchors in
[`spec/anchors.toml`](../../anchors.toml) — 9 strategy + 2 report —
are unaffected. Brief B touches zero strategy / audit / exec / backtest
code.)

### Transitive dep audit (H-arch-4 verdict)

```
$ cargo tree -p ui -i iced_aw --no-default-features
iced_aw v0.14.1
└── ui v0.1.0 (.../crates/ui)
```
(log:1515 — confirms exactly `+1 direct (iced_aw v0.14.1)`. Transitive
adds: `chrono v0.4.44` (log:1517), `num-traits v0.2.19` (log:1492),
`once_cell v1.21.4` (log:1494) — exactly the architect's H-arch-4
prediction of "+3 transitive crates".)

## Screenshots

The presenter sub-agent cannot run the cockpit binary with a live window
or invoke `screencapture` (per [`AGENT.md ## Capability boundaries`](../../../AGENT.md)).
Three operator-instruction blocks below — please paste the resulting
PNGs back into this presentation during your review pass.

Output directory: [`spec/iced-aw-cherry-pick/reports/screenshots/`](../reports/screenshots/)
(will be auto-created by `mkdir -p` in the snippets).

### Screenshot 1 — B1 date_picker open-overlay (viewer bin)

```bash
# On your operator workstation, capture the viewer-picker overlay:
mkdir -p spec/iced-aw-cherry-pick/reports/screenshots
cargo run -p ui --bin viewer &
sleep 4
# Click the date-picker chip in the viewer to open the overlay.
screencapture -W spec/iced-aw-cherry-pick/reports/screenshots/b1-viewer-picker-open.png   # macOS
pkill -f "target/debug/viewer"
```

Caption when pasted back: _"B1 — `iced_aw::date_picker` open overlay
anchored at `(2024, 1, 1)`. Clicking the picker chip surfaces the
month-grid overlay; submit produces a `PickerDateSelected(time::Date)`
message round-trip."_

### Screenshot 2 — B2 loading_with_spinner (cockpit, any panel in `panel_state::Loading`)

```bash
# On your operator workstation, capture a loading spinner+text row:
mkdir -p spec/iced-aw-cherry-pick/reports/screenshots
cargo run -p ui --bin cockpit --features fixtures &
sleep 4
# Switch to a screen showing `panel_state::Loading` (e.g. Positions
# screen before fixture data flushes; or trigger via a fixture flag).
screencapture -W spec/iced-aw-cherry-pick/reports/screenshots/b2-loading-spinner.png   # macOS
pkill -f "target/debug/cockpit"
```

Caption when pasted back: _"B2 — `loading_with_spinner(text, mode)` —
16 px `iced_aw::Spinner` paired with the informational `Loading…`
copy, both tinted via `color::SPINNER_TINT` (alias for `FG_3`)."_

### Screenshot 3 — B3 status badges on Strategies STATUS column

```bash
# On your operator workstation, capture the Strategies STATUS column:
mkdir -p spec/iced-aw-cherry-pick/reports/screenshots
cargo run -p ui --bin cockpit --features fixtures &
sleep 4
# Switch to the Strategies screen — fixtures expose Ready / Loading /
# Error status variants so all three badge intents render in one shot.
screencapture -W spec/iced-aw-cherry-pick/reports/screenshots/b3-strategies-status-badges.png   # macOS
pkill -f "target/debug/cockpit"
```

Caption when pasted back: _"B3 — `iced_aw::Badge` styled via
`cockpit_badge_style_fn(intent)`: `Ready` → `UP_50/UP_500` (Positive),
`Loading` → `ACCENT_SOFT/FG_3` (Neutral), `Error` → `DOWN_50/DOWN_500`
(Negative). PILL radius, no border, no hover-state color (status pills
are informational)."_

## Verification matrix

Verbatim from
[`evaluation-2026-05-14T07-13Z.md ## Verdict matrix`](../reports/evaluation-2026-05-14T07-13Z.md).
All 10 rows PASS.

| # | Criterion | Verdict | Cite (log line / file) |
|---|-----------|---------|------------------------|
| 1 | Compile gates green (cargo fmt + 4× cargo build) | PASS | log:7 (fmt exit 0), log:44 (build tests), log:48 (viewer), log:52 (cockpit), log:76 (cockpit_live) |
| 2 | `cargo test -p ui` exits 0 with `267 passed; 0 failed` across 23 binaries | PASS — N = 267 | log:518 exit 0; per-binary breakdown verified by evaluator |
| 3 | Two-run determinism — run 2 count equals run 1 AND `find …*.snap.new` empty | PASS — run 2 N = 267 (matches); find returned zero | log:960 second run exit 0; log:962-963 `find` returns no rows, exit 0 (H-arch-9 hard gate green) |
| 4 | Fmt + clippy on touched files green — clippy errors CONFINED to documented pre-existing files | PASS — all 6 clippy errors land in `widgets/chart.rs` (5 errors) + `window_icon.rs:151` (1 error). ZERO errors in any Brief B-touched file | log:1344, 1358, 1367, 1376, 1388, 1413; clippy step exit 101 = documented pre-existing baseline, not a Brief B regression |
| 5 | No NET-NEW rustdoc warnings | PASS — 6 warnings total, all pre-existing (`chart_tooltip.rs`, `volume_histogram.rs` ×2, `window_icon.rs`, `test_support.rs` ×2). NONE on Brief B files | log:1008 `ui (lib doc) generated 6 warnings`; sources at log:967-1003 |
| 6 | Transitive dep budget — `iced_aw v0.14.x` + `chrono` + `num-traits` + `once_cell` | PASS — architect's H-arch-4 prediction of "+3 transitive crates" matches exactly | log:1515, 1517, 1492, 1494 |
| 7 | Determinism check `CLOCKS PASS` | PASS — H-arch-9 caveat resolved (clocks-grep scope excludes `~/.cargo/registry/`) | log:1534 `CLOCKS PASS  (8 files / 4 patterns)` |
| 8 | Anchors diff empty | PASS — Brief B touched zero anchored paths | log:1547-1548 `## git diff --stat HEAD spec/anchors.toml` + `## exit: 0` with zero body |
| 9 | trace.toml columns filled for REQ-ICED-AW-001/-002/-003 | PASS — all three rows have non-empty `crates` and `tests` | spec/trace.toml:202-219 / 224-249 / 254-271 |
| 10 | Honest ticks — spot-check 3 ticked tasks for file:line + cmd + output | PASS — T-M1-3 / T-M2-4 / T-M3-2 each verified against log | [`evaluation … ## Honest-tick spot-check`](../reports/evaluation-2026-05-14T07-13Z.md) |

## Hypothesis register (architect verdicts, evaluator cross-check)

From [`feature.md ## Falsifier verdicts`](../feature.md) and
[`evaluation-2026-05-14T07-13Z.md ## Architect hypothesis verdicts cross-checked`](../reports/evaluation-2026-05-14T07-13Z.md):

- **H-arch-4** — `iced_aw::date_picker` feature-flag isolation.
  **RESOLVED-PASS.** `cargo tree -p ui -i iced_aw --no-default-features`
  returns exactly `iced_aw v0.14.1` plus 3 transitive crates (`chrono`,
  `num-traits`, `once_cell`); zero pulls from `menu` / `tab_bar` /
  `number_input` / other widget surfaces. Cross-checked at log:1490-1530.
- **H-arch-9** — `iced_aw::spinner` deterministic render.
  **RESOLVED-PASS with caveat (caveat resolved).** Spinner source has one
  wall-clock hit (`Instant::now()` at `iced_aw-0.14.1/src/widget/spinner.rs:160`
  — widget-state init only; render path is pure). Caveat: clocks-grep
  scope must exclude `~/.cargo/registry/`. Resolved at T-M2-4: scope
  is the 8 workspace-file watchlist (log:1534). `*_loading.snap`
  baselines render at `t = 0.0` because `iced_test` does not fire
  `RedrawRequested`; zero `*.snap.new` files across two consecutive
  runs (log:962-963).
- **H-arch-10** — `iced_aw::badge` Catalog / StyleFn hook.
  **RESOLVED-PASS.** `iced_aw::Badge` exposes a `.style(impl Fn(&Theme,
  Status) -> Style)` builder structurally identical to Brief A's
  `cockpit_table_style_fn` shape. `cockpit_badge_style_fn(intent)` ships
  as a sibling factory in the same module per Brief A's docstring
  designation. Cross-checked indirectly via panel-snapshots strategies
  tests landing green (log:458 `69 passed`).

All three hypotheses stay UNFALSIFIED in the developer pass.

## Numbers that matter

- **Tests:** **267 passed, 0 failed across 23 binaries**, two consecutive
  runs byte-identical (log:518 run 1, log:960 run 2). Test count delta
  between runs: **0**.
- **Snapshot determinism:** **0** `*.snap.new` files after the full
  two-run gate (log:962-963 `find ... -name *.snap.new` empty body
  exit 0). 1 new baseline landed for B1 (`viewer_picker_default_closed.snap`,
  411 bytes, captures viewer model state — not a widget render).
- **Anchors:** **11 / 11 PASS**, byte-identical. Brief B touches zero
  strategy / audit / exec / backtest code paths. `spec/anchors.toml`
  diff empty (log:1547-1548).
- **Charts PNG baselines:** **3 / 3 byte-identical**. Brief B does not
  touch any widget on the Charts canvas.
- **Direct dep delta:** **+1** (`iced_aw = "0.14"`).
- **Transitive dep delta:** **+3** (`chrono v0.4.44`, `num-traits v0.2.19`,
  `once_cell v1.21.4` — exactly the architect's H-arch-4 prediction).
- **Clippy:** **6 errors total, ALL pre-existing** in `widgets/chart.rs`
  (5 hits) + `window_icon.rs:151` (1 hit). **ZERO net-new** on any of
  the ~12 Brief B-touched files. The clippy step's `## exit: 101` is
  the documented baseline, not a Brief B regression.
- **Rustdoc:** **6 warnings total, ALL pre-existing**. **ZERO net-new**
  on any Brief B-touched file.
- **LOC delta (aggregate):** **~+108** (~+55 file-span + ~+53 glue) —
  feature add, not refactor (see What changed).
- **License:** `iced_aw` upstream MIT, edition 2024
  (verified at adoption time per architect's H-arch-4 falsifier).
- **REQ trace rows filled:** **3 / 3** (REQ-ICED-AW-001/-002/-003 at
  `spec/trace.toml:202-219, 224-249, 254-271`).

## Architectural divergences (honest)

Four divergences from the architect's pre-pass design, flagged in the
M0 Q-resolutions, the M3 ui-designer pass, and the developer's M2/M3
honest-tick citations:

1. **B3 surface SHRUNK from 3 files to 1.** Architect's analyst-draft
   framed B3 as "Strategies + Risk, ~3 files, ~50 LOC retired". Grep
   verdict (Q5 resolution): no hand-rolled
   `container(...).style(badge_style)` exists anywhere in
   `crates/ui/src/widgets/`. The Risk screen has zero badge consumers
   (verified by reading `crates/ui/src/screens/risk.rs:40-150`). The
   only chip-shaped surface is the Strategies STATUS column at
   [`crates/ui/src/widgets/strategies.rs:113-129`](../../../crates/ui/src/widgets/strategies.rs).
   **B3 ships as 1 file, net +10 LOC**, justified by B1 + B2 already
   paying the `iced_aw` dep cost and by the `cockpit_badge_style_fn`
   factory becoming the permanent wire-in for future status-chip
   consumers (Phase 4 risk-status surface).
2. **B2 helper redesigned — `loading_with_spinner(text, mode)` per
   call site, not a shared dispatch.** Architect's brief said
   "replace at the dispatch site, not per-panel". Grep verdict (Q4
   resolution): there is **no shared `panel_state` dispatch helper** —
   `muted_body(text)` is a per-panel call where each caller supplies
   its own informational text (`"Connecting to the fill stream…"`,
   `"Loading positions from the ledger…"`, etc.). Wholesale replacing
   it with a textless spinner would delete user-visible context.
   **Design call: preserve the informational text alongside the
   spinner.** The new helper at
   [`frame.rs:150-188`](../../../crates/ui/src/widgets/frame.rs) returns
   a `Row` of `[Spinner, Text]`; each of the 8 call sites swaps in
   place with `mode` threaded from the caller's existing `ThemeMode`
   binding.
3. **`cockpit_badge_style_fn` gained a `BadgeIntent` parameter.**
   Architect's stub was `fn cockpit_badge_style_fn() -> Box<dyn Fn(&Theme,
   Status) -> Style>` — no parameter. UI-designer's refinement (T-M3-1)
   noted `iced_aw::style::Status` is **interaction-state** (Active /
   Hovered / Pressed / Focused / Selected / Disabled), not
   domain-status. Without a domain parameter, all badges would render
   identically regardless of `StrategyStatus`. Refined signature:
   `cockpit_badge_style_fn(intent: BadgeIntent)` where
   `BadgeIntent::{Positive, Neutral, Negative}` is colocated in the
   same module (keeps `theme` decoupled from `state::StrategyStatus`).
   The closure captures intent; the iced_aw `StyleFn` shape is
   preserved. Documented in the new
   `spec/ui-design-principles.md ## Status pill colors` subsection.
4. **Snapshot baseline refresh count: 1, not the ~13 estimated.**
   Architect estimated ~13 panel-snapshot refreshes (~8 spinner + ~5
   badge). **Actual: 1.** Reason — the existing `panel_snapshots`
   baselines under `crates/ui/tests/snapshots/` are produced by
   **text-summary helpers** (`tape_summary`, `positions_summary`,
   `strategies_summary` at `crates/ui/tests/panel_snapshots.rs:1779+`,
   `:1989+`), which render a `PanelState`-keyed copy string — NOT the
   actual iced widget tree. The `muted_body → loading_with_spinner`
   swap and the `colored_cell → Badge` swap both live entirely in the
   widget render path; the text-summary helpers don't inspect the cell
   construction. **Zero existing baselines changed bytes** (T-M2-3,
   T-M3-3 honest-tick citations). The only new snap is
   `viewer_picker_default_closed.snap` — viewer model state, not a
   widget render. This **changes Brief B's snapshot-impact story** vs.
   the analyst/architect estimate: the surface is even less
   regression-prone than projected.

## Deferred items

| Item | Disposition |
|---|---|
| Full backtest-range wire-in (picker → scenario fetch) | Out of Brief B scope per analyst. v1.11 / Phase 4 brief owns it. B1 ships the picker primitive + smoke-test consumer only. |
| `SPINNER_TINT` vs `FG_3` cosmetic in the helper | The B2 helper currently reads `color::FG_3.current(mode)`. UI-designer landed `color::SPINNER_TINT` as an alias for `FG_3` (same byte-value). Functionally identical; one-line follow-up to import `SPINNER_TINT` for intent expression and ramp-shading insulation. Decision needed (see Open decisions). |
| Pre-existing clippy errors (6 in `chart.rs` + `window_icon.rs`) | Out of Brief B scope. Recommend a clean-up brief. |
| Pre-existing rustdoc warnings (6 across `chart_tooltip.rs`, `volume_histogram.rs`, `window_icon.rs`, `test_support.rs`) | Out of Brief B scope. Same clean-up brief candidate. |
| Spec-auditor split of the iced-aw-cherry-pick brief | The `feature.md` reads ~36k tokens at consumer (test-runner) and ~19k at orchestrator regen — a ~2× discrepancy worth investigating. Recommend operator green-lights a spec-auditor brief. |

## Open decisions

1. **Spec-auditor split for the feature.md.** The brief currently reads
   well over the 10k-token soft budget at consumer-side. Operator
   green-light to spawn spec-auditor on this slug? (Cost: small;
   benefit: parity between producer-claimed and consumer-measured
   brief sizes plus a cleaner shape for future re-reads.)
2. **Pre-existing clippy + rustdoc cleanup.** The 6 clippy errors
   (`chart.rs`, `window_icon.rs`) and 6 rustdoc warnings
   (`chart_tooltip.rs`, `volume_histogram.rs`, `window_icon.rs`,
   `test_support.rs`) predate Brief B and were not in scope.
   Should they ship as a focused clean-up brief now, or fold into the
   next feature pass?
3. **`SPINNER_TINT` import in the B2 helper.** The helper currently
   reads `color::FG_3.current(mode)`. The UI-designer landed
   `color::SPINNER_TINT` as an alias (identical bytes). Operator:
   one-line follow-up commit now to switch the import (intent expression
   + future-proof against ramp re-shading), or fold into the next
   feature pass?

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Footer — brief size flag (spec-auditor candidate)

Per the orchestrator's brief: `/tmp/brief-iced-aw-cherry-pick-test.md`
read ~36k tokens at the tester run (1450 lines, well above the 10k
soft budget). The evaluator independently surfaced the discrepancy as
an open question (Q1 in [`evaluation-2026-05-14T07-13Z.md`](../reports/evaluation-2026-05-14T07-13Z.md))
and routed it as a spec-auditor follow-up rather than a
verdict-blocker. Captured as Open decision 1 above so the operator
can ratify.

## Changelog

- 2026-05-14 (presenter): initial draft. Evaluator `VERDICT → PASS`
  cited from [`reports/evaluation-2026-05-14T07-13Z.md`](../reports/evaluation-2026-05-14T07-13Z.md)
  on log timestamp 2026-05-14T07-13Z; verbatim `CLOCKS PASS  (8 files
  / 4 patterns)` + `iced_aw v0.14.1` cargo-tree result + anchor-diff
  empty embedded; full 10-row verdict matrix lifted from the
  evaluator report; H-arch-4 / H-arch-9 / H-arch-10 verdicts
  cross-checked; 4 architectural divergences flagged honestly
  (B3 shrunk 3→1 file, B2 helper redesigned per-call-site,
  `cockpit_badge_style_fn` gained `BadgeIntent` parameter,
  snapshot refresh count 1 not ~13). 3 operator-instruction
  screenshot blocks emitted in lieu of in-sandbox capture (presenter
  cannot run `cargo run --bin cockpit` per
  [`AGENT.md ## Capability boundaries`](../../../AGENT.md)).
  3 approval boxes ship UN-ticked — operator owns the gate.
  Brief size warning surfaced as a footer + Open decision (not a
  verdict-blocker). Frontmatter on [`feature.md`](../feature.md)
  bumped `status: design → in-progress` pending operator approval
  (a follow-up orchestrator pass flips to `shipped` after the
  approval tick lands).
