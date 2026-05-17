---
slug: lumen-phase-1-foundation
status: shipped
owner: architect
updated: 2026-05-17
version: 2.0.1
---

# Lumen design adoption — Phase 1: Foundation

> **Phase 1 of 4** in the
> [`lumen-design-adoption`](lumen-design-adoption.md) initiative. Master
> roadmap is the orientation; this brief is the **shippable feature**.
> Operator-locked constraints (no brand, no voice rewrite, sequential
> phases, Phase 4 reserved) are documented in the master file and apply
> here without re-litigation.

## Why

The shipped cockpit's design surface is **operator-correct** (the
[principles doc](../ui-design-principles.md) codifies what is) and
**system-thin** (the implementation is 12 tokens + 6 widgets + flat
elevation). Phase 1 closes the system gap by adopting the
purpose-built Lumen design system's foundational primitives:
**concrete light-mode hexes**, **3-tier elevation language**, **whisper-
shadow language**, **focus-ring language**, **active-row pattern**, and
the **always-visible status bar**.

### What's missing today, concretely

1. **No light-mode hex palette is wired.** The
   [principles doc lines 97–110](../ui-design-principles.md) propose a
   light table but no hex constant lands in
   [`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs). A future
   light-mode ship would have to invent values; Phase 1 wires the
   Lumen-derived values now.
2. **No elevation language.** Every panel today uses the same
   `BG_ELEV` (`#1A1F29`) and the same `BORDER` (`#2A313F`). A modal
   adopting a stronger frame and an inset input field both reach for
   `BORDER_STRONG` (recently added) — but there is no semantic
   `panel_raised` vs `panel_sunken` distinction. The Lumen system
   gives every surface a tier and reads as "different surface, same
   palette".
3. **No whisper-shadow tokens.** Today's panels are flat. The Lumen
   spec calls for whisper shadows
   ([colors_and_type.css:96–104](../design/project/colors_and_type.css))
   that are **barely there** but read as elevation in
   peripheral vision. iced 0.14 supports container shadows; the
   token is missing.
4. **No focus ring.** The principles doc commits to "focus rings use
   `border_strong`, not `accent`"
   ([principles line 354](../ui-design-principles.md)). The Lumen
   focus-ring token (3 px low-alpha accent) is the proper pattern;
   Phase 1 adopts the Lumen ring **on top of** the principles
   border-strong rule (see Q7: principles supersede).
5. **No active-row pattern in tables.** Today the active strategy
   row (when one is selected) renders by token-tinting the row fill.
   The Lumen pattern is a 2 px left rule in `accent`, **no fill
   change**
   ([desktop.css:357–360](../design/project/ui_kits/desktop/desktop.css))
   — this preserves the column-of-numbers rhythm the operator scans.
6. **No status bar.** The cockpit shell has a header (panel titles)
   and a body (panel grid) but nothing at the bottom. Connection
   state lives in a small badge inside the latency widget; account
   identity has no surface; server time has no surface. The Lumen
   status bar
   ([Shell.jsx:67–81](../design/project/ui_kits/desktop/Shell.jsx))
   gives the operator a fixed eye-anchor between scans.
7. **No motion-token semantics.** The principles doc's motion table
   ([principles lines 215–223](../ui-design-principles.md)) is correct
   but unwired — the kill-confirm modal, for example, is timeless
   (no transition). Lumen's motion ladder
   (`dur_1=80 / dur_2=140 / dur_3=220 / dur_4=320 ms` +
   `cubic-bezier(0.22, 0.61, 0.36, 1)`) is a closed scale.

### What Phase 1 ships

- **The full Lumen palette** — both light and dark, contrast-checked,
  including the warm + cool neutral scales, the accent ramp, the
  semantic ramps (sage / clay / warn / info), and the surface tokens
  (canvas / panel / panel_raised / panel_sunken / overlay).
- **Tier 0/1/2/3 elevation tokens** wired through every panel widget,
  with the modal-surface widget adopting Tier 3.
- **Whisper-shadow tokens** (`shadow_1` / `shadow_2` / `shadow_3` +
  `shadow_inset`) on every Tier-1+ surface.
- **Focus ring** on every interactive widget (3 px low-alpha accent
  per Lumen, on top of the principles' `border_strong` element
  border).
- **Spacing** ladder extended to the Lumen scale.
- **Radii** ladder extended to the Lumen scale.
- **Typography** ladder extended to the Lumen 7-step scale (with the
  existing 4-step scale either deprecated-and-aliased or hard-
  replaced — see Q3).
- **Motion** tokens (durations + easings) consumed by the
  `journal_transaction_modal` open/close transition.
- **Active-row pattern** in `widgets::positions` and
  `widgets::strategies`.
- **A new `widgets::status_bar` widget** rendering connection /
  latency / account / server-time, always visible at the bottom of
  the cockpit shell.
- **A rewritten `spec/ui-design-principles.md`** (Lumen-anchored,
  ~300–400 lines, single-file replace).
- **The 36 panel snapshot baselines refresh once** (one-time
  `cargo insta review`); 11 / 11 backtest body-SHA-256 anchors
  unchanged.

### What Phase 1 does NOT ship — load-bearing

- **No brand**. No "Lumen" string. No eye/lens logo. No
  rename of any binary or crate.
- **No `ui::strings` rewrite.** Voice rules unchanged.
- **No icon adoption.** Lucide icons stay deferred per the
  principles' "no icons until needed" rule.
- **No new widgets beyond status bar.** The Lumen UI kit's 16 JSX
  components include OrderBook, OrderTicket, Watchlist, Chart,
  ApprovalQueue, FleetSummary, StrategyDetail, Assistant — all
  out of scope.
- **No backtest-path change.** Anchor risk is zero by construction.
- **No `cockpit` / `cockpit_live` rename.** Both bins refresh in
  place.

### Why now

The
[v1.5b multi-venue feature](v1-5b-multi-venue.md) shipped
2026-05-03 — closing the largest queued backend feature. The cockpit
is now **stable on the data side** (3 venues, 20-symbol USDT+USDC
universe, 1 s aggregated trades) and the **right time** to refresh
the visual surface is *between* large backend changes, not on top of
one. Phase 1 lands cleanly on the v1.5b-shipped base.

The new feature pipeline has no backend-only work pending that
would conflict with a UI refresh. Phase 1 is in a clean window.

## Requirements

Numbered, testable, derived from
[`spec/design/project/colors_and_type.css`](../design/project/colors_and_type.css),
[`spec/design/project/ui_kits/desktop/desktop.css`](../design/project/ui_kits/desktop/desktop.css),
the [Lumen brand book](../design/project/README.md), and the existing
[`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs) +
[`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) shape. Each
ends with a one-line **acceptance** the tester can verify. All
requirements preserve the operator-locked constraints (no brand,
no voice rewrite) and the cross-feature invariants in the
[master roadmap](lumen-design-adoption.md#cross-feature-invariants).

### R1 — Replace `theme::color` palette with Lumen tokens

- **R1.1** Replace the dark-mode hex constants in
  `crates/ui/src/theme.rs` with the Lumen dark palette
  ([`spec/design/project/colors_and_type.css:113–160`](../design/project/colors_and_type.css)):
  `canvas = cool-800 (#131820)`, `panel = cool-700 (#1C2127)`,
  `panel_raised = cool-600 (#2A3038)`, `panel_sunken = cool-900
  (#0B0F15)`, `overlay = rgba(0,0,0,0.55)`, `fg_1 = #E8ECF1`,
  `fg_2 = #B7BFCB`, `fg_3 = #808993`, `fg_4 = #5C6571`,
  `accent = accent-300 (#6FB6AE)`, `border_1 = #232A33`,
  `border_2 = #2E3640`, `border_strong = #404954`.
- **R1.2** Add the **light palette** as a parallel `light` const
  block — enables Q7's principles-doc supersede commitment to a
  contrast-checked light surface even though the runtime mode
  switch is a downstream feature.
- **R1.3** Add the warm + cool **neutral scales** (`warm-25 / 50 /
  100 / 200 / 300 / 400 / 500 / 600 / 700 / 800 / 900`,
  `cool-25 / 50 / 100 / 200 / 300 / 400 / 500 / 600 / 700 / 800 /
  900`) as the **internal source-of-truth** for the surface
  tokens. Widgets never reach for the raw scale; surface tokens
  are the public API.
- **R1.4** Add the **accent ramp** `accent-50 / 100 / 200 / 300 /
  400 / 500 / 600 / 700 / 800 / 900`
  ([colors_and_type.css:18–27](../design/project/colors_and_type.css)).
  The dark-mode `accent` is `accent-300`; the light-mode
  `accent` is `accent-400`.
- **R1.5** Add the **semantic ramps** `up-{50,400,500}` (sage),
  `down-{50,400,500}` (clay), `warn-{50,400,500}`, `info-{50,400,500}`.
  Replace the existing `POS = #3ECF8E`, `NEG = #FF6B6B`, `WARN
  = #FFC45A`, `INFO = #7BC2FF` constants — see R9 for the
  rename strategy.
- **Acceptance:** every hex literal in `theme.rs` is sourced from
  [`spec/design/project/colors_and_type.css`](../design/project/colors_and_type.css);
  zero hex literals appear in `theme.rs` that aren't in the
  Lumen source CSS; the
  `tests::*_has_principles_dark_hex` tests update to assert the
  new hexes.

### R2 — Add Tier 0/1/2/3 surface tokens

- **R2.1** New `theme::color::CANVAS` — Tier 0 app background.
  Maps to dark `cool-800` / light `warm-50`.
- **R2.2** New `theme::color::PANEL` — Tier 1 default panel
  surface. Maps to dark `cool-700` / light `warm-25`.
- **R2.3** Existing `theme::color::BG_ELEV` → renamed
  `theme::color::PANEL_RAISED` (Tier 2 — dialogs, popovers, the
  modal card; `cool-600` dark / `#FFFFFF` light).
- **R2.4** New `theme::color::PANEL_SUNKEN` — input fields, table
  stripes; `cool-900` dark / `warm-100` light.
- **R2.5** Existing `theme::color::BG_OVERLAY` aligns with
  Lumen's `overlay` (`rgba(0,0,0,0.55)` dark / `rgba(20,19,15,0.45)`
  light) — the existing semantics already match; the hex value
  shifts, the role is preserved.
- **Acceptance:** `cargo test --workspace -p ui` exercises a
  `theme::tier_token_presence_test` that asserts each of `CANVAS`,
  `PANEL`, `PANEL_RAISED`, `PANEL_SUNKEN`, `BG_OVERLAY` is non-
  zero and that they form a strict luminance ladder
  (`PANEL_SUNKEN < CANVAS < PANEL < PANEL_RAISED` in dark mode;
  reversed in light mode).

### R3 — Add whisper shadow tokens

- **R3.1** New `theme::shadow::SHADOW_1`, `SHADOW_2`, `SHADOW_3` —
  three soft elevation levels. Spec values from
  [colors_and_type.css:96–104](../design/project/colors_and_type.css)
  (light) and `:150–155` (dark).
- **R3.2** New `theme::shadow::SHADOW_INSET` — sunken inset shadow
  for input fields and table stripes.
- **R3.3** Shadows in dark mode are darker (more black-alpha), not
  bigger — Lumen's specific guidance
  ([README.md:97–98](../design/project/README.md)).
- **R3.4** Define a `theme::shadow::Spec` struct holding offset_x /
  offset_y / blur_radius / colour alpha for each shadow, in a
  shape that maps directly to iced 0.14's
  `iced::widget::container::Style::shadow` field — see Q3 for the
  iced API confirmation.
- **Acceptance:** `theme::shadow::*` constants exist; a
  `shadow_dark_is_more_black_than_light` unit test asserts the
  total alpha-weighted darkness of `SHADOW_1.dark` exceeds
  `SHADOW_1.light` (per the Lumen "darker, not bigger" rule).

### R4 — Add focus-ring token

- **R4.1** New `theme::FOCUS_RING` — Lumen's `0 0 0 3px <accent at
  low alpha>` ([colors_and_type.css:107](../design/project/colors_and_type.css)).
  Light mode uses `rgba(63, 150, 141, 0.28)`; dark mode uses
  `rgba(166, 213, 207, 0.30)`.
- **R4.2** The focus ring is a **box-shadow-equivalent ring**
  rendered via iced container shadow with no offset and a 3 px
  spread-equivalent. Fallback if iced doesn't support
  spread: a 3 px solid border using the same colour, rendered as
  a parent container — architect picks at design time.
- **R4.3** The principles-doc commit "focus rings use `border_strong`,
  not `accent`" (existing principles line 354) is **superseded** in
  the Phase 1 principles rewrite (Q7) by Lumen's accent-ring
  pattern. The `border_strong` token stays in use for hover /
  active borders; the focus state gains the additional ring.
- **Acceptance:** the focus ring renders on the kill-switch
  button when keyboard-focused; visible in the Phase 1
  presentation screenshots.

### R5 — Extend the spacing scale to the Lumen ladder

- **R5.1** Replace `space::XS / S / M / L / XL / XXL` (4 / 8 / 12
  / 16 / 24 / 32) with the Lumen 13-step ladder
  ([colors_and_type.css:165–178](../design/project/colors_and_type.css)):
  `0 / 2 / 4 / 6 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64`.
  Naming convention: `theme::space::{ZERO, TICK, XS, XXS, S, M, L,
  L_PLUS, XL, XXL, XXXL, HUGE, MASSIVE}` — see Q10 for the
  naming-convention ratification.
- **R5.2** Existing call sites (every widget) get a one-time sweep
  to map old → new names. The principle scale is a **superset**
  of the old (4 / 8 / 12 / 16 / 24 / 32 are all preserved with
  identical pixel values), so the migration is a rename, not a
  resize.
- **R5.3** The principles doc's commit "spacing scale is closed"
  (existing principles line 498) is **expanded but still closed**
  — the new closed scale is the Lumen 13-step ladder. No `10` or
  `18` exceptions.
- **Acceptance:** `crates/ui/tests/consistency.rs` rule
  "spacing scale is closed" updates its allow-list to the Lumen
  13-step ladder; every widget compiles against the new names; no
  existing pixel value silently changes.

### R6 — Extend the radii scale

- **R6.1** Replace `radius::SMALL = 2.0` and `radius::MEDIUM = 4.0`
  with the Lumen 5-step radii ladder
  ([colors_and_type.css:181–186](../design/project/colors_and_type.css)):
  `radius_1 = 2 px (dense table inputs)`, `radius_2 = 4 px
  (default control)`, `radius_3 = 6 px (buttons + chips)`,
  `radius_4 = 8 px (cards + panels)`, `radius_5 = 12 px (modals)`,
  `radius_pill = 999 px (tags, toggle thumbs)`.
- **R6.2** The `journal_transaction_modal` adopts `radius_5 (12 px)`
  for its outer card; existing `radius::MEDIUM` (4 px) stays valid
  via alias for the duration of Phase 1's sweep.
- **Acceptance:** `theme::radius::*` exposes the 6-step Lumen
  ladder; every widget compiles against the new names.

### R7 — Add the typography ladder

- **R7.1** Add the Lumen 7-step typography ladder
  ([colors_and_type.css:215–222](../design/project/colors_and_type.css)):
  `fs_display = 32 px`, `fs_h1 = 24 px`, `fs_h2 = 18 px`, `fs_h3 =
  15 px`, `fs_body = 13 px`, `fs_small = 12 px`, `fs_micro = 11 px`.
- **R7.2** Reconcile with the existing 4-step ladder (`caption =
  11`, `body = 13`, `title = 16`, `display = 22`):
  - `caption (11) ≅ micro (11)` — exact match, alias.
  - `body (13) ≅ body (13)` — exact match, alias.
  - `title (16) ≅ h2 (18)` — **2 px shift**; new value adopted
    everywhere via Q3 hard-replace.
  - `display (22) ≅ h1 (24)` — **2 px shift**; new value adopted.
- **R7.3** The principles doc's commit "type scale is closed" stays
  — the **new** closed scale is the Lumen 7-step. The existing
  4-step is **deprecated** (the values change for `title` and
  `display`).
- **R7.4** The font-family contract stays unchanged. Lumen
  prescribes Inter (UI) + JetBrains Mono (numerics); the
  principles doc keeps the platform-default sans + platform-
  default monospace and **does not bundle Inter or JetBrains
  Mono**. Operator-locked: every kilobyte of TTF is a kilobyte
  not spent on faster bar rendering.
- **Acceptance:** `theme::text::*` exposes the 7-step Lumen
  ladder; existing widget call sites for `title` / `display` re-
  point at the new sizes; one-time snapshot baseline updates
  reflect the 2 px shift.

### R8 — Add motion tokens

- **R8.1** New `theme::motion::DUR_1 = 80 ms` (tap feedback),
  `DUR_2 = 140 ms` (hover / focus), `DUR_3 = 220 ms` (panel
  reveal), `DUR_4 = 320 ms` (modal enter)
  ([colors_and_type.css:198–201](../design/project/colors_and_type.css)).
- **R8.2** New `theme::motion::EASE_OUT =
  cubic_bezier(0.22, 0.61, 0.36, 1)` and `EASE_IN_OUT =
  cubic_bezier(0.4, 0, 0.2, 1)`. Encoded as constant arrays of
  control points; converted to whatever shape iced animations
  consume — architect resolves at design time.
- **R8.3** The `journal_transaction_modal` open-transition adopts
  `DUR_4 + EASE_OUT` (220 ms ≅ existing principles-doc 180 ms;
  the new value is 320 ms — slightly longer, matches Lumen's
  modal-enter duration). Operator-visible **only on the modal
  open** — every other widget is timeless or already at 140 ms.
- **R8.4** No bounces, no spring physics
  ([README.md:111](../design/project/README.md)). Existing
  principles "trading UI must never feel kinetic" stays.
- **Acceptance:** `theme::motion::*` exposes the four durations
  + two easings; the journal-tx-modal open transition uses
  `DUR_4 + EASE_OUT`.

### R9 — One-time token-rename sweep

- **R9.1** **Hard-replace existing token names** (no aliases retained
  beyond the immediate Phase-1 PR window) per Q1 recommended
  resolution. Mapping:
  - `BG → CANVAS`
  - `BG_ELEV → PANEL_RAISED`
  - `BG_OVERLAY` (kept; aligns with Lumen `overlay`)
  - `FG → FG_1`
  - `FG_MUTED → FG_2`
  - `BORDER → BORDER_1`
  - `BORDER_STRONG → BORDER_STRONG` (kept; aligns)
  - `ACCENT → ACCENT` (kept; value shifts to teal `#6FB6AE` from
    blue `#5EA3FF`)
  - `POS → UP_500`
  - `NEG → DOWN_500`
  - `WARN → WARN_400`
  - `INFO → INFO_400`
- **R9.2** Every existing widget gets a one-time refactor pass:
  `tape`, `positions`, `pnl`, `kill`, `strategies`,
  `journal_transaction_modal`, plus `frame` / `latency` / `num`
  helpers.
- **R9.3** The `theme::color_for_delta` helper signature is
  unchanged; internally it returns `UP_500 / DOWN_500 / FG_2`
  instead of `POS / NEG / FG_MUTED`. Cross-feature invariant:
  the [`real-mtm-unrealized-pnl`](real-mtm-unrealized-pnl.md)
  feature's positive / negative / zero rendering is preserved
  (semantically identical, hex values shift to sage / clay).
- **R9.4** The `theme::color_for_latency_ms` helper is preserved
  with a band-name reconcile per Q8: green / yellow / red →
  `UP_500 / WARN_400 / DOWN_500`. Bands stay (< 500 ms = OK,
  < 2 s = WARN, ≥ 2 s = NEG / HALTED at ≥ 10 s).
- **Acceptance:** zero references to the old token names
  (`POS`, `NEG`, `BG`, etc.) across `crates/ui/src/`; every
  widget compiles against the new names; the consistency-test
  allow-list updates atomically.

### R10 — Panel widgets adopt Tier 1 styling

- **R10.1** Every panel widget — `tape`, `positions`, `pnl`,
  `strategies`, plus `frame` (the panel-frame helper) — adopts:
  - `background = PANEL` (Tier 1)
  - `border = 1 px solid BORDER_1` (hairline)
  - `border_radius = radius_4 (8 px)`
  - `box_shadow = SHADOW_1` (whisper)
- **R10.2** Panel headers ("Tape", "Positions", etc.) adopt the
  `panel_raised` background tint (Tier 2), one step lighter than
  the panel body — matches
  [desktop.css:174–181](../design/project/ui_kits/desktop/desktop.css).
- **R10.3** The kill-switch panel (`widgets::kill`) adopts Tier 1
  styling identical to other panels; it is no longer visually
  distinct as "the dangerous one" — the **typed-confirm phrase**
  carries that signal, not the panel chrome.
- **Acceptance:** every panel renders with hairline border +
  whisper shadow; visible in the Phase 1 presentation
  screenshots.

### R11 — Input widgets adopt sunken styling

- **R11.1** The kill-switch confirm-input field (where the operator
  types `HALT BTC`) adopts:
  - `background = PANEL_SUNKEN`
  - `border = 1 px solid BORDER_2`
  - `box_shadow = SHADOW_INSET`
  - `border_radius = radius_2 (4 px)`
  Matches [desktop.css:478–485](../design/project/ui_kits/desktop/desktop.css).
- **R11.2** On focus, the input gains the `FOCUS_RING` outline
  (R4.1) and the border shifts to `accent`.
- **R11.3** No other input field exists in the shipped cockpit
  today — the universe is config-driven, no order entry, no
  search bar. R11 covers the kill-confirm input only. Future
  inputs (Phase 2's backtest runner is OUT of scope per the
  master roadmap; Phase 3's pause / override are not inputs)
  inherit the styling automatically.
- **Acceptance:** the kill-confirm input has the sunken styling
  and gains the focus ring on focus.

### R12 — Active-row pattern in tabular widgets

- **R12.1** `widgets::positions` and `widgets::strategies` add an
  **active-row indicator**: a 2 px left rule in `accent`, **no
  fill change** to the row
  ([desktop.css:357–360](../design/project/ui_kits/desktop/desktop.css)).
- **R12.2** "Active" semantics:
  - Positions: the row whose symbol is currently selected via
    keyboard navigation (when keyboard nav lands; Phase 1
    pre-wires the styling without yet wiring the selection
    state).
  - Strategies: the row whose strategy is currently selected
    for the audit-detail view (consumed by the existing
    `tape-row-audit-modal` flow).
- **R12.3** The indicator is **additive**: hover styling
  (`PANEL_SUNKEN` row tint per
  [desktop.css:357](../design/project/ui_kits/desktop/desktop.css))
  stacks below the active rule; an actively-selected hovered
  row shows both.
- **Acceptance:** when the strategies panel has a selection,
  the selected row renders the 2 px accent rule; visible in
  the Phase 1 presentation screenshots.

### R13 — Status bar widget (new)

- **R13.1** New widget `crates/ui/src/widgets/status_bar.rs`. Per
  Q4: separate widget (matches Lumen's component split).
- **R13.2** Layout: horizontal flexbox, 24 px tall,
  `background = PANEL`, `border-top = 1 px BORDER_1`,
  `font-size = fs_micro (11 px)`, `color = FG_3`. Matches
  [desktop.css:124–134](../design/project/ui_kits/desktop/desktop.css).
- **R13.3** Four primary fields, left-to-right:
  - **Connection** — a 6 px coloured dot + a label.
    Connected (data feed up) → `up_500` dot + "Connected ·
    {venue list}". Halted → `warn_400` dot + "Reconnecting".
    Down → `down_500` dot + "Disconnected".
  - **Latency** — `Latency <num> ms`, where `<num>` is the
    median fill-stream latency over the last 60 s. Colour band
    per `theme::color_for_latency_ms`.
  - **Account** — operator identity from `config/agent.toml`
    (e.g. "Paper · BTC sandbox"). Static for the session.
  - **Server time** — local clock or audit DB
    `now_utc()` (Q5 selects). Tabular-figures rendering via
    `widgets::num`.
- **R13.4** Right-side fields (after a flex-grow spacer):
  - **CPU %** — process CPU. Optional in Phase 1 (Q5 lazy
    population); architect can drop if data source is heavy.
  - **Version** — `crates/ui` Cargo version + `· rust`. Static
    for the session.
- **R13.5** The status bar is **always visible** at the bottom of
  the cockpit shell. Implementation: the shell layout becomes a
  `column![titlebar, body, status_bar]` with the body taking
  `Length::Fill` and the status bar taking `Length::Fixed(24)`.
- **R13.6** Both `cockpit` (fixtures bin) and `cockpit_live`
  (unified live bin) render the status bar. Per Q7: both bins
  refresh.
- **Acceptance:** a `status_bar_snapshot_render` insta snapshot
  test renders the status bar in connected / reconnecting /
  disconnected states; visible in the Phase 1 presentation
  screenshots.

### R14 — Existing panel snapshots refresh once

- **R14.1** The existing 36 insta snapshot baselines under
  `crates/ui/tests/snapshots/` refresh once via
  `cargo insta review`. Each diff is reviewed by the
  ui-designer agent; the reviewer attests in the per-snapshot
  pending file that the diff is the expected token / tier /
  shadow / spacing shift, not an unintended regression.
- **R14.2** The snapshot review is a **single-PR commit**: all 36
  baselines update together, the developer runs
  `cargo insta accept`, and the resulting diff is the visible
  artefact in the Phase 1 review.
- **R14.3** New snapshots that land with Phase 1:
  - `status_bar_*` (R13).
  - The `journal_transaction_modal` re-snapshot picks up the
    new Tier 3 shadow + 12 px radius.
  - The Tier 1 panel chrome shows up in every existing panel
    snapshot.
- **Acceptance:** `cargo insta test --workspace` returns clean
  after the accept; no leftover `*.pending-snap` files.

### R15 — Cross-feature invariants

- **R15.1** `operator-success-reports` R7 latency badges: green /
  yellow / red bands map to `up_500 / warn_400 / down_500`. The
  *band thresholds* (< 500 ms, < 2 s, ≥ 2 s, ≥ 10 s) are
  unchanged; the *colours* shift to the new palette per R9.
- **R15.2** `live-cockpit-unified` halted banner: the banner
  uses `down_500` background + `fg_on_accent` text; the
  `AGENT HALTED` string is unchanged.
- **R15.3** `real-mtm-unrealized-pnl` P&L card: the realised /
  unrealised columns render via `color_for_delta`; the helper
  signature is unchanged; the colour values shift sage / clay.
- **R15.4** `per-symbol-position-accounts` strategy-id chip: the
  chip background uses `accent_soft` (a new derived token —
  see Q10) for accent in light mode; in dark mode, the chip
  uses `rgba(111, 182, 174, 0.12)`
  ([colors_and_type.css:134](../design/project/colors_and_type.css)).
- **R15.5** `tape-row-audit-modal` modal frame: the modal adopts
  Tier 3 styling (`PANEL_RAISED + SHADOW_3 + radius_5`); the
  modal-trigger flow (click any tape row → modal opens) is
  unchanged.
- **R15.6** `journal-tx-metadata` modal-header rendering:
  unchanged.
- **R15.7** `v1.5b-multi-venue` venue-tagged tick rendering:
  unchanged. The status bar's "Connected · {venue list}" string
  reads the active-venue list from the EventBus mode channel
  (Q5).
- **Acceptance:** the tester's per-feature invariant table
  shows PASS for all 7 rows.

### R16 — Anchor regression

- **R16.1** All 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) verify byte-identical
  post-Phase 1.
- **R16.2** No new anchor scenarios; no re-lock budget; zero
  exceptions.
- **R16.3** **Architect runs the grep**:
  `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
  spec/reports/` (case-insensitive). Expected count: zero.
- **Acceptance:** `verify-anchors` PASS; the tester's anchor
  table is 11 / 11 PASS.

### R17 — Backwards compat

- **R17.1** `cockpit --features fixtures` (dev bin) launches and
  renders.
- **R17.2** `cockpit_live` (unified live bin) launches and
  renders.
- **R17.3** Both bins consume the same `crates/ui/src/widgets/`;
  Phase 1 does not split the widget code by bin.
- **R17.4** The shipped tape-row-audit-modal flow works
  end-to-end against a fixtures-mode tape (a click on any row
  opens the modal with the new Tier 3 chrome).
- **Acceptance:** the Phase 1 presentation includes a recorded
  run of both bins (or a `capture-screenshot` fallback if the
  presenter is in a headless sandbox); both bins render
  cleanly.

## Verification (V-items)

The tester gates Phase 1 ship against these V-items.

- **V1 — Token presence.** Every Lumen token from R1–R8 has a
  corresponding `theme::*` Rust constant. Verified by a
  `theme_token_presence_test` that lists the expected token names
  and asserts each is non-zero in both modes.
- **V2 — Widget tier compliance.** Every panel widget renders with
  the correct tier (Tier 1 for panels, Tier 2 for panel headers,
  Tier 3 for the modal). Verified by visual review of the 36
  refreshed snapshots; the diff in each snapshot matches the
  expected tier shift.
- **V3 — Active-row visual.** When the strategies panel has a
  selection, the selected row renders the 2 px accent left rule
  with no fill change. Verified by a `strategies_active_row`
  insta snapshot.
- **V4 — Status bar visible.** The status bar appears at the
  bottom of both bins; the four primary fields populate
  correctly per R13. Verified by a `status_bar_*` snapshot
  group + a manual run of both bins.
- **V5 — All 36 existing snapshots refreshed coherently.**
  `cargo insta accept` is run once; `cargo insta test` returns
  clean. Diff review attestation in the PR.
- **V6 — Anchors 11 / 11 PASS.** `verify-anchors` PASS.
- **V7 — Cross-feature invariants.** All 7 invariants in R15
  PASS in the tester's per-feature invariant table.
- **V8 — Dark + light mode parity.** The light palette is wired
  even if the runtime mode switch is downstream — every dark
  token has a light counterpart with the same role. Verified
  by a `light_palette_present` unit test.
- **V9 — Contrast WCAG AA on every text/bg pair.** The principles
  doc's contrast table (R7-superseded) is updated for the new
  palette; every pairing in the table clears AA (4.5:1 body)
  or AAA (7:1 equity). Verified by a future `tests/contrast.rs`
  (the rule lands as a documented gate; the computational test
  is itself a Phase 1 task).

## Backtest Scenarios

_n/a — UI feature, no new backtest scenarios._

The 11 locked backtest body-SHA-256 anchors in
[`spec/anchors.toml`](../anchors.toml) are preserved byte-identical
post-Phase 1 (R16.1, architect-confirmed grep gate R16.3). Phase 1
does not introduce a new strategy, does not modify the report
renderer, does not touch any code path the anchored scenarios
exercise.

## Open questions for architect

These resolutions are deliberately punted to the architect; analyst
provides recommendations but defers the call. Each question maps
to one or more R-items above.

### Q1 — Token-rename strategy: alias vs hard-replace

**The question:** R9's existing-token rename (`BG → CANVAS`,
`POS → UP_500`, etc.) — preserve old names as aliases (`pub const
BG: Color = CANVAS;`) for one phase, then deprecate? Or hard-
replace + sweep call sites in the same PR?

**Recommended (analyst):** **hard-replace + sweep**. Aliases
clutter the namespace and create two ways to spell the same
colour. The sweep is mechanical (`sed` against the widget
files) and reviewable in one diff. The principles doc's
"semantic colour tokens — only `pos`, `neg`, `accent`" rule
gets *better* under the new sage / clay names (`up_500`,
`down_500`) — they're more semantic than `pos` / `neg` / `warn`.

**Alternative:** keep aliases for one phase, deprecate next
phase. Cons: every reviewer of every Phase-2 + Phase-3 PR has
to know which spelling is canonical mid-flight.

### Q2 — Snapshot refresh strategy: single-accept vs side-by-side

**The question:** Phase 1's 36-snapshot refresh — accept the
diff in one PR (`cargo insta review` → `cargo insta accept`),
or extend the snapshot harness to render BOTH old and new for
side-by-side review?

**Recommended (analyst):** **single accept**. Faster review;
the visual diff is what the operator wants to see at the
presentation, not a comparison of two systems. The
side-by-side harness is engineering work that nobody asked
for.

**Alternative:** side-by-side. Pros: explicit "before / after"
artefact for the presentation. Cons: throwaway harness code.

### Q3 — Shadow rendering in iced 0.14

**The question:** does iced 0.14 support container shadows
(`offset_x`, `offset_y`, `blur_radius`, `color`) directly on
`iced::widget::container::Style`? The shipped cockpit uses
flat panels; we haven't exercised the shadow API.

**Recommended (analyst):** **architect confirms via spike** at
design time. iced 0.14 release notes mention
`container::Style::shadow` (added in 0.13); confirm the field
shape and that dark / light shadows can be themed. If the API
doesn't support spread (R4.2), the focus ring falls back to a
parent-container border.

**Risk if false:** shadows are rendered via custom widgets that
draw a soft-edge `image` mask; ugly but possible. Worst case
falls back to a 1 px solid border + no shadow — visibly
flatter, principle preserved.

### Q4 — Status-bar widget separation

**The question:** Phase 1's status bar — separate widget at
`crates/ui/src/widgets/status_bar.rs`, or extend the existing
shell-layout module?

**Recommended (analyst):** **separate widget**. Matches Lumen's
component split (Shell.jsx exports TitleBar + SideRail +
StatusBar as siblings). Matches the existing widgets/ folder
discipline (one file per widget). Future Phase 2 + Phase 3
widgets land alongside, not nested.

**Alternative:** inline in the shell layout. Pros: less
indirection. Cons: violates the widgets/ discipline and the
"three uses → refactor" rule kicks in late.

### Q5 — Status-bar data-flow architecture

**The question:** the status bar's four fields (connection,
latency, account, server time) come from four backend surfaces:

- **Connection** — from the EventBus mode channel
  (`agent::runtime::EventBus`)? Or from a new
  `agent::feed_health` query?
- **Latency** — from the existing `widgets::latency` band logic?
  Reused as-is, or a separate median-over-60-s calculation?
- **Account** — from `config/agent.toml`'s identity block?
- **Server time** — from local clock (`std::time::SystemTime`),
  or from the audit DB's `now_utc()` helper?

**Recommended (analyst):** architect picks per data-flow
architecture at design time. **Tentative recommendation:**

  - Connection: EventBus mode channel (existing).
  - Latency: existing `widgets::latency` band logic + a new
    median calculator (60 s rolling).
  - Account: `config/agent.toml`'s `[identity]` block (new
    config field if one doesn't exist; falls back to the
    operator email).
  - Server time: local clock (the audit DB's `now_utc()` is
    deterministic-friendly but irrelevant for a status bar).

**Risk:** the audit DB approach for server time would deliver
a single source-of-truth across panels, but at the cost of a
DB query every paint frame. Local clock is fine for a status
bar.

### Q6 — Light vs dark mode default

**The question:** does Phase 1 ship with **dark as default** (the
existing principles-doc lock) or **toggle on first run**? The
Lumen brand book says light + dark are "equal weight"
([README.md:23](../design/project/README.md)).

**Recommended (analyst):** **dark stays the default** per the
principles doc's
[long-session justification](../ui-design-principles.md#dark--light-mode-parity).
Lumen's "equal weight" is about both modes being maintained,
not about which is the cold-start default. Architect ratifies.

**Alternative:** light as default. Cons: the operator's session
is long and dim-room-biased; light at midnight is hostile.

### Q7 — Principles-doc supersede shape

**The question:** the existing 599-line
[`spec/ui-design-principles.md`](../ui-design-principles.md) gets
**replaced** with a Lumen-anchored rewrite (~300–400 lines,
single-file replace), or **split** into two files (Lumen tokens
in a new file; project-specific patterns stay in the existing)?

**Recommended (analyst):** **single-file replace**. The
existing doc's project-specific patterns (kill-switch typed
confirm, latency bands, P&L colour rules, "show the why") are
preserved verbatim in the new doc; the **token tables** at the
top are replaced with the Lumen palette + tier + shadow + motion
+ spacing tables. **Estimated section breakdown of the new
doc:**

  - Why this document exists — preserved verbatim (~30 lines).
  - Aesthetic direction — preserved verbatim (~50 lines, still
    cites Bloomberg / Linear / Stripe).
  - Visual language — **rewritten**. Tokens table replaced with
    Lumen tokens; tier system added; whisper-shadow language
    added; focus-ring rule updated per Q4.2 / R4.3 (~120 lines).
  - Component principles — preserved verbatim (~80 lines).
  - Voice and copy — preserved verbatim (~30 lines), with a new
    one-paragraph note that our voice rules align with Lumen's
    (per the operator-locked constraint #2).
  - Trading-specific patterns — preserved verbatim (~60 lines).
  - Dark / light mode parity — **rewritten** to commit to the
    Lumen-derived light palette as the source of truth (~30 lines).
  - Consistency enforcement — preserved verbatim (~30 lines).
  - What's NOT in scope — preserved verbatim (~30 lines).
  - Open questions — refreshed to reflect Phase 1 / 2 / 3 / 4
    state (~20 lines).

**Total: ~480 lines** (down from 599; the savings come from
dropping the proposed-light-table at the top now that the values
are in code).

**Alternative:** two-file split. Pros: token doc and
patterns-doc owned by different agents. Cons: every UI review
has to read two files.

### Q8 — Latency-band-name reconcile

**The question:**
[`operator-success-reports`](operator-success-reports.md) R7
latency badges use the labels "OK / Slow / High / Halted" with
green / yellow / red colour mapping. Lumen's voice and labels are
not directly specified for latency badges. Does Phase 1:

- (a) Keep the existing labels (OK / Slow / High / Halted) +
  swap the colours to the new palette (`up_500 / warn_400 /
  down_500`).
- (b) Adopt Lumen's "Connected / Reconnecting / Disconnected"
  vocabulary for the **status bar's connection field** but
  keep "OK / Slow / High / Halted" for the **latency badge**.
- (c) Reconcile fully — single vocabulary for both surfaces.

**Recommended (analyst):** **(b)**. The status bar's connection
field is a **boolean state** (up / reconnecting / down); the
latency badge is a **continuous-band measurement** (OK / Slow /
High / Halted). Different vocabularies because different
semantics. This matches the Lumen status bar's status-dot
pattern
([Shell.jsx:71](../design/project/ui_kits/desktop/Shell.jsx))
and preserves the operator-success-reports R7 contract.

### Q9 — Existing kill-switch tooltip / dialog scope

**The question:** the kill-switch tooltip + confirm dialog
(operator-approved 2026-05-03 in the principles doc) — does
Phase 1 touch their behavior, or only their visual chrome?

**Recommended (analyst):** **NO behavior change**. The
typed-confirm phrase `HALT BTC` is unchanged; the principles-doc
lock stays. **Visual changes only:** the dialog adopts Tier 3
chrome (`PANEL_RAISED + SHADOW_3 + radius_5`); the input field
adopts sunken styling per R11; the confirm-button's focus state
gains the focus ring per R4. Behavior, copy, phrase: untouched.

## Design

_Architect-owned. Resolves Q1–Q9 + the cross-cutting master Q10 (token
naming convention) — every recommendation lands as **ratified** unless
called out under "Architect override". Task breakdown lives at
[`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../tasks/lumen-phase-1-foundation.md);
this section is the design contract the developer reads alongside it._

### Open-question resolutions

| Q   | Question                              | Resolution                                                                                                                             |
|-----|---------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| Q1  | Token-rename: alias vs hard-replace   | **Hard-replace + sweep call sites** in T1502. Aliases rot. The 36-snapshot review absorbs the diff once.                              |
| Q2  | Snapshot refresh                      | **Single accept** via `cargo insta review` + `cargo insta accept` after the dev pipeline lands (T1511).                                |
| Q3  | iced 0.14 shadow API                  | **Confirmed first-class.** `iced_core-0.14.0/src/shadow.rs` ships `Shadow { color, offset: Vector, blur_radius }`; `container::Style` exposes a `shadow: Shadow` field. T1503 wires it. |
| Q4  | Status-bar widget separation          | **Separate widget** at `crates/ui/src/widgets/status_bar.rs`. Matches Lumen's component split + the existing one-file-per-widget rule. |
| Q5  | Status-bar data flow                  | Connection from `EventBus::market_health` (existing v1.5b watchdog); latency from existing `widgets::latency` band logic; account derived from `agent::config::Config.mode + universe`; server time from local `std::time::SystemTime`. See "Status bar data sources" below. |
| Q6  | Light vs dark default                 | **Dark stays default.** Light palette wired (V8) but the runtime mode toggle is downstream; cold-start is `iced::Theme::Dark`.        |
| Q7  | Principles-doc supersede              | **Single-file replace.** `spec/ui-design-principles.md` is rewritten Lumen-anchored (~480 lines, T1510). Voice section preserved verbatim per operator-locked Constraint 2. |
| Q8  | Latency-band reconcile                | **Option (b) — split vocabulary.** Status bar uses Connected / Reconnecting / Disconnected; latency badge keeps OK / Slow / High / Halted. Colours swap to `up_500 / warn_400 / down_500`; thresholds (500 ms / 2 s / 10 s) unchanged.   |
| Q9  | Kill-switch tooltip / dialog scope    | **Visual chrome only — no text or behavior change.** `HALT BTC` typed-confirm phrase preserved; modal adopts Tier 3 chrome; input adopts sunken styling; focus state gains the new ring. |
| Q10 | Token naming convention (master)      | **Flat `theme::color::*` constants** in SHOUTY_SNAKE_CASE. Matches existing shape; no submodules. `theme::color::PANEL_RAISED`, not `theme::color::tier::raised`. |

**No principled overrides.** Analyst recommendations are operator-aligned
and consistent with the master roadmap's Constraint 1 / Constraint 2
locks; the architect ratifies all nine.

### Crate map delta

```
crates/ui/
├── Cargo.toml              [unchanged — iced 0.14.0 already supports shadows]
├── src/
│   ├── theme.rs            [REWRITE — 12 → ~50 tokens; full Lumen palette + tiers + shadows + motion + typography]
│   ├── strings.rs          [unchanged — operator-locked Constraint 2]
│   ├── state.rs            [unchanged for Phase 1; status bar reads existing fields]
│   ├── live.rs             [+1 subscriber recipe — `MarketHealth` channel feeds status bar connection state]
│   ├── widgets/
│   │   ├── frame.rs        [Tier 1 styling — hairline + whisper shadow + tinted bg]
│   │   ├── kill.rs         [Tier 1 panel chrome; sunken input on confirm field; focus ring]
│   │   ├── latency.rs      [token rename only — `color_for_latency_ms` returns up_500/warn_400/down_500]
│   │   ├── pnl.rs          [Tier 1 panel chrome; `color_for_delta` returns up/down]
│   │   ├── positions.rs    [Tier 1 + active-row pattern]
│   │   ├── strategies.rs   [Tier 1 + active-row pattern + chip uses accent_soft]
│   │   ├── tape.rs         [Tier 1 panel chrome; rows still tape semantics]
│   │   ├── num.rs          [token rename only]
│   │   ├── journal_transaction_modal.rs [Tier 3 chrome — shadow_3 + radius_5 + overlay]
│   │   └── status_bar.rs   [NEW — connection / latency / account / server time / version]
│   └── bin/
│       ├── cockpit.rs      [shell becomes column![body, status_bar]; top-level container = canvas]
│       └── cockpit_live.rs [same shell change; subscribes MarketHealth + plumbs to status bar]
└── tests/
    ├── consistency.rs      [no rule changes — hex-only check; new tokens still live in theme.rs]
    └── snapshots/          [36 baselines refresh + new status_bar snapshots land]

spec/
├── architecture.md         [Frontend section — tier system + status_bar widget + principles supersede pointer]
├── features/lumen-phase-1-foundation.md [this file — Design appended]
├── tasks/lumen-phase-1-foundation.md    [NEW — T15xx]
└── ui-design-principles.md [REWRITE — Lumen-anchored ~480 lines]
```

No new crate dependencies. Library-compat checklist is trivially
satisfied because nothing is added — iced 0.14.0 is already pinned.

### Token mapping table — Q1 hard-replace

The 12 existing tokens map to Lumen names in T1502. Hex values shift to
the Lumen palette; semantic role is preserved. Old names are deleted
in the same PR (Q1: hard-replace, no aliases).

| # | Old (theme.rs)          | Old hex (dark) | New name             | New hex (dark)         | New hex (light)        | Lumen source                     |
|---|-------------------------|----------------|----------------------|------------------------|------------------------|----------------------------------|
| 1 | `color::BG`             | `#11141A`      | `color::CANVAS`      | `#131820` (cool-800)   | `#F6F4EF` (warm-50)    | `colors_and_type.css:73, 114`    |
| 2 | `color::BG_ELEV`        | `#1A1F29`      | `color::PANEL_RAISED`| `#2A3038` (cool-600)   | `#FFFFFF`              | `colors_and_type.css:75, 116`    |
| 3 | `color::BG_OVERLAY`     | `#0B0D12`      | `color::OVERLAY`     | `rgba(0,0,0,0.55)`     | `rgba(20,19,15,0.45)`  | `colors_and_type.css:77, 118`    |
| 4 | `color::FG`             | `#E8ECF2`      | `color::FG_1`        | `#E8ECF1`              | `#14130F` (warm-900)   | `colors_and_type.css:79, 120`    |
| 5 | `color::FG_MUTED`       | `#8B93A3`      | `color::FG_3`        | `#808993`              | `#6F6A5E` (warm-500)   | `colors_and_type.css:81, 123`    |
| 6 | `color::ACCENT`         | `#5EA3FF`      | `color::ACCENT`      | `#6FB6AE` (accent-300) | `#3F968D` (accent-400) | `colors_and_type.css:21, 89, 131`|
| 7 | `color::POS`            | `#3ECF8E`      | `color::UP_500`      | `#6E9B6A` (sage)       | `#547A52`              | `colors_and_type.css:57, 137`    |
| 8 | `color::NEG`            | `#FF6B6B`      | `color::DOWN_500`    | `#C97B5E` (clay)       | `#A95F46`              | `colors_and_type.css:62, 141`    |
| 9 | `color::WARN`           | `#FFC45A`      | `color::WARN_500`    | `#E0B45C`              | `#B7862F`              | `colors_and_type.css:65, 144`    |
|10 | `color::INFO`           | `#7BC2FF`      | `color::INFO_500`    | `#84A6D0`              | `#436A9A`              | `colors_and_type.css:69, 147`    |
|11 | `color::BORDER`         | `#2A313F`      | `color::BORDER_1`    | `#232A33`              | `#E2DDD2` (warm-200)   | `colors_and_type.css:85, 127`    |
|12 | `color::BORDER_STRONG`  | `#3A4456`      | `color::BORDER_STRONG`| `#404954`             | `#9E9788` (warm-400)   | `colors_and_type.css:87, 129`    |

**Architect note on Q8 / R9.4 reconcile:** the analyst's R9.1 says
`POS → UP_500`, `NEG → DOWN_500`, `WARN → WARN_400`, `INFO →
INFO_400`. The Lumen CSS exposes both `*-400` and `*-500` ramps; the
**canonical default** in the CSS body uses `*-500` for `up` / `down`
text colour (`colors_and_type.css:295–296`) and `*-400` only inside
the `[data-theme="dark"]` block (`:298`). The architect ratifies
**`UP_500` / `DOWN_500`** as the public token names (matching
analyst R9.1) and ships **dual mode-keyed values** behind each name —
i.e. `theme::color::UP_500` is `Color { dark: #6E9B6A, light:
#547A52 }` via the dual-palette struct (see "Theme module shape"
below). For warn / info we adopt **`WARN_500` / `INFO_500`** as the
public default (R9.1 used `WARN_400` / `INFO_400`; the architect
prefers `_500` for stylistic alignment with `UP_500` / `DOWN_500` —
the named-band semantic is what matters, the suffix is bookkeeping).
The mapping table above is canonical.

### Theme module shape

```rust
// crates/ui/src/theme.rs — post-Phase-1 (T1501)

pub mod color {
    use iced::Color;

    /// Dual-mode token. `Color::dark` is the cold-start render
    /// (Q6 — dark default); `Color::light` is wired but selected only
    /// when the runtime theme switches (downstream feature).
    #[derive(Debug, Clone, Copy)]
    pub struct ModeColor { pub dark: Color, pub light: Color }

    impl ModeColor {
        /// Resolve to the active theme — dark in cold start.
        pub const fn current(&self, mode: ThemeMode) -> Color { /* ... */ }
    }

    // Surface tier tokens
    pub const CANVAS:        ModeColor = /* warm-50 / cool-800 */;
    pub const PANEL:         ModeColor = /* warm-25 / cool-700 */;
    pub const PANEL_RAISED:  ModeColor = /* #FFFFFF / cool-600 */;
    pub const PANEL_SUNKEN:  ModeColor = /* warm-100 / cool-900 */;
    pub const OVERLAY:       ModeColor = /* see table */;

    // Foreground
    pub const FG_1: ModeColor;  pub const FG_2: ModeColor;
    pub const FG_3: ModeColor;  pub const FG_4: ModeColor;
    pub const FG_ON_ACCENT: ModeColor;

    // Accent ramp + soft
    pub const ACCENT:       ModeColor;  pub const ACCENT_HOVER:  ModeColor;
    pub const ACCENT_PRESS: ModeColor;  pub const ACCENT_SOFT:   ModeColor;

    // Semantic ramps (sage / clay / warn / info)
    pub const UP_50:    ModeColor;  pub const UP_400:   ModeColor;  pub const UP_500:   ModeColor;
    pub const DOWN_50:  ModeColor;  pub const DOWN_400: ModeColor;  pub const DOWN_500: ModeColor;
    pub const WARN_50:  ModeColor;  pub const WARN_400: ModeColor;  pub const WARN_500: ModeColor;
    pub const INFO_50:  ModeColor;  pub const INFO_400: ModeColor;  pub const INFO_500: ModeColor;

    // Borders
    pub const BORDER_1: ModeColor;  pub const BORDER_2: ModeColor;
    pub const BORDER_STRONG: ModeColor;
}

pub mod shadow {
    use iced::{Color, Shadow, Vector};
    /// Each level holds two layered shadows + alpha set per mode.
    /// Iced takes one `Shadow` per container::Style; we collapse the
    /// layered Lumen spec into the dominant outer shadow (the inner
    /// 1px shadow is achieved via the hairline border, not a second
    /// Shadow draw).
    pub fn shadow_1(mode: ThemeMode) -> Shadow { /* offset (0,1), blur 2, dark/light alpha */ }
    pub fn shadow_2(mode: ThemeMode) -> Shadow { /* offset (0,4), blur 10 */ }
    pub fn shadow_3(mode: ThemeMode) -> Shadow { /* offset (0,12), blur 28 */ }
    /// Sunken input — rendered as a 1px inner-top border because iced
    /// `Shadow` is outer-only. See "Shadow rendering" below.
    pub fn shadow_inset(mode: ThemeMode) -> Color { /* hairline alpha */ }
}

pub mod focus {
    use iced::{Color, Shadow};
    /// 3px low-alpha accent ring rendered as an outer container shadow
    /// with offset (0,0) and blur ~3.
    pub fn ring(mode: ThemeMode) -> Shadow;
}

pub mod space {
    pub const ZERO: u32 = 0;     pub const TICK: u32 = 2;
    pub const XXS:  u32 = 4;     pub const XS:   u32 = 6;
    pub const S:    u32 = 8;     pub const M:    u32 = 12;
    pub const L:    u32 = 16;    pub const L_PLUS: u32 = 20;
    pub const XL:   u32 = 24;    pub const XXL:    u32 = 32;
    pub const XXXL: u32 = 40;    pub const HUGE:   u32 = 48;
    pub const MASSIVE: u32 = 64;
}

pub mod radius {
    pub const R1: f32 = 2.0;  pub const R2: f32 = 4.0;
    pub const R3: f32 = 6.0;  pub const R4: f32 = 8.0;
    pub const R5: f32 = 12.0; pub const PILL: f32 = 999.0;
}

pub mod text {
    pub const MICRO:   u32 = 11;  pub const SMALL: u32 = 12;
    pub const BODY:    u32 = 13;  pub const H3:    u32 = 15;
    pub const H2:      u32 = 18;  pub const H1:    u32 = 24;
    pub const DISPLAY: u32 = 32;
}

pub mod motion {
    use std::time::Duration;
    pub const DUR_1: Duration = Duration::from_millis(80);
    pub const DUR_2: Duration = Duration::from_millis(140);
    pub const DUR_3: Duration = Duration::from_millis(220);
    pub const DUR_4: Duration = Duration::from_millis(320);
    /// Cubic-bezier control points; consumed by future iced animation
    /// crate or hand-rolled interp helpers. No bouncing.
    pub const EASE_OUT:    [f32; 4] = [0.22, 0.61, 0.36, 1.0];
    pub const EASE_IN_OUT: [f32; 4] = [0.4,  0.0,  0.2,  1.0];
}

/// Theme mode — `Dark` is the cold-start (Q6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode { #[default] Dark, Light }
```

The `ThemeMode` argument flows through every render path. For Phase 1
the `cockpit` and `cockpit_live` bins always pass `ThemeMode::Dark`
(matches `iced::Theme::Dark`). Adding a runtime toggle is a downstream
ship; the wired light values mean V8 (light palette parity) can be
verified by a unit test even before the toggle exists.

### Tier system specification

| Tier      | Surface           | Used by                                                   | Visual recipe                                                              |
|-----------|-------------------|-----------------------------------------------------------|----------------------------------------------------------------------------|
| Tier 0    | `CANVAS`          | Top-level shell container in `cockpit.rs` / `cockpit_live.rs` | flat `canvas` bg, no border, no shadow                                  |
| Tier 1    | `PANEL`           | `widgets::frame::panel`, `tape`, `positions`, `pnl`, `kill`, `strategies`, `latency`, `status_bar` | `panel` bg, 1 px hairline `BORDER_1`, `radius::R4` (8 px), `shadow_1`     |
| Tier 1.h  | `PANEL_RAISED`    | Panel **headers** only (header strip inside a Tier 1 frame) | `panel_raised` bg, no own shadow (lives inside Tier 1's shadow), `border-bottom: 1 px BORDER_1` |
| Tier 2    | `PANEL_RAISED`    | Dialogs, popovers (none in Phase 1; reserved for future Cmd-K) | `panel_raised` bg, hairline border, `radius::R4`, `shadow_2`           |
| Tier 3    | `PANEL_RAISED`    | `journal_transaction_modal` modal card                    | `panel_raised` bg, hairline `BORDER_1`, `radius::R5` (12 px), `shadow_3`, sits on `OVERLAY` backdrop |
| Sunken    | `PANEL_SUNKEN`    | Kill-switch confirm input field; future table stripes     | `panel_sunken` bg, 1 px `BORDER_2`, `radius::R2` (4 px), `shadow_inset` (1 px inner-top) |

The `frame::panel` helper grows a `Tier` arg (default Tier 1) and a
`header: bool` flag (default true) so individual widgets can opt the
header in or out without forking the helper.

### Active-row pattern (R12)

Lumen's active-row pattern is a 2 px left rule in `accent`, no fill
change. iced doesn't have a per-side border colour; we render this as
**a `Container` wrapping the row, styled with a left border via the
`Border::width` field unevenly** — but `iced::Border` only supports
uniform width. The canonical solution is a **two-element `Row`**:

```rust
fn active_row<Msg>(active: bool, content: Element<'_, Msg>) -> Element<'_, Msg> {
    let rule_color = if active { color::ACCENT.current(mode) } else { Color::TRANSPARENT };
    let rule = Container::new(Space::new())
        .width(Length::Fixed(2.0))
        .height(Length::Fill)
        .style(move |_t| container::Style { background: Some(rule_color.into()), ..Default::default() });
    Row::new().push(rule).push(content).spacing(0).into()
}
```

The rule is **always present** (occupies 2 px); only the colour
toggles. This guarantees zero layout shift between active and inactive
rows — the operator's column-of-numbers rhythm stays steady.

`widgets::positions` and `widgets::strategies` consume `active_row`.
For Phase 1, `widgets::positions` has no selection state today;
T1507 wires the helper but leaves `active = false` for every row
(the styling lands; the wiring of which row is "active" is a
downstream feature). `widgets::strategies` already tracks
`selected_strategy_id` for the audit-modal flow — that flag drives
the active rule.

### Shadow rendering — iced 0.14 verification (Q3)

**Confirmed:** iced 0.14.0 ships `iced::Shadow` (in `iced_core::shadow`)
with fields `color: Color`, `offset: Vector`, `blur_radius: f32`. The
`iced::widget::container::Style` struct exposes a public `shadow:
Shadow` field. Verified via the iced_core 0.14.0 source layout
(`src/shadow.rs` present in the registry checkout used by the
project's compiled `target/debug/deps/iced_core-*.d`). Existing
`container::Style { background, border, text_color, ..Default::default() }`
patterns in the codebase work because the `..Default::default()` falls
through to `Shadow::default()` (zero offset, transparent). T1503 makes
the field explicit on Tier-1+ surfaces.

**Three-level shadow ladder — concrete iced values:**

| Token         | Mode  | offset (x, y) | blur_radius | color (rgba)              |
|---------------|-------|---------------|-------------|---------------------------|
| `shadow_1`    | dark  | (0.0, 1.0)    | 2.0         | (0, 0, 0, 0.30)           |
| `shadow_1`    | light | (0.0, 1.0)    | 2.0         | (20, 19, 15, 0.04)        |
| `shadow_2`    | dark  | (0.0, 4.0)    | 10.0        | (0, 0, 0, 0.35)           |
| `shadow_2`    | light | (0.0, 4.0)    | 10.0        | (20, 19, 15, 0.06)        |
| `shadow_3`    | dark  | (0.0, 12.0)   | 28.0        | (0, 0, 0, 0.50)           |
| `shadow_3`    | light | (0.0, 12.0)   | 24.0        | (20, 19, 15, 0.08)        |

The Lumen CSS layers two box-shadows per level (`0 1px 1px ... , 0 1px
2px ...`). iced takes one shadow per container — the architect collapses
to the **outer / wider** shadow (the dominant visual layer). The inner
hair-shadow is inherited from the 1 px hairline border (which already
draws an edge at the same colour budget). This is a bounded
approximation; the visual fidelity loss is sub-perceptual at the
"barely there" alpha values Lumen specifies.

**`shadow_inset` — outer-only API workaround:** iced's `Shadow` is
outer-only (no `inset` flag). The Lumen sunken-input look is achieved
via **a 1 px solid line at the top inside edge of the input**, drawn
as a thin `Container` row above the input's content. This matches the
visual that `inset 0 1px 0 rgba(...)` produces in CSS. Implementation
reuses the active-row "two-element Row" trick rotated 90°. No special
iced API needed.

**`focus_ring` — accent ring via outer shadow:** the 3 px low-alpha
accent ring renders as `Shadow { color: accent_alpha28, offset: (0,
0), blur_radius: 3.0 }`. This is the iced-idiomatic equivalent of
CSS `box-shadow: 0 0 0 3px rgba(...)`. Iced doesn't natively render a
spread but a 3 px blur with offset 0 produces a soft halo that reads
as the same visual signal. The principles-doc focus-ring rule is
superseded per R4.3 — `BORDER_STRONG` stays the default border on
hover / interactive elements; the **focus state** layers the new ring
on top.

**Fallback if shadow render breaks under tiny-skia:** Phase 1 uses
the `tiny-skia` renderer (per `Cargo.toml: features = ["tiny-skia",
...]`). The architect spike at T1503 verifies tiny-skia honours
`Shadow` correctly. If tiny-skia's shadow draw is broken or visibly
wrong, the fallback is **flat panels with a hairline border + 1 px
luminance shift between Tier 1 and the canvas** — readable as
elevation without the whisper-shadow language. This degrades V2
(visual tier compliance) from "shadow-driven" to "tint-driven" but
preserves R10 (hairline border + tier separation). The presenter
flags the fallback explicitly if it lands.

### Status bar widget specification (R13)

**File:** `crates/ui/src/widgets/status_bar.rs`. Single `view(state:
&Cockpit) -> Element<Message>` entry point.

**Layout (left → right):**

```
┌─────────────────────────────────────────────────────────────────┐
│ ● Connected · Binance · Coinbase · Kraken    Latency 124 ms    │
│   Paper · USDT 20-symbol            [flex spacer]               │
│                              Server 2026-05-04 14:32:08 UTC    │
│                              CPU —    v0.X.Y · rust            │
└─────────────────────────────────────────────────────────────────┘
height: 24 px • bg: PANEL • border-top: 1 px BORDER_1 • font: 11 px FG_3
```

**Field-by-field data sources (Q5 ratified):**

| Field         | Source (Phase 1)                                                    | Update cadence            | Population |
|---------------|---------------------------------------------------------------------|---------------------------|------------|
| Connection    | `EventBus::market_health` (existing v1.5b watchdog) → new `Cockpit::market_health: HashMap<Venue, MarketHealthState>` field; widget reduces to `{ all-fresh, any-stale, none-fresh }` ternary | event-driven (debounced) | eager via boot snapshot of Config-declared venues, then live updates |
| Latency       | Existing `widgets::latency` band logic — read `Cockpit.latency` Latency::Known { ms } and pass through `theme::color_for_latency_ms` | per tick (existing)       | lazy — shows `—` until first tick |
| Account       | Derived: `format!("{} · {} {}", config.mode, universe_label, symbol_count)`. `universe_label = if config.universe.usdc_enabled { "USDT+USDC" } else { "USDT" }`. Symbol count from `config.universe.symbols.len()`. | static for session        | eager — populated at boot from Config |
| Server time   | `std::time::SystemTime::now()` formatted RFC 3339 to second precision; iced `time::every` subscription at 1 Hz | 1 Hz                      | eager — first paint |
| CPU %         | **Optional / deferred to R13.4 lazy** — Phase 1 ships `CPU —` placeholder. A `sysinfo` dep would be the cheapest path; architect defers because adding the dep crosses the library-compat budget for a non-load-bearing field. | n/a Phase 1               | placeholder dash |
| Version       | `env!("CARGO_PKG_VERSION") + " · rust"` formatted const at compile time | static                    | eager       |

**Status-dot colour rule (Q8 vocabulary):**

| State           | Dot colour    | Label                               |
|-----------------|---------------|-------------------------------------|
| Connected       | `UP_500`      | `"Connected · {venue list}"`        |
| Reconnecting    | `WARN_500`    | `"Reconnecting · {venue}"`          |
| Disconnected    | `DOWN_500`    | `"Disconnected"`                    |

The venue list reads from `config.data.sources` (`binance` always; +
`coinbase` / `kraken` when enabled per v1.5b config). The widget
sorts venue display order alphabetically for determinism (matches
the v1.5b "HashMap iteration sorted before any cross-run comparison"
discipline). String content is added to `ui::strings` per the no-
inline-prose rule (T1508 lists the new constants).

**Why `EventBus::market_health` over a new `feed_health` query:** the
v1.5b watchdog (`agent::runtime::spawn_market_health_watchdog`)
already publishes `MarketHealth::{Fresh, Stale, Recovered}` on
`bus.market_health()`. Adding a new query path duplicates the
contract. The cockpit subscribes via `ui::live::subscription` (one
new recipe in T1508, slot tenth alongside the nine existing).

**Why local clock over audit `now_utc()`:** the audit DB's `now_utc`
helper exists for deterministic write ordering; reading it from a UI
1 Hz tick injects a DB query into a paint loop. Local clock is fine
for a status indicator — operator's eye is reading "is it stuck?",
not "what is the canonical truth?".

**Shell wiring (R13.5):** the cockpit shell becomes
`column![body, status_bar].height(Length::Fill)` with `body` at
`Length::Fill` and `status_bar` at `Length::Fixed(24.0)`. Both bins
share the same shell layout; same `view()` body; same status-bar
call. The status bar's iced `Subscription` (server-time tick) folds
into the bin's existing `Subscription::batch` block.

### Test strategy (per V-item)

| V   | Test name (file)                                                    | Asserts                                                                                                              |
|-----|---------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| V1  | `theme_token_presence_test` (`crates/ui/src/theme.rs:tests::*`)     | Each of `CANVAS`, `PANEL`, `PANEL_RAISED`, `PANEL_SUNKEN`, `OVERLAY`, `FG_1..4`, `ACCENT*`, `UP/DOWN/WARN/INFO_{50,400,500}`, `BORDER_{1,2,STRONG}` is non-zero in both modes. |
| V1b | `tier_token_presence_test` (theme.rs)                               | Strict luminance ladder `PANEL_SUNKEN < CANVAS < PANEL < PANEL_RAISED` in dark; reversed in light.                  |
| V1c | `shadow_dark_is_more_black_than_light` (theme.rs)                   | `shadow_1.dark.color.alpha > shadow_1.light.color.alpha` (Lumen "darker, not bigger" rule).                          |
| V2  | Refreshed `panel_snapshots__*.snap` (`crates/ui/tests/snapshots/`)  | 32 panel snapshots reflect the tier shift; reviewer attests in PR description that each diff = expected token shift. |
| V3  | `strategies_active_row` (`tests/snapshots/panel_snapshots__strategies_active.snap` — NEW) | Selected row's left-rule colour is `ACCENT`; non-selected row's left-rule is transparent; row content unshifted.   |
| V4  | `status_bar_*` (4 new snapshots in `tests/snapshots/`)              | `status_bar_connected`, `status_bar_reconnecting`, `status_bar_disconnected`, `status_bar_with_latency`.            |
| V5  | `cargo insta test --workspace -p ui`                                | After T1511 `cargo insta accept`, no `*.pending-snap` files; `cargo insta test` returns clean.                       |
| V6  | `scripts/verify_anchors.sh`                                         | 11 / 11 anchor body-SHA-256 PASS (R16.1).                                                                           |
| V6b | Architect grep `R16.3`                                              | `grep -rni "lumen\\|panel-raised\\|panel-sunken\\|cool-800" spec/reports/` → zero matches (locked into the tester report). |
| V7  | Cross-feature invariant table (tester report — NEW section)         | 7 / 7 PASS rows; the tester runs each prior feature's existing test suite + checks each invariant in R15.            |
| V8  | `light_palette_present` (theme.rs)                                  | For every `ModeColor` token, `.light` differs from `.dark` and is non-zero.                                          |
| V9  | `tests/contrast.rs` (NEW — under T1510 principles supersede)        | Every `(fg, bg)` pair in the contrast table clears WCAG AA (4.5:1 body) or AAA (7:1 equity numbers). Implemented as a relative-luminance computation; bands fail loudly. |

**Snapshot accept workflow (V5 / T1511 — load-bearing):**

```
$ cargo test -p ui --features fixtures               # produces .pending-snap files
$ cargo insta review                                  # interactive: inspect each diff
$ cargo insta accept                                  # writes baselines after review
$ cargo test -p ui --features fixtures               # green; no pending files left
```

The reviewer (ui-designer agent during their pass on T1511) checks
each diff matches the expected pattern: `pos → up_500`, `neg →
down_500`, `fg_muted → fg_3`, `accent` value shifts blue→teal.
Anything else gets pushed back to T1502 (sweep missed a call site)
or T1505/T1506/T1507 (tier styling regressed unintentionally).

### Anchor & determinism guardrails

- **R16.1:** all 11 backtest body-SHA-256 anchors verify byte-identical.
  Phase 1 touches only `crates/ui/`; the report renderer lives in
  `crates/reports/` and `crates/backtest/` — no UI dep flows into
  either path. The tester runs `verify-anchors` as a hard gate.
- **R16.3 grep:** `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800" spec/reports/`
  must return zero. The architect ratifies this as a tester-report
  required row; tester fails if any match.
- **No determinism concerns:** UI-only feature. No new `SystemTime::now()`
  reachable from a backtest replay — the status-bar's wall-clock read
  is in the cockpit binary's iced subscription, not in any
  strategy / audit / exec / backtest module.
- **No new RNG** — no random sourcing.
- **No new money math** — no `f64` introduction.
- **Front-matter discipline preserved:** the new principles doc
  (T1510) is a spec doc, not a hashed report; no anchor implications.

### Cross-feature invariants (R15 — wiring sketch)

Below is the implementation sketch the developer threads through
each feature's existing tests:

| Feature                       | Invariant                          | How preserved in Phase 1                                                       |
|-------------------------------|------------------------------------|--------------------------------------------------------------------------------|
| `operator-success-reports`    | Latency badges colour contract     | `theme::color_for_latency_ms` returns `UP_500 / WARN_400 / DOWN_500` instead of `POS / WARN / NEG`; Q8(b) keeps the labels.        |
| `live-cockpit-unified`        | Halted banner trips on file watch / kill / heartbeat | Banner uses `DOWN_500` bg + `FG_ON_ACCENT` text; `AGENT HALTED` string unchanged in `ui::strings`. |
| `real-mtm-unrealized-pnl`     | P&L card columns; `color_for_delta` signature unchanged | `color_for_delta` keeps `(Decimal) -> Color` signature; internal returns shift to `UP_500 / DOWN_500 / FG_3`. |
| `per-symbol-position-accounts`| Strategy-id chip                   | Chip background uses new `ACCENT_SOFT` token (rgba(111,182,174,0.12) dark / `#ECF6F5` light).            |
| `tape-row-audit-modal`        | Modal reachable + frame contract   | Modal frame adopts Tier 3 chrome (`PANEL_RAISED + SHADOW_3 + R5`); trigger flow unchanged; `BORDER_STRONG` still drawn. |
| `journal-tx-metadata`         | Modal header `description + strategy_id` | No code change — Tier 3 chrome inherits via `journal_transaction_modal::view`'s style closure. |
| `v1.5b-multi-venue`           | Venue-tagged ticks + reconnect events | `cockpit_live` subscription preserved; status bar reads venue list from same `MarketHealth` channel — additive consumer, no change to producer. |

The tester's per-phase report includes a `## Cross-feature invariants`
section — one row per feature, PASS / FAIL.

### Risks + mitigations

| #  | Risk                                                                                       | Likelihood | Mitigation                                                                                                                              |
|----|--------------------------------------------------------------------------------------------|-----------:|-----------------------------------------------------------------------------------------------------------------------------------------|
| 1  | Snapshot drift cascades — a missed call site in T1502 shows up as a "wrong colour" snapshot 36× | Medium     | T1502 grep gate: `grep -rn "ACCENT\|BG\|FG\|POS\|NEG\|WARN\|INFO\|BORDER" crates/ui/src/` matches old names → must be zero before T1511 runs.   |
| 2  | iced 0.14 `Shadow` API surface different than expected (e.g. `tiny-skia` renderer ignores blur) | Low–Medium | T1503 spike runs first — paints a smoke test that visually confirms a panel renders with a soft edge under tiny-skia. Fallback documented (see "Shadow rendering"). |
| 3  | Principles-doc rewrite-vs-merge — agents in flight reference old line numbers              | Low        | T1510 is parallel-safe (spec only) but the developer's R citations include line-number-anchored paths; new doc preserves section anchor names so existing links survive. |
| 4  | Cross-crate token sweep ripple — non-`ui` crates reference `ui::theme` indirectly          | Low        | `grep -rn "ui::theme" crates/` → only `crates/ui/` (verified at architect time). No external consumer.                                  |
| 5  | `cockpit_live` binary visual regression — live bin renders differently from fixtures bin   | Medium     | T1514 forces both bins to launch in the gate. T1509 wires the status bar identically into both bins. ui-designer reviews fixtures-bin screenshots; presenter runs both bins side-by-side. |
| 6  | Dark-mode-only operators losing day-mode polish — light palette wired but never visually tested | Low        | V8 unit test asserts non-zero light values; V9 contrast test asserts WCAG AA on every pair. The runtime light-mode toggle is not in Phase 1, but the values are first-class, not TODOs. |
| 7  | Status-bar latency staleness — operator sees a stale `124 ms` after the feed dropped       | Low        | `MarketHealth::Stale` event flips the connection field to "Reconnecting" (warn dot); the latency value is rendered alongside but the operator's eye reads the dot first. Acceptable. |
| 8  | `cargo insta accept` accepts a regression by mistake                                       | Low        | Two-step: T1511 dev runs `cargo insta review` (interactive); ui-designer agent then re-runs `cargo insta test` and visually inspects a sampled subset (5 of 36) before signoff. |
| 9  | `MarketHealth` channel doesn't exist in fixtures bin                                       | Low        | The fixtures path falls back to `MarketHealthState::Fresh` for every Config-declared venue (no "stale" simulation in fixtures). Status bar shows "Connected" persistently in the dev bin — by design, not a bug. |
| 10 | Iced `Shadow::default()` introduces an unintended shadow on every existing Tier 0 surface  | Low        | `Shadow::default()` in iced is `{ color: TRANSPARENT, offset: zero, blur: 0 }` — visually a no-op. Verified by T1503 spike. Existing `..Default::default()` patterns continue to render flat. |

### Operator-locked invariants the design preserves (R15 + cross-cutting)

- **No brand adoption (master Constraint 1).** No `"Lumen"` string
  in any title, no logo asset, no wordmark. The `frame::panel`
  helper takes a `title: &str` from `ui::strings` — none of those
  strings change.
- **No voice rewrite (master Constraint 2).** `ui::strings` is
  byte-identical pre/post Phase 1. The principles-doc rewrite
  (T1510) preserves the entire "Voice and copy" section verbatim.
- **No icon adoption.** Existing widgets render text labels
  (`"Stop trading"`, `"Close"`, `"Connected"`); no Lucide import,
  no glyph font. The principles-doc rewrite documents this lock
  explicitly under the rewritten Iconography section: "Lumen
  prescribes Lucide; we defer until a specific text label fails
  the operator's scan-test."
- **Backtest path untouched.** `crates/strategy/`, `crates/audit/`,
  `crates/exec/`, `crates/backtest/`, `crates/reports/` — none of
  these crates depend on `ui::theme`. Anchors are byte-stable by
  construction.
- **Determinism guardrails (architect.md):** no new
  `SystemTime::now()` in any backtest path (status-bar wall-clock
  is iced-subscription only); no `f64` math; no `thread_rng`; no
  audit-row schema change.

### Sample snapshot diff expectation (informational — for the operator review at T1511)

The 32 panel snapshots are textual summaries (`color: pos`,
`color: neg`, `color: fg_muted`). After T1502's hard-replace, the
diffs the reviewer sees in `cargo insta review` will look like:

```diff
 panel: latency
 title: Feed latency
 badge: Ok
 label: OK
-color: pos
+color: up_500
 value: 120 ms
```

```diff
 rows:
-  BTCUSDT qty=0.45 cost=18000.00 mark=40500.00 pnl=225.00 pnl_color=pos pct=1.25 exp=18.30
-  BNBUSDT qty=60.00 cost=18000.00 mark=300.00 pnl=0 pnl_color=fg_muted pct=0 exp=18.00
-  ETHUSDT qty=7.50 cost=18300.00 mark=2400.00 pnl=-300.00 pnl_color=neg pct=-1.64 exp=18.20
+  BTCUSDT qty=0.45 cost=18000.00 mark=40500.00 pnl=225.00 pnl_color=up_500 pct=1.25 exp=18.30
+  BNBUSDT qty=60.00 cost=18000.00 mark=300.00 pnl=0 pnl_color=fg_3 pct=0 exp=18.00
+  ETHUSDT qty=7.50 cost=18300.00 mark=2400.00 pnl=-300.00 pnl_color=down_500 pct=-1.64 exp=18.20
```

The 4 modal snapshots pick up an additional `tier: 3` /
`shadow: shadow_3` line if the snapshot summary is extended to
include those fields (recommended in T1505); otherwise they shift
the same colour-name set. The new `status_bar_*` snapshots are
born-greenfield — no diff, just baseline lock.

### Implementation parallelism map

```
T1501 (theme.rs rewrite — foundation gate, sequential)
  └─ T1502 (call-site sweep, sequential after T1501)
        ├─ T1503 (shadow spike + ladder — parallel)
        ├─ T1504 (focus ring — parallel)
        ├─ T1505 (Tier 1 panels — parallel)
        ├─ T1506 (sunken inputs — parallel)
        ├─ T1507 (active-row pattern — parallel)
        ├─ T1508 (status_bar widget — parallel; no shared file)
        └─ T1510 (principles-doc supersede — parallel; spec-only)
              │
              ▼
        T1509 (status_bar shell wiring — sequential after T1505 + T1508)
              │
              ▼
        T1511 (snapshot refresh — sequential after T1505/T1506/T1507/T1509)
              │
              ▼
        T1512 (cross-feature invariants verify — sequential)
              │
              ▼
        T1513 (anchor regression — sequential)
              │
              ▼
        T1514 (backwards compat: both bins launch — sequential)
              │
              ▼
        T_FINAL_LUMEN_PHASE_1 (tester gate — VERDICT → presenter on PASS)
```

Eight T-tasks fan out after T1502; the gate narrows back at T1509
(status bar joins the shell) and T1511 (snapshot refresh after every
visual lands).

## Implementation

_developer fills this — task list at
[`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../tasks/lumen-phase-1-foundation.md)._

## Verification — links

_tester fills this — links to
`spec/lumen-design-adoption/phase-1-foundation/reports/test-<timestamp>-lumen-phase-1-foundation.md`._

## UI

_ui-designer fills this — links to refreshed snapshots and
the Phase 1 presentation under `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md` (phase-1-foundation section)._

## Changelog

- 2026-05-03 (analyst): initial Phase 1 brief. 17 R-items
  (R1–R17), 9 V-items (V1–V9), 9 open questions for the
  architect (Q1–Q9). Adopts Lumen tokens (full palette, tier
  system, shadows, focus ring, spacing, radii, typography,
  motion), Tier 1 panel styling, sunken inputs, active-row
  pattern, status bar widget, principles-doc supersede.
  Operator-locked constraints inherited from the
  [master roadmap](lumen-design-adoption.md): no brand,
  no voice rewrite, sequential phasing, dark-default. Anchor
  risk: zero — UI feature, no backtest path touched. 36
  panel snapshot baselines refresh once. Cross-feature
  invariants for 7 prior shipped features documented.
  HANDOFF → architect (Phase 1 first; master roadmap for
  orientation).
- 2026-05-04 (architect): appended `## Design`. Q1–Q9
  resolved (hard-replace, single-accept, iced 0.14 shadow
  confirmed via `iced_core-0.14.0/src/shadow.rs`, separate
  status_bar widget, Q5 data sources from `EventBus::market_health`
  + `widgets::latency` + `Config` + local clock, dark default,
  single-file principles supersede, Q8(b) split vocabulary,
  visual chrome only on kill switch). Master Q10 ratified
  (flat `theme::color::*` SHOUTY_SNAKE_CASE constants). Token
  mapping table covers all 12 existing tokens; ~50 new tokens
  scoped. Tier system spec (Tier 0/1/2/3 + Sunken), active-row
  pattern (transparent-default 2 px rule via Row composition),
  shadow ladder with iced 0.14 `Shadow` values + `tiny-skia`
  fallback, focus-ring as outer-shadow (R4.3 supersedes
  principles `border_strong`-only rule). Status bar field
  contract: connection from `MarketHealth` watchdog,
  account derived from `Config.mode + universe`, server
  time local 1 Hz, CPU placeholder, version from
  `CARGO_PKG_VERSION`. 10 risks with mitigations; cross-
  feature invariants wired per R15 row. Anchor risk verified
  zero by construction (no `crates/strategy/audit/exec/backtest/`
  touch). Task list at [`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../tasks/lumen-phase-1-foundation.md)
  with 14 T15xx tasks + tester `T_FINAL_LUMEN_PHASE_1` gate.
  HANDOFF → developer (T1501 foundation gate first; multi-way
  fan-out at T1503–T1508 + T1510 after T1502 lands).
- 2026-05-17 (architect): additive palette extension — four new
  comparison-line color tokens (`ACCENT_2`, `ACCENT_3`, `ACCENT_4`,
  `ACCENT_5`) documented in
  [`spec/dev-notes/lumen-accent-palette-extension-2026-05-17.md`](../../dev-notes/lumen-accent-palette-extension-2026-05-17.md).
  Forced by `ui-rethink-phase-a-lab` operator-decision Q-A1; values
  added to `crates/ui/src/theme.rs` as part of that feature's M1 work.
  Phase 1 foundation contract is unchanged; only the palette grows.
