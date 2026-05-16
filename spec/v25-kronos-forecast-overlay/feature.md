---
slug: v25-kronos-forecast-overlay
status: in-progress
owner: architect
updated: 2026-05-16
version: 2.5.0
predecessor: v2-llm-strategy v2.0.0
---

# v2.5 — Kronos foundation-model forecast overlay

## Why

This brief promotes the **v2.5 DL-forecaster slot** from candidate to
analyst-owned per
[product.md → Strategy library roadmap](../product.md#strategy-library--roadmap):
*"v2.5 — DL forecaster — Kronos foundation model primary candidate —
First DL model in production."* The operator greenlit promotion on
2026-05-16 once
[`v2-llm-strategy`](../v2-llm-strategy/feature.md) shipped 2026-05-13;
that was the only blocker, because v2.5 inherits v2's record/replay
cache pattern (Q8) for research-mode determinism.

**What v2.5 is.** A first-class strategy implementation that consumes
Kronos forecasts (next-bar OHLCV with a sampled distribution) and
emits `Signal::buy/sell/hold` through the existing
[`Strategy` trait](../architecture/02-strategy-registry.md). Kronos is a
**decoder-only Transformer** pre-trained on K-line data from 45+ global
exchanges, MIT license, AAAI 2026, 23.8k GitHub stars
([shiyu-coder/Kronos](https://github.com/shiyu-coder/Kronos)). Five
sizes ship on Hugging Face (4.1M / 24.7M / 102.3M / 499.2M params plus
the mini-2k); we consume **pre-trained weights**, no training, no
fine-tuning in v2.5.

**What v2.5 is not.** Not a replacement for any existing v0–v2 strategy.
Not a multi-model ensemble (one model, one strategy). Not an LLM
(Kronos is a Transformer over discrete OHLCV tokens; there is no
prompt and no `LlmProvider` impl). Not a live-trading authorization —
v2.5 ships as a paper-mode-only strategy gated by the regular
[promotion ladder](../product.md#strategy-lifecycle--promotion-gates),
exactly like every previous strategy. Not a fine-tuning pipeline
(deferred to a v2.5.x follow-up if base-model edge proves marginal).

**Why this slot, not a new tier.** The
[pre-evaluation breadcrumb](../dev-notes/kronos-evaluation-2026-05-10.md)
already re-framed the orchestrator's original "v3+" framing: the
v2.5 roadmap row reads literally *"DL forecaster (TCN or small
Transformer)"*, which Kronos is (4.1M to 499M params puts it in the
small-to-medium Transformer class). v3 is RL, v4+ is event-driven —
neither describes Kronos. So v2.5 is the right home.

**Why now.** Three lined-up enablers:

1. **v2 LLM shipped 2026-05-13.** The record/replay cache pattern
   ([v2-llm-strategy feature.md Q8](../v2-llm-strategy/feature.md#v2-llm-strategy-q8--replay-storage-sqlite-at-datallm-replaydb-paper-and-cratesllmtestsfixturesllm-replaydb-fixture-sha-256-hash-over-canonical-json-of-model-system-messages-tools-max_tokens-temperature-schema_version-migration-column-no-lru-cap-one-canned-response-per-provider-per-role-atomic-write-contract-via-the-cost-crate-borrowed-atomic-write-helper))
   is now a shipped primitive: SQLite WAL + SHA-256 over canonical
   JSON inputs + `schema_version` migration column. v2.5 reuses this
   pattern wholesale for research-mode determinism — non-zero
   temperature sampling becomes deterministic via cache hits the same
   way LLM completions did.
2. **Strategy + risk + audit surfaces are stable.** The
   [`Strategy` trait](../architecture/02-strategy-registry.md) has been
   stable since v0; [risk clamping](../architecture/04-risk-and-money.md)
   and the audit ledger absorbed v0–v2 without breaking. Kronos drops
   in as one more `Strategy` impl with no infra movement.
3. **No new universe.** v2.5 stays on the v1 top-10 USDT universe
   ([universe ladder](../product.md#universe--data-fidelity-ladder)),
   matching the data Kronos was pre-trained on (OHLCV from major spot
   pairs). No new venues, no DEX, no perps — those wait for v3+.

**Differentiator alignment.** v2.5 extends the
[moat bet](../product.md#differentiator) — every Kronos forecast posts
a journal entry with `correlation_id`, and lesson cards key on
"forecast vs. realized OHLCV residual" once the trade closes. The
v1.8 reflection-memory loop now has a new signal source to learn
against.

## Requirements

R-numbering matches the v2-llm-strategy convention so the architect's
follow-up tasks can cite individual R-rows.

### R1 — Pre-trained, base model only

**R1.1.** v2.5 ships consuming the Kronos **base** pre-trained
checkpoint (102.3M params, 512-token context) from Hugging Face's
NeoQuasar org. No fine-tuning pipeline ships in v2.5. Justification
in the open-questions table (Q1 + Q2): fine-tuning needs GPU + a
training-data curation pass; pre-trained base hits the "first DL
model in production" promotion bar without that infra. If base
under-performs on backtest scenarios, the v2.5.x follow-up brief adds
fine-tuning.

**R1.2.** Checkpoint version is pinned via Hugging Face revision SHA
in `LlmConfig`-equivalent (likely `ForecastConfig` — architect names).
Operator must update the SHA explicitly to consume a new checkpoint;
no auto-update.

**R1.3.** Checkpoint license verification is part of the build:
NeoQuasar weights are MIT-licensed, and the build script asserts the
license tag on download.

### R2 — Strategy shape: overlay, not pure

**R2.1.** v2.5 ships as an **overlay** strategy, not a pure-Kronos
strategy. Concretely: the new `kronos_forecast` strategy consumes
forecasts and emits a *signal-modulator* that downstream strategies
respect through composition (cf. v0.5 composed strategies
[ADR-0010](../architecture/adr/0010-v05-composed-exit-policy.md)).
First wiring: `kronos_forecast × v1_momentum` — Kronos dampens
momentum entries when it forecasts mean-reversion, boosts them when
it forecasts trend continuation.

**R2.2.** Overlay is the orchestrator's prior from the pre-eval and
matches the v2 LLM news/sentiment overlay pattern shipped at v2.0.0.
Pure-Kronos remains a future option (see Q5 below) but is not in
v2.5 scope.

**R2.3.** Overlay output schema lives in `crates/core` next to the
existing `Signal` enum. Architect names the type — strawman:
`ForecastOverlay { confidence: Decimal, direction: Direction,
horizon_bars: u32 }`.

### R3 — Forecast horizon

**R3.1.** v2.5 ships a **single-bar (next-bar) forecast** only.
Multi-bar rolling forecasts and ensembles across horizons (cf.
Q4 pre-eval) are deferred to v2.5.x.

**R3.2.** Bar granularity matches the strategy it overlays. For the
v1_momentum overlay (R2.1), this is **1h bars** per
[v1-cross-sectional-momentum](../v1-cross-sectional-momentum.md). 1m
overlays are a v2.5.x follow-up.

**R3.3.** Forecast call cadence: once per closed bar. No intra-bar
re-forecasting in v2.5. Audit row posts at forecast emission, not at
signal application.

### R4 — Integration path: ONNX export + tract

**R4.1.** v2.5 ships **Option B — ONNX export + `tract` in-process
inference** (see ## Design integration-path argument below). Pre-eval
listed three options; the analyst confirms Option B over A
(subprocess) and C (candle native).

**R4.2.** ONNX conversion runs as a one-off build script (PyTorch →
ONNX via `torch.onnx.export`); the resulting `.onnx` files commit to
`crates/forecast/assets/` and load at runtime via
[`tract`](https://github.com/sonos/tract). No Python at runtime; no
network access at inference time.

**R4.3.** If ONNX conversion fails for unsupported decoder ops (a
real risk per the pre-eval cons list), the architect spawns a 1-day
spike to either (a) add the op to `tract` or (b) fall back to
Option A (subprocess + IPC). The architect names the deadline; the
fallback is documented but not pre-built.

### R5 — Determinism contract (inherits v2 Q8 pattern)

**R5.1.** Research-mode (`backtest` skill, replay path) MUST be
deterministic — same inputs produce byte-identical reports across
runs. This is the hard rule from
[product.md → Operating modes](../product.md#operating-modes) §1.

**R5.2.** Kronos uses **temperature + nucleus sampling** by default
(non-deterministic). v2.5 inherits the v2-LLM record/replay pattern
([v2-llm-strategy Q8](../v2-llm-strategy/feature.md#v2-llm-strategy-q8--replay-storage-sqlite-at-datallm-replaydb-paper-and-cratesllmtestsfixturesllm-replaydb-fixture-sha-256-hash-over-canonical-json-of-model-system-messages-tools-max_tokens-temperature-schema_version-migration-column-no-lru-cap-one-canned-response-per-provider-per-role-atomic-write-contract-via-the-cost-crate-borrowed-atomic-write-helper)):
SHA-256 over canonical JSON of `(model_revision, input_ohlcv_window,
temperature, top_p, top_k, max_tokens, sampling_seed)`; cache stored
at `data/kronos-replay.db` (paper) and
`crates/forecast/tests/fixtures/kronos-replay.db` (fixture).

**R5.3.** Sampling seed is **explicitly part of the cache key** so
two operator-chosen seeds produce two cache entries (and two
deterministic forecasts).

**R5.4.** `schema_version` column matches the v2 LLM cache shape;
the architect confirms whether the two caches share a Rust crate
(strawman: split into `crates/replay-cache/` with v2.5 as the second
caller).

### R6 — Anchor non-regression (R14.2 / V8 mirror)

**R6.1.** The 9 strategy anchors at
[`spec/anchors.toml`](../anchors.toml) lines 15–58 MUST stay
**byte-identical** after v2.5 ships. Kronos is additive — a new
strategy with a new anchor — never a modification of an existing
strategy's output.

**R6.2.** The 2 `report-sample-*` anchors at
[`spec/anchors.toml`](../anchors.toml) lines 75–83 (`v2.0.0`) **may
move** if Kronos forecasts surface in the System Health section of
the operator success report. Analyst recommends they **do not move**
in v2.5 — surface Kronos forecasts as a follow-up brief
`reporting-kronos-signal-surface` rather than couple report-shape
changes to the strategy ship. If the architect disagrees, route
back to the analyst with rationale.

**R6.3.** v2.5 locks a **new 12th anchor** for the Kronos backtest
scenario (R8.1 below). Naming convention follows existing pattern:
`top10-2024-h1-kronos-momentum` and `top10-2024-h2-kronos-momentum`
— two new anchors (so 13 total post-v2.5).

### R7 — Cost telemetry: `CostEvent::Infra`

**R7.1.** Kronos local inference is **not an LLM call**; it is local
compute. The existing
[`cost::CostEvent::Infra { line, usd, period }`](../../crates/cost/src/event.rs)
variant absorbs this — `line = "kronos_inference"`, `usd` computed
from a per-call energy estimate × the operator's
`KronosConfig.energy_cost_per_kwh` (default zero, opt-in).

**R7.2.** No new `CostEvent` variant is needed. No new ledger account
is needed. The default-zero-energy-cost setting means existing
backtest reports stay byte-identical (covered by R6.1).

**R7.3.** If the operator opts in to a non-zero energy cost, an
`expense:infra:kronos_inference` ledger entry posts per forecast call.
This is a **per-operator-config** posting, not a default behavior, so
fixture-based reports remain deterministic.

### R8 — Backtest scenarios (see ## Backtest Scenarios)

**R8.1.** At least **two anchored backtest scenarios** with concrete
date ranges, one new anchor per scenario. Default scenarios: BTC-USDT
H1 2024 (Jan–Jun) and BTC-USDT H2 2024 (Jul–Dec) — matching the
existing v1 anchor convention (`top10-2024-h1-momentum`).

**R8.2.** Comparison baseline: v1 cross-sectional momentum on the
same date ranges, deterministically replayed. The backtest report
**must include a side-by-side Sharpe / drawdown comparison** so the
operator can read "does Kronos overlay improve momentum?" at a
glance.

### R9 — Crate placement

**R9.1.** New crate `crates/forecast/` houses the Kronos integration:
ONNX loader, `tract` glue, `ForecastProvider` trait (sibling to
`LlmProvider` in shape), replay-cache wiring. Architect confirms
whether the trait lives here or moves to `crates/core/` to mirror
where `Strategy` lives.

**R9.2.** The actual strategy impl (`kronos_forecast`) lives in
`crates/strategy/` next to its peers (sma_cross, momentum, etc.).
`crates/strategy/` calls into `crates/forecast/` through the trait.

**R9.3.** **Not** in `crates/llm/` — Kronos is not an LLM.

## Design

Architect pass landed 2026-05-16. The cross-cutting forecast-overlay
pattern (signal-level composition, `ForecastProvider` trait,
`ForecastOverlay` value type, audit-row shape) is documented in
[`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md).
The v2.5-specific resolutions (Q2 / Q3 / Q4 / Q5 / Q6 / Q7 / Q8) live in
[ADR-0027](../architecture/adr/0027-kronos-onnx-tract-integration.md).
This Design section is the per-feature summary; the ADR is canonical
for conflicts.

### Operator-locked decisions (LOCKED at architect spawn 2026-05-16)

| Q | Resolution | Source |
|---|---|---|
| Q1 | Pre-trained `base` only (102.3M params). No fine-tuning in v2.5. | Operator + R1 |
| Q3 | Option B: ONNX export + `tract` in-process. Subprocess (Option A) is the named fallback. | Operator + R4 |
| Q9 | New crate `crates/forecast/`. | Operator + R9 |
| Q10 budget | Try to extract `crates/replay-cache/` generic `<K, V>` within **2 dev-days**; if exceeded, ship duplicate caches and open a v2.5.x follow-up. | Operator |
| Q11 | Fine-tuning pipeline deferred to v2.5.x. | Operator |
| Q12 | ONNX checkpoint vendored at `crates/forecast/assets/kronos-base.onnx` (git LFS). NOT download-on-first-use. | Operator |
| Q13 | Overlay composition at **signal level** inside `Strategy::tick()`. NOT risk-clamp level. | Operator |
| Backtest baseline | **BS-1 = 2023 full-year top-10 USDT; BS-2 = 2024 full-year top-10 USDT.** Overrides the analyst's H1/H2 default for regime-change evidence. | Operator |

### Architect-decided resolutions (this pass)

| Q | Resolution | One-line rationale |
|---|---|---|
| Q2 | `base` 102.3M / 512-ctx. | Analyst default accepted; mini too small, large 5× cost without K-line quality gain. |
| Q4 | Single-bar (next-bar) horizon only. | 1-bar matches the 1h v1-momentum cadence; multi-bar ensembles deferred to v2.5.x if BS-1/BS-2 are noisy. |
| Q5 | Overlay on v1 momentum (signal-level composition). | Matches v2 LLM news/sentiment overlay; pure-Kronos kept as v2.6 option, no rework cost. |
| Q6 | Inherit v2 LLM record/replay wholesale (SHA-256 over canonical JSON, SQLite WAL, strict-replay-only in research mode). Sampling seed `0xC0FFEE` in cache key. | The v2.0.0 pattern shipped 2026-05-13; reuse beats reinvent. |
| Q7 | 9 strategy + 2 report-sample anchors stay byte-identical; 2 new anchors lock at v2.5 ship (BS-1 + BS-2). Optional 3rd new anchor for BS-2 v1-baseline. | Kronos is additive; default `energy_cost_per_kwh = 0` keeps fixture reports identical. |
| Q8 | `CostEvent::Infra { line: "kronos_inference", usd, period }` with `usd = 0` by default. No new variant; no new ledger account at default config. | Existing `Infra` variant is the right shape; opt-in non-zero energy cost is per-operator config. |

### Crate layout

- `crates/forecast/` — `ForecastProvider` trait + `tract` glue + ONNX
  loader + tokenizer + replay-cache wiring (or shim into shared crate).
- `crates/strategy/src/kronos_momentum.rs` — the consuming `Strategy`
  impl. Composes v1 momentum signal with `ForecastOverlay` per Q13.
- `crates/core/src/forecast.rs` — `ForecastOverlay` value type and
  `Direction` enum, next to `Signal`.
- `crates/replay-cache/` (conditional) — generic `ReplayCache<K, V>`
  if the 2-day extraction budget succeeds; otherwise duplicate caches
  in `crates/llm/src/replay.rs` and `crates/forecast/src/replay.rs`.
- Explicitly **not** `crates/llm/` (Kronos is not an LLM); **not**
  `crates/models/` (that slot is for `candle` training prototypes).

### `tick()` flow — overlay composition

```text
fn tick(bar):
    base_signal = v1_momentum_signal(bar)
    request     = ForecastRequest::from_bar(bar, kronos_config)
    forecast    = forecast_provider.forecast(request).await?
    overlay     = ForecastOverlay::from(forecast)
    signal      = combine(base_signal, overlay, threshold)
    cost_sink.emit(CostEvent::Infra { line: "kronos_inference", usd, period })
    audit.emit(JournalEntry::forecast_emitted(overlay, correlation_id))
    return signal
```

Where `combine`:

- `overlay.direction == base.direction && overlay.confidence ≥ threshold` → boost (`StrongBuy` / `StrongSell`).
- `overlay.direction != base.direction && overlay.confidence ≥ threshold` → dampen (`Hold` or `Weak*`).
- `overlay.direction == Flat || overlay.confidence < threshold` → pass through `base_signal` unchanged.

Threshold is a `KronosConfig.overlay_confidence_threshold` knob
(default `0.6`, operator-tunable, present in the cache key by
construction because it's not a model input — it's a downstream
combination parameter, so it does NOT need to be in the SHA-256
cache key).

### Audit row + correlation_id

One `audit::journal` row per forecast call with
`kind = "forecast_emitted"` (new open-set TEXT value per
[architecture/02 § Cross-cutting rules](../architecture/02-strategy-registry.md#cross-cutting-rules-formalised-by-the-strategy-clusters))
carrying the `ForecastOverlay` payload + `correlation_id`. The
realised-outcome side of the join (forecast → trade close) is a
**v2.5.x follow-up** (`reflection-kronos-residual` brief); v2.5 just
emits the row so the future reflection-memory loop has the signal.

### Out of scope for v2.5 (explicit deferrals)

- Multi-bar rolling forecasts and ensemble-across-horizons (Q4 deferral).
- Pure-Kronos strategy (Q5 deferral).
- Fine-tuning pipeline (Q1 + Q11 deferral).
- Risk-level forecast modulation (Q13 deferral).
- Surface Kronos forecasts in operator success report System Health
  section (per R6.2, a separate `reporting-kronos-signal-surface` brief).
- `ForecastError::ReplayMiss` permissive fallback (strict-replay-only
  at v2.5 ship per ADR-0019 Q8 precedent).
- 1m-bar overlays (R3.2 deferral).
- Mini / small / large checkpoint support (Q2 deferral — base only).

### Risks the architect carries into developer handoff

1. **ONNX op-set compatibility.** Kronos's decoder may use ops `tract`
   doesn't support. M0 includes a thin conversion-spike subtask
   (T-M0-7); if it fails, fallback to Option A or a `tract` upstream
   PR per ADR-0027 Q3.
2. **2-day replay-cache extraction budget.** Real risk the extraction
   takes longer; T-M2-1 carries a hard budget exit so the developer
   ships duplicate caches without blocking the milestone.
3. **Git LFS for the 410 MB checkpoint.** Project hasn't used LFS
   before; M1 includes an LFS-bootstrap subtask.
4. **Tokenizer parity.** The Kronos Python tokenizer must round-trip
   bit-identically with the Rust port; M1 includes a parity test
   (T-M1-3) as a gate.
5. **Strict-replay coverage.** Backtest tooling must populate the
   cache from a fixture before BS-1/BS-2 run; M2 ships a fixture
   build subtask (T-M2-3).

## Backtest Scenarios

**Operator override at architect spawn 2026-05-16.** The analyst
default (2024 H1 + H2) is overridden to **2023 full-year + 2024
full-year** on the top-10 USDT universe. Rationale: two full years
across distinct macro regimes (2023 = post-FTX recovery / spot-ETF
speculation; 2024 = halving + spot-ETF launch) provide regime-change
evidence that an intra-year H1/H2 split cannot. Captured in
[ADR-0027 § Backtest scenarios](../architecture/adr/0027-kronos-onnx-tract-integration.md#backtest-scenarios--2023--2024-full-year-operator-override).

Following the convention from
[`spec/anchors.toml`](../anchors.toml) lines 40–58 and the v1
momentum cadence ([v1-cross-sectional-momentum](../v1-cross-sectional-momentum.md)).

### BS-1 — Kronos overlay on momentum, top-10 USDT 2023 full year

| Field            | Value |
|---|---|
| Universe         | Top-10 USDT spot (matches v1 universe) |
| Date range       | 2023-01-01T00:00:00Z → 2023-12-31T23:59:59Z |
| Granularity      | 1h bars |
| Base strategy    | v1 cross-sectional momentum (unchanged config) |
| Overlay          | `kronos_momentum` with base checkpoint, 512-ctx, next-bar horizon |
| Sampling seed    | `0xC0FFEE` (matches the project's existing fixture seed) |
| Anchor name      | `top10-2023-fy-kronos-momentum` |
| Comparison       | Side-by-side vs existing anchor `top10-2023-1h-momentum` (v1, no overlay) — already locked at line 41–43 |

**Pass criterion (v2.5 ship gate):** Kronos overlay delivers
Sharpe ≥ v1 baseline Sharpe × 1.05 on this scenario (5% lift, modest
because we're consuming a pre-trained base model with no fine-tuning).
If the lift is negative or < 1.05×, the v2.5.x fine-tuning brief
opens automatically.

### BS-2 — Kronos overlay on momentum, top-10 USDT 2024 full year

| Field            | Value |
|---|---|
| Universe         | Top-10 USDT spot |
| Date range       | 2024-01-01T00:00:00Z → 2024-12-31T23:59:59Z |
| Granularity      | 1h bars |
| Base strategy    | v1 cross-sectional momentum |
| Overlay          | Same Kronos config as BS-1 |
| Sampling seed    | `0xC0FFEE` |
| Anchor name      | `top10-2024-fy-kronos-momentum` |
| Comparison       | vs a new v1 baseline anchor `top10-2024-fy-momentum` (architect-preferred: lock at the same tester pass so verify-anchors stays fast). Tester confirms or routes back. |

**Pass criterion:** Same 1.05× Sharpe lift rule on BS-2. The
two-full-years frame (2023 vs 2024) gives the operator two distinct
regime windows — 2023 was post-FTX recovery and spot-ETF
anticipation, 2024 was the halving + spot-ETF launch macro shift. If
BS-1 passes but BS-2 fails, the operator + analyst discuss regime
sensitivity before promoting.

### BS-3 — Anchor non-regression sweep (mandatory)

Re-run **all 9 strategy anchors + 2 report-sample anchors** with the
v2.5 build to prove non-regression (R6.1 + R6.2). This is the
existing `scripts/verify_anchors.sh` invocation; tester gate. No new
anchor locked here.

### Anchor count summary (post-v2.5 ship)

| Anchor | Status |
|---|---|
| 9 existing strategy anchors (lines 15–58) | byte-identical |
| 2 existing `report-sample-*` anchors (lines 75–83) | byte-identical |
| `top10-2023-fy-kronos-momentum` (BS-1) | NEW, locked at tester pass |
| `top10-2024-fy-kronos-momentum` (BS-2) | NEW, locked at tester pass |
| `top10-2024-fy-momentum` (BS-2 baseline) | NEW conditional, locked if architect-preferred path |

Architect-preferred count post-v2.5: **14 anchors**. Tester may
choose 13 (no separate 2024 baseline anchor; computed live each
verify-anchors run) if the BS-2 baseline runtime stays acceptable.

## Implementation

### Wave A — M1 + M2 (developer, 2026-05-16)

**Crates created:**

- `crates/replay-cache/` — generic `ReplayCache<K, V>` SQLite WAL
  primitive. `schema_version = 1`, canonical-JSON SHA-256 keys,
  strict-replay mode (`ReplayCacheError::Miss`), read-only + read-write
  open modes. 8 unit tests green.
- `crates/forecast/` — `ForecastProvider` async trait,
  `KronosForecaster` stub (returns `ForecastError::Inference` until M3),
  `overlay::combine()` pure function for signal-level composition.
  15 unit tests green.
- `crates/core/src/forecast.rs` — `ForecastOverlay`, `Direction`,
  `ForecastRequest`, `ForecastResponse`, `ForecastError`, `OhlcvBar`,
  `SamplingParams`. 7 inline unit tests green. `confidence` is
  `rust_decimal::Decimal` throughout.

**M2 ONNX vendoring:**

- `build.rs` checksum gate: skips gracefully when ONNX absent (PENDING hash);
  asserts SHA-256 match once ONNX is present.
- `.gitattributes` declares `crates/forecast/assets/*.onnx filter=lfs`.
- `scripts/dev/kronos_torch_to_onnx.py` — one-off PyTorch → ONNX
  conversion script with license assertion (MIT) + SHA-256 write.
- `crates/forecast/README.md` — LFS bootstrap documentation.
- **BLOCKER T-M2-3**: `git-lfs` not installed on dev machine;
  `kronos-base.onnx` not committed. Operator must run LFS bootstrap
  before M3 can proceed. T-M2-4 gate is written and active.

**T-M1-4 replay-cache extraction note:**

`crates/replay-cache/` extracted successfully; `crates/forecast` consumes
it. `crates/llm` NOT migrated (schema divergence: `llm_replay` vs
`replay_cache` table name; would require LLM migration + fixture DB changes
— not within 2-dev-day budget risk profile). LLM still uses its own
replay module unchanged. `cargo test --workspace` green (zero regressions).

**Gate results:**

- `cargo check --workspace` — PASS
- `cargo clippy -- -D warnings` — PASS (no errors, one build-script warning about PENDING)
- `cargo test --workspace` — PASS (all tests ok, zero failures)
- `cargo test -p replay-cache` — 8 passed
- `cargo test -p forecast` — 15 passed
- `cargo test -p trading_core forecast` — 7 passed
- `cargo test -p llm` — 84+... passed, zero regressions

## Verification

_tester fills this._

Tester runs at minimum:

- `cargo test -p forecast` (new crate unit tests).
- `cargo test -p strategy kronos_momentum` (strategy integration).
- `backtest` skill with BS-1, BS-2, BS-3 scenarios above.
- `scripts/verify_anchors.sh` → **`ANCHORS PASS (13/13)` or
  `ANCHORS PASS (14/14)`** depending on whether the tester locks the
  optional BS-2 v1-baseline anchor — including the 2 new
  `top10-{2023,2024}-fy-kronos-momentum` anchors.
- `spec-lint` and `verify-anchors` mandatory pre-tick gates.

## Open questions

The 8 pre-eval questions plus 5 new ones (Q9–Q13). Each carries a
**default recommendation** (operator can quick-accept) and a
**cost-if-wrong** so the operator knows what's at stake. Marked
`[OPERATOR]`, `[ARCHITECT]`, or `[ANALYST]` for the routing.

### Q1 — Pre-trained vs fine-tuned [OPERATOR]

- **Default:** Pre-trained base only (R1.1).
- **Cost if wrong:** If pre-trained base under-performs on BS-1/BS-2,
  v2.5 ships without an edge claim and a v2.5.x fine-tuning brief
  opens. Cost = roughly one analyst + architect + developer round
  (~2 weeks elapsed) plus a one-time GPU spend (~$50–200 cloud for
  a small fine-tune on 2023 BTC/USDT data).

### Q2 — Which model size [ARCHITECT]

- **Default:** `base` (102.3M params, 512-ctx). Mini (4.1M, 2048-ctx)
  is too small for serious forecast quality; small (24.7M) is
  bigger-than-mini but same 512 ctx as base; large (499M) is 5×
  inference cost without commensurate quality on K-line tasks per
  the Kronos paper.
- **Cost if wrong:** Wrong size = mediocre forecasts. If base is too
  small for our universe, switch to large (+5× inference latency,
  still in-budget on Apple Silicon). If base is too large for
  latency budgets, drop to small. Architect re-decides without
  spec re-write.

### Q3 — Integration path (subprocess / ONNX+tract / candle) [ARCHITECT]

- **Default:** ONNX export + `tract` (Option B per R4.1).
- **Cost if wrong:** If ONNX conversion fails on unsupported decoder
  ops, architect spawns a 1-day spike — outcome is either a `tract`
  PR or a switch to Option A (subprocess + IPC, ~1 week of plumbing
  to add Python deployment surface). Worst case adds 1–2 weeks
  before BS-1 can run.

### Q4 — Forecast horizon [ANALYST]

- **Default:** Single-bar next-bar forecast (R3.1).
- **Cost if wrong:** Single-bar forecasts may be too noisy for a
  1h overlay. If BS-1 Sharpe is unstable, a v2.5.x brief adds a
  rolling N-bar ensemble. Cost = one developer round; no architecture
  movement.

### Q5 — Strategy shape (pure vs overlay) [ANALYST]

- **Default:** Overlay on v1 momentum (R2.1).
- **Cost if wrong:** If the overlay composition pattern doesn't
  produce a clean signal (e.g. Kronos and momentum disagree often),
  a pure-Kronos strategy ships as a v2.6 brief. The v2.5 work is
  not wasted — the ONNX/tract integration carries forward.

### Q6 — Determinism contract [ARCHITECT]

- **Default:** Inherit v2 LLM record/replay (R5.1–R5.4).
- **Cost if wrong:** If the cache key isn't right (e.g. sampling
  seed not actually used by the model's RNG), backtests are
  non-deterministic and the tester catches it on the first
  `verify_anchors` run. Cost = one architect-developer round to
  fix the key shape. No data loss because the cache is regenerable.

### Q7 — Anchor impact [TESTER]

- **Default:** 9 strategy anchors stay byte-identical; 2
  `report-sample-*` anchors stay byte-identical; 2 new anchors
  locked (R6.1–R6.3).
- **Cost if wrong:** If a report-sample anchor moves, tester routes
  back to architect to decide whether to re-lock (R14.2 / T1937
  precedent from v2.0.0) or revert the report-shape change. Cost
  = half a round.

### Q8 — Cost telemetry shape [ARCHITECT]

- **Default:** `CostEvent::Infra { line: "kronos_inference", ... }`
  with default-zero usd (R7.1–R7.3).
- **Cost if wrong:** If the operator wants per-token-style cost
  accounting (matching LLM `CostEvent::Llm`), architect adds a new
  variant. Cost = one architect-developer round for the schema
  change + ledger account.

### Q9 — Crate placement: new `crates/forecast/` vs absorb into existing crate [ARCHITECT]

- **Default:** New crate `crates/forecast/` (R9.1).
- **Cost if wrong:** If the crate has too few callers to justify
  its existence, fold into `crates/strategy/`. Cost = one cargo
  refactor PR; no behavior change.

### Q10 — Shared `ReplayCache<K, V>` crate, or duplicate the v2 pattern in-place? [ARCHITECT]

- **Default:** Extract a `crates/replay-cache/` generic over `K, V`
  and migrate v2 LLM's `crates/llm/src/replay.rs` to use it as the
  same brief.
- **Cost if wrong:** If extraction is harder than expected (the v2
  LLM cache has provider-specific bits), keep the two caches
  separate and accept the duplication. Cost = the architect names
  the budget; if extraction exceeds 2 dev-days, abort and ship
  separate.

### Q11 — Fine-tuning data curation pipeline — in v2.5 or v2.5.x? [OPERATOR]

- **Default:** **Deferred to v2.5.x.** v2.5 ships pre-trained only
  (R1.1).
- **Cost if wrong:** If the operator wants fine-tuning in v2.5,
  scope doubles: GPU procurement, training-data curation, fine-tune
  scripts, checkpoint signing, separate replay cache for fine-tuned
  outputs. Add ~3 weeks elapsed.

### Q12 — ONNX export: build-script-on-CI vs vendored artifact in git? [ARCHITECT]

- **Default:** **Vendored `.onnx` files** committed to
  `crates/forecast/assets/` (small enough — base model is ~410 MB).
  Build-script-on-CI requires a Python toolchain on every developer
  machine, which contradicts R4.2's "no Python at runtime."
- **Cost if wrong:** If git LFS gets unwieldy at base+large+small
  checkpoint count, switch to a download-on-first-use cache.
  Cost = one developer round.

### Q13 — Overlay composition: signal-level (Strategy::tick) vs risk-level (clamp) [ARCHITECT]

- **Default:** **Signal-level** — Kronos overlay modulates `Signal`
  inside the strategy's `tick()` (matches v0.5 composed strategies).
- **Cost if wrong:** If overlay-at-risk-level proves cleaner (e.g.
  Kronos suggests a position size hint that risk respects), refactor
  in v2.5.x. Cost = one architect-developer round.

## Integration-path argument (R4 expanded)

The analyst is recommending **Option B (ONNX + `tract`)** over Option
A (subprocess + IPC) and Option C (candle native). Four-axis argument
per the brief:

### (a) Determinism guarantees

- **Option A (subprocess):** Python's `torch.use_deterministic_algorithms(True)`
  plus seed-pinning is well-understood but adds an IPC layer where
  serialization edge cases (NaN, float-precision) can creep in.
  Manageable but adds surface.
- **Option B (ONNX + tract):** ONNX export bakes the computation
  graph into a closed-form artifact. `tract` is pure Rust,
  deterministic by default (no GPU non-determinism, no thread-pool
  ordering), and reads bit-identically across our two supported
  platforms (macOS Apple Silicon dev, Linux x86_64 paper deploy).
  **Wins this axis.**
- **Option C (candle):** `candle` also pure Rust but lower-level —
  we'd re-implement the Kronos decoder ops and weight-loading
  ourselves. More surface area for subtle non-determinism.

### (b) Anchor-byte stability

- **Option A:** IPC serialization shape is a versioned protocol;
  any Python-side library upgrade may shift the protocol. Need a
  pin file (poetry.lock, conda env spec) maintained.
- **Option B:** `tract` version pinned in `Cargo.toml`; ONNX
  artifact pinned by SHA in `crates/forecast/assets/`. Cargo
  workspace already gives byte-stable builds. **Wins this axis.**
- **Option C:** Same as B for Rust pinning, but we own more code
  → more bug surface for anchor-affecting changes.

### (c) Operational footprint

- **Option A:** Two processes to supervise (Rust agent + Python
  worker); Python deployment surface (Conda / venv / poetry);
  process-restart logic on Python crashes; IPC error handling.
  Worst footprint.
- **Option B:** One Rust binary; one extra `crates/forecast/`
  dependency; one ONNX file per checkpoint at deploy time.
  **Wins this axis.**
- **Option C:** Same as B but the `crates/forecast/` crate is
  larger because it contains the decoder impl.

### (d) Speed to first ship

- **Option A:** Fastest *initial spike* (just call Python). But
  the "real" integration (process supervision, IPC framing,
  deployment) drags. Net = slowest to a production-quality ship.
- **Option B:** ONNX conversion is the only novel work; `tract`
  is established for this project (`CLAUDE.md` names it as the
  ONNX serving default). 1–2 days conversion + 1 week tract
  glue + 1 week strategy wiring = ~2.5 weeks to BS-1 run.
  **Wins this axis** unless ONNX conversion blocks (Q3
  fallback).
- **Option C:** 4+ weeks. Re-implementing the decoder is the bulk
  of v2.5 effort and adds maintenance burden.

**Recommendation:** B. Subprocess (A) is the named fallback per
R4.3 if ONNX conversion proves impossible.

**Deviation from the pre-eval's hint:** None — the pre-eval also
recommended B. The analyst confirms.

## Changelog

- 2026-05-16 (architect): Design section authored, replacing the
  analyst's stub. Q4 (single-bar horizon), Q5 (signal-level overlay
  on v1 momentum), Q6 (inherit v2 LLM record/replay), Q7 (2 new
  anchors at ship; 11 existing stay byte-identical), Q8
  (`CostEvent::Infra` with default-zero usd) resolved with rationale.
  Operator decisions Q1/Q3/Q9/Q10-budget/Q11/Q12/Q13 recorded
  verbatim. Backtest Scenarios updated per operator override:
  BS-1 = 2023 full-year top-10 USDT (was 2024 H1); BS-2 = 2024
  full-year top-10 USDT (was 2024 H2). New cross-cutting
  architecture file [`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md)
  documents the `ForecastProvider` trait, `ForecastOverlay` value
  type, and signal-level overlay composition pattern.
  [ADR-0027](../architecture/adr/0027-kronos-onnx-tract-integration.md)
  ratifies the v2.5 instantiation (Option B ONNX + `tract`, base
  102.3M, 2-day replay-cache extraction budget). Frontmatter:
  `owner: analyst → architect`, `updated: 2026-05-16`. HANDOFF →
  developer.
- 2026-05-16 (analyst): promoted from `candidate` → `in-progress`.
  Authored Why, Requirements (R1–R9), Backtest Scenarios (BS-1 to
  BS-3), and the 13 open questions (Q1–Q13). Integration path
  resolved to Option B (ONNX + `tract`) with a four-axis argument.
  Design / Implementation / Verification left as architect /
  developer / tester stubs per the spec-update skill skeleton.
  Frontmatter shape: `status: in-progress`, `owner: analyst`,
  `version: 2.5.0`, `predecessor: v2-llm-strategy v2.0.0`,
  `updated: 2026-05-16`. The previous "candidate stub" text
  authored 2026-05-10 by the orchestrator is replaced by this
  expanded brief; the cross-reference list it carried is
  preserved through the architecture/02 + Q8 v2-LLM links
  embedded in the requirements above. Inherits the
  [v2 LLM record/replay pattern](../v2-llm-strategy/feature.md#v2-llm-strategy-q8--replay-storage-sqlite-at-datallm-replaydb-paper-and-cratesllmtestsfixturesllm-replaydb-fixture-sha-256-hash-over-canonical-json-of-model-system-messages-tools-max_tokens-temperature-schema_version-migration-column-no-lru-cap-one-canned-response-per-provider-per-role-atomic-write-contract-via-the-cost-crate-borrowed-atomic-write-helper)
  per R5. The 9 strategy + 2 report-sample anchors at
  [`spec/anchors.toml`](../anchors.toml) MUST remain byte-identical
  (R6.1 / R6.2). Two new anchors lock at v2.5 ship (R6.3).
  HANDOFF → architect.
- 2026-05-10 (orchestrator): stub created during v2-llm-strategy
  pause. See pre-eval at
  [`spec/dev-notes/kronos-evaluation-2026-05-10.md`](../dev-notes/kronos-evaluation-2026-05-10.md)
  for the technical breadcrumb (license, integration paths,
  context-window caveats, the 8 original open questions).
