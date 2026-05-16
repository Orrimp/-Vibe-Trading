---
adr: 0027
title: v2.5 — Kronos foundation-model forecast overlay (ONNX + tract)
status: accepted
date: 2026-05-16
supersedes: none
superseded-by: none
---

# ADR-0027: v2.5 — Kronos foundation-model forecast overlay (ONNX + tract)

## Context

v2.5 promotes the [Kronos](https://github.com/shiyu-coder/Kronos)
foundation model from candidate to in-progress per
[product.md § Strategy library roadmap](../../product.md#strategy-library--roadmap)
("v2.5 — DL forecaster — Kronos foundation model primary candidate").
Kronos is a decoder-only Transformer pre-trained on K-line data from
45+ exchanges, MIT-licensed, AAAI 2026. The analyst pass landed
2026-05-16 — [feature.md](../../v25-kronos-forecast-overlay/feature.md)
— with 13 open questions, four of which the operator locked at spawn
(Q1 / Q3 / Q9 / Q10-budget / Q12 / Q13) and five of which the architect
resolves here (Q4 / Q5 / Q6 / Q7 / Q8).

Three integration paths were on the table from the
[2026-05-10 pre-eval](../../dev-notes/kronos-evaluation-2026-05-10.md):
(A) subprocess + IPC to a Python `KronosPredictor`, (B) ONNX export
plus in-process `tract` inference, (C) pure-Rust re-implementation
in `candle`. All five operator decisions (notably Q3 — Option B) and
the analyst's four-axis argument
([feature.md § Integration-path argument](../../v25-kronos-forecast-overlay/feature.md#integration-path-argument-r4-expanded))
point to Option B; this ADR ratifies it.

The cross-cutting "what is a forecast overlay" pattern (signal-level
composition, `ForecastProvider` trait, `ForecastOverlay` type,
audit-row shape) is documented in
[architecture/12-forecast-overlay.md](../12-forecast-overlay.md) so
future DL/ML forecasters can re-use it without re-deciding. This ADR
captures the v2.5 instantiation.

## Decisions

### Q2 — Model size: `base` (102.3M params, 512-ctx)

Operator-locked. `base` is the analyst default; the pre-eval
established that `mini` (4.1M) is too small for serious K-line quality
and `large` (499M) is 5× inference cost without commensurate quality
gain on K-line tasks per the Kronos paper. The 102.3M-param `base` at
512-ctx fits comfortably in the `tract` memory budget on macOS Apple
Silicon (dev) and Linux x86_64 (paper deploy). Forecast latency target
≤ 100 ms p99 for a 1h-bar overlay; spike if exceeded.

### Q3 — Integration path: ONNX export + `tract`, in-process

Operator-locked. One-off conversion script exports the Kronos `base`
checkpoint from PyTorch to ONNX via `torch.onnx.export`; the resulting
`.onnx` artifact commits to `crates/forecast/assets/`; runtime loads
via [`tract`](https://github.com/sonos/tract) (named as the ONNX
serving default in
[architecture/10-foundation-libraries.md § Numerics & ML](../10-foundation-libraries.md#numerics--ml)).

**Fallback:** if ONNX conversion fails on unsupported decoder ops, a
1-day spike either (a) adds the op to `tract` (PR upstream) or (b)
falls back to Option A (subprocess + IPC). The fallback is named in
[feature.md § R4.3](../../v25-kronos-forecast-overlay/feature.md#r4--integration-path-onnx-export--tract)
but not pre-built. The spike outcome routes back through the
architect; no developer-side decision on fallback shape.

### Q4 — Forecast horizon: single-bar (next-bar) only

Architect-decide. v2.5 ships a 1-bar forecast — Kronos predicts the
next OHLCV bar and the overlay consumes only that prediction. Multi-bar
rolling forecasts and ensemble-across-horizons are explicitly deferred
to a v2.5.x follow-up if the single-bar BS-1 / BS-2 numbers are noisy.

Rationale: the 1h-bar v1 momentum overlay is the only consumer at
v2.5; the v1 strategy's signal cadence is already per-bar, so a
matching per-bar Kronos cadence is the smallest shape that proves
the integration. Multi-bar horizons add (a) ensemble-aggregation
logic to author and test, (b) larger replay-cache values, (c) a
multi-horizon `ForecastOverlay` shape that breaks the simple
`direction + confidence + horizon_bars = 1` summary the audit row
carries. None of those are load-bearing for the v2.5 ship gate.

### Q5 — Strategy shape: overlay, not pure-Kronos

Analyst-decide already-resolved at the R2 level
([feature.md § R2](../../v25-kronos-forecast-overlay/feature.md#r2--strategy-shape-overlay-not-pure)).
The architect confirms: signal-level overlay (per Q13) on
`v1_momentum`. Pure-Kronos is a v2.6 brief option if the overlay
composition doesn't produce clean signals; the integration work
(ONNX, `tract`, `ForecastProvider`, replay cache) carries forward
without rework.

### Q6 — Determinism contract: inherit v2 LLM record/replay (Q8 pattern)

Architect-decide. v2.5 inherits the v2 LLM record/replay shape
wholesale per [ADR-0019](0019-v2-llm-strategy.md) Q8:

- SQLite + WAL mode for storage.
- Cache key = SHA-256 over canonical JSON of `(model_revision,
  ohlcv_window, temperature, top_p, top_k, max_tokens, sampling_seed)`.
- `schema_version` migration column.
- Atomic-write contract via the `cost` crate's `atomic_write` helper.
- **Strict-replay-only at v2.5 ship**: a request that doesn't hit the
  cache in replay mode returns `ForecastError::ReplayMiss`, matching
  the v2 LLM Q8 rule.

The sampling seed (default `0xC0FFEE` to match the project's existing
fixture seed and the
[ADR-0002 `ChaCha20Rng` seed convention](0002-rng-chacha20.md)) is
explicitly part of the cache key. Two operator-chosen seeds produce
two cache entries and two deterministic forecasts.

**Shared crate extraction (operator-locked Q10):** the architect
attempts to extract `crates/replay-cache/` generic over `K, V` and
migrate `crates/llm/src/replay.rs` to use it. **Budget = 2 dev-days
(operator-locked).** If the extraction exceeds 2 days, the developer
aborts and ships separate caches per the operator's locked fallback;
the architect records the abort in a follow-up ADR and opens a
`replay-cache-extraction` brief for v2.5.x.

### Q7 — Anchor impact: 11 existing stay, 2 new at ship

Architect + tester decide (architect proposes, tester locks). After
v2.5 ships:

- The 9 strategy anchors at
  [`spec/anchors.toml`](../../anchors.toml) lines 15–58 stay
  **byte-identical** (the v2.5 strategy is additive — a new
  `Strategy` impl, not a modification of any existing impl).
- The 2 `report-sample-*` anchors at
  [`spec/anchors.toml`](../../anchors.toml) lines 75–83 (`v2.0.0`)
  stay **byte-identical**. The default `KronosConfig.energy_cost_per_kwh
  = 0` means the v2.5 build emits a `$0.00 expense:infra:kronos_inference`
  line that does not surface in the fixture-driven `report-sample-*`
  bodies (those fixtures don't enable v2.5).
- 2 new anchors lock at the tester pass for BS-1
  (`top10-2023-fy-kronos-momentum`) and BS-2
  (`top10-2024-fy-kronos-momentum`). Total post-v2.5: **13 anchors**
  (was 11).

If a `report-sample-*` anchor moves unexpectedly, the tester routes
back to this ADR for a re-lock decision per the
[T1937 negative-invariant gate](../../anchors.toml). Routes **back to
the analyst** for a re-lock decision; do not silently re-lock. This
matches the
[v2.0.0 Q11 precedent](0019-v2-llm-strategy.md#q11--operator-success-report-llm-spend-denominator-option-c-hot-fix).

### Q8 — Cost telemetry: `CostEvent::Infra` with default-zero usd

Architect-decide. Every forecast call posts:

```rust
CostEvent::Infra {
    line: "kronos_inference".to_string(),
    usd: estimated_kwh * config.energy_cost_per_kwh,  // 0 by default
    period: CostPeriod::PerCall,
}
```

No new `CostEvent` variant per
[ADR-0022](0022-cost-telemetry-crate.md). No new ledger account at
default config (`energy_cost_per_kwh = 0` produces `$0.00`, which the
cost crate's existing posting path absorbs without surfacing in the
report-sample anchors). Operator opt-in to non-zero energy cost posts
to `expense:infra:kronos_inference` per operator config; that path is
already supported by the v0.5 `cost::CostEvent::Infra` scaffolding
([architecture.md:2891](../../architecture.md) — pre-extraction
reference).

If a future operator wants per-token-style accounting (matching the
LLM `CostEvent::Llm` shape), a new variant lands via a separate ADR
and breaks no existing consumer. Cost telemetry is additive; this
decision can be revisited without anchor impact.

### Backtest scenarios — 2023 + 2024 full-year (operator override)

Analyst's default was 2024 H1 + H2. The operator overrode at architect
spawn: **BS-1 = 2023 full-year top-10 USDT** and **BS-2 = 2024
full-year top-10 USDT**. Rationale (operator-supplied): two full
years across distinct macro regimes (2023 = post-FTX recovery / spot
ETF speculation; 2024 = halving + spot ETF launch) provide
regime-change evidence that a same-year H1/H2 split cannot.

The anchor names update accordingly: `top10-2023-fy-kronos-momentum`
and `top10-2024-fy-kronos-momentum`. The comparison baseline is the
existing v1 momentum anchor `top10-2023-1h-momentum` (a 2023 anchor
already exists at line 41–43) for BS-1; BS-2 needs a fresh
`top10-2024-fy-momentum` baseline anchor locked at the same tester
pass for the side-by-side comparison
([feature.md § R8.2](../../v25-kronos-forecast-overlay/feature.md#r8--backtest-scenarios)).

If BS-2's v1 baseline anchor is locked at the v2.5 tester pass, that
is **a third new anchor** (so 14 total post-v2.5). The tester chooses
between "lock BS-2 baseline at v2.5" and "compute BS-2 baseline
fresh each verify-anchors run" — the latter avoids the third anchor
but adds runtime to every verify-anchors invocation. **Architect
prefers lock-at-v2.5** because verify-anchors must stay fast (under
60s on dev hardware per [ADR-0011](0011-v05-cockpit-strategies-panel.md)
performance budget conventions); the tester confirms or routes back.

## Alternatives considered

- **Option A (subprocess + IPC).** Rejected. Adds a Python deployment
  surface (Conda / venv / poetry); two processes to supervise; IPC
  serialization shape becomes a versioned protocol. The pre-eval
  flagged this as fallback-only and the analyst's four-axis argument
  confirms it. Named as the Q3 fallback if ONNX conversion fails.
- **Option C (candle native).** Rejected. Highest implementation cost
  (re-implement the Kronos decoder + weight loader); adds anchor-byte
  bug surface; 4+ weeks to BS-1 vs ~2.5 weeks for Option B.
- **Multi-bar rolling forecast at v2.5 (Q4).** Rejected. Doubles the
  surface area (ensemble aggregation, larger replay-cache rows,
  multi-horizon `ForecastOverlay` shape) without proving the
  integration. Deferred to v2.5.x conditional on BS-1/BS-2 noise.
- **Pure-Kronos strategy (Q5).** Rejected. Overlay is the
  orchestrator's prior + analyst's pick + the v2 LLM precedent.
  Pure-Kronos is a v2.6 option if overlay composition proves messy.
- **Risk-level forecast modulation (Q13).** Rejected. Violates
  "strategy proposes, risk disposes"
  ([architecture/02 § Cross-cutting rules](../02-strategy-registry.md#cross-cutting-rules-formalised-by-the-strategy-clusters));
  contaminates the risk-clamp event taxonomy. Signal-level
  composition matches the v0.5 composed-strategies precedent
  ([ADR-0010](0010-v05-composed-exit-policy.md)). Deferred — not
  scheduled.
- **New `CostEvent::Forecast` variant (Q8).** Rejected. The
  existing `CostEvent::Infra` variant absorbs local-inference compute
  cost without a schema change. Per-token accounting (matching
  `CostEvent::Llm`) is meaningful only if Kronos becomes paid-API —
  if it does, a new variant lands via a separate ADR.
- **Download-on-first-use ONNX (Q12).** Rejected (operator-locked).
  Vendored `.onnx` in `assets/` keeps the build hermetic; no network
  at runtime; SHA-pinned at conversion time. Cost: ~410 MB checkpoint
  in git (LFS). Acceptable for a single checkpoint at v2.5; if the
  checkpoint count grows (mini + small + base + large) the trade-off
  flips and a download-on-first-use cache replaces this. Tracked as a
  follow-up trigger condition, not a v2.5 commitment.

## Consequences

- The 9 strategy anchors at `spec/anchors.toml` lines 15–58 stay
  byte-identical. Any drift = tester routes back through this ADR
  for an explicit re-lock. No silent mutation.
- The 2 `report-sample-*` anchors at lines 75–83 stay byte-identical
  (default `energy_cost_per_kwh = 0`). Any drift = tester routes back
  per the same negative-invariant rule.
- 2 new anchors lock at the v2.5 tester pass:
  `top10-2023-fy-kronos-momentum`, `top10-2024-fy-kronos-momentum`.
  Plus a likely third (`top10-2024-fy-momentum` v1 baseline for BS-2
  side-by-side) — tester confirms count.
- The `forecast_emitted` audit-row `kind` value becomes a new
  open-set TEXT value per
  [architecture/02 § Cross-cutting rules](../02-strategy-registry.md#cross-cutting-rules-formalised-by-the-strategy-clusters).
  No schema migration; consumers that scan `strategy_events.kind`
  must handle the new value gracefully (most do — they UPPER + match
  on a known set with a default-pass branch).
- `ForecastError::ReplayMiss` becomes the strict-replay rule for
  v2.5. Any code path that calls `ForecastProvider::forecast()` in
  research mode without a populated cache MUST fail loudly.
- The Kronos ONNX checkpoint (~410 MB) commits to
  `crates/forecast/assets/kronos-base.onnx` via git LFS. The
  developer adds LFS metadata in the M1 milestone.
- The 2-dev-day extraction budget for `crates/replay-cache/` is
  load-bearing — exceeding it forces the developer to abort, ship
  duplicate caches, and open a v2.5.x follow-up brief. The
  developer flags the budget exit at the T-marker in `tasks.md`.
- If the Q3 fallback fires (ONNX conversion blocks), the M1
  milestone reshapes: subprocess + IPC plumbing replaces the
  `tract` glue, and the v2.5 ship window slips ~1–2 weeks.
- Future DL/ML forecasters consume the
  [12-forecast-overlay.md](../12-forecast-overlay.md) pattern as the
  default shape. Departing from it requires a superseding ADR.

## Anchor implications (summary)

| Anchor | Status after v2.5 |
|---|---|
| 9 strategy anchors (lines 15–58) | byte-identical |
| 2 `report-sample-*` anchors (lines 75–83) | byte-identical |
| `top10-2023-fy-kronos-momentum` | NEW, locked at v2.5 ship |
| `top10-2024-fy-kronos-momentum` | NEW, locked at v2.5 ship |
| `top10-2024-fy-momentum` (v1 baseline for BS-2) | NEW conditional, locked at v2.5 ship if architect-preferred path |

## Changelog
- 2026-05-16 (architect): initial accept. Captures Q2 / Q3 / Q4 / Q5 /
  Q6 / Q7 / Q8 resolutions plus the operator's backtest-baseline
  override (2023 + 2024 full-year, not H1/H2). Cross-cutting pattern
  (signal-level overlay, `ForecastProvider` trait, audit-row shape)
  lives in [architecture/12-forecast-overlay.md](../12-forecast-overlay.md).
