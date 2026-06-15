---
slug: simple-strategy-bear-survey
version: 0.1.0
status: shipped
owner: architect
priority: P2
updated: 2026-06-15
trace: REQ-SIMPLE-STRATEGY-BEAR-SURVEY-001
---

# Simple-strategy bear-market survey — does ANY simple strategy show a PATH-ROBUST edge in 2021-22, or does it firm ship-passive? — v0.1.0

> **The single sharp question.** Re-run the four shipped simple strategies
> (SMA 20/50, textbook MACD, RSI, BBands) over the **just-shipped 2021-22 bear
> corpus** (`data/binance-2122/`, 10 symbols, hourly, pinned `4f390622`) and ask:
> **does ANY of the four show a PATH-ROBUST edge in a real, deep bear market**
> — 2022 was BTC ≈ −64% / LUNA / FTX, 2021 a two-peak bull with a −50% mid-year
> drawdown — **or does this further firm the 2026-06-08 "ship passive" terminal
> verdict on a wider, deeper bear sample?**
>
> This is a **stress-test of an already-closed decision, not a casual reopening
> of it.** The active-vs-passive search CONCLUDED 2026-06-08 (ship passive across
> price + positioning + on-chain); the 2026-06-15 overfit-guard then showed the
> survey's one apparent down-market "hedge" (AVAX/DOT 2024) was **path-FRAGILE**
> ([`analysis-2026-06-15-simple-strategy-overfit-guard.md`](../dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md)).
> The honest motivation: that fragility finding rested on **2 idiosyncratic
> alt-coin dips in an otherwise-bull 2023-24**. The corpus simply did not contain
> a real, market-wide bear. Now it does. The right move is to **re-run the same
> path-robustness discipline on the wider sample** — with humility — so the
> ship-passive verdict is tested against the deepest bear we have, not just
> asserted to hold there.
>
> **The decision rule was frozen BEFORE any of this data existed.** The
> pre-registered § 0 rule ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0,
> **p5 Sharpe < 0 ⇒ FRAGILE**) is reused **AS-IS** — it is the ruler; the 2021-22
> numbers are scored against it, not the reverse. No band is re-derived or softened.
>
> **Both answers are decision-grade, and they are asymmetric — name the high-value
> tail.** A **null result** (no strategy survives § 0 on the bear sample) *firms*
> ship-passive on the strongest available evidence and closes the bear-market
> active story too. A **positive result** (any apparent winner comes back ROBUST
> on a real bear) would be **surprising** given the 2026-06-08 conclusion and the
> 2024 fragility finding — it would **REOPEN** the active-vs-passive question for a
> scoped v0.2.0 follow-on (a trend-following down-market product line). That tail is
> low-prior but high-value: the whole point of stress-testing on the deepest bear
> is that a survivor *there* would be the most credible non-passive signal the
> program has produced. Neither outcome adds a live-trading path (analysis tooling
> only; operator constraint 2026-06-12).

---

## Why (motivation)

The 2026-06-13/14 real-data survey
([`realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md))
ran the four shipped simple strategies on the real Binance **2023-24** hourly
corpus (10 symbols, net of 4 bps taker cost) vs buy-and-hold. Its Finding 1
(now **revised**) flagged that in the only 2 down-market cells (AVAX 2024
−8.2%, DOT 2024 −19.6%) trend-following protected capital — *"suggestive of the
protection property, not statistically conclusive."* The 2026-06-15 overfit-guard
then block-bootstrap-tested exactly those 2 cells (N=500, frozen § 0 rule) and
found them **all FRAGILE** (positive median Sharpe, negative p5 tail): the hedge
was an artifact of the one 2024 ordering, not a robust property.

That fragility verdict is honest but **narrow by construction**: it rests on **2
idiosyncratic alt-coin dips** inside an otherwise broadly-bullish 2023-24. As the
overfit-guard's own scope cap (D-OG.5) and the survey's § Caveats both state, a
**wider, deeper down-market sample** is the named v0.2.0 follow-on. The
`binance-corpus-expansion` feature shipped exactly that input on 2026-06-15:
**2021-22 hourly for the same 10 symbols** (`data/binance-2122/`, pin
`4f390622`), turning a 2-point down sample into a market-wide bear (the entire
universe in 2022 + the H1→H2-2021 drawdown). **2022 is the real test**: a
multi-month, cross-universe drawdown (BTC ≈ −64%, ETH ≈ −67%, with LUNA/3AC in
May–Jun and FTX in Nov) — exactly the regime where a trend-follower's "cut losers,
sidestep the drawdown" property *should* shine if it is real anywhere.

This feature consumes that corpus to answer the sharp question above. It is an
**analysis-tooling feature** (a re-runnable `#[ignore]` harness + a `findings`
dev-note), not a strategy feature: it ships **no new strategy code, no overlay,
no live path**. It is the direct successor to the overfit-guard, generalised from
"bootstrap the 2 apparent-hedge cells" to "survey the whole bear corpus, then
bootstrap the apparent winners."

## The two-stage method (the efficient shape — mirrors how overfit-guard worked)

The overfit-guard was implicitly two-stage: the **survey** identified 2 apparent
down-market winners (cheap, 1 path each), then the **bootstrap** guarded only
those 2 cells (expensive, N=500 each). This feature makes that two-stage shape
**explicit and corpus-wide**:

### Stage 1 — point survey (cheap, 1 path per cell)

Run the **point survey** over 2021-22: all **10 symbols × 2 years × 4 strategies**
= **80 single-path backtests**, each strategy's total return vs buy-and-hold,
net of cost. This is the existing `realdata_simple_strategy_survey` shape, just
pointed at `data/binance-2122/` instead of `data/binance/`. Fast (~the same
per-cell cost as the 2023-24 survey).

**Stage-1 output: the APPARENT-winner set.** A (symbol·year·strategy) cell is an
**apparent winner** iff it beats buy-and-hold by a margin in the bear sample —
i.e. the candidate-selection predicate (see Q-BS.2, the threshold is the riskiest
open question). The expected shape, by the 2023-24 precedent, is that
apparent winners cluster in the **down/sideways cells** (all of 2022 + the 2021
drawdown leg) where trend-followers go flat-to-positive while B&H bleeds. Stage 1
**does not conclude anything** — a single-path margin over B&H is exactly the
observation the overfit-guard showed can be path-fragile. Stage 1 only **selects
candidates** for the real test.

### Stage 2 — block-bootstrap path-robustness guard (expensive, N=500 per candidate)

For **each apparent winner from Stage 1** (NOT all 80 cells — bootstrap the
*candidates*, exactly as overfit-guard bootstrapped the 2 apparent-hedge cells),
run the **N=500 stationary block-bootstrap** robustness guard: resample
plausibly-different orderings of that cell's real bars, reduce to Sharpe
p5/p25/p50/p75/p95 + prob-of-loss + max-DD tail, and **score against the frozen
§ 0 rule AS-IS** (composite = worst band; **p5 Sharpe < 0 ⇒ FRAGILE**). This is
the existing `realdata_simple_strategy_overfit_guard` ensemble shape, pointed at
`data/binance-2122/` and driven by the Stage-1 candidate list instead of a
hard-coded 2-cell matrix.

**Stage-2 output = the headline.** *Are any of the apparent-winner cells actually
PATH-ROBUST under the frozen § 0 rule?* If every candidate scores FRAGILE/MARGINAL
→ the bear sample **firms ship-passive** (no path-robust edge even in the deepest
bear). If any candidate scores **ROBUST** → that is the surprising, high-value
tail that **reopens** the question for a scoped v0.2.0.

### Negative control (carried from overfit-guard AC-OG.4)

RSI and BBands are the no-edge mean-reverters (flat-to-negative everywhere in
2023-24). If either is **selected as an apparent winner in Stage 1 AND scores
ROBUST in Stage 2**, the harness is miscalibrated — escalate (a no-edge
mean-reverter coming back path-robust on a bear sample is a RED flag, not a
discovery). At minimum, ≥1 mean-reverter candidate (if any are selected) and ≥1
up-market contrast cell (a 2021-bull-leg cell) must be scored as the
discrimination check, mirroring the overfit-guard's AVAX·2023 SMA control.

## Compute cost (flag the watch-recipe need for the developer)

- **Stage 1:** ~80 cheap single-path backtests. By the 2023-24 survey timing this
  is well under the 2-minute threshold — likely a single sub-minute run.
- **Stage 2:** `N_candidates × 500` bootstrap runs. The overfit-guard's
  **9 ensembles × 500 = ~78–79 s**. If Stage 1 selects materially more than ~9
  candidates (plausible — 2022 is a market-wide bear, so down-cells could be many
  more than the 2023-24 sample's 2), the Stage-2 **release** run **will exceed
  2 minutes**. **The developer MUST emit a copy-pasteable `watch -n N '<probe>'`
  block when kicking off the Stage-2 release run** (per the MEMORY watch-recipe
  contract). The architect should set a sane **candidate cap** if Stage 1 is
  permissive (Q-BS.2) so the Stage-2 cost stays bounded and the run is
  finishable in one sitting.

## Anchoring decision — UN-ANCHORED, mirroring the survey + overfit-guard precedent

**UN-ANCHORED.** Per the survey precedent (no report, no `anchors.toml` row) and
the overfit-guard precedent (`#[ignore]` harness, `findings` dev-note, no
anchored `spec/*/reports/` file, no `anchors.toml` row), the deliverable is a
**re-runnable `#[ignore]` harness** whose `--nocapture` stdout IS the finding,
plus a `findings`-status dev-note recording the actual numbers — **NOT** an
anchored report and **NOT** an `anchors.toml` row.

**Justification for not anchoring (not a rubber-stamp):**
- The question is a **one-shot epistemic stress-test** ("does any simple strategy
  survive on the bear sample?"), not a regression-protected production surface.
  The block-bootstrap *generator* (C1) is already anchored; re-anchoring its
  *application* to the bear candidates adds `anchors.toml` churn for a finding
  cited once and folded into the passive-baseline thesis.
- ADR-0051's anchoring discipline exists for **canonical robustness verdicts that
  gate paper→live**. There is no live path here, and the § 0 ruler is already the
  anchored contract being applied.
- **Determinism is still enforced** (ADR-0051 D1 seeds: `path_seed_j =
  ensemble_seed.wrapping_add(j·0x9E3779B9)`, constant fill_seed `0xC0FFEE`), so
  the `#[ignore]` run is byte-reproducible without an anchor — the same guarantee
  an anchor would give, without the row.

**Escalation hook (Q-BS.3):** IF Stage 2 returns a ROBUST survivor AND the
operator greenlights a trend-following *product line* off it, THEN that follow-on
should anchor a canonical report per ADR-0051 § D6. That is the **v0.2.0 reopen
path**, explicitly deferred — it is NOT triggered by a null result.

## The CLAUDE.md baseline-equity-divergence e2e gate is N/A (justified, not rubber-stamped)

The CLAUDE.md non-negotiable — "every strategy overlay or sizing-modifier ships
with a day-1 baseline-equity-divergence e2e test" — exists to catch a **no-op
overlay** (a `scale` computed but never applied to equity; the v3-vol-overlay
precedent). **This feature introduces no overlay and no sizing modifier.** It is
read-only analysis tooling: it runs the four **already-shipped** survey strategy
ids (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`) unchanged through the
**already-shipped** `run_scenario` engine over (Stage 1) the real 2021-22 bars and
(Stage 2) bootstrap-resampled bars, and reduces the output. There is **no new
decision variable that could silently fail to wire** — the strategies'
equity-affecting code is the untouched production path.

The applicable correctness guards — the moral equivalent of the divergence gate
for an analysis harness, exactly as the overfit-guard's D-OG.6 established — are:
1. **Two-run byte-identical determinism** (AC-BS.5) — fixed seeds make the
   ensemble reproducible; a non-deterministic harness is a wiring bug.
2. **The negative control** (AC-BS.6) — a no-edge mean-reverter must NOT come
   back ROBUST; if it does, the harness is miscalibrated. This is the tripwire
   that the test is reading the market, not blessing churn.

Gate stated **N/A on substance, not skipped**.

## Requirements

- **R-BS.1 (Stage-1 corpus + loader)** — Stage 1 loads real 2021-22 hourly bars
  from **`data/binance-2122/`** via `data::ReplayFeed::new(root.join("data/binance-2122"), true).subscribe_bars(sym, Timeframe::OneHour)` and UTC year-boundary
  windowing — i.e. the survey's `load_year_bars` shape with the corpus root
  changed from `data/binance` to `data/binance-2122` (and the year boundaries set
  to 2021 + 2022). The 10-symbol universe is **identical** to the 2023-24 survey
  (BTC, ETH, BNB, SOL, XRP, ADA, DOGE, AVAX, DOT, LINK USDT) so cells line up
  symbol-for-symbol with the prior survey. NO new loader logic.
- **R-BS.2 (Stage-1 point survey)** — For each of the 10 symbols × {2021, 2022} ×
  4 strategies (80 cells), run one single-path backtest via the survey's
  `run_scenario` + `bars_override` runner and report total return % vs
  buy-and-hold, net of cost. NO new engine code. SKIP cleanly when
  `data/binance-2122/` is absent (mirror the survey's `…/2023/01.parquet`
  existence guard with a 2021-22 path).
- **R-BS.3 (candidate selection)** — From Stage 1, deterministically select the
  **apparent-winner set**: cells where the strategy beats buy-and-hold by the
  candidate-selection margin (Q-BS.2 — the threshold the architect locks). The
  predicate MUST be explicit, logged (print which cells were selected and why),
  and bounded by a candidate cap (Q-BS.2) so the Stage-2 cost is finishable.
- **R-BS.4 (Stage-2 bootstrap, frozen N + block length)** — For each apparent
  winner, generate **N=500** stationary block-bootstrap paths with
  `BlockBootstrapPathGen` in single-symbol mode (1-entry universe),
  `BlockLengthPolicy::Auto` (Politis–White). N=500 and Auto transfer **AS-IS**
  from the overfit-guard so the § 0 bands (calibrated at N=500) apply without
  recalibration. NO new path generator.
- **R-BS.5 (Stage-2 reduce + score)** — Reduce the N per-path equity curves with
  `stats::DistributionSummary::from_path_metrics(&[PathMetrics])` (the **correct**
  reducer per overfit-guard D-OG.0 — assemble `PathMetrics` per path from
  `RunReport.equity_series` via the shipped `compute_sharpe_hourly` /
  `compute_sortino_hourly` / `compute_calmar` / `compute_max_drawdown_f64` /
  `compute_total_return` helpers, index-order, do NOT sort). Score each ensemble
  against the **frozen § 0 rule AS-IS**: `sharpe.p5 < 0 ⇒ FRAGILE`; `prob_loss >
  0.35 ⇒ FRAGILE`; `max_dd_tail_p95 > 0.70 ⇒ FRAGILE`; ROBUST iff `sharpe.p5 ≥ 0.5
  AND prob_loss ≤ 0.15 AND max_dd_tail_p95 ≤ 0.50`; else MARGINAL; composite =
  worst band. Do NOT re-derive or soften the bands (if seen-then-changed, log per
  that note's changelog discipline). NO new statistics.
- **R-BS.6 (negative control + contrast)** — Score ≥1 selected mean-reverter
  candidate (if any) and ≥1 up-market-contrast cell (a 2021-bull-leg cell, e.g.
  an H2-2021 or full-2021 up-cell for a symbol that rose) as the discrimination
  check (carried from overfit-guard AC-OG.4 / R-OG.5). A no-edge mean-reverter
  scoring ROBUST is a RED flag → escalate.
- **R-BS.7 (Decimal, never f64)** — All money/equity math in `Decimal` /
  `Money<Usdt>`; f64 permitted ONLY inside the already-shipped `stats` reducer
  (the existing Decimal-in→f64-out contract). Do not introduce new f64 equity
  paths.
- **R-BS.8 (determinism)** — Per ADR-0051 D1: `path_seed_j =
  ensemble_seed.wrapping_add(j·0x9E3779B9)`, a DISTINCT `ensemble_seed` per
  (strategy × cell), `ScenarioConfig.seed` the CONSTANT `[0xC0,0xFF,0xEE,…]` for
  every path (orthogonality — per-path variation lives ONLY in the bootstrap
  `path_seed_j`, NOT the engine fill-tie-break seed). Two runs ⇒ byte-identical
  summary. **Determinism caveat (inherited):** byte-identity is contracted on the
  Apple-Silicon canonical box (ADR-0051 D5 / ADR-0043 precedent); cross-platform
  parity is not contracted and does not affect the decision (the percentiles are
  the percentiles).
- **R-BS.9 (UN-ANCHORED)** — `#[ignore]`, no `spec/*/reports/` file, no
  `anchors.toml` row (per § Anchoring). The `--nocapture` stdout + a `findings`
  dev-note are the deliverable. NO baseline-divergence e2e gate (analysis tooling,
  not a strategy overlay — gate N/A, stated explicitly above).
- **R-BS.10 (no live path)** — NO live-trading path, NO order-execution surface
  beyond what the survey's `run_scenario` already exercises in paper mode
  (operator constraint, 2026-06-12).
- **R-BS.11 (watch recipe)** — The developer MUST emit a copy-pasteable
  `watch -n N '<probe>'` block when kicking off the Stage-2 release run if it may
  exceed 2 minutes (likely, given a market-wide-bear candidate count — see §
  Compute cost).
- **R-BS.12 (corpus untouched)** — This feature **reads** `data/binance-2122/`
  (pin `4f390622`); it MUST NOT mutate the corpus, its `REVISION.toml`, or
  `data/binance`/`data/yahoo`. No `REVISION.toml` re-emit.
- **R-BS.13 (spec-lint zero-new)** — `scripts/spec_lint.py` MUST stay at **70**
  findings with **zero new** findings attributable to this feature's files
  (baseline measured 70 at the parent commit).

## Acceptance criteria

- **AC-BS.1 (Stage 1 runs)** — Running the harness on a machine WITH
  `data/binance-2122/` prints the full Stage-1 point-survey table: 10 symbols ×
  {2021, 2022} × 4 strategies, each cell's total return % (trade count) vs
  buy-and-hold, net of cost. (Cells with thin/absent on-disk parquet print a
  `(only N bars)` row and are skipped — never silently synthetic, mirroring the
  survey.)
- **AC-BS.2 (candidate set is explicit)** — The harness prints the **selected
  apparent-winner set** with the selection predicate and threshold value, and the
  count is within the architect's candidate cap. A reader can see exactly which
  cells advanced to Stage 2 and why.
- **AC-BS.3 (Stage 2 runs on candidates)** — For each apparent winner, the harness
  prints N, Sharpe p5/p25/p50/p75/p95, prob-of-loss, max-DD p50/p95, P(Sharpe>0),
  and the frozen § 0 composite verdict (ROBUST/MARGINAL/FRAGILE).
- **AC-BS.4 (SKIP-safe)** — On a machine WITHOUT `data/binance-2122/`, the harness
  SKIPs cleanly (prints a SKIP line, exits green) — never fabricates synthetic
  data, never fails the default suite (it is `#[ignore]`).
- **AC-BS.5 (determinism)** — Two consecutive `--release --ignored --nocapture`
  runs produce **byte-identical** summary numbers (R-BS.8), demonstrable by
  diffing two captures. (Stage-1 selection must be deterministic too — same
  candidate set both runs.)
- **AC-BS.6 (negative control)** — Any selected mean-reverter (RSI/BBands)
  candidate scores FRAGILE or MARGINAL, NOT ROBUST — the test does not spuriously
  bless no-edge churn. If a mean-reverter comes back ROBUST, escalate as
  miscalibration.
- **AC-BS.7 (headline answered in writing)** — A `findings`-status dev-note
  (`spec/dev-notes/analysis-<date>-simple-strategy-bear-survey.md` — written by
  the analyst/orchestrator AFTER the run, NOT an anchored `spec/*/reports/` file)
  states, with the actual p5 Sharpe + prob-of-loss numbers per candidate, whether
  **any** simple strategy shows a **path-robust edge** on the 2021-22 bear sample
  — and folds the result into the passive-baseline thesis (firms ship-passive on a
  null; flags the v0.2.0 reopen on a ROBUST survivor). The claim is **scope-capped**
  (§ Scope cap).
- **AC-BS.8 (corpus + anchors untouched)** — `git status` shows no change to
  `data/binance-2122/REVISION.toml`, `data/binance/REVISION.toml`,
  `data/yahoo/REVISION.toml`, or any `anchors.toml` row. `scripts/verify_anchors.sh`
  stays green. No `spec/*/reports/` file is written.
- **AC-BS.9 (lint + clippy clean)** — `scripts/spec_lint.py` = 70, zero new
  (R-BS.13). `cargo clippy --tests -p <crate> -- -D warnings` clean on the new
  harness. No `.unwrap()` outside `#[cfg(test)]`.

## Scope cap (honest, per the overfit-guard D-OG.5 precedent)

**State the result at the level the evidence supports, and no wider:**

> Even a wider, deeper sample is still **hourly bars, shipped-default params, 10
> large-cap symbols, and 2 specific bear years (2021-22)**. A **null result**
> (no path-robust edge) firms ship-passive on the strongest available bear
> evidence — but it does **NOT** prove "no strategy can ever beat passive in a
> bear market." Untuned defaults, one timeframe, a fixed universe, and two
> specific years are not the space of all strategies. A **positive result** (a
> ROBUST survivor) would be **surprising** given the 2026-06-08 conclusion and
> the 2024 fragility finding, and would **REOPEN** the active-vs-passive question
> for a scoped v0.2.0 follow-on — it would NOT by itself greenlight live trading
> or a product line (that is a separate operator decision with its own gates).

Specifically, per the overfit-guard's symmetric cap:
- A per-symbol-year ROBUST verdict is a statement about **path-resampling within
  that symbol's 2021 or 2022 individually**, NOT a cross-sectional "trend-following
  hedges bear markets in general" claim.
- The frozen § 0 rule scores robustness **to resampled real history**, not to
  out-of-history regimes (block bootstrap cannot synthesize a regime 2021-22 never
  contained — § 0 scope note).
- 2021-22 is **two specific bear/sideways years**; a different bear (a future
  2025-style event, a regulatory shock) is outside what this distribution speaks to.

**The high-value tail, named explicitly:** a ROBUST survivor on a real
market-wide bear is the most credible non-passive signal the program could
produce — which is *why* this stress-test is worth running. It is low-prior
(everything to date says passive wins net of cost) but, if it appears, it is the
one result that should most change the operator's mind. Flag it loudly if it
appears; do not bury it as a footnote.

## Open questions (for the architect)

- **Q-BS.1 (the seam — RISKIEST)** — How to wire Stage 1 → Stage 2 over the new
  corpus, given **both** existing harnesses hard-code `data/binance`
  ([`realdata_simple_strategy_survey.rs:58/112`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs),
  [`realdata_simple_strategy_overfit_guard.rs:162/345`](../../crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs))
  and the overfit-guard's cell matrix is a **hard-coded 2-cell list**, not a
  Stage-1-derived candidate set? **Three shapes:**
  - **(a) One new combined `#[ignore]` harness** (`realdata_simple_strategy_bear_survey.rs`)
    that runs Stage 1 over `data/binance-2122/`, derives the candidate set in
    process, then runs Stage 2 on the candidates — a single self-contained file
    that composes the two shipped pieces (survey runner + bootstrap ensemble) end
    to end. **(Recommended — durable.)** It is the exact shape the overfit-guard
    proved works (survey-identifies → bootstrap-guards), now with the Stage-1→2
    hand-off *in code* instead of done by hand across two features. One artifact,
    one run, one deterministic candidate list, no cross-corpus coupling to the
    `data/binance`-pinned existing harnesses. Slightly more code than (b) but it is
    the composable, re-runnable, single-source-of-truth artifact a future
    bear-survey (other years, daily) extends — it does not spawn a "now reconcile
    two harnesses" cleanup. **If-budget-tightens fallback: (b).**
  - **(b) Parameterize the two existing harnesses on the corpus root** (thread a
    `corpus_root` / env-var so they can target `data/binance-2122/`), then run
    them in sequence and hand the candidate list across by hand. Cheaper to type
    (~edits to 2 files) BUT mutates two **shipped, working** harnesses (regression
    surface on the 2023-24 survey + the anchored-adjacent overfit-guard finding),
    splits the two-stage logic across two files + a manual hand-off step, and
    leaves the candidate selection un-automated (a human transcribes Stage-1
    winners into the Stage-2 matrix — exactly the manual seam the overfit-guard
    had, which is fine for 2 cells but error-prone for a market-wide-bear count).
    Fallback only.
  - **(c) Extend `monte_carlo.rs`** — rejected for the same reason the
    overfit-guard rejected it (Q-OG.2): couples a one-shot finding to the
    anchor-grade production MC binary; incurs an ADR-0051 re-emission obligation
    on any anchored-report drift; no benefit here. NOT recommended.
  Recommended architect lock: **(a)**. The two existing harnesses stay
  byte-untouched (no regression to the 2023-24 survey, no `data/binance` re-read),
  and the new combined harness is the durable artifact the v0.2.0 (daily / other
  bear years) extends.
- **Q-BS.2 (candidate-selection threshold + cap — second-riskiest)** — What is the
  Stage-1 predicate that promotes a cell to a Stage-2 candidate, and what is the
  candidate cap? Two sub-questions:
  - **Threshold.** Options: (i) **beats B&H by ≥ X percentage points** (absolute
    return margin — the overfit-guard's implicit predicate was "trend-follower
    flat-to-positive while B&H lost", i.e. a large margin in down-cells); (ii)
    **beats B&H AND B&H was negative** (down-market-protection-specific — directly
    targets the hedge thesis, ignores bull-cell churn); (iii) **top-K by margin**
    (rank-based, bounds the count deterministically regardless of regime). The
    analyst **lean is (ii) gated by a margin** — *cell beats B&H by ≥ ~5 pp AND
    B&H was negative* — because the whole motivation is the **down-market** hedge,
    and a bull-cell where a strategy happens to beat a flat B&H is not the thesis.
    But the architect owns the exact X and the AND/OR shape; it is a real design
    decision because it sets what "apparent winner" means and therefore the
    Stage-2 cost. **Lock it BEFORE the run (pre-registration discipline — same as
    the § 0 rule): the predicate is fixed first, the candidate set scored against
    it, not the reverse.** Whatever is chosen, the harness prints it.
  - **Cap.** If the predicate is permissive (2022 is a market-wide bear → many
    down-cells), the candidate count could be large and the Stage-2 run long. Set
    a **candidate cap** (e.g. top-N by margin, or N≤~20) so the run is finishable
    in one sitting and the watch-recipe stays honest. Recommend a cap that keeps
    Stage 2 ≲ a few minutes (overfit-guard's 9 ensembles ≈ 79 s → ~20 ensembles
    ≈ ~3 min is a reasonable ceiling).
- **Q-BS.3 (anchoring + reopen path)** — UN-ANCHORED for v0.1.0 (§ Anchoring)?
  **Recommended default: YES, un-anchored** (matches both precedents, lowest
  `anchors.toml` churn, the § 0 ruler is the anchored contract). Anchor a canonical
  report ONLY on the **ROBUST-survivor reopen path** if the operator greenlights a
  trend-following product line off a positive result (then it becomes a
  paper-adjacent gate → ADR-0051 § D6) — a v0.2.0 decision, explicitly deferred,
  NOT triggered by a null.
- **Q-BS.4 (ragged early-2021 coverage)** — The corpus-expansion feature
  (ADR-0056 Q3) tolerated ragged early-2021 coverage (a thin symbol-month writes a
  short parquet, does not fail the fetch). The developer M-report says **all 10
  symbols had 744 bars in 2021-01** (no raggedness in practice), but the harness
  MUST still tolerate a thin cell gracefully (print `(only N bars)`, skip — never
  fail, never synthesize). Recommend: tolerate-and-report, same guard as the survey.
- **Q-BS.5 (block-length degeneration on a bear series)** — Politis–White `Auto`
  block length is the frozen default (R-BS.4). On the overfit-guard's 2024 series
  it gave L > 1 (no i.i.d. degeneration). If `Auto` degenerates to **L=1** on a
  2021-22 series (the harness already WARNs on this per the overfit-guard's
  `selected_block_length ≤ 1` check), the developer logs it and the architect
  decides whether to pin `Fixed` with a value — but do NOT pre-empt; run `Auto`
  first and observe. (A degenerate L=1 would mean i.i.d. resampling, which
  understates path dependence — a real finding to surface, not silently fix.)

## References

- **Sharp-question lineage / what this builds on:**
  [`realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md)
  (the original 2023-24 survey + revised Finding 1),
  [`analysis-2026-06-15-simple-strategy-overfit-guard.md`](../dev-notes/analysis-2026-06-15-simple-strategy-overfit-guard.md)
  (the path-fragility finding this stress-tests on a wider sample).
- **Frozen decision rule (reused AS-IS):**
  [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0.
- **Ship-passive thesis being stress-tested:**
  [`passive-baseline.md`](../runbooks/passive-baseline.md) (+ its 2026-06-15
  REVISION).
- **Corpus consumed:**
  [`binance-corpus-expansion/feature.md`](../binance-corpus-expansion/feature.md)
  (`data/binance-2122/`, pin `4f390622`),
  [ADR-0056](../architecture/adr/0056-binance-corpus-timeframe-layout-convention.md)
  (own-root-per-timeframe layout).
- **Harnesses to compose / extend:**
  [`realdata_simple_strategy_survey.rs`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs)
  (Stage-1 point-survey shape),
  [`realdata_simple_strategy_overfit_guard.rs`](../../crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs)
  (Stage-2 N=500 block-bootstrap shape + the proven reducer/seed/scoring seam).
- **MC determinism + reducer API:**
  [ADR-0051](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)
  (D1 seeds; D-OG.0 correction: `DistributionSummary::from_path_metrics`, not
  `EnsembleSummary`).

## Design

_Architect M-T1 lock (2026-06-15). Seam confirmed end-to-end against source:
both shipped harnesses' primitives are test-local privates, so the new combined
harness COPIES them with the corpus root + years changed — no edit to either
shipped file. `RunReport.equity_series` / `RunReport.kpis` verified at
[`engine.rs:269/271/275`](../../crates/backtest/src/engine.rs); the bootstrap
API (`BlockBootstrapPathGen::new`, `.generate`, `path.bars_by_symbol`,
`path.selected_block_length`, `BlockLengthPolicy::Auto`) verified at
[`synth/bootstrap.rs:111`](../../crates/data/src/synth/bootstrap.rs) +
[`synth/mod.rs:70/73/102`](../../crates/data/src/synth/mod.rs); the reducer
`DistributionSummary::from_path_metrics(&[PathMetrics])` + the five `compute_*`
helpers verified at [`stats/mod.rs`](../../crates/backtest/src/stats/mod.rs)
(NOT `EnsembleSummary` — confirmed absent, per overfit-guard D-OG.0). ADR-0051
D1 seed string `path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`
verified at [ADR-0051 § D1:75](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md).
Corpus present on disk: `data/binance-2122/{10 symbols}/{2021,2022}/NN.parquet`._

### D-BS.1 — The seam: ONE new combined `#[ignore]` harness (Q-BS.1 → option (a), LOCKED)

**Decision.** Create ONE new self-contained harness
**`crates/backtest/tests/realdata_simple_strategy_bear_survey.rs`** that runs
Stage 1 over `data/binance-2122/`, derives the candidate set in-process, then
runs Stage 2 on the candidates. The two shipped harnesses
(`realdata_simple_strategy_survey.rs`, `realdata_simple_strategy_overfit_guard.rs`)
stay **byte-untouched**.

**Rationale (one line).** Both harnesses' Stage-1 + Stage-2 primitives are
test-local `fn`s (not `pub`), so they cannot be imported — the choice is
copy-into-new-file vs extract-to-shared-util; **copy wins** because it keeps the
two shipped (anchor-adjacent) harnesses' `data/binance` reads provably intact
(zero regression surface) at the cost of ~3 small duplicated helpers, whereas
extract-to-util would touch all three files and put the shipped overfit-guard's
behavior at risk for no decision-grade benefit. Option (b) parameterize-existing
is rejected (mutates two shipped harnesses); option (c) extend `monte_carlo.rs`
is rejected (couples a one-shot finding to the anchor-grade MC binary → ADR-0051
re-emission obligation), same rationale as overfit-guard Q-OG.2.

**Copy-vs-extract — COPY (explicit).** The new harness copies, with the corpus
root changed from `data/binance` → `data/binance-2122` and years 2023/24 →
2021/22: `workspace_root`, `load_year_bars`, `buy_and_hold_pct` (from the
survey); `run_one_path`, `path_metrics_from_report`, `score_verdict`,
`run_ensemble` (from the overfit-guard). These are ≤ ~10 lines each; duplication
is the cheaper, lower-risk option than a shared `tests/util` module that would
force a behavior-identical re-verification of both shipped harnesses (that
re-verification is the only path under which extract is allowed, and it is NOT
worth it here — flagged as the rejected alternative).

**Harness shape (developer builds to this):**

```text
realdata_simple_strategy_bear_survey.rs
├── const SEED: [u8;32]            // 0xC0FFEE… constant engine seed (ADR-0051 D1)
├── const N_PATHS: usize = 500
├── const STRATS: &[(&str,&str)]   // 4 survey ids verbatim
├── struct Stage1Cell { sym, year_label, year_bounds:(u64,u64), strat_idx,
│                       strat_id, strat_label, bh_pct: Decimal,
│                       strat_ret_pct: Decimal, trade_count, n_bars }
│       // one per (symbol × {2021,2022} × strategy) = up to 80
├── fn select_candidates(&[Stage1Cell]) -> Vec<&Stage1Cell>   // D-BS.2 predicate + cap
├── fn ensemble_seed_for(strat_idx, candidate_rank) -> u64     // distinct per (strategy × cell)
├── async fn run_ensemble(...) -> Option<DistributionSummary>  // copied from overfit-guard
└── #[tokio::test] #[ignore] async fn realdata_simple_strategy_bear_survey()
        1. SKIP-guard: data/binance-2122/BTCUSDT/2022/01.parquet absent → SKIP, return.
        2. STAGE 1: for sym in 10, for year in {2021,2022}: load_year_bars;
           thin-cell (<100 bars) → print "(only N bars)" + skip; else for each of
           4 strats run_strategy → push Stage1Cell. Print the full 80-cell table.
        3. SELECT: candidates = select_candidates(&cells); print the predicate
           string + threshold + the selected cells with their margins (D-BS.2).
        4. STAGE 2: for (rank, cand) in candidates.enumerate(): ensemble_seed =
           ensemble_seed_for(cand.strat_idx, rank); run_ensemble (N=500, Auto);
           score_verdict; print the § 0 row. WARN if selected_block_length ≤ 1.
        5. Print legend + negative-control reminder.
```

The Stage-1 candidate hand-off is **in-code** (`select_candidates` consumes the
`Vec<Stage1Cell>` Stage 1 built and returns the cells Stage 2 loops) — no manual
transcription, deterministic, single source of truth. This is the durable
artifact a v0.2.0 (daily / other bear years) extends.

### D-BS.2 — Candidate predicate + cap (Q-BS.2) — ⚠️ PRE-REGISTERED / FROZEN BEFORE THE RUN ⚠️

> **PRE-REGISTRATION NOTICE (same discipline as the § 0 rule).** The predicate
> and cap below are **fixed now, before the harness emits a single Stage-1
> number.** The candidate set is scored against this predicate, not the predicate
> tuned to the candidate set. If either the threshold or the cap is changed after
> seeing Stage-1 output, that change MUST be logged in this feature's Changelog
> with the before/after value and an explicit operator signoff — silent post-hoc
> tuning of "what counts as an apparent winner" would p-hack Stage-2 selection
> exactly as a moved § 0 band would p-hack the verdict.

**Predicate (FROZEN).** A Stage-1 cell `(symbol · year · strategy)` is an
**apparent winner** — and therefore a Stage-2 candidate — iff **BOTH** hold:

1. **Down-market gate:** `buy_and_hold_pct < 0` (B&H lost money that
   symbol-year — the hedge thesis is *down-market protection*, so a bull-cell
   where a strategy merely beats a rising B&H is NOT the thesis and is excluded);
   **AND**
2. **Margin gate:** `strat_ret_pct − buy_and_hold_pct ≥ 10.0` percentage points
   (the strategy beat passive by ≥ 10 pp).

This is the analyst's lean (ii) — *beats B&H AND B&H negative* — with the margin
threshold **X = 10 pp** locked.

**Why X = 10 pp (FROZEN rationale).** The 2023-24 down-cells that motivated this
whole lineage cleared this bar with room: AVAX·2024 SMA was +5.0% vs B&H −8.2%
(margin ≈ 13.2 pp) and DOT·2024 SMA was +6.4% vs B&H −19.6% (margin ≈ 26.0 pp).
A 10-pp floor (a) admits exactly the class of genuine "strategy went flat-to-up
while passive bled double digits" cells the overfit-guard was built to test,
(b) rejects noise-band ties where a strategy is marginally less-bad than a small
loss (those are not a hedge signal worth 500 bootstrap paths), and (c) is a
round, defensible number set without reference to any 2021-22 figure. It is
deliberately the **same order** as the smaller of the two real motivating
margins, so the bar is "this would have caught the 2024 cells" — not tighter (it
would not change which 2024 cells qualified) and not looser (no sub-10-pp churn).

**Cap (FROZEN).** **N_candidates ≤ 16.** If `select_candidates` finds more than
16 qualifying cells, keep the **top 16 by margin** `(strat_ret_pct − bh_pct)`
descending. **Deterministic tie-break** for equal margins (and to fix iteration
order before truncation): sort by `(margin DESC, symbol ASC alphabetical, year
ASC [2021<2022], strat_idx ASC [SMA<MACD<RSI<BBands])`. The harness prints the
full qualifying set AND which cells were kept vs dropped by the cap.

**Why cap = 16 (FROZEN rationale).** Stage-2 cost is `N_candidates × 500`. The
overfit-guard's 9 ensembles × 500 ran in ≈ 79 s (≈ 8.8 s/ensemble). 16 ensembles
≈ **140 s (~2.3 min)** — comfortably finishable in one sitting while still
exceeding the 2-min watch-recipe threshold (so R-BS.11's `watch` block is
mandatory). 16 also leaves headroom under the analyst's ~20 ceiling. A
market-wide bear (2022 was the whole universe down) could plausibly produce > 16
qualifiers; the cap-by-margin keeps the **most-protective** cells (the strongest
apparent hedges — exactly the ones whose path-robustness most matters) and the
print makes any truncation visible and auditable. If Stage 1 yields ≤ 16, the cap
is a no-op and every qualifier is bootstrapped.

**Negative-control + contrast coverage under this predicate (R-BS.6 / AC-BS.6).**
The predicate is regime-driven, not strategy-driven, so a mean-reverter (RSI /
BBands) CAN qualify if it happened to beat a deeply-negative B&H by ≥ 10 pp in
some 2022 cell — and if it does, it MUST score FRAGILE/MARGINAL in Stage 2 (a
no-edge mean-reverter scoring ROBUST on a bear sample is a RED flag → escalate,
do not pass). For the **up-market contrast** (AC-BS.6's discrimination check,
carried from overfit-guard AVAX·2023 SMA): because the predicate's down-market
gate **excludes** all bull cells, the harness ADDITIONALLY scores ONE fixed
up-market contrast cell outside the predicate — **SMA on a 2021 full-year cell
for a symbol whose 2021 B&H was strongly positive** (developer picks the
highest-2021-B&H symbol from the Stage-1 table at runtime, deterministically; BTC
or ETH 2021 is the expected pick). This contrast cell is bootstrapped + scored
alongside the candidates and clearly labelled `(up-market contrast)`, exactly as
the overfit-guard ran AVAX·2023 for SMA only. It does NOT count against the cap.

### D-BS.3 — Frozen knobs transfer AS-IS (Q-BS.5 noted, not pre-empted)

- **N=500** (R-BS.4) — the § 0 bands were calibrated at N=500; do NOT reduce.
- **`BlockLengthPolicy::Auto`** (Politis–White) — frozen default. **Q-BS.5:** run
  `Auto` first and OBSERVE. The harness already WARNs when
  `selected_block_length ≤ 1` (copied from overfit-guard
  [`:303-314`](../../crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs)).
  If `Auto` degenerates to L=1 on a 2021-22 series, the developer **logs it and
  surfaces it as a finding** (a degenerate L=1 = i.i.d. resampling, which
  understates path dependence) — do NOT silently pin `Fixed`. The architect
  decides on a `Fixed` fallback ONLY after seeing a real degeneration, per the
  overfit-guard D-OG.3 precedent. Not pre-empted.
- **Seeds (ADR-0051 D1):** `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`;
  a **DISTINCT `ensemble_seed` per (strategy × candidate-cell)**; the **CONSTANT**
  `ScenarioConfig.seed = SEED` (`0xC0FFEE…`) for every path (orthogonality —
  per-path variation lives ONLY in the bootstrap `path_seed_j`, NEVER the engine
  fill-tie-break seed). Because the candidate set is data-derived (not a fixed
  matrix), `ensemble_seed_for(strat_idx, candidate_rank)` derives the per-ensemble
  seed deterministically from a base `0x00C0_FFEE_0000_0000` + `strat_idx*0x100`
  + `candidate_rank` (rank within the deterministic-sorted candidate list). Since
  the candidate list order is deterministic (D-BS.2 tie-break), the seeds are
  reproducible run-to-run. **Determinism caveat (inherited):** byte-identity is
  contracted on the Apple-Silicon canonical box only (ADR-0051 D5 / ADR-0043);
  cross-platform parity is not contracted and does not affect the percentiles.

### D-BS.4 — UN-ANCHORED; baseline-divergence gate N/A; no new ADR (Q-BS.3)

- **UN-ANCHORED (LOCKED).** `#[ignore]`, no `spec/*/reports/` file, **no
  `anchors.toml` row** — per the § Anchoring justification (one-shot epistemic
  stress-test; the § 0 ruler + the C1 generator are the already-anchored
  contracts; determinism via fixed seeds gives byte-reproducibility without an
  anchor). The `--nocapture` stdout + the analyst `findings` dev-note are the
  deliverable.
- **Escalation hook (reopen path).** IF Stage 2 returns a ROBUST survivor AND the
  operator greenlights a trend-following product line, THEN the v0.2.0 follow-on
  anchors a canonical report per ADR-0051 § D6. Explicitly deferred; NOT triggered
  by a null result.
- **Baseline-equity-divergence e2e gate — N/A on substance (NOT rubber-stamped).**
  Per § "The CLAUDE.md baseline-equity-divergence e2e gate is N/A" and the
  overfit-guard D-OG.6 precedent: this harness introduces **no overlay and no
  sizing modifier**. It runs the four already-shipped strategy ids unchanged
  through the already-shipped `run_scenario` over (Stage 1) real bars and
  (Stage 2) bootstrap-resampled bars and reduces the output — there is **no new
  decision variable that could silently fail to wire**. The equivalent
  correctness tripwires for an analysis harness are **AC-BS.5 (two-run
  byte-identical determinism)** + **AC-BS.6 (a no-edge mean-reverter must NOT come
  back ROBUST)**. Gate N/A on substance.
- **No new ADR (LOCKED).** This is a test harness that *reuses* three already-ratified
  contracts unchanged: ADR-0051 (D1 seeds + the reducer shape), the frozen § 0
  decision rule, and ADR-0056 (the `data/binance-2122/` corpus layout). It
  introduces no new cross-cutting decision, mutates none of the 9 anchor SHAs in
  `spec/anchors.toml`, and adds no dependency. An ADR would be ceremony with no
  durable decision to record. `arch` trace column points to ADR-0051 + the § 0
  rule note + ADR-0056 (see Handoff).

## Backtest Scenarios
_Stage 1: 10 symbols × {2021, 2022} × 4 strategies single-path vs B&H over
`data/binance-2122/`. Stage 2: N=500 block-bootstrap per Stage-1 apparent winner,
scored against the frozen § 0 rule. Both UN-ANCHORED; fixed seeds for determinism;
no `anchors.toml` row._

## Implementation

ONE new `#[ignore]` harness created at
`crates/backtest/tests/realdata_simple_strategy_bear_survey.rs`.

Stage-1 primitives (`workspace_root`, `load_year_bars`, `buy_and_hold_pct`) copied
from `realdata_simple_strategy_survey.rs` with corpus root → `data/binance-2122/` and
years → 2021/2022 UTC boundaries.

Stage-2 primitives (`run_one_path`, `path_metrics_from_report`, `score_verdict`,
`run_ensemble_from_bars`) copied from `realdata_simple_strategy_overfit_guard.rs`.
`Stage1Cell` struct and `select_candidates` implement the frozen D-BS.2 predicate
(`bh_pct < 0 AND margin >= 10 pp`, cap 16, deterministic tie-break).
`ensemble_seed_for(strat_idx, candidate_rank)` implements the D-BS.3 seed derivation.
Up-market contrast cell uses a reserved rank `0xF0` (no cap collision).

Both shipped harnesses verified byte-untouched (`git diff` = empty).

**Stage-1 result (2026-06-15):** 40 qualifying cells before cap; top-16 bootstrapped.
All 16 candidates + contrast cell results:
- All 16 candidates: **FRAGILE** (sharpe.p5 < 0 on every cell).
- Up-market contrast (SOLUSDT·2021 SMA): **MARGINAL** (p5=0.439, positive median).
- Negative control (RSI/BBands in candidates): all FRAGILE — harness is correctly
  calibrated (mean-reverters did NOT score ROBUST).
- `Auto` block length: 200–210 bars across all cells (no L≤1 degeneration — Q-BS.5).

**Headline:** The 2021-22 bear sample FIRMS ship-passive. 16/16 candidates FRAGILE.
The 2026-06-08 terminal verdict stands.

Build: `cargo clippy --tests -p backtest -- -D warnings` clean.
Default suite: `cargo test -p backtest --tests` green (harness is `#[ignore]`d).
Determinism (AC-BS.5): two consecutive runs produce byte-identical table rows.
spec-lint: 70 (zero new findings).

## Verification
_tester links to reports here (note: UN-ANCHORED — the deliverable is the
`--nocapture` stdout + the analyst `findings` dev-note, not an anchored report)._

## Changelog

- 2026-06-15 (developer): T-BS.5–.9 complete. Created
  `crates/backtest/tests/realdata_simple_strategy_bear_survey.rs` (381 lines).
  Stage-1: 80-cell survey over data/binance-2122/ — 40 qualifying cells before cap,
  top-16 bootstrapped. Stage-2: all 16 candidates FRAGILE (sharpe.p5 < 0 on every
  cell); up-market contrast (SOLUSDT·2021 SMA) MARGINAL; Auto block length 200–210
  (no degeneration). Headline: 2021-22 bear sample FIRMS ship-passive.
  Clippy clean; default suite green; determinism verified; spec-lint 70.
  Two shipped harnesses byte-untouched. HANDOFF → tester.
- 2026-06-15 (architect): M-T1 design lock. Resolved Q-BS.1–.5 and wrote § Design
  (D-BS.1–.4). **Q-BS.1 seam → option (a)**: ONE new combined `#[ignore]` harness
  `crates/backtest/tests/realdata_simple_strategy_bear_survey.rs` that COPIES the
  survey's Stage-1 primitives + the overfit-guard's Stage-2 primitives (both are
  test-local privates — copy, not extract; extract rejected as it would put both
  shipped harnesses' behavior at risk for no decision benefit) with the corpus
  root → `data/binance-2122/` and years → 2021/22; the two shipped harnesses stay
  byte-untouched. Specified the exact harness shape (`Stage1Cell` struct +
  in-code `select_candidates` hand-off + Stage-2 loop). **Q-BS.2 candidate
  predicate + cap → PRE-REGISTERED/FROZEN**: apparent winner iff `B&H < 0` AND
  `strat_ret − B&H ≥ 10 pp` (down-market gate + 10-pp margin; X=10 justified
  against the 2024 motivating margins of ≈13/26 pp); **cap N≤16 by margin DESC**
  with a deterministic `(margin,symbol,year,strat_idx)` tie-break (16×500 ≈ ~140 s,
  under the analyst ~20 ceiling, over the 2-min watch threshold). Added a fixed
  out-of-predicate up-market contrast cell (SMA on the highest-2021-B&H symbol)
  for the AC-BS.6 discrimination check. **Q-BS.3 → UN-ANCHORED** confirmed;
  baseline-divergence e2e gate **N/A on substance** (no overlay/sizing-modifier;
  determinism + negative-control are the tripwires); **NO new ADR** (reuses
  ADR-0051 D1 + § 0 rule + ADR-0056 unchanged; touches no anchor SHA). **Q-BS.5
  block-length**: run `Auto` first, log + surface (do NOT pre-empt) if it
  degenerates to L=1. Verified all seam names against source (`RunReport.equity_series`,
  bootstrap API, `DistributionSummary::from_path_metrics`, ADR-0051 D1 seed string,
  corpus on disk). spec-lint confirmed 70 before handoff.
- 2026-06-15 (analyst): v0.1.0 draft. Scoped a **two-stage** bear-market re-run of
  the 4 simple strategies over the just-shipped 2021-22 corpus (`data/binance-2122/`,
  pin `4f390622`): Stage 1 = 80-cell point survey vs B&H (cheap, 1 path each) →
  apparent-winner candidate set; Stage 2 = N=500 block-bootstrap path-robustness
  guard on the candidates ONLY, scored against the **frozen § 0 rule AS-IS**.
  Headline: does ANY simple strategy show a path-robust edge in a real deep bear,
  or does it firm ship-passive? Framed as a STRESS-TEST of the 2026-06-08 terminal
  verdict (with humility), generalising the 2026-06-15 overfit-guard's
  survey-identifies→bootstrap-guards shape from 2 hand-picked cells to a corpus-wide
  automated pipeline. UN-ANCHORED (survey + overfit-guard precedent; `#[ignore]`,
  fixed seeds, no `anchors.toml` row). Baseline-divergence e2e gate ruled **N/A on
  substance** (analysis tooling, no overlay/sizing-modifier; determinism +
  negative-control are the tripwire). Stated the honest SCOPE CAP (hourly, default
  params, 10 large-caps, 2 specific bear years — null firms ship-passive but does
  NOT prove no strategy can ever work; a ROBUST survivor REOPENs the question — the
  high-value tail). Flagged the Stage-2 watch-recipe need (market-wide-bear
  candidate count may push the release run > 2 min). Riskiest open questions:
  Q-BS.1 (the Stage-1→Stage-2 seam — one new combined harness (a, Recommended) vs
  parameterize the 2 existing `data/binance`-hardcoded harnesses (b)) and Q-BS.2
  (the candidate-selection predicate + cap, locked BEFORE the run). Created
  `[[req]]` row REQ-SIMPLE-STRATEGY-BEAR-SURVEY-001 (proposed). R-BS.1–13 /
  AC-BS.1–9 / Q-BS.1–5. HANDOFF → architect.
