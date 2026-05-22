---
slug: v3-volatility-forecaster-rebaseline
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-22
parent: v3-volatility-forecaster
parent_version: 0.1.0
parent_disposition: shipped-with-MODEL-BROKEN-NO-ALPHA-advisory
---

# v3 volatility forecaster — re-baseline pass

> **Spawned by operator routing pick (b) RE-BASELINE FIRST**, ratified
> 2026-05-22 on the parent feature's presenter deck
> (`spec/v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md`).
> Tightly scoped 1-day re-baseline pass that disambiguates the
> synthetic-vs-real data caveat in the parent's joint advisory verdict
> **V3 × T-VOL-NO-ALPHA → MODEL-BROKEN / NO-ALPHA** before the operator
> commits multi-week budget to (a) RETIRE-C1, (c) DEBUG-V3, or (d) v0.1.1
> GARCH refit.

## Why

The parent v3-volatility-forecaster v0.1.0 shipped with a load-bearing
data caveat: its
[`reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`](../v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md)
compares an **un-targeted v1 momentum baseline on SYNTHETIC GBM bars**
against the **GARCH vol-targeting overlay on REAL Binance hourly OHLCV**.
The Sharpe baseline value (`-0.026770`) and the net delta (`+0.029868`,
landing on the T-VOL-NO-ALPHA side of the +0.05 floor) are both
data-mismatch contaminated.

Per the developer's open-question note at the parent's tester handoff:
"Baseline comparison uses synthetic v1 momentum data while vol-target
overlay uses real Binance data — the sharpe report notes this
explicitly." The presenter elevated this to a deck-level decision and
the operator chose **(b) RE-BASELINE FIRST** — the highest-EV play
before the multi-week routes. The V3 model-broken classification
(`mean_calibration_ratio = 2.952191` outside `[0.7, 1.4]`, driven by
GARCH non-convergence on AVAX/DOGE/DOT) is a **GARCH-only diagnostic**
and is independent of the baseline; this re-baseline addresses ONLY the
T-classifier half of the joint verdict.

The four possible re-baseline outcomes route the next-feature decision
deterministically — see § Routes below.

## Scope

ONE deliverable: a new anchored Sharpe / drawdown comparison report
that swaps the synthetic v1 momentum baseline for a real-data
un-targeted v1 momentum baseline, recomputes the net Sharpe delta, and
re-evaluates the ADR-0038 § D1.c T-classifier gate. The overlay scenario
stays byte-identical to the parent (anchor `top10-2023-fy-vol-target-overlay-realdata`
already locked under `[v3.0.0-volatility]` at body-SHA `66cd69ad…`).

Default deliverable path (Q3 default):

```
spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md
```

Anchored under a NEW `[v3.0.0-volatility-rebaseline]` namespace block in
`spec/anchors.toml` (+1 anchor; the existing 3 `[v3.0.0-volatility]`
anchors stay byte-immutable per ADR-0038 § D6 anchor-additive contract).

## Out of scope

- **No model retrain.** GARCH(1,1) MLE checkpoints in
  `crates/forecast/src/garch.rs` and the JSON manifest stay byte-identical.
- **No GARCH hyperparameter search.** That's route (d) — out of scope
  here.
- **No per-symbol calibration debug.** That's route (c) — handled by
  the queued-but-not-spawned `v3-garch-calibration-tune` follow-on.
- **No new strategy variant.** Only the baseline scenario changes; the
  vol-target overlay is reused verbatim.
- **No verdict-shape ADR amendment.** ADR-0038 § D1.c (T-classifier) and
  § D6 (anchor-additive) carry forward unchanged.
- **No risk-engine integration work.** Parent's R-risk-engine deferral
  still holds.

## Investigation findings (analyst — embedded for architect lift-and-shift)

Quoted briefly so the architect does not have to re-grep the source
tree at M-T1:

1. **What baseline the existing sharpe-comparison report uses.** The
   `--scenario vol-target-bs1` dispatch path in
   `crates/forecast/src/bin/sharpe_comparison.rs:1284-1352` re-runs
   exactly two scenarios in a hard-coded array at line 1292-1295:
   ```rust
   let vol_target_scenarios = [
       "top10-2023-1h-momentum",
       "top10-2023-fy-vol-target-overlay-realdata",
   ];
   ```
   The baseline name is also embedded as a string literal in
   `render_vol_target::render_report` at lines 975 ("Baseline scenario
   | top10-2023-1h-momentum (v1 cross-sectional momentum, synthetic)"),
   1049 ("Sharpe baseline | {…} (top10-2023-1h-momentum)"), and 1082
   ("Baseline (top10-2023-1h-momentum) uses synthetic GBM bars; overlay
   uses real Binance 2023 data"). All three sites are advisory text
   the developer must update at the same time.

2. **Does a real-data un-targeted v1 momentum scenario exist?** **No.**
   `Scenario::from_name` in `crates/backtest/src/main.rs:283-321` only
   registers two v1 momentum scenarios — `top10-2023-1h-momentum` (line
   284) and `top10-2024-h1-momentum` (line 303) — and both have
   `data_source: ScenarioDataSource::Synthetic` (lines 300, 319) and
   `expected_revision_sha: None`. The five `-realdata` scenarios at
   lines 450-592 (4 TCN-overlay variants + 1 PatchTST + 1 vol-target)
   are all **overlay** variants; no un-targeted realdata momentum
   scenario is registered.

3. **Is there an anchored real-data momentum report we can reuse?**
   **No.** Every `backtest-…-momentum*.md` report under
   `spec/backtest-real-binance-data/reports/` is an `-overlay-realdata`
   variant (TCN or PatchTST), not un-targeted momentum.
   `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md`
   compares passthrough-overlay against real-weights-overlay — both
   sides are overlay variants; neither is the un-targeted v1 baseline
   we need. ADR-0038 § D7 (line 431) references
   `spec/backtest-real-binance-data/reports/backtest-…-top10-2023-1h-momentum-realdata.md`
   as the un-targeted v1 baseline, but **that file does not exist in
   the repository** — it was an aspirational anchor in the ADR text.

Net: the cheapest path (anchored-report-reuse) is **closed**. The
developer MUST add a new realdata scenario. The change is additive —
~25 LoC mirroring the existing `top10-2023-fy-tcn-overlay-realdata`
shape at `crates/backtest/src/main.rs:450-475` minus the overlay
strategy + forecaster_id (use `ScenarioStrategy::Momentum { config_id:
"top10_momentum_h1" }` per the existing synthetic scenario at lines
292-294, `data_source: ScenarioDataSource::RealData`, and the same
pinned `expected_revision_sha` 3a8b96c43f… that the parent vol-target
scenario uses). The sharpe_comparison.rs hard-coded array swap +
3 string-literal updates is a follow-on ~10 LoC patch in the same wave.

Wall-clock estimate: scenario add ~10 min + sharpe_comparison.rs swap
~10 min + backtest re-run for the new scenario ~40s + sharpe-comparison
re-run ~10s + 2-run byte-identity determinism check ~25s + anchor
emission + report write ~5 min. End-to-end <2 hours of developer time,
~1 day with architect M-T1 + tester verification + presenter pass.

## Requirements

R1–R5 are tight per the 1-day scope; this is not a 7-day feature.

- **R1 — REAL-data baseline scenario.** The new sharpe-comparison
  report's baseline column MUST be a real-Binance-data un-targeted v1
  momentum scenario (NOT synthetic GBM). The overlay column stays
  byte-identical to the parent's
  `top10-2023-fy-vol-target-overlay-realdata` scenario. Acceptance:
  report frontmatter + § Methodology table both name the real-data
  baseline scenario and assert `data_source = real` for both columns.

- **R2 — Baseline provenance pinned per ADR-0032.** The new baseline
  scenario MUST set `data_source: ScenarioDataSource::RealData` and
  `expected_revision_sha: Some("3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into())`
  in `Scenario::from_name`, matching the parent vol-target realdata
  scenario's revision pin exactly. Bar interval = 1h, universe = 10
  symbols (ADAUSDT / AVAXUSDT / BNBUSDT / BTCUSDT / DOGEUSDT / DOTUSDT
  / ETHUSDT / LINKUSDT / SOLUSDT / XRPUSDT), span = 2023-01-01T00:00:00Z
  .. 2024-01-01T00:00:00Z (UTC, half-open), `bar_count = 8760`,
  `initial_capital = $100_000.00`, `slippage_bps = 2`, `taker_fee_bps =
  4`, `seed = 0xC0FFEE`. Acceptance: backtest report frontmatter shows
  the same `data_revision_sha = 3a8b96c43f…` as the parent overlay
  report and the same Span / Loaded bars block.

- **R3 — T-classifier re-evaluated on new net_delta.** The new
  sharpe-comparison report MUST re-compute the T-classifier verdict
  per ADR-0038 § D1.c against the new `net_delta = sharpe_overlay -
  sharpe_real_baseline`. The threshold grid stays unchanged (>=+0.10 →
  T-VOL-ALPHA-UNLOCKED, [+0.05,+0.10) → T-VOL-MARGINAL, <+0.05 →
  T-VOL-NO-ALPHA). The V-verdict line MUST also be re-emitted but
  carries forward the parent's V3 (mean_calibration_ratio = 2.952191)
  finding verbatim — the calibration ratio is a GARCH-only diagnostic
  and is independent of the baseline. Acceptance: § Verdict block
  shows all five rows (Sharpe baseline / Sharpe overlay / Gross delta
  / Net delta / T-classifier / V-verdict joint) populated with the
  recomputed values.

- **R4 — Anchor-additive through report emission.** The new report
  body MUST be anchored under a NEW
  `[v3.0.0-volatility-rebaseline]` namespace block in
  `spec/anchors.toml`. The three existing `[v3.0.0-volatility]` anchors
  (`vol-verdict-bs1-realdata` / `top10-2023-fy-vol-target-overlay-realdata`
  / `sharpe-comparison-vol-target-bs1-realdata`) stay byte-immutable
  per ADR-0038 § D6. Net anchor count: **+1 at M-FINAL** (33 → 34
  PASS). The new backtest report for the un-targeted baseline scenario
  is optionally anchored under the same namespace if the operator
  chooses to lock it for future re-baseline runs (Q-anchor below).
  Acceptance: tester anchor-lock report shows 34/34 PASS (or 35/35 if
  the operator opts in on the baseline backtest anchor).

- **R5 — 2-run byte-identity determinism.** The new sharpe-comparison
  report (and the new baseline backtest report) MUST pass the
  determinism contract carried forward from the parent's R11.9 /
  R11.10: two consecutive runs from a clean tempdir produce
  byte-identical bodies (timestamps and host-specific frontmatter rows
  are normalized per the existing `hash_report.py` rules).
  Acceptance: orchestrator-verified `hash_report.py` body-SHA-256
  matches across both runs; tester M-FINAL re-runs once for the
  PASS-record.

## Risks

- **K-rebase-1 — anchored-report-reuse impossible (CONFIRMED at
  analyst pass).** Per § Investigation findings #3, no anchored
  un-targeted v1 momentum report on real data exists anywhere in
  `spec/`. The developer MUST add a new
  `top10-2023-fy-momentum-realdata` scenario. Mitigation: this is a
  ~25 LoC additive change mirroring the existing `-realdata` scenario
  pattern; ~40s wall-clock to backtest; no design churn. Time impact
  on the 1-day budget: <2 hours of developer time including the
  sharpe_comparison.rs swap.

- **K-rebase-2 — re-baseline still lands T-VOL-NO-ALPHA.** Possible
  if the real-data un-targeted baseline Sharpe is sufficiently close
  to the synthetic Sharpe (-0.026770) that the net_delta stays below
  +0.05. The operator's next-feature decision narrows to **(a) RETIRE
  C1** vs **(c) DEBUG V3** — V3 still fires independently regardless
  of T-classifier outcome. Routing implication: presenter's next deck
  recommends (a) under the "real-vs-real NO-ALPHA confirmed"
  argument.

- **K-rebase-3 — re-baseline flips to T-VOL-MARGINAL or
  T-VOL-ALPHA-UNLOCKED.** Possible if the un-targeted real-data v1
  baseline performs sufficiently worse than synthetic (recall: the
  synthetic baseline's Sharpe is `-0.026770` and total return is
  `-43.72%`, which is already a poor showing). If real-data
  un-targeted momentum is even worse, the overlay's `+13.48%` real-
  data total return becomes a relatively bigger lift and net_delta
  rises. Routing implication: the operator's next-feature decision
  narrows to (d) v0.1.1 GARCH refit (under T-VOL-MARGINAL — close but
  needs a polish) OR (c) DEBUG V3 still warranted under
  T-VOL-ALPHA-UNLOCKED (V3 calibration ratio still outside the
  envelope; fix calibration before banking the alpha). This route
  would also re-open the v0.1.0 retirement question — the parent
  feature's "shipped with MODEL-BROKEN-NO-ALPHA advisory" disposition
  would need a 2026-05-23+ amendment.

- **K-rebase-4 — determinism failure on the new realdata baseline
  scenario.** Theoretical only; the existing `-realdata` scenarios
  are all 2-run byte-identical and the new one mirrors their shape.
  Mitigation: tester runs the standard `hash_report.py` 2-run check
  at M-FINAL; failure here would block the ship and route back to
  developer for a determinism fix (one-iteration debugging within
  the 1-day budget).

## Hypotheses

- **H-rebase-1 — real-vs-real comparison will show SOME net_delta
  movement.** Magnitude unknown; direction unknown. The synthetic GBM
  baseline and the real Binance baseline are different stochastic
  processes (drift, volatility clustering, jump component, regime
  shifts), so the un-targeted v1 momentum Sharpe MUST differ between
  them. Whether the delta moves enough to flip the T-classifier
  threshold is the load-bearing empirical question this re-baseline
  answers. Prior probability of T-classifier flip: MEDIUM — synthetic
  GBM is well-behaved (constant volatility, no fat tails); the real-
  data baseline is likely to underperform, which would expand the
  overlay-vs-baseline delta upward. The presenter's framing in the
  parent deck §
  "Operator routing options" implicitly carries this MEDIUM prior.

- **H-rebase-2 — V3 verdict survives independently.** GARCH
  calibration ratios are derived from realized-vs-predicted variance
  on the SAME real-data scenario (vol-verdict-bs1-realdata report);
  they do NOT depend on what synthetic-vs-real choice we make for the
  un-targeted momentum baseline. So even if H-rebase-1 confirms (net
  delta moves) and the T-classifier flips, the V3
  `mean_calibration_ratio = 2.952191` finding holds verbatim and the
  joint verdict cannot evaporate to "MODEL-WORKING / ALPHA-UNLOCKED"
  on this single pass. Routing implication: a T-classifier flip
  triggers (c) DEBUG V3, NOT a clean alpha-banking — the next deck
  must call this out explicitly so the operator does not
  over-interpret a flipped T-classifier as full vindication.

## Routes (deterministic next-feature mapping)

The four possible re-baseline outcomes (cross-product of T-classifier
× determinism gate) pre-draw the routing tree the presenter inherits.
**Standing Autoapprove** from the operator's prior session applies to
Q1–Q3 defaults; route selection here is the operator's
next-decision-after-(b)-completes.

| Outcome | T-classifier on new net_delta | Determinism | Routing implication | Next feature |
|---------|------------------------------|-------------|---------------------|--------------|
| **R-O1** | T-VOL-NO-ALPHA (`net_delta < +0.05`) | PASS | Confirms parent's advisory verdict on real-vs-real evidence; the synthetic-vs-real caveat does NOT save C1. | **(a) RETIRE C1** — promote C2 (`v3-regime-classifier`) or C5 (`v3-llm-forecaster`) per the parent deck's HYBRID sequencing. |
| **R-O2** | T-VOL-MARGINAL (`+0.05 <= net_delta < +0.10`) | PASS | Re-baseline rescues the alpha signal partially; V3 still fires (model-broken under MLE non-convergence on AVAX/DOGE/DOT). | **(d) v0.1.1 GARCH refit + return** — keep the workspace structure, iterate hyperparameters, single-iteration ship-or-skip. ~2-3 days. |
| **R-O3** | T-VOL-ALPHA-UNLOCKED (`net_delta >= +0.10`) | PASS | Real-vs-real comparison vindicates the alpha hypothesis H2 from the parent feature; V3 still fires and MUST be fixed before banking the alpha into a live signal. | **(c) DEBUG V3** — spawn `v3-garch-calibration-tune` for per-symbol hyperparameter search (ω, α, β ranges; `max_iters > 500`; tighter convergence tol; Garman-Klass fallback for non-convergent symbols). ~2-3 weeks. Re-opens v0.1.0 retirement discussion. |
| **R-O4** | (any) | FAIL | Determinism contract broken on the new baseline scenario; cannot ship. | Route back to **developer** for a single-iteration determinism fix within the 1-day budget. If iteration overflows, escalate to operator-decide on (a) vs extend-budget. |

The presenter's next deck inherits this 4-row routing table; the
operator's decision after re-baseline ships is mechanical given the
verdict cell.

## Operator-decide questions (Q1..Q3)

Three operator-decide rows kept tight per the 1-day scope. **Standing
Autoapprove** from the operator's 2026-05-22 prior session applies to
the analyst-recommended defaults; the orchestrator may tick all three
automatically before spawning the architect.

### Q1 — Which real-data baseline scenario?

| Option | Action | Analyst recommendation |
|--------|--------|------------------------|
| **(a)** | Reuse a closest-existing anchored realdata momentum scenario from another feature folder. | **REJECTED** by § Investigation findings #3 — no such scenario or anchored report exists. |
| **(b)** | Introduce a new `top10-2023-fy-momentum-realdata` scenario in `Scenario::from_name` (mirrors existing `-realdata` pattern, ~25 LoC additive, no refactor). | **DEFAULT** — the only feasible path. Architect locks the exact name + shape at M-T1. |

**Default: (b).**

### Q2 — Anchor naming for the new report.

| Option | Anchor scenario name | Namespace | N_new |
|--------|---------------------|-----------|-------|
| **(a)** | `sharpe-comparison-vol-target-bs1-realbaseline` (clear "realbaseline" suffix distinguishes from the parent's `…-realdata` anchor name) | `[v3.0.0-volatility-rebaseline]` (NEW namespace block in `spec/anchors.toml`; mirrors `[v3.0.0-volatility]` shape) | +1 (just the sharpe-comparison report) |
| **(b)** | Same as (a) PLUS anchor the new `top10-2023-fy-momentum-realdata` backtest body. | `[v3.0.0-volatility-rebaseline]` | +2 |
| **(c)** | Anchor the sharpe-comparison under the existing `[v3.0.0-volatility]` namespace (re-uses parent's namespace, no new block). | `[v3.0.0-volatility]` | +1 |

**Default: (a).** Net `+1` anchor at M-FINAL; new namespace signals the
re-baseline pass as a distinct anchor cohort from the parent v0.1.0
ship. Anchor-additive contract holds (existing 3 `[v3.0.0-volatility]`
anchors stay byte-immutable per ADR-0038 § D6).

### Q3 — Where does the deliverable land?

| Option | Path | Rationale |
|--------|------|-----------|
| **(a)** | `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md` | Keeps the new pass in its own feature folder; clean separation from the parent's `[v3.0.0-volatility]` evidence cohort. |
| **(b)** | Append to `spec/v3-volatility-forecaster/reports/`. | Reuses the parent's folder; risks confusing future readers about which sharpe-comparison is authoritative. |

**Default: (a).**

## References

- Parent feature brief: [`spec/v3-volatility-forecaster/feature.md`](../v3-volatility-forecaster/feature.md) — § Verification carries the joint advisory verdict + data caveat.
- Parent presenter deck: [`spec/v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md`](../v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md) — routing pick (b) ticked.
- Contaminated sharpe-comparison report: [`spec/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md`](../v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md).
- Anchored vol-target overlay backtest (reused verbatim as overlay column): [`spec/v3-volatility-forecaster/reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md`](../v3-volatility-forecaster/reports/backtest-20260522-082914-top10-2023-fy-vol-target-overlay-realdata.md).
- Sharpe-comparison bin (CLI to extend): `crates/forecast/src/bin/sharpe_comparison.rs:1284-1352` (vol-target dispatch) + lines 975 / 1049 / 1082 (advisory string literals).
- Scenario registry (where the new baseline scenario lands): `crates/backtest/src/main.rs:283-321` (synthetic v1 momentum reference shape) + `crates/backtest/src/main.rs:450-475` (existing `-realdata` overlay reference shape).
- T-classifier threshold grid: [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md) § D1.c.
- Anchor-additive contract: ADR-0038 § D6 + parent feature Q5=(a).
- Realdata revision pin (ADR-0032): `data_revision_sha = 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`.

## Verification

_tester links to reports here after M-FINAL_

## Implementation

Developer completed Waves A + B + C on 2026-05-22.

### Wave A — New realdata baseline scenario

Added `top10-2023-fy-momentum-realdata` to `Scenario::from_name`
(`crates/backtest/src/main.rs`, before line 546, alphabetical placement).
Extended `MomentumScenarioInput` with `bars_override: Option<Vec<Bar>>`
and `data_revision_sha: Option<String>` (`crates/backtest/src/cli_types.rs:44`).
Updated `momentum::run` to use `bars_override` when provided
(`crates/backtest/src/scenarios/momentum.rs:200`). Extended `is_momentum`
dispatch in `main.rs` to load Binance parquet data for `RealData` scenarios.
Updated `report::momentum::write` to include `data_revision_sha` in frontmatter.
Added scenario to `scenario_to_feature` routing table.

Report emitted: `spec/v3-volatility-forecaster-rebaseline/reports/backtest-20260522-095222-top10-2023-fy-momentum-realdata.md`.
Confirmed `data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`.

### Wave B — Extend sharpe_comparison.rs

Added `ScenarioFamily::VolTargetRebaseline` variant
(`crates/forecast/src/bin/sharpe_comparison.rs:59`). Added out-dir match arm
(line 1247-1249). Added new dispatch arm `if args.scenario == VolTargetRebaseline`
(inserted before `VolTarget` arm, line ~1288). Added `render_vol_target_rebaseline`
sibling module (~250 LoC; no shared-extract needed; parent `render_vol_target` is
byte-identical per anchor-additive contract ADR-0038 § D6). Three advisory string
swaps per decomp.md § T-AR-2 lock. Parent anchor
`ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` confirmed
byte-identical after wave.

### Wave C — End-to-end results

- Report: `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md`
- T-classifier: **T-VOL-NO-ALPHA** (net_delta < 0.05)
- Body-SHA256 (canonical, 2-run byte-identical): `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`
- All 4 hygiene gates pass: fmt / clippy / 311 tests / ANCHORS PASS (33 / 33)

### Architecture deviation note

The decomp.md § T-AR-1 estimated ~25 LoC for Wave A. Actual diff was ~100 LoC
across 4 files because the `Momentum` strategy dispatch had no real-data path;
the `MomentumScenarioInput` needed two new fields; `momentum::run` needed
`bars_override` support; and `report::momentum::write` needed `data_revision_sha`
frontmatter emission. This is additive-only; no existing behavior was mutated.

## Changelog

- 2026-05-22 (analyst): brief authored at v0.1.0 / status=proposed.
  Investigation findings embedded (Q1=(b) forced — no
  anchored-report-reuse path; Q2=(a) default; Q3=(a) default). Q1–Q3
  carry analyst-recommended defaults; standing Autoapprove applies.
  HANDOFF → operator-decide (Q1..Q3) → architect M-T1.
- 2026-05-22 (developer): Waves A + B + C complete. T-D-N1..T-D-N12
  ticked. T-classifier = T-VOL-NO-ALPHA. 2-run body-SHA256 =
  d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8.
  33 parent anchors byte-identical confirmed. HANDOFF → tester M-FINAL.
