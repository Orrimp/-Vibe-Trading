---
slug: ui-gallery-bin
version: 0.1.0
status: in-progress
owner: analyst
predecessor: ui-test-harness-bootstrap v0.1.0
updated: 2026-05-15
---

# Widget gallery binary (`ui-gallery-bin`) — v0.1

> **Cycle-1, item C** of the rollout locked in
> [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md §5.2`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#52-recommended-sequencing).
> Sibling cycle-1 features (A — `ui-contrast-asserter`, B —
> `ui-update-proptest`, D — `ui-test-harness-canvas-state-seeding`,
> F — `iced-test-bytes`, G — locale/font determinism fixtures) are
> independent of this brief. Cycle-2 item **E** (`ui-test-harness-viewport-matrix`)
> is the downstream blocked-by-this feature: per [§5.3 Keep/Drop/Replace](../dev-notes/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan),
> E without the gallery means 50+ per-widget baselines with no shared
> review surface, so this feature lands first.

## Why

The operator's founding pain — "an agent cannot see pixels" — is
quantified in [`dev-note §1.2`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#12-the-agent-cant-see-pixels-sandbox-is-a-strength-not-just-a-constraint):
the
[AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)
regime forbids sub-agents from launching the cockpit, and the
ui-test-harness-bootstrap v0.1 ship proved that snapshot tests CAN
close the loop without a screencapture. What's still missing is a
**single artifact that enumerates the cockpit's widget surface** so
that:

1. A new sub-agent can name what exists ("show me the gallery") before
   it tries to write a test against it.
2. The presenter / operator can review a single scroll instead of
   N×M individual screenshots when the harness expands across the full
   widget matrix (cycle-2 item E).
3. The future VLM-judge (cycle-3 item K, deferred per
   [`Q-VLM lock`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#6-open-questions-for-the-operator))
   has a single page to evaluate per claim — amortizing cost.

The analyst's framing in [`dev-note §3.3 (b)`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin)
is verbatim: this is the **highest-ROI agent-friendly artifact** in
the deep-dive. The Rust GUI world has no equivalent today
([`boringcactus 2025 Rust GUI survey`](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html));
egui's demo app is the closest analogue
([`egui repo`](https://github.com/emilk/egui)). One screenshot of this
binary captures 30+ widget × state cells; the cockpit-without-gallery
would need ~30 individual launches to give the agent the same coverage.

**Dependent features** (consumers of this brief's outputs):

- **Cycle-2 item E — `ui-test-harness-viewport-matrix`** — full-widget
  viewport matrix; per
  [`dev-note §5.3`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan)
  E reuses the gallery's `[widget × state]` route table to seed its
  per-cell baselines. E's analyst spawn waits on this brief shipping.
- **`tester.md` agent-contract change** — the visual-fail HTML
  artifact stanza in
  [`dev-note §4.1`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose)
  templates the gallery-snapshot diff PNG as the embedded failure
  image. The stanza authoring is a separate workflow update; this
  brief's V-items are the precondition.
- **`ui-designer.md` agent-contract change** —
  [`dev-note §4.2`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#42-ui-designermd--render-preview-before-handoff)
  mandates the ui-designer cite a gallery snapshot section before
  HANDOFF → tester. That stanza is meaningless until the gallery
  exists.

This brief authors the bin; the agent-contract stanzas land as
follow-up spec-updates once the bin is approved.

## Scope locked

Operator decisions and dev-note pins that this brief inherits without
reopening:

- **Q-GALLERY-SCOPE — LOCKED 2026-05-15 → reuse
  `crates/ui/src/fixtures.rs` (live state-builders, shared with
  cockpit).** Per [`dev-note §6 stanza 4`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#6-open-questions-for-the-operator):
  the gallery does NOT fork the state-builders. It imports
  `ui::fixtures::*` directly so a `fake_cockpit_v1_steady_state()`
  update is visible to the cockpit, the gallery, and the snapshot
  baselines simultaneously. Drift is impossible by construction —
  this is the dev-note §3.3 (d) risk mitigation made structural.
- **Single mega-canvas presentation** — per [`dev-note §3.3 (a)`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin)
  the gallery is **one scrolling window** rendering every widget ×
  every state on one page (not an index-page + per-cell routes).
  Rationale: the screen-recorder / `screencapture` use-case wants a
  single canvas the operator scrolls through; per-cell routes
  multiply the agent's discovery cost without compressing the
  review surface.
- **Three-viewport matrix** — same slot table as
  [`ui-test-harness-bootstrap v0.1`](../ui-test-harness-bootstrap/feature.md#r2--viewport-matrix-dev-note-3-layer-3):
  `floor` (1280×720, 1.0×), `typical` (1920×1080, 1.0×), `operator`
  (3360×1890, 2.0×). The gallery's snapshot test reuses the bootstrap's
  `const SLOTS` shape.
- **`tiny-skia` rendering backend** — pinned by the workspace
  [`iced = "=0.14.0", features = ["tiny-skia", ...]`](../../Cargo.toml)
  declaration. Tiny-skia CPU determinism was empirically confirmed
  by H1 in the bootstrap ship (operator decision 2026-05-12).
- **Cycle-2 item E reuses the route table** — the
  `[widget × state]` cell registry this brief authors IS the seed
  for E's per-widget viewport matrix. E does not re-author it.
- **Exhaustiveness check** — per
  [`dev-note §3.3 (d) risk mitigation`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin)
  and [`§2.15 Widget-tree coverage`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#215-feature-completeness-scaffolding--the-reachability-question)
  the MVP includes a Rust unit test that fails when a module under
  `crates/ui/src/widgets/` is added without a corresponding gallery
  route. Drift fails loud.
- **Effort budget — 3 dev-days** per
  [`dev-note §5.1 row C`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#51-idea-table)
  (effort M, ROI **High**, risk Low). The task list below sums to
  3 days at standard developer cadence.

## Out of scope

People will ask for these; v0.1 does NOT ship them. They get their own
features later in the rollout:

- **No live cockpit state** — the gallery is fixtures-only. Wiring a
  live broadcast bus is the `cockpit_live` binary's job and is
  explicitly cycle-3 / cycle-4 territory.
- **No interaction recording / replay** —
  [`ui-session-journal`](../backlog.md#process--tooling)
  (`dev-note §3.6`, deferred to cycle 4) owns this.
- **No MCP exposure** — [`ui-inspect-mcp`](../backlog.md#process--tooling)
  (`dev-note §3.1`, deferred to cycle 4 per Q-MCP lock) is a
  separate feature with its own security review. The gallery is
  a passive renderer, not a queryable inspector.
- **No VLM-judge integration** — Layer 6 (`ui-vlm-judge`,
  `dev-note §3.2`) lands in cycle 3 / shadow-mode-only per
  Q-VLM lock. The gallery's snapshots are an input to that work,
  not a co-deliverable.
- **No interaction firing** — `Message::*` injection is out of scope.
  Buttons in the rendered cells are inert (no `on_press` wiring
  beyond what the production widget already exposes). The
  `ui-update-proptest` cycle-1 sibling feature owns Message coverage.
- **No `cargo insta review` parity for the gallery snapshot baselines**
  — the bootstrap's
  [`cargo insta review integration gap`](../ui-test-harness-bootstrap/feature.md#cargo-insta-review-integration-gap)
  applies here too. `iced-test-bytes` (cycle-1 sibling item F) is
  the cross-cutting fix.
- **No widget mutation (theme-swap, font-swap, locale-swap)** — the
  gallery renders the current cockpit defaults. Locale / font
  determinism is cycle-1 sibling G's territory.
- **No new widgets** — this is test/dev infrastructure only. The
  existing 22 modules under `crates/ui/src/widgets/` (per
  [`widgets/mod.rs:12-34`](../../crates/ui/src/widgets/mod.rs))
  are the surface; nothing new ships.

## Design

### Bin target

Add a new `[[bin]]` to [`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml):

```toml
[[bin]]
name = "ui-gallery"
path = "src/bin/ui_gallery.rs"
required-features = ["fixtures"]
```

**Why `required-features = ["fixtures"]` and not the default build:**
the gallery imports `ui::fixtures::*` (locked Q-GALLERY-SCOPE), and
the `fixtures` module is feature-gated at
[`crates/ui/src/lib.rs:40`](../../crates/ui/src/lib.rs). Mirrors the
existing `cockpit` bin's contract
([`Cargo.toml:17-20`](../../crates/ui/Cargo.toml)) — alternative was
default-build with auto-fixtures, rejected because it pollutes every
`cargo build -p ui`. Agent-friendliness gap is closed by V1's
canonical invocation form and a `README` row added in T21.

The bin compiles without `--features live` (no agent / audit / tokio
dep) — identical gate to `cockpit`.

### Route table

The single source of truth for the gallery's content is a const slice
at `crates/ui/src/gallery.rs`:

```rust
pub struct GalleryCell {
    pub widget: &'static str,   // matches crates/ui/src/widgets/<name>.rs
    pub state:  &'static str,   // identifies the fixture variant
    pub render: fn(&Cockpit) -> Element<'static, Message>,
    pub seed:   fn() -> Cockpit, // builds the fixture Cockpit for this cell
}

pub const GALLERY_CELLS: &[GalleryCell] = &[ ... ];
```

The exhaustiveness test (R3 below) cross-references the `widget`
column against the `mod` declarations in
[`crates/ui/src/widgets/mod.rs`](../../crates/ui/src/widgets/mod.rs).

**Concrete matrix** (10 widgets × 2–4 states × 3 viewports). Each cell
is one entry in `GALLERY_CELLS`. The state-name column maps to the
`crates/ui/src/fixtures.rs` helper that builds the seed cockpit; where
no direct helper exists, the developer composes via `fake_cockpit_ready()`
+ mutation (the bootstrap's
[`charts_screen_with_hovered_marker`](../ui-test-harness-bootstrap/feature.md#fixture-authoring-strategy)
is the canonical pattern). All cells render at all three viewports
(slot rows omitted from the table for brevity — see § Viewport rendering
below).

| # | Widget (`crates/ui/src/widgets/`) | State | Fixture seed (`ui::fixtures::*`) |
|---|---|---|---|
| 1 | `positions.rs` | loading | `fake_cockpit_ready()` with `positions = PanelState::Loading` |
| 2 | `positions.rs` | empty | `fake_cockpit_ready()` with `positions = PanelState::Ready(vec![])` |
| 3 | `positions.rs` | ready_v1_three | `fake_cockpit_v1_steady_state()` (fills `positions` via `fake_v1_three_symbol_portfolio`) |
| 4 | `positions.rs` | ready_negative_pnl | composed from `fake_positions()` with `fake_pnl_negative()` injected |
| 5 | `pnl.rs` | positive | `fake_cockpit_ready()` with `pnl = PanelState::Ready(fake_pnl_positive())` |
| 6 | `pnl.rs` | negative | `fake_cockpit_ready()` with `pnl = PanelState::Ready(fake_pnl_negative())` |
| 7 | `strategies.rs` | loading | `fake_cockpit_ready()` with `strategies = PanelState::Loading` |
| 8 | `strategies.rs` | ready_v1_with_events | `fake_cockpit_v1_steady_state()` |
| 9 | `strategies.rs` | with_error_row | `fake_cockpit_with_strategies()` patched so one row carries `StrategyStatus::Error(..)` |
| 10 | `strategies.rs` | with_one_veto | `fake_cockpit_with_one_veto()` |
| 11 | `chart.rs` | charts_screen_hovered | `charts_screen_cockpit()` (from `test_support`, calls `fake_cockpit_ready_with_three_fills`) + tooltip seed |
| 12 | `chart.rs` | charts_screen_empty | `fake_cockpit_ready()` with no fills / empty bars |
| 13 | `latency.rs` | healthy | `fake_cockpit_ready()` + `fake_market_health` healthy snapshot |
| 14 | `latency.rs` | degraded | `fake_cockpit_ready()` with degraded venue rows |
| 15 | `human_control.rs` | auto-mode | `fake_cockpit_ready()` with `AgentMode::Auto` |
| 16 | `human_control.rs` | paused | `fake_cockpit_ready()` with `AgentMode::Paused` |
| 17 | `human_control.rs` | killed | `fake_cockpit_ready()` with `AgentMode::Killed` (or kill-switch active) |
| 18 | `agent_feed.rs` | empty | `fake_cockpit_ready()` with no fills |
| 19 | `agent_feed.rs` | with_three_fills | `fake_cockpit_ready_with_three_fills()` |
| 20 | `num.rs` | format showcase | static `Column<Text>` rendering `fmt_usdt`, `fmt_price`, `fmt_qty`, `fmt_pct`, `format_pct_sentiment(pos/neg)`, `format_sharpe` — no Cockpit needed |
| 21 | `volume_histogram.rs` | mixed bins | static `view(fake_volume_bins(), ThemeMode::Dark)` — developer authors `fake_volume_bins()` in `fixtures.rs` as a sibling to `synthetic_candles` |
| 22 | `volume_histogram.rs` | empty | `view(vec![], ThemeMode::Dark)` |
| 23 | `chart_tooltip.rs` | fill_tooltip | `chart::tooltip_view_for_fill(&fake_fill_view(0), Some("momentum-h1".into()))` |
| 24 | `chart_tooltip.rs` | signal_tooltip | `chart::tooltip_view_for_signal(&fake_signal_view(0))` (signal fixture stub authored alongside) |

**Cell count:** 24 (widget, state) rows × 3 viewports = **72 logical
cells** in the matrix. Snapshot baselines = 3 PNGs (one per viewport,
each PNG embeds all 24 cells in one scrollable canvas). See § Viewport
rendering below for why per-viewport baselines aren't 24×3 separate
PNGs.

**Note on `num.rs`:** the user-supplied widget list includes `num` even
though [`num.rs`](../../crates/ui/src/widgets/num.rs) is a
formatting-helpers module (no `pub fn view`). The gallery treats it
as a showcase cell rendering a `Column<Text>` of formatted outputs —
this is the §2.15 "every widget module is reachable" contract; it
does NOT make `num` a renderable widget.

### Navigation

**Single scrollable mega-canvas** per [`dev-note §3.3 (a)`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin)
verbatim — "every widget × every state × every viewport on one page."
Implementation: one `iced::widget::scrollable(column(...))` wrapping
all `GALLERY_CELLS` rendered in order. Each cell is wrapped in a
`Container` with:

- A header strip showing `widget :: state` (font: `text::MICRO`,
  color: `color::SUBTLE`; matches the bootstrap's existing label
  conventions).
- The rendered widget at its production sizing.
- A 1-px `border = color::BORDER_1` separator below.

No index page; no per-cell route. The
[`screencapture` MVP workflow](../../.claude/skills/rust-validate/SKILL.md)
(macOS-locked per [`D3`](../ui-test-harness-bootstrap/feature.md#scope-locked))
is: run the bin, scroll to the cell, capture. This is the dev-note's
intent — one screen-recorder pass covers the whole gallery without
navigation friction.

The alternative (index page + named anchors with deep-link
`--cell=positions/ready`) was considered and rejected for v0.1
because the screen-recorder use case is the only operator-facing
flow today; deep-links are useful for the future MCP shim
(`ui-inspect-mcp`, deferred to cycle 4).

### Viewport rendering

The bin renders at **one logical viewport at a time** (whatever the
operator drags the window to). The snapshot test
(`crates/ui/tests/gallery_snapshots.rs`) drives `iced_test::screenshot`
at the three slots from the bootstrap's `const SLOTS` table and
produces three PNG baselines:

```
crates/ui/tests/visual-baselines/ui_gallery_dark_floor.png       (1280 × 720, 1.0×)
crates/ui/tests/visual-baselines/ui_gallery_dark_typical.png     (1920 × 1080, 1.0×)
crates/ui/tests/visual-baselines/ui_gallery_dark_operator.png    (3360 × 1890, 2.0× → 6720×3780 physical)
```

The `operator` slot baseline will be **tall**: 24 cells × ~220px
average × 2× scale ≈ 10,500 px tall × 6720 px wide. At 4 bytes/pixel
RGBA that's ~280 MB in memory and ~3–6 MB on disk after PNG zlib —
acceptable but the largest committed baseline in the repo. See **Risks**
below.

The viewport count is **3 baselines, not 24×3 = 72**, because the
single-canvas presentation means all 24 cells are inside the one
scrollable, and the iced_test screenshot captures the entire
scrollable's content rectangle (not just the viewport-visible portion)
— `iced_test::screenshot(...)` renders at the requested logical
`(viewport_w, viewport_h)` then captures the full rendered content
(scrollable expands to its content's intrinsic size in `iced_test`'s
no-runtime path). The bootstrap's
[`H2 caveat`](../ui-test-harness-bootstrap/feature.md#hypothesis-register)
applies: the `Duration::ZERO` setting may need a bump to
`Duration::from_millis(50)` if the gallery's child widgets pull
deferred-task initialization. Per H-GAL-1 below this is the load-bearing
test for the architect's pass.

### Screenshot capture protocol

An agent does NOT run the bin. The agent runs the snapshot test:

```bash
cargo test -p ui --features fixtures --test gallery_snapshots
```

The first run writes the three PNG baselines; subsequent runs
byte-compare. The orchestrator (per
[`AGENT.md ## Capability map`](../../AGENT.md#capability-map)) is the
only agent that can `cargo run --bin ui-gallery` and capture a window
via macOS `screencapture` — this is the bootstrap's existing
operator-territory contract and is unchanged.

The bin's role is **operator + presenter**, not **sub-agent**: when
the presenter assembles a deck, it embeds the
`ui_gallery_dark_operator.png` baseline as the "current widget
surface" exhibit. The
[`capture-screenshot` skill](../../.claude/skills/) is the
orchestrator's tool here — same macOS `screencapture` flow the
bootstrap already uses.

### `fixtures.rs` reuse contract (Q-GALLERY-SCOPE)

The gallery imports `ui::fixtures::*` directly. NO fork. NO
duplication. NO test-only fixture file. Concretely:

- `crates/ui/src/gallery.rs` declares
  `use crate::fixtures as fx;` at the top.
- Every `GalleryCell.seed` closure calls `fx::fake_*(...)` builders.
- If a gallery cell needs a state not in `fixtures.rs`, the developer
  **extends `fixtures.rs`** rather than authoring a local builder.
  This guarantees the cockpit (which also imports `fixtures` under
  `--features fixtures`) and the gallery see the same builders.

State-drift between gallery and cockpit is **mechanically
impossible** under this contract (Q-GALLERY-SCOPE's locked rationale,
[`dev-note §6 stanza 4`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#6-open-questions-for-the-operator)).

### Exhaustiveness test

Per
[`dev-note §2.15 widget-tree coverage`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#215-feature-completeness-scaffolding--the-reachability-question)
+ §3.3 (d) risk mitigation. Mechanism:

```rust
// crates/ui/src/gallery.rs

/// The canonical list of widget-module names the gallery is expected
/// to cover. Sync this with crates/ui/src/widgets/mod.rs ANY time a
/// new pub mod lands there.
pub const EXPECTED_WIDGETS: &[&str] = &[
    "agent_feed", "chart", "chart_tooltip", "human_control",
    "latency", "num", "pnl", "positions", "strategies",
    "volume_histogram",
    // Plus the chrome widgets that ship as part of the cockpit shell:
    "chart_legend", "drawdown_band", "equity_curve", "focus_ring",
    "frame", "journal_transaction_modal", "kill", "kpi_strip",
    "override_risk_veto", "sidebar_nav", "sparkline", "status_bar",
];

#[cfg(test)]
mod tests {
    use super::{EXPECTED_WIDGETS, GALLERY_CELLS};

    #[test]
    fn every_expected_widget_has_at_least_one_gallery_cell() {
        let covered: std::collections::HashSet<&str> =
            GALLERY_CELLS.iter().map(|c| c.widget).collect();
        let missing: Vec<&&str> = EXPECTED_WIDGETS
            .iter()
            .filter(|w| !covered.contains(*w))
            .collect();
        assert!(
            missing.is_empty(),
            "gallery is missing cells for widgets: {missing:?}",
        );
    }

    #[test]
    fn every_widget_mod_is_listed_in_expected_widgets() {
        // grep-based cross-check against widgets/mod.rs at build time.
        // Concrete shape: a build.rs (or a const fn macro) that emits
        // ALL_WIDGETS_FROM_MOD_RS for the test to compare against
        // EXPECTED_WIDGETS. Architect picks build.rs vs const-fn
        // proc-macro in Design pass.
        let mod_rs = include_str!("widgets/mod.rs");
        // ... parse `pub mod NAME;` lines, assert set equality.
    }
}
```

The test is named so the failure message tells the developer **what
to add**: the missing widget set. This is the §3.3 (d) drift
mitigation made structural.

**v0.1 scope on covered widgets:** the 10 widgets the user-instruction
list calls out (`positions`, `pnl`, `strategies`, `chart`, `latency`,
`human_control`, `agent_feed`, `num`, `volume_histogram`,
`chart_tooltip`) get state-matrix cells in `GALLERY_CELLS`. The
other 12 chrome-widget modules (`chart_legend`, `drawdown_band`,
`equity_curve`, `focus_ring`, `frame`, `journal_transaction_modal`,
`kill`, `kpi_strip`, `override_risk_veto`, `sidebar_nav`,
`sparkline`, `status_bar`) get **single-cell** entries — one
representative state per widget — so the exhaustiveness test passes.
Architect's Design pass may demote some of these to "covered via
parent screen" (e.g. `sidebar_nav` is part of every cockpit shell; a
dedicated cell may be redundant). Architect picks.

## Acceptance / verification (V-items)

Per [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries):
every V-item below is checkable from a sub-agent's sandbox. No
V-item requires `screencapture`, `osascript`, a live display server,
or operator eyeballing.

- **V1 — Bin compiles under the default+`fixtures` profile.**
  `cargo build -p ui --bin ui-gallery --features fixtures` exits 0.
  Sandbox-safe: pure `cargo build`. Closes the "agent reach" design
  decision above.
- **V2 — Bin runs without panicking (smoke).** `cargo run -p ui
  --bin ui-gallery --features fixtures -- --smoke` exits 0 within
  5 seconds. The `--smoke` flag (developer-authored CLI surface) is a
  fixtures-load + first-frame-render gate that exits before the iced
  event loop starts. Sandbox-safe: pure `cargo run`. NO window
  appears in `--smoke` mode (no `display required` failure).
  Without `--smoke`, the orchestrator runs the bin manually and
  captures the screen for the presenter deck (out of band of
  sub-agent V-items).
- **V3 — Exhaustiveness test green.** `cargo test -p ui --features
  fixtures gallery::tests::every_expected_widget_has_at_least_one_gallery_cell`
  exits 0. Adding a new widget to `widgets/mod.rs` WITHOUT a
  corresponding `GalleryCell` row fails this test loudly.
- **V4 — Mod-rs cross-check test green.** `cargo test -p ui --features
  fixtures gallery::tests::every_widget_mod_is_listed_in_expected_widgets`
  exits 0. Adding a new `pub mod` to `widgets/mod.rs` WITHOUT
  updating `EXPECTED_WIDGETS` fails this test loudly. This is the
  §2.15 mechanical-coverage contract.
- **V5 — Bin compiles with `tiny-skia` (default backend).**
  `cargo build -p ui --bin ui-gallery --features fixtures` resolves
  the iced features set declared at
  [`Cargo.toml:69`](../../Cargo.toml)
  (`tiny-skia`, `thread-pool`, `advanced`, `canvas`) — no GPU/Vulkan
  resolver path. The tester pastes the `cargo build -v` line
  citing `iced_tiny_skia` in the dep tree. Sandbox-safe.
- **V6 — `crates/ui/tests/gallery_snapshots.rs` exists and produces
  three slot-named baselines.** `cargo test -p ui --features fixtures
  --test gallery_snapshots` produces three PNGs at
  `crates/ui/tests/visual-baselines/ui_gallery_dark_{floor,typical,operator}.png`
  on first run AND second-run `git status` shows zero modifications.
  Two-run determinism gate identical to the bootstrap's V2.
- **V7 — Non-regression: 818+ tests stay green.**
  `cargo test --workspace --features fixtures` produces zero
  failures and the prior pass count + the net-new tests this
  feature adds (V3, V4, V6 sub-tests). Sandbox-safe.
- **V8 — `bash scripts/verify_anchors.sh` PASS 11/11.** The
  feature touches zero non-UI crates; the 11 backtest-report SHA
  anchors stay byte-identical. Sandbox-safe: pure shell.
- **V9 — Zero changes outside scope.** `git diff --name-only
  HEAD~..HEAD` post-developer-pass shows changes only under
  `crates/ui/src/bin/`, `crates/ui/src/gallery.rs`,
  `crates/ui/src/lib.rs` (mod export), `crates/ui/tests/`,
  `crates/ui/Cargo.toml`, `Cargo.lock`, and `spec/`. Sandbox-safe:
  pure git.
- **V10 — Snapshot determinism contract holds.** A second consecutive
  `cargo test -p ui --features fixtures --test gallery_snapshots`
  run produces byte-identical PNGs (H-GAL-1 falsifier — see
  Hypothesis register). Sandbox-safe: pure `cargo test` + SHA check.

## Dependencies

All four required crates are already pinned in the workspace; no
version bumps and no new external crates.

- `iced = "=0.14.0"` ([`Cargo.toml:69`](../../Cargo.toml)) — runtime.
- `ui::fixtures` ([`crates/ui/src/fixtures.rs`](../../crates/ui/src/fixtures.rs))
  under `--features fixtures` — Q-GALLERY-SCOPE state-builders.
- `iced_test = "=0.14.0"` ([`crates/ui/Cargo.toml:108`](../../crates/ui/Cargo.toml))
  + `image-compare = "=0.4"` (`:109`) — dev-deps from the bootstrap;
  the gallery snapshot test reuses the bootstrap's
  [`tests/fixtures/visual_diff.rs`](../ui-test-harness-bootstrap/feature.md#fixture-authoring-strategy)
  helper unchanged.
- `clap` ([`Cargo.toml:65`](../../crates/ui/Cargo.toml)) — for the
  `--smoke` flag (same pattern as `viewer`).

## Risks

- **Drift between gallery and cockpit** — MITIGATED structurally by
  the Q-GALLERY-SCOPE lock (both import the same `fixtures.rs`).
- **Baseline-bloat / review usability** — 3 baselines × ≤6 MB ≤ 18
  MB. Operator review flow is the same Preview.app triple-open the
  bootstrap uses; `cargo insta review` parity returns via cycle-1
  sibling item F
  ([`iced-test-bytes`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#216-ci-ergonomics--solving-agent-proposes-human-approves)).
- **Operator-slot baseline disk size** — 24 cells × ~220 px @ 2×
  scale × 6720 px wide ≈ 3-6 MB on disk. If T19 measures > 10 MB,
  architect splits the gallery into two scrollables (six baselines
  instead of three).
- **H2 caveat carries over** — per
  [`bootstrap H2`](../ui-test-harness-bootstrap/feature.md#hypothesis-register):
  inline `chart` cell may not render hovered tooltip (canvas state
  is internal). Mitigation: the `chart_tooltip` cells render the
  `chart_tooltip::view(...)` Element standalone — not coupled to
  canvas state.
- **`fixtures` feature gate** — agents see `--bin ui-gallery` but
  must remember `--features fixtures`. Same shape as `cockpit`; V1
  canonicalizes the invocation; T21 cross-references from README.
- **Agent-screenshot reliability** — sub-agent V-items are pure
  `cargo test` (no `screencapture` reachable); only the
  orchestrator's presenter-deck capture invokes `screencapture`,
  operator-supervised.

## Out-files

Files this feature creates / touches:

**New:**

- `crates/ui/src/bin/ui_gallery.rs` — the bin entry point. Wraps
  the `gallery` module's `Program` factory + the iced runtime call.
- `crates/ui/src/gallery.rs` — module hosting `GalleryCell`,
  `GALLERY_CELLS`, `EXPECTED_WIDGETS`, the `view(...)` composer, and
  the exhaustiveness unit tests.
- `crates/ui/tests/gallery_snapshots.rs` — three slot-named
  `#[test] fn` driving `iced_test::screenshot` against the gallery
  program at the bootstrap's three viewports.
- `crates/ui/tests/visual-baselines/ui_gallery_dark_floor.png`
  (1280 × 720, written by V6 first-run).
- `crates/ui/tests/visual-baselines/ui_gallery_dark_typical.png`
  (1920 × 1080, written by V6 first-run).
- `crates/ui/tests/visual-baselines/ui_gallery_dark_operator.png`
  (3360 × 1890 logical → 6720 × 3780 physical, written by V6 first-run).

**Modified:**

- `crates/ui/Cargo.toml` — adds the `[[bin]] name = "ui-gallery"`
  stanza under `required-features = ["fixtures"]`.
- `crates/ui/src/lib.rs` — adds `pub mod gallery;` (gated under
  `#[cfg(any(feature = "fixtures", test))]` to match the
  `fixtures` module's gate).
- `crates/ui/src/fixtures.rs` — adds the small fixture builders the
  matrix needs that don't exist yet (per Design § route table: a
  `fake_volume_bins()` helper, a `fake_signal_view(n)` helper, and
  possibly a `fake_strategy_row_error_in_v1_set()` helper for cell
  9). These are pure additions; no existing helper signature changes.

**No changes outside `crates/ui/`** and the standard
`Cargo.lock` + `spec/` updates. V9 enforces.

## Hypothesis register

Per [`AGENT.md ## Capability boundaries — Architect / hypothesis-only`](../../AGENT.md#architect--hypothesis-only):
the analyst seeds the register with hypotheses identifiable at brief
time; the architect's Design pass appends more, and the orchestrator
runs the falsifiers.

- **H-GAL-1 — Tiny-skia determinism holds across two consecutive
  `cargo test -p ui --features fixtures --test gallery_snapshots`
  runs.**
  - *Statement:* Same machine, same fixture, same theme, same
    viewport, same scale_factor → byte-identical PNG output. This is
    the bootstrap's H1 hypothesis extended to the gallery's wider
    surface; the bootstrap proved H1 for one screen (charts), this
    hypothesis tests it for ~24 widget cells in one canvas.
  - *Falsifier:* V10. Two consecutive runs; SHA-compare the three
    baselines.
  - *Status:* unresolved — falsified by V10.

- **H-GAL-2 — `iced_test::screenshot` correctly captures
  scrollable-content beyond the viewport rectangle.**
  - *Statement:* The single-canvas presentation assumes
    `iced_test::screenshot(..., viewport=(1280, 720), ...)`
    renders the FULL gallery height (~10,500 px on operator) when
    the scrollable's content exceeds the viewport. If it instead
    clips to the visible 1280×720 rectangle, the design above
    breaks: floor and typical baselines would only show the
    top-of-scroll cells, not all 24.
  - *Falsifier:* Architect's M0 pass runs a trivial spike — a
    1280×720 viewport rendering a `column!` of 24 200-px-tall
    blocks. If the resulting PNG height is 720 px the hypothesis is
    falsified and the design switches to one-PNG-per-cell (3 × 24 =
    72 baselines) OR rendering the gallery with `column!` (no
    scrollable, intrinsic height) and accepting the operator-slot
    PNG height bloat.
  - *Status:* unresolved — **load-bearing for the architect's
    Design pass** (resolve before T01 / bin target authoring).

- **H-GAL-3 — Every widget's `pub fn view` is callable without
  `#[cfg(feature = "live")]` reach.**
  - *Statement:* All 22 modules under
    [`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) compile
    under default + `fixtures`; none require `live` (which would
    pull `agent` / `audit` / `tokio` and break V1).
  - *Falsifier:* T-M0-G — `grep -rn 'cfg(feature = "live")'
    crates/ui/src/widgets/`. Expected empty.
  - *Status:* unresolved — orchestrator-runnable; expected PASS
    based on the existing `cockpit` bin's fixtures-only gate.

- **H-GAL-4 — All 24 matrix cells are buildable in ≤ 80 LOC of
  `fixtures.rs` additions.**
  - *Statement:* Each (widget, state) row either reuses an existing
    `fake_*` helper or extends `fixtures.rs` with a small helper
    (cells 9 / 21 / 24 are the known new-helper sites — strategy
    error-row, volume bins, signal tooltip).
  - *Falsifier:* T03 review — if `git diff` of `fixtures.rs`
    exceeds 100 LOC or mutates an existing `fake_*` signature,
    matrix shrinks (drop cells 9 / 21 / 24 to cycle-2 item E).
  - *Status:* unresolved — falsified by T03.

## Predecessors / related

- [`spec/ui-test-harness-bootstrap/feature.md`](../ui-test-harness-bootstrap/feature.md)
  v0.1 — provides `iced_test::screenshot` + the
  [`const SLOTS`](../ui-test-harness-bootstrap/feature.md#r2--viewport-matrix-dev-note-3-layer-3)
  table + `tests/fixtures/visual_diff.rs` + the tiny-skia pin. H2
  caveat carries over (see Risks).
- [`spec/iced-native-widgets/feature.md`](../iced-native-widgets/feature.md)
  v0.1.0 (shipped 2026-05-13) — current cockpit widget surface
  (native `table`/`grid`/`float`). The
  [`positions.rs`](../../crates/ui/src/widgets/positions.rs) +
  [`strategies.rs`](../../crates/ui/src/widgets/strategies.rs)
  cells render the post-shipped native-table form.
- [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](../dev-notes/ui-testability-deep-dive-2026-05-15.md)
  — implements item **C** from
  [`§5.1`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#51-idea-table)
  + [`§3.3`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin)
  + the Q-GALLERY-SCOPE lock.
- [`spec/ui-design-principles.md`](../ui-design-principles.md) —
  Lumen tokens used for cell-header styling; no new tokens.
- [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)
  — all V-items sandbox-safe; only `screencapture` is the orchestrator-
  supervised presenter-deck capture.

## Open questions for architect

The analyst surfaces these for the architect's Design pass. All are
**architect-decide** (none require an operator round-trip), per
[`AGENT.md ## Architect / hypothesis-only`](../../AGENT.md#architect--hypothesis-only).

- **Q-ARCH-1 — `--smoke` flag CLI parse.** `clap::Parser` derive or
  hand-rolled `std::env::args` check? Strawman: `clap` (matches the
  `viewer` bin's existing pattern at
  [`crates/ui/src/bin/viewer.rs`](../../crates/ui/src/bin/viewer.rs)).
- **Q-ARCH-2 — Exhaustiveness test mechanism for parsing
  `widgets/mod.rs`.** Options: (a) `include_str!("widgets/mod.rs")`
  + naive regex (suggested in Design); (b) a `build.rs` that emits
  a `const ALL_WIDGETS: &[&str]` slice; (c) a proc-macro that does
  the same at parse time. Strawman: (a) — 30 LOC of pure
  `std::str` parsing, no build complexity. Architect picks if (a)
  proves fragile for `#[cfg]`-gated mod declarations.
- **Q-ARCH-3 — Single mega-canvas height for the `operator` slot.**
  H-GAL-2 is the load-bearing falsifier. If the architect's M0
  spike falsifies H-GAL-2, the design switches to a fixed-height
  `column!` (no scrollable). Architect commits one shape in Design.
- **Q-ARCH-4 — Cell render-failure handling.** If a fixture builder
  panics or a widget's `pub fn view` returns an error-state Element,
  does the gallery still render the surrounding cells, or does the
  whole canvas blow up? Strawman: each cell is wrapped in
  `iced::widget::Container::new(...)` with no panic-catching
  (consistent with the cockpit's existing tolerance); the
  exhaustiveness test (V3) catches missing cells, the visual
  snapshot test (V6) catches rendering regressions. Architect
  confirms.
- **Q-ARCH-5 — Whether to gate the gallery's snapshot baselines
  behind a `cargo insta accept` ergonomic.** Bootstrap deferred
  this to cycle-1 item F (`iced-test-bytes`). The gallery's
  matches_image path uses the bootstrap's existing helper
  unchanged. Architect confirms no insta-integration work in this
  brief (F is a parallel feature).
- **Q-ARCH-6 — Whether to add a `cargo run -p ui --bin ui-gallery
  --features fixtures` row to
  [`README.md`](../../README.md)** so operators know it exists.
  Strawman: yes; one-line addition. Architect confirms README is
  in-scope for the developer's V9 file list.

## Changelog

- 2026-05-15 (analyst): initial draft. Cycle-1 item C of the locked
  rollout from
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](../dev-notes/ui-testability-deep-dive-2026-05-15.md)
  (§3.3 + §5.1 row C). Q-GALLERY-SCOPE inherited from operator
  lock (reuse `fixtures.rs` directly). Single mega-canvas
  presentation per §3.3 (a) verbatim. 10 V-items, all sandbox-safe
  per AGENT.md ## Capability boundaries. 4 H-* hypotheses seeded;
  H-GAL-2 (scrollable-content capture) is load-bearing for the
  architect's M0 spike. Three baseline PNGs at the
  bootstrap's `floor` / `typical` / `operator` slots. Effort
  budget: 3 dev-days per §5.1 row C. HANDOFF → architect (Design
  section + tasks.md fleshing on the 6 Q-ARCH questions).
