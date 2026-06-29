---
slug: advisor-overfitting-scorecard
status: dev-done
owner: ui-designer
version: 0.1.0
updated: 2026-06-29
---

# Overfitting Scorecard (P0-1) — the credibility layer

The v2 #1 feature (surfaced #1 in 6 of 9 research topics): a **report-only**
overfitting scorecard surfaced next to every bake-off recommendation, answering
"did we fool ourselves by trying many strategies?" Closed-form
**N_eff → Deflated-Sharpe → MinBTL** (PBO deferred to the Tune surface). **Additive
to the FROZEN gate — never a veto.** This is the literal "traceable & plausible"
product thesis.

**Design + ratified decisions:** [`v2-architecture.md`](../v2-architecture.md) §1 P0-1
+ §3 (the `Scorecard` design sketch) + **§6.0** (operator-ratified: report-only;
closed-form N_eff frozen at the 24-config scale; no PBO / threshold / crown-veto in
v2). Analyst framing: [`v2-analysis.md`](../v2-analysis.md) §2. Research:
[`research/backtesting/application-overfitting-and-multiple-testing.md`](../../../research/backtesting/application-overfitting-and-multiple-testing.md)
§6 + [`research/evolution/application-anti-overfitting-and-search-discipline.md`](../../../research/evolution/application-anti-overfitting-and-search-discipline.md) §6.

## What shipped (this dev increment)

- `crates/backtest/src/bakeoff/scorecard.rs` — a pure module: the `Scorecard` struct +
  `n_eff` / `min_btl` / `dsr` closed forms, `normal_cdf` + a high-accuracy
  `normal_inv_cdf` (Acklam rational + one Halley refinement step), skew/kurtosis,
  `sharpe_variance`, `compute_scorecard`.
- `Recommendation.scorecard` carrier (`bakeoff/mod.rs`), computed in `run_bakeoff`
  from inputs that already exist (per-candidate Sharpe vector + the crown's bootstrap
  `DistributionSummary` + the crown's return skew/kurtosis). **Report-only** — logged
  + carried; never fed into crown/rank/verdict.

## What shipped (the UI increment — ui-designer)

- `ui`-owned `ScorecardView` (plain `usize`/`f64`/`bool` — NO `backtest::Scorecard`
  in widgets), mirrored from `Recommendation.scorecard` inside the single
  `BakeoffReportMirror::from_report` boundary (`leaderboard/state.rs`). `pbo` is
  omitted (always `None` in v2). Degenerate (`n_candidates == 0`) → `None` so the
  block paints nothing rather than an all-zero readout.
- The leaderboard **"How much to trust this"** block (`screens/leaderboard.rs`,
  `scorecard_block`), rendered between the recommendation and the ranked table when
  the report carries a scorecard. Four plain-language facts + the report-only note.
- Copy in `crate::strings` (13 new `LEADERBOARD_SCORECARD_*` constants). **Zero new
  theme tokens, zero new widgets** (reuses `frame::panel` + existing tokens).
- **Zero new `ui` dep edge** — `ScorecardView` crosses as plain scalars over the
  pre-existing `backtest` mirror seam.

## Not in this increment (per §6.0)

- **PBO/CSCV** — deferred to the Tune/sweep surface where CSCV is statistically honest (D1).
- Any **DSR/PBO crown-veto** — report-only (D3); `Scorecard.crown_clears_dsr` is an
  informational flag, one-line-switch-ready for a future veto + its own ADR.

ADR-0075 (reserved — written/registered atomically when the increment lands).

## UI

### Wireframe (leaderboard `Ready` pane, scrollable, top-to-bottom)

```text
┌─ Recommendation ────────────────────────────────────────────────┐
│ SMA crossover looks best on BTCUSDT.                             │
│   It held up under resampling.                                   │
│   · Highest Sharpe among the strategies that held up…           │
│   · Beat buy-and-hold on risk-adjusted return.   [ Explain… ]   │
└──────────────────────────────────────────────────────────────────┘
┌─ #  Strategy            Return   Sharpe   Max DD   Trades ───────┐
│ 1  SMA crossover ★ best  +18.37%  1.4200   -6.12%   38           │
│ …                                                                │
└──────────────────────────────────────────────────────────────────┘
┌─ How much to trust this ────────────────────────────────────────┐   ← NEW (P0-1)
│ An honesty check on the search behind the pick — it never        │
│ changes the result.                                              │
│                                                                  │
│ STRATEGIES TRIED                                                 │
│ 13 — about 8 truly independent                                   │
│                                                                  │
│ DEFLATED CONFIDENCE                                              │
│ 61%                                                              │
│ Chance the edge is real after accounting for how many we tried.  │
│                                                                  │
│ MINIMUM HISTORY NEEDED                                          │
│ about 6.4 years of data                                          │
│ Trust the result only with at least this much history behind it. │
│                                                                  │
│ BEATS HOLDING AFTER THE SEARCH?                                 │
│ ✗ Not clearly — holding is the honest call                      │
│ Informational, not a gate — this never changes the pick above.   │
└──────────────────────────────────────────────────────────────────┘
  Not financial advice. Results are simulated…  (persistent)
```

**Placement:** directly UNDER the ranked table (not between the recommendation
and the table). Reads as "here's the ranked field + crowned pick, and here's how
much to trust the winner". This keeps the ranked rows at their established
position in the result pane so the load-bearing populated-render guards
(`leaderboard_populated_render.rs`, which measure the table band in a fixed
1080px viewport) are unaffected by the new block — the scorecard adds height
below the table rather than displacing them out of frame.

### New screens / panels / widgets

- **Panel:** the "How much to trust this" scorecard block — a `frame::panel`
  (reused, no new widget) titled from `LEADERBOARD_SCORECARD_TITLE`, holding a
  caption + four `scorecard_fact` label/value/hint stacks. Rendered only when
  `BakeoffReportMirror.scorecard.is_some()`. Display-only; no interactivity.
- **No new screen, no new top-level widget, no new `Message` variant** (the block
  is pure read-only display off existing `Ready` state).

### New strings added to `ui::strings` (13)

`LEADERBOARD_SCORECARD_TITLE`, `_CAPTION`, `_TRIED_LABEL`,
`_TRIED_EFFECTIVE_FMT`, `_CONFIDENCE_LABEL`, `_CONFIDENCE_HINT`,
`_HISTORY_LABEL`, `_HISTORY_FMT`, `_HISTORY_HINT`, `_BEATS_HOLD_LABEL`,
`_BEATS_HOLD_YES`, `_BEATS_HOLD_NO`, `_INFORMATIONAL_NOTE`. All registered in
`strings::all()`.

### New theme tokens

**None.** The block composes from existing `color::{FG_1,FG_3}`, `text::{H3,SMALL,
MICRO}`, and `space::{XXS,M}` — the near-zero-new-token target is met.

### Human-friendliness / accessibility notes

- **No jargon undefined:** the two terms of art (DSR → "Deflated confidence";
  MinBTL → "Minimum history needed") each carry a one-line plain-language gloss
  directly beneath the figure.
- **Honesty framing, not a verdict:** title + caption frame it as "an honesty
  check on the search"; the "Beats holding?" row carries the load-bearing
  "Informational, not a gate — this never changes the pick above" note. When
  buy-and-hold is crowned (the modal case) the "✗ Not clearly — holding is the
  honest call" value reads as the expected, fine answer, not a failure.
- **Colour is never the only signal:** the yes/no carries a ✓/✗ glyph; the value
  is muted `FG_1` (NOT pos/neg sentiment) so a "no" never reads as a red loss.
- **Both themes:** the block uses only `ModeColor` tokens via `.current(mode)`, so
  it renders correctly under `--theme dark` and `--theme light`. (Render guard is
  macOS-canonical Dark per ADR-0057 D2; light parity is by token construction.)
- **Render-verified at the pixel layer:** `crates/ui/tests/leaderboard_scorecard_render.rs`
  (populated block paints strictly more foreground than the same screen with the
  scorecard removed; the modal `BenchmarkWins` case still paints the block).

## Changelog

- 2026-06-29 (ui-designer): shipped the leaderboard "How much to trust this"
  scorecard block + `ScorecardView` mirror + 13 strings; zero new theme tokens,
  zero new `ui` dep edge. Render-verified (`leaderboard_scorecard_render.rs`).
