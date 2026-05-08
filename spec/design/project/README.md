# Lumen Design System

> Calm interfaces for AI-driven trading. Built for Rust-powered desktop apps.

Lumen is a brand and design system for a trading product where **the work is already stressful — the interface should not be**. The visual language is restrained, structured, and quietly technical, with one warm accent and a single playful gesture (the eye/lens mark) so it never tips into clinical.

This system is the source of truth for colors, type, spacing, motion, iconography, components, and screen patterns across Lumen's products.

---

## Brand at a glance

| | |
|---|---|
| **Name** | Lumen |
| **What it is** | AI-driven trading platform |
| **Primary surface** | Native desktop apps (Rust + Tauri-class) |
| **Audience** | Active and pro traders who live in the app for hours |
| **Tone** | Calm, precise, unhurried. A senior colleague, not a hype account. |
| **Logo metaphor** | An eye / lens — AI seeing the market clearly |
| **Type** | Inter (UI) + JetBrains Mono (numerics, tabular) |
| **Accent** | Muted teal `#3F968D` — confident, not loud |
| **Themes** | Light and dark, equal weight |

---

## Sources

This system was generated from a brief, with no codebase, Figma, or reference materials provided. All design decisions in this repo are first-party — when the real product exists, replace the relevant assets and rev the README.

If/when sources arrive, list them here:
- Codebase: _none yet_
- Figma: _none yet_
- Marketing: _none yet_

---

## Content fundamentals

**Voice.** Calm and competent. We write the way a good senior trader talks — short sentences, no hedging, no hype. We never use exclamation marks. We never use "blazing fast", "revolutionary", "game-changing", or any superlative an investor deck would use. The product does the work; the copy steps out of the way.

**Person.** "You" for the user. "We" only when speaking as the company in marketing copy; never inside the app. The AI assistant is referred to as **Lumen** or **the assistant** — never "I", never anthropomorphized.

**Casing.**
- Sentence case for everything: buttons, menus, headers, dialogs.
- Title Case is reserved for proper nouns (product names, ticker symbols, exchange names).
- ALL CAPS only for micro-labels (column headers, timestamps, tags), with `letter-spacing: 0.06em`.

**Numerics.** Always tabular figures. Always with a sign for deltas (`+1.24%`, `−$420.50`). Use the minus sign `−` (U+2212), not a hyphen. Currency with the symbol leading and no space (`$1,240.00`, `€980.00`). Percentages always two decimals.

**Voice examples**

| Don't | Do |
|---|---|
| 🚀 You're crushing it today! | Up 1.24% today. |
| Whoa! That order is huge. | This order exceeds your daily limit. |
| Oops, something went wrong! | Order rejected — insufficient buying power. |
| Welcome back, trader! 👋 | Markets opened 14 minutes ago. |
| AI Insights: Buy NVDA NOW!!! | Lumen sees momentum building in NVDA. Review the signal. |

**Emoji.** Not used. Anywhere.

**Punctuation.** Em-dashes, en-dashes, real ellipses (`…`). No double-spaces. Oxford comma.

**Numbers and letters that look alike.** Inter is loaded with stylistic set 01 and a slashed zero on, so `0` and `O`, `1` and `l` are unambiguous in tables.

---

## Visual foundations

### Color
A **warm-paper light mode** and a **cool-deep dark mode**, joined by one teal accent. Up/down semantics are sage and clay — calmer than the standard neon green/red and easier on the eyes during a long session. Full token list lives in `colors_and_type.css`.

### Type
Inter Display for everything UI. JetBrains Mono for every number that participates in a calculation, every ticker, every timestamp, every order ID. Numerics are **always tabular**. Body sizes are **px-based**, not rem — this is a desktop app at fixed zoom.

### Spacing & rhythm
4-px base. Components hit a 4 / 8 / 12 / 16 grid. Trading layouts are **dense**: 6–8 px vertical padding inside table rows is normal. Breathing room belongs in the marketing site, not the desk.

### Backgrounds
Flat, never gradient. In light mode the canvas is a faint warm cream (`--warm-50`); in dark mode it's a cool near-navy (`--cool-800`). No textures, no patterns, no full-bleed photography in product. Marketing may use one accent gradient and quiet abstract photography, never both.

### Surfaces & elevation (the panel system)
Panels are how Lumen organizes the screen, and they are intentionally easy to tell apart:

- **Tier 0 — Canvas.** App background. No shadow.
- **Tier 1 — Panel.** Tint shift from canvas (lighter in light mode, lighter in dark mode) + 1 px hairline border (`--border-1`) + `--shadow-1` (whisper). This is the default container.
- **Tier 2 — Raised.** Dialogs, popovers, command palette. Same border + `--shadow-2`.
- **Tier 3 — Modal.** Full-screen overlay + `--shadow-3` + `--overlay` scrim.
- **Sunken.** Inputs and table stripes use `--panel-sunken` + `--shadow-inset` — the opposite direction, to read as "data goes in here."

Cards-with-elevation are **still modern in 2025**, but the modern read is *whisper shadow + 1 px border*, not iOS-card lift. Lumen's Tier 1 is exactly this.

### Borders
Hairline 1 px is the default. 2 px borders only on the focus ring and on the active state of segmented controls. Border colors come from the neutral scale, never from the accent.

### Shadows
Three soft levels (see tokens). Shadows in dark mode are darker, not bigger — the goal is depth without bloom.

### Corner radii
- 2 px on dense table inputs
- 4 px default
- 6 px buttons and chips
- 8 px cards and panels
- 12 px modals
- Pills (`999px`) only on tags and toggle thumbs

No fully circular elements except avatars and the toggle thumb.

### Motion
Short and soft. Hover and focus transition in 140 ms. Panel reveals in 220 ms. Modals enter in 320 ms with `cubic-bezier(0.22, 0.61, 0.36, 1)`. **No bounces. No spring physics.** A trading UI must never feel kinetic.

### Hover & press
- **Hover:** background tint shifts toward `--panel-raised` (light) / `--panel-raised` (dark). Text doesn't move.
- **Press:** background tint shifts one step further; the element does **not** scale or translate.
- **Active row in a table:** thin 2 px left rule in `--accent`, no fill change.

### Focus
Always visible. 3 px outer ring at low alpha of the accent (`--focus-ring`). Never `outline: none` without a replacement.

### Transparency & blur
Used sparingly. The command palette and dropdown popovers may use a 12 px backdrop blur over a 70 % opaque panel. Tooltips, badges, table rows are opaque.

### Imagery (when used at all)
Cool-toned, low-saturation, with light grain. Never stock-photo grins. The product itself rarely shows imagery; marketing carries it.

### Layout rules
- Fixed app shell: title bar, side rail, status bar.
- Resizable panel grid in the workspace.
- Status bar is **always visible** and shows: connection, latency, account, server time.
- Right-side AI assistant panel is **opt-in**, collapsible, and remembers state.

---

## Iconography

Lumen uses **Lucide** as its icon system — geometric, 1.5 px stroke, rounded joins. It pairs naturally with Inter and stays calm at small sizes.

- Default size: **16 px** in dense UI, **20 px** in toolbars, **24 px** in empty states.
- Stroke: **1.5 px**. Never filled (except where Lucide ships a filled variant for a specific semantic, e.g., the accent star on a watchlisted symbol).
- Color: inherits `--fg-2` by default, `--fg-1` on hover, `--accent` only when an icon represents an active/selected state.
- Loaded via the `lucide` CDN in HTML files; the SVG sources are in `assets/icons/` for offline use.

**No emoji.** Emoji are tonally wrong for a calm, professional trading app and render inconsistently across platforms.

**Custom marks** (logo, monogram, lens glyph) live in `assets/brand/` as SVG.

---

## What's in this repo

```
/
├── README.md                  ← this file
├── SKILL.md                   ← agent skill manifest (cross-compatible with Claude Code)
├── colors_and_type.css        ← all design tokens
├── fonts/                     ← @font-face files (currently CDN-loaded; see SKILL.md)
├── assets/
│   ├── brand/                 ← logo, monogram, lens glyph (SVG)
│   └── icons/                 ← Lucide subset for offline use
├── preview/                   ← cards rendered in the Design System tab
│   ├── 01-logo.html
│   ├── 02-type-display.html
│   ├── ...
└── ui_kits/
    └── desktop/               ← Lumen Trading Desktop UI kit
        ├── README.md
        ├── index.html
        ├── *.jsx
```

### Design System tab

Every preview card under `/preview/` is registered into the project's Design System tab, grouped by **Brand**, **Type**, **Colors**, **Spacing**, and **Components**. Use it as a contact-sheet view of the system.

### UI kits

- **`ui_kits/desktop/`** — a Rust-flavored desktop trading app: title bar, side rail, multi-panel workspace (watchlist, chart placeholder, order book, order ticket, AI assistant), status bar. Light and dark.

---

## Caveats and where this needs work

- The **logo** is first-party. When you have a real mark, drop it into `assets/brand/` and rev `01-logo.html`.
- Fonts are **CDN-loaded** from Google Fonts (Inter, JetBrains Mono). For offline / Tauri builds, drop TTFs into `/fonts` and switch the CSS to `@font-face`.
- **Lucide icons** are loaded from CDN; a small offline subset can be added if/when needed.
- The desktop UI kit is **cosmetic** — interactions are mocked, not wired to any data. It's a fidelity reference, not production code.

---

## How to use this

If you're an agent designing for Lumen, read `SKILL.md`. Then `colors_and_type.css`. Then the relevant UI kit. Copy assets out — never link cross-project.

---

## Index

| File | Purpose |
|---|---|
| `README.md` | This document — brand book |
| `SKILL.md` | Agent skill manifest |
| `colors_and_type.css` | All design tokens (light + dark) |
| `assets/brand/lumen-mark.svg` | Primary mark — eye/lens |
| `assets/brand/lumen-wordmark.svg` | Mark + "lumen" wordmark lockup |
| `assets/brand/lumen-monogram.svg` | "l" monogram inside the lens |
| `assets/brand/lumen-ai-lens.svg` | AI-flavored variant (3 satellite dots) |
| `preview/01-logo.html` … `preview/20-ai-card.html` | Design System tab cards |
| `ui_kits/desktop/index.html` | Full desktop trading app demo |
| `ui_kits/desktop/*.jsx` | Per-panel components |
