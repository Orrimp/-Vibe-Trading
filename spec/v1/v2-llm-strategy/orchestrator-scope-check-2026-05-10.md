---
slug: v2-llm-strategy
type: orchestrator-scope-check
status: paused
audience: operator-on-resumption
authored: 2026-05-10
authored_by: orchestrator
---

# v2-llm-strategy — orchestrator scope-check (paused 2026-05-10)

This file is a **resumption breadcrumb**. The v2-llm-strategy feature
is mid-workflow at the architect → developer handoff. Operator paused
on 2026-05-10 with the message *"Write it down for now. I will come
to this point a while later."* Read this file first when you resume;
it holds the orchestrator's pre-developer-spawn scope-check, the
three resumption-time decisions, and the recommended next move.

## Where we are in the workflow

The standard pipeline is **analyst → architect → developer → tester
→ presenter → human**. We're paused at the gate between architect
output and developer spawn:

| Stage | Status | Commit |
|---|---|---|
| Analyst draft | Shipped | `9cde582` (1106-line brief, 14 R-items, 12 Q-items, 11 V-items, 7 milestones) |
| Operator Q-resolutions (Q1, Q2, Q3, Q10) | Shipped | `0d2056a` (Q1=A foundation-only, Q2=A Anthropic, Q3=C config-file, Q10=strawman) |
| Architect Design (Q4–Q9, Q11) + T19xx tasks | Shipped | `3f1e647` (989-line Design, 45 dev tasks T1901–T1945 + T_FINAL) |
| **Developer pass** | **Paused — awaiting operator scope-check resumption** | — |
| Tester at T_FINAL_V2_LLM_STRATEGY | Pending | — |
| Presenter release deck | Pending | — |
| Human approval | Pending | — |

## Architect Q-resolutions (one-line summary)

Full rationale lives in [`feature.md` § Design](feature.md) at lines
1167–2156. Tester / developer cite via the cross-reference rule.

| Q | Decision |
|---|---|
| Q4 trait shape | async + non-streaming + tool-use day one + batch deferred + `serde_json::Value` schemas + 8-variant `LlmError`. **Bonus mechanical rename**: `cost::LlmProvider` enum → `ProviderKind` (12 call sites in cost crate) to free the `LlmProvider` name for the trait. |
| Q5 prompt cache | TTL-driven; 2 breakpoints (project + role); provider-aware `CachedSystemPrompt::build_for(ProviderKind)`; per-role per-day Prometheus counter pair; **new `audit::query::cache_hit_ratio_since`** for a new System Health row in the operator-success-report. |
| Q6 budget gate | `BudgetedProvider<Inner>` decorator with `AtomicU64` cents counter + `try_reserve(estimate_usd)` pre-call gate + 0.2% concurrent-overshoot bound documented. **New V12** verification gate added. |
| Q7 cost-rate lookup | Hybrid: hard-coded base table at `crates/llm/src/pricing.rs` + TOML override at `[llm.pricing.<provider>.<model>]`. Module owned by `llm` crate (preserves `llm` → `cost` dep edge). |
| Q8 replay storage | SQLite WAL at `data/llm-replay.db` + canonical-JSON SHA-256 (`serde-canonical-json`) + `schema_version` column + 9-row fixture cache (3 providers × 3 roles) + per-process tokio Mutex for writer + **strict replay-only at v2.0.0** (best-effort fallthrough deferred to v3). |
| Q9 retries | Exponential backoff with full jitter, 3 retries, no circuit breaker at v2.0.0, `Retry-After` header honored, retry policy in leaf provider impl (not generic decorator). |
| Q11 report denominator | **Option C confirmed.** 1-line `$135 → $200` hot-fix bundled with Q5d's Cache hit ratio System Health row. The two `report-sample-*` anchors re-lock once at `T_FINAL_V2_LLM_STRATEGY` (tester only; architect did **not** pre-modify `spec/anchors.toml`). The 9 strategy backtest anchors at `spec/anchors.toml:15-58` stay byte-identical (R14.2 enforced via T1937 negative-invariant gate). |

## Surface area

- **32 new files** (12 `crates/llm/` source files + 1 SQL migration + 1 fixture replay DB + 8 integration tests + `llm-smoke` binary + `config/agent.toml.local.example` template + 2 new runbooks at `spec/runbooks/llm-{cost,replay}.md`).
- **22 modified files** (existing `crates/llm/` v0 stub swap; `crates/cost/{event,lib,sink,budget}.rs` absorb the `ProviderKind` rename + the `Llm` event extension; `crates/audit/src/{query,journal}.rs` add `cache_hit_ratio_since`; `crates/agent/src/{config,main}.rs` wire `LlmConfig` from `agent.toml.local` + factory at boot; `config/agent.toml` gets the new `[llm]` block; `crates/reports/src/{lib,render/system_health}.rs` land the Q5d row + the Q11 denominator hot-fix; the two `spec/operator-success-reports/reports/success-fixed-report-sample-*.md` bodies regenerate; `spec/architecture.md` stub at `:421-432` is replaced + the new "v2 — LLM strategy resolutions (Q4–Q11) — confirmed 2026-05-10" decisions-index section appends).
- **45 developer tasks** T1901–T1945 + `T_FINAL_V2_LLM_STRATEGY` (tester gate).

## Estimate

This is **3–5× the developer surface of reflection-memory** (14 tasks
/ 23 new files / 11 modified). Likely **~3–5 days of focused
developer work, multi-pass.** The orchestrator will resume the
developer on context-budget cuts at clean tick boundaries (the same
pattern reflection-memory used; that ran cleanly in one pass at 14
tasks but 45 will need 2–4 passes).

The Q4 bonus rename touches 12 cost-crate call sites; those land at
the very start (T1901) as a pre-flight cleanup before the new trait
surface arrives. If a developer pass cuts mid-T1901, the rename is
self-contained enough to leave half-done without breaking the
workspace (the rename is mechanical sed-style; missing call sites
fail to compile loudly, not silently).

## Three resumption-time decisions

The orchestrator already approved the design as a unit, but **three
specific items deserve operator confirmation before the developer
spawns**. Each has a default the orchestrator picked; "go" at
resumption time accepts all three.

### Decision 1 — Q4 bonus mechanical rename: keep or defer?

The architect added a rename **outside the strict Q4 ask**:
`cost::LlmProvider` enum → `ProviderKind` (12 call sites in cost
crate). It's defensible — frees the `LlmProvider` name for the trait
that's the heart of the feature — but it pulls a name change into
the v2 commit history.

| Option | Tradeoff |
|---|---|
| **A — keep the rename in v2** *(orchestrator default)* | Cleanup gets harder later and easier now while the cost crate is being touched anyway. Mechanical sed-style; 12 call sites; loud compile failure if any miss. |
| B — defer rename to follow-up PR | Trait would be named `LlmProviderTrait` or live in a sibling module (`crates/llm/src/provider.rs`). Keeps v2 strictly additive on the cost crate side; cleaner blame separation between v2 implementation and post-v2 cleanup. |

### Decision 2 — Q8 replay scope: strict or best-effort?

The architect picked **strict replay-only** at v2.0.0 — the
`ReplayProvider` panics on any cache miss in research mode. Best-
effort fallthrough (miss → call live API → record) is deferred to v3.

| Option | Tradeoff |
|---|---|
| **A — strict at v2** *(orchestrator default, architect's pick)* | Determinism is the primary research-mode contract (per `spec/product.md` operating modes line 290+). Strict makes test failures deterministic too — a missing fixture row is a loud test failure, not a silent live-API call that costs money + breaks reproducibility. |
| B — best-effort at v2 | More ergonomic during development (no need to seed fixtures before iterating). *Con:* breaks determinism; risks accidental live-API calls during `cargo test`. |

### Decision 3 — Q11 denominator change: bundle or defer?

Q11 changes the operator-success-report's `LLM spend` line from
`$X / $135` to `$X / $200` (v2 ceiling per `spec/product.md:339`).
The architect picked **Option C: bundle in v2** — re-locks the two
`report-sample-*` anchors once at T_FINAL.

| Option | Tradeoff |
|---|---|
| **C — bundle in v2** *(orchestrator default, architect's pick)* | Report immediately reflects the v2 ladder. Anchor re-lock is bundled with the Q5d cache-hit row addition (one re-lock cycle, not two). |
| A | Deferred to first LLM-consumer brief — report keeps showing `$135` post-v2 ship even though the ceiling moved to `$200`. Anchor stays at the current SHA. |
| B | Architect-only update of the rendered string in this brief — same outcome as C since the body change drives the re-lock either way. |

## Hard constraints — all preserved

The architect Design preserves every hard constraint the
orchestrator pinned at architect-spawn time:

1. `spec/anchors.toml` not pre-modified by the architect (tester-only at T_FINAL).
2. The 9 strategy backtest anchors at `spec/anchors.toml:15-58` stay byte-identical (R14.2 / V8 / hard constraint #2). Negative-invariant test at T1937 enforces.
3. `Strategy` trait shape unchanged (no strategy code wires to LLM in v2.0.0 under Q1=foundation-only).
4. No new bus channel — `agent::bus::Bus` shape stays unchanged.
5. Atomic-write contract preserved — replay cache uses SQLite WAL.
6. No secrets in committed artifacts — V9 grep gate extended to **every artifact** written during a smoke run, not just logs (per Q3=C resolution).
7. Anthropic-isms stay behind provider impl — trait remains provider-agnostic.
8. Body-vs-front-matter discipline preserved — only Q11 (denominator) + Q5d (cache hit ratio row) cause body-byte changes; both are explicit in the brief and re-locked once at T_FINAL.
9. Q11 disposition documented — 2 re-locks at T_FINAL, 9 strategy anchors stay byte-identical → final gate is `ANCHORS PASS (11 / 11)`.

## Resumption playbook

When you come back, the workflow is:

1. **Read this file.** Read the [feature.md](feature.md) Design
   section for the deep version of any Q-resolution.
2. **Make the three resumption-time decisions** in the section
   above. The default is "go" — accepts all three orchestrator
   recommendations (A / A / C).
3. **Tell the orchestrator** *"go ahead — keep rename / strict
   replay / bundle Q11"* (or whatever overrides you pick).
4. The orchestrator spawns the developer for T1901–T1945. Multi-
   pass expected; orchestrator resumes the developer on each
   context-budget cut. Each developer pass commits at the clean
   tick boundary it stops at.
5. After T1945 lands, the orchestrator spawns the tester for
   `T_FINAL_V2_LLM_STRATEGY` — re-locks the 2 `report-sample-*`
   anchors, runs the V1–V12 verification matrix, writes an
   immutable tester report at
   `spec/v2-llm-strategy/reports/test-2026-MM-DD-HHMM-v2-llm-strategy-final.md`.
6. After tester `VERDICT → PASS`, the orchestrator spawns the
   presenter for the release deck at
   `spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-MM-DD.md`.
7. Operator approves (or rejects) the deck. Approval ticks the
   `[x]` on the deck. Backlog moves v2-llm-strategy from Active to
   Recent.

## Useful greps for resumption

```bash
# Read the architect Q-resolutions in order:
sed -n '1167,2156p' spec/v2-llm-strategy/feature.md

# Read the developer's task list in order:
grep -E '^- \[ \] \*\*T19' spec/v2-llm-strategy/tasks.md

# Read the architect's Crate / module surface table:
sed -n '/^### Crate \/ module surface/,/^### /p' spec/v2-llm-strategy/feature.md

# Read the new architecture.md decisions-index section the architect added:
grep -nE '^### v2 — LLM strategy resolutions' spec/architecture.md
```

## Resumption-time orchestrator recommendation

**Defaults are correct as-is.** Say "go" to accept all three
decisions and spawn the developer. The architect's Design is tight
(989 lines, every R-item mapped to a T-task, all 9 hard constraints
preserved with explicit negative-invariant tests). The risk is
context budget — 45 tasks at this size will need multiple developer
passes. That's fine; reflection-memory's pattern of orchestrator-
managed multi-pass spawns scales here.

If you want to push back on any of the three decisions, the
orchestrator routes back to the architect for revision before the
developer spawns. Each revision is cheaper than during developer
implementation, so push back now if anything feels wrong.

## Cross-references

- [`spec/v2-llm-strategy/feature.md`](feature.md) — the brief (analyst + architect + operator Q-resolutions baked in).
- [`spec/v2-llm-strategy/tasks.md`](tasks.md) — task list (T1901–T1945 + T_FINAL).
- [`spec/backlog.md`](../../backlog.md) Active — paused annotation points back at this file.
- [`spec/architecture.md`](../../architecture.md) — the architect appended a "v2 — LLM strategy resolutions (Q4–Q11) — confirmed 2026-05-10" decisions-index section.
- [`spec/product.md`](../../product.md) — LLM strategy section (lines 240–258) + cost economics (332+) + memory loop (262+) + agent roster (105+).
- [`spec/reflection-memory/feature.md`](../reflection-memory/feature.md) — Q1 + Q4 carry-forward consumers blocked on this brief (`reflection-memory-llm-enrichment`, `reflection-memory-trader-wiring`).
- [`crates/llm/src/lib.rs`](../../../crates/llm/src/lib.rs) — the v0 stub being replaced.
- [`crates/cost/src/lib.rs`](../../../crates/cost/src/lib.rs) — the wired scaffolding the new trait + factory consume.

## Pause-time changelog

- 2026-05-10 (orchestrator, paused): operator paused at architect → developer handoff with *"Write it down for now. I will come to this point a while later."* Three resumption-time decisions documented in this file. Orchestrator's recommendation is "accept all defaults" (A / A / C) but operator can override at resumption.
- 2026-05-12 (orchestrator, **RESUMED**): operator confirmed all three resumption-time decisions:
  - **D1 = A** — Keep Q4 bonus mechanical rename (`cost::LlmProvider` enum → `ProviderKind`, 12 call sites) in v2.
  - **D2 = A** — Strict replay-only at v2.0.0. `ReplayProvider` panics on any cache miss in research mode. Best-effort fallthrough deferred to v3.
  - **D3 = C** — Bundle Q11 denominator change (`$135 → $200`) in v2. Two `report-sample-*` anchors re-lock once at `T_FINAL_V2_LLM_STRATEGY` alongside Q5d cache-hit-ratio row addition.
  Developer pass spawning. Status: in-progress → architect-design-ratified-by-operator → developer-multi-pass. Workflow note: the brief predates the [`AGENT.md ## Capability boundaries`](../../../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent) amendment (committed 2026-05-12). No retrofit applied — brief is structurally sound (R-items + V-items + T-tasks all mapped, 9 hard constraints preserved). New `## Hypothesis register` pattern + test-runner/evaluator split applies to the NEXT feature.
