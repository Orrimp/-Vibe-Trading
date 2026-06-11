---
slug: v3-xgboost-cheap-classifier
version: 0.1.0
status: retired
owner: analyst
updated: 2026-06-08
predecessor: spec/dev-notes/post-v3-strategy-direction-2026-05-29.md (Route A Candidate 6 pre-position)
parent: strategy-reformulation-survey-2026-05-22 Candidate 6
priority: P2
sibling_picks:
  - v3-volatility-forecaster (C1; RETIRED 2026-05-22 NEGATIVE-NET-DELTA -0.022)
  - v3-regime-classifier (C2; RETIRED 2026-05-29 NEGATIVE-NET-DELTA -0.294)
  - v3-llm-forecaster (C5; SHIPPED-PARTIAL 2026-05-22 inconclusive)
---

# v3 — XGBoost cheap classifier (low-capacity regime label on hourly OHLCV)

> **Queue pre-position per post-v3 Route A.** Staged at Queue (NOT
> Active) pending operator Route A pick from
> [`spec/dev-notes/post-v3-strategy-direction-2026-05-29.md`](../dev-notes/archive/2026-Q2/post-v3-strategy-direction-2026-05-29.md).
> M0 deliverables (feature.md + tasks.md + trace row + backlog Queue
> entry) authored now so promotion is one-step on operator's call.

> **⛔ FORECLOSED — DO NOT PROMOTE (2026-06-08).** This OHLCV regime-classifier
> lane is foreclosed by the program's terminal verdict (active ≤ passive across
> all reachable channels, *including* OHLCV) — see [`spec/product.md`](../product.md)
> for the terminal verdict and [`spec/dev-notes/onchain-netflow-spike-2026-06-08.md`](../dev-notes/onchain-netflow-spike-2026-06-08.md)
> for the hard-stop. The H1/H2 asymmetric-falsification frame below is moot:
> "edge isn't extractable from hourly OHLCV regardless of model class" (the
> Route-C branch this brief named) is now the *established* result, so a
> low-capacity XGBoost re-test on the same OHLCV substrate cannot change the
> conclusion. Status is `retired` (research-line closure, not deletion — the M0
> brief stays in the tree as authored). This brief was never promoted to Active.

## Why now

The 2026-05-22 strategy-reformulation survey's three-pick set retired
NEGATIVE or INCONCLUSIVE: C1 vol-forecaster -0.022, C2 regime-classifier
-0.294, C5 LLM-forecaster inconclusive. **All three share a model-class
assumption: medium-to-high-capacity supervised/unsupervised learning on
hourly OHLCV** (Markov-switching, GARCH+DL, LLM). Candidate 6 tests the
opposite hypothesis: low-capacity gradient-boosted trees may suit
low-SNR data better by underfitting-by-design.

Asymmetric falsification frame:

- XGBoost ≥ v1 baseline → refutes "edge isn't in fancy model choice".
- XGBoost ≤ baseline like C1/C2/C5 → strengthens "edge isn't
  extractable from hourly OHLCV regardless of model class" →
  Route C (engineer elsewhere) becomes the durable next call.

**Either outcome is information-bearing.** Cost ~4-6 weeks per
[post-v3 dev-note Route A](../dev-notes/archive/2026-Q2/post-v3-strategy-direction-2026-05-29.md);
cheapest remaining orthogonal test on the model-class axis.

## Scope (v0.1.0)

### R1 — XGBoost classifier (low-capacity, 100-200 trees)

100-200 tree gradient-boosted classifier (per spec; not regressor — see
Q1) over engineered features. Output is **REGIME LABEL**
(Bull / Bear / Volatile / Calm — same 4-state as v3 Wave A frozen seam
per operator's 2026-05-28 Q1 lock; preserves K4 byte-identity from v3
Wave B `embedding.rs` slot layout). Feature surface menu (architect
M-T1 picks final set):

- Rolling log-return mean + std (168-bar / 720-bar windows).
- Rolling autocorrelation at lag 1 / lag 24.
- Parkinson HL-vol from `crates/forecast/src/vol.rs` (C1 sibling reuse).
- Hurst exponent over 720-bar window.
- ADX or trend-strength index.

Hyperparameter envelope (architect M-T1 ratifies): `n_trees` 100-200,
`max_depth` 4-6, `learning_rate` 0.05-0.10, `min_child_weight` ≥ 5.

### R2 — `RegimeClassifier` trait reuse (v3 Wave A frozen seam)

XGBoost impl satisfies the SAME `RegimeClassifier` trait signature
from [`crates/forecast/src/markov_switching.rs`](../../crates/forecast/src/markov_switching.rs):

```rust
pub trait RegimeClassifier {
    fn fit(&mut self, log_returns: &[f64]) -> Result<(), RegimeError>;
    fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError>;
}
```

Drop-in replacement of the v3 dispatcher path IF operator ever wants to
re-attempt that integration mode at v0.2.0+. The trait seam was
explicitly frozen at v0.1.0 as "future-compatible for v0.2.0+ alternate
model classes" per the markov_switching.rs module doc.

### R3 — Integration mode: overlay-style multiplier on v1 momentum

Per operator's 2026-05-28 Q4 lock for v3-regime-classifier (= (b)
dispatcher), the durable choice **for XGBoost** is overlay-style
multiplier (different operator-lock per model class): lower blast
radius; smaller scope than reviving v3's retired cash-fallback
architecture; preserves degenerate `Strategy::on_bar` shape; tests
XGBoost classification quality directly without dispatcher
amplification.

Overlay shape (mirrors v3-volatility-forecaster + v2.5 TCN): per-symbol
momentum signal × regime-dependent scalar at strategy → executor handoff.
Bull/Bear = 1.0 (pass through); Volatile/Calm = 0.0 (suppress).
CLAUDE.md non-negotiable: ships with baseline-equity-divergence e2e
test from day 1 per
[`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs).

### R4 — Verification cross-section

2 scenarios against un-overlaid v1 momentum baseline on anchored
real-Binance (10 USDT pairs, mirroring v3-regime-classifier Wave E):
`top10-2023-fy-xgboost-overlay-realdata` (train) +
`top10-2024-fy-xgboost-overlay-realdata` (held-out val). 2 anchors
total. Train/val split per C2 precedent: 2023 train; 2024 val. H1
accuracy + H2 Sharpe-delta evaluated on 2024 held-out only.

### R5 — Non-regression

- **R-NR.1** — `verify_anchors.sh` PASS (75/75); anchors additive
  under namespace `v3.5.0-xgboost` (architect M-T1 ratifies).
- **R-NR.2** — `spec_lint.py` no NEW violation categories.
- **R-NR.3** — v3 retired surfaces untouched:
  `crates/strategy/src/regime_dispatcher.rs`,
  `crates/strategy/src/cash_hold.rs`,
  `crates/forecast/src/markov_switching.rs` byte-identical
  (R2 trait reuse, not amendment).
- **R-NR.4** — `RegimeTag` enum + `embedding.rs` slot layout
  byte-identical from v3 Wave B (K4 invariant).
- **R-NR.5** — v1 momentum scenarios byte-identical baselines.
- **R-NR.6** — Zero `MIGRATION:` comments (durable contract per AGENT.md
  2026-05-29); zero new design tokens; zero `strings.rs` adds beyond
  v3 Wave B-registered classifier labels.
- **R-NR.7** — CLAUDE.md non-negotiable: overlay ships with
  baseline-equity-divergence e2e test from day 1
  (`crates/strategy/tests/xgboost_overlay_end_to_end.rs`); asserts
  overlay equity ≠ un-overlaid baseline ≥ 1 bp.
- **R-NR.8** — XGBoost crate dependency documented +
  architect-confirmed at M-T1 K1 pre-flight. No Python sidecar.

## K — Risk register / falsifiers

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **XGBoost crate flaky on Apple Silicon.** Linker / ARM macros / OpenMP integration may break on macOS aarch64 canonical box. The `xgboost` crate is C++-FFI-wrapped; build can fail mysteriously. | Architect M-T1 pre-flight: `cargo build -p forecast --features xgboost` on canonical box BEFORE Wave A spawn. If fails, Q3 cheap fallback `lightgbm-rust` (Q3=(b)) or pure-Rust `gradient-boosting` (Q3=(c)) per analyst Q3 framing below. |
| **K2** | **Train time budget bust.** 100-200 trees + 5 features × 17k bars × 10 pairs may train slower than budgeted. K3 budget allows < 1 hour CPU on Apple Silicon canonical box. | Architect M-T1 measures train time at Q3 pick; if > 1 hour, downgrade `n_trees` to 50-100 OR drop to 3-pair subset (BTC + ETH + SOL) at Q5 fallback. |
| **K3** | **XGBoost output dampens to single-class (V-REG-2-style).** Classifier may predict one regime > 95% of bars — same shape as v2.5 TCN F4 dampened=0 outcome and v3 Wave A V-REG-1 ConvergenceFailed shape. Detection: per-pair fitted prediction entropy < threshold. | Wave A unit test: `xgboost_class_distribution_non_degenerate` asserts no single class > 90% on 2024 validation. If trips → V-XGB-DAMPENED verdict → retire under V-REG-2 precedent. |
| **K4** | **Classifier accuracy < 50% on held-out validation.** Worse than random 4-state coin flip (25%) is a high bar; < 50% accuracy is the floor — at that point the model is producing noise. | H1 falsifier: accuracy < 50% on 2024 held-out = V-XGB-DAMPENED retire route. Wave D held-out accuracy report is the load-bearing evidence. |
| **K5** | **Same OHLCV signal-floor as C1/C2/C5.** Low-capacity model still operates on the same hourly OHLCV substrate that defeated DL + Markov + LLM. The signal may simply not be present at hourly cadence, regardless of model capacity. | This is the **load-bearing falsification we WANT** — if H1 + H2 both clear, refutes the v3 retirement hypothesis. If both fail, strengthens it for Route C pivot. R4 reports + cross-survey table at M-FINAL document the outcome. |
| **K6** | **v3-vol-targeting noop-fix precedent recurrence.** Overlay implemented with correct multiplier logic but `Signal.quantity_scale` field not propagated to executor (same K6 as v3-regime-classifier). | R-NR.7 mandatory e2e divergence test from day 1; mirrors C2 Wave F gate verbatim. CLAUDE.md non-negotiable. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | **Classifier accuracy ≥ 60%** on 2024 held-out validation. Better than random 4-state guessing (25%); not a stretch target. | Medium | Wave D held-out accuracy report. Falsifies if < 50% → V-XGB-DAMPENED retire. [50%, 60%) → V-XGB-MARGINAL operator-decide. |
| **H2** | **Overlay-style multiplier on v1 momentum yields Sharpe-delta ≥ 0.0** (BREAKEVEN) on 2024 held-out vs un-overlaid v1 baseline. Weaker than v3 +0.10 alpha-unlock; XGBoost as "honest cheap baseline that doesn't lose money" is sufficient ship signal. | LOW-MEDIUM (per post-v3 dev-note) | Wave E backtest report at M-FINAL. Falsifies if Sharpe-delta < 0.0 → V-XGB-CLASSIFIER-ONLY route (H1 ≥ 60% AND H2 < 0.0) or V-XGB-DAMPENED. |
| **H3** | **Train time < 1 hour CPU on Apple Silicon canonical box.** 100-200 trees × 5 features × ~170k samples (17k bars × 10 pairs) is tractable for XGBoost which trains in minutes on this scale per industry baseline. | High | Wave A architect M-T1 measurement; K2 escape if violated. |

## Operator-decide questions (Q1-Q3)

### Q1 — Classifier vs regressor

- **(a) Classifier (REGIME LABEL output)** [Recommended — DURABLE] —
  preserves v3 Wave A `RegimeClassifier` trait seam → future drop-in
  replacement of v3 dispatcher path if operator re-attempts; preserves
  K4 byte-identity; 4-state maps onto operator's 2026-05-28 Q1 lock.
- (b) Regressor (next-bar log-return) [cheap fallback] — continuous
  output directly multiplies momentum signals; **breaks trait seam**
  (`forward_filter` return type doesn't fit single scalar without dummy
  wrapper) → ~1-2 week v0.2.0 cleanup brief required if XGBoost ships
  positive. Strictly worse on durability per AGENT.md 2026-05-29.

**Analyst-recommended Q1 = (a).** Preserves v3 Wave A durable seam at
zero marginal cost (XGBoost classifiers are industry-standard).

### Q2 — Integration mode

- **(a) Overlay-style multiplier on v1 momentum** [Recommended —
  DURABLE] — smallest blast radius; mirrors v3-vol-forecaster + v2.5 TCN
  overlay precedents; e2e divergence-gate template ready; isolates
  model-class as the variable.
- (b) Replace v3 dispatcher's classifier [cheap fallback if Q1=(a)] —
  reuses dispatcher infra; but v3 dispatcher is RETIRED 2026-05-29 →
  reviving it re-litigates the C2 V-REG-5 verdict, muddling "model vs
  integration mode".
- (c) Ensemble with v1 momentum [out-of-scope at v0.1.0] — adds
  architect surface; defer to v0.2.0 if H1+H2 both clear.

**Analyst-recommended Q2 = (a).** Overlay is the cleanest experimental
shape that isolates model-class as the load-bearing variable.

### Q3 — Training tooling (XGBoost crate selection)

- **(a) `xgboost` crate (Cargo)** [Recommended — DURABLE] if
  Apple-Silicon-tested + crate-graph clean per architect M-T1 K1
  pre-flight — industry-standard C++-FFI; battle-tested.
- (b) `lightgbm-rust` [cheap fallback if `xgboost` linker-fails] —
  similar model class; cleaner ARM compat track record; hyperparameter
  surface slightly differs.
- (c) `gradient-boosting` pure-Rust [escape if neither] — zero FFI;
  ~3-5× slower train; may bust K2 budget.

**Analyst-recommended Q3 = (a)** if K1 passes; **(b)** credible cheap
fallback per AGENT.md durable contract.

## Cost framing (both routes per durable contract)

### DURABLE (Q1+Q2+Q3 all (a)): ~4-6 weeks

Analyst M0 ~0.5 day; operator-decide ~1 day; architect M-T1 ~3-5 days
(Q1-Q3 ratification + K1 crate pre-flight + Wave decomposition +
ADR-0049 § Changelog amendment expected, no new ADR); Wave A
classifier core ~5-7 days; Wave B overlay + e2e divergence ~3-5 days;
Wave C backtest + 2 anchors under `v3.5.0-xgboost` ~3-5 days; Wave D
held-out validation ~2-3 days; tester M-FINAL ~3 days. **Total ~4-6
weeks** within Route A budget. Empirical alpha-attempt closes with
quantified verdict — ships SOMETHING (≥ baseline = ship; < baseline =
honest retire like C1/C2/C5; either outcome adds to retrospective).

### Cheap fallback (Q1=(b) + Q2=(a) + Q3=(c)): ~2-3 weeks

Saves ~2 weeks now; trait-seam break + slower train + less interpretable.
**v0.2.0 cleanup brief required if positive** (~1-2 weeks to restore
classifier shape). Net wall-clock ≥ DURABLE; strictly worse on
durability per AGENT.md 2026-05-29 — surfaced for honest framing only,
not recommended.

## Pre-drawn 4-cell verdict tree (presenter inherits at M-P2)

| Cell | Condition | Route |
|---|---|---|
| **V-XGB-PASS** | H1 ≥ 60% accuracy AND H2 ≥ 0.0 Sharpe-delta AND R-NR.1-7 green AND R-NR.7 divergence gate fires | **SHIP** v0.1.0 as honest cheap baseline. Operator approval block; refutes v3 retirement hypothesis ("edge isn't in fancy model"). Spawn v0.2.0 follow-on briefs: Q2=(c) ensemble with v1 momentum + LLM (C5 reuse) per operator decide. |
| **V-XGB-CLASSIFIER-ONLY** | H1 ≥ 60% accuracy AND H2 < 0.0 Sharpe-delta AND R-NR green | **PARTIAL** — classifier accuracy is informational evidence (XGBoost can label hourly crypto regimes better than random) but doesn't unlock alpha via overlay multiplier. Document + retire; XGBoost-as-overlay retired; analyst-recommends future v0.2.0 Q2=(c) ensemble follow-on. Mirrors C5 v0.1.0-PARTIAL ship pattern. |
| **V-XGB-DAMPENED** | H1 < 60% accuracy OR XGBoost outputs single-regime > 90% per K3 | **TRIVIAL retire.** Same retirement shape as C2 V-REG-5; XGBoost can't classify hourly crypto regimes meaningfully → "edge isn't in fancy OR cheap model on hourly OHLCV" — strong empirical support for v3 retirement hypothesis → Route C pivot stronger case. |
| **V-XGB-INCONCLUSIVE** | Training fails K1 (crate-graph break) OR K2 budget bust (> 1 hour CPU at Q3=(c) escape) | **ROUTE BACK TO ANALYST** with crate-graph or budget mitigation; possible outcomes: Q3=(b) `lightgbm-rust` re-pick, Q5 fallback (3-pair subset), or full retirement with what-not-to-chase dev-note. |

**Cell semantics:** ALPHA = H2 ≥ +0.10 Sharpe-delta (not a goal here);
PASS = H2 ≥ 0.0 BREAKEVEN; CLASSIFIER-ONLY = H1 green + H2 red;
DAMPENED = H1 red OR K3 trip; INCONCLUSIVE = K1 or K2 trip pre-Wave-D.

## Cross-references

- Predecessor (Route A framing) — [`spec/dev-notes/post-v3-strategy-direction-2026-05-29.md`](../dev-notes/archive/2026-Q2/post-v3-strategy-direction-2026-05-29.md)
- Parent (Candidate 6) — [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../dev-notes/archive/2026-Q2/strategy-reformulation-survey-2026-05-22.md)
- Siblings — [`v3-volatility-forecaster`](../v3-volatility-forecaster/feature.md) (C1 RETIRED), [`v3-regime-classifier`](../v3-regime-classifier/feature.md) (C2 RETIRED), [`v3-llm-forecaster`](../v3-llm-forecaster/feature.md) (C5 PARTIAL)
- Frozen v0.1.0 trait seam — [`crates/forecast/src/markov_switching.rs`](../../crates/forecast/src/markov_switching.rs)
- ADR-0049 § Changelog (probable amendment site) — [`spec/architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md`](../architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md)
- CLAUDE.md e2e divergence precedent — [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
- Trace row — `REQ-V3-XGBOOST-001` in [`spec/trace.toml`](../trace.toml)
- Tasks — [`tasks.md`](tasks.md)

## Changelog

- 2026-05-29 (analyst): M0 brief authored as Queue pre-position per
  post-v3 Route A Candidate 6. Pre-flight Queue reconciliation: no
  existing `spec/v3-xgboost-*` folder. R1-R5 + R-NR + K1-K6 + H1-H3 +
  Q1-Q3 with DURABLE-recommended defaults + cheap-fallback labels per
  AGENT.md 2026-05-29. 4-cell verdict tree pre-drawn. Stays Queue
  (NOT Active) pending operator Route A pick.
- 2026-06-08 (orchestrator): status `draft` → `retired` — track C lane
  FORECLOSED (spec-hygiene wind-down, audit-2026-06-08). The program's
  terminal verdict (active ≤ passive across all reachable channels incl.
  OHLCV) forecloses this OHLCV regime-classifier lane: a low-capacity
  XGBoost re-test on the same OHLCV substrate cannot change the established
  conclusion, so the brief is closed without promotion. Added a `⛔
  FORECLOSED` body note pointing to [`spec/product.md`](../product.md)
  (terminal verdict) + [`spec/dev-notes/onchain-netflow-spike-2026-06-08.md`](../dev-notes/onchain-netflow-spike-2026-06-08.md)
  (hard-stop). `retired` is the closest valid `spec_lint.py` enum
  (research-line closure, not deletion — the M0 brief stays in the tree as
  authored); never promoted to Active. Frontmatter + body note only.
