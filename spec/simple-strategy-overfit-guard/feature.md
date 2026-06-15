---
slug: simple-strategy-overfit-guard
version: 0.1.0
status: presenter-done
owner: architect
priority: P2
updated: 2026-06-15
---

# Simple-strategy overfit / robustness guard — is the down-market trend-following "hedge" real or 2-case noise? — v0.1.0

> **The single load-bearing question.** The 2026-06-14 real-data survey
> ([`realdata-simple-strategy-survey-2026-06-13.md`](../dev-notes/realdata-simple-strategy-survey-2026-06-13.md))
> found that in the **2 of 20** (symbol·year) cases where buy-and-hold LOST money,
> trend-following protected capital: **AVAX 2024 B&H −8.2% → SMA +5.0% / MACD +6.1%**,
> **DOT 2024 B&H −19.6% → SMA +6.4%**. In the other 18/20 (all up-markets) passive
> crushed every active strategy. The survey itself flags this honestly: *"Two
> down-market data points is suggestive of the trend-following protection property,
> not statistically conclusive."* This feature answers — with a **pre-registered,
> path-level robustness test** — whether that down-market hedge is a **real,
> repeatable property of the strategy** or an **artifact of the one exact 2024 AVAX/DOT
> price ordering** (one lucky path).
>
> **Why this is a *robustness* question, not a *parameter-overfit* question.** The
> survey ran **shipped default configs, no tuning** (SMA 20/50, textbook MACD). There
> was no parameter search, so there is no parameter-overfitting to deflate. The threat
> model is narrower and sharper: **small-sample path fragility** — does the −8.2%→+5.0%
> protection survive when we resample plausibly-different orderings of the *same*
> AVAX-2024 down-market, or does it collapse to a coin-flip? That is precisely the
> question the **concluded block-bootstrap Monte-Carlo harness** + the **frozen § 0
> decision rule (p5 Sharpe < 0 → FRAGILE)** were built to answer
> ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0).
>
> **Either answer is decision-grade.** If the hedge is ROBUST (p5 Sharpe ≥ 0 on the
> down-market resamples) → the survey's "trend-following is a defensible downside
> hedge" claim is upgraded from *suggestive* to *evidenced*, and the product has its
> first non-passive niche worth a follow-on. If FRAGILE → the honest conclusion is
> "ship passive, full stop; the 2-case hedge was sampling noise" and the active-strategy
> down-market story is closed on this data. Neither outcome adds a live-trading path
> (analysis tooling only).

---

## 0. Method decision — block-bootstrap-on-real-data (NOT CPCV, NOT Deflated Sharpe)

The backlog C5 item ([`backlog.md`](../backlog.md) "C5 — CPCV / Deflated-Sharpe
overfit guard") bundles three candidate methods. They answer **different questions**;
the survey's down-market-hedge question selects exactly one. Decided here, justified:

| Method | What it perturbs / answers | Fit to THIS question | Verdict |
|---|---|---|---|
| **CPCV** (Combinatorial Purged Cross-Validation, López de Prado) | Perturbs the **train/test partition** → Probability of Backtest Overfitting (PBO). Answers *"did my parameter/model selection overfit the in-sample split?"* | The survey did **no parameter selection** (shipped defaults). There is no IS→OOS selection step to compute PBO over. CPCV would also need **new infra** (purge/embargo partitioner, in/out-of-sample scorer) that does not exist. | **Reject** — wrong threat model (no selection), new infra. |
| **Deflated Sharpe Ratio** (DSR, Bailey–López de Prado) | Deflates a Sharpe for the **number of trials** that produced it (multiple-testing inflation). Answers *"is this Sharpe significant after N strategies were tried?"* | The survey tried 4 strategies × 20 cells, but the **down-market hedge is a 2-cell observation**, not a max-over-trials selection. DSR deflates a *winner-take-best*; we are not claiming a best, we are claiming a *down-market property*. DSR is **complementary, defer-able**, and needs the trials-count machinery (new). | **Defer** — orthogonal, not the binding question; candidate for a v0.2.0 follow-on if a tuned variant is ever proposed. |
| **Block-bootstrap-on-real-data** (Politis–Romano stationary bootstrap) | Perturbs the **path ordering** of the real returns → distribution of Sharpe / prob-of-loss / max-DD over plausibly-different histories of the SAME market. Answers *"is the result a property of the strategy or of the one path ordering?"* | **Exactly the question.** The hedge claim is "SMA protected on the AVAX-2024 *path*." Bootstrap asks "would it protect on resampled AVAX-2024 paths?" **Reuses the concluded C1 generator** (`BlockBootstrapPathGen`) + the **frozen § 0 rule** + the `BinanceCache` realdata loader the survey already uses. **Zero new statistical infra.** | **CHOOSE** ✅ |

**Decision: block-bootstrap-on-real-data, scored against the frozen § 0 decision
rule.** It is the only method that (a) answers the path-fragility question the survey
actually raised, and (b) reuses shipped infra end-to-end. CPCV and DSR are correct
tools for a *parameter-tuned* variant — out of scope here precisely because the survey
deliberately ran untuned defaults. (Architect: if you disagree, the § Open questions
Q-OG.1 is the place to escalate — but note CPCV/DSR both cost new infra AND answer a
question the survey did not pose.)

---

## 1. Existing infrastructure this reuses (file:line — the architect's map)

Everything below is **shipped and concluded**. This feature is a thin re-targeting of
it onto the 4 survey strategies in the 2 down-market cells — no new engine, no new
statistics, no new path generator.

| Component | File:line | Reuse |
|---|---|---|
| **Block-bootstrap path generator (C1)** | [`crates/data/src/synth/bootstrap.rs:71`](../../crates/data/src/synth/bootstrap.rs) (`BlockBootstrapPathGen`), `:194` (`MonteCarloPathGen::generate`) | Strategy-agnostic: takes `universe: &[(Symbol, Decimal)]` + source bars → `GeneratedPath`. **Single-symbol mode = a 1-entry universe** (the survey's mode). Politis–White auto block length at `:251`. |
| **Stationary-bootstrap math** | [`crates/data/src/synth/bootstrap.rs:258`](../../crates/data/src/synth/bootstrap.rs) (index draw), `block_length::politis_white_block_length` | Geometric blocks, mean L, circular wrap. Already anchor-grade. |
| **Frozen § 0 decision rule** | [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0; encoded thresholds in [`feature.md` of horizon-retest](../horizon-retest-robustness/feature.md) lines 65–67 | **p5 Sharpe < 0 → FRAGILE**; prob-of-loss > 35% → FRAGILE; p95 MaxDD > ~70% → FRAGILE; composite = worst band. Pre-registered ruler — applied AS-IS. |
| **Metric reducer (Sharpe/DD/prob-loss percentiles)** | [`crates/backtest/src/stats/mod.rs:283`](../../crates/backtest/src/stats/mod.rs) (`MetricDistribution`, `EnsembleSummary::from_path_metrics:365`) | Consumes per-path equity → p5/p25/p50/p75/p95, `prob_sharpe_gt_0`, `max_dd_tail_p50`. **Decimal equity in, f64 stats out** — the existing contract. |
| **Per-metric stats (Decimal equity → f64)** | [`crates/backtest/src/stats/mod.rs:40`](../../crates/backtest/src/stats/mod.rs) (`compute_sharpe_hourly`), `:238` (`compute_max_drawdown_f64`) | Reused verbatim. Note: **hourly** Sharpe (corpus is 1h) — matches the survey's hourly bars. |
| **Real-data corpus loader** | [`realdata_simple_strategy_survey.rs:55`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs) (`load_year_bars` via `data::ReplayFeed` over `data/binance/`) | The exact `BinanceCache` path + UTC year-boundary windowing the survey used. **Copy the loader, do not reinvent.** SKIPs cleanly when `data/binance/` absent. |
| **Simple-strategy runner** | [`realdata_simple_strategy_survey.rs:85`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs) (`run_strategy` → `engine::run_scenario` with `ScenarioDataSource::BinanceCache` + `bars_override`) | The survey runs the 4 strategies (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`) as `ComposedStrategy` ids through `run_scenario`. **This is the seam** — see § 2. |
| **ADR-0051 (MC determinism + anchoring)** | [`spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md`](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md) | D1 seed orthogonality: `path_seed_j = ensemble_seed.wrapping_add(j·0x9E3779B9)`, fill_seed CONSTANT `0xC0FFEE`. **Followed AS-IS** for determinism; anchoring deferred (§ 3). |

### The one architectural seam (the only non-trivial wiring)

The concluded `monte_carlo.rs` binary wires `BlockBootstrapPathGen` **only to
`MomentumStrategy`** via `montecarlo::run_path` (cross-sectional, top-K). The survey's
**4 simple strategies are `ComposedStrategy` ids run through `engine::run_scenario`**,
NOT `MomentumStrategy`. So the ensemble loop here must:

1. Generate N paths with `BlockBootstrapPathGen` (1-symbol universe = AVAX, then DOT) — **reused**.
2. Feed each `GeneratedPath`'s bars to `engine::run_scenario(..., bars_override: Some(path_bars), data_source: BinanceCache, strategy: <survey id>)` — **the survey's existing runner path** — to get a per-path equity series.
3. Reduce the N equity series with `stats::EnsembleSummary::from_path_metrics` — **reused**.

Steps 1 and 3 are pure reuse. Step 2 reuses the survey's `run_strategy` shape but
loops it over bootstrap paths instead of the one real path. **No new engine code.** The
open architectural question (Q-OG.2) is whether to (a) extend `monte_carlo.rs` to accept
a ComposedStrategy id, or (b) add a small dedicated `#[ignore]` harness mirroring the
survey — see § Open questions for the recommendation.

---

## 2. What this concretely computes (the deliverable)

For each of the **2 confirmed down-market cells** (AVAX·2024, DOT·2024) and each of the
**4 survey strategies** (SMA, MACD, RSI, BBands) — i.e. **8 strategy·cell ensembles** —
plus **buy-and-hold as the same-path benchmark**:

- Load the real 2024 hourly bars for the symbol (`load_year_bars`, the survey's loader).
- Generate **N = 500 stationary-bootstrap paths** of that one real year (`BlockBootstrapPathGen`, 1-symbol universe, Politis–White auto block length, ADR-0051 D1 seeds).
- Run the strategy on each path → 500 equity curves → `EnsembleSummary`:
  - **Sharpe p5 / p25 / p50 / p75 / p95** (hourly Sharpe per `stats`).
  - **prob-of-loss** = `P(final_equity < initial)`.
  - **max-drawdown tail p50 + p95**.
  - **P(Sharpe > 0)**.
- Score each ensemble against the **frozen § 0 bands** → `ROBUST | MARGINAL | FRAGILE`
  (composite = worst band; **p5 Sharpe < 0 ⇒ FRAGILE**).
- **The headline answer:** does SMA (and MACD) on AVAX·2024 + DOT·2024 land **p5 Sharpe ≥ 0
  AND prob-of-loss ≤ 15%**? If yes → the down-market hedge is **path-robust** (real). If
  the p5 tail dips negative → it was **one lucky ordering** (noise). RSI/BBands are
  expected FRAGILE everywhere (consistent with the survey's "no edge anywhere") — they
  are the **negative control** that validates the test discriminates.

**Control / falsification:** also run the **up-market cells the survey says passive
dominates** as the contrast is not required for the core question, BUT at minimum run
**one up-market AVAX·2023 SMA ensemble** as a sanity contrast — if the test cannot tell
the −8.2%→+5.0% down-market apart from a bull-market churn case, the harness is
miscalibrated (this is the § 6 small-N latitude check, mirrored from horizon-retest).

---

## 3. Anchoring decision — UN-ANCHORED ad-hoc, mirroring the survey precedent

**Default: UN-ANCHORED.** Per the survey precedent (`realdata-simple-strategy-survey`
wrote **no report, touched no `anchors.toml`**) and the task constraint ("UN-ANCHORED
ad-hoc analysis OK — no `anchors.toml` churn"), the deliverable is a **re-runnable
`#[ignore]` harness** whose `--nocapture` stdout IS the finding, plus a `findings`-status
dev-note recording the numbers — **NOT** an anchored `spec/*/reports/` file.

**Justification for not anchoring:**
- The question is a **one-shot epistemic check** ("is the 2-case hedge real?"), not a
  **regression-protected production surface**. The block-bootstrap *generator* is already
  anchored (C1); re-anchoring its *application* to 2 cells adds `anchors.toml` churn for a
  finding that will be cited once and folded into the passive-baseline thesis.
- ADR-0051's anchoring discipline exists for **canonical robustness verdicts that gate
  paper→live**. There is no live path here, and the § 0 ruler is already the anchored
  contract being applied.
- **Determinism is still enforced** (ADR-0051 D1 seeds + constant fill_seed) so the
  `#[ignore]` run is byte-reproducible across machines — the same guarantee an anchor
  would give, without the `anchors.toml` row.

**Escalation hook (Q-OG.3):** IF the architect/operator wants this to be a *permanent
gate* on any future "ship a trend-following overlay" decision (not just a one-shot
finding), THEN it should anchor a canonical report per ADR-0051 § D6. The analyst
recommendation is **un-anchored** for v0.1.0 (matches the survey, lowest churn, answers
the question); anchoring is the durable-but-heavier path deferred to a v0.2.0 IF a
trend-following product line is greenlit off this finding.

---

## 4. Requirements

- **R-OG.1** — Reuse `BlockBootstrapPathGen` ([`bootstrap.rs:71`](../../crates/data/src/synth/bootstrap.rs))
  in **single-symbol mode** (1-entry universe) to generate N=500 stationary-bootstrap
  paths of each down-market cell's real 2024 hourly bars. NO new path generator. Politis–White
  auto block length (`BlockLengthPolicy::Auto`).
- **R-OG.2** — Reuse the survey's real-data loader (`load_year_bars` over `data::ReplayFeed`
  / `data/binance/`) and the survey's `run_scenario` + `bars_override` runner to execute
  each of the 4 strategy ids (`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`) per path.
  NO new engine code. SKIP cleanly when `data/binance/` is absent (mirror survey line 112).
- **R-OG.3** — Reduce the N per-path equity curves with `stats::EnsembleSummary::from_path_metrics`
  ([`stats/mod.rs:365`](../../crates/backtest/src/stats/mod.rs)) → Sharpe p5/p25/p50/p75/p95,
  prob-of-loss, max-DD p50/p95, P(Sharpe>0). NO new statistics.
- **R-OG.4** — Score every strategy·cell ensemble against the **frozen § 0 decision rule**
  ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
  AS-IS: composite = worst band; **p5 Sharpe < 0 ⇒ FRAGILE**. Do NOT re-derive or soften
  the bands (if seen-then-changed, log per that note's changelog discipline).
- **R-OG.5** — Cover the **2 confirmed down-market cells** (AVAX·2024, DOT·2024) for all 4
  strategies (8 ensembles) PLUS **≥1 up-market contrast** (AVAX·2023 SMA) as the
  miscalibration check (§ 2 falsification).
- **R-OG.6** — All money/equity math in `Decimal` / `Money<Usdt>`; **never f64** for equity.
  (f64 is permitted ONLY inside the already-shipped `stats` reducer, which is the existing
  Decimal-in→f64-out contract — do not introduce new f64 equity paths.)
- **R-OG.7** — Determinism per ADR-0051 D1: `path_seed_j = ensemble_seed.wrapping_add(j·0x9E3779B9)`,
  ensemble seed `0xC0FFEE…`, fill_seed CONSTANT `0xC0FFEE`. Two runs ⇒ byte-identical summary.
- **R-OG.8** — UN-ANCHORED (`#[ignore]`, no report file, no `anchors.toml` row) per § 3.
  The `--nocapture` stdout + a `findings` dev-note are the deliverable. NO baseline-divergence
  e2e gate (analysis tooling, not a strategy overlay — CLAUDE.md gate N/A, stated explicitly).
- **R-OG.9** — NO live-trading path, NO order-execution surface beyond what the survey's
  `run_scenario` already exercises in paper mode (operator constraint, 2026-06-12).
- **R-OG.10** — `scripts/spec_lint.py` MUST stay at ≤ 70 findings with **zero new** findings
  attributable to this feature's files (baseline measured 70 at `0436d08`).

## 5. Acceptance criteria

- **AC-OG.1** — Running the harness (`cargo test … -- --ignored --nocapture`) on a machine
  WITH the `data/binance/` corpus prints, for each of the 8 down-market ensembles + the
  up-market contrast: N, Sharpe p5/p25/p50/p75/p95, prob-of-loss, max-DD p50/p95, P(Sharpe>0),
  and the § 0 composite verdict (ROBUST/MARGINAL/FRAGILE).
- **AC-OG.2** — On a machine WITHOUT the corpus, the harness SKIPs cleanly (prints a SKIP
  line, exits green) — never fabricates synthetic data, never fails the default suite
  (it is `#[ignore]`).
- **AC-OG.3** — Two consecutive runs produce **byte-identical** summary numbers (R-OG.7
  determinism), demonstrable by diffing two `--nocapture` captures.
- **AC-OG.4** — The **negative control passes**: RSI and BBands on the down-market cells
  score FRAGILE (or at best MARGINAL) — i.e. the test does NOT spuriously bless the
  no-edge mean-reverters. (If RSI/BBands come back ROBUST, the harness is miscalibrated —
  RED flag, escalate.)
- **AC-OG.5** — The **headline question is answered in writing**: a `findings`-status
  dev-note (`spec/dev-notes/analysis-<date>-simple-strategy-overfit-guard.md` — written by
  the analyst/orchestrator AFTER the run, NOT a `spec/*/reports/` anchored file) states,
  with the actual p5 Sharpe + prob-of-loss numbers, whether the down-market trend-following
  hedge is **path-robust (real)** or **fragile (one lucky path)**.
- **AC-OG.6** — `scripts/spec_lint.py` ≤ 70, zero new findings (R-OG.10). `cargo clippy
  -- -D warnings` clean on the new harness. No `.unwrap()` outside `#[cfg(test)]`.

## 6. Open questions (for the architect)

- **Q-OG.1 (method)** — Is block-bootstrap-on-real-data the right method vs CPCV/DSR for
  the *down-market-hedge* question? **Recommended default: YES, block-bootstrap** (§ 0
  table — it is the only one answering path-fragility AND reusing shipped infra; CPCV/DSR
  answer parameter/multiple-testing questions the survey did not pose, and both need new
  infra). Escalate here only if the architect sees a selection step I missed.
- **Q-OG.2 (the seam — riskiest)** — How to run the **ComposedStrategy survey ids** through
  the bootstrap ensemble, given `monte_carlo.rs` wires the generator only to
  `MomentumStrategy`? **Two options:**
  - **(a) Add a small dedicated `#[ignore]` harness** under `crates/backtest/tests/`
    mirroring `realdata_simple_strategy_survey.rs`, looping `run_scenario` over
    `BlockBootstrapPathGen` paths. ~120–180 LoC, self-contained, zero risk to the anchored
    `monte_carlo.rs` binary and its § 0 reports. **(Recommended)** — it is the *durable*
    choice: it keeps the concluded `monte_carlo.rs` anchor-grade binary byte-untouched (no
    regression-anchor risk), composes the two shipped pieces (generator + survey runner) at
    the test layer where the survey already lives, and does not entangle a one-shot finding
    harness with the production MC binary's CLI surface. Slightly more typing than (b) but
    spawns no follow-on cleanup.
  - **(b) Extend `monte_carlo.rs`** to accept a `--strategy <composed-id>` and dispatch to
    `run_scenario` instead of `run_path`. ~40–60 LoC in the binary BUT touches the
    anchor-grade MC binary, risks the § 0 report determinism surface, and couples a one-shot
    finding to the production CLI. **Fallback only if the operator wants a single unified MC
    entry point** — note it adds an audit obligation on every future `monte_carlo.rs` change
    and a possible ADR-0051 re-emission if any anchored report body shifts.
  - **If budget tightens:** option (a) is *also* the cheaper-to-get-right path here (it
    cannot break an existing anchor), so there is no quick-vs-durable tension — (a) wins on
    both axes. The only reason to pick (b) is an explicit operator preference for one MC binary.
- **Q-OG.3 (anchoring)** — UN-ANCHORED (§ 3) for v0.1.0? **Recommended default: YES,
  un-anchored** (matches the survey precedent, lowest `anchors.toml` churn, the § 0 ruler is
  already the anchored contract). Anchor a canonical report ONLY if the operator greenlights
  a trend-following *product line* off this finding (then it becomes a paper→live-adjacent
  gate → ADR-0051 § D6) — deferred to a v0.2.0.
- **Q-OG.4 (N and block length)** — N=500 paths (the C1/Q-RH-1 ratified default) and
  Politis–White auto block length? **Recommended default: YES** — same N and policy the
  concluded harness used, so the § 0 bands (calibrated at N=500) transfer AS-IS. A smaller N
  would widen the percentiles and is NOT a valid economy here (it would change what FRAGILE
  means). Fixed-L is a fallback only if Auto degenerates on the 2024 down-market series.
- **Q-OG.5 (small-N latitude)** — The down-market sample is **2 symbols**. Even a ROBUST
  per-symbol bootstrap verdict is a statement about *path-resampling within each symbol's
  2024*, NOT cross-sectional generality. Should the finding explicitly cap its claim at
  "path-robust within AVAX-2024 and DOT-2024 individually" rather than "trend-following
  hedges down-markets in general"? **Recommended default: YES, cap the claim** (the honest §
  6 small-N latitude — a wider down-market universe is the v0.2.0 follow-on, not this ship).

## Design (architect — 2026-06-15)

Verdict: **ACCEPT the analyst's method and recommendations, with ONE load-bearing
correction to the reducer API.** This is thin test-layer glue over shipped,
anchor-grade components; no new ADR is warranted (ADR-0051 already governs MC
determinism + seeds; the § 0 note is the frozen ruler). `arch` trace column →
[`adr/0051`](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)
+ [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0.

### D-OG.0 — THE CORRECTION the developer MUST internalise (reducer API)

The feature brief (§ 1, § 2, R-OG.3) and tasks name **`stats::EnsembleSummary::from_path_metrics`**.
**That symbol does not exist.** Verified against `crates/backtest/src/stats/mod.rs`:

- The reducer is **`DistributionSummary::from_path_metrics(metrics: &[PathMetrics]) -> Result<Self, DistributionError>`** (`stats/mod.rs:365`).
- It consumes a **`&[PathMetrics]`** where each `PathMetrics` (`:336`) is `{ sharpe, sortino, calmar, max_drawdown, total_return: f64, final_equity, initial_equity: Decimal }` — i.e. **per-path scalars the harness must compute itself**, NOT raw equity curves.
- It emits `DistributionSummary` (`:307`): `sharpe/sortino/calmar/max_drawdown/total_return: MetricDistribution` (each with `p5/p25/p50/p75/p95/mean/std/min/max`), plus `prob_loss`, `prob_sharpe_gt_0`, `prob_sharpe_gt_1`, `max_dd_tail_p50`, `max_dd_tail_p95`.

So the brief's "feed each equity series to `EnsembleSummary`" is one layer too coarse.
The real shape: **harness computes `PathMetrics` per path from that path's equity
curve using the already-shipped `compute_*` helpers, collects `Vec<PathMetrics>`,
then calls `DistributionSummary::from_path_metrics`.** No new stats — every helper exists:
`compute_sharpe_hourly`, `compute_sortino_hourly`, `compute_calmar`,
`compute_max_drawdown_f64`, `compute_total_return` (all `&[Decimal] -> f64`, `stats/mod.rs:40–277`).
`from_path_metrics` requires **index-order** input (ADR-0051 D2) — push in `j = 0..N`
ascending; do NOT sort.

### D-OG.1 — Q-OG.2 RESOLVED: dedicated `#[ignore]` harness (option a), verified single-symbol

**Decision: option (a) — a new dedicated `#[ignore]` harness at**
**`crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs`.** Rationale: it
composes the two shipped pieces at the test layer where the survey already lives,
keeps the anchor-grade `bin/monte_carlo.rs` + `scenarios/montecarlo.rs::run_path`
binary **byte-untouched** (zero § 0 anchor-report risk), and — confirmed — is also the
cheaper-to-get-right path (it cannot break an existing anchor). Option (b) is rejected:
it would couple a one-shot finding to the production MC CLI and incur an ADR-0051
re-emission obligation on any anchored-report drift, for no benefit.

**The seam is buildable end-to-end with NO new engine/generator/stats** (each link verified against source):

1. **Generate (reuse `BlockBootstrapPathGen`, single-symbol).** `BlockBootstrapPathGen::new(vec![(sym, real_bars)], BlockLengthPolicy::Auto)` then `.generate(&[(sym, start_price)], n_bars, path_seed_j)`. **Single-symbol mode is NOT an edge case — it is a directly-tested path:** `bootstrap.rs` unit tests `fp_c1_2`, `fp_c1_3` (lag-1 acf), `fp_c1_4` (moment preservation), and `auto_block_length_is_some` all drive a **1-entry universe** `vec![(btc(), dec!(30_000))]` and pass. The `RaggedUniverse` guard is vacuous for one symbol; Politis–White `Auto` block length runs on the single series' absolute-return series at `:251`. `n_bars` = the real bars' length (`source_len()`); `start_price` = `real_bars[0].close`.
2. **Run (reuse the survey's `run_scenario` + `bars_override`).** For each path, take `generated.bars_by_symbol[0].clone()` (the single symbol's bars) and pass it as `ScenarioConfig.bars_override = Some(path_bars)` with `data_source: BinanceCache`, `strategy: StrategyId(id)`, `range: DateRange::Last30d` (ignored when `bars_override` is `Some`), `write_report: false`. This is the survey's `run_strategy` shape **verbatim** — but it must NOT discard the result: `run_scenario` returns **`RunReport`** (`engine.rs:268`) which carries **`equity_series: Vec<(Timestamp, Money<Usdt>)>`** (`:271`). The survey throws this away (`.map(|r| r.kpis)`); the guard harness KEEPS it.
3. **Per-path metrics (reuse `compute_*`).** Map `report.equity_series.iter().map(|(_, m)| m.amount()).collect::<Vec<Decimal>>()` → one `Vec<Decimal>` equity curve, then build `PathMetrics { sharpe: compute_sharpe_hourly(&eq), sortino: compute_sortino_hourly(&eq), calmar: compute_calmar(&eq), max_drawdown: compute_max_drawdown_f64(&eq), total_return: compute_total_return(&eq), final_equity: report.kpis.final_equity.amount(), initial_equity: report.kpis.initial_equity.amount() }`. **All money stays `Decimal`/`Money<Usdt>` until the `compute_*` boundary (R-OG.6 honoured — f64 only inside the shipped stats layer).**
4. **Reduce (reuse `DistributionSummary`).** `DistributionSummary::from_path_metrics(&metrics)` → the percentiles + prob-of-loss + DD tail (D-OG.0).
5. **Score (reuse § 0 bands AS-IS).** Map the `DistributionSummary` fields to the § 0 read: `sharpe.p5 < 0 ⇒ FRAGILE`; `prob_loss > 0.35 ⇒ FRAGILE`; `max_dd_tail_p95 > 0.70 ⇒ FRAGILE`; composite = worst band (§ 0 step 3). Print one verdict line per ensemble.

**Harness signature sketch (the developer's contract):**
```rust
// crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs
#[tokio::test]
#[ignore]
async fn realdata_simple_strategy_overfit_guard() { /* see tasks.md T-OG.5–.8 */ }

// helpers (copy load_year_bars from the survey verbatim):
async fn load_year_bars(root: &Path, sym: &Symbol, start_ms: u64, end_ms: u64) -> Vec<Bar>; // survey:55
// ONE path → equity curve → PathMetrics (the new glue):
async fn run_one_path(sym: &Symbol, strat: &str, path_bars: Vec<Bar>) -> Option<stats::PathMetrics>;
// ONE ensemble (strategy × cell): N=500 paths → DistributionSummary:
async fn run_ensemble(root:&Path, sym:&Symbol, year:(u64,u64), strat:&str, ensemble_seed:u64)
    -> Option<stats::DistributionSummary>;
```
`run_ensemble` loads the real year bars ONCE, constructs ONE `BlockBootstrapPathGen`,
loops `j in 0..500` deriving `path_seed_j` (D-OG.3), and collects `PathMetrics` in
index order. `stats` is `backtest::stats` (the harness is in `crates/backtest/tests/`,
so `use backtest::stats::{self, PathMetrics, DistributionSummary, compute_sharpe_hourly, …}`).

### D-OG.2 — Q-OG.1 RESOLVED: block-bootstrap (no selection step exists)

**ACCEPT.** The survey ran shipped defaults with zero parameter search → there is no
IS/OOS selection partition for CPCV/PBO to score and no max-over-trials for DSR to
deflate. Block-bootstrap is the only method that answers the path-fragility question
the survey actually raised AND reuses shipped infra. CPCV/DSR are correct for a
*tuned* variant (out of scope, § 7). No escalation.

### D-OG.3 — Q-OG.4 RESOLVED: N=500, Politis–White `Auto`, ADR-0051 D1 seeds AS-IS

**ACCEPT, transfer unchanged so § 0 bands apply without recalibration.**
- `N = 500` (the C1/Q-RH-1 ratified default the § 0 bands were calibrated at). A smaller N widens the percentiles and changes what FRAGILE *means* — not a valid economy.
- `BlockLengthPolicy::Auto` (Politis–White). Fixed-L is a fallback ONLY if `Auto` degenerates to `L=1` (i.i.d.) on a 2024 series — if observed, log it and pin `Fixed` with the value, but do not pre-empt.
- Seeds per ADR-0051 D1: `ensemble_seed` is a distinct `u64` per (strategy × cell) ensemble; `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`. **One caveat the developer must respect:** the survey's `SEED` is a `[u8;32]` for `run_scenario`'s fill-tie-break; ADR-0051 holds that **constant `0xC0FFEE`** across all paths (orthogonality — measure path variance only, not path⊕fill noise). So `ScenarioConfig.seed` is the SAME constant `[0xC0,0xFF,0xEE,…]` for every path; the **per-path variation lives entirely in the bootstrap `path_seed_j`** that selects the bars, NOT in the engine seed. Do not vary `ScenarioConfig.seed` per path.

### D-OG.4 — Q-OG.3 RESOLVED: UN-ANCHORED for v0.1.0

**ACCEPT.** `#[ignore]`, no `spec/*/reports/` file, **no `anchors.toml` row** (§ 3).
The `--nocapture` stdout + the analyst's post-run `findings` dev-note are the deliverable.
Determinism (D-OG.3) gives byte-reproducibility without an anchor. **Escalation hook
(carried to the deck):** if the operator greenlights a trend-following *product line*
off this finding, it becomes a paper-adjacent gate and SHOULD anchor a canonical report
per ADR-0051 § D6 — that is a v0.2.0 decision, explicitly deferred.

### D-OG.5 — Q-OG.5 RESOLVED: cap the claim at per-symbol path-robustness

**ACCEPT.** Even a ROBUST verdict is a statement about *resampling within AVAX-2024 and
DOT-2024 individually* — 2 symbols is not cross-sectional generality. The analyst's
closing dev-note (T-OG.13) MUST scope the claim to "path-robust within AVAX-2024 /
DOT-2024 individually," NOT "trend-following hedges down-markets in general." A wider
down-market universe is the v0.2.0 follow-on (§ 7).

### D-OG.6 — CLAUDE.md baseline-divergence e2e gate is N/A (justified, not rubber-stamped)

The CLAUDE.md non-negotiable "every strategy overlay or sizing-modifier ships with a
day-1 baseline-equity-divergence e2e test" exists to catch a **no-op overlay** (a
`scale` computed but never applied to equity — the v3-vol-overlay precedent). **This
feature introduces no overlay and no sizing modifier.** It is *read-only analysis
tooling*: it runs the four ALREADY-SHIPPED survey strategy ids unchanged through the
ALREADY-SHIPPED `run_scenario` engine over bootstrap-resampled bars, and reduces the
output. There is no new decision variable that could silently fail to wire — the
strategies' equity-affecting code is the production path, untouched. The applicable
correctness guard is instead **AC-OG.3 (two-run byte-identical determinism)** + **AC-OG.4
(the negative control: RSI/BBands MUST land FRAGILE/MARGINAL, proving the test
discriminates and is not blessing no-edge churn)** — that pair is the harness's
no-op/miscalibration tripwire, the moral equivalent of the divergence gate for an
analysis harness. Gate stated N/A on substance, not skipped.

## 7. Out of scope (v0.1.0)

- **CPCV / PBO / Deflated Sharpe** — deferred (§ 0); correct tools for a *parameter-tuned*
  variant, which the survey deliberately is not. A future "tune SMA per-symbol" feature
  would pull these in.
- **Cross-sectional / wider down-market universe** — the 2-symbol sample is the data we
  have; widening it (more 2024 down-market names, other bear years) is the v0.2.0 follow-on
  Q-OG.5 names, not this ship.
- **Live / paper-forward execution** — operator constraint (2026-06-12). Pure offline analysis.
- **Anchored canonical report** — deferred to a trend-following product greenlight (§ 3 / Q-OG.3).
- **Re-deriving or re-calibrating the § 0 bands** — frozen; applied AS-IS (R-OG.4).

---

## Changelog

- 2026-06-15 (architect): added § Design (D-OG.0–6). Resolved all five open questions
  (ACCEPT analyst recs on Q-OG.1/3/4/5; Q-OG.2 → dedicated `#[ignore]` harness, option a).
  **Load-bearing correction D-OG.0:** the named reducer `EnsembleSummary::from_path_metrics`
  does NOT exist — the real API is `DistributionSummary::from_path_metrics(&[PathMetrics])`,
  and the harness must assemble `PathMetrics` per path from `RunReport.equity_series` via the
  shipped `compute_sharpe_hourly`/`compute_sortino_hourly`/`compute_calmar`/`compute_max_drawdown_f64`/
  `compute_total_return` helpers. Verified the seam end-to-end: `run_scenario` returns `RunReport`
  carrying `equity_series` (engine.rs:271), single-symbol `BlockBootstrapPathGen` is a directly-tested
  path (bootstrap.rs `fp_c1_2/3/4`, `auto_block_length_is_some` all use 1-entry universes). No new ADR
  (ADR-0051 + § 0 ruler govern); no architecture.md change; no anchors.toml row. Baseline-divergence e2e
  gate ruled N/A on substance (D-OG.6 — no overlay/modifier; AC-OG.3+AC-OG.4 are the miscalibration tripwire).
  HANDOFF → developer.
- 2026-06-15 (analyst): v0.1.0 draft. Scoped the robustness/overfit guard for the
  2026-06-14 survey's down-market trend-following hedge finding. Chose
  **block-bootstrap-on-real-data** over CPCV/DSR (§ 0 — the only method answering the
  path-fragility question AND reusing shipped C1 + survey infra). Mapped the existing
  infra file:line (§ 1), identified the single architectural seam (ComposedStrategy ids
  vs `monte_carlo.rs`'s `MomentumStrategy`-only wiring, § 1 + Q-OG.2), and pre-registered
  the § 0 scoring AS-IS. UN-ANCHORED per the survey precedent (§ 3). R-OG.1–10 / AC-OG.1–6 /
  Q-OG.1–5. Created `[[req]]` row REQ-SIMPLE-STRATEGY-OVERFIT-GUARD-001 (proposed). HANDOFF → architect.
