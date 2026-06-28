---
adr: 0039
title: LLM-forecaster verdict criteria L0-L4 (parallel to ADR-0033 § D3 and ADR-0038 § D1, not extension)
status: accepted
date: 2026-05-22
supersedes: none
superseded-by: none
---

# ADR-0039: LLM-forecaster verdict criteria L0-L4

## Context

[ADR-0033](0033-tcn-alpha-investigation-report-shape.md) § D3 codified
the **F-verdict** algorithm (F1/F2/F3/F4) for the v2.5 TCN
alpha-investigation. The retrospective
([`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/archive/2026-Q2/v25-dl-journey-retrospective-2026-05-22.md)
§ Lessons learned, item #2) locked **F-verdict immutability** as the
load-bearing invariant for cross-paradigm evidence comparability — the
F-verdict thresholds (`abs_p95 < 1e-6`, `std/sigma_train > 0.1`, etc.)
cannot drift across follow-on ships because they anchor the comparable
measurement bar the v25-tcn / v25-patchtst chain converged on.

[ADR-0038](0038-vol-forecast-verdict-shape.md) § D1 ratified the
sibling **V-verdict** algorithm (V1/V2/V3/V4/V5 + V_ALPHA) for the
retired C1 `v3-volatility-forecaster` programme — **parallel** to
ADR-0033 § D3, not an extension. The retired-C1 lane's V-verdict has
GARCH-specific inputs (per-symbol QLIKE, calibration ratio, dispersion)
that don't translate to LLM forecasts; ADR-0038 § D1 is the precedent
that *new verdict shapes for new forecaster paradigms get their own
ADR*.

[`spec/v3-llm-forecaster/feature.md`](../../v1/v3-llm-forecaster/feature.md)
asks the architect to lock the **LLM-forecaster verdict shape**. Per
Q6 = (b) (operator-decided 2026-05-22, analyst-strawman LOCKED — no
expansion authorization at M-T1; architect cap "≤2 new priorities
beyond the analyst-strawman before re-surface to operator-decide"),
this ADR codifies the L0-L4 priority tree for LLM-forecaster
M-FINAL evidence routing. The verdict must:

1. Route the operator's promotion-vs-retire-vs-tune decision **without
   hand-eyeballing per-call reasoning traces** — the algorithm has to
   be code-checkable and reproducible (ADR-0033 § D3 + ADR-0038 § D1
   precedent).
2. Stay **mutually exclusive across L1-L4 + L0** (priority-tree
   fallthrough; a fixture grid asserts exclusivity).
3. Stay **parallel to ADR-0033 § D3 and ADR-0038 § D1, not an
   extension** — the F-verdict and V-verdict algorithms in those ADRs
   stay IMMUTABLE per retrospective lesson #2. ADR-0039 is the **third
   sibling**.
4. Carry the **L1-L4 priority tree LOCKED at analyst-strawman** — no
   silent expansion. The architect cap is `≤2 new priorities beyond
   strawman before re-surface to operator-decide`. If a future
   LLM-forecaster ship surfaces a 6th-or-later priority candidate,
   that is an **operator-decide event**, not a silent ADR amendment.

Three orthogonal decisions to lock here, cited from feature.md §
R8.4 + Q6:

1. **L-verdict priority tree** — L1-L4 (model-quality) + L0 (PASS)
   shape, mutual exclusivity, evidence string format. Mirrors
   ADR-0033 § D3 + ADR-0038 § D1 priority-tree structure but with
   LLM-specific inputs (rating-distribution + confidence-outcome
   calibration + cost-overrun + reasoning-trace degeneracy).
2. **Report body shape** — frontmatter (advisory) vs. body (hashed),
   per-call LLM-cost table format, rating-distribution histogram
   representation, verdict section placement, follow-on routing
   language. Both report families (the 2 anchored realdata
   backtest scenarios) follow the ADR-0032 § D4 + ADR-0033 § D2
   precedent (run-varying fields in frontmatter only). Detailed body
   shape lives in `spec/v3-llm-forecaster/decomp.md` § T-AR-6; this
   ADR only locks the **verdict section**.
3. **L_ALPHA strategy-side gate parallel to F4's M-SHARPE + V_ALPHA's
   T-classifier** — Sharpe-delta vs un-targeted v1 momentum baseline;
   net-of-turnover gating metric. The L_ALPHA gate is sibling to the
   L1-L4 priority tree (NOT an L5 branch); architectural shape
   identical to ADR-0033 § D3.c (M-SHARPE) and ADR-0038 § D1.c
   (T-classifier).

## Decision

### D1. L-verdict priority tree (parallel to ADR-0033 § D3 and ADR-0038 § D1, not extension)

The L-verdict is **a sibling of both the F-verdict and the V-verdict**,
evaluated by an independent algorithm over the M-FINAL LLM-forecaster
backtest report's per-call statistics. The three verdicts share no
code path; F-verdict and V-verdict remain exactly as locked in
ADR-0033 § D3 and ADR-0038 § D1 respectively.

#### D1.a — Per-backtest inputs (collected over the BS evaluation span)

The L-verdict bin computes (one tuple per backtest scenario):

```rust
// crates/strategy/src/llm_forecaster/verdict.rs
struct LlmCallStats {
    scenario: String,                // e.g. "top10-2023-fy-llm-forecaster-realdata"
    n_calls: u64,                    // count of forecast() invocations
    n_unique_traces: u64,            // count of distinct reasoning_trace bodies
    rating_dist: [u32; 5],           // counts per Rating (STRONG_SELL .. STRONG_BUY)
    mean_trace_len_chars: f64,       // mean reasoning_trace length
    n_traces_below_50_chars: u32,    // count of traces with len < 50
    confidence_outcome_corr: f64,    // Pearson(confidence_t, |realised_log_return_{t+1}|)
                                     //   over n_calls (signed-correctness-weighted; see D1.b L2)
    cost_actual_usd: f64,            // total LLM cost over the backtest
    cost_projected_usd: f64,         // architect-locked projection from llm-forecaster-bench
    cache_hit_ratio: f64,            // fraction of forecast() calls served from ReplayProvider
    cost_cap_usd: f64,               // config.llm_forecaster.cost_cap_usd_per_backtest
}
```

**Cross-scenario aggregates** — for v0.1.0, the L-verdict evaluates
each scenario independently (2 scenarios at ship: `top10-2023-fy-`
+ `top10-2024-fy-`). The joint table (D1.c) combines the per-scenario
L-verdicts with the L_ALPHA Sharpe-delta gate.

#### D1.b — Per-scenario verdict function (L1..L4)

```rust
fn classify_l(stats: &LlmCallStats) -> LVerdict {
    // L1 — Bias collapse.
    //
    // The LLM produces an overwhelming majority of HOLD ratings
    // (≥95% of calls), signalling "no opinion" — the analogue of
    // F1 "training collapse to numerically zero." Operationalisation:
    //   hold_frac = rating_dist[HOLD as usize] / n_calls
    //   L1 fires iff hold_frac >= 0.95
    //
    // 95% is the threshold because (i) any meaningful forecaster
    // produces directional ratings on a non-trivial minority of
    // bars (≥5% of 87,600 hourly bars = ~4,380 directional calls
    // per backtest); (ii) below 95% HOLD, the strategy can plausibly
    // emit alpha via the directional minority; (iii) tighter (e.g.
    // 99%) lets a near-bias-collapsed LLM slip through to L4 and
    // produce false-positive reasoning-trace evidence.
    let hold_frac = stats.rating_dist[2 /* HOLD index */] as f64
        / (stats.n_calls.max(1) as f64);
    if hold_frac >= 0.95 {
        return LVerdict::L1 {
            evidence: format!(
                "hold_frac = {} / {} = {:.6} >= 0.95 (bias collapse to HOLD)",
                stats.rating_dist[2], stats.n_calls, hold_frac,
            ),
            follow_on: "v3-llm-forecaster-prompt-redesign",
        };
    }

    // L2 — Calibration failure.
    //
    // The LLM's `confidence` field does not predict realised outcome
    // correctness — the analogue of F2 sigma_train mis-calibration
    // and V3 calibration drift. Operationalisation:
    //   Pearson correlation between confidence_t and a
    //   signed-correctness indicator (+1 if rating direction matches
    //   sign of realised next-bar log-return; -1 if opposite; 0 if
    //   either is HOLD) over n_calls.
    //   L2 fires iff |correlation| < 0.05 (essentially zero linear
    //   relationship).
    //
    // 0.05 is the threshold because (i) Pearson on 87,600 samples
    // has a standard error of ~0.0034 under H0=ρ=0, so |ρ| < 0.05 is
    // well within noise; (ii) tighter (0.01) admits near-zero
    // correlation as "calibrated"; (iii) looser (0.10) rejects
    // genuinely-weak-but-real calibration (the survey K-llm-3
    // LOW-MEDIUM-EV prior says the signal may be weak by design).
    if stats.confidence_outcome_corr.abs() < 0.05 {
        return LVerdict::L2 {
            evidence: format!(
                "|confidence_outcome_corr| = {:.6} < 0.05 (calibration failure)",
                stats.confidence_outcome_corr.abs(),
            ),
            follow_on: "v3-llm-forecaster-calibrate-or-retire",
        };
    }

    // L3 — Cost overrun.
    //
    // The actual LLM cost over the backtest exceeds 2× the architect-
    // locked projection from llm-forecaster-bench (R5.2). No analogue
    // in F-verdict or V-verdict — new for the LLM-as-forecaster
    // paradigm. Operationalisation:
    //   overrun_ratio = cost_actual_usd / cost_projected_usd.max(1e-6)
    //   L3 fires iff overrun_ratio > 2.0 OR
    //                cost_actual_usd > cost_cap_usd.
    //
    // 2.0 is the threshold because (i) 1.5× is within bench-error
    // (token-count estimate vs actual); (ii) 2.0× signals a real
    // mis-estimate (prompt grew, cache-hit ratio worse than
    // projected, or N-bar batching cadence wrong); (iii) the
    // hard cost-cap path is a separate signal — overrun-ratio
    // surfaces "bench was wrong"; cap-breach surfaces "budget gate
    // worked."
    let overrun_ratio = stats.cost_actual_usd
        / stats.cost_projected_usd.max(1e-6);
    if overrun_ratio > 2.0 || stats.cost_actual_usd > stats.cost_cap_usd {
        return LVerdict::L3 {
            evidence: format!(
                "cost_actual_usd = {:.6}, cost_projected_usd = {:.6}, \
                 overrun_ratio = {:.6} > 2.0 OR \
                 cost_actual_usd > cost_cap_usd = {:.6}",
                stats.cost_actual_usd, stats.cost_projected_usd,
                overrun_ratio, stats.cost_cap_usd,
            ),
            follow_on: "v3-llm-forecaster-cost-tune",
        };
    }

    // L4 — Reasoning trace degenerate.
    //
    // The `reasoning_trace` field is either too short (< 50 chars)
    // on a majority of calls OR is highly duplicate across calls
    // (the LLM emits boilerplate). No analogue in F-verdict or
    // V-verdict — new for the LLM-as-forecaster paradigm. The
    // reasoning trace IS the differentiator (H3 + Phase F
    // Assistant slot R9); degenerate traces forecloses on H3.
    // Operationalisation:
    //   short_frac = n_traces_below_50_chars / n_calls
    //   duplicate_frac = 1.0 - (n_unique_traces / n_calls)
    //   L4 fires iff short_frac > 0.50 OR duplicate_frac > 0.50.
    //
    // 50 chars: a trace shorter than ~10 words is operationally
    // useless for operator trust-judgment (the Phase F Assistant
    // slot reasoning-card body is meant to be 1-3 sentences of
    // explanation). 50% threshold: a duplicate-or-short majority
    // signals systematic boilerplate; below 50% is within "LLM
    // sometimes terse" tolerance.
    let short_frac = (stats.n_traces_below_50_chars as f64)
        / (stats.n_calls.max(1) as f64);
    let duplicate_frac = 1.0
        - (stats.n_unique_traces as f64) / (stats.n_calls.max(1) as f64);
    if short_frac > 0.50 || duplicate_frac > 0.50 {
        return LVerdict::L4 {
            evidence: format!(
                "short_frac = {} / {} = {:.6} > 0.50 OR \
                 duplicate_frac = 1 - {} / {} = {:.6} > 0.50",
                stats.n_traces_below_50_chars, stats.n_calls, short_frac,
                stats.n_unique_traces, stats.n_calls, duplicate_frac,
            ),
            follow_on: "v3-llm-forecaster-trace-quality-tune",
        };
    }

    // L0 — PASS.
    //
    // Fallback case: L1-L4 all false. The LLM-forecaster emits a
    // non-degenerate rating distribution, calibrated confidence,
    // within-budget cost, and substantive reasoning traces. The
    // strategy is healthy enough to route to the L_ALPHA Sharpe-
    // delta gate (D1.c). Note: L0 PASS does NOT imply alpha — it
    // only certifies the LLM is producing usable evidence. The
    // L_ALPHA gate is the alpha-unlock decision.
    LVerdict::L0 {
        evidence: format!(
            "hold_frac = {:.6} < 0.95; |confidence_outcome_corr| = {:.6} >= 0.05; \
             overrun_ratio = {:.6} <= 2.0; short_frac = {:.6} <= 0.50; \
             duplicate_frac = {:.6} <= 0.50",
            hold_frac, stats.confidence_outcome_corr.abs(), overrun_ratio,
            short_frac, duplicate_frac,
        ),
        follow_on: "l_alpha_strategy_gate",
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LVerdict {
    L0 { evidence: String, follow_on: &'static str },
    L1 { evidence: String, follow_on: &'static str },
    L2 { evidence: String, follow_on: &'static str },
    L3 { evidence: String, follow_on: &'static str },
    L4 { evidence: String, follow_on: &'static str },
}
```

**Mutual exclusivity** — L1 → L2 → L3 → L4 → L0 fallthrough in
priority order. The first triggering case returns. A unit test in
`crates/strategy/tests/llm_forecaster_verdict_mutual_exclusivity.rs`
asserts mutual exclusivity over a hand-built fixture grid + a
property test (random fixture → exactly one verdict returned). Same
shape as `crates/forecast/tests/vol_verdict_mutual_exclusivity.rs`
(ADR-0038 § D1.b precedent) and
`crates/forecast/tests/forecast_distribution_verdict.rs`
(ADR-0033 § D3.d precedent).

**Threshold derivation summary** (the full per-threshold rationale
appears inline in the `classify_l` body above):

| L  | Threshold                            | Rationale                                   |
|----|--------------------------------------|---------------------------------------------|
| L1 | `hold_frac ≥ 0.95`                   | Bias collapse — 5% directional minority is the floor for "real opinion." |
| L2 | `\|confidence_outcome_corr\| < 0.05` | Calibration failure — 0.05 is well outside Pearson noise at n=87,600. |
| L3 | `overrun_ratio > 2.0 OR cost_actual > cost_cap` | Cost overrun — 1.5× is bench-error, 2.0× is a real mis-estimate. |
| L4 | `short_frac > 0.50 OR duplicate_frac > 0.50` | Reasoning trace degenerate — 50% boilerplate forecloses on H3. |
| L0 | none of L1-L4 trigger                | PASS — routes to L_ALPHA strategy-side gate. |

**Architect cap on priority expansion** (Q6 operator-locked constraint):
**≤2 new priorities beyond the analyst-strawman L1-L4 before
re-surface**. If a future LLM-forecaster ship (e.g. v0.1.1 multi-symbol
batched-prompts; v0.2.0 overlay-on-momentum) surfaces an L5 / L6 / L7
candidate priority, the proposing party (architect / analyst / tester)
**must** route to operator-decide as a separate Q-question rather than
silently amending this ADR. The intent is the same as ADR-0033 §
F-verdict immutability + ADR-0038 § D6.b re-emission protocol: the
verdict thresholds anchor the comparable measurement bar across
future LLM-strategy ships, and silent expansion defeats that anchor
property.

#### D1.c — L_ALPHA strategy-side gate (parallel to F4's M-SHARPE and V_ALPHA's T-classifier)

L_ALPHA is **NOT** part of the L1..L4 priority tree. It is a
**separate strategy-side gate** that runs against the M-SHARPE
report (Sharpe-comparison bin) — exactly the same architectural
shape as F4's M-SHARPE in ADR-0033 § D3.c (the M-SHARPE Sharpe-delta
verdict is a sibling of the F-verdict, not a 5th branch) and
V_ALPHA's T-classifier in ADR-0038 § D1.c.

```rust
// crates/strategy/src/llm_forecaster/verdict.rs
// (sharpe-comparison-llm-forecaster dispatch extension)
fn classify_l_alpha(
    sharpe_baseline: f64,            // un-targeted v1 momentum (top10-2023-1h-momentum)
    sharpe_llm: f64,                 // top10-2023-fy-llm-forecaster-realdata
    sharpe_llm_net: f64,             // net of turnover + LLM cost (gating metric)
) -> LAlphaVerdict {
    let gross_delta = sharpe_llm - sharpe_baseline;
    let net_delta = sharpe_llm_net - sharpe_baseline;
    if net_delta >= 0.10 {
        LAlphaVerdict::LAlphaUnlocked { gross_delta, net_delta }
    } else if net_delta >= 0.05 {
        LAlphaVerdict::LMarginal { gross_delta, net_delta }
    } else {
        LAlphaVerdict::LNoAlpha { gross_delta, net_delta }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LAlphaVerdict {
    LAlphaUnlocked { gross_delta: f64, net_delta: f64 },
    LMarginal      { gross_delta: f64, net_delta: f64 },
    LNoAlpha       { gross_delta: f64, net_delta: f64 },
}
```

**Net-of-turnover-AND-LLM-cost is the gating metric** — per K-llm-2
+ K-vol-1 precedent (rebalancing turnover eats lift; LLM cost eats
into post-cost Sharpe). Gross Sharpe-delta is reported side-by-side
for diagnostic visibility. The Sharpe-delta thresholds
(+0.10 unlocked / +0.05 marginal / <+0.05 no-alpha) are inherited
verbatim from ADR-0038 § D1.c (T-classifier) for cross-paradigm
comparability — every v3-era strategy ship gets the same alpha bar.

**Joint advisory verdict** (recorded at M-FINAL in feature.md §
Verification):

| L-verdict | L_ALPHA classifier   | Joint advisory verdict | Operator routing |
|-----------|----------------------|------------------------|------------------|
| L0        | L-ALPHA-UNLOCKED     | ALPHA-UNLOCKED         | Ship; promote to paper-trading; spawn `v3-llm-forecaster-overlay-on-momentum` (Q4=(b) deferred). |
| L0        | L-MARGINAL           | MARGINAL               | Spawn `v3-llm-forecaster-tune` (N-bar cadence or prompt restructure). |
| L0        | L-NO-ALPHA           | NO-ALPHA               | Retire C5; route budget to C2 `v3-regime-classifier` (still in backlog Queue). |
| L1/L2/L4  | (any)                | MODEL-BROKEN           | Follow L-verdict's `follow_on` field. |
| L3        | (any)                | COST-OVERRUN           | Bump R5.4 `fire_every_n_bars` to 168 (weekly) OR downgrade to quick-think tier; re-run backtest only. |

The joint table is the **only** place L-verdict and L_ALPHA combine.
They never combine inside the verdict bins themselves; that keeps
each bin's output anchor-deterministic in isolation (same shape as
ADR-0033 § D3.c joint table and ADR-0038 § D1.c joint table).

### D2. Report verdict section shape (delegates body shape to decomp.md § T-AR-6)

Both report families (`top10-2023-fy-llm-forecaster-realdata` +
`top10-2024-fy-llm-forecaster-realdata`) follow ADR-0032 § D4 +
ADR-0033 § D2 + ADR-0038 § D2 precedents exactly: run-varying fields
in YAML frontmatter (excluded from body hash via
`scripts/hash_report.py`); deterministic content in the body.

The full body table layout (per-call cost rows, rating-distribution
histogram, cache-hit-ratio row, reasoning_trace_sha256 histogram) is
locked in `spec/v3-llm-forecaster/decomp.md` § T-AR-6 (not here —
the body shape is a per-feature anchor target, not a cross-feature
ADR contract). This ADR only locks the **Verdict section** placement
+ shape:

```markdown
## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Case              | L0                                             |
| Trigger evidence  | hold_frac = 0.230612 < 0.95; \|confidence_outcome_corr\| = 0.142318 >= 0.05; overrun_ratio = 1.124003 <= 2.0; short_frac = 0.078521 <= 0.50; duplicate_frac = 0.046218 <= 0.50 |
| Routes to         | L_ALPHA strategy-side gate (Sharpe-comparison bin) |
```

Floating-point canonicalisation in the Verdict section uses
`format!("{:.6}", x)` (6 decimals) — mirror of ADR-0033 § D2.a and
ADR-0038 § D2.a. Integer fields (e.g. `n_calls`, `rating_dist[i]`)
use `format!("{}", x)`.

### D3. L_ALPHA Sharpe-comparison bin extension

Per feature.md § R8.3 + decomp.md § T-AR-7 Wave E, the Sharpe-
comparison bin
[`crates/forecast/src/bin/sharpe_comparison.rs`](../../../crates/forecast/src/bin/sharpe_comparison.rs)
extends with a new dispatch arm for the LLM-forecaster scenarios:

```yaml
sources:
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-1h-momentum-realdata.md  # un-targeted v1 baseline
  - spec/v3-llm-forecaster/reports/backtest-…-top10-2023-fy-llm-forecaster-realdata.md
```

The dispatch extension is additive — existing TCN / PatchTST /
vol-target arms stay byte-identical. The new sharpe-comparison
report under
`spec/v3-llm-forecaster/reports/sharpe-comparison-llm-forecaster-bs1-realdata-YYYYMMDD.md`
mirrors ADR-0038 § D2.b structurally; the L_ALPHA verdict cell
(LAlphaUnlocked / LMarginal / LNoAlpha) appears in the Verdict
section with both gross + net columns reported side-by-side.

**Anchor decision for the sharpe-comparison report** — locked in
`spec/v3-llm-forecaster/decomp.md` § T-AR-6 (analyst-strawman:
**NOT anchored at v0.1.0** per ADR-0038 § D6 + ADR-0033 precedent
of "anchor the underlying backtest scenarios but defer the
sharpe-comparison anchor to v0.2.0 when overlay scenarios join").

### D4. Replay-cache namespace additive extension

Per feature.md § R6 + decomp.md § T-AR-2, the LLM-forecaster's
replay-cache lives at a **dedicated namespaced sqlite file**:

- Path: `data/llm-forecaster-replay.db` (live recording mode) /
  `crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz`
  (checked-in compressed fixture for backtest determinism — mirrors
  `crates/llm/tests/fixtures/llm-replay.db` precedent at
  v2-llm-strategy v2.0.0).
- The existing `crates/llm::RecordingProvider` + `ReplayProvider`
  schema is reused **verbatim** — no new infra. The strategy
  initialises a new sqlite handle pointed at the new path; the
  existing `(request_hash, response)` row format applies.
- Cache rows carry an additive `cache_schema_version` field
  (analyst-strawman = 1 at v0.1.0; bump invalidates anchor — see
  decomp.md § T-AR-5).
- The architect's K5 resolution (decomp.md § T-AR-2) commits to
  **checking in the compressed fixture** (analyst-recommended option
  ii) at < 50 MB. Cold-checkout determinism preserved without
  cloud-spend coupling.

### D5. Strategy-side composition (v0.1.0); Phase F Assistant slot promotion (v0.1.0); overlay deferred to v0.1.1

Per feature.md § R4 + Q4 = (a)+(c) hybrid (operator-locked
2026-05-22; Wave F UNGATED at T-OD4):

- **v0.1.0 ships standalone `LlmForecasterStrategy` AND Phase F
  Assistant slot body promotion**. The strategy emits per-bar
  `Signal` derived from the L-tier rating; the Assistant slot body
  renders the most-recent reasoning trace + cited lesson cards +
  cost line (R9.2 body composition; runtime-gated per R9.3 so
  default-disabled config keeps Phase F snapshot baselines byte-
  identical).
- **Q4=(b) overlay-on-momentum DEFERRED to v0.1.1** — composes the
  v3 LLM forecaster as an overlay on v1 momentum; mirrors v2.5 TCN
  overlay pattern. Spawn when v0.1.0 ships L0 / L-ALPHA-UNLOCKED.
- **Q4=(d) all-three-as-builders DEFERRED to v0.2.0+** — exposes
  Q4=(a) / (b) / (c) as opt-in builders the operator composes via
  config.
- The Q5 = (b) replay-cache + temperature=0 determinism contract
  ships at v0.1.0 (no new infra; extends usage of v2.0.0
  `crates/llm::RecordingProvider` + `ReplayProvider`).

### D6. Anchor + version naming

- **New anchors** (under version `v3.0.0-llm-forecaster` per Q7=(a)):
  - `top10-2023-fy-llm-forecaster-realdata` (BS-1 backtest).
  - `top10-2024-fy-llm-forecaster-realdata` (BS-2 backtest).
- **Existing 34 anchors stay byte-identical** — this ship is
  anchor-additive only.
- **`sharpe-comparison-llm-forecaster-bs1-realdata`** ships
  **without an anchor in v0.1.0** per analyst-recommended deferral
  (decomp.md § T-AR-6) — added in v0.1.1 if byte-deterministic
  and if the v0.1.0 ship clears L0 / L-ALPHA-UNLOCKED or
  L-MARGINAL (precedent: ADR-0038 § D6 + `sharpe-comparison-vol-
  target-bs1-realdata` deferred to v0.1.1 in the original C1 ship).

Anchor count progression (post-noop-fix C1 baseline = 34):
- Pre-feature: 34 (current baseline post v3-volatility-forecaster-
  noop-fix v0.1.0 in-place re-emissions; confirmed via
  `scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)` on
  2026-05-22).
- Post M-FINAL: 36 (+ 2 LLM-forecaster realdata anchors).

**Re-emission protocol** — inherited from ADR-0038 § D6.b verbatim.
If a future LLM-forecaster wiring-bug discovery requires SHA re-
emission under the new `v3.0.0-llm-forecaster` namespace, the 5-step
protocol applies: enumerate affected anchors, cite bug site, include
would-have-caught test, architect signs off, negative invariant
preserved.

## Alternatives considered

1. **Extend ADR-0033 § D3 with LLM-classifier branches.** Rejected
   per Q4=(a) reject in ADR-0033's analogous decision + retrospective
   lesson #2: ADR-0033 § D3 is IMMUTABLE for return-target
   forecasters. The F-verdict thresholds anchor the comparable
   measurement bar across v25-tcn / v25-patchtst evidence; mutating
   ADR-0033 to host L1-L4 branches breaks that immutability property
   for zero architectural benefit. Same rejection logic as ADR-0038
   § Alternatives item #1.

2. **Extend ADR-0038 § D1 with LLM-classifier branches.** Rejected
   for the same reason as #1 above — ADR-0038's V1-V5 priority tree
   has GARCH-specific inputs (per-symbol QLIKE, calibration ratio)
   that don't translate to LLM forecasts. Forcing the L1-L4
   thresholds into ADR-0038's `classify_v` would require splitting
   the function on input-type, defeating the immutability property
   that anchors the vol-paradigm measurement bar.

3. **Embed L1-L4 thresholds in TOML config.** Rejected per the
   ADR-0033 § Alternatives and ADR-0038 § Alternatives precedent:
   thresholds are load-bearing for the algorithm; a TOML config
   invites operators to tune them silently between runs and defeats
   the anchor contract. Future tuning happens in a superseding ADR
   with an explicit follow-on feature.

4. **5-or-more-priority tree at v0.1.0** (e.g. add L5 "retrieval
   relevance" measuring how often `cited_lesson_ids` were materially
   informative). Rejected per Q6 operator-lock + architect cap on
   priority expansion (D1.b last paragraph): the analyst-strawman
   L1-L4 priorities are minimal viable; expansion beyond 2 new
   priorities (i.e. up to L6 total) without re-surface to operator-
   decide is forbidden. The "retrieval relevance" measurement is
   genuinely interesting but is **not** a verdict-tree concern at
   v0.1.0 — surface as a v0.1.1 Q-question.

5. **No new ADR; track verdict criteria inline in the report.**
   Rejected per Q6 = (b) operator-pick. Inline tracking is fragile
   across future LLM-strategy ships — the v0.1.1 overlay-on-momentum
   and v0.2.0 all-three-as-builders + any future LLM-classifier
   strategy would each have to re-author the L1-L4 thresholds in
   their per-feature report. Codifying once in an ADR is the
   precedent set by ADR-0033 § D3 + ADR-0038 § D1.

6. **MSE / MAE-of-rating-error instead of confidence-outcome
   correlation for L2.** Rejected: rating-error is ill-defined for
   a 5-tier categorical scale (STRONG_SELL / SELL / HOLD / BUY /
   STRONG_BUY) — there's no canonical numerical interpolation between
   adjacent tiers. Pearson correlation on
   `(confidence, signed_correctness_indicator)` is well-defined for
   the categorical scale (the indicator is +1/-1/0 by construction)
   and captures the "high-confidence calls should be more often
   directionally correct" property that L2 is meant to detect.

7. **Per-call L-verdict instead of per-scenario.** Rejected: per-call
   L-verdict makes no sense for L1 (bias collapse needs n_calls
   denominator) or L2 (correlation needs ≥ ~100 samples) or L3
   (overrun-ratio aggregates). L4 *could* be per-call (a single
   degenerate trace is locally diagnostic) but the operationalised
   per-scenario short_frac + duplicate_frac aggregates capture the
   systemic case and admit a clean priority-tree fallthrough.

## Consequences

**New files (this ADR scope):**
- This file: `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md`
- `crates/strategy/src/llm_forecaster/verdict.rs` (~150 LoC; D1 priority tree + L_ALPHA classifier; analyst pass).
- `crates/strategy/tests/llm_forecaster_verdict_mutual_exclusivity.rs` (~120 LoC; fixture grid + property test; mirrors `vol_verdict_mutual_exclusivity.rs`).

**Modified files (this ADR scope):**
- `crates/forecast/src/bin/sharpe_comparison.rs` (additive:
  `--scenario llm-forecaster-bs1` dispatch arm; existing TCN /
  PatchTST / vol-target dispatch byte-identical).
- `spec/architecture/adr/README.md` — registry row added for ADR-0039.
- `spec/anchors.toml` — 2 new anchor rows under `v3.0.0-llm-forecaster`
  (at developer M-FINAL — not at ADR-author time).
- `spec/trace.toml` — `REQ-V3-LLM-FORECASTER-001` `arch` column
  extended at M-T1 close.

**Cross-phase implications:**
- v0.1.1 (if v0.1.0 finishes L0 / L-MARGINAL or L0 / L-ALPHA-UNLOCKED):
  spawned ship `v3-llm-forecaster-overlay-on-momentum` inherits the
  L-verdict report shape verbatim; substitute
  `llm-forecaster-realdata` with `llm-forecaster-overlay-on-momentum-
  realdata` in the bin paths. The L1-L4 priority branches apply as-is
  (they classify the underlying LLM forecaster, not the overlay
  composition).
- v0.2.0 (if v0.1.1 finishes L0 / L-ALPHA-UNLOCKED on the overlay):
  spawned ship `v3-llm-forecaster-all-three-builders` inherits the
  L-verdict report shape verbatim per-builder. Three independent
  L-verdicts at M-FINAL, one per builder.
- v2x-trading-state-bus (if promoted ahead of v0.1.1 per Q-V2X-SEQ):
  the L-verdict algorithm is paradigm-agnostic — refactoring
  `ForecastContext → TradingState` substrate does not change the
  L1-L4 inputs (rating distribution + confidence correlation + cost
  + trace quality are computed downstream of the context shape).

**Enforced by:**
- `cargo test -p strategy --test llm_forecaster_verdict_mutual_exclusivity` —
  one fixture per L-label + mutual-exclusivity property test (mirrors
  `vol_verdict_mutual_exclusivity.rs`).
- `cargo test -p strategy --test llm_forecaster_neutrality` —
  R10.2 carry-forward; re-runs `top10-2023-fy-tcn-overlay-realdata`
  and asserts body-SHA `8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642`
  unchanged after registry add.
- `bash scripts/verify_anchors.sh` — must report `36/36` post M-FINAL;
  pre-M-FINAL must report `34/34` (current baseline as of
  2026-05-22, confirmed by architect M-T1 quoted literal in
  `spec/v3-llm-forecaster/decomp.md` § Baseline).
- 2-run byte-identity determinism gate on the new
  `top10-2023-fy-llm-forecaster-realdata-*.md` report (decomp.md
  § T-AR-5 K4 mitigation: **3-back-to-back identical cache-build
  runs** before anchor lock).
- 2-run byte-identity determinism gate on the new
  `top10-2024-fy-llm-forecaster-realdata-*.md` report.

**What breaks if this is violated:**
- An L-verdict authored without the algorithm (e.g. hand-written in
  a report body) → mutual-exclusivity check fails or evidence string
  doesn't match the values, the orchestrator cannot route. Caught
  by `llm_forecaster_verdict_mutual_exclusivity`.
- A future ship silently adding an L5 priority branch (e.g. retrieval-
  relevance) → defeats the architect cap on priority expansion and
  the comparable-measurement-bar property. Surfaces at code review
  via the inline architect-cap comment in `verdict.rs`; further
  protected by the operator-decide gate ("≤2 new before re-surface").
- Floating-point format drift in the per-scenario Verdict section
  (e.g. `%.5f` instead of `%.6f`) → body SHA flips on a second run,
  LLM-forecaster anchor lock fails.
- A developer changes the L1 `hold_frac` threshold (0.95) without an
  ADR amendment → cross-paradigm comparability breaks; future ships
  cannot reliably interpret an L1-vs-L0 verdict.

**What this enables:**
- Operator gets a code-checkable L-verdict that routes follow-on
  promotion-vs-retire-vs-tune decisions without eyeballing per-call
  reasoning traces.
- v0.1.1 overlay-on-momentum ship reuses the L-verdict report shape
  verbatim, dropping its authoring cost to ~1 day.
- v0.2.0 all-three-as-builders + any future LLM-classifier strategy
  inherits the L-verdict algorithm as the baseline measurement bar.
- Joint advisory verdict (L × L_ALPHA) table at D1.c gives the
  presenter a code-checkable routing tree from M-FINAL evidence →
  operator decision (5-cell tree mirrors ADR-0033 § D3.c and
  ADR-0038 § D1.c precedents).

## References

- [ADR-0019](0019-v2-llm-strategy.md) — v2 LLM strategy foundation;
  this ADR builds on the `crates/llm` infra shipped at v2-llm-strategy
  v2.0.0 (`LlmProvider` trait + `BudgetedProvider` + `RecordingProvider`
  + `ReplayProvider` + `CachedSystemPromptBuilder` + `ToolSchema`).
- [ADR-0029](0029-tcn-checkpoint-provenance.md) — canonical-arch
  descriptor; the LLM-forecaster's `ForecastContext::request_hash`
  (decomp.md § T-AR-2 R6.6) inherits the canonicalisation discipline.
- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) —
  realdata path + frontmatter-vs-body discipline (the precedent this
  ADR's D2 follows).
- [ADR-0033](0033-tcn-alpha-investigation-report-shape.md) §§
  D2/D3/D3.c/D3.d — IMMUTABLE F-verdict; this ADR is PARALLEL per
  Q6=(b) operator-pick, not extension.
- [ADR-0038](0038-vol-forecast-verdict-shape.md) §§ D1/D1.c/D2/D6 —
  V-verdict + T-classifier + report-body discipline + re-emission
  protocol (the load-bearing sibling precedent; this ADR mirrors the
  ADR-0038 structure verbatim, swapping vol-specific decisions for
  LLM-specific ones).
- [`spec/v3-llm-forecaster/feature.md`](../../v1/v3-llm-forecaster/feature.md)
  R1-R10, H1-H5, K-llm-1..10, Q1-Q8 + Q-PROMOTE + Q-V2X-SEQ +
  Q-ASSISTANT-WAKE operator-decide bundle (resolved 2026-05-22 —
  Q1/Q2/Q3/Q5/Q7/Q8 + Q-V2X-SEQ + Q-ASSISTANT-WAKE under standing
  Autoapprove; Q4 + Q6 explicit operator-pick).
- [`spec/v3-llm-forecaster/decomp.md`](../../v1/v3-llm-forecaster/decomp.md)
  § T-AR-9 — this ADR's authoring spec.
- [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../../dev-notes/archive/2026-Q2/strategy-reformulation-survey-2026-05-22.md)
  § Candidate 5 — survey-time cost / EV / reuse scoping.
- [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../../dev-notes/archive/2026-Q2/v25-dl-journey-retrospective-2026-05-22.md)
  § Lessons learned (cheap-first; F-verdict immutability) — the
  guardrails this ADR honors.
- Patton 2011 (analogue for verdict-shape discipline) — *Volatility
  forecast comparison using imperfect volatility proxies* — Journal
  of Econometrics 160(1) — codified loss-function discipline that
  ADR-0038 inherited; ADR-0039 inherits the discipline structurally
  (correlation as a robust "calibration" measurement on a
  categorical-rating scale where MSE / MAE-of-rating is ill-defined).

## Changelog

- 2026-05-22 (architect): initial accept. Locks six orthogonal
  decisions (D1 L1-L4 + L_ALPHA priority tree; D2 verdict section
  shape; D3 L_ALPHA Sharpe-comparison bin extension; D4 replay-cache
  namespace additive extension; D5 strategy-side + Assistant slot
  composition v0.1.0 + overlay deferred; D6 anchor + version naming
  + re-emission protocol inheritance from ADR-0038 § D6.b). Covers
  T-AR-9 from `spec/v3-llm-forecaster/tasks.md`. PARALLEL to
  ADR-0033 § D3 AND ADR-0038 § D1, NOT extension (Q6=(b) operator
  default 2026-05-22; retrospective lesson #2 honored). Analyst-
  strawman L1-L4 priorities LOCKED per Q6 operator constraint;
  architect cap "≤2 new priorities beyond strawman before
  re-surface" codified inline at D1.b. Cross-refs
  `REQ-V3-LLM-FORECASTER-001` in `spec/trace.toml`.
