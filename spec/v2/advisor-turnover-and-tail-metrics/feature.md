---
slug: advisor-turnover-and-tail-metrics
status: tester-done
owner: tester
version: 0.2.0
updated: 2026-06-29
---

# Turnover KPI + Coherent Tail / Median Reporting (P1-1 + P1-2)

Two report-honesty increments shipped together because they share the same
additive code path and are near-free reductions over data already captured.

**Design:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-1 + P1-2.
**Research:** `research/risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1;
`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P2-D;
`research/backtesting/application-cost-and-impact-modeling.md` §6 A.

---

## P1-1 — Turnover formula (chosen and documented here)

**Formula:** `turnover = Σ(price × qty) / mean_equity`

That is, the sum of absolute trade notional (fill price × fill quantity, in
USDT) divided by the time-average equity over the backtest window.  The result
is a unitless ratio: "how many times did the strategy churn its capital?"

- A `turnover` of 1.0 means the strategy transacted its entire equity once.
- A `turnover` of 0.0 means no trades were executed (idle / buy-and-hold).
- A `turnover` of 10.0 means ten capital-equivalents of volume.

**Why this formula:** it maps directly to what the operator already has in
`RunReport.fills` (each fill has `price: Price` and `qty: Quantity`, both
`Decimal`-backed) and `report.equity_series` (from which mean equity is the
arithmetic mean of the equity values).  No new capture, no new engine fields.
The result is comparable across strategies with different position sizes because
it is normalised by mean equity — a strategy that holds a $1 000 position
transacting $500 notional scores 0.5, same as a $10 000 strategy transacting
$5 000.

**What it is NOT:** it is not a round-trip count (that's `trade_count / 2`); it
is not a per-day rate (annualising is the UI-designer's call); it is not P&L.

**Type:** `pub turnover: Decimal` on `CandidateKpis` — Decimal, consistent with
the existing `total_return_pct` / `max_drawdown` money fields.

---

## P1-2 — CVaR not VaR (rationale)

**CVaR (Expected Shortfall / Conditional Value-at-Risk) is the correct tail
metric because it is sub-additive (coherent), meaning the risk of a combined
portfolio never exceeds the sum of the individual risks.**  VaR is NOT
sub-additive: two individually "safe" portfolios at a given VaR level can
combine into a portfolio that exceeds that level — it rewards concentration
over diversification.  For a single-coin advisor this distinction is less
acute, but reporting a non-coherent measure is dishonest; the research and the
architect are both explicit on this point.

**CVaR_α:** the mean of the worst α-fraction of the bootstrap `total_return`
distribution across 1 000 paths.
- `cvar_95`: mean of the bottom 5% of paths by `total_return` (i.e. the α=0.05
  tail — "expected loss in the worst 5% of scenarios").
- `cvar_99`: mean of the bottom 1% of paths.

**Computed over `total_return`** (not `final_equity`) for two reasons:
1. `total_return` is already a fraction — comparable across budget sizes.
2. The existing `PathMetrics.total_return` is already captured as `f64`;
   `final_equity` is `Decimal` (kept for P(loss) integer comparisons).  Using
   `total_return` keeps all tail stats as `f64`, consistent with the existing
   statistical convention (ADR-0003 / R-NR.3).

**Median terminal wealth:** the p50 of `final_equity.to_f64()` across paths.
Answers "what does the middle outcome actually look like in dollars?" — more
intuitive than mean (which is pulled by extreme wins).

**Skew:** 3rd standardised central moment of `total_return` across the 1 000
bootstrap paths.  Positive skew = right tail (lottery-style); negative = left
tail (crash-prone).  Zero on a symmetric distribution.

---

## What shipped

### `crates/backtest/src/stats/mod.rs`

- `DistributionSummary` extended with four new fields:
  - `pub cvar_95: f64` — CVaR at α=0.05 (mean of worst 5% `total_return` paths).
  - `pub cvar_99: f64` — CVaR at α=0.01 (mean of worst 1% `total_return` paths).
  - `pub median_terminal_wealth: f64` — p50 of `final_equity` across paths.
  - `pub skew: f64` — 3rd standardised central moment of `total_return` across paths.
- `from_path_metrics` extended to compute all four.
- Unit tests: CVaR on a known hand-built path vector; median; skew (zero on
  symmetric, positive on right-skewed, negative on left-skewed).

### `crates/backtest/src/bakeoff/mod.rs`

- `CandidateKpis` extended with `pub turnover: Decimal`.
- `derive_candidate_kpis` computes turnover from `RunReport.fills` (sum of
  `price × qty`) / mean equity from `RunReport.equity_series`.
- `run_bakeoff` — default (zero) turnover for the benchmark arm fallback
  in the `CandidateKpis` literal (the unreachable branch).
- Unit tests: idle (zero fills → turnover 0); one round-trip; multi-trade.
- Frozen-gate-identity test: `rank_candidates` output is byte-identical before
  and after the new `turnover` field is populated.

### `crates/ui/src/leaderboard/state.rs`

- `LeaderRow` extended with `pub turnover: Decimal`.
- `BakeoffReportMirror::from_report` mirrors `c.kpis.turnover`.
- Fixture / test constructors updated (`row()` helper in the test module).

## Not in this increment

- Annualised turnover rate (a future formatting decision — the current
  `"N.N×"` ratio is the operator-natural "this-many-capital-equivalents"
  framing; the per-year scaling can be added later if needed).
- Per-candidate tail summary (the current `crown_tail` is the CROWN only —
  mirrors the scorecard's "one block per bake-off" precedent; per-row tail
  expansion is a future polish if requested).

## Implementation

**P1-1 Turnover:**
- `CandidateKpis.turnover: Decimal` at `crates/backtest/src/bakeoff/mod.rs:651`.
- `derive_candidate_kpis` computes `Σ(fill.price.get() × fill.qty.get()) / mean_equity`.
  Mean equity = arithmetic mean of the equity-series amounts. Zero when fills or equity empty.
- `LeaderRow.turnover: Decimal` at `crates/ui/src/leaderboard/state.rs:71`.
- `from_report` mirror updated at same file.

**P1-2 Tail metrics:**
- `DistributionSummary.{cvar_95, cvar_99, median_terminal_wealth, skew}` at
  `crates/backtest/src/stats/mod.rs:347-364`.
- `from_path_metrics` computes all four at same file (`use ToPrimitive` moved to function top per `items_after_statements` lint).
- Helper `compute_cvar` at `stats/mod.rs:617`; `compute_distribution_skew` at `stats/mod.rs:641`.

**Cascade fixes (struct literals):**
- `crates/backtest/src/bakeoff/rank.rs`: `make_candidate` test helper.
- `crates/backtest/src/bakeoff/scorecard.rs`: `make_candidate` closure.
- `crates/backtest/src/bakeoff/robustness.rs`: `make_summary` test helper.
- `crates/backtest/src/bakeoff/sweep.rs`: `fallback_distribution_summary`.
- `crates/backtest/src/bin/param_robustness_sweep.rs`: `make_summary`.
- `crates/backtest/tests/robustness_bootstrap_bites.rs`: `CandidateKpis` literal.
- `crates/ui/src/fixtures.rs`: 49 `LeaderRow` struct literals.
- `crates/ui/src/tune/state.rs`: `make_dist` and `make_kpis` helpers.

**Gate outputs (2026-06-29):**
- `cargo test -p backtest --lib`: `test result: ok. 193 passed; 0 failed; 8 ignored`
- `cargo clippy -p backtest --tests -- -D warnings`: `CLIPPY_EXIT: 0`
- `cargo fmt -- --check`: `FMT_CHECK_EXIT: 0`
- `bash scripts/verify_anchors.sh`: `ANCHORS PASS (119 / 119)`
- `python3 scripts/spec_lint.py`: `spec-lint: PASS (0 violations)`

## UI (advisor-turnover-and-tail-metrics, 2026-06-29 — Opus 4.7)

The "cost story" + "tail / median honesty layer" surfaced on the leaderboard
screen. Two new visible additions, both REPORT-ONLY (never feed the crown /
rank / verdict — same discipline as the scorecard block).

### Wireframe (post-feature leaderboard, ASCII)

```text
┌─ Strategy bake-off ─────────────────────────────────────────────────────────┐
│ Headline + caption                                          [ Run bake-off ] │
├─────────────────────────────────────────────────────────────────────────────┤
│ Plan your bake-off (coin + budget + lookback + timeframe + capital)          │
├─────────────────────────────────────────────────────────────────────────────┤
│ Ranking strategies for €200 in BTCUSDT.   13 strategies head-to-head…        │
├─────────────────────────────────────────────────────────────────────────────┤
│ Recommendation                                                                │
│   <headline> + <reasons / Explain control>                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ #  Strategy            Return   Sharpe   Max DD   Trades   Churn  ← NEW col  │
│ 1  v0.sma  ★ best     +18.37%  1.4200   -6.12%   38       3.4×   ← P1-1     │
│ 2  v0.5.macd          +9.21%   0.8800   -10.43%  64       5.8×              │
│ …                                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ How much to trust this              ← P0-1 scorecard (existing)              │
│   Strategies tried · Deflated confidence · Min history · Beats holding?      │
├─────────────────────────────────────────────────────────────────────────────┤
│ Risk story                          ← NEW P1-2 block                         │
│   Typical outcome (median) ………………………… €104,500                              │
│     "What the middle path actually ends at — more representative…"          │
│   Average loss in the worst 5 % of paths …… −18.0 %                          │
│   Average loss in the worst 1 % of paths …… −31.0 %                          │
│     "Expected shortfall (CVaR) — coherent, unlike plain VaR…"               │
│   Surprise shape (skew) ………………………………… +0.42                                  │
│     "Positive = rare big wins; negative = rare big losses…"                 │
│   Downside-only Sharpe (Sortino) ………………… +1.95                              │
│   Return vs worst drawdown (Calmar) ……………… +2.32                            │
│   Informational, not a gate — these never change the pick above.            │
├─────────────────────────────────────────────────────────────────────────────┤
│ Not financial advice. Results are simulated…  (persistent)                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### New screens / panels / widgets

- **`Churn` column** in the ranked table (`screens/leaderboard.rs`): rightmost
  numeric column, after `Trades`. Width `W_TURNOVER = 80.0` (narrower than the
  other numerics — values are always short). Format helper
  `format_turnover_ratio(Decimal) -> String` renders as `"N.N×"` (e.g.
  `"3.4×"`, `"0.0×"`). One decimal place. The `×` glyph (`\u{00d7}`) directly
  expresses "this many capital-equivalents" — unambiguous beyond `100 %` (a
  turnover of 12.7× is `1270 %`, confusing once the value exceeds 1.0).
- **`Risk story` panel** (`risk_story_block` in `screens/leaderboard.rs`):
  rendered DIRECTLY UNDER the scorecard block when `BakeoffReportMirror.tail`
  is `Some(..)`. `frame::panel` with the same chrome as `scorecard_block`. Six
  facts, each via `risk_story_fact` (composed from `scorecard_fact`):
  1. **Typical outcome (median)** — `median_terminal_wealth` via `fmt_usdt`,
     neutral `FG_1` (a typical outcome can be above or below the budget).
  2. **Average loss in the worst 5 % of paths** — `cvar_95` as a signed pct
     via `fmt_signed_pct_from_f64`, `DOWN_500` (losses by construction).
  3. **Average loss in the worst 1 % of paths** — `cvar_99`, same style.  The
     shared CVaR gloss ("Expected shortfall (CVaR) — coherent, unlike plain
     VaR") sits under this second row so the term-of-art is defined once.
  4. **Surprise shape (skew)** — `skew` via `format_signed_decimal(_, 2)`,
     neutral `FG_1` (positive AND negative skew are informational; the gloss
     explains the sign meaning).
  5. **Downside-only Sharpe (Sortino)** — crown row's `sortino` via the same
     `format_signed_decimal(_, 2)`.
  6. **Return vs worst drawdown (Calmar)** — crown row's `calmar`, same style.
  Bottom `informational, not a gate` note (`MICRO` muted) makes the
  REPORT-ONLY framing load-bearing — same shape as the scorecard's footer
  note.

### Backend wiring (a small additive change)

- **`backtest::TailSummary`** (new struct) — four `f64` fields. Sibling of
  `Scorecard` on `Recommendation`. Public-seam type so the `ui` mirror crosses
  as plain scalars (no `DistributionSummary` import on the `ui` side).
- **`Recommendation.crown_tail: Option<TailSummary>`** — the crown's tail/median
  summary, projected from `compute_robustness_distribution` for the CROWNED
  candidate only (mirrors how `Scorecard` is computed once at the end for the
  crown). `None` when `RobustnessMode::Skip` or the curve was too short.
- **`compute_robustness_flag` → `compute_robustness_distribution`** is NOT
  changed (the per-candidate path still uses `compute_robustness_flag`, which
  per the docstring "delegates to `compute_robustness_distribution` and
  discards the summary — bit-identical"). The crown's distribution is a
  *separate* call at the end, mirroring the scorecard path — small + isolated.

### UI mirror seam (the invariant)

- **`ui::leaderboard::TailSummaryView`** — four `f64` fields, identical to
  `backtest::TailSummary`. The single mirror `from_tail` is the only `ui` site
  that names the engine type.
- **`BakeoffReportMirror.tail: Option<TailSummaryView>`** — projected from
  `Recommendation.crown_tail` in `from_report`. `None` → no Risk story block
  paints (the negative control).
- **`LeaderRow.turnover`** was already mirrored by the developer in commit
  `66286e2`; the UI just had to populate the column.
- `ui` MUST NOT gain a dep on strategy/exec/llm/models — confirmed.  Only
  `backtest` plain scalars cross the existing mirror boundary.

### New strings in `ui::strings`

P1-1 (turnover column):
- `LEADERBOARD_COL_TURNOVER` ("Churn")

P1-2 (Risk story block):
- `LEADERBOARD_RISK_STORY_TITLE`
- `LEADERBOARD_RISK_STORY_CAPTION`
- `LEADERBOARD_RISK_STORY_MEDIAN_LABEL`
- `LEADERBOARD_RISK_STORY_MEDIAN_HINT`
- `LEADERBOARD_RISK_STORY_CVAR_95_LABEL`
- `LEADERBOARD_RISK_STORY_CVAR_99_LABEL`
- `LEADERBOARD_RISK_STORY_CVAR_HINT` (the shared "CVaR coherent, unlike VaR" gloss)
- `LEADERBOARD_RISK_STORY_SKEW_LABEL` / `LEADERBOARD_RISK_STORY_SKEW_HINT`
- `LEADERBOARD_RISK_STORY_SORTINO_LABEL` / `LEADERBOARD_RISK_STORY_SORTINO_HINT`
- `LEADERBOARD_RISK_STORY_CALMAR_LABEL` / `LEADERBOARD_RISK_STORY_CALMAR_HINT`
- `LEADERBOARD_RISK_STORY_INFORMATIONAL_NOTE`

All registered in `strings::all()` for the future-localization seam.

### New theme tokens

**Zero new theme tokens** — every colour / spacing / radius / text size reuses
existing tokens (`color::FG_1` / `FG_3` / `DOWN_500`; `space::M` / `XXS`;
`radius::R4` via `frame::panel`; `text::H3` / `SMALL` / `MICRO` / `BODY`).
One new layout constant `W_TURNOVER: f32 = 80.0` (a per-table column width —
not a token, the same way `W_RANK` and `W_NUM` aren't tokens).

### Accessibility notes

- **Sign is always present beyond colour.** The CVaR rows always render the
  unicode minus prefix (`\u{2212}`), and skew/Sortino/Calmar always show an
  explicit `+`/`-` sign — colour is never the only signal.
- **Right-aligned, single-decimal turnover column.** Reads as a clean grid in
  the table.
- **No new keyboard targets.** The Risk story block + the Churn column are
  display-only — no buttons, no interaction surface beyond the existing
  row-click that inspects in the Lab.
- **Plain language for every term of art.** Every label has a one-line gloss
  beneath ("median" → "the middle path's outcome"; "CVaR" → "expected
  shortfall, coherent unlike VaR"; "skew" → "rare big wins vs rare big
  losses"; "Sortino" → "downside-only Sharpe"; "Calmar" → "reward per unit of
  worst-case loss") — the no-jargon human-friendliness rule.

### Render-layer verification (CLAUDE.md non-negotiable)

`crates/ui/tests/leaderboard_risk_story_render.rs` — populated + negative
control on the 2-row `benchmark_wins` fixture (short table → block in
viewport).
- `risk_story_block_paints_and_exceeds_no_tail` — strict foreground delta
  `> 1500 px` (vs scorecard's `> 1200` because 6 facts vs 4).
- `risk_story_block_present_in_benchmark_wins_modal_case` — same fixture, but
  asserts the modal "holding wins" case still paints the block + reads
  sensibly (wider negative tail, mildly negative skew).
- Writes `/tmp/leaderboard_risk_story_render.png`,
  `/tmp/leaderboard_no_risk_story_render.png`,
  `/tmp/leaderboard_risk_story_benchmark_wins_render.png` for operator
  eyeball.

The Churn column is exercised by the existing
`leaderboard_populated_render.rs` (general table render — the new column
inherits the standard numeric-cell discipline) plus the
`format_turnover_ratio` unit test (`screens/leaderboard.rs::tests`).
