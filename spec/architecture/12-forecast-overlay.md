---
slug: architecture-12-forecast-overlay
status: shipped
owner: architect
updated: 2026-05-17
---

# Forecast overlays — `ForecastProvider`, overlay composition, replay

> **Status note (2026-05-22):** The v2.5 DL forecaster programme was
> RETIRED 2026-05-22 after joint F4-F4-F4 evidence across TCN BS-1/BS-2
> @ 1h horizon and PatchTST BS-1 @ 24h horizon. The overlay-composition
> pattern documented here remains the canonical shape for any future
> DL/ML forecaster — but the v2.5 instantiation (TCN @ 1h, PatchTST @
> 24h, vanilla Transformer @ TBD, bake-off) did NOT extract +0.10
> Sharpe-delta vs the v1 momentum baseline. See
> [`spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md`](../dev-notes/v25-dl-journey-retrospective-2026-05-22.md)
> for the full evidence chain and "what NOT to chase" guardrails.
> Future forecaster designs (volatility forecasting, regime
> classification, longer horizons, crypto-specific features) can reuse
> this overlay shape; the constraint is on the prediction task, not
> the architecture pattern.

This section formalises the cross-cutting overlay-strategy pattern
introduced at v2.5 for the DL forecaster slot. It is the canonical entry
point for any future DL/ML forecaster that emits a distributional forecast
and modulates an existing strategy's signal.

The v2.5 strategy-specific decisions (model family, training framework,
crate placement, audit-row shape) live in
[ADR-0028](adr/0028-v25-dl-forecast-overlay-candle.md) — which superseded
the original Kronos-targeted [ADR-0027](adr/0027-kronos-onnx-tract-integration.md)
on 2026-05-16. This file is the *shape*; the v2.5 instantiation is now
retired (see status note above).

## What is a forecast overlay

A **forecast overlay** is a `Strategy` impl that does NOT emit
position-sizing intent on its own. Instead, it consumes a base
strategy's `Signal` and a `ForecastOverlay` value, returns a modulated
`Signal`, and posts an audit row carrying the forecast plus a
`correlation_id` that ties forecast → realised outcome at trade close.

The overlay is composed at the **signal level**, inside `tick()`,
not at the risk-clamp level. The reasons:

1. **Risk stays universal.** `risk::size_portfolio_target` already
   clamps every strategy's intent uniformly; introducing per-strategy
   forecast-aware sizing would split that surface in two.
2. **"Strategy proposes, risk disposes" stays intact**
   ([architecture/02 § Cross-cutting rules](02-strategy-registry.md#cross-cutting-rules-formalised-by-the-strategy-clusters)).
3. **The v0.5 composed-strategy precedent**
   ([ADR-0010](adr/0010-v05-composed-exit-policy.md)) already composes
   at the signal level. Overlays are a generalisation of that pattern,
   not a new one.
4. **Audit clarity.** A signal-level overlay posts one journal entry
   per forecast call, traceable to the bar that triggered it; a
   risk-level overlay would post inside `risk::size_portfolio_target`
   and contaminate the risk-clamp event taxonomy.

Risk-level forecast modulation (e.g. "forecast lowers max position size
for the next N bars") is a deliberate v2.5.x follow-up question, not a
v2.5 commitment.

## `ForecastProvider` trait

A new trait sits alongside `LlmProvider` in shape, lives in
`crates/forecast/`, and is consumed by `crates/strategy/` impls via DI:

```rust
#[async_trait]
pub trait ForecastProvider: Send + Sync {
    async fn forecast(
        &self,
        request: ForecastRequest,
    ) -> Result<ForecastResponse, ForecastError>;
}
```

The trait is intentionally narrower than `LlmProvider`: no tool-use,
no streaming, no provider-aware prompt cache. It is one method, one
request type, one response type, one error type.

`ForecastError` variants (mirrors the 8-variant `LlmError` pattern from
[ADR-0019](adr/0019-v2-llm-strategy.md) Q4 but only the ones that
actually apply): `Provider | Timeout | InvalidInput | InvalidResponse |
ReplayMiss | Inference | BudgetExceeded`. No `RateLimited` (local
inference); no `Network` / `Auth` (local). `BudgetExceeded` is for
the future `CostEvent::Infra` budget gate.

`ForecastRequest` carries the OHLCV window, sampling parameters
(temperature, top-p, top-k, seed), and the `model_revision` — a
SHA-256 hash over the training-run artifacts (weights + config +
training data span) so every prediction is provenance-pinned.

`ForecastResponse` carries the forecasted OHLCV samples (the full
distribution, not just the mean), the model revision actually used,
and a `correlation_id` that the audit row consumes downstream.

## `ForecastOverlay` value type

The wire format between a `ForecastProvider` and a consuming
`Strategy` impl is `ForecastOverlay`, defined in `crates/core/` next
to `Signal`:

```rust
pub struct ForecastOverlay {
    pub correlation_id: Uuid,
    pub confidence: rust_decimal::Decimal,  // [0, 1]
    pub direction: Direction,               // Up / Down / Flat
    pub horizon_bars: u32,                  // 1 at v2.5
    pub model_revision: String,             // training-run SHA
    pub sampled_at: time::OffsetDateTime,   // 6-digit fractional seconds
}
```

This shape is deliberately small and serde-stable so it can land in
audit rows and replay-cache values without dragging the full sampled
OHLCV distribution into the journal. The full distribution stays in
the replay-cache row (keyed by the same hash); the audit row carries
only the summary.

`rust_decimal::Decimal` for `confidence` per
[ADR-0003](adr/0003-decimal-money-math.md) — no `f64` anywhere in
spec-traceable types.

## Overlay composition pattern (signal-level)

A consuming strategy implements `Strategy::tick()` as:

```text
1. Read the base strategy's Signal (e.g. v1 momentum's bar-cross signal).
2. Call ForecastProvider::forecast() with the OHLCV window for the bar.
3. Combine: if forecast.direction agrees with base.direction AND
   confidence ≥ threshold → boost (Strong*); if disagrees → dampen
   (Hold or Weak*); if Flat → pass-through.
4. Emit the modulated Signal.
5. Emit one CostEvent::Infra { line: "forecast_inference", … }.
6. Emit one audit row carrying the ForecastOverlay + correlation_id.
```

The "combine" step is the only piece each consumer authors. The other
five steps are boilerplate, lifted into helper functions in
`crates/forecast/src/overlay.rs` so the strategy's tick body stays
short.

Step ordering matters for determinism. The audit row posts AFTER the
cost event AFTER the forecast call. In replay mode, the forecast call
either hits the cache (deterministic) or fails with
`ForecastError::ReplayMiss` (per [ADR-0019](adr/0019-v2-llm-strategy.md)
Q8's strict-replay-only rule, which v2.5 inherits wholesale).

## Replay cache — shared with v2 LLM

Operator-locked decision (carried forward across ADR-0027 → ADR-0028):
**a generic `crates/replay-cache/` crate is extracted within a 2-dev-day
budget; if the extraction exceeds budget, two separate caches are kept
and revisited at v2.5.x.**

Wave A bootstrap (2026-05-16) shipped the extracted crate. The DL
forecaster consumes it via `namespace = "forecast"`. The v2 LLM
migration to the shared crate is deferred to v2.5.x (schema-divergence
concern documented in the Wave A commit).

Both caches share the substantive shape:

- SQLite + WAL mode.
- Cache key = SHA-256 over canonical JSON of `(model_revision, inputs,
  sampling params, seed)`.
- `schema_version` migration column.
- Atomic-write contract.
- Body-SHA-256 indexed lookup.

The LLM cache values are `ChatResponse` (text + tool calls); the
forecaster cache values are `ForecastResponse` (numeric arrays). The
generic `ReplayCache<K, V>` absorbs both with `K = body-SHA`,
`V: Serialize + DeserializeOwned`.

## Audit-row shape — forecast emission

A forecast call posts ONE journal entry to the audit ledger
(`audit::journal` table) with these columns:

| Column            | Value                                              |
|---|---|
| `kind`            | `forecast_emitted` (new TEXT value, additive)      |
| `correlation_id`  | UUID — joins forecast → realised trade close       |
| `strategy_id`     | The consuming strategy's id (e.g. `dl_overlay_momentum`) |
| `symbol`          | Bar symbol                                         |
| `payload_json`    | `ForecastOverlay` serde JSON + cache-hit/miss flag |
| `posted_at`       | 6-digit fractional-second timestamp                |

`kind = forecast_emitted` is open-set TEXT per the precedent in
[architecture/02 § Cross-cutting rules](02-strategy-registry.md#cross-cutting-rules-formalised-by-the-strategy-clusters)
— no schema migration needed.

The `model_revision` value in `payload_json` is SHA-256 over the
canonical bytes of the forecaster's `<sha>.metadata.json` provenance
file. The canonicalisation rules + schema shape are locked at
[ADR-0029](adr/0029-tcn-checkpoint-provenance.md) as a cross-phase
contract — v2.5 (TCN), v2.5a (PatchTST), and v2.5b (vanilla
Transformer) all emit `model_revision` under the same rules. Inspectors
can recompute the SHA from a checkpoint's metadata file without
loading the safetensors body, which is the audit-trail property the
schema is designed to preserve.

The realised-outcome side of the correlation is a v2.5.x follow-up
(`reflection-forecast-residual` brief): once a trade closes, the
reflection-memory loop joins `forecast_emitted` rows to the trade's
`pnl_attribution` row by `correlation_id` and computes the
forecast-vs-realised OHLCV residual as a lesson signal.

## Cost telemetry — `CostEvent::Infra` only

Every forecast call posts:

```rust
CostEvent::Infra {
    line: "forecast_inference".to_string(),
    usd: estimated_kwh * config.energy_cost_per_kwh,  // 0 by default
    period: CostPeriod::PerCall,
}
```

`energy_cost_per_kwh` defaults to zero, which keeps the fixture backtest
reports byte-identical (the cost line is present in the report but the
dollar amount is `$0.00` and the LLM-spend denominator stays unchanged
— see [ADR-0019](adr/0019-v2-llm-strategy.md) Q11). Operators who want
non-zero energy accounting opt in via config; their reports diverge
from the fixture anchors deterministically, which is correct because
operator-config divergence is per-operator-config (not project-wide).

No new `CostEvent` variant. No new ledger account at default config.
If the operator opts in to non-zero energy cost, the existing
`expense:infra:forecast_inference` posting path (which
`cost::CostEvent::Infra` already routes through per
[ADR-0022](adr/0022-cost-telemetry-crate.md)) absorbs it without
further plumbing.

## Crate placement

- `crates/forecast/` — `ForecastProvider` trait, overlay-composition
  helpers, replay-cache wiring. **Model-agnostic** — the concrete
  forecaster (e.g. candle-trained Transformer/TCN for v2.5) plugs in
  here.
- `crates/strategy/src/<consumer>.rs` — the consuming strategy impl
  (one per consuming base-strategy). Calls into `crates/forecast/`
  through the trait.
- `crates/core/src/forecast.rs` — `ForecastOverlay` value type +
  `Direction` enum + `OhlcvBar` + `SamplingParams`. Lives in `core`
  next to `Signal` because both are domain types crossed by every
  consumer.
- **Not** `crates/llm/` (forecasters are not LLMs); **not**
  `crates/models/` (which is the slot for `candle` model definitions
  + training loops, deliberately separate from the `forecast` runtime
  serving boundary).

## Cross-references

- [architecture/02-strategy-registry.md](02-strategy-registry.md) —
  the `Strategy` trait the overlay consumes.
- [architecture/05-llm-and-reflection.md](05-llm-and-reflection.md) —
  the LLM-side precedent for replay caching and provider traits.
- [architecture/10-foundation-libraries.md § Numerics & ML](10-foundation-libraries.md#numerics--ml)
  — `candle` named as the prototyping framework; v2.5 is its first
  concrete consumer.
- [ADR-0028](adr/0028-v25-dl-forecast-overlay-candle.md) — the active
  v2.5 instantiation of this pattern (small custom Transformer/TCN
  trained in `candle`).
- [ADR-0027](adr/0027-kronos-onnx-tract-integration.md) — superseded
  Kronos-targeted ADR (preserved for archaeology).
- [v25-dl-forecast-overlay/feature.md](../v25-dl-forecast-overlay/feature.md)
  — the active v2.5 brief.

## Changelog
- 2026-05-17 (architect): audit-row § extended to reference ADR-0029
  (cross-phase forecaster-checkpoint provenance schema). No shape
  change to the journal table; `model_revision` semantics formalised.
- 2026-05-16 (orchestrator): genericised after the Kronos→candle pivot
  (ADR-0028 supersedes ADR-0027). Removed all Kronos-specific clauses
  from the section; updated cost-event `line` from `"kronos_inference"`
  to `"forecast_inference"`; updated crate-placement notes to flag
  `crates/models/` as the future home for `candle` training loops.
  The model-agnostic design (signal-level overlay, `ForecastProvider`
  trait, replay-cache, audit-row shape) is unchanged.
- 2026-05-16 (architect): file created. Captures the signal-level
  overlay pattern, `ForecastProvider` trait shape, shared
  `ReplayCache<K, V>` extraction policy with 2-day budget,
  `CostEvent::Infra` shape, `ForecastOverlay` value type, audit-row
  shape, and crate placement for v2.5.
