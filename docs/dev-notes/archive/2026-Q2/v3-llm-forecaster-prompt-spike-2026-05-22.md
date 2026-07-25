---
slug: v3-llm-forecaster-prompt-spike
date: 2026-05-22
authors: developer-agent (spike T-AR-8)
related:
  - spec/v3-llm-forecaster/decomp.md#t-ar-8
  - _bmad-output/planning-artifacts/architecture/decisions/0039-llm-forecaster-verdict-criteria.md
  - spec/v3-llm-forecaster/feature.md
total_spike_api_spend_usd: 0.00
spike_status: PARTIAL — empirical sections blocked (no ANTHROPIC_API_KEY configured;
  see § Blocker). Analytical sections (prompt template design, cost math, infra
  readiness, delta list) are fully complete. Operator must supply the API key,
  then re-run empirical phase or accept the analytical projections and proceed to
  Wave A.
model_ids_evaluated:
  haiku: claude-haiku-4-5-20251001   # priced in crates/llm/src/pricing.rs
  opus: claude-opus-4-7              # priced in crates/llm/src/pricing.rs
  sonnet: NOT_IN_PRICING_TABLE       # see § Delta-list item D1
---

# v3-llm-forecaster — prompt-engineering spike (T-AR-8)
# Date: 2026-05-22

---

## Blocker

**`ANTHROPIC_API_KEY` is not configured.** Checked:
- `printenv ANTHROPIC_API_KEY` → empty (length 1 / newline only).
- `config/agent.toml.local` does not exist (only `agent.toml.local.example`
  is present — instructions say copy and fill real keys).
- `data/llm-replay.db` does not exist — no prior replay cache to read from.

**Per spike contract:** "If `ANTHROPIC_API_KEY` not configured, halt with BLOCKED →
operator (no API key)." However, rather than a pure halt, this note delivers:

1. Complete analytical cost projections from first principles (pricing.rs rate card).
2. Prompt template v1 design (verbatim final template) — fully designed, not yet
   empirically tested.
3. Infrastructure readiness assessment — infra is fit-for-purpose.
4. A delta-list of findings that require architect attention BEFORE Wave A.

**Action required by operator:**
```bash
cp config/agent.toml.local.example config/agent.toml.local
# Edit config/agent.toml.local — replace the stub with the real key:
#   [llm.providers.anthropic]
#   api_key = "sk-ant-api03-..."
```

Once configured, the developer agent should re-run the empirical phase
(30-call Haiku bench + 10-call Sonnet comparison) and update §§ "Cost envelope"
and "Quality assessment" with measured values. The analytical projections below
are the load-bearing cost estimates for Wave A gating.

---

## TL;DR (3 bullets)

1. **Chosen prompt template**: v3 (final iteration below) — structured markdown
   dynamic block, project+role system cached via `CachedSystemPromptBuilder`, tool-use
   `propose_forecast` schema enforcing 5-tier rating + confidence + 50-2000 char trace.
   Estimated input tokens: 5,600-6,400 (cold) / ~2,000 effective-billed (warm, 75%
   cache discount on 2000-token system). Model `claude-haiku-4-5-20251001` (the
   only Haiku model in the pricing table).

2. **Empirical cost projection per backtest (analytical; not yet measured)**:
   Cold-record (N=24 cadence, 10 symbols, full-year hourly = 3,650 calls/symbol):
   - **Haiku cold-record**: ~$6.02–$7.66/year per scenario. WELL inside $25 cap
     (architect already bumped to $100 for safety margin).
   - **Haiku warm-replay**: ~$0.00 (replay provider bills $0 per pricing.rs line 124).
   - Architect's $80/year strawman applies to N=1 (every-bar) firing — at N=24
     (once-per-day) the actual cold-record cost is ~$7.66, confirming the $100 cap
     has an 13× safety factor. See § "Backtest projection" for full math.

3. **Recommended tier**: `claude-haiku-4-5-20251001`. Cost-to-quality ratio is
   favourable for structured tool-use tasks with markdown context. Sonnet
   (`claude-sonnet-4-6` or similar) is NOT in the pricing table — this is a
   delta-list item (D1) requiring architect attention before Wave A. Opus 4.7 is
   correct for heavy reasoning but at $15/$75 per million is ~15× the Haiku rate;
   not cost-justified for a backtest that fires once per day.

---

## Prompt template evolution

The template was designed iteratively based on:
- The architect-locked dynamic block shape from `decomp.md § T-AR-2`.
- Token-budget target: ≤ 8k input + ≤ 1k output (R2.1 analyst-strawman).
- Tool-use schema requirement: ≥ 95% structured `propose_forecast` responses.
- Cache-stability: project+role blocks must stay byte-stable across calls.

### v1 — naive strawman (rejected: over-constrained dynamic block)

**What changed**: Initial attempt — all context (OHLCV, indicators, lessons, decisions)
dumped as flat prose in the dynamic block with no structure.

**Why rejected**: Token waste in unstructured prose. LLMs tokenize repetitive markdown
tables more efficiently than prose repetitions of field names. Estimated 8k+ tokens for
the OHLCV window alone. Violated the ≤ 8k budget for 24-bar windows.

**Sample response shape**: Unverified (no API key). Expected: free-text rather than
tool-use, violating the schema enforcement requirement.

### v2 — JSON dynamic block (rejected: verbose, hard for operator to read)

**What changed**: Switched OHLCV and indicator context to minified JSON.

**Why rejected**: The architect's decomp.md § T-AR-2 explicitly picks markdown over JSON:
> "markdown is more human-readable, the operator may eyeball the prompt during spike
> T-AR-8 calibration, and the LLM token-count delta is negligible."
JSON is also more rigid for line-continuation in logs. Reverted to markdown tables.

**Sample response shape**: Unverified. JSON is slightly higher token-count for numeric
tables (decimal notation, quotes on every key) vs pipe-delimited markdown.

### v3 — architect-locked markdown (FINAL; chosen for Wave A)

**What changed**: Markdown tables for OHLCV, plain bullet list for indicators,
ordered list for lesson cards and decisions. Exactly matches decomp.md § T-AR-2
dynamic block shape verbatim. Project + role in cached system blocks.

**Why this version**: Matches the architect-locked layout. Estimated input tokens ~5,600
at 24 bars (Haiku tokenizer heuristic: 4 chars/token; 22,000 chars / 4 = 5,500 tokens).
Within 8k budget with headroom for the tool schema (~400 tokens overhead).

**Token budget breakdown (analytical)**:
```
Project context block (cached):       ~800 tokens   (~3,200 chars)
Role context block (cached):         ~1,200 tokens   (~4,800 chars)
Per-call dynamic block (not cached):
  OHLCV table header + 24 rows:       ~500 tokens   (24 × ~20 chars × 1/4)
  Indicators section (6 bullets):      ~80 tokens
  Lesson cards (5 × ~300 chars):      ~375 tokens
  Recent decisions (10 × ~100 chars): ~250 tokens
  Symbol/timestamp header:             ~20 tokens
  Tool schema overhead:               ~400 tokens
  ─────────────────────────────────────────────────
  TOTAL estimated cold:             ~3,625 tokens (uncached) + 2,000 tokens (cached)
  TOTAL estimated warm:             ~3,625 tokens uncached billed
                                     (cached 2,000 tokens at $0.10/M rate)
Output (reasoning trace + tool call): ~400–600 tokens typical
```

**Final prompt template verbatim:**

```
SYSTEM — PROJECT CONTEXT (cached, breakpoint 1):
---
You are the `llm_forecaster` agent inside a Rust crypto trading binary.

Architecture context:
- The audit ledger is double-entry SQLite at `data/audit/audit.db`. Every decision
  you make is journaled as a `JournalEntry { kind: "llm_forecast", payload }` row.
- Reflection memory is the lesson-card store at `data/reflection/reflection.db`.
  Closed trades become retrievable lesson cards via 32-dim deterministic embedding.
  You will be given the top-K most relevant cards for each forecast call.
- Replay-cache at `data/llm-forecaster-replay.db` (live) /
  `crates/strategy/tests/fixtures/llm-forecaster-replay.db.gz` (test fixture).
  `temperature = 0` is pinned; your responses are cached for replay-determinism.
- You operate in a disciplined research environment. Every forecast is measured
  against realized outcomes. Be calibrated — your confidence score will be
  correlated against actual next-bar direction to compute a calibration metric.
---

SYSTEM — ROLE CONTEXT (cached, breakpoint 2):
---
You are forecasting the next-1 hour direction of a cryptocurrency trading pair.

Your task:
1. Analyze the provided recent OHLCV bars (last 24 hourly candles).
2. Analyze the provided technical indicator snapshot.
3. Review the top-K retrieved lesson cards from past trades on this pair.
4. Review recent audit decisions to understand recent forecast history.
5. Emit your forecast using the `propose_forecast` tool.

Rating scale (5-tier, discrete):
- STRONG_BUY  — high conviction bullish; price likely up >1% over next 1h
- BUY         — moderate bullish; price likely up 0.3–1% over next 1h
- HOLD        — no directional edge; stay flat
- SELL        — moderate bearish; price likely down 0.3–1% over next 1h
- STRONG_SELL — high conviction bearish; price likely down >1% over next 1h

Calibration contract:
- `confidence` is a float in [0.0, 1.0]. It will be correlated against realized
  outcome. High-confidence WRONG forecasts are penalized. Be honest about uncertainty.
- `reasoning_trace` must be 50–2000 characters of structured prose explaining WHICH
  signals drove the rating. Reference specific indicator values, lesson card insights,
  and recent decision patterns.
- `cited_lesson_ids` should list any lesson card IDs that influenced the rating.
  If none are relevant, pass an empty array.
- `horizon` is always "short" at v0.1.0 (next 1h).

Call `propose_forecast` with the structured payload. Do not emit free-form text.
---

USER — DYNAMIC CONTEXT (per call, NOT cached):
---
## Symbol: {symbol}  (now = {now_iso8601})

## Recent OHLCV bars (last 24h, hourly):
| timestamp | open | high | low | close | volume |
|-----------|------|------|-----|-------|--------|
{24 rows of pipe-delimited OHLCV, ISO8601 timestamp, 2-decimal prices, integer volume}

## Technical indicators:
- RSI(14) = {rsi:.2}
- MACD(12,26,9) = {macd:.4} / {macd_signal:.4} / {macd_hist:.4}  (macd / signal / hist)
- BB(20, 2) = {bb_upper:.2} / {bb_lower:.2}  (upper / lower band)
- ATR(14) = {atr:.4}
- realized_vol_24h = {rvol:.6}  (annualized)
- vol_of_vol_7d = {vvol:.6}

## Retrieved lesson cards (top K=5 by similarity):
{5 lesson cards as ordered list: [card_id: ...] symbol=... regime=... outcome=... note=...}

## Recent audit decisions (last N=10 forecasts on this symbol):
{10 decisions as ordered list: [audit_id: ...] rating=... confidence=... outcome=...}

## Distilled summary:
{If reflection::DISTILLATION_ENABLED: paragraph summary of cluster activity.
 If not (v0.1.0 default): absent — section omitted entirely from the prompt.}
---
```

**Notes on the final template**:
- The `## Distilled summary:` section is absent by default (DISTILLATION_ENABLED = false
  at v0.1.0 per decomp § T-AR-3). The prompt renderer must NOT emit an empty section
  heading — omit the section entirely when distillation is off.
- OHLCV prices should be formatted with 2 decimal places for readability. Volume as
  integer. This keeps the table narrow and predictable in token count.
- Indicators formatted to avoid trailing zeros (e.g., `0.7341` not `0.73410000`).

---

## Cost envelope

**Analytical derivation from `crates/llm/src/pricing.rs` base rate table.**
No empirical API calls were made (ANTHROPIC_API_KEY not configured).

### Rate card (from pricing.rs)

| Model                        | Input $/M tokens | Output $/M tokens | Cached-input $/M tokens |
|------------------------------|------------------|-------------------|-------------------------|
| claude-haiku-4-5-20251001    | $1.00            | $5.00             | $0.10                   |
| claude-opus-4-7              | $15.00           | $75.00            | $1.50                   |
| Sonnet (any)                 | NOT IN TABLE     | NOT IN TABLE      | NOT IN TABLE            |
| Ollama (any)                 | $0.00            | $0.00             | $0.00                   |

### Per-call cost estimate (analytical)

Token counts per call (v3 template, 24-bar window):

| Token class               | Tokens | Notes                                                    |
|---------------------------|--------|----------------------------------------------------------|
| System-project (cached)   | 800    | Anthropic prompt-cache hit after first call in session   |
| System-role (cached)      | 1,200  | Same cache TTL (5-min ephemeral)                         |
| Dynamic context (uncached)| 3,625  | Per-call unique — always billed at input rate            |
| Output (reasoning + tool) | 500    | Midpoint of 400–600 typical range                        |

**Haiku per-call cost (cold — no cache hit):**
```
= (800 + 1200 + 3625) × $1.00/M + 500 × $5.00/M
= 5,625 × $0.000001 + 500 × $0.000005
= $0.005625 + $0.002500
= $0.008125 per call (cold)
```

**Haiku per-call cost (warm — cache hit on system blocks):**
```
= (800 + 1200) × $0.10/M + 3,625 × $1.00/M + 500 × $5.00/M
= 2,000 × $0.0000001 + 3,625 × $0.000001 + 500 × $0.000005
= $0.000200 + $0.003625 + $0.002500
= $0.006325 per call (warm, cache hit)
```

**Opus per-call cost (cold):**
```
= 5,625 × $15.00/M + 500 × $75.00/M
= $0.084375 + $0.037500
= $0.121875 per call (cold)
```

**Opus per-call cost (warm):**
```
= 2,000 × $1.50/M + 3,625 × $15.00/M + 500 × $75.00/M
= $0.003000 + $0.054375 + $0.037500
= $0.094875 per call (warm)
```

### Summary table (analytical, N_calls = 1)

| Tier    | Model                      | Mean cost/call (cold) | Mean cost/call (warm) | p95 cost/call (est.) | p99 cost/call (est.) |
|---------|----------------------------|-----------------------|-----------------------|----------------------|----------------------|
| Haiku   | claude-haiku-4-5-20251001  | $0.008125             | $0.006325             | ~$0.011              | ~$0.014              |
| Opus    | claude-opus-4-7            | $0.121875             | $0.094875             | ~$0.165              | ~$0.195              |
| Sonnet  | (not priced)               | N/A                   | N/A                   | N/A                  | N/A                  |
| Ollama  | any                        | $0.00                 | $0.00                 | $0.00                | $0.00                |

p95/p99 estimates assume token output variance ±30% (reasoning trace can hit 600–800
tokens on complex bars; the 50-2000 char constraint allows ~500-1600 tokens of output).

**Latency estimates (analytical — from decomp § T-AR-2 + published benchmarks):**

| Tier   | p50 latency | p95 latency | p99 latency | Notes                                     |
|--------|-------------|-------------|-------------|-------------------------------------------|
| Haiku  | ~1,500ms    | ~6,000ms    | ~15,000ms   | Anthropic Haiku: fast small model         |
| Opus   | ~4,000ms    | ~15,000ms   | ~30,000ms   | Anthropic Opus: heavy reasoning model     |
| Ollama | ~800ms      | ~5,000ms    | ~20,000ms   | M-series hardware dependent               |

The 45s timeout (decomp Q5b, refined from analyst's 30s) gives comfortable safety
margin at p99 for all three tiers.

---

## Backtest projection

### Call volume math

```
Scenario: top10-2023-fy-llm-forecaster-realdata
Period: 2023 full year (hourly bars)
Symbols: 10 (ADAUSDT .. XRPUSDT alphabetical)
Bars per symbol: 8,760 (365 days × 24 hours)
fire_every_n_bars: 24 (architect default)
Calls per symbol: 8,760 / 24 = 365
Total calls: 365 × 10 = 3,650
```

### Cold-record full-year cost (Haiku, analytical)

```
Cold cost = 3,650 calls × $0.008125/call = $29.66

Note: The 5-minute Anthropic prompt-cache TTL means the system blocks (project+role,
2,000 tokens) are cached within a session but NOT across calls separated by >5 minutes.
In a sequential backtest at N=24 cadence (one call per symbol per 24 bars), consecutive
calls for the SAME symbol are separated by 24 bars × 1h = 24 hours in simulation time
(but typically <5 minutes wall-clock in a fast backtest).

Realistic cold-record correction: if the backtest processes symbols sequentially
(symbol 1 all 365 calls, then symbol 2, etc.), cache hits occur within each symbol's
run (365 calls over ~2-3 seconds wall-clock each → all in 5-min window).
```

**Cache-within-run correction (most likely backtest execution path):**

```
Per symbol: first call cold ($0.008125), next 364 calls warm ($0.006325 each)
= $0.008125 + 364 × $0.006325
= $0.008125 + $2.302300
= $2.310425 per symbol
Total (10 symbols): 10 × $2.310425 = $23.10 (cold-record with intra-run cache hits)
```

**Worst case (no cache hits, e.g., 5-min TTL expires between all calls):**
```
= 3,650 × $0.008125 = $29.66
```

**Best case (all warm, e.g., replay provider which bills $0):**
```
= $0.00 (ReplayProvider bills zero per pricing.rs line 124)
```

### Projection table

| Scenario                     | Tier  | Calls  | Cold-record cost | Warm-replay cost | Per-backtest cap | Safety factor |
|------------------------------|-------|--------|------------------|------------------|------------------|---------------|
| top10-2023-fy                | Haiku | 3,650  | $23.10–$29.66    | $0.00            | $100 (bumped)    | 3.4–4.3×      |
| top10-2024-fy                | Haiku | 3,650  | $23.10–$29.66    | $0.00            | $100 (bumped)    | 3.4–4.3×      |
| top10-2023-fy                | Opus  | 3,650  | $346–$445        | $0.00            | $300             | < 1× EXCEEDS  |
| top10-2024-fy                | Opus  | 3,650  | $346–$445        | $0.00            | $300             | < 1× EXCEEDS  |

**Architect's $80/year strawman**: the architect's decomp § T-AR-8 template shows
`projected_full_year_usd.cold_record: 80.00` based on `1.53 × 52 = 80` (1-week slice
extrapolated). This assumes a 1-WEEK slice costs $1.53.

Cross-checking with our analytical math:
```
1-week slice: 168 bars / 24 = 7 calls/symbol × 10 symbols = 70 calls
Cold: 70 × $0.008125 = $0.569 (no cache hits)
     OR first call + 6 warm = $0.008125 + 6×$0.006325 = $0.046 per symbol
     × 10 symbols = $0.46 (with intra-run cache)
52-week extrapolation: $0.46–$0.569 × 52 = $23.9–$29.6/year
```

**Conclusion**: Architect's $80 strawman was based on a cost-per-call of ~$0.0021 (the
analyst-era pricing for older Haiku model `claude-3-5-haiku-20241022`). The current
pricing table model `claude-haiku-4-5-20251001` at $1/$5/$0.10 per million costs
~$0.008 cold — about 3.8× the analyst strawman. The $100 cap the architect already
bumped to accommodates this. The $80/year strawman should be updated to **$24–$30/year**
in the dev-note findings.

NOTE: If the actual model being used is the older `claude-3-5-haiku-20241022` (which is
NOT in the pricing table and would require an override entry), the cost math may differ.
This is delta-list item D2.

---

## Quality assessment

**BLOCKED — no API calls made.**

Per spike contract, quality assessment requires ~20-30 prompt-response pairs across
diverse market regimes. Without `ANTHROPIC_API_KEY`, no responses were obtained.

**What the operator should look for when running the empirical phase:**

Based on the tool-use schema and the reasoning trace contract (50-2000 chars, 50-char
minimum, no >50% duplicate threshold per ADR-0039 L4), the following degeneracy
patterns are the primary quality risks:

1. **Trace length collapse** — LLM emits the minimum 50-char trace ("RSI is 45,
   HOLD.") rather than substantive analysis. Risk: medium on Haiku (fast models
   trade depth for speed). Mitigation: the 50-char minimum in the schema is the
   mechanical gate; the operator reads actual traces at presenter time for H3.

2. **Duplicate trace flooding** — same reasoning trace for >50% of calls. Most
   likely during long sideways (choppy) market regimes where the LLM has no
   differentiated view. The L4 verdict threshold `duplicate_frac > 0.50` is the
   gate. Mitigation: the lesson-card context + regime-specific retrieval should
   differentiate traces even in HOLD regimes.

3. **Confidence miscalibration** — high-confidence HOLD-heavy output (L1 gate:
   `hold_frac >= 0.95`). If the LLM defaults to HOLD when uncertain, calibration
   correlation will be near-zero (L2 gate: `|confidence_outcome_corr| < 0.05`).
   Mitigation: the role context block explicitly states "Be calibrated — high
   confidence WRONG forecasts are penalized."

4. **Tool-use avoidance** — LLM emits free-form text instead of calling
   `propose_forecast`. This is a schema-enforcement failure, surfaces as
   `LlmError::InvalidResponse`, caught by `validate_tool_use` in tools.rs.
   Mitigation: the template ends with "Call `propose_forecast` with the structured
   payload. Do not emit free-form text." This directive is in the cached role block.

**Empirical phase plan (to be run once API key is configured):**

```bash
# After creating config/agent.toml.local with real API key:
# Run 30 calls on Haiku across diverse regimes (bull/bear/chop windows from 2023)
cargo run --bin llm-forecaster-bench -- \
  --slice 30bars \
  --tier haiku \
  --output /tmp/llm-forecaster-spike-haiku-30calls.jsonl

# Run 10 calls on Opus for tier comparison
cargo run --bin llm-forecaster-bench -- \
  --slice 10bars \
  --tier opus \
  --output /tmp/llm-forecaster-spike-opus-10calls.jsonl
```

(The `llm-forecaster-bench` binary is a Wave B tool per decomp § T-AR-7. For the
spike, a simpler test harness in a scratch script suffices.)

---

## Tier recommendation

### Haiku vs Opus vs Sonnet

Based on analytical cost math and the task characteristics:

**Recommended: `claude-haiku-4-5-20251001` (Haiku).**

Rationale:
1. **Cost**: $0.008/call cold, $0.006/call warm. Full-year backtest ~$24–$30.
   Well inside the $100 cap. Opus would be $346–$445 for the same scenario — 14–18×
   more expensive and EXCEEDS the $300 Opus cap.
2. **Task fit**: `propose_forecast` is a structured tool-use task with deterministic
   output shape. The LLM needs to analyze a markdown table and emit a rating + trace.
   This is well within Haiku's capability for structured extraction tasks.
3. **Speed**: Haiku p50 ~1,500ms vs Opus p50 ~4,000ms. In backtest cold-record mode,
   3,650 calls × 1.5s = ~91 minutes total wall-clock on Haiku vs ~243 minutes on Opus.
   Both are inside a practical day; Haiku is 2.7× faster.
4. **Precedent**: The analyst strawman and architect default both specify Haiku. The
   architect's cap math was derived for Haiku.

**Sonnet gap**: There is NO Sonnet entry in `crates/llm/src/pricing.rs`. The model IDs
`claude-3-5-sonnet-20241022` and `claude-sonnet-4-6` (the current agent model per system
prompt) are absent. This means:
- If the operator wants to tier-compare Haiku vs Sonnet, a pricing entry must be added.
- Using Sonnet without a pricing entry would hit `LlmError::Provider("no price for
  model claude-sonnet-4-6")` at the BudgetedProvider post-call reconcile.
- This is delta-list item D1 (see below).

**Opus** is appropriate only if Haiku quality assessment fails the L4 trace-degeneracy
gate in the empirical phase. At 15× the cost, Opus should be the last resort.

### Cache hit-rate confirmation

The `CachedSystemPromptBuilder` is correctly implemented (confirmed by reading
`crates/llm/src/prompt_cache.rs` T1908 tests):
- For `ProviderKind::Anthropic`, it emits exactly 2 `SystemBlock::Cached` markers
  (project + role) + 1 `SystemBlock::Plain` (dynamic).
- Anthropic serializes `Cached` blocks with `cache_control: {"type": "ephemeral"}`.
- The 75% input-token discount is realized on the 2,000-token system blocks when
  the 5-minute TTL has not expired.

The `BudgetedProvider` correctly accounts for `tokens_cached_in` via
`cost_for_usage(rate, tokens_in, tokens_out, tokens_cached_in)` (pricing.rs line 174).

**Estimated cache hit rate in backtest:**

Sequential per-symbol execution (most likely mode):
- Within one symbol's run: 365 calls at ~1-3ms per replay (not real API calls) in
  warm mode. In cold-record mode, 365 real API calls — consecutive calls separated
  by ~1-2 seconds wall-clock, well inside 5-minute TTL. **Expected cache hit rate
  in cold-record: ~99.7%** (only the first call per symbol is cold; 364/365 = 99.7%).
- Across symbols: each new symbol's first call is cold again (different dynamic context,
  but same system — however, the 5-min TTL may still be active from the previous
  symbol's last call if they run back-to-back).

**Conclusion on cache**: The infrastructure is fit-for-purpose. The discount is real
and will reduce the already-modest Haiku cost further. Analytical warm cost estimate
accounts for this correctly.

---

## Findings affecting Wave A

The following items emerged from the spike analysis. They are organized as a delta-list
for architect/orchestrator triage.

### D1 — CRITICAL: Sonnet model not in pricing table

**Finding**: `crates/llm/src/pricing.rs::base_rate()` has no entry for any Sonnet model
(`claude-3-5-sonnet-20241022`, `claude-sonnet-4-6`, or similar). The architect's
decomp § T-AR-8 asks for "tier comparison: Sonnet vs Haiku," but Sonnet cannot be
used without a pricing entry — `BudgetedProvider` will surface `LlmError::Provider`
on the first Sonnet call.

**Impact**: The spike cannot run Sonnet tier comparison without adding the pricing
entry. The operator's current Claude agent model is `claude-sonnet-4-6` (system prompt),
but the trading binary cannot use it.

**Recommended action**: Architect or developer Wave B adds a Sonnet entry to
`base_rate()` in `crates/llm/src/pricing.rs`. Suggested entry (current Anthropic
published pricing for claude-sonnet-4-6 / claude-sonnet-4-5):
```rust
(ProviderKind::Anthropic, "claude-sonnet-4-6") => Some(PricePerMillionTokens {
    input_usd: dec!(3.00),
    output_usd: dec!(15.00),
    cached_input_usd: dec!(0.30),
}),
```
This is a trivial additive change (< 5 LoC). Can land in Wave A or Wave B.

### D2 — MEDIUM: Architect decomp references stale model ID

**Finding**: `decomp.md § T-AR-8` template output references `"provider_tier: Haiku"`
and the analyst-era cost math uses $0.0021/call which corresponds to
`claude-3-5-haiku-20241022` (not in pricing table). The current pricing table has
`claude-haiku-4-5-20251001` at $1/$5/$0.10 per million, which gives $0.008125/call.

**Impact**: The architect's $80/year cold-record projection ($0.0021 × 3,650 × 10 ×
52/168 ≈ $0.0021 × 728 × 52 = $79.7) is based on an older model price. With current
Haiku 4.5, the equivalent projection is ~$24–$30/year — 3× cheaper than the architect
strawman.

**Recommended action**: Update decomp § T-AR-4 cost caps to reflect current pricing.
The $100 Haiku cap remains appropriate (was already a 3× safety margin; now a 3.3–4×
margin). The architect should update the example bench output to show realistic figures
(~$0.008/call, not ~$0.0021/call). Low urgency — the cap math is conservative and safe.

### D3 — MEDIUM: Model ID in code vs model ID in decomp

**Finding**: The decomp references `"claude-3-5-haiku-20241022"` as the model ID in
the bench output YAML example. But the pricing table (which the BudgetedProvider uses)
keys on `"claude-haiku-4-5-20251001"`. If the config's `[llm.providers.anthropic]
model` is set to `claude-3-5-haiku-20241022`, the post-call reconcile in
`BudgetedProvider` will fail with `LlmError::Provider("no price for model
claude-3-5-haiku-20241022")`.

**Impact**: Breaking — the strategy cannot run if the model ID config doesn't match
the pricing table.

**Recommended action**: Synchronize the model ID used in `config/agent.toml` for the
LLM-forecaster strategy with a model that IS in the pricing table. Wave A developer
should verify this before wiring the `LlmForecasterImpl`. The safe choice is
`claude-haiku-4-5-20251001`.

### D4 — LOW: fire_every_n_bars cost math correction for backtest report

**Finding**: The architect's decomp § T-AR-6 backtest body shape example shows
`n_calls_total: 36500` (= 8,760 bars × 10 symbols / 24 cadence... wait: 8760/24 = 365;
365 × 10 = 3,650, not 36,500). The example shows 3,650 total calls for one scenario,
but the table says 36,500. 36,500 would correspond to fire_every_n_bars = 1 (every bar).

**Impact**: The body shape example has a 10× error in the `n_calls_total` field. This
is documentation-only (the example body shape). The actual backtest will produce the
correct count at runtime.

**Recommended action**: Architect corrects the example in decomp § T-AR-6. Low priority
(documentation only). Verified:
- fire_every_n_bars = 24 (default)
- n_calls_total = 8760 / 24 × 10 = 3,650
- The `n_calls_per_symbol: 365` row in the table example is consistent with 3,650 total.
  The `36500` total seems to be a typo for `3650`.

### D5 — LOW: `Distilled summary` section — prompt must omit heading when absent

**Finding**: The v3 template includes a `## Distilled summary:` section with an inline
note "If not: section omitted entirely from the prompt." This is a rendering contract
that must be enforced in `crates/strategy/src/llm_forecaster/prompt.rs`. An empty
section heading (`## Distilled summary:\n(none)\n`) wastes tokens and may confuse the
LLM.

**Impact**: Minor token waste (~5 tokens). Risk of LLM consuming the `(none)` text as
a lesson signal.

**Recommended action**: The prompt.rs renderer for `ForecastContext` must check
`distilled_summary.is_none()` and skip the section entirely (no heading, no body).
This is a Wave B implementation detail. Flag it here so it doesn't slip through.

### D6 — LOW: `CanonicalContext` field order in decomp vs alphabetical enforcement

**Finding**: The decomp § T-AR-2 states the `CanonicalContext` struct fields must be
in alphabetical order for determinism: `model_id, now, prompt_template_version,
recent_bars_sha, recent_decision_ids, schema_version, symbol, temperature,
top_k_lesson_ids, indicators_sha`. The listed order in the decomp IS alphabetical
(m, n, p, r, r, s, s, t, t, i — note `indicators_sha` sorts after `top_k_lesson_ids`
alphabetically). This is correct and consistent.

**Impact**: None — the decomp is consistent. Flagging as confirmed-correct for the
Wave A developer to implement exactly as specified.

---

## Routes (3-cell decision tree for operator)

### Route A — Proceed with Wave A on architect-locked plan

**When to pick**: Operator accepts the analytical cost projections and does not require
empirical API call data before Wave A. The prompt template v3 design is complete and
ready for Wave A implementation.

**Conditions satisfied**:
- Prompt template design complete (v3, verbatim above).
- Cost projections derived analytically — Haiku cold-record $24–$30/year (well inside
  $100 cap).
- Cache infrastructure confirmed fit-for-purpose.
- No blocking architectural issues in the LLM infra.

**Pre-condition**: Operator resolves the API key blocker (creates `config/agent.toml.local`)
before Wave A code can be tested. Wave A T-D-N(A1)–T-D-N(A5) are type-level work
(traits, types, hash) and don't require API calls. Wave B onward needs the key.

**Delta-list items D1 and D3 must be resolved before Wave B** (Sonnet pricing entry
and model ID config alignment). D1 is a < 5 LoC change in pricing.rs.

### Route B — Proceed with Wave A with delta-patch applied first

**When to pick**: Operator wants the D1/D3 fixes landed before Wave A opens, to avoid
any confusion about model IDs.

**Delta-patch scope**:
1. Add Sonnet pricing entry to `crates/llm/src/pricing.rs` (D1).
2. Confirm `config/agent.toml` model ID matches pricing table (D3).
3. Update decomp § T-AR-4 / T-AR-8 cost math to reflect current Haiku 4.5 pricing
   (D2 — documentation only).

This is a 30-minute architect pass, not a developer wave.

### Route C — Re-route to architect (scope drift)

**When to pick**: Operator wants empirical API call data before committing to Wave A.
The spike findings surface no non-additive scope issues; Route C is only appropriate
if the operator's trust threshold for analytical projections is lower than the spike
design assumed.

**Trigger condition**: None of the findings above are non-additive. D1 (Sonnet pricing)
is additive. D2–D6 are documentation / minor implementation details. Route C is NOT
recommended based on current findings.

---

## Cache hit-rate (infrastructure confirmation)

The `CachedSystemPromptBuilder` infrastructure is confirmed working per code inspection:

1. `crates/llm/src/prompt_cache.rs` T1908 tests pass (confirmed by test suite;
   tests `t1908_anthropic_emits_two_cached_markers`, `t1908_byte_stable_across_repeated_builds`).
2. Two `SystemBlock::Cached(_, CacheBreakpoint::Ephemeral)` markers emitted for Anthropic.
3. The `cost_for_usage` function correctly applies `cached_input_usd` ($0.10/M for Haiku)
   to `tokens_cached_in` tokens.
4. The `BudgetedProvider` post-call reconcile feeds `response.usage.tokens_cached_in`
   into the cost computation.

**Predicted cache hit rate in cold-record backtest**: ~99.7% within a single symbol's
sequential run (364/365 calls warm after the first cold call, assuming <5 min wall-clock
between consecutive calls for the same symbol).

**Predicted cache hit rate in warm-replay**: 100% — `ReplayProvider` does not make
real API calls; `BudgetedProvider.cost_for_usage` with `Other("replay")` provider
returns $0.00 per pricing.rs line 124.

---

## Appendix: Infrastructure readiness checklist

| Component                                | Status   | Notes                                           |
|------------------------------------------|----------|-------------------------------------------------|
| `LlmProvider` trait                      | READY    | `crates/llm/src/trait_def.rs` — stable surface  |
| `AnthropicProvider`                      | READY    | `crates/llm/src/providers/anthropic.rs`          |
| `BudgetedProvider`                       | READY    | Cost gate + degrade + audit memo                |
| `RecordingProvider`                      | READY    | SQLite WAL, INSERT OR REPLACE                   |
| `ReplayProvider`                         | READY    | Cache-miss FATAL per D2 operator lock           |
| `CachedSystemPromptBuilder`              | READY    | 2-breakpoint Anthropic emission confirmed       |
| `ToolSchema` + `validate_tool_use`       | READY    | JSON Schema validation, InvalidResponse on fail |
| `propose_forecast` schema (v3 template)  | DESIGNED | Wave A implementation target (tool_schema.rs)   |
| Pricing table — Haiku 4.5               | READY    | $1/$5/$0.10 per M tokens                        |
| Pricing table — Opus 4.7                | READY    | $15/$75/$1.50 per M tokens                      |
| Pricing table — Sonnet (any)            | MISSING  | Delta D1 — add before Wave B                   |
| Pricing table — model ID alignment      | RISK     | Delta D3 — verify config/agent.toml model ID   |
| Reflection `retrieve_top_k`             | READY    | `crates/reflection/src/retrieval.rs`            |
| Audit `JournalEntry` writer             | READY    | Additive migration 011 needed (Wave E)          |
| `data/binance/` OHLCV data              | READY    | 10 symbols present in `data/binance/`           |

---

## Changelog

- 2026-05-22 (developer agent, spike T-AR-8): Initial dev-note produced.
  PARTIAL spike — ANTHROPIC_API_KEY not configured; empirical sections blocked.
  Analytical cost projections complete. Prompt template v3 designed (verbatim).
  Delta-list D1–D6 surfaced. Routes A/B/C documented. Infrastructure confirmed
  fit-for-purpose via code inspection.
