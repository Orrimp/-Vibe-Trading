---
slug: v25-tcn-alpha-investigation
status: in-progress
owner: architect
updated: 2026-05-18
version: 0.1.0
predecessor: backtest-real-binance-data v0.1.0
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 — TCN alpha-verdict investigation

> Forensic, read-only investigation into the `dampened=0` finding that
> persists even after `backtest-real-binance-data v0.1.0` wired the real
> Binance hourly OHLCV pipeline into the backtest harness
> ([test report](../backtest-real-binance-data/reports/test-20260518-1800-backtest-real-binance-data.md)).
> Scope is **read-only**: we do not re-train, do not change the model,
> do not move existing anchors. We instrument what the BS-1 and BS-2
> checkpoints actually emit on real OHLCV and produce the alpha-verdict
> comparison report that the M3 deck promised but could not deliver.

## Why

Four scenarios from `backtest-real-binance-data` (commit `df73780`)
report `dampened=0` on real 2023/2024 Binance hourly OHLCV across the
full 10-symbol top-USDT universe (87,590 BS-1 bars; 87,840 BS-2 bars).
That replicates exactly what M3 reported on synthetic GBM data
([BS-1 M3 report](../v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md) § "Finding"),
which previously read as "model is correctly silent on
out-of-distribution data." Real Binance OHLCV is **the training
distribution** — the model should produce non-trivial output. It does
not. That finding falsifies the M3 hypothesis and surfaces a
distinguishable root cause we must read before paying for any fix.

Four candidate root causes (the "4 buckets" from the
[backlog stub](../backlog.md#strategy)):

- **(a) Gating envelope too tight.** ε=0.0005 deadband (R6 of the
  parent feature) and `confidence_threshold=0.6` (D5 of the parent
  feature) may simply be too restrictive. If `r_hat` is widely
  distributed but every value lies inside ε, OR is just-outside ε but
  `|r_hat|/sigma_train < 0.6`, the gate silences a model that has real
  signal.
- **(b) Horizon mismatch.** The TCN predicts next-1h log-return. The
  v1 cross-sectional momentum baseline operates on a 20-bar (20h)
  lookback. A 1h-ahead head over a 20h-horizon signal may simply be
  the wrong horizon for the consuming strategy.
- **(c) Training pathology.** Final val Huber loss
  `1.539e-5` (BS-1) and `1.051e-5` (BS-2) are suspiciously small for
  Huber δ=0.001 on real crypto log-returns whose stdev is well above
  the δ knee. The hypothesis: the model collapsed to "predict ≈ 0"
  because that minimises Huber when most targets are small. Direct
  inspection of held-out forward-pass outputs distinguishes "tight
  prediction" from "near-zero collapse."
- **(d) Sharpe table.** Independent of (a)-(c), the M3 presenter deck
  promised an alpha-verdict deliverable that has not been authored.
  Even with current `dampened=0`, the honest comparison report
  (TCN-overlay vs v1-momentum baseline on the four `-realdata`
  anchors) needs to exist on disk so the operator can read
  "dampened=0 → equity curves identical → Sharpe equal → drawdown
  equal" rather than infer it from the absence of a report.

The investigation cheaply decides which of (b) / (c) — if either — is
actually worth funding by reading (a) + (d) first.

### Quantitative-finance context

Hourly crypto log-returns on the top-10 USDT pairs have an
empirical standard deviation in the range `0.002 - 0.015`
(20-200 bps per hour, varies by symbol and regime) — far above the
parent feature's ε=0.0005 (5 bps) deadband. The TCN literature for
hourly-horizon regression with Huber loss (BKK18 § 4 §
adding-problem & word-cnn benchmarks) does NOT establish a
predicted-stdev floor — Huber regression on heavy-tailed targets is
known to be susceptible to "predict the mean" collapse when (i) the
δ knee is well below the signal std AND (ii) the loss surface around
0 is locally convex. Both conditions hold here. This is not
exotic; this is a known failure mode that the investigation reads
directly.

## Requirements

### R1 — Histogram report family

A new report family lives at
`spec/v25-tcn-alpha-investigation/reports/forecast-distribution-<scenario>-<YYYYMMDD>.md`
carrying, for each of the two checkpoints (BS-1 and BS-2) on the
held-out real-OHLCV span:

- Summary stats over all `r_hat` outputs: `count`, `mean`, `std`,
  `min`, `max`, percentiles `p1/p5/p10/p25/p50/p75/p90/p95/p99`,
  fraction `|r_hat| <= ε` (currently ε=0.0005), fraction
  `|r_hat|/sigma_train >= confidence_threshold` (currently 0.6).
- A coarse ASCII or markdown-table histogram of `r_hat` (≥30 bins
  centred on 0) sufficient for the operator to read distribution
  shape without external tooling. Architect chooses representation
  (text table vs. embedded data block) at T-AR-2.
- A second histogram of `|r_hat|/sigma_train` (the calibrated
  confidence pre-clamp) to show how many bars would gate-survive at
  each candidate threshold τ ∈ {0.1, 0.2, …, 0.9}.

Report frontmatter follows the existing report contract
(`run_id`, `commit`, `verdict`). Body is deterministic given
(checkpoint, real-OHLCV span, code).

- **Operator decision needed?** No — analyst-locks. Architect picks
  histogram representation in design.

### R2 — Anchor scope

The investigation produces **two new anchors at most**, both under a
new version string `v2.6.0-alpha-investigation`:

- `forecast-distribution-bs1-realdata` — body-SHA over the BS-1 forecast-distribution
  report body.
- `forecast-distribution-bs2-realdata` — body-SHA over the BS-2 forecast-distribution
  report body.

The Sharpe-table report from R5 / bucket (d) is **also anchored** if
its body is fully deterministic (same `data/binance/` REVISION.toml,
same seed, same checkpoint). Architect confirms determinism at T-AR-2;
if any per-run field (wall-clock, ISO timestamp at second precision)
leaks into the body, that field MUST move to frontmatter per the
ADR-0032 § D4 precedent before anchor lock.

Naming: anchor file follows the existing `top10-…` family convention
but under the new version `v2.6.0-alpha-investigation` to keep the
investigation cleanly separated from the four `v2.6.0-realdata`
anchors locked by `REQ-BACKTEST-REALDATA-001`.

- **Operator decision needed?** No — analyst-locks scope (≤3 new
  anchors, one new version).

### R3 — Read path: forward-pass inspector

Investigation reads checkpoint outputs via a **new read-only binary**
at `crates/forecast/src/bin/forecast_distribution.rs` (architect may
relocate). The binary:

- Takes `--scenario {bs1|bs2}`, `--year {2023|2024}`,
  `--data-root data/binance/`, `--out spec/v25-tcn-alpha-investigation/reports/`
  as CLI args.
- Loads the matching LFS-tracked anchor checkpoint via the existing
  `AnchorScenario::load_anchor()` API
  (`crates/forecast/src/tcn.rs`, shipped at T-D-13).
- Iterates `windows_for_symbol()`
  (`crates/forecast/src/features.rs`, shipped at T-D-1/T-D-2) for each
  of the 10 USDT symbols over the requested year.
- For each window, runs `TcnForecaster::forecast()` and records the
  raw `r_hat` (BEFORE direction quantisation and BEFORE
  confidence-clamp) into a per-symbol accumulator.
- Emits the R1 histogram report.

Justification for a new binary rather than extending
`crates/forecast/tests/anchors_load.rs`: the anchors-load test smokes
3 windows per checkpoint to verify load correctness in <10s; this
investigation needs to drive ~87,500 inferences per scenario. A test
suite is the wrong vehicle (wall-clock budgets, no report-writing
plumbing). A bin under `crates/forecast/src/bin/` is exactly the shape
that `train_tcn.rs` already establishes.

Hard contract on the bin: **no mutation** of any
`crates/forecast/checkpoints/anchors/*` file; **no mutation** of any
`spec/anchors.toml` entry; only files written are under
`spec/v25-tcn-alpha-investigation/reports/`.

- **Operator decision needed?** No — analyst-locks the read-only
  contract. Architect picks bin name + module split at T-AR-2.

### R4 — Failure-mode taxonomy & branching paths

The R1 histogram readout deterministically classifies into one of
four cases that drive next-feature decisions. These are the
investigation's **outcomes**; the feature ships when one is reported
honestly.

| Case | Trigger condition (from R1 stats) | What it means | Next direction |
|------|-----------------------------------|---------------|----------------|
| **F1 — Training collapse** | `\|r_hat\|` p95 < 1e-6 across both checkpoints | Model output is numerically zero everywhere; collapsed to "predict ≈ 0" minimiser (bucket (c) confirmed) | Follow-on feature `v25-tcn-retrain` with revised loss (MSE on z-scored returns or quantile head); operator-decide. |
| **F2 — Sigma_train mis-calibration** | `\|r_hat\|` stdev observed in inference > 0.1 × `sigma_train` (large absolute spread) BUT `\|r_hat\|/sigma_train` p99 < 0.6 (no bar survives gate) | sigma_train stored at training time does not match `r_hat` spread at inference; calibration bug | Follow-on feature `v25-tcn-recalibrate` to re-pin sigma_train from a held-out forward pass; cheap (~hours), no re-training. |
| **F3 — Gating too tight** | `\|r_hat\|` p25–p75 straddles ε; `\|r_hat\|/sigma_train` p75 > 0.6; raw signal exists but ε filters it | Real but small-magnitude signal exists | Follow-on feature `v25-tcn-threshold-tuning` to re-anchor with operator-chosen (ε, confidence_threshold) pair, or migrate to a percentile-based gate. |
| **F4 — Model genuinely has no signal at 1h horizon** | Wide `r_hat` distribution survives ε AND survives the 0.6 gate, but the R5 Sharpe table shows no alpha (or worse-than-baseline) | Model emits forecasts but they're directionally wrong / uncorrelated with realised next-bar returns | Follow-on feature `v25-tcn-horizon-bump` (bucket (b)) OR retirement of TCN at v2.6 bake-off — operator-decide. |

The R1 report body MUST emit the case label (F1 / F2 / F3 / F4) in a
dedicated `## Verdict` section so the orchestrator can route without
re-parsing the histogram.

- **Operator decision needed?** No — analyst-locks the four cases.
  Operator decides which follow-on (if any) to fund.

### R5 — Sharpe / drawdown comparison report (bucket (d))

A second report family at
`spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-<YYYYMMDD>.md`
authored over the existing four `-realdata` anchor scenarios:

- Columns: scenario × {v1-baseline (passthrough), TCN-overlay
  (real-weights)} × {final equity, total return, max drawdown, trade
  count, dampen rate, Sharpe annualised, Sortino annualised, Calmar}.
- Rows: `top10-2023-fy-tcn-overlay-realdata`,
  `top10-2024-fy-tcn-overlay-realdata`,
  `top10-2023-fy-tcn-overlay-weights-realdata`,
  `top10-2024-fy-tcn-overlay-weights-realdata`.

Honest reporting contract: if `dampened=0` across the board, the
table shows the equity curves are byte-identical and Sharpe / Sortino
/ Calmar are mechanically equal — that IS the alpha verdict, not an
absence of verdict.

Sharpe / Sortino formulas locked at analyst defaults: hourly returns
annualised by `√(24·365) ≈ 92.6` factor; risk-free rate = 0 (consistent
with v1 momentum reporting precedent). Architect may parameterise but
defaults ship.

The Sharpe-comparison report itself becomes the anchor `sharpe-comparison-realdata`
under `v2.6.0-alpha-investigation` per R2 (subject to determinism check).

- **Operator decision needed?** No — analyst-locks. Annualisation
  factor + zero risk-free are the standard.

### R6 — Anchor neutrality

This feature MUST keep the **19 existing anchors** byte-identical:

- 9 strategy synthetic anchors (pre-v2.5)
- 2 v2.5 TCN passthrough (`top10-2023-fy-tcn-overlay`, `top10-2024-fy-tcn-overlay`)
- 2 v2.5 TCN real-weights (`top10-2023-fy-tcn-overlay-weights`, `top10-2024-fy-tcn-overlay-weights`)
- 4 v2.6.0-realdata (the four `-realdata` scenarios)
- 2 operator-success reports

`bash scripts/verify_anchors.sh` must report `ANCHORS PASS (19/19)` at
M-FINAL — same as the `backtest-real-binance-data` ship. Adding the
new investigation anchors makes the post-ship count 21/21 or 22/22
(depending on whether R5's Sharpe-comparison anchor lands; see R2).

This is the load-bearing non-regression contract. The investigation
is **read-only** — we are looking at the model, not changing it.

- **Operator decision needed?** No — analyst-locks. Tester verifies.

### Operator-decide questions (must answer before architect lock)

**STATUS: RESOLVED 2026-05-18 — operator picked MINIMAL scope** (analyst-
recommended default). Active milestones: M-R-HAT (bucket a) + M-SHARPE
(bucket d). Skipped milestones (move to follow-on features if R4's verdict
demands): M-DIAG (bucket c) + M-HORIZON (bucket b). See Changelog below.
Architect is unblocked.

**Q1 — Scope of THIS feature (the only operator question).** Three
candidate scopes, listed from cheap to expensive:

1. **Minimal (RECOMMENDED).** Cover only (a) + (d) — the histogram
   inspector (R1, R3) and the Sharpe-comparison table (R5). No
   re-training. No checkpoint inspection beyond the histogram. The
   R4 verdict (F1 / F2 / F3 / F4) tells us cheaply whether (b) or (c)
   are even necessary BEFORE we pay to investigate them. Wall-clock
   budget per the `backtest-real-binance-data` tester report ≈ 40s
   per scenario × 2 scenarios = ~80s for the histogram pass + a
   bounded report-authoring step.
2. **Diagnostic.** (a) + (c) + (d). Adds checkpoint-internal
   inspection (held-out forward-pass batches with intermediate-layer
   activation logging) beyond the boundary `r_hat` histogram of (a).
   Distinguishes "model output is all-zero" from "model output is
   small but non-zero" more directly. Still NO re-training. Higher
   signal-per-effort but the boundary-only histogram from minimal
   scope answers the same question for most failure modes.
3. **Full root-cause-and-fix.** (a) + (b) + (c) + (d). Includes
   a horizon-bumped re-training pass (e.g. multi-horizon head
   {1h, 4h, 24h}) under bucket (b). Substantially bigger — best
   estimate 2-3 weeks given the Metal-vs-CPU training-time cost from
   the M3 milestone history (training was ~30 min per checkpoint on
   M-series). Multi-horizon adds heads → multiple training runs.

**Analyst recommendation: Scope 1 (Minimal).** Reasoning:

- Bucket (a) cheaply distinguishes the four failure modes in R4.
  Until we know which one we have, paying for (b) or (c) is
  premature optimisation.
- Bucket (d) is a deliverable the M3 deck promised; authoring it
  closes an open commitment with the operator regardless of which
  failure mode (a) reveals.
- Buckets (b) and (c) are best handled as **separate follow-on
  features** whose scope is decided BY the F1/F2/F3/F4 verdict from
  this investigation. That's the spec-driven idiom: each feature
  ships an answer that unblocks the next decision.

**Default if operator doesn't answer:** ship minimal scope. The
investigation has no destructive cost; if minimal scope is wrong
the diagnostic / full scopes can be opened as separate features
later.

## Backtest Scenarios

**None new.** The investigation reads the existing
`v2.6.0-realdata` anchor scenarios (4 of them) for the Sharpe-comparison
table (R5), and runs the `forecast_distribution` bin (R3) directly
against the LFS-tracked BS-1 / BS-2 checkpoints. No new scenarios are
added to `crates/backtest/src/main.rs`.

If the architect determines at T-AR-2 that R1's histogram readout is
cheaper to emit FROM `crates/backtest` (re-using the
`run_tcn_overlay_weights_backtest()` strategy plumbing) rather than
a standalone bin, the architect may instead extend
`run_tcn_overlay_weights_backtest()` with a `--emit-r-hat-histogram`
side-effect path. That is an architect call at design time, not an
analyst call.

## Risk register

| Risk | Mitigation |
|------|------------|
| **K1 — Investigation lands a "no actionable next step" verdict.** R4's failure-mode table guarantees this can't happen; every case maps to a follow-on feature. BUT: if the operator declines to fund any of (F1→retrain, F2→recalibrate, F3→threshold-tune, F4→horizon-bump or retire), the investigation closes with an explicit "operator declined follow-on" disposition recorded in `feature.md` § Verification. | This feature does not silently leave the v25-tcn-overlay parent feature in `in-progress`; the verdict + operator disposition are part of the ship contract. |
| **K2 — Wall-clock cost of the inspection pass.** `backtest-real-binance-data` tester report measures ~40s per `-weights-realdata` scenario at release. ~87,500 inferences for an unbounded histogram is the same shape — the bin should reuse the existing inference loop, NOT add a parallel one. Estimated total wall-clock for R3 = 2 × ~40s + ~5s report authoring per scenario ≈ 90s. | The inspection runs only on operator-trigger (a binary, not a CI test). M-FINAL anchor-lock step adds a single second run for K5 determinism check (×2 = ~180s total). |
| **K3 — Report-anchor determinism.** Both report families (R1 histogram, R5 Sharpe) need frontmatter-vs-body discipline per the ADR-0032 § D4 precedent (run-varying values in frontmatter, deterministic values in body) before they can be anchored. | Architect verifies in design (T-AR-2); tester confirms by running each report-emitting bin twice and asserting body-SHA equality. Pattern: see `crates/backtest/src/main.rs::write_tcn_overlay_report()` for the existing shape that already works for the four `-realdata` anchors. |
| **K4 — Histogram representation drift.** A coarse text-table histogram has formatting room (column widths, separator chars, locale-sensitive number rendering) that can drift across machines unintentionally. | Architect locks rendering: fixed-width integer counts, fixed-precision floats (e.g. `format!("{:.6}", x)` per ADR-0029 D4 precedent), ASCII-only, LF-only line endings. Same canonicalisation idiom as the metadata-JSON canonicaliser. |
| **K5 — Retraining-implies-this-feature scope creep.** Operator might read "training pathology" and request a re-train inside THIS feature. | Hard analyst boundary: re-training is NOT in scope for any of the three operator-selectable scopes (minimal / diagnostic / full) **as defined here**. Scope 3 (full) includes a horizon-bumped re-train, which is the bucket (b) horizon work; a loss-function-changed re-train under bucket (c) becomes a separate follow-on feature recommended in R4 case F1. |

## Success criteria

Investigation is **done** when ALL three are true:

1. **Verdict published.** The R1 forecast-distribution reports for BS-1
   and BS-2 are on disk, anchored under `v2.6.0-alpha-investigation`,
   carrying an explicit F1 / F2 / F3 / F4 verdict per R4.
2. **Sharpe table published.** The R5 sharpe-comparison report is on
   disk, honestly showing the four `-realdata` scenarios' equity /
   Sharpe / Sortino / drawdown comparison (TCN-overlay vs v1-baseline).
   Whether the verdict is "TCN adds alpha" or "TCN is identical to
   passthrough because dampened=0," the table exists.
3. **Operator disposition recorded.** Based on the F1-F4 verdict, the
   operator either (a) funds a named follow-on feature
   (e.g. `v25-tcn-retrain`, `v25-tcn-recalibrate`,
   `v25-tcn-threshold-tuning`, `v25-tcn-horizon-bump`) or (b)
   explicitly declines to pursue alpha further at v2.5 and pivots to
   v2.5a (PatchTST). Disposition is recorded in
   `feature.md` § Verification + a backlog changelog HTML comment.

The `v25-tcn-overlay` parent feature is **NOT** moved out of
`in-progress` by this investigation. That happens when the alpha
verdict either confirms signal at acceptable threshold OR the
operator pivots to v2.5a. The investigation merely makes the verdict
readable.

## Verification

_tester fills this — verifies R1 + R5 reports anchor cleanly, R6
non-regression contract holds (19 originals byte-identical), bin from
R3 is read-only as advertised, no mutation of trained checkpoint
files. Anchor count at M-FINAL: 19 → 21 (R1 only) or 22 (R1 + R5
Sharpe-table)._

## Sources cited

- [`spec/backtest-real-binance-data/reports/test-20260518-1800-backtest-real-binance-data.md`](../backtest-real-binance-data/reports/test-20260518-1800-backtest-real-binance-data.md)
  — the immediate trigger; `dampened=0` across all four real-data
  scenarios at commit `df73780`; §5 quotes
  `dampened=0` for all four `-realdata` scenarios.
- [`spec/v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md`](../v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md)
  § "Finding: TCN model outputs Flat on synthetic data" — the
  earlier, now-incomplete read of the same phenomenon. BS-1 metadata
  cited: `final_train_loss=1.217e-5`, `final_val_loss=1.539e-5`,
  `sigma_train=10.954`.
- [`spec/v25-tcn-overlay/reports/m3-bs2-training-2026-05-18.md`](../v25-tcn-overlay/reports/m3-bs2-training-2026-05-18.md)
  — same finding on BS-2. BS-2 metadata cited:
  `final_train_loss=8.001e-6`, `final_val_loss=1.051e-5`,
  `sigma_train=6.916`.
- [`spec/v25-tcn-overlay/feature.md`](../v25-tcn-overlay/feature.md)
  § R6 — codifies the ε=0.0005 deadband and
  `confidence = clamp(|r_hat|/sigma_train, 0, 1)` calibration; § D5
  — codifies `confidence_threshold = 0.6`.
- `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`
  and `tcn-bs2-3fabcabe…metadata.json` — verbatim training metadata
  pinned at checkpoint time.
- [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  — TCN checkpoint provenance + canonical-JSON metadata schema
  (anchors `model_revision` and `sigma_train` semantics).
- [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  § D4 — frontmatter-vs-body anchor discipline; the precedent the
  R5 Sharpe-comparison report must follow to be anchorable.
- `crates/forecast/src/features.rs::windows_for_symbol()` — the
  proven real-parquet read used by both training and inference (the
  iterator the R3 inspector bin reuses verbatim).
- `crates/forecast/src/tcn.rs::AnchorScenario::load_anchor()` — the
  shipped anchor-load API (T-D-13) the R3 inspector bin reuses.
- BKK18 (Bai, Kolter, Koltun 2018,
  [arxiv:1803.01271](https://arxiv.org/abs/1803.01271)) — TCN
  empirical evaluation reference; relevant context for what's a
  reasonable hourly-horizon log-return prediction envelope, and for
  the (b) horizon-bump direction if F4 lands.
- The "predict-the-mean" Huber-collapse failure mode is canonical
  in quantitative-finance regression on heavy-tailed targets;
  intuitively: when target std ≫ Huber δ AND target distribution is
  zero-symmetric, the empirical risk minimiser collapses to a
  near-constant near zero. Hypothesis (c) tests for this directly.

## Out of scope

- Re-training the TCN under any altered loss function or horizon —
  that's a separate follow-on feature recommended by R4's verdict.
- Changing ε / `confidence_threshold` defaults in
  `crates/strategy/src/tcn_overlay_momentum.rs` or
  `crates/forecast/src/tcn.rs` — a recommendation that emerges from
  R4 case F3 is itself a separate follow-on feature
  (`v25-tcn-threshold-tuning`).
- Moving / unlocking any of the 19 existing anchors. R6 is the
  load-bearing non-regression contract.
- Comparing TCN against PatchTST or vanilla Transformer (that's v2.6
  bake-off territory, two features downstream).
- Authoring a new architectural ADR. The investigation reads what
  ADR-0029 already pins; it does not amend the provenance schema.

## Changelog

- 2026-05-18 (operator): **Scope-decide Q1 resolved: MINIMAL** (analyst-
  recommended default). Active milestones: M-R-HAT (bucket a, r_hat
  histogram + F1-F4 verdict) + M-SHARPE (bucket d, comparison table vs
  v1 baseline). Skipped milestones (move to follow-on features if R4's
  F-verdict demands): M-DIAG (bucket c, checkpoint internals) and
  M-HORIZON (bucket b, multi-horizon retraining). Status flipped
  `draft → in-progress`. Owner flipped `analyst → architect`. T-OP-1
  ticked in tasks.md.
- 2026-05-18 (analyst): full analyst pass at commit `c43ca56`. Brief
  authored with R1-R6 and four failure-mode taxonomy F1-F4 (R4).
  One operator-decide Q (scope: minimal / diagnostic / full) with
  analyst-recommended **minimal** default. Status: draft. Owner:
  analyst. Predecessor: `backtest-real-binance-data v0.1.0`. Parent:
  `v25-tcn-overlay v2.5.0 (in-progress)`. Trace row
  `REQ-V25-TCN-ALPHA-001` opened in `proposed` state. HANDOFF →
  operator-decide (1 Q) → architect.
