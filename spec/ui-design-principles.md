---
slug: ui-design-principles
status: living
owner: ui-designer
updated: 2026-05-02
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
  liars by construction. See `Motion` below.
- Not dark-mode-only by default of laziness. Both modes are first-class
  (see `Dark / light mode parity`).

## Visual language

### Color palette

The shipped palette is 9 semantic tokens (in `crates/ui/src/theme.rs`).
This document **extends** the set with three additions that already
exist *implicitly* (we get away without them today by reusing `BORDER`
and never touching `info`-style state) but will hurt at v1.5+ as we add
funding-rate observation badges, PR-style "did you mean" hints, and
elevated panel layering for modal overlays.

#### Dark mode (default)

| Token            | Hex (dark)  | Role                                                  | Status      |
|------------------|-------------|-------------------------------------------------------|-------------|
| `bg`             | `#11141A`   | Cockpit canvas background                             | shipped     |
| `bg_elev`        | `#1A1F29`   | Cards, panels, raised surfaces                        | shipped     |
| `bg_overlay`     | `#0B0D12`   | Modal-dialog backdrop (kill-confirm, future Cmd-K)    | **propose** |
| `fg`             | `#E8ECF2`   | Primary text                                          | shipped     |
| `fg_muted`       | `#8B93A3`   | Labels, captions, secondary text                      | shipped     |
| `accent`         | `#5EA3FF`   | Primary interactive (links, active tab)               | shipped     |
| `pos`            | `#3ECF8E`   | Gains, buys, healthy                                  | shipped     |
| `neg`            | `#FF6B6B`   | Losses, sells, danger, halted                         | shipped     |
| `warn`           | `#FFC45A`   | Slow latency, soft warnings, recoverable error        | shipped     |
| `info`           | `#7BC2FF`   | Observation-only signals (e.g. funding-rate badge)    | **propose** |
| `border`         | `#2A313F`   | Panel outlines, separators (shipped as `BORDER`)      | shipped     |
| `border_strong`  | `#3A4456`   | Focused / hovered border, modal frame                 | **propose** |

#### Light mode (parity, not default)

| Token            | Hex (light) | Role                                                  | Status      |
|------------------|-------------|-------------------------------------------------------|-------------|
| `bg`             | `#FAFBFC`   | Cockpit canvas background                             | propose     |
| `bg_elev`        | `#FFFFFF`   | Cards, panels                                         | propose     |
| `bg_overlay`     | `#0B0D12CC` | Modal backdrop (80% opacity onto light bg)            | propose     |
| `fg`             | `#0F1217`   | Primary text                                          | propose     |
| `fg_muted`       | `#5B6473`   | Labels, captions                                      | propose     |
| `accent`         | `#1F6FE5`   | Primary interactive                                   | propose     |
| `pos`            | `#0E9F6E`   | Gains                                                 | propose     |
| `neg`            | `#D14343`   | Losses, danger                                        | propose     |
| `warn`           | `#B4751B`   | Soft warnings                                         | propose     |
| `info`           | `#1F6FE5`   | Observation-only (same as accent in light)            | propose     |
| `border`         | `#E2E6EC`   | Panel outlines                                        | propose     |
| `border_strong`  | `#C9D0DA`   | Focused / hovered                                     | propose     |

> Light mode is a future task — the shipped cockpit is dark-only. The hex
> values above are the design contract; first implementation extends
> `theme::color` with a runtime mode switch and a parallel `light::*` const
> block. This document fixes the values so a developer can land that change
> mechanically without re-litigation.

#### Contrast contract (WCAG AA, 4.5:1 minimum)

Every text-on-surface pairing in the cockpit must clear WCAG AA for body
text (4.5:1) and AAA for the equity display (7:1). The shipped pairings
were hand-checked; the proposed extensions follow the same rule:

| Pairing                          | Dark ratio | Light ratio | Required |
|----------------------------------|------------|-------------|----------|
| `fg` on `bg`                     | 13.7:1     | 16.1:1      | AAA 7:1  |
| `fg_muted` on `bg`               | 5.0:1      | 5.7:1       | AA 4.5:1 |
| `fg` on `bg_elev`                | 11.2:1     | 15.5:1      | AAA 7:1  |
| `pos` / `neg` / `warn` on `bg`   | ≥ 4.6:1    | ≥ 4.5:1     | AA 4.5:1 |
| `accent` on `bg`                 | 5.9:1      | 5.2:1       | AA 4.5:1 |

Color-pair changes ship with a contrast-table update. A future
`tests/contrast.rs` will computationally enforce this against the same
table; until then the rule is on the design reviewer.

### Typography

Four sizes total. We refuse a fifth.

| Token     | Size | Line-height | Weight | Use                                            |
|-----------|------|-------------|--------|------------------------------------------------|
| `caption` | 11px | 16px        | 400    | Column headers, axis labels, helper text       |
| `body`    | 13px | 20px        | 400    | Cell content, paragraph copy, button labels    |
| `title`   | 16px | 24px        | 600    | Panel titles, card headings, dialog titles     |
| `display` | 22px | 32px        | 600    | Equity number, halted banner, kill-switch label |

**Font choice:**

- **Sans:** the platform default (`-apple-system`, `Inter`, `Segoe UI`).
  iced's default font already routes here on macOS / Linux. We do not
  bundle a custom font — every kilobyte of ttf is a kilobyte not spent
  on faster bar rendering.
- **Mono (digits):** **non-negotiable.** Numbers in the tape, positions,
  P&L card, and any future leaderboard use a monospaced font (`SF Mono`
  on macOS, `JetBrains Mono` if bundled, falling back to the platform
  monospace). Monospaced digits in a proportional font (tabular-nums)
  are an acceptable substitute when shipping a custom font is too heavy.
- **Italic:** never, except verbatim error messages from upstream
  (`error_summary` from a `StrategyLoadError`). Italics imply emphasis
  the cockpit doesn't need.

**Number formatting** (all rendered through `widgets::num`, which is the
single place this rule lives):

- Right-aligned in tabular contexts.
- Thousands separator: locale-default (` ` for de-DE, `,` for en-US).
  We default to en-US until v3+ i18n.
- Decimal precision per type: prices to symbol minor tick (e.g. 2 dp
  for USDT pairs), quantities to 6 dp, P&L to 2 dp, percentages to 2 dp.
- Sign always shown for deltas (`+12.34` / `-5.67`), never for absolute
  values (`52,341.20`, not `+52,341.20`).
- Color-of-sign **only** on signed deltas, not on absolute balances.
  See `color_for_delta` in `theme.rs`.

### Spacing scale

The shipped scale `4 / 8 / 12 / 16 / 24 / 32` (`xs / s / m / l / xl / xxl`)
is correct and stays as the closed set. Justification:

- Six values is dense enough for a 12-pixel grid (the cockpit is a
  desktop application; we don't need 4-pixel-grid precision).
- The geometric-ish jump (4→8→12→16→24→32) gives sufficient discrimination
  at every level — no two adjacent values are perceptually close.
- Powers of 2 *plus* multiples of 4 cover both Material-style 8-grid
  and 12-column row layouts without ad-hoc values.

If a feature thinks it needs `10` or `20`, the answer is "use `12` or
`16` and a different font size". This is the most violated rule on
casual feature work, and the most worth defending.

### Density

Two density modes — **compact** for the live cockpit (operator scans at
a glance) and **comfortable** for the offline viewer (operator reads
reports). Pinned metrics:

| Metric                   | Compact (cockpit) | Comfortable (viewer) |
|--------------------------|-------------------|----------------------|
| Table row height         | 24 px             | 32 px                |
| Table cell horizontal pad| 12 px             | 16 px                |
| Panel inner pad          | 16 px             | 24 px                |
| Panel outer gap          | 16 px             | 24 px                |
| Card title → body gap    | 12 px             | 16 px                |
| Dialog inner pad         | 24 px             | 24 px                |

The shipped cockpit already uses these via `theme::layout::*`. The viewer
sees less hourly scrutiny but has higher per-glance reading load — a
backtest report is a 30-second read, not a 30-millisecond read.

### Motion

Trading UIs must look **still when nothing is happening**. The agent is
not Slack. Surprise motion costs operator attention and trains them to
ignore real updates.

| Element                        | Duration | Easing       |
|--------------------------------|----------|--------------|
| Tooltip / hover surface        | 60 ms    | linear       |
| Button press feedback          | 80 ms    | linear       |
| Panel state transition         | 150 ms   | ease-out     |
| Modal open / close             | 180 ms   | ease-out     |
| Number flash on update         | 200 ms   | ease-out fade|
| Halted banner appear           | 150 ms   | ease-out     |

Hard rules:

- **No auto-advancing carousels.** The operator scrolls, not the UI.
- **No parallax.** It only exists on marketing sites.
- **No idle animation.** A pulsing dot for "live" is fine; a pulsing
  card border is not.
- **No 60 fps animations on charts.** The equity curve renders once per
  bar close (1s in v0, 1m at v0.5+); requestAnimationFrame is for games.
- **Never animate the kill switch.** Confirm dialogs open immediately
  (no slide-in, no fade); operators clicking that button should not be
  delayed by a single ms.

### Iconography

**Position: no icons until needed.** v0–v1.5 ships with **zero icons**.
Every interactive element is labeled with text. A button reading "Stop
trading" beats a square with a stop-sign glyph for an operator at
hour ten.

When a v2+ feature legitimately needs an icon (e.g. a per-row "view
audit trail" affordance in the tape, where text labels would balloon
the row), the rule is:

- **Style:** Lucide / Feather — line icons, 1.5 px stroke.
- **Sizes:** 16 / 20 / 24 px on a 24-px grid. No 18 or 22.
- **Color:** `fg_muted` default; `accent` on hover; never colored.
- **Pairing:** every icon has a `tooltip` constant in `ui::strings`. No
  text-free buttons. Screen-reader labels are mandatory.

The justification for "no icons" is empirical: the shipped cockpit has
zero icons and is readable. Adding them is a one-way door (the operator
learns the glyph, then can't un-learn it); deferring is free.

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

## Trading-specific patterns

Patterns that exist because this is a trading UI, not a generic web
dashboard.

### P&L coloring

- **Positive** P&L → `pos`.
- **Negative** P&L → `neg`.
- **Zero** P&L → `fg_muted` (NOT `fg`, NOT a third "neutral" color).

The reason zero is muted, not foreground: when the operator scans a
column of P&L, the zeroes should fade so positives and negatives jump
out. A column of all-zeroes (no movement on a quiet bar) should look
like background, not data.

This is implemented in `theme::color_for_delta` and is the only legal
source for delta colors. Widgets calling it inline rather than picking
their own `pos` / `neg` per-row is enforced by the consistency tests.

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
| `< 500 ms`     | OK    | `pos`      | "OK"     |
| `< 2 s`        | WARN  | `warn`     | "Slow"   |
| `< 10 s`       | HIGH  | `neg`      | "High"   |
| `≥ 10 s`       | HALTED| `neg` + banner | "Halted" |

Three-band thresholds are deliberate — two bands (OK / not-OK) loses
the operator's grace period; four bands (OK / Slow / High / Halted)
gives a "things are getting worse" signal before the kill threshold.
The same color is used for High and Halted by design — once the
operator sees `neg`, they look at the label, not the color.

### Kill-switch confirmations

Already covered in `Confirm destructive actions`. Re-affirming the
non-negotiable: the kill button is a sealed, typed-confirm flow with
the safety phrase `HALT BTC`. The exact phrase is not negotiable
mid-session — changing it during operations is a recipe for the
operator typing the previous phrase under stress.

### Numbers that change frequently — flash on update

P&L, last price, equity, latency: subtle 200 ms fade from `accent` (or
`bg_elev` in the case of equity) back to the resting color when the
value updates. The flash is **subtle** — it is barely perceptible at
the corner of the eye, and invisible to the operator who is staring
straight at it. It is the "the number changed and you didn't miss
it" backup channel.

Hard rule: the flash never **flickers**. A new value that arrives 50 ms
after the previous flash queues, it does not interrupt. A value that
hasn't changed numerically (same float to same float) does not flash.

This is a v1+ task; v0 ships static rendering. The principle is locked
so the implementation lands without re-litigation.

## Dark / light mode parity

Both modes are first-class, both maintained, both contrast-checked.
The default is **dark**:

- The cockpit session is long (six- to twelve-hour shifts).
- The operator is most often working in a dim room or at night
  (most strategies in v0/v0.5 are crypto, which is 24/7).
- The dark palette has been operator-tested for the v0 ship.

Light mode is for screenshot-friendliness in presentations and for the
small fraction of the operator's sessions in bright daylight. Both
modes share strings, spacing, typography, and copy — only the color
constants change.

Implementation contract: a single `ThemeMode { Dark, Light }` enum is
threaded through the cockpit's `Cockpit` model; all `theme::color::*`
becomes `theme::color::accent(mode)` etc. No conditional `if dark { … }
else { … }` blocks at call sites — the function returns the right
color for the active mode.

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
- **Spacing scale is closed.** `4 / 8 / 12 / 16 / 24 / 32` — zero
  exceptions.
- **Type scale is closed.** `caption / body / title / display` — zero
  fifth size.
- **Color tokens are semantic.** No `red`, `green`, `blue` — only
  `pos`, `neg`, `accent`, etc.
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

## Open questions

Real questions that need the operator's call before the next UI
feature lands. Each is one-sentence-answerable.

1. **Global command palette (`Cmd-K`)?** Linear-style fuzzy command
   bar — "halt agent", "go to strategies", "view audit for tx_id 4f9…".
   Cheap to add, high-impact for keyboard-first operators. *Default
   answer if no input: yes, ship in v2.*
   **Operator decision 2026-05-03 (vitaliy.schreibmann@senacor.com):
   defer to v2-or-later, near project completion. Not a near-term
   feature.**

2. **Tabbed views or single-pane?** The shipped cockpit is single-pane
   (one window, all panels visible). At v1+ multi-symbol-portfolio
   scale, do we (a) keep single-pane and let it scroll, (b) tab by
   strategy / symbol, or (c) multi-window iced (which we already
   support)? *Default answer: (a) until 5+ active strategies, then re-evaluate.*

3. **Snapshot / share button?** A "copy a screenshot of this panel
   to the clipboard" affordance for sharing in operator-team Slack.
   Trivially implementable on macOS via `screencapture`. *Default
   answer: yes for the P&L card and the strategies panel; no for the
   tape (privacy).*
   **Operator decision 2026-05-03: NO. Project is single-operator,
   private on the operator's local machine — no Slack/share scenario
   exists. The macOS-native `Cmd-Shift-4` is sufficient when the
   operator wants a screenshot.**

4. **Live-tape row click → audit modal?** Currently the tape is
   read-only. Should clicking a row open a modal with the full
   `journal_transaction` (debits, credits, transaction_id, source
   strategy)? *Default answer: yes, ship in v1.5+.*
   **Operator decision 2026-05-03: YES. Promoted to backlog as
   `tape-row-audit-modal` for v1.5+ scope.**

5. **P&L card — sparkline?** A 60-bar sparkline of equity tucked
   beneath the equity number, Unicode `▁▂▃▄▅▆▇█` style (already in
   use in success reports per architecture v1+ Q4). Or a `plotters-iced`
   embedded mini-chart? *Default answer: Unicode sparkline first,
   `plotters-iced` only if the operator says it's hard to read.*

6. **Sound alerts?** A short tone when the agent halts, when a fill
   exceeds N USDT, or when latency crosses HALTED. *Default answer:
   no — the operator's auditory environment varies (headphones /
   open-office / overnight). Visual + macOS Notification Center is
   enough; sound can be added per-operator as a config flag.*
   **Operator decision 2026-05-03: NO. Confirmed default. Visual +
   macOS Notification Center suffices.**

7. **Strategy panel — chart per strategy?** Each strategy row could
   expand to show a 60-bar sparkline of that strategy's signals.
   Could clutter; could illuminate. *Default answer: defer; revisit
   when the operator has 3+ strategies running concurrently.*

8. **Persisted layout?** Operator can drag panels to reorder (left-
   column / right-column / which order). *Default answer: no —
   the layout is the operator's mental model. If we change it,
   we change it for everyone in a deliberate ship.*

The operator's one-line answer to any of these unblocks an architect
brief. None of them are blocking the current cockpit's correctness.

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
