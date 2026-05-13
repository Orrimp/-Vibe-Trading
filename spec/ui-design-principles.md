---
slug: ui-design-principles
status: living
owner: ui-designer
updated: 2026-05-04
---

# UI Design Principles

## Why this document exists

This is the living design contract for the trading agent's user interface
— the iced-based `cockpit` (live ops) and `viewer` (offline backtest reports)
binaries in `crates/ui/`. Every future UI feature is measured against the
rules here.

It is **not** a feature brief. There is no acceptance test, no task list, no
ship date. It is the constitution: when an architect proposes a new panel,
when an analyst writes user-visible copy, when a developer adds a button —
this is the document they consult before reaching for `theme.rs` or `strings.rs`.

It is also **not** a redesign. The shipped surface (cockpit + viewer in
iced 0.14, five panels, 9 semantic colors, 4 type sizes, 6 spacing tokens)
is the floor. This document codifies what is, justifies why, and proposes
extensions only where the shipped scale is **missing** something an operator
will need at v1.5+ scale. No breaking changes; everything below is additive
or a clarified rule.

The audience is two future readers:

1. The **operator** (single user, also the developer) — so they know what
   to expect and what to push back on.
2. **Future ui-designer agents** — so design decisions are not re-litigated
   per feature.

## Aesthetic direction

The cockpit is an instrument, not a product. The operator runs it for
six- to twelve-hour sessions while real money is on the line. Every visual
decision is measured against one question: *does this make the operator
faster, safer, and less tired at hour ten than they were at hour one?*

Three reference points the operator will recognize:

- **Bloomberg Terminal** — for **density**. Numbers per square centimeter.
  Color reserved for signal (positive / negative / warning), never decoration.
  Monospaced digits, right-aligned columns, four-digit minor-tick precision.
  We are not Bloomberg-ugly; we are Bloomberg-disciplined.
- **Linear / Vercel** — for **typographic taste**. One quiet sans-serif,
  generous whitespace inside panels, restrained shadows, content-first
  hierarchy. The cockpit should feel like a 2026 product, not a 1996
  exchange terminal.
- **Stripe Dashboard** — for **clarity of state**. Empty / loading / error
  / ready are first-class for every component, with copy that names the
  next action. State is never inferred from the absence of pixels.

What we are **not**:

- Not skeuomorphic. No knobs, no LED-style displays, no faux-paper textures.
- Not glassmorphic. Frosted blurs cost GPU and obscure numbers.
- Not gradient-heavy. Solid surfaces only; gradients are a code smell.
- Not animation-rich. Trading UIs that move when nothing has happened are
  liars by construction. **Bounded state transitions** (fade-in on first
  paint, focus-ring pulse on tab, panel slide on screen change, spinner
  during real I/O) are allowed when motion signals that something
  specific has happened. Motion that runs continuously without an event
  behind it stays forbidden. See `Motion` below. (Operator decision Q-O1
  2026-05-13, recorded in
  [`spec/iced-ecosystem-evaluation/feature.md`](iced-ecosystem-evaluation/feature.md).)
- Not dark-mode-only by default of laziness. Both modes are first-class
  (see `Dark / light mode parity`).

## Visual language

All tokens below live in `crates/ui/src/theme.rs`. Names are the Rust constant
names — grep the source to confirm. Color constants are `ModeColor` with
`.current(mode: ThemeMode) -> Color` unless noted otherwise.

### Color palette

29 color constants in `theme::color`. Hex values are pinned by `t1501_*` tests
in `theme.rs` — that file is the hex source of truth.

**Surface tiers:** `CANVAS` · `PANEL` · `PANEL_RAISED` · `PANEL_SUNKEN` ·
`OVERLAY` — see Tier elevation model below.

**Foreground:** `FG_1` (primary) · `FG_2` (secondary) · `FG_3` (tertiary /
labels) · `FG_4` (placeholder / disabled) · `FG_ON_ACCENT` (on-accent text).

**Accent ramp:** `ACCENT` (primary interactive) · `ACCENT_HOVER` · `ACCENT_PRESS`
· `ACCENT_SOFT` (chip/highlight fill, low-alpha).

**Semantic ramps** — three steps each (_50 soft tint, _400 brighter, _500 default):

| Group | _50 tint | _400 shade | _500 default | Domain          |
|-------|----------|------------|--------------|-----------------|
| Up    | `UP_50`  | `UP_400`   | `UP_500`     | Positive P&L    |
| Down  | `DOWN_50`| `DOWN_400` | `DOWN_500`   | Negative P&L    |
| Warn  | `WARN_50`| `WARN_400` | `WARN_500`   | Latency / caution |
| Info  | `INFO_50`| `INFO_400` | `INFO_500`   | Observation-only |

**Borders:** `BORDER_1` (Tier 1 hairline) · `BORDER_2` (sunken-input /
table-stripe divider) · `BORDER_STRONG` (hover / active, base for focus rings).

**R4.3 — Focus ring rule.** `focus::ring` (3 px low-alpha accent halo,
zero offset, iced `Shadow`) layers **on top of** `BORDER_STRONG` for every
keyboard-focused element — `BORDER_STRONG` is the resting border, the ring
adds the accent glow so keyboard users distinguish focused-from-active.

### Tier elevation model

| Tier    | Surface token   | Border          | Shadow                      | Notes                        |
|---------|-----------------|-----------------|-----------------------------|------------------------------|
| 0       | `CANVAS`        | none            | none                        | Top-level shell only         |
| 1       | `PANEL`         | `BORDER_1`      | `shadow_1`                  | Every panel widget           |
| 2       | `PANEL_RAISED`  | `BORDER_2`      | `shadow_2`                  | Dialogs, popovers, dropdowns |
| 3       | modal card      | `BORDER_STRONG` | `shadow_3` + `OVERLAY` scrim| Kill-confirm, future Cmd-K   |
| Sunken  | `PANEL_SUNKEN`  | `BORDER_2`      | `shadow_inset`              | Input fields, table stripes  |

### Shadow ladder

`shadow::shadow_1/2/3` (in `theme::shadow`) return iced `Shadow`.
`shadow_inset` returns `Color` — iced's outer-only shadow API means the inset
is a 1 px hairline container on the input's top edge.

| Function       | Dark (offset_y, blur, alpha) | Light (offset_y, blur, alpha) |
|----------------|------------------------------|-------------------------------|
| `shadow_1`     | (1, 2, 0.30)                 | (1, 2, 0.04)                  |
| `shadow_2`     | (4, 10, 0.35)                | (4, 10, 0.06)                 |
| `shadow_3`     | (12, 28, 0.50)               | (12, 24, 0.08)                |
| `shadow_inset` | white @ 3 % alpha            | warm-900 @ 4 % alpha          |

Lumen rule: dark mode uses **darker alpha, not bigger blurs**.

### Spacing ladder

13 steps in `theme::space`. The consistency-test allow-list is exactly:
`0 / 2 / 4 / 6 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64`.

| `ZERO`=0 | `TICK`=2 | `XXS`=4 | `XS`=6 | `S`=8 | `M`=12 | `L`=16 | `L_PLUS`=20 | `XL`=24 | `XXL`=32 | `XXXL`=40 | `HUGE`=48 | `MASSIVE`=64 |

If a feature thinks it needs `10` or `18`, the answer is "use `M` or `L` and
adjust the font size". This is the most violated rule on casual feature work.

### Border radii

6 steps in `theme::radius`: `R1`=2 px (dense table inputs), `R2`=4 px (default
control), `R3`=6 px (buttons/chips), `R4`=8 px (cards/panels), `R5`=12 px
(modals/sheets), `PILL`=999 px (tags, toggle thumbs, status-bar dots).

### Typography ladder

7 steps in `theme::text`. Font stacks in `theme::font`: `FONT_SANS` (Inter for
UI) and `FONT_MONO` (JetBrains Mono for numerics).

| Constant  | px | Use                                                |
|-----------|----|----------------------------------------------------|
| `MICRO`   | 11 | Column headers, timestamps, status-bar text        |
| `SMALL`   | 12 | Small labels                                       |
| `BODY`    | 13 | Default body / UI text                             |
| `H3`      | 15 | Sub-section headers                                |
| `H2`      | 18 | Panel titles, card headings                        |
| `H1`      | 24 | Page-level headings, equity number                 |
| `DISPLAY` | 32 | Hero numbers, halted banner                        |

Sans for UI, mono for all numerics — non-negotiable. Italic: never, except
verbatim error messages from upstream. Number formatting via `widgets::num`:
right-aligned, locale separator, sign shown only on deltas, color-of-sign
via `theme::color_for_delta` only.

### Density

Two modes: **compact** (cockpit) and **comfortable** (viewer).

| Metric                    | Compact | Comfortable |
|---------------------------|---------|-------------|
| Table row height          | 24 px   | 32 px       |
| Table cell horizontal pad | 12 px   | 16 px       |
| Panel inner pad           | 16 px   | 24 px       |
| Panel outer gap           | 16 px   | 24 px       |
| Card title → body gap     | 12 px   | 16 px       |
| Dialog inner pad          | 24 px   | 24 px       |

### Motion

Trading UIs must look **still when nothing is happening**. Four durations and
two easings in `theme::motion`. No bounces.

| Token          | Value                              | Use                        |
|----------------|------------------------------------|----------------------------|
| `DUR_1`        | 80 ms                              | Tap / button press         |
| `DUR_2`        | 140 ms                             | Hover, focus transition    |
| `DUR_3`        | 220 ms                             | Panel reveal               |
| `DUR_4`        | 320 ms                             | Modal enter / exit         |
| `EASE_OUT`     | `cubic-bezier(0.22, 0.61, 0.36, 1)`| Panel / modal, no overshoot|
| `EASE_IN_OUT`  | `cubic-bezier(0.4, 0.0, 0.2, 1)`  | Symmetric transitions      |

No auto-advancing carousels. No parallax. No idle animation. No 60 fps chart
animations. Never animate the kill switch — confirm dialogs open immediately.

### Iconography

**No icons until needed.** v0–v1.5 ships zero icons; every element is text-
labeled. When v2+ legitimately needs an icon: Lucide line icons (1.5 px
stroke), sizes 16/20/24 px, `FG_3` default / `ACCENT` hover, tooltip constant
in `ui::strings` mandatory.

## Component principles

These are the rules of thumb every widget obeys.

### Numbers are scannable

Right-aligned. Monospaced digits. Locale separator. Color **only** on
signed deltas (`pos` / `neg`), never on absolute balances. Zero stays
`fg_muted` — it is "no movement", not "neutral movement".

This is the most operator-noticeable principle. A misaligned column is
an operator squinting; a colored absolute is the operator misreading
"$52,341" as a positive event.

### No blank screens

Every panel renders one of `Loading`, `Empty`, `Error`, `Ready`. "No
data" is not a state — write what the operator should do next:

- **Loading**: explicit verb ("Connecting to the fill stream…", not
  "Fetching…"). Three dots ellipsis (Unicode `…`, not `...`).
- **Empty**: actionable hint. "No strategies loaded. Drop a TOML under
  `config/strategies/` to begin." beats "No data".
- **Error**: name the cause **and** the next action. "Trading agent
  disconnected. Check the agent log and restart it." beats "Error: ECONNRESET".
- **Ready**: the happy path. Numbers, rows, surfaces.

The `PanelState<T>` enum in `state.rs` enforces this at the type level.
A widget that doesn't handle all four arms doesn't compile.

### Plain language

Operator vocabulary, not engineer vocabulary. The shipped strings get
this right:

- "Stop trading" beats "Halt agent".
- "P&L today" beats "intraday realized delta".
- "Open positions" beats "non-flat inventory".
- "Strategy is armed and watching" beats "Sentinel mode active".

When a term of art is unavoidable (Sharpe, drawdown, z-score, hedge
ratio, funding rate) — surface a one-line tooltip in `ui::strings`.
Never ship a terminus the operator might guess wrong.

### Sensible defaults

- **Most-used view first.** The cockpit's left-column-tape +
  right-column-positions/strategies layout is correct because it's what
  the operator looks at most.
- **Favorites at top.** When v1.5+ adds multi-symbol dashboards,
  pinned symbols sort first.
- **Remember last choice.** Time-range selectors, density toggles,
  theme — persist to a `config/cockpit-state.toml`.
- **The "right" button is the default.** The Cancel button in a
  destructive dialog is enter-keyed; the Confirm button only fires
  on explicit click after the safety phrase.

### Confirm destructive actions

The "typed safety phrase" pattern is locked. Every destructive surface:

| Action               | Phrase    | State                                  |
|----------------------|-----------|----------------------------------------|
| Stop trading         | `HALT BTC`| Shipped (`KILL_SAFETY_PHRASE`)          |
| Close all positions  | `FLATTEN` | Future v1.5+                            |
| Cancel all orders    | `CANCEL`  | Future v1.5+                            |
| Override risk veto   | `OVERRIDE`| Future v2+                              |

The phrase is **case-sensitive** and matched on exact equality. The
Confirm button stays disabled (iced `on_press = None`) until the typed
input matches. Mismatch shows a `WARN`-colored hint. The Cancel button
is always enabled and is the keyboard default.

Undo is offered where physically possible (e.g. an unsent order).
Where the action has hit the venue (a sent order, a fill, a halted
agent), undo is not lied about — the dialog body says what happens
and the operator decides.

### Show the why

Every order, signal, fill, risk veto, and strategy event is
click-through to its decision trail in the audit ledger. The operator
should never wonder "why did the agent do that".

The current cockpit links the kill switch to the runbook. The pattern
extends: in v1+, every tape row is clickable to its `journal_transaction`
in a viewer-style modal. Every strategy row is clickable to its
`strategy_events` history. This is a future feature, not a shipped
one — the principle is that the audit ledger is the canonical "why",
and the UI is its read-only window.

### Accessibility minimums

- **Keyboard navigation** for every interactive element. `Tab` cycles
  the focus order; `Enter` triggers the focused button; `Esc` closes
  the topmost modal. Focus rings use `border_strong`, not `accent`,
  so the keyboard user can tell focused-from-active.
- **Contrast** ≥ 4.5:1 (AA) for body text, ≥ 7:1 (AAA) for the equity
  display. Verified against the table in `Color palette` above.
- **Color is never the only signal.** Buy / Sell are colored AND
  labeled "BUY" / "SELL". Halted state is colored AND banner-labeled
  "AGENT HALTED". A red-green colorblind operator never loses
  information.
- **Screen-reader labels** on every interactive element. iced support
  is incomplete here; the rule stands as a documented intent and a
  future task when iced's a11y story matures.
- **No motion-only signals.** A flashing border without a copy change
  is not enough.

## Voice and copy

How strings sound:

- **Direct.** Imperative mood for buttons ("Stop trading", "Confirm
  stop"). Declarative for state ("AGENT HALTED", "No strategies
  loaded").
- **Terse.** No "Please". No "Sorry". No "We". The agent does not
  apologize, and the cockpit does not have a personality. It is an
  instrument.
- **Present-tense.** "Connecting…" not "Will connect to…". "No fills
  yet" not "Fills have not been received".
- **Sentence case.** Labels start with a capital, no trailing period.
  Full sentences (banner bodies, error states) end with a period.
- **No jargon without a tooltip.** If the word would not appear in a
  trading-Slack message between two humans, it doesn't ship.
- **Error messages name the cause AND the next action.** Pattern:
  `<what's broken>. <what to check>.` — e.g. "Trading agent
  disconnected. Check the agent log and restart it." This is already
  enforced by the shipped `CONNECTION_*` constants.

The `ui::strings` module is the single review surface. A copy review
is reading one file in one diff.

Our voice rules are independently aligned with the directness-and-brevity
principles in the Lumen voice table (operator-locked Constraint 2 — no
Lumen voice rewrite; the alignment is structural, not adoptive).

## Trading-specific patterns

Patterns that exist because this is a trading UI, not a generic web
dashboard.

### P&L coloring

- **Positive** P&L → `UP_500`.
- **Negative** P&L → `DOWN_500`.
- **Zero** P&L → `FG_3` (NOT `FG_1`, NOT a third "neutral" color).

The reason zero is muted, not foreground: when the operator scans a
column of P&L, the zeroes should fade so positives and negatives jump
out. A column of all-zeroes (no movement on a quiet bar) should look
like background, not data.

This is implemented in `theme::color_for_delta` and is the only legal
source for delta colors. Widgets calling it inline rather than picking
their own `UP_500` / `DOWN_500` per-row is enforced by the consistency tests.

### Position sizing display

Every position row shows **both** absolute base quantity and notional
value, side by side. Operators think in both at different times of day
("I have 0.4 BTC" vs "I have $40k of BTC exposure"). Hiding either
forces a mental calculation the operator should not be doing.

Future v1.5+ pattern (when chart-of-accounts adds per-strategy
positions): the position row also carries a **strategy-id chip** so
the operator can see which strategy is responsible for which lot.

### Latency badges

Bands are pinned in `theme::latency`:

| Range          | Band  | Color      | Label    |
|----------------|-------|------------|----------|
| `< 500 ms`     | OK    | `UP_500`   | "OK"     |
| `< 2 s`        | WARN  | `WARN_500` | "Slow"   |
| `< 10 s`       | HIGH  | `DOWN_500` | "High"   |
| `≥ 10 s`       | HALTED| `DOWN_500` + banner | "Halted" |

Three-band thresholds are deliberate — two bands (OK / not-OK) loses
the operator's grace period; four bands (OK / Slow / High / Halted)
gives a "things are getting worse" signal before the kill threshold.
The same color is used for High and Halted by design — once the
operator sees `DOWN_500`, they look at the label, not the color.

### Kill-switch confirmations

Already covered in `Confirm destructive actions`. Re-affirming the
non-negotiable: the kill button is a sealed, typed-confirm flow with
the safety phrase `HALT BTC`. The exact phrase is not negotiable
mid-session — changing it during operations is a recipe for the
operator typing the previous phrase under stress.

### Numbers that change frequently — flash on update

P&L, last price, equity, latency: subtle 200 ms fade from `ACCENT` (or
`PANEL_RAISED` in the case of equity) back to the resting color when the
value updates. The flash is **subtle** — it is barely perceptible at
the corner of the eye, and invisible to the operator who is staring
straight at it. It is the "the number changed and you didn't miss
it" backup channel.

Hard rule: the flash never **flickers**. A new value that arrives 50 ms
after the previous flash queues, it does not interrupt. A value that
hasn't changed numerically (same float to same float) does not flash.

This is a v1+ task; v0 ships static rendering. The principle is locked
so the implementation lands without re-litigation.

### Charts — price plot with audit-anchored markers

Charts (Phase 2+) render a **per-symbol price series** with **buy/sell
markers** drawn from the audit ledger. The pattern is opinionated:

- **Background = `PANEL`**; **gridlines = `BORDER_1`** (1 px, low-alpha
  horizontals only — no vertical grid; vertical noise competes with
  the marker triangles).
- **Series style** — line series in `ACCENT` for the default plot;
  OHLC candles in `UP_500` (close > open) / `DOWN_500` (close ≤ open)
  for the candlestick variant. Architect resolves the default at
  Phase 2 design (see master roadmap Q11–Q14).
- **Buy markers** = upward triangle in `UP_500`, anchored at the
  fill price on the time axis. **Sell markers** = downward triangle
  in `DOWN_500`. Markers never use a fill colour different from the
  P&L colour pair — the operator's "green = my side won, red = my
  side lost" mental model carries over from the P&L card.
- **Marker source = audit ledger**, never a runtime accumulator.
  This is the same rule as "ledger is single source of truth for
  P&L" applied to fills: the chart shows what the audit query
  returns, not what the cockpit thinks happened. Any
  ledger / chart divergence is a data bug surfacing through the
  visual cross-check the chart was added to enable.
- **Visible window** = fixed 60 minutes of 1-minute bars by default;
  pan/zoom is out of scope for Phase 2 and may land in a later phase.
- **Empty state** — when the visible window contains no bars (very
  rare in live mode; possible in fixtures mode for the very first
  minute), the chart renders the gridlines + an inline `FG_3` "No
  data" label centred. Never blank.
- **Symbol selector** — chip row at the top of the Charts screen,
  active chip uses the T1507 active-row pattern (2 px ACCENT left
  rule). The selected symbol persists across screen switches via
  `Cockpit::selected_symbol` so the operator can navigate away and
  back without losing context.

The chart is a **read-only surface**: no order entry, no draw
tools, no annotations. The cockpit's job is to show what the
agent did and what the market did; tools for "what if I drew
this trendline" belong in a research surface that this product
deliberately does not have (see [`product.md`](product.md)
non-goals).

## Information architecture

Patterns for how cockpit screens compose, switch, and persist state.
Locked at the post-Phase-1 roadmap revision (2026-05-04).

### Sidebar nav — fixed-width, text-labelled, T1507-styled

The cockpit's left rail is a fixed-width (~180 px) sidebar of
text-labelled nav entries. The selected entry uses the T1507
`active_row` pattern (2 px ACCENT left rule). Two reasons fixed-width
beats collapsible at this surface size:

- **Icons are still operator-locked out** (see § Iconography).
  Collapsible navs need icon glyphs to make sense in the collapsed
  state, so collapsibility forces icon adoption — a re-litigation
  this revision deliberately doesn't want.
- **Desk display surface is ample.** The operator's monitor is
  ≥ 1440 px wide; 180 px sidebar + screen body fits comfortably
  without crowding the trading view.

The sidebar's nav order is the operator's normal scan order: most
frequently used at top. Phase 2 ships **Home → Debug → Charts**.
Phase 3 inserts **Strategies → Risk → Audit** between Debug and
Charts (architect resolves the exact ordering at Phase 3 design).

### Screens are pure render dispatches, never side-effecting

`Cockpit::current_screen` is the only state that drives the screen
shown. `Message::SwitchScreen(Screen)` is a pure assignment — no
side effects, no async work, no mutation of any other field. The
shell's `view()` reads `current_screen` and dispatches to the
appropriate screen body's `view()` function.

Two corollaries:

- **No screen "loads" anything on entry.** Every screen renders
  whatever is currently in `Cockpit` for it. Data freshness is the
  bus's responsibility; the screen's switch is instantaneous.
- **Screens never mutate sibling-screen state.** The Audit screen
  does not, on switch, refresh anything for the Risk screen.

This keeps the screen model trivial to test (insta snapshot per
screen) and trivial to reason about ("what does this screen show?
exactly what's in `Cockpit` and the screen's `view()` function").

### Persistence: selected symbol, current screen

These two pieces of `Cockpit` state persist across screen switches:

- `Cockpit::current_screen` — last screen the operator was on.
- `Cockpit::selected_symbol` — which `(venue, symbol)` was last
  selected on the Charts screen.

Both are session-scoped (cleared on cockpit restart) — a deliberate
decision; the operator should not feel the cockpit "remembers"
across separate runs because the trading session is the natural
unit of state. Persisting across runs would surface a "did the
operator close on this screen, and is it still meaningful now?"
question we don't want to litigate per session.

### Navigation does not write to backend

Switching screens is **never** observed by the backend. The
sidebar nav writes `Message::SwitchScreen(Screen)`, the cockpit
mutates `current_screen`, the next `view()` renders the new screen
body. No bus event, no audit writer, no agent state change. This is
the same one-way contract that
[`audit::query`](architecture.md#cockpit--auditquery) reads have:
the cockpit sees, the cockpit doesn't tell.

### Right-rail Assistant slot — reserved, hidden by default

Phase 2's shell grid reserves a **right column-track** for the
Phase 6 Assistant slot. Reservation = the column exists in the
grid spec with zero width when the v2 LLM strategy is not
enabled. No widget renders in it; no token references it. When
v2 LLM ships, Phase 6 sets the track width and inserts the
assistant widget — no Phase 2-side change needed. (See master
roadmap Constraint 4.)

## Dark / light mode parity

The Lumen dual-palette in `crates/ui/src/theme.rs` is the single source of
truth for both dark and light mode color values. The old proposed-light-table
(previously at lines 97–110 of this document) is retired — those values now
live as pinned constants in `theme.rs` and are verified by
`t1501_palette_light_hex_pinned` in the `theme.rs` test suite.

Both modes are first-class, both maintained, both contrast-checked.

**Q6 — cold-start behavior:** The cockpit cold-starts in **dark mode**
(`ThemeMode::Dark` is the `#[default]`). Reasons:

- The cockpit session is long (six- to twelve-hour shifts).
- The operator is most often working in a dim room or at night
  (most strategies in v0/v0.5 are crypto, which is 24/7).
- The dark palette has been operator-tested for the v0 ship.

**Light mode is wired, not yet runtime-toggled.** Every `ModeColor`
constant carries both dark and light values. A future runtime toggle
calls `.current(mode)` with `ThemeMode::Light` and the entire palette
switches without any token rewrite. The `t1501_palette_light_hex_pinned`
test guarantees the light values are not stubs.

Both modes share strings, spacing, typography, and copy — only the color
constants change. No conditional `if dark { … } else { … }` blocks at
call sites — `ModeColor::current(mode)` returns the right color for the
active mode.

## Consistency enforcement

Re-stating the existing rules. These are already enforced by
`crates/ui/tests/consistency.rs`:

- **All colors in `ui::theme`.** Zero inline hex literals in widgets.
  Zero `Color::from_rgb(…)` calls outside `theme.rs`. The
  `no_inline_hex_colors_in_widgets_or_state` test fails the build on
  violation.
- **All strings in `ui::strings`.** Zero string literals in widgets.
  The `no_inline_user_visible_strings_in_widgets` test fails the build
  on violation.
- **All reusable widgets in `ui::widgets`.** Three-uses rule: the
  third copy is a refactor, not a copy-paste.
- **Spacing scale is closed.** The consistency-test allow-list is the
  new spacing scale (`0/2/4/6/8/12/16/20/24/32/40/48/64`) — zero
  exceptions.
- **Type scale is closed.** `MICRO / SMALL / BODY / H3 / H2 / H1 / DISPLAY`
  — zero eighth size.
- **Color tokens are semantic.** No `red`, `green`, `blue` — only
  `UP_500`, `DOWN_500`, `ACCENT`, etc.
- **`Message::*` is exhaustive.** No `_ => {}` catch-all in `update`.
  Adding a new message variant forces a compile-time review of every
  arm. This is also what makes the `Message` enum an honest public
  API — every variant is a contract.

Drift starts with "just one exception". The cockpit is small enough
to keep zero exceptions; we keep zero.

## What's NOT in scope

To prevent gold-plating, the following are explicitly out of scope:

- **Branding.** No logo. No marketing site. No "Trading Cockpit by
  Acme Corp" splash. The product is single-operator and self-hosted.
  (Master Constraint 1 — no brand adoption.)
- **Onboarding flows.** The operator IS the developer. No first-run
  wizard, no tour, no empty-state CTAs that explain what the agent
  does. The agent runs trading; the cockpit shows what it did.
- **Internationalization.** Single English locale until v3+. The
  `ui::strings` module is structured for future i18n (one constant
  per string, `all()` enumerates them) but no actual translation lands
  pre-v3. The locale-default thousands separator is the **system**
  default at runtime, not a UI-controlled choice.
- **Mobile / touch.** Cockpit is desktop-only — macOS / Linux native
  iced window. No tap targets sized for fingers, no responsive
  breakpoints, no PWA. The operator runs the cockpit at a desk.
- **In-cockpit configuration editing.** TOML files in `config/` are
  edited in `$EDITOR`, not in the cockpit. The cockpit is read-only
  on configuration; the kill switch is the **only** write surface
  (and it writes to `.halt`, not to a TOML).
- **In-cockpit chart editing / drawing tools.** Equity curves and
  bar charts are read-only. No trendlines, no annotations. Annotations
  are an analyst's job in markdown reports.
- **Voice rewrite.** The cockpit copy voice is operator-locked as-is
  (master Constraint 2). No Lumen voice adoption; no copy rewrite
  driven by a design system update.
- **Icon adoption.** Lucide icons are deferred until a text label
  fails the operator's scan-test. v0–v1.5 ships zero icons.

## Open questions

Questions deferred past Phase 1. Phase-1-resolved items (Q6 dark default,
Q7 single-file replace, dual-palette source of truth) have been closed and
are documented in this file or in `theme.rs`.

1. **Tabbed views or single-pane (Phase 2)?** The shipped cockpit is
   single-pane (one window, all panels visible). At v1+ multi-symbol-portfolio
   scale, do we (a) keep single-pane and let it scroll, (b) tab by
   strategy / symbol, or (c) multi-window iced? *Default answer: (a) until
   5+ active strategies, then re-evaluate.*

2. **Live-tape row click → audit modal (Phase 2, v1.5+)?** Promoted to
   backlog as `tape-row-audit-modal`.
   **Operator decision 2026-05-03: YES. Promoted to backlog for v1.5+ scope.**

3. **P&L card — sparkline (Phase 2)?** A 60-bar Unicode sparkline or a
   `plotters-iced` embedded mini-chart beneath the equity number.
   *Default answer: Unicode sparkline first.*

4. **Strategy panel — chart per strategy (Phase 3)?** Each strategy row
   could expand to show a 60-bar sparkline of that strategy's signals.
   *Default answer: defer until the operator has 3+ strategies running
   concurrently.*

5. **Persisted layout (Phase 3)?** Operator can drag panels to reorder.
   *Default answer: no — the layout is the operator's mental model. Any
   change is a deliberate ship.*

6. **Runtime theme toggle UI (Phase 4)?** The light palette is wired in
   `theme.rs`; the runtime toggle surface (button, keyboard shortcut,
   `config/cockpit-state.toml` persistence) is a Phase 4 task once the
   operator has expressed a preference for daytime sessions.

## Changelog

- 2026-05-02 (ui-designer): initial principles document. Locks
  aesthetic direction (Bloomberg-density + Linear-taste +
  Stripe-clarity), proposes three additive color tokens
  (`bg_overlay`, `info`, `border_strong`), pins type scale at four
  sizes, pins spacing scale at six values, locks density tables for
  cockpit (compact) and viewer (comfortable), pins motion timings,
  documents trading-specific patterns (P&L coloring, latency bands,
  kill-confirm phrase pattern, flash-on-update), and lists eight
  open questions for operator input.
- 2026-05-03 (operator decisions, vitaliy.schreibmann@senacor.com):
  resolved 4 of 8 open questions. Q1 (Cmd-K palette) deferred to v2+
  near project completion. Q3 (snapshot/share) NO — project is
  single-operator/private, no Slack scenario. Q4 (tape-row → audit
  modal) YES — promoted to backlog as `tape-row-audit-modal` for v1.5+.
  Q6 (sound alerts) NO — default confirmed. Q2/Q5/Q7/Q8 left at their
  default-answer state pending future revisit.
- 2026-05-04 (ui-designer, T1510): Lumen-anchored rewrite per Q7
  single-file replace. "Visual language" section fully rewritten —
  token tables replaced with Lumen palette (29 colors), tier model,
  shadow ladder, 13-step spacing scale, 6-step radii, 7-step type,
  motion tokens; all constants cite `theme.rs` names for grep-ability.
  R4.3 focus-ring rule documented. "Dark/light mode parity" rewritten
  — commits to `theme.rs` dual-palette as source of truth, documents
  Q6 dark default, drops the old proposed-light-table. Colour names
  updated throughout (`pos` → `UP_500`, `neg` → `DOWN_500`). Voice,
  consistency, not-in-scope, and open-questions sections updated with
  additive paragraphs/bullets per task brief.
- 2026-05-04 (analyst, post-Phase-1 roadmap revision): added two new
  top-level subsections capturing the post-Phase-1 IA + chart
  patterns: **"Charts — price plot with audit-anchored markers"**
  under Trading-specific patterns (background / gridline rules, line
  vs candle styling, buy/sell triangle markers from the audit ledger,
  read-only surface contract) and **"Information architecture"** as
  a new top-level section (sidebar fixed-width / text-labelled / T1507
  selected-entry; screens are pure render dispatches; persistence
  scope = current_screen + selected_symbol session-only; navigation
  never writes to backend; right-rail Assistant slot reservation for
  Phase 6). Locks the design rules every Phase 2 / 3 / 4 / 5 / 6
  widget plugs into; per-phase R-items live in the per-phase briefs
  at [features/lumen-phase-2-shell-ia-charts.md](features/lumen-phase-2-shell-ia-charts.md)
  through
  [features/lumen-phase-6-assistant-slot.md](features/lumen-phase-6-assistant-slot.md).
