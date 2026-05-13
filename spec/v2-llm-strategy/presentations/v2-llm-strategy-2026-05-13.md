---
slug: v2-llm-strategy
mode: release
status: shipped
audience: human-operator
updated: 2026-05-13
generated: 2026-05-13T05:04:28Z
---

# v2 LLM strategy (v2.0.0) — release

## TL;DR

v2.0.0 ships the **LLM substrate made callable**: a real
`LlmProvider` trait, three provider impls (Anthropic / OpenAI-
compatible / Ollama), an Anthropic prompt-cache builder, a
`BudgetedProvider` decorator enforcing the **$200/mo deep-think
ceiling with auto-degrade at 80% and block at 100%**, a
strict-replay SQLite cache for deterministic research-mode runs,
a 3-provider × 3-role `llm-smoke` harness, and two operator
runbooks. Per operator decision Q1=A this is **foundation-only**
— no strategy code wires to LLM in v2.0.0; the first consumer
brief is queued post-ship
([reflection-memory-llm-enrichment](../reflection-memory/feature.md#q1--llm-driven-post_mortem_analyst-vs-deterministic-v1)
+ reflection-memory-trader-wiring). Tester `VERDICT → PASS` on
commit `8a41b47` with **1203 / 0 tests across 158 binaries**
and **ANCHORS PASS (11 / 11)** including the 2 re-locked
`report-sample-*` anchors. Deferred to v2.1: T1938 cockpit "LLM
budget" tile (blocked on the unshipped `audit::query::llm_spend_this_month`)
and the T1915 tracing-Layer half (pure-fn `redact()` landed; the
field-visitor side needs a new dep).

## What changed

Foundation only — `Strategy` trait, audit chart of accounts, and
the 9 strategy backtest anchors are unchanged.

- **`crates/llm/` v0 stub → real trait surface** (T1901, pass 1
  `d0bcad2`). Async non-streaming `LlmProvider::complete`,
  `ChatRequest` / `ChatResponse` / `ContentBlock` / `ToolSchema`
  types. 8-variant `LlmError` ([`crates/llm/src/error.rs`](../../../crates/llm/src/error.rs)).
- **`cost::LlmProvider` enum → `cost::ProviderKind`** (D1 honored).
  Lands at [`crates/cost/src/event.rs:15`](../../../crates/cost/src/event.rs#L15)
  — `pub enum ProviderKind {`. Frees the `LlmProvider` name for
  the trait that's the heart of the feature.
- **Three provider impls** (T1902–T1906, pass 2 `c61afa5`):
  - Anthropic — [`crates/llm/src/providers/anthropic.rs`](../../../crates/llm/src/providers/anthropic.rs).
    `anthropic-version: 2023-06-01` header, `cache_control` markers,
    `cache_read_input_tokens` reporting.
  - OpenAI-compatible — [`crates/llm/src/providers/openai.rs`](../../../crates/llm/src/providers/openai.rs).
    Covers OpenAI, OpenRouter, DeepSeek, LM Studio via configurable
    base URL.
  - Ollama — [`crates/llm/src/providers/ollama.rs`](../../../crates/llm/src/providers/ollama.rs).
    Local-only, `$0.00` cost events.
  - Retry helper — [`crates/llm/src/retry.rs`](../../../crates/llm/src/retry.rs).
    Exponential backoff + full jitter, 3 retries, `Retry-After`
    honored (Q9).
- **Prompt-cache builder + observability** (T1907–T1910, pass 3
  `441c136`). [`crates/llm/src/prompt_cache.rs`](../../../crates/llm/src/prompt_cache.rs)
  emits 2 cache breakpoints (project + role contexts) for
  Anthropic; silently drops markers for OpenAI; no-op for Ollama.
  New `audit::query::cache_hit_ratio_since` at
  [`crates/audit/src/query.rs:164`](../../../crates/audit/src/query.rs#L164)
  powers the System Health row.
- **`BudgetedProvider<Inner>` decorator** (T1911–T1918, pass 3
  `441c136` + pass 4 `f1dbe05`). [`crates/llm/src/budgeted.rs`](../../../crates/llm/src/budgeted.rs).
  `AtomicU64` cents counter, `try_reserve(estimate_usd)` pre-call
  gate. Auto-degrade `deep_think → quick_think` at 80%; block with
  `LlmError::BudgetExceeded` at 100%. T1912 budget-audit-memo
  flipped from `[~]` to `[x]` after journal helpers landed
  (pass 4).
- **Record / replay** (T1919–T1927, pass 5 `f1128e9`). SQLite WAL
  cache at `data/llm-replay.db`; canonical-JSON SHA-256
  `request_hash` ([`crates/llm/src/recording.rs`](../../../crates/llm/src/recording.rs),
  [`crates/llm/src/replay.rs`](../../../crates/llm/src/replay.rs)).
  D2 honored — strict miss → `LlmError::ReplayMiss { hash,
  provider, model }` at
  [`crates/llm/src/replay.rs:299`](../../../crates/llm/src/replay.rs#L299)
  (no fallthrough). 9-row fixture cache (3 providers × 3 roles)
  at [`crates/llm/fixtures/replay-v1.db`](../../../crates/llm/fixtures/replay-v1.db).
- **`llm-smoke` binary + wiremock harness** (T1923–T1924). Lives at
  [`crates/llm/src/bin/llm-smoke.rs`](../../../crates/llm/src/bin/llm-smoke.rs).
  `t1924_smoke_harness_three_providers_three_roles` round-trips
  3 providers × 3 roles in 0.39s.
- **`audit::query::cache_hit_ratio_since` + System Health row +
  $200 denominator** (T1935, pass 6 `faaaec1`). Q11 bundled in v2
  (D3 honored). The two `success-fixed-report-sample-*.md` bodies
  now show `| LLM spend | $0.00 / $200 |` and `| Cache hit ratio
  | 0.0% |` ([sample-7d:66-67](../../operator-success-reports/reports/success-fixed-report-sample-7d.md),
  [sample-90d:68-69](../../operator-success-reports/reports/success-fixed-report-sample-90d.md)).
- **Agent config + wire-up + runbooks** (T1928–T1934, pass 6
  `faaaec1`). New `[llm]` block in `config/agent.toml`;
  `config/agent.toml.local.example` template at
  [`config/agent.toml.local.example`](../../../config/agent.toml.local.example);
  `LlmProviderFactory::build` at
  [`crates/llm/src/factory.rs`](../../../crates/llm/src/factory.rs);
  TOML-local key reader at [`crates/llm/src/auth.rs`](../../../crates/llm/src/auth.rs);
  runbooks at [`spec/runbooks/llm-cost.md`](../../runbooks/llm-cost.md)
  + [`spec/runbooks/llm-replay.md`](../../runbooks/llm-replay.md).
- **No new crates.** `crates/llm/` swapped from v0 stub to v2
  surface. New deps: `anthropic-version: 2023-06-01` (HTTP header
  only), `jsonschema` 0.30 (tool-use schema validation),
  `image-compare` (test-only), `wiremock` (test-only), `sqlx`
  (replay store), `serde_json`, `rust_decimal`, `sha2`.

## What changed in process

This feature **predates the AGENT.md `## Capability boundaries`
amendment** (adopted 2026-05-12 after the chart-canvas-overhaul
retrospective; see
[AGENT.md ## Capability boundaries](../../../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)).
The structural artifacts in this brief — V1–V12 matrix, R-items
mapped 1:1 to T1901–T1945, 9 hard constraints, T1937 negative-
invariant gate — were already in place when the new rules
landed, so per the orchestrator-scope-check pause-time changelog
([orchestrator-scope-check-2026-05-10.md:204-207](../orchestrator-scope-check-2026-05-10.md))
**no retrofit was applied**; v2 ships as-is and the new
patterns (`## Hypothesis register`, test-runner/evaluator split,
default-FAIL PreToolUse hooks) apply to the next feature.

The cycle ran under the **single-tester model** (not the new
test-runner / evaluator split), across **six developer passes
over two days**, with orchestrator-managed context-budget resumes
at clean tick boundaries:

| Pass | Commit | Scope | Anchor of work |
|---|---|---|---|
| 1 | `d0bcad2` | T1901 — llm crate rewrite + `ProviderKind` rename | trait surface lands |
| 2 | `c61afa5` | T1902–T1906 — M2 three providers + retry helper | provider impls land |
| 3 | `441c136` | T1907–T1915 — M3 prompt cache + M4 budget gate | cache + budget land |
| 4 | `f1dbe05` | T1916–T1918 — M5 telemetry + T1912 audit-memo flip `[~]→[x]` | budget audit-memo wired |
| 5 | `f1128e9` | T1919–T1927 — M6 record/replay + T1913 factory Research/Recording arms flip `[~]→[x]` | replay path lands |
| 6 | `faaaec1` | T1928–T1945 — M7 config + agent wire-up + runbooks (T1938 deferred) | wire-up + report rows |
| T_FINAL | `8a41b47` | tester re-locked 2 `report-sample-*` anchors → ANCHORS PASS (11/11) | ship gate |

**Honest [~] discipline** was held across two partial ticks that
flipped to `[x]` in later passes once their dependencies landed:
T1912 (audit-memo couldn't land until pass 4's journal helpers
were in) and T1913 (factory's `Research` + `Recording` arms
couldn't wire until pass 5's `ReplayProvider` + `RecordingProvider`
landed). Each [~] cited the missing dependency at the time and
flipped cleanly when the dep was met — see tasks.md task bodies
at lines 887 and 1034.

## Why

v2 is the project's **first LLM integration** and the largest-
scope feature shipped to date. Two crates have been pre-positioned
for this moment, both carrying zero callers across two minor
versions:

1. [`crates/llm/`](../../../crates/llm/) — the v0 23-line stub.
2. [`crates/cost/`](../../../crates/cost/) — the fully-wired
   `CostEvent::Llm`, `LedgerCostSink`, `CostBudget` with auto-
   degrade at 80% / block at 100% already implemented in v0 at
   [`crates/cost/src/budget.rs:40-53`](../../../crates/cost/src/budget.rs#L40-L53).

The contract is set by four
[`spec/product.md`](../../product.md) sections — LLM strategy
(dual-tier + provider abstraction + cost controls), cost
economics (the **$135 → $200 monthly ceiling** ladder confirmed
2026-04-17), the trading-time agent roster (ten LLM-driven roles
queued as consumers), and the strategy-library roadmap (the v2
"LLM-augmented news/sentiment overlay" row that Q1 explicitly
descopes from v2.0.0). Foundation-only means each queued LLM
consumer drops in as an R-level addition on a stable trait
surface, instead of re-litigating the trait shape on every
follow-up brief.

## What you can do now

| Action | Command |
|--------|---------|
| Run the smoke binary in research mode against the shipped fixture | `cargo run --bin llm-smoke -- --mode research --replay-path crates/llm/fixtures/replay-v1.db` |
| Verify the 11 anchors are still green | `bash scripts/verify_anchors.sh` |
| Verify no API-key substrings leaked into artifacts | `bash scripts/check_no_secrets_in_llm_artifacts.sh` |
| Re-run the full V-matrix test suite | `cargo test --workspace --all-targets` |
| Read the LLM cost runbook | [`spec/runbooks/llm-cost.md`](../../runbooks/llm-cost.md) |
| Read the LLM replay runbook | [`spec/runbooks/llm-replay.md`](../../runbooks/llm-replay.md) |
| Provision a local TOML override for keys | copy [`config/agent.toml.local.example`](../../../config/agent.toml.local.example) to `config/agent.toml.local` and fill in keys |

## Live demo

Per the AGENT.md `## Capability boundaries` rule the presenter
sub-agent does not run the cockpit or capture screenshots; v2 is
headless / CLI anyway. The two presenter-callable demos that
prove ship-readiness are the **anchor gate** and the **workspace
test summary**, both verbatim from the tester report
([test-2026-05-12-2219-v2-llm-strategy-final.md](../reports/test-2026-05-12-2219-v2-llm-strategy-final.md)):

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
---
ANCHORS PASS  (11 / 11)
```

```
$ cargo test --workspace --all-targets 2>&1 | awk '/test result:/ {p+=$4; f+=$6; i+=$8} END {printf "TOTAL: %d passed; %d failed; %d ignored (%d binaries)\n", p, f, i, NR}'
TOTAL: 1203 passed; 0 failed; 3 ignored (158 binaries)
```

Notice the **9 strategy backtest anchors** at the top of the
list have **byte-identical SHAs** to v1.5a — confirming the
foundation-only Q1 invariant: zero strategy code wired to LLM in
v2.0.0. The two `report-sample-*` SHAs are the **only changing
anchors**, re-locked once at T_FINAL alongside the bundled
Q5d cache-hit-ratio row + Q11 `$135 → $200` denominator hot-fix
(see [`spec/anchors.toml:67-83`](../../anchors.toml)).

## Verification matrix

Imported verbatim from the tester report's V1–V12 matrix (see
[test-2026-05-12-2219-v2-llm-strategy-final.md § 4](../reports/test-2026-05-12-2219-v2-llm-strategy-final.md)).
V1 is `PARTIAL — non-blocking`; every other row is `PASS`.

| #   | Definition                                                                                  | Cite / Command                                                                                                                                                                                                                                                                       | Result |
|-----|---------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------|
| V1  | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo audit`, `cargo deny`.               | `cargo fmt --all --check` → PASS. `cargo clippy --workspace --all-targets -- -D warnings` → 2 NEW pedantic warnings on touched v2 code at [`crates/audit/src/query.rs:219, 221`](../../../crates/audit/src/query.rs#L219) (T1910 `cast_possible_truncation`); rest pre-existing chart-buy-sell-emphasis + chart-canvas-overhaul + polars-license out-of-v2-scope. | PARTIAL — non-blocking (v2.1 cleanup queued) |
| V2  | `cargo test --workspace` zero failures, zero unexplained `#[ignore]`.                       | `cargo test --workspace --all-targets` → 158 binaries, **1203 passed, 0 failed, 3 ignored** (pre-existing).                                                                                                                                                                          | PASS |
| V3  | `llm-smoke` round-trips 3 providers via wiremock.                                           | [`crates/llm/tests/smoke_harness.rs::t1924_smoke_harness_three_providers_three_roles`](../../../crates/llm/tests/smoke_harness.rs) → ok in 0.39s.                                                                                                                                    | PASS |
| V4  | Zero outbound HTTPS to real LLM hosts during `cargo test --workspace`.                      | [`crates/llm/tests/no_real_api_test.rs::t1940_no_real_api_calls_in_tests`](../../../crates/llm/tests/no_real_api_test.rs) → ok in 0.01s.                                                                                                                                             | PASS |
| V5  | Balanced expense ↔ liability journal pair; `\|dr − cr\| ≤ 1e-8`.                            | [`crates/llm/tests/budget_audit_memo_test.rs`](../../../crates/llm/tests/budget_audit_memo_test.rs) → 3 / 3 ok (incl. `audit_memo_degrade_lands_with_ledger`, `audit_memo_block_lands_with_ledger`).                                                                                  | PASS |
| V6  | Two runs of `llm-budget-degrade` produce byte-identical degrade events (corr id excluded).  | [`crates/llm/tests/budget_gate_test.rs`](../../../crates/llm/tests/budget_gate_test.rs) → 3 / 3 ok. Determinism via `ChaCha20Rng::from_seed`.                                                                                                                                        | PASS |
| V7  | Two runs of `ReplayProvider` against same hash → byte-identical.                            | [`crates/llm/tests/replay_round_trip_test.rs::t1927_record_then_replay_byte_identical`](../../../crates/llm/tests/replay_round_trip_test.rs) → ok.                                                                                                                                   | PASS |
| V8  | 9 strategy anchors byte-identical; 2 `report-sample-*` re-lock once at T_FINAL.             | [`crates/reports/tests/strategy_anchors_unchanged.rs::t1937_nine_strategy_anchors_unchanged`](../../../crates/reports/tests/strategy_anchors_unchanged.rs) → ok. Re-lock evidence at tester report § 5.                                                                              | PASS |
| V9  | Grep over `target/logs/*`, `data/llm-replay.db`, audit DB → no API-key substrings.          | [`crates/llm/tests/no_secrets_in_artifacts_test.rs::t1926_no_secrets_in_artifacts`](../../../crates/llm/tests/no_secrets_in_artifacts_test.rs) → ok; stdout `V9 PASS: no secret patterns found in any scanned artifact`.                                                              | PASS |
| V10 | Each provider's `complete()` < 200 ms wiremock wall; 3-provider smoke < 1 s total.          | `smoke_harness::t1924_*` → 0.39s wall (≪ 1s). Avg `complete()` ≈ 43 ms.                                                                                                                                                                                                              | PASS |
| V11 | Fixture cache schema migration forward-compat.                                              | [`crates/llm/tests/replay_schema_forward_compat.rs`](../../../crates/llm/tests/replay_schema_forward_compat.rs) → 3 / 3 ok (accepts v1, rejects v2-future structured, empty-cache permitted).                                                                                       | PASS |
| V12 | 10 parallel `complete()` calls vs $200 budget at $199.50; reconcile ≤ $200.40.              | [`crates/llm/tests/budget_stress_test.rs::t1918_v12_concurrent_overshoot_bound_holds`](../../../crates/llm/tests/budget_stress_test.rs) → ok in 0.21s; supplementary `t1918_v12_demonstrates_concurrent_overshoot` exercises the bound.                                              | PASS |

## Numbers that matter

- **Commits:** 6 developer passes (`d0bcad2` → `c61afa5` →
  `441c136` → `f1dbe05` → `f1128e9` → `faaaec1`) + 1 tester
  T_FINAL (`8a41b47`).
- **Surface:** 117 files touched, +14 500 / −456 LOC (orchestrator
  brief inventory; estimated 32 new files + 22 modified at
  architect's pre-developer scope-check —
  [orchestrator-scope-check-2026-05-10.md § Surface area](../orchestrator-scope-check-2026-05-10.md)).
- **Tasks:** 45 architect-emitted (T1901–T1945), **44 ticked
  `[x]`**, **1 ticked `[~]` deferred** (T1938 → v2.1; the
  T1915 [~] partial covers the second deferred-half) + tester
  gate `T_FINAL_V2_LLM_STRATEGY`.
- **Tests:** **1203 passed, 0 failed**, 3 pre-existing `#[ignore]`,
  across **158 binaries** at commit `faaaec1`.
- **Anchors:** **11 / 11 PASS** (9 strategy backtest anchors
  byte-identical to v1.5a; 2 `report-sample-*` re-locked once at
  T_FINAL by tester for the bundled Q5d + Q11 body changes).
- **26 anchors & constants verified** across the V-matrix
  (11 anchor SHAs + 12 V-item integration tests + 3 D1/D2/D3
  operator-decision evidence cites).
- **Non-regression contract:** 0 untouchable-crate changes (the
  9 strategy backtest anchors prove `strategy`, `risk`, `backtest`
  body-byte identity to v1.5a; T1937 negative-invariant gate
  enforces).
- **Budget ladder lands:** `$135 → $200/mo deep-think ceiling`
  (Q11=C, D3 honored); auto-degrade at 80% spend; block at 100%.
- **Concurrent-overshoot bound:** 0.2% documented + verified
  under V12 stress (10 parallel calls vs $199.50 budget;
  reconciliation ≤ $200.40).

## D1 / D2 / D3 honored

All three resumption-time decisions in
[orchestrator-scope-check-2026-05-10.md § Pause-time changelog
(2026-05-12 RESUMED)](../orchestrator-scope-check-2026-05-10.md)
are evidenced verbatim in the code:

| Decision | Resolution | Evidence |
|---|---|---|
| **D1** — Keep Q4 bonus mechanical rename `cost::LlmProvider` enum → `ProviderKind` | A (keep in v2) | [`crates/cost/src/event.rs:15`](../../../crates/cost/src/event.rs#L15) → `pub enum ProviderKind {`. 12 call sites in cost crate updated. |
| **D2** — Strict replay-only at v2.0.0; miss → panic-equivalent structured error | A (strict) | [`crates/llm/src/replay.rs:299`](../../../crates/llm/src/replay.rs#L299) → `return Err(LlmError::ReplayMiss { hash, provider, model })` inside the `None` arm; no fallthrough. |
| **D3** — Bundle Q11 denominator `$135 → $200` + Cache hit ratio row | C (bundle) | [`spec/operator-success-reports/reports/success-fixed-report-sample-7d.md:66-67`](../../operator-success-reports/reports/success-fixed-report-sample-7d.md) → `\| LLM spend \| $0.00 / $200 \|` then `\| Cache hit ratio \| 0.0% \|`. Same shape at `success-fixed-report-sample-90d.md:68-69`. |

## Deferred items

| Item | Disposition |
|---|---|
| **T1938** — Cockpit "LLM budget" tile | Deferred to **v2.1**. Depends on an unshipped `audit::query::llm_spend_this_month` API; the tile is operator-visible at first LLM-consumer ship anyway. Tracked at [`tasks.md:2482`](../tasks.md). |
| **T1915 tracing-Layer half** — pure-fn `redact()` landed; field-visitor wiring deferred | Deferred to **v2.1**. The redact helper is in [`crates/llm/src/redact.rs`](../../../crates/llm/src/redact.rs); the `tracing_subscriber::Layer` field-visitor needs a new dep. Tracked at [`tasks.md:1221`](../tasks.md) (`[~] T1915`). |
| 2 pedantic clippy warnings on [`crates/audit/src/query.rs:219, 221`](../../../crates/audit/src/query.rs#L219) (T1910 `cast_possible_truncation` on `u128 → u64`) | Cleanup queued for **v2.1**. Pedantic-tier; not a correctness bug (`.min(u128::from(u64::MAX))` makes the cast lossless). Idiomatic fix is the saturating `u64::try_from(...).unwrap_or(u64::MAX)` pattern documented in tester report § 2.1. |
| `cargo-audit` not installed in sandbox | Pre-existing infra item, out-of-v2-scope. |
| `cargo deny check licenses FAILED` on `polars-error v0.46.0` | Pre-existing since 2026-04-18 commit `b85f876`; out-of-v2-scope. |
| **`reflection-memory-llm-enrichment`** — LLM rewrite of the lesson-card `note` field | Queued post-v2 (was already deferred from v1.8.0 reflection-memory Q1=A). v2 unblocks this brief. |
| **`reflection-memory-trader-wiring`** — top-K retrieval into the trader | Queued post-v2 (was already deferred from v1.8.0 reflection-memory Q4). v2 unblocks this brief. |

## Open decisions

_None pending — ready to ship._

The three resumption-time decisions (D1 / D2 / D3) were
operator-resolved 2026-05-12 at brief resume and are evidenced
verbatim in the code per the D1 / D2 / D3 table above.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-13 (presenter): initial draft. Tester `VERDICT → PASS`
  on commit `8a41b47` (T_FINAL re-locked 2 `report-sample-*`
  anchors after Q5d + Q11 body changes); 1203 passed / 0 failed
  across 158 binaries; ANCHORS PASS (11 / 11) with 9 strategy
  anchors byte-identical to v1.5a; D1 / D2 / D3 honored.
  Workflow predates the AGENT.md `## Capability boundaries`
  amendment (2026-05-12); no retrofit applied per
  orchestrator-scope-check resumption note. Approval block left
  UN-ticked for the operator gate.
- 2026-05-13 (operator): `[x] Approved — ship`. Pre-tick gate
  PASS; tester `VERDICT → PASS` (`8a41b47`); anchors PASS 11/11;
  D1/D2/D3 honored; 1203 tests passing. Status flipped
  `draft → shipped`. v2.1 follow-ups: T1938 cockpit tile, T1915
  tracing-Layer half, T1910 pedantic clippy cleanup.
