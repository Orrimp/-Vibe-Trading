---
adr: 0049
title: v3 regime-classifier — Markov-switching 4-state priors + dispatcher with cash-fallback + V-REG verdict shape (sibling to ADR-0038)
status: accepted
date: 2026-05-28
supersedes: none
superseded-by: none
---

# ADR-0049: v3 regime-classifier Markov-switching verdict shape

## Context

[`spec/v3-regime-classifier/feature.md`](../../../../spec/v1/v3-regime-classifier/feature.md)
v0.1.0 M-OD closed 2026-05-28 (commit `6b47027`) with operator going
bolder than analyst defaults on three load-bearing knobs: **Q1=(b)
4-state Bull/Bear/Volatile/Calm**, **Q3=(b) Markov-switching regression
(Hamilton 1989)**, **Q4=(b) strategy-switching dispatcher**.

The overrides interact: Q4=(b) needs a strategy per regime, but no v1.5
price-MR sibling exists for Chop / Volatile regimes (`vol_meanreversion.rs`
is a vol-surprise MR, wrong shape). Q1=(b) introduces two new RegimeTag
variants but `crates/reflection/src/embedding.rs:120-126` ordinally
encodes `Bull=0, Bear=1, Chop=2` and 7+ downstream tests + lesson-card
embeddings depend on byte-identity (K4 invariant). Q3=(b) replaces
plain HMM with regression-form mixture where each state has explicit
{μ_s, σ²_s} mapping onto Q1 semantics.

[ADR-0038](0038-vol-forecast-verdict-shape.md) § D1 codified the
V-verdict / T-classifier pattern. [ADR-0033](0033-tcn-alpha-investigation-report-shape.md)
§ D3 F-verdict stays IMMUTABLE. This ADR is the third v3 sibling
(parallel to ADR-0038, NOT extension) covering regime-classification.

## Decision

### D1. Model class — Markov-switching with operator-set priors + Baum-Welch refinement

**Q3=(b) Hamilton 1989 Markov-switching regression**, implemented in
`crates/forecast` (sibling to `garch.rs` / `vol.rs`). 4 states with
explicit {μ_s, σ²_s} parameters and a 4×4 row-stochastic transition
matrix P. **Operator-set semantic priors lock regime identities**;
Baum-Welch refines parameter *values* but the state-label assignment
stays pinned to the initial-prior ordering (no post-hoc reassignment).

| State    | μ_s prior (hourly log-return) | σ²_s prior                       |
|----------|-------------------------------|----------------------------------|
| Bull     | +1e-4 (≈+0.01%/h drift)       | Low — 25th-pctile realized var   |
| Bear     | −1e-4                         | Low — 25th-pctile                |
| Volatile | 0                             | High — 90th-pctile               |
| Calm     | 0                             | Low — 10th-pctile                |

**EM convergence:** Δ log-likelihood ≤ 1e-6 over 5 consecutive iters;
max 200 iters; failure → V-REG-1. Per-pair fit on the 2023 train window
(Q2=(c) split). **Alternatives rejected:** plain HMM with post-hoc
labeling (state-label drift breaks D3 dispatcher routing); K-means
priors (same).

### D2. RegimeTag ordinal encoding — option (γ): preserve Chop, append Volatile + Calm

**The conflict:** Q1=(b) brief says "4-state Bull/Bear/Volatile/Calm"
which *drops* Chop, but embedding.rs ordinally pins Chop=2 and 7+
downstream tests depend on byte-identity.

**Resolution (γ):** keep `Chop` as **deprecated-but-preserved-for-K4**
+ append `Volatile` + `Calm`. The new Markov-switching classifier
**emits only the 4 Q1=(b) variants**; the legacy daily `classify_regime`
keeps emitting `{Bull, Bear, Chop}` byte-identically; the dispatcher
routes only on the 4 Q1=(b) variants.

```text
enum RegimeTag {
    Bull,      // 0 — existing (K4)
    Bear,      // 1 — existing (K4)
    Chop,      // 2 — DEPRECATED for new classifiers; preserved for daily seed + legacy K4
    Volatile,  // 3 — NEW (appended)
    Calm,      // 4 — NEW (appended)
}
```

**Display:** `Volatile → "volatile"`, `Calm → "calm"`. Register new
strings in `crates/ui/src/strings.rs` `all()` only if Wave D Trail
surface ships (default yes — dispatcher exposes regime tags per-bar).

**Embedding K4 mitigation:** `regime_slot()` adds `Volatile=>3, Calm=>4`;
embedding vector length grows by 2 one-hot slots; legacy 3-state
embedding output **stays byte-identical** because every existing
fixture emits one of {Bull, Bear, Chop} only (Volatile/Calm slots zero
on every legacy card). Wave B adds `regime_overlay_neutrality_4state.rs`
(analogous to `patchtst_overlay_neutrality`) that re-runs ≥ 1 legacy
fixture and asserts byte-identity. **Escape hatch:** if vector-length
growth itself breaks downstream byte-compare, Wave B promotes to a
versioned schema (`EmbeddingV1` legacy vs `EmbeddingV2` 5-slot); this
is declared in-scope here, no ADR amendment required.

**Alternatives rejected:**
- **(α) remap Chop→Calm + bump ordinal** — breaks 30 v2.5-chain anchor
  body-SHAs via lesson-card embedding determinism. Hard NO.
- **(β) actual 5-state classifier** — contradicts Q1=(b); Chop and
  Calm collapse semantically (both "no drift, low variance"); dispatcher
  faces ambiguous routing.

### D3. Dispatcher integration mode + cash-fallback

**Q4=(b) strategy-switching dispatcher**, new
`crates/strategy/src/regime_dispatcher.rs` (sibling to
`vol_targeting_overlay.rs`).

```text
Regime → Strategy routing (v0.1.0):
  Bull     → v1 MomentumStrategy
  Bear     → v1 MomentumStrategy   (momentum works in trends regardless of direction)
  Volatile → CashHoldStrategy (NEW, degenerate)
  Calm     → CashHoldStrategy (NEW, degenerate)
```

**Prerequisite resolution (HARDEST Q): option (i) degenerate cash-hold.**
Brief surfaces (i)/(ii)/(iii)/(iv); (ii) blows v0.1.0 to ~10+ weeks;
(iii) defeats Q4=(b); (iv) needs operator. **(i) is the architect's
call** — keeps wall-clock at ~5-7 weeks and preserves the dispatcher
seam for v0.2.0+ to fill with a real MR sibling.

**CashHoldStrategy contract** (Wave C):
- New `crates/strategy/src/cash_hold.rs`. Emits `SignalKind::Hold` for
  every symbol, every bar.
- **Existing positions are HELD, not liquidated**, on regime transition
  `Bull/Bear → Volatile/Calm`. Cash-fallback is **SUPPRESSION, not
  LIQUIDATION** — load-bearing distinction. Natural exits via existing
  composed exit policy (ADR-0010).
- On reverse transition Volatile/Calm → Bull/Bear, resume forwarding
  momentum signals.

**v0.2.0 deferred:** follow-on brief
`v1.5-mean-reversion-for-regime-dispatcher v0.1.0` (analyst spawns
after this feature ships). Dispatcher seam is forward-compatible:
swap `CashHoldStrategy → MeanReversionStrategy` in the routing table,
no dispatcher rewire.

### D4. V-REG verdict shape (sibling to ADR-0038 § D1)

V-REG is sibling to V-VOL (ADR-0038) and F (ADR-0033). Computed by
`crates/forecast/src/bin/regime_verdict.rs` over the held-out 2024 val
span. Priority tree, fall-through:

```text
V-REG-1  Convergence failure   — EM didn't converge.       Follow-on: regime-em-tune
V-REG-2  Trivial classifier    — one regime > 95% bars on ≥ 5 symbols. Follow-on: prior-recalibrate
V-REG-3  Flicker               — switch rate > 20/week.    Follow-on: stability-tune
V-REG-4  Calibration drift     — empirical μ diverges from fit μ_s by > 2σ on ≥ 5 symbols. Follow-on: prior-recalibrate
V-REG-5  Healthy (fallback)    — converged; ≥ 2 regimes populated ≥ 5% on ≥ 7/10 symbols; switch ≤ 20/wk; cal within 2σ. → T-REG
```

**T-REG strategy-side gate** (sibling to T-VOL):

```text
T-REG-ALPHA-UNLOCKED  net_delta ≥ +0.10        → SHIP (R-O1)
T-REG-MARGINAL        net_delta ∈ [+0.05, +0.10) → SHIP-WITH-CAVEATS or HOLD (R-O2)
T-REG-NO-ALPHA        net_delta < +0.05        → HOLD-FOR-OPERATOR (R-O3)
```

**Joint advisory table** (M-FINAL): V-REG-5 × T-REG-ALPHA-UNLOCKED →
SHIP + spawn v1.5-MR follow-on; V-REG-5 × T-REG-MARGINAL → operator;
V-REG-5 × T-REG-NO-ALPHA → C2 retire + close v3 three-pick set;
V-REG-1..4 × any → MODEL-BROKEN, follow V-REG's follow-on.

**Canonicalisation** (ADR-0038 § D1 precedent): all {μ_s, σ²_s},
confidence, switch-rate, Sharpe-delta fields use `format!("{:.6}", x)`.
Symbol row order alphabetical USDT: ADA, AVAX, BNB, BTC, DOGE, DOT,
ETH, LINK, SOL, XRP.

### D5. Anchor namespace pin

**Namespace = `v3.0.0-regime`** (matches v3 sibling line; bumped from
analyst Q6 default `v2.7.0-regime` because Q1+Q3+Q4 overrides are
larger-scope than originally framed; follows `v3.0.0-volatility`
precedent at anchors.toml:246).

**4 planned anchors** (Wave E):

```text
top10-2023-fy-regime-dispatcher-realdata
top10-2024-fy-regime-dispatcher-realdata
regime-verdict-bs1-realdata
sharpe-comparison-regime-dispatcher-bs1-realdata
```

Anchor count: 70 → 74 at M-FINAL ship (additive; zero existing-SHA
delta). R-NR.1 `verify_anchors.sh` PASS gate.

### D6. K-reg-2 mitigation — max-confidence dispatcher gate

Dispatcher gates strategy switches on **`max_regime_confidence ≥ 0.70`**
from the Markov-switching forward filter. Below threshold, the previous
regime's strategy keeps running.

**Justification of 0.70:**
- For 4 states, uniform prior = 0.25; 0.70 = 2.8× uniform — meaningful
  posterior concentration.
- Forward filter bounded in [0,1] per state; empirically (Hamilton 1989
  + crypto-vol literature) well-fit 4-state filters spend ~60-70% of
  bars at max_p ≥ 0.7 on liquid hourly OHLCV.
- Trades coverage for stability; combined with the H4 ≤ 20 switches/week
  Wave A unit-test gate, defensible upper-bound mechanism.

**Falsification:** Wave A
`dispatcher_confidence_gate_zero_when_uncertain` asserts no switch when
`max_p < 0.70`; `dispatcher_switches_when_confident` asserts switch
when `max_p ≥ 0.70`. Wave E backtest report logs
`confidence_distribution` histogram for tester audit.

## Consequences

- **Wave A** implements the Markov-switching fitter under D1; **Wave B**
  handles K4 ordinal encoding under D2; **Wave C** ships dispatcher
  under D3 with cash-fallback; **Wave E** emits 4 anchors under D5;
  Waves A+D ship V-REG / T-REG bins under D4.
- **K4 invariant** is load-bearing — Wave B's
  `regime_overlay_neutrality_4state.rs` gates the entire feature.
  Failure → versioned embedding schema (D2 escape hatch; no ADR
  amendment).
- **ADR-0033 § D3 + ADR-0038 § D1 stay IMMUTABLE.** V-REG is a third
  independent sibling.
- **v0.2.0 follow-on** (`v1.5-mean-reversion-for-regime-dispatcher`)
  fills the D3 dispatcher seam; no v0.1.0 rewrite needed.
- **`spec/v3-regime-classifier/feature.md` § Design** is populated by
  architect with cross-refs to D1-D6.
- Mechanical enforcement:
  - D2 ordinal contract → `regime_overlay_neutrality_4state.rs`.
  - D6 confidence gate → two named unit tests above.
  - D5 anchor namespace → pinned in `spec/anchors.toml` at Wave E.

## Changelog

- 2026-05-28 (architect): initial accept. M-T1 lock for
  v3-regime-classifier v0.1.0 post M-OD closure (commit `6b47027`).
  Sibling to ADR-0038 NOT extension.
