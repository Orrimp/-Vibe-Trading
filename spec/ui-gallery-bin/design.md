---
slug: ui-gallery-bin
type: design-addendum
parent: ui-gallery-bin/feature.md
version: 0.1.0
owner: architect
updated: 2026-05-15
---

# Design addendum — `ui-gallery-bin` v0.1

> Resolves the six **Q-ARCH-N** questions raised in
> [`feature.md ## Open questions for architect`](feature.md#open-questions-for-architect)
> + closes **H-GAL-2** (load-bearing) via a source-read of
> `iced_test = "0.14.0"` (no in-repo spike needed). The analyst's
> brief stands; this addendum mutates the **route table** module
> location (now `crates/ui/src/gallery/`), the **mega-canvas
> rendering** (column-no-scrollable, viewport sized to content), and
> the **mod-rs cross-check** mechanism. tasks.md delta at the bottom
> — developer applies the delta as first step in M0 close.

## Decisions

### Q-ARCH-1 — `--smoke` CLI parse: `clap::Parser` derive

> *"`clap::Parser` derive or hand-rolled `std::env::args` check?"*

**Resolution.** `clap::Parser` derive with a single
`#[arg(long)] smoke: bool` field. The
[`viewer` bin](../../crates/ui/src/bin/viewer.rs#L39-L83)
is the **existence proof** that `clap::Parser::parse()` +
`iced::application(...).run()` co-exist cleanly on macOS:
`clap::Parser::parse()` runs and returns BEFORE
`iced::application(...)` is constructed; iced takes the main
runloop afterwards. No overlap. The macOS concern raised in
the briefing is refuted.

**Rationale.** `clap` is already non-optional at
[`crates/ui/Cargo.toml:62-65`](../../crates/ui/Cargo.toml).
Hand-rolled `std::env::args` lacks `--help` text and a
parse-failure exit-code contract.

**Impact on tasks.md.** T03 keeps the strawman. No new T-item.

---

### Q-ARCH-2 — Exhaustiveness mod-rs parse: `include_str!` + pure-stdlib parse

> *"Options: (a) `include_str!` + regex, (b) `build.rs`,
> (c) proc-macro."*

**Resolution.** Option (a), **but no `regex` crate**. Pure
stdlib: `include_str!("widgets/mod.rs")` + per-line
`line.trim_start().strip_prefix("pub mod ").and_then(|s|
s.strip_suffix(";"))`. The `strip_prefix("pub mod ")` matches
ONLY `pub mod NAME;` lines, NOT `pub(crate) mod NAME;` — this
omits `canvas_chart` (which is `pub(crate)` per
[`widgets/mod.rs:13`](../../crates/ui/src/widgets/mod.rs)) from
the cross-check set, matching the gallery's "agent-visible public
surface" contract.

**Rationale.** Confirmed via grep that no `#[cfg]`-gated mod
declarations exist in `widgets/mod.rs`. `build.rs` and proc-macro
are over-engineered for 24 modules. ~25 LOC of `std::str` no-deps
keeps the test sandbox-pure.

**Impact on tasks.md.** T16 keeps the include_str! mechanism;
update its regex strawman to the stdlib chain (see § tasks.md
delta). `canvas_chart` is intentionally NOT in
`EXPECTED_WIDGETS`. Estimate unchanged.

---

### Q-ARCH-3 — Mega-canvas: `column!` (no scrollable), viewport sized to intrinsic content height

> *"Single mega-canvas height for the operator slot. H-GAL-2 is the
> load-bearing falsifier."*

**Resolution.** **H-GAL-2 FALSIFIED** by source-read of
[`iced_test-0.14.0/src/emulator.rs:444-498`](#)
(local source at
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_test-0.14.0/src/emulator.rs`).
The relevant code:

```rust
let mut user_interface = UserInterface::build(
    program.view(&self.state, self.window),
    self.size,                          // viewport — layout LIMIT
    /* ... */
);
let physical_size = Size::new(
    (self.size.width  * scale_factor).round() as u32,
    (self.size.height * scale_factor).round() as u32,
);
let rgba = self.renderer.screenshot(physical_size, ...);
```

The PNG dimensions are **exactly `viewport × scale_factor`**.
Scrollable content beyond the viewport is clipped during layout;
the renderer writes exactly `physical_size` pixels. The feature.md
design (mega-canvas in `scrollable`, baseline at 1280×720 capturing
24 cells) is mechanically impossible.

**Fix.** Three-part:

1. The gallery's `view_all()` returns `column!(...)` of cells —
   **no `scrollable` wrapper**.
2. The snapshot test passes viewport `(slot_w, GALLERY_LOGICAL_HEIGHT)`
   where `GALLERY_LOGICAL_HEIGHT = 6_400` (a `pub const` in
   `gallery/mod.rs` analytically computed: 24 cells × ~250 px avg
   + headers + padding).
3. The **interactive bin** (operator-run) keeps `scrollable(view_all(...))`
   for normal-window UX. Only the **snapshot test** path drops the
   scrollable. This is allowed because sub-agents only run the
   snapshot test; the bin is operator-only (post-VERDICT presenter
   capture).

**PNG-size estimate (per Risk R-DESIGN-1).**

| slot     | viewport (logical) | scale | physical PNG  | est. zlib size |
|---|---|---|---|---|
| floor    | 1280 × 6400        | 1.0×  | 1280 × 6400   | ~1.0–2.0 MB    |
| typical  | 1920 × 6400        | 1.0×  | 1920 × 6400   | ~1.5–3.0 MB    |
| operator | 3360 × 6400        | 2.0×  | 6720 × 12800  | ~4.0–7.0 MB    |

Operator-slot RGBA in memory ≈ 330 MB during screenshot — see
Risk R-DESIGN-2 for the 1.5× fallback if iced/CI hardware runs out
of memory.

**Impact on tasks.md.** T-M0-S ticks as FALSIFIED-via-source-read
(no spike). T02 returns `column!`, NOT `scrollable(column!)`. T03
bin keeps scrollable. T17 passes `(slot_w, GALLERY_LOGICAL_HEIGHT)`
not `(slot_w, slot_h)`. NEW T02b: unit test asserting every cell
Container has `height(Length::Fixed(N))` (no `Length::Fill` —
collapsing 24 cells to share viewport height would defeat the
fix). Estimate +0.05d.

---

### Q-ARCH-4 — Cell render-failure: no panic-catching

> *"Does the gallery still render the surrounding cells if a
> fixture builder panics? Strawman: each cell wraps `Container`,
> no catch_unwind."*

**Resolution.** **Strawman confirmed.** No `std::panic::catch_unwind`.
The cockpit's existing tolerance (panic kills the iced runtime)
is the precedent; the gallery doesn't diverge. V3 (exhaustiveness)
catches missing cells; V6 (snapshot) catches rendering regressions;
V2 (`--smoke`) catches panicking fixtures before operator capture.
`catch_unwind` across iced's render thread is fragile — iced 0.14
doesn't promise unwind-safety, and panicking inside
`UserInterface::draw` risks renderer-state leaks.

**Impact on tasks.md.** No change. T03's `--smoke` exit path is
the implicit safety net (already in V2).

---

### Q-ARCH-5 — `cargo insta` integration: deferred to cycle-1 sibling F

> *"Architect confirms no insta-integration work in this brief."*

**Resolution.** **Confirmed.** Reuse
[`tests/fixtures/visual_diff.rs::matches_screenshot`](../../crates/ui/tests/fixtures/visual_diff.rs#L74-L137)
unchanged. Cycle-1 sibling **F (`iced-test-bytes`)** brings
`cargo insta review` parity to all `iced_test::screenshot`-driven
baselines (this gallery's + the bootstrap's + cycle-2 E's).
Inlining insta here would fork the helper and force a de-fork
when F lands.

**Impact on tasks.md.** No T-item.

---

### Q-ARCH-6 — README discoverability: YES, one-line add

> *"Add a `cargo run -p ui --bin ui-gallery --features fixtures`
> row to `README.md`?"*

**Resolution.** **Confirmed.** One line under the existing
`cargo run --bin cockpit` row, cross-linking to
`spec/ui-gallery-bin/feature.md`. README is in-scope for V9 —
the developer extends V9's permitted-paths set to include
`README.md` (clarified in § tasks.md delta).

**Impact on tasks.md.** T21 already covers; V9 widening noted.

---

## Module layout

The feature.md proposed a flat `crates/ui/src/gallery.rs`. The
architect splits into a small `gallery/` module so the
24-cell route table doesn't bloat one file beyond ~300 LOC.
Three files, plus the bin entry point:

```text
crates/ui/
├── src/
│   ├── bin/
│   │   └── ui_gallery.rs                  [NEW, ≤ 60 LOC] entry point
│   ├── gallery/
│   │   ├── mod.rs                         [NEW, ~120 LOC] public surface
│   │   ├── cell.rs                        [NEW, ~80 LOC] GalleryCell struct + view()
│   │   └── routes.rs                      [NEW, ~250 LOC] GALLERY_CELLS const + EXPECTED_WIDGETS
│   ├── fixtures.rs                        [MODIFIED, +≤ 80 LOC] new helpers per H-GAL-4
│   └── lib.rs                             [MODIFIED, +2 lines] `pub mod gallery;`
├── tests/
│   ├── gallery_snapshots.rs               [NEW, ~150 LOC] 3× #[test] fn
│   └── visual-baselines/
│       ├── ui_gallery_dark_floor.png      [NEW, written by V6 first-run]
│       ├── ui_gallery_dark_typical.png    [NEW, written by V6 first-run]
│       └── ui_gallery_dark_operator.png   [NEW, written by V6 first-run]
└── Cargo.toml                              [MODIFIED, +5 lines] `[[bin]]` stanza
```

**Public-API surface — the gallery contract (≤ 5 items).** The
rest of the `ui` crate treats these as the stable gallery
boundary. Adding to / removing from this set requires an ADR.

| # | Item                                               | Module           | Stability |
|---|----------------------------------------------------|------------------|-----------|
| 1 | `pub struct GalleryCell`                           | `gallery::cell`  | stable    |
| 2 | `pub const GALLERY_CELLS: &[GalleryCell]`          | `gallery::routes`| stable    |
| 3 | `pub const EXPECTED_WIDGETS: &[&str]`              | `gallery::routes`| stable    |
| 4 | `pub const GALLERY_LOGICAL_HEIGHT: u32`            | `gallery`        | stable    |
| 5 | `pub fn view(cell: &GalleryCell) -> Element<'_, Message>` | `gallery::cell` | stable    |

Plus an internal `gallery::view_all(model: &Cockpit) -> Element<'_, Message>`
helper used by both the bin and the snapshot test — this one is
`pub(crate)` (not part of the contract above) so cycle-2 item E
can author its own composer without touching gallery internals.

**Per-file responsibilities.**

- **`bin/ui_gallery.rs`** — `clap::Parser` derive (`--smoke`
  bool), early-exit on `--smoke` after fixture-load + first
  `gallery::view_all(&cockpit)` call (constructs Element, drops
  it; no iced runtime entered). On the live path: builds the
  cockpit fixture (`fake_cockpit_v15a_pairs_steady_state()`
  exactly like `cockpit.rs:144`), boots
  `iced::application(...).run()` with a `scrollable(view_all(...))`
  wrapping so the operator can scroll the gallery in-window. ≤ 60
  LOC.
- **`gallery/mod.rs`** — re-exports `GalleryCell`,
  `GALLERY_CELLS`, `EXPECTED_WIDGETS`, `GALLERY_LOGICAL_HEIGHT`,
  `view_all`, `view` (cell-level). Module rustdoc cites the
  Q-GALLERY-SCOPE lock and the dev-note §3.3 origin. Hosts the
  `#[cfg(test)] mod tests` for V3 (`every_expected_widget_*`) +
  V4 (`every_widget_mod_*`).
- **`gallery/cell.rs`** — `pub struct GalleryCell { widget: &'static
  str, state: &'static str, render: fn(&Cockpit) -> Element<'static,
  Message>, seed: fn() -> Cockpit }`. `pub fn view(cell) -> Element`
  wraps the rendered widget in a Container with the cell-header
  strip (label = `format!("{} :: {}", cell.widget, cell.state)`,
  font `text::MICRO`, color `color::SUBTLE`, 1-px
  `color::BORDER_1` separator below). Each cell Container has
  `height(Length::Fixed(N))` per Q-ARCH-3 (no Length::Fill).
- **`gallery/routes.rs`** — `GALLERY_CELLS: &[GalleryCell]` with
  all 24 entries from feature.md's route table + the 12
  chrome-widget single-cells from feature.md M3. `EXPECTED_WIDGETS`
  is the explicit list of 22 widget-module names (note:
  `canvas_chart` is `pub(crate)` and intentionally excluded — see
  Q-ARCH-2).

---

## State seeding contract

The gallery imports `ui::fixtures::*` directly. Q-GALLERY-SCOPE
is **LOCKED** to fixtures-reuse — this section specifies the
interface, not the choice.

**Import line at top of `gallery/routes.rs`:**

```rust
use crate::fixtures as fx;
```

**Every `GalleryCell.seed` closure calls a `fx::fake_*(...)`
builder.** No local builders. No `fn fake_*` inside
`gallery/**`. The evaluator's drift-gate `grep -n 'fn fake_\|fn
synth' crates/ui/src/gallery/` MUST be empty (this is the
Q-GALLERY-SCOPE mechanical enforcement; tasks.md M_FINAL_EVAL
already cites this rule — see § tasks.md delta for the path
update from `gallery.rs` to `gallery/`).

**New `fixtures.rs` exports required** (per feature.md H-GAL-4
80-LOC budget — architect confirms the budget):

| # | New `pub fn`                                      | Site            | Est. LOC |
|---|---------------------------------------------------|-----------------|----------|
| 1 | `fake_volume_bins() -> Vec<VolumeBin>`            | cell 21 / 22    | ~25      |
| 2 | `fake_signal_view(n: i64) -> SignalView`          | cell 24         | ~20      |
| 3 | `fake_strategy_row_error_in_v1_set() -> Cockpit`  | cell 9          | ~15      |
| 4 | `fake_market_health_degraded() -> MarketHealthState` | cell 14      | ~10      |
| **Total** |                                             |                 | **~70**  |

Confirmed: under the 80-LOC budget. The developer extends
`fixtures.rs` with these four `pub fn` items (no signature
changes to existing builders). H-GAL-4 falsifier (T03 review,
≤ 100 LOC) is structurally easier to pass under this
breakdown than the analyst's worst-case estimate.

**Excluded helpers** (analyst pre-listed but architect's review
finds existing coverage): `fake_journal_rows(n)` already exists
(used by cockpit.rs:182), no new fixture for the
`journal_transaction_modal` chrome-cell needed.

---

## Snapshot test path

**Confirm.** Use the bootstrap's
[`tests/fixtures/visual_diff.rs::matches_screenshot`](../../crates/ui/tests/fixtures/visual_diff.rs#L74)
unchanged — the analyst's choice. The `iced_test::Snapshot`
byte-accessor gap (the helper's first-paragraph rationale) is
already documented; the gallery inherits the bootstrap's
workaround as-is.

**Test file owner.** `crates/ui/tests/gallery_snapshots.rs`
(NEW). Confirmed via the bootstrap's
[`tests/visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs)
pattern.

**Baseline directory.** `crates/ui/tests/visual-baselines/`
(EXISTING — three `charts_screen_dark_*.png` already committed
there per bootstrap V2 first-run). The three new gallery
baselines land as siblings:

```text
crates/ui/tests/visual-baselines/
├── charts_screen_dark_floor.png         [EXISTING, bootstrap]
├── charts_screen_dark_typical.png       [EXISTING, bootstrap]
├── charts_screen_dark_operator.png      [EXISTING, bootstrap]
├── ui_gallery_dark_floor.png            [NEW, gallery]
├── ui_gallery_dark_typical.png          [NEW, gallery]
└── ui_gallery_dark_operator.png         [NEW, gallery]
```

**Shape of `gallery_snapshots.rs`** (developer reference):

```rust
const SLOTS: &[(&str, u32, f32)] = &[
    ("floor",    1280, 1.0),
    ("typical",  1920, 1.0),
    ("operator", 3360, 2.0),
];

fn run_slot(slot_name: &str) {
    let (_, slot_w, scale) = SLOTS.iter().find(|(s, _, _)| *s == slot_name).copied().unwrap();
    let cockpit = ui::gallery::routes::seed_for_all_cells();  // pub(crate) fn — picks the v15a steady state
    let program = ui::gallery::program_from_cockpit(cockpit); // pub(crate) fn — Application wrapper
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(
        &program,
        &theme,
        (slot_w, ui::gallery::GALLERY_LOGICAL_HEIGHT),  // <-- intrinsic content height, not 720/1080/1890
        scale,
        Duration::ZERO,
    );

    let baseline = format!(
        "{}/tests/visual-baselines/ui_gallery_dark_{slot_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );
    matches_screenshot(&screenshot, &baseline, &format!("ui_gallery_dark_{slot_name}"))
        .unwrap_or_else(|err| panic!("gallery visual snapshot mismatch:\n{err}"));
}

#[test] fn ui_gallery_dark_floor()    { run_slot("floor"); }
#[test] fn ui_gallery_dark_typical()  { run_slot("typical"); }
#[test] fn ui_gallery_dark_operator() { run_slot("operator"); }
```

Note the **viewport tuple** uses `GALLERY_LOGICAL_HEIGHT`
(constant, ≈6400) for ALL three slots — only the **width**
varies per slot. This is the Q-ARCH-3 fix.

---

## Build & run

**Sub-agent / sandbox-safe (V-items).**

```bash
# V1 — bin builds
cargo build -p ui --bin ui-gallery --features fixtures

# V2 — smoke (no window, exits in < 5 s)
cargo run -p ui --bin ui-gallery --features fixtures -- --smoke

# V3 + V4 — exhaustiveness tests
cargo test -p ui --features fixtures gallery::tests

# V5 — tiny-skia dep tree
cargo build -p ui --bin ui-gallery --features fixtures -v 2>&1 | grep iced_tiny_skia

# V6 + V10 — snapshot determinism (two runs, byte-compare)
cargo test -p ui --features fixtures --test gallery_snapshots
shasum -a 256 crates/ui/tests/visual-baselines/ui_gallery_dark_*.png
cargo test -p ui --features fixtures --test gallery_snapshots
shasum -a 256 crates/ui/tests/visual-baselines/ui_gallery_dark_*.png   # must match

# V7 — workspace green
cargo test --workspace --features fixtures

# V8 — anchors PASS
bash scripts/verify_anchors.sh

# V9 — file-list gate (run by tester after dev pass)
git diff --name-only HEAD~..HEAD
```

**Operator-only (presenter deck, T22 — post-VERDICT → PASS).**

```bash
cargo run -p ui --bin ui-gallery --features fixtures
# (operator scrolls the in-window iced scrollable + screencaptures)
```

**`--smoke` semantics (Q-ARCH-1).** When `--smoke` is set:

1. `clap::Parser::parse()` consumes argv.
2. The bin builds a `Cockpit` via the same `fake_*` builders the
   live path uses.
3. The bin calls `gallery::view_all(&cockpit)` once (constructs
   Element, drops it).
4. Bin returns `ExitCode::SUCCESS` **without** calling
   `iced::application(...).run()`.

This catches a panicking fixture builder, a missing `pub mod`,
or a compile-time regression in any cell's `render` closure
**without requiring a display server**. The 5-second budget in
V2 is generous; expected wall-clock is ~200 ms.

---

## Risk register

Architect-resolved-at-design-time risks land in feature.md;
*open* risks the developer carries:

| # | Trigger | Fallback | Owner |
|---|---|---|---|
| R-DESIGN-1 | Operator-slot PNG measured >10 MB at T19 | Split gallery into TWO column halves (rows 1–12 and 13–24); produce six baselines (two per slot). Adds ~0.3d to M4. | architect-on-call (developer escalates) |
| R-DESIGN-2 | `iced_test::screenshot` at `(viewport_w, 6400)` runs out of memory on CI/dev hardware (op-slot 2× = 6720×12800×4 = ~330 MB RGBA, plus iced's internal buffers) | Drop the operator slot to 1.5× scale; document in feature.md changelog. M4 still passes at floor + typical + reduced-operator. | developer (during T17) |
| R-DESIGN-3 | A widget under `widgets/` adds a `#[cfg(feature = "live")]` gate that breaks the `--features fixtures` build of the gallery (falsifies H-GAL-3) | Per-cell `#[cfg]` mirror — wrap the offending `GalleryCell` entry in `#[cfg(feature = "live")]` and add the widget to `EXPECTED_WIDGETS` only when feature is on. Test V3 conditionally asserts. | developer (during T-M0-G) |
| R-DESIGN-4 | A cell's `seed` closure pulls a panic out of `crate::fixtures` (e.g. `unwrap_or_else(|_| unreachable!())` trips on a path we don't currently exercise) | `--smoke` (V2) catches at sub-agent time; developer fixes the underlying fixture before T17 lands. No catch_unwind in the gallery itself (Q-ARCH-4). | developer (during T03 / V2) |
| R-DESIGN-5 | `GALLERY_LOGICAL_HEIGHT = 6400` constant under-sized — cells overflow vertically and the bottom of the canvas clips | Snapshot test asserts the **visible label "chart_tooltip :: signal_tooltip"** (last cell) shows up in the PNG via the bootstrap's text-asserter pattern. Developer measures the actual cell heights on first paint and bumps the const if the assert trips. Estimate adjustment: +0.05d to T02b. | developer (during T02b) |

---

## tasks.md delta

Developer applies as first step of M1. Architect leaves
analyst's tasks.md intact in M0.

**M0 ticks (architect-owned).**

- **T-M0-A..F** — tick `[x]`, cite design.md § Q-ARCH-1..6.
- **T-M0-S** — tick `[x]` as **FALSIFIED via source-read** of
  `iced_test-0.14.0/src/emulator.rs:444-498`. NO orchestrator
  spike needed. Cite design.md § Q-ARCH-3.
- **T-M0-G** — keep open; orchestrator still runs the
  `grep -rn 'cfg(feature = "live")' crates/ui/src/widgets/`
  check. If non-empty → R-DESIGN-3.

**M1–M5 changes.**

| T-item | Change |
|---|---|
| T01    | unchanged |
| T02    | target files split: `gallery/{mod,cell,routes}.rs` (3 files); `view_all()` returns `column!`, NOT `scrollable(column!)`; add `pub const GALLERY_LOGICAL_HEIGHT: u32 = 6_400` |
| **T02b** | **NEW** — `gallery::cell::tests::every_cell_has_fixed_height()` asserts every cell's wrapped Container has `Length::Fixed(N)` (no `Length::Fill`). Acceptance: test green. **Estimate 0.05d** |
| T03    | bin path wraps `view_all` in `scrollable(...)` for operator window UX; `--smoke` builds Element + drops before iced runtime |
| T04–T14 | unchanged (matrix wiring) |
| T15    | unchanged |
| T16    | regex strawman → pure-stdlib `strip_prefix("pub mod ")` chain per Q-ARCH-2; inline comment that `canvas_chart` is `pub(crate)` and intentionally excluded |
| T17    | viewport passed to `iced_test::screenshot` is `(slot_w, GALLERY_LOGICAL_HEIGHT)`, NOT `(slot_w, slot_h)`; inline comment cites design.md § Q-ARCH-3 / H-GAL-2 |
| T18    | unchanged |
| T19    | threshold sharpened to 10 MB per R-DESIGN-1 |
| T20    | unchanged |
| T21    | acceptance widens V9's permitted-paths set to include `README.md` |
| T22–T25 | unchanged |
| M_FINAL_EVAL drift-gate | update grep target from `gallery.rs` (single file) to `gallery/` (directory) |

**Effort sum (developer-owned T01..T25):** 3.0 + 0.05 (T02b) =
**3.05 dev-days**. Operator-slot baseline-split contingency
(R-DESIGN-1) reserves another ~0.3d budget if it fires;
otherwise lands on-budget per dev-note §5.1 row C.

---

## Changelog

- **2026-05-15 (orchestrator, post-impl):** Developer-found deviation
  during V5+ verification. The `GalleryCell::render` fn-pointer
  signature `fn(&Cockpit) -> Element<'static, Message>` (per
  Q-ARCH / Module layout) is structurally incompatible with iced
  widgets that return borrowed `Element<'_, Message>` (most do,
  including `positions::view`, `pnl::view`, `strategies::view`).
  Forcing `'static` cascades into 22 lifetime errors + 8 E0515
  errors in the implementation. Fix: `render` becomes `fn(&Cockpit)
  -> Element<'_, Message>` (lifetime-elided HRTB-equivalent for
  fn pointers); `cell::view` leaks the seeded cockpit to `'static`
  via `Box::leak` (test-only binary, bounded leaks per render).
  The Q-ARCH-3 / H-GAL-2 design remains intact — only the render
  signature changes. **GalleryCell.seed retained** for now (used by
  `cell::view` to build the cockpit before leak); future refactor
  can remove it once per-cell state-injection patterns stabilize.
- **2026-05-15 (orchestrator, post-impl):** V5+ BLOCKED. Tiny-skia
  panics at render time on `GALLERY_CELLS[7]` (`strategies ::
  ready_v1`) with "Build quad rectangle" at
  `iced_tiny_skia-0.14.0/engine.rs:686`. iced 0.14's
  `widget::table::Table` (used by `strategies::view`) produces a
  degenerate-bounds quad when rendered inside a fixed-height
  `cell::view` Container. Bumping `CELL_HEIGHT_PX` 260 → 500 does
  not resolve. Diagnostic kept at
  [`crates/ui/tests/gallery_bisect.rs`](../../crates/ui/tests/gallery_bisect.rs)
  (`#[ignore]`d). V5+ snapshot tests at
  [`crates/ui/tests/gallery_snapshots.rs`](../../crates/ui/tests/gallery_snapshots.rs)
  are `#[ignore]`d with cross-refs. Suggested follow-up feature:
  `ui-gallery-table-cell` — either replace strategies with a
  non-table render in the gallery, or special-case the strategies
  cell with a different wrapper (no `Container` height constraint,
  or explicit `Length::Shrink`).
- **2026-05-15 (architect):** Initial Design addendum. Six Q-ARCH-N
  resolutions documented. **H-GAL-2 FALSIFIED** via source-read
  of `iced_test-0.14.0/src/emulator.rs:444-498` (no spike run; the
  emulator passes `viewport` as the layout limit AND multiplies it
  by `scale_factor` to produce the PNG dimensions — scrollable
  content beyond the viewport is clipped). Design switches:
  snapshot path uses `column!`-no-scrollable + a viewport whose
  height equals `GALLERY_LOGICAL_HEIGHT = 6400`. Bin keeps
  `scrollable` for operator window UX. Module layout split from
  flat `gallery.rs` to `gallery/{mod,cell,routes}.rs` (three
  files, ~450 LOC total). Five-item public-API contract pinned.
  `fixtures.rs` adds 4 helpers (~70 LOC) — under the 80-LOC
  H-GAL-4 budget. Risk register seeds five `R-DESIGN-*` items
  the developer carries; R-DESIGN-1 (operator-slot >10 MB) is
  the most likely to fire at T19. tasks.md delta: T02 split
  across three files, NEW T02b for the height-const + unit test,
  T17 viewport-tuple change, T16 pure-stdlib parser, T21
  README/V9 widening. Effort 2.85d net (under the 3.0d budget).
