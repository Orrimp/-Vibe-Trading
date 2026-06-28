# LLM Cost Runbook

**Version:** v2.0.0
**Owner:** operator / on-call
**Related code:** `crates/llm/src/budgeted.rs`, `crates/llm/src/pricing.rs`, `crates/reports/src/render/system_health.rs`

---

## Overview

The v2 LLM stack ships with a per-process budget gate (`BudgetedProvider`)
that tracks monthly USD spend across every provider call. The operator sees
spend in three places:

1. **Cockpit "LLM budget" right-rail tile** (T1938) — live, polls
   `audit::query::llm_spend_this_month(ledger)`.
2. **Operator success report → System health row** — `LLM spend | $X / $200`
   (Q11 denominator). The denominator is the monthly ceiling
   `cfg.llm.budget_usd_month`.
3. **Audit ledger memos** — `BudgetEventKind::DegradeToQuickThink` and
   `BudgetEventKind::Block` rows in the `journal` table when the gate
   tripped (debounced to once / 60s per BudgetedProvider).

The gate's behaviour at the budget boundary is:

| Spent vs. ceiling | Action |
|-------------------|--------|
| `spent < 0.8 × ceiling` | passthrough (no warn) |
| `0.8 × ceiling ≤ spent < ceiling` | passthrough + cockpit tile colour flips to `Theme::Warn` |
| `spent ≥ ceiling` | `BudgetedProvider` returns `LlmError::BudgetExceeded`; no outbound HTTP; cockpit tile colour flips to `Theme::Halt`; audit memo posts |
| `spent < ceiling` AND request is `DeepThink` AND `mode_override == Some(QuickThink)` | request rewritten to QuickThink model; `tracing::warn!(target: "llm.budget", "degrade_to_quick_think", ...)`; audit memo posts |

---

## What the System Health "LLM spend" line means

In every operator success report, the `| LLM spend | $X / $Y |` row reads:

- **`$X`** — sum of `expense:llm:*` ledger rows posted during the report
  period.
- **`$Y`** — the configured `cfg.llm.budget_usd_month`. v2.0.0 defaults
  this to **$200** (was $135 in v1.5a; the denominator update is part of
  Q11 — see [feature.md § Q11](../v1/v2-llm-strategy/feature.md#q11---operator-success-report-llm-spend-denominator-update-architect-decide)).

The body row is byte-stable across runs (R10.3) — re-anchored in
`spec/anchors.toml` at `T_FINAL_V2_LLM_STRATEGY`.

A new sibling row **`| Cache hit ratio | X.X% |`** lands between
`LLM spend` and `Funding poll success` (Q5d). Source:
`audit::query::cache_hit_ratio_since(ledger, period_start)`.

---

## What the operator does on a degrade event

A `BudgetEventKind::DegradeToQuickThink` memo means the budget gate
re-wrote a `DeepThink` request to use the configured QuickThink model
(default Haiku 4.5). The forensic record carries:

- the original tier / model
- the rewritten tier / model
- the current `spent_usd / ceiling_usd`
- the correlation id

**Operator actions:**

1. Open the cockpit and confirm the LLM budget tile colour
   (`Theme::Warn` is expected).
2. Read the most recent `journal` rows tagged `llm_budget_event` for
   the past hour: `audit::query::recent_llm_budget_events(ledger, 1h)`.
3. Decide whether to raise the ceiling (edit `config/agent.toml`,
   restart the agent) or accept the degrade.
4. If raising: bump `[llm] budget_usd_month` and restart the agent;
   the new ceiling takes effect at the next process boot (no hot-reload
   in v2.0.0).

A `BudgetEventKind::Block` memo means the gate refused the call
entirely (no outbound HTTP). The caller saw `LlmError::BudgetExceeded`.
The error already propagates to the consumer-side error router; the
operator's job is to confirm the cockpit alert and either raise the
ceiling or wait for the next billing month.

---

## How to update cost-rate entries

Two paths:

1. **Hard-coded base table** — `crates/llm/src/pricing.rs:94-130` carries
   the v2 rate card. To add a new model or rev a price:
   - edit the `base_rate()` match;
   - run `cargo test -p llm pricing` to confirm the unit test asserts
     the new entry;
   - rebuild + restart the agent.
2. **TOML override** — `[llm.pricing.<provider>.<model>]` in
   `config/agent.toml`. Override syntax:
   ```toml
   [llm.pricing.anthropic."claude-opus-4-7"]
   input_usd        = "15.0"
   output_usd       = "75.0"
   cached_input_usd = "1.5"
   ```
   Restart the agent to pick up the change. The override map is loaded
   at boot only; the budget gate consults it on every post-call reconcile.

The base table catches typos at compile time (the match is exhaustive
over the supported model set); a typo'd override silently falls back to
the base.

---

## How to swap providers

1. Edit `config/agent.toml`:
   ```toml
   [llm]
   default_provider = "openai"  # was "anthropic"

   [llm.deep_think]
   provider = "openai"
   model    = "gpt-5"

   [llm.quick_think]
   provider = "openai"
   model    = "gpt-5-mini"
   ```
2. Ensure `config/agent.toml.local` carries the OpenAI key:
   ```toml
   [llm.providers.openai]
   api_key = "sk-..."
   ```
3. Restart the agent. The factory rebuilds the provider stack at boot.

---

## Real-API smoke procedure (operator-only, requires real keys)

This procedure exercises live provider endpoints. It is **not** run in
CI (V3 / V4 forbid outbound HTTPS during `cargo test`).

```bash
# 1. Copy the example overlay and edit in real keys.
cp config/agent.toml.local.example config/agent.toml.local
$EDITOR config/agent.toml.local

# 2. Smoke-test paper mode (records every successful complete() into
#    data/llm-replay.db).
cargo run --bin llm-smoke -- --mode paper

# 3. Verify the byte-stable result table:
#    provider | model | tokens_in | tokens_out | usd | latency_ms | result
#    anthropic | claude-opus-4-7 | ... | ... | ... | ... | OK
#    openai    | gpt-5           | ... | ... | ... | ... | OK
#    ollama    | <local model>   | ... | ... | ... | ... | OK

# 4. Confirm the replay cache has 9 rows (3 providers × 3 roles).
sqlite3 data/llm-replay.db 'SELECT COUNT(*) FROM replay_entries;'
# Expected: 9

# 5. Re-run in research mode against the freshly captured cache.
cargo run --bin llm-smoke -- --mode research
# Expected: same green table, no outbound HTTP.
```

The smoke binary exits `0` on a green table; non-zero on any provider
returning non-`OK` (or a research-mode cache miss).

---

## Related runbooks

- [llm-replay.md](llm-replay.md) — record/replay playbook.
- [kill-switch.md](kill-switch.md) — agent hard-stop procedures.
