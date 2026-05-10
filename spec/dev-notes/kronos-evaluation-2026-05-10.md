# Kronos foundation model — pre-analyst evaluation (2026-05-10)

This is a **forward-compatibility breadcrumb** for the eventual analyst
who picks up the v2.5 DL-forecaster strategy slot
([product.md → Strategy library roadmap](../product.md#strategy-library--roadmap)).
It captures the technical evaluation of [Kronos](https://github.com/shiyu-coder/Kronos)
done at the architect→developer pause for v2-llm-strategy 2026-05-10,
so the analyst doesn't redo the survey.

## TL;DR

Kronos is a decoder-only Transformer foundation model pre-trained on
K-line (candlestick) data from 45+ global exchanges. It tokenises
multi-dimensional OHLCV into hierarchical discrete tokens, then
autoregressively forecasts next-bar OHLCV. Five sizes ship pre-trained
on Hugging Face (4.1M to 499M params); MIT license; AAAI 2026 paper.
**It's the strongest off-the-shelf candidate for the v2.5 DL-forecaster
slot** — supersedes the original "TCN or small custom Transformer"
strawman because we'd skip training entirely and consume pre-trained
weights.

The v2.5 row in product.md was originally written as "TCN or small
Transformer" (architect-side strawman; never spawned). Kronos
specialises that slot to a specific pre-trained foundation model
candidate. The analyst makes the final call when v2.5 is promoted —
this note is one input among several, not a commitment.

## Why this is the v2.5 slot, not a new v3+ tier

Re-framing on closer reading of product.md:

- `spec/product.md:183` v2.5 row reads: *"DL forecaster (TCN or small Transformer) — First DL model in production"*. Kronos is a small-to-medium Transformer (4.1M–499M params); it fits the slot description literally.
- v3 row reads: *"RL policy on constrained action space — Learning agent"*. Kronos is not RL.
- v4+ is *"Event-driven (listings, exploits, regime shifts)"*. Kronos is not event-driven.

So the right home is v2.5. The original orchestrator framing on
2026-05-10 was "v3+" but that conflated "later than v2 LLM" with "new
roadmap tier"; the v2.5 slot already exists.

## Source repository

- **Repo:** [shiyu-coder/Kronos](https://github.com/shiyu-coder/Kronos)
- **License:** MIT
- **Paper:** AAAI 2026 (preprint arXiv:2508.02739)
- **Maturity at survey time:** 23.8k stars, 4.2k forks, 156 open issues,
  active maintenance through 2025; fine-tuning scripts released
  2025-08.
- **Pre-trained weights:** Hugging Face Hub, "NeoQuasar" organisation.

## Technical fit assessment

### What Kronos provides

| Dimension | Detail |
|---|---|
| Model family | Decoder-only autoregressive Transformer |
| Sizes | mini 4.1M, small 24.7M, base 102.3M, large 499.2M params |
| Context window | 2048 tokens (mini) / 512 tokens (others) |
| Input | Pandas DataFrames of OHLCV (Open / High / Low / Close / Volume) |
| Tokenisation | Hierarchical discrete tokens via domain-specific tokenizer |
| Output | Forecasted OHLCV + optional volume / amount |
| Sampling | Temperature + nucleus (top-p) — probabilistic forecasting |
| API surface | `KronosPredictor` class — preprocessing, normalisation, prediction, denormalisation; `predict_batch` for GPU parallelism |

### What our project provides that Kronos consumes / pairs with cleanly

| Dimension | What we have | How Kronos plugs in |
|---|---|---|
| Data | `crates/data/` reads Binance / Coinbase / Kraken via WebSocket + Parquet historical | OHLCV is exactly what Kronos tokenises |
| Feature pipeline | `crates/features/` computes indicators | Kronos forecasts could be one more feature column upstream of strategy |
| Strategy trait | `crates/strategy/` has stable `Strategy` trait since v0 | Kronos forecast → `Signal::buy/sell/hold` via a new `kronos_forecast` strategy impl |
| Risk | `crates/risk/` enforces typed limits | Kronos signal flows through risk-clamping unchanged |
| Execution | `crates/exec/` paper + multi-venue live | No change |
| Audit | `crates/audit/` records every decision | Kronos forecast lands as a journal entry with `correlation_id` |
| Cost telemetry | `crates/cost/` (post-v2 LLM ship) | Kronos is a local inference path; if we use the HF-served API, `CostEvent::Llm`-shaped accounting may apply (not LLM but a paid API call) — analyst confirms |

## Integration paths (analyst picks one)

### Option A — Subprocess + IPC

- Spin up a Python process holding the loaded `KronosPredictor`.
- Rust agent sends an OHLCV slice via stdin/JSON; Python returns
  forecasted OHLCV.
- Precedent in this codebase: `crates/reports/src/bin/report.rs` is
  invoked by the kill-switch via `std::process::Command` (see
  [`crates/agent/src/kill_switch.rs:106`](../../crates/agent/src/kill_switch.rs)).
- Pros: zero Rust ML dependencies; clean boundary; can swap models by
  changing the Python script.
- Cons: per-call IPC overhead (~milliseconds); two processes to
  supervise; Python deployment surface (Conda / venv / poetry).

### Option B — ONNX export + `tract`

- Export Kronos to ONNX (vanilla decoder Transformer; should be
  straightforward via `torch.onnx.export` or `optimum`).
- Load via `tract` (already named in `CLAUDE.md` as the ONNX serving
  default for the project).
- Pros: in-process inference; no Python at runtime; matches the
  project's stated ML serving stack.
- Cons: one-time ONNX conversion work; some Transformer ops may need
  `tract` op-set updates; model size in process memory.

### Option C — Candle (pure-Rust ML)

- Re-implement the Kronos decoder in `candle` (the project's
  prototyping ML framework per `CLAUDE.md`); load HF weights.
- Pros: idiomatic Rust top-to-bottom.
- Cons: highest implementation cost; Kronos uses HF-format weights
  the architect must thread through.

**Orchestrator's prior:** Option B (ONNX + `tract`) most likely; it
matches the project's stated ML serving default and keeps deployment
to a single Rust process. Subprocess (Option A) is the fallback if
ONNX conversion proves harder than expected.

## Caveats the Kronos authors flag (and our response)

| Author caveat | Our response |
|---|---|
| Raw signals are NOT production alpha — need portfolio optimisation + risk-factor neutralisation. | We have both: `crates/risk::size_portfolio_target` does portfolio optimisation; risk-factor neutralisation is the job of `crates/risk` exposure caps. ✓ |
| Need transaction-cost modelling, slippage, market impact. | `crates/exec/` has `bps: 2` slippage + `0.04%` taker fee in the paper engine; live exec at v1.5b has multi-venue routing. ✓ |
| Backtest example is a basic top-K strategy. | Our backtest harness already runs 9 anchored scenarios across v0/v0.5/v1/v1.5a/v1.5b; we can run Kronos as one more scenario without harness changes. ✓ |
| Some `finetune/` code comments AI-generated by Gemini 2.5 Pro and may contain inaccuracies. | We consume the model, not the fine-tuning code. Irrelevant. ✓ |
| Max context = 512 (small / base) or 2048 (mini). | **Real constraint.** At 1m bars, 512 = 8.5h of context. Fine for intraday momentum; weak for multi-week regime patterns (our v1.5a pairs strategy depends on 60+ minute lookbacks but not multi-day). The mini model's 2048-token context = 34h at 1m — better but still intraday. **Analyst confirms which strategy timeframes Kronos serves at promotion time.** |

## Open questions for the analyst (when v2.5 is promoted)

1. **Q1 — Pre-trained vs fine-tuned.** Ship the pre-trained model
   directly, or fine-tune on our specific universe (BTCUSDT / Top-10
   USDT / pairs)? Fine-tuning needs GPU + training data (we have
   Parquet for 2023+); pre-trained skips that but may underperform on
   our specific markets.
2. **Q2 — Which size.** mini (4.1M, 2048 ctx) vs small (24.7M, 512
   ctx) vs base (102.3M, 512 ctx). Tradeoff is forecast quality vs
   inference latency vs context length vs memory.
3. **Q3 — Integration path.** A (subprocess) / B (ONNX + tract) / C
   (candle native). Orchestrator prior is B.
4. **Q4 — Forecast horizon.** Kronos predicts next-bar OHLCV; how
   many bars ahead does the strategy consume? 1-bar (next-tick) vs
   N-bar rolling forecast vs ensemble across N forecasts at varying
   horizons.
5. **Q5 — Strategy shape.** Pure Kronos (only consumes Kronos
   forecasts) vs overlay (Kronos forecast modulates an existing
   strategy's signal — e.g. dampens momentum when Kronos predicts
   reversal). Overlay is closer to the v2 LLM news/sentiment overlay
   pattern and is the orchestrator's prior.
6. **Q6 — Determinism.** Kronos uses temperature + nucleus sampling
   — non-deterministic by default. For research-mode determinism
   (per `spec/product.md:290+` operating modes), sampling temperature
   must be 0 OR we record/replay forecasts via the v2-llm-strategy
   replay-cache pattern (Q8 of `spec/v2-llm-strategy/feature.md`).
7. **Q7 — Anchor impact.** A new strategy adds a new anchored
   backtest scenario; the 9 existing strategy anchors stay
   byte-identical (the new strategy is additive). The 2 v1+
   `report-sample-*` anchors may or may not move depending on
   whether Kronos forecasts surface in the System Health section.
8. **Q8 — Cost telemetry.** Local inference is not an LLM call but
   does consume compute / energy. Does it post a `CostEvent::Infra`
   row? `cost::CostEvent` already has an `Infra { line, usd, period }`
   variant scaffolded for v1+ (see `spec/architecture.md:2891`).

## Status as of 2026-05-10

- **Not promoted.** Sits in `spec/backlog.md` Strategy queue as a
  candidate for the v2.5 slot.
- **Stub feature folder:** [`spec/v25-kronos-forecast-overlay/feature.md`](../v25-kronos-forecast-overlay/feature.md)
  with `status: candidate`. Holds the slot in the file system + a
  pointer back at this dev-note.
- **No analyst spawn.** The v2.5 promotion happens after v2 LLM
  ships (which itself is paused at architect→developer handoff —
  see [`spec/v2-llm-strategy/orchestrator-scope-check-2026-05-10.md`](../v2-llm-strategy/orchestrator-scope-check-2026-05-10.md)).

## Authoring context

This breadcrumb was authored by the orchestrator after the operator
asked *"Can we learn something from it and update our product?"*
during the v2-llm-strategy pause. The operator approved the
recommendation: capture as v2.5 candidate, no spec rewrite of the
v2 brief, no commitment to ship in any specific window.

The orchestrator's framing of "v3+" in the conversation that led
here was wrong on closer inspection of `spec/product.md:183`; v2.5
is the correct tier per the existing roadmap. This note carries the
re-framing forward.
