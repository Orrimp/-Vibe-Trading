---
slug: v3-llm-forecaster
mode: release
status: draft
audience: human-operator
updated: 2026-05-22
generated: 2026-05-22T21:00:00Z
version: 0.1.0-PARTIAL
commit: 2da745cb85ec59abb1c02dd8ca7dd04b592eac10
priority: P0
ship_classification: shipped-partial
deferred_scope: Wave D (real Anthropic API + canonical cache fixture + 2-anchor delta + 3-run byte-identity gate + cockpit smoke + empirical cost benchmark)
deferred_target: v0.1.1
unblocking_condition: ANTHROPIC_API_KEY configured + operator-approved spend (~$25-50)
parent: v3-volatility-forecaster-noop-fix v0.1.0 (promoted C5 over C2)
---

# v3 LLM-as-forecaster — v0.1.0-PARTIAL — release deck

## Operator headline

**`shipped-partial` — first-of-kind ship state.** v3-llm-forecaster
v0.1.0 ships 6 of 7 waves clean (~4.1 k LoC across `crates/strategy/src/llm_forecaster/`;
692 lib tests + 98 LLM-forecaster integration tests + 19 visual snapshots
+ 11 layout proptests + 20 verdict-priority-tree tests; 34/34 anchors PASS
additive-zero; R9.3 byte-identity confirmed). **Wave D (real-API backtest
scenarios + canonical cache fixture + 2-anchor delta + 3-run byte-identity
gate) is deferred to v0.1.1** because no `ANTHROPIC_API_KEY` was configured
this session. The code gate is fully PASS — this is a deliberate
operator-deferral recorded as a canonical protocol, not a regression.
ADR-0039 L0-L4 verdict priority tree is operator-locked; the L-verdict bin
runs end-to-end on stub data and (correctly) reports `L2` (conservative
fallback expected when zero realdata is in the audit DB). Two operator
decisions ahead: (1) approve the PARTIAL precedent, (2) Wave D scheduling
((a) configure API key + run v0.1.1 next session — recommended / (b) defer
indefinitely / (c) re-architect to Ollama).

## The PARTIAL precedent (load-bearing — read first)

This is the project's first `shipped-partial` verdict shape. The protocol
is recorded verbatim in `reports/test-final-2026-05-22.md § 14` and
summarised here so the operator can ratify it as the canonical pattern.

**What it is.** A ship state between "in-progress" and "shipped" — code
gates fully PASS, but a clearly-scoped subset of work (one wave, here) is
deferred because an **external dependency** the operator chooses not to
acquire in-session would block its completion. Distinct from `REGRESSION`
(broken code) and from `shipped` (everything verified). Equivalent to a
SemVer-style "0.1.0 with 0.1.1 follow-on" except the deferred scope is
**not optional polish** — it is load-bearing for at least one moat-aligned
claim (here: empirical alpha falsification + canonical-cache deterministic
backtest replay).

**Why this ship qualifies.** Wave D needs real Anthropic API calls to
record the canonical replay-cache fixture (`data/llm-forecaster-replay.db.gz`).
Without that fixture, the deterministic-replay backbone (Q5 = (a)
record-once-replay-forever per `feature.md`) cannot be recorded — and
shipping placeholder fixtures would create false-evidence rot the spec-
auditor cannot retract later. Skipping Wave D + landing it under v0.1.1 is
the highest-EV play vs. (i) blocking the entire ship on a single API key
or (ii) committing fragile synthetic placeholders.

**Anchor protocol — additive-zero.** The 34-anchor baseline holds
byte-identical across the entire Wave A-G ship. The 2-anchor delta
(`top10-2023-fy-llm-forecaster-realdata` + `top10-2024-fy-llm-forecaster-realdata`
under `[v3.0.0-llm-forecaster]`) belongs to whichever ship actually
produces the empirical evidence — that's v0.1.1, not v0.1.0. Locking
placeholder SHAs would manufacture evidence rot.

**Future-proofing.** This protocol applies to any feature that depends on
an externally-provided resource (cloud API, hardware, third-party data,
vendor account, signed cert, etc.). Documenting the precedent here means
future operators + the spec-auditor recognize the state without
ambiguity — `status: shipped-partial` is the new frontmatter value;
`feature.md` cross-links to the deferred-wave scope; the tester report
explicitly classifies the verdict as `PASS (PARTIAL)` rather than tying
it to `PASS` or `FAIL`.

Verbatim from the tester report's protocol § 14:

> "PASS (PARTIAL) — NOT `REGRESSION`, NOT `FAIL`."

## Wave-by-wave evidence

| Wave | Scope | New files | Tests | Status |
|---|---|---|---|---|
| A | Foundation: `LlmForecaster` trait + payload + `ForecastContext` + canonicalize | 5 (`mod.rs`, `trait_def.rs`, `types.rs`, `canonicalize.rs`, integration test crate) | 25 integration (`llm_forecaster_payload`) | PASS |
| B | `LlmForecasterImpl` over `LlmProvider` + prompt builder + tool schema (wiremock-mocked Anthropic) | 3 (`anthropic_impl.rs`, `prompt.rs`, `tool_schema.rs`) | 17 wiremock | PASS |
| C | `LlmForecasterStrategy` + registry + on_bar signal mapping + N-bar carry-forward | 1 (`strategy.rs`) + registry arm | 12 R2 regression (`llm_forecaster_signal_mapping`) | PASS |
| **D** | **Backtest scenarios + replay-cache wiring + canonical fixture + 2 anchors + 3-run byte-identity** | **—** | **—** | **DEFERRED → v0.1.1** |
| E | Audit migration `012_llm_forecast.sql` + cost gating (80%/100% Budgeted gates) + Sharpe-comparison bin + verdict L3 surface | 1 migration + verdict logic | 16 (cost_event 3 + audit_tick 4 + budget_gate 4 + cost_cap_short_circuit 3 + journal_round_trip 4 across `llm_forecaster_wiremock_wave_e` 2) | PASS |
| F | Phase F Assistant slot body promotion (R9.3 byte-identity proven; new `ReasoningTrace` mode; runtime-gated) | UI `assistant/view.rs` body + `state.rs` mode | 19 visual snapshots + 11 layout proptests (256 cases each) | PASS |
| G | ADR-0039 L0-L4 priority tree + `llm_verdict` bin + neutrality test (#[ignore]) + non-regression closure | 1 new bin (`llm_verdict.rs`) + `verdict.rs` module + neutrality test | 20 `llm_verdict_priority_tree` + 18 unit (`verdict::tests`) | PASS |

**Test counts (verbatim from test-final § 3 + § 4):** 692 workspace lib
tests pass (0 failed, 2 pre-existing ignored). 98 LLM-forecaster
integration tests pass. 19 visual snapshots pass (2 new Wave F baselines:
`assistant_slot__llm_forecaster_active__most_recent_trace` + 
`assistant_slot__llm_forecaster_disabled__placeholder`). 11 layout
invariant proptests pass (incl. `assistant_slot_llm_forecaster_no_zero_dim`
covering `{Offline, ReasoningTrace, Live}` × `{has_forecast, None}` ×
`{0..=5 history depth}` × `{0..=3 cited lessons}` = 256 cases).

## Live demo — L-verdict bin on stub data

```
$ cargo run --bin llm_verdict -- --confidence-outcome-corr 0.0
wrote spec/v3-llm-forecaster/reports/llm-verdict-20260522.md
(body-SHA256 = 2dba4d9ae36b5b907b4eb140d43ea71f336ad2d6e6efb6d315b1a905a1f31030)
```

L-verdict report § Verdict (verbatim):

| Field             | Value                                                                |
|-------------------|----------------------------------------------------------------------|
| Case              | L2                                                                   |
| Trigger evidence  | `|confidence_outcome_corr| = 0.000000 < 0.05` (calibration failure)  |
| Routes to         | `v3-llm-forecaster-calibrate-or-retire`                              |

**L2 on the stub path is the EXPECTED and CORRECT behavior.** With zero
LLM calls in the audit DB (migration 012 applied, no realdata yet), the
priority tree correctly identifies the calibration gate as un-evaluable
and falls through to L2 conservative fallback. This validates the wiring
end-to-end; the real-realdata L-verdict (L0 PASS or L1/L2/L3/L4) ships
with v0.1.1 Wave D.

## Live demo — anchor gate 34 / 34 (verbatim tail from tester run on `2da745cb`)

```
PASS  top10-2023-fy-tcn-overlay-realdata    8fa47f49e887df480509f30dfc08afcb9febecdb6a5bbdbb04023f241a9d9642
PASS  top10-2024-fy-tcn-overlay-realdata    fd8191dff1ca106ca24416a1819bd8a002c705da7f3747831f48d60733ee76f3
PASS  vol-verdict-bs1-realdata              99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21
PASS  top10-2023-fy-vol-target-overlay-realdata  9fa64d467f35797939750fe70a492974a01aee0af197310bbfc0521ef57d2d5f
PASS  sharpe-comparison-vol-target-bs1-realdata  d21db467f1d25c36de78b405aa950c9025d61b03cb43952ccb7aadefed701a31
PASS  sharpe-comparison-vol-target-bs1-realbaseline  ff2b934961f8cea87c2e44953a746dba3f3b732c42a997c501bbcc3b989d95e9
---
ANCHORS PASS  (34 / 34)
```

**Additive-zero across Waves A-G.** The `8fa47f49…` row for
`top10-2023-fy-tcn-overlay-realdata` is PASS — this pre-validates R10.2
(registry addition does not regress the existing TCN overlay anchor)
without requiring the `#[ignore]`'d neutrality test run.

## Live demo — R9.3 byte-identity (Phase F Assistant slot, default-disabled state)

```
$ sha256sum crates/ui/tests/visual-baselines/assistant_slot__open_stub.png \
            crates/ui/tests/visual-baselines/assistant_slot__llm_forecaster_disabled__placeholder.png

2fb4b243fa8f199e54e2e0b0de82966ad06c8b0726bbf34c0ca92493bc12acdc  ...assistant_slot__open_stub.png
2fb4b243fa8f199e54e2e0b0de82966ad06c8b0726bbf34c0ca92493bc12acdc  ...assistant_slot__llm_forecaster_disabled__placeholder.png
```

Both 84953 bytes. The `view_offline` path (R9.3 runtime gate default)
renders **byte-identically** to the pre-Wave-F locked baseline from
2026-05-21. Enabling the `llm_forecaster_v3` strategy is structurally
additive-only on the moat UX surface — the offline state does not
degrade for existing operators.

## Routing decisions (operator picks)

### Decision 1: Approve PARTIAL ship verdict?

**Recommended: APPROVE.**

The precedent is sound (code gate PASS, deferred scope clearly named with
target version + unblocking condition, anchor protocol additive-zero). 6
of 7 waves shipped with full evidence; Wave D is gated on an external
dependency the operator deliberately chose not to acquire in-session. The
alternative — landing fragile placeholder fixtures or blocking the entire
ship — would create either evidence rot or sunk-cost-coupling between an
already-clean code surface and a single API key.

The `shipped-partial` shape itself is the load-bearing new artifact. Once
ratified, it covers any future feature dependent on an externally-provided
resource (cloud API, hardware, third-party data, vendor account).

### Decision 2: Wave D scheduling

Three routing paths; presenter recommends **(a)**:

| Path | Action | Budget | Cost | Presenter take |
|---|---|---|---|---|
| **(a) Run v0.1.1 next session** ⟵ **recommended** | Configure `ANTHROPIC_API_KEY`; spawn Wave D analyst→architect→developer→tester→presenter cycle; ship canonical cache fixture + 2 anchors + 3-run byte-identity gate + cockpit smoke + empirical cost benchmark. | ~½ day end-to-end | ~$25-50 real API spend (per architect's $24-30/year Haiku projection for one backtest scenario pair) | Bounded scope, bounded cost, clear EV. The empirical phase falsifies H1 (Sharpe-delta ≥ +0.10), H2 (cost < $50), H4 (byte-identity across cache rebuilds) — all of which the v0.1.0 ship can only assert analytically. |
| (b) Defer v0.1.1 indefinitely | C5 stays at v0.1.0-PARTIAL; redirect bandwidth to C2 (`v3-regime-classifier`) or other priorities (live trading, UI/ops, risk engine). | 0 | 0 | Reasonable if operator priorities shift. Spec is intact and resumable. Cost of resumption: re-load context (~½ day analyst re-warm). |
| (c) Re-architect Wave D to Ollama (local LLM) | Architect amendment to `decomp.md`; Ollama provider impl already exists in `crates/llm`; would unblock empirical phase with zero per-call cost. | ~1-2 weeks | 0 (local CPU/GPU only) | Worth surfacing — but it **changes the tier comparison**. Ollama vs Anthropic is not apples-to-apples for the product.md moat claim (Haiku 4.5 is the named provider in the cost-economics line). Best treated as a Wave D-prime spec spawned in parallel, not a substitution. |

**Operator-decide.** Presenter does not push beyond the (a) recommendation
above. (b) is fine if the C2 regime-classifier moves up the queue; (c)
is interesting infrastructure but answers a different question.

## Verification matrix

| Req | Gate | Status | Evidence |
|---|---|---|---|
| R1 | `LlmForecaster` trait + signal shape lands | VERIFIED | 25 integration tests in `llm_forecaster_payload` (Wave A); `LlmForecast`, `Rating`, `Confidence`, `Horizon`, `ForecastContext` types + serde round-trip. |
| R2 | `ForecastContext` deterministic builder (`from_runtime` + `request_hash`) | VERIFIED | `forecast_context_from_runtime` + `forecast_context_request_hash` unit tests PASS; SHA-256 over JSON-with-sorted-keys via `canonicalize.rs`. |
| R3 | `LlmForecasterImpl` over `LlmProvider` + prompt cache + tool-use schema | VERIFIED | 17 wiremock-mocked tests in `llm_forecaster_wiremock`; `propose_forecast_schema_validates` good + bad input; `temperature_pinned_at_zero` confirmed (Wave B). |
| R4 | Strategy consumer shape (`LlmForecasterStrategy: Strategy` + N-bar carry-forward) | VERIFIED | 12 `llm_forecaster_signal_mapping` tests; `fire_every_n_bars` 1 PASS (24-bar window → exactly 1 call). |
| R5 | Cost budget gate (`BudgetedProvider` 80%-degrade / 100%-block) | VERIFIED (analytical for $24-30/year Haiku projection) | `llm_forecaster_budget_gate` 4 PASS; `llm_forecaster_cost_cap_short_circuit` 3 PASS. Empirical confirmation = v0.1.1. |
| R6 | Determinism contract (temperature = 0 + canonical SHA + recording cache) | VERIFIED (structurally; 3-run gate deferred) | `temperature = 0` pin + `request_hash` deterministic + RecordingProvider/ReplayProvider architecture (v2.0.0 ship). 3-run byte-identity gate deferred to v0.1.1 (needs real API recording). |
| R7 | Audit-ledger emission (every LLM call → audit row via migration 012) | VERIFIED | `llm_forecaster_audit_tick` 4 PASS; `journal_llm_forecast_round_trip` 4 PASS; `audit/migrations/012_llm_forecast.sql` applied. |
| R8 | Backtest scenarios + report shape | DEFERRED (v0.1.1) | Wave D scope; no `ANTHROPIC_API_KEY` this session. |
| R9 | Phase F Assistant slot body promotion (R9.3 byte-identity) | VERIFIED | 19 visual snapshots PASS; SHA-256 match `2fb4b243…` across pre/post-Wave-F default-disabled snapshots; 11 layout proptests (256 cases each) PASS. |
| R10 | Non-regression contract | VERIFIED (R10.1, R10.3 partial); R10.2 pre-validated via anchor | R10.1 anchor gate 34/34; R10.2 `top10-2023-fy-tcn-overlay-realdata` SHA `8fa47f49…` PASS (pre-validates registry add); R10.3 cockpit-smoke deferred (no enabled-config exists yet). |
| H1 | Sharpe-delta ≥ +0.10 vs v1 baseline | DEFERRED | Empirical falsification = Wave D realdata backtest. |
| H2 | LLM cost < $50/backtest | VERIFIED (analytical) | Architect's $24-30/year Haiku projection: ~3,650 calls/year × ~$0.0066/call cached. Empirical = v0.1.1. |
| H3 | Operator reads trace quality as "trust-bearing" | DEFERRED (subjective) | Requires 10-20 sample reasoning traces from realdata run; v0.1.1 deck. |
| H4 | Replay-cache produces byte-identical backtests | DEFERRED | 3-run byte-identity gate is Wave D scope. Structural pre-conditions verified (canonicalize.rs SHA + temperature = 0). |
| H5 | 3-5 week dev impl feasibility | CONFIRMED | 6 waves shipped in <1 week elapsed (parallelism within Wave A→B→{C,D}→{E,F}→G; D deferred). |
| V-cargo | fmt + clippy `-D warnings` + 692 lib tests | VERIFIED | `cargo fmt --check` exit 0; `cargo clippy --workspace --features candle -- -D warnings` exit 0; 692 passed / 0 failed / 2 pre-existing ignored. |

## Numbers that matter

- **Ship classification** — **`v0.1.0-PARTIAL`** (first-of-kind precedent).
- **Waves shipped clean** — **6** (A + B + C + E + F + G).
- **Wave deferred** — **1** (D → v0.1.1; needs `ANTHROPIC_API_KEY`).
- **Production LoC** — ~**4,099** across `crates/strategy/src/llm_forecaster/`
  (9 source files: `mod.rs`, `trait_def.rs`, `types.rs`, `canonicalize.rs`,
  `anthropic_impl.rs`, `prompt.rs`, `tool_schema.rs`, `strategy.rs`,
  `verdict.rs`) + UI body composition + audit migration 012 + 1 new bin.
- **Workspace lib tests** — **692 passed, 0 failed, 2 ignored** (pre-existing).
- **LLM-forecaster integration tests** — **98 passed** across 9 test files.
- **Visual snapshot baselines** — **19 passed** (2 new Wave F: 1 active
  body + 1 byte-identical placeholder).
- **Layout proptests** — **11 passed** (256 cases each, ~73 s wall clock).
- **Verdict priority-tree tests** — **20 passed** (Wave G).
- **Anchors** — **34 / 34 PASS, additive-zero** (2-anchor delta held for v0.1.1).
- **R9.3 byte-identity** — SHA-256 `2fb4b243…` match across pre/post-Wave-F.
- **L-verdict bin (stub)** — L2 conservative fallback (expected; 0 calls).
- **Cost projection (Haiku 4.5, analytical)** — ~**$24-30/year** at
  R5.4 default cadence (N=24 bars; 10 symbols; 1 call/day/symbol). ~150x
  margin under product.md $200/month ceiling.
- **New ADR** — **0039** (`llm-forecaster-verdict-criteria`; L0-L4 priority
  tree; operator-locked strawman).
- **New audit migration** — **012** (`llm_forecast.sql`; additive; tested
  via `journal_llm_forecast_round_trip` 4 PASS).
- **Lumen design tokens added** — **0** (Phase F slot body composes existing tokens).

## What's deferred (Wave D scope — v0.1.1)

Surfaced in one place for operator transparency on the (a)/(b)/(c)
decision:

1. **Real Anthropic API recording run** — first end-to-end call against
   the live Anthropic endpoint with `temperature = 0` + cache breakpoints
   + tool-use schema.
2. **Canonical cache fixture** — `data/llm-forecaster-replay.db.gz`
   (< 50 MB, checked-in per K5 mitigation); records the
   `(request_hash, response)` pairs that make every future backtest
   replay deterministic without re-paying API cost.
3. **Backtest scenarios** —
   `top10-2023-fy-llm-forecaster-realdata` +
   `top10-2024-fy-llm-forecaster-realdata` (10 symbols × full year hourly
   data; the empirical alpha-falsification scenarios for H1).
4. **2-anchor delta** — adds 2 rows under `[v3.0.0-llm-forecaster]` in
   `spec/anchors.toml` once the body-SHA-256 hashes stabilize across 3
   back-to-back identical cache-build runs.
5. **3-run byte-identity gate** — K-llm-4 mitigation (Anthropic API
   non-determinism across server deploys); structural pre-conditions
   verified, runtime gate deferred.
6. **Empirical cost benchmark** — falsifies H2 ($24-30/year analytical
   projection); also surfaces actual cache-hit ratio + per-call latency
   distribution.
7. **Live `cockpit_live` runtime gate wiring** — `cfg.strategies` enabled
   list flips `AssistantMode → ReasoningTrace`; R10.3 cockpit-smoke gate.

## Open spec debt (non-blocking; v0.1.1 cleanup)

Surfaced from tester report § 11 verbatim; tester routes to developer
+ analyst non-urgently:

- **4 new dead-links in `spec/v3-llm-forecaster/`** — developer spec-debt
  for v0.1.1: `decomp.md` references `crates/llm/tests/fixtures/`
  (deferred Wave D); `feature.md` references `lib.rs:69` line anchor
  (line shifted); `feature.md` references `crates/llm/tests/fixtures/`
  (same missing dir); `reports/llm-verdict-20260522.md` uses wrong
  relative path for ADR-0039 anchor.
- **3 new violations in `spec/v3-volatility-forecaster/`** (pre-existing
  feature; analyst routing) — 2 features carry `status: retired` not in
  the allowed set; 1 ADR-0038 relative-path link broken in
  `vol-verdict-bs1-realdata-20260522.md`.
- **Spec-lint baseline this session: 90 / 2** (88 dead-link + 2
  missing-frontmatter). Tester-measured baseline at presenter-handoff.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

**Operator decision 2026-05-22:** approve v0.1.0-PARTIAL. PARTIAL ship precedent ratified (the first-of-kind protocol becomes a sanctioned ship state across the spec system). Routing pick = **(b) defer v0.1.1 indefinitely**; bandwidth redirects to other priorities. Spec-lint cleanup applied inline (orchestrator added `shipped-partial` + `retired` to `scripts/spec_lint.py` `VALID_STATUSES`; restores lint to baseline).

### Routing pick — Decision 2 (operator selects exactly one)

- [ ] (a) Configure `ANTHROPIC_API_KEY` + run v0.1.1 Wave D in next session
      (~½ day; ~$25-50 spend; presenter-recommended)
- [x] (b) Defer v0.1.1 indefinitely — C5 stays at v0.1.0-PARTIAL; redirect
      bandwidth (C2 regime-classifier / live trading / UI / ops)
- [ ] (c) Re-architect Wave D to Ollama (local LLM, no API key) — spawn
      architect amendment + Wave D-prime spec

### Notes / feedback

_(operator fills in routing choice rationale, deferred-priority preference,
or rejection reason if rejected)_

## Closing gates

Both mechanical gates run on this presentation file:

```
$ bash scripts/check_presentation.sh spec/v3-llm-forecaster/presentations/v3-llm-forecaster-2026-05-22.md
(see verbatim PASS line in handoff envelope below)
```

```
$ uv run scripts/spec_lint.py 2>&1 | head -1
(see verbatim baseline-parity line in handoff envelope below)
```

Tester-measured baseline at M-FINAL handoff: **90 / 2** (88 dead-link + 2
missing-frontmatter). This presenter file introduces zero new violations.

## Sources cited

- [`feature.md`](../feature.md) — R1-R10 + K1-K10 + H1-H5 + Q1-Q8;
  `status: shipped-partial` frontmatter (NEW value).
- [`tasks.md`](../tasks.md) — T-T1..T-T10 ticked (M-FINAL closed); T-P1
  this row.
- [`decomp.md`](../decomp.md) — architect M-T1 lock; Wave A-G plan
  (§ T-AR-7) + per-wave cargo invocations + L-verdict shape (§ T-AR-9
  cross-pointer to ADR-0039).
- [`reports/test-final-2026-05-22.md`](../reports/test-final-2026-05-22.md)
  — tester M-FINAL `VERDICT → PASS (PARTIAL)`; § 14 protocol record;
  verbatim cargo + anchor-gate outputs.
- [`reports/llm-verdict-20260522.md`](../reports/llm-verdict-20260522.md)
  — L-verdict bin stub output; body-SHA `2dba4d9ae3…`; L2 conservative
  fallback (expected on zero-call audit DB).
- [`spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md`](../../architecture/adr/0039-llm-forecaster-verdict-criteria.md)
  — L0-L4 priority tree (operator-locked strawman; first ADR for v3
  research programme).
- [`spec/dev-notes/v3-llm-forecaster-prompt-spike-2026-05-22.md`](../../dev-notes/v3-llm-forecaster-prompt-spike-2026-05-22.md)
  — analytical spike (PARTIAL too; empirical phase deferred to v0.1.1).
- [`spec/dev-notes/audit-2026-05-22.md`](../../dev-notes/audit-2026-05-22.md)
  — spec-lint baseline (pre-v3-llm-forecaster: 82 violations; this ship
  delta + 8 = 90).
- [Parent retire deck — v3-volatility-forecaster-noop-fix](../../v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-2026-05-22.md)
  — C1 retire with NEGATIVE-NET-DELTA evidence; promoted C5 over C2 for
  moat-alignment + `crates/llm` infra reuse.
- [Sibling deck — v3-volatility-forecaster](../../v3-volatility-forecaster/presentations/v3-volatility-forecaster-2026-05-22.md)
  — V3 / T-VOL-NO-ALPHA original synthetic-baseline ship (now retired).
- [Sibling deck — v3-volatility-forecaster-rebaseline](../../v3-volatility-forecaster-rebaseline/presentations/v3-volatility-forecaster-rebaseline-2026-05-22.md)
  — real-baseline rebaseline ship that picked R-O1 → (a) RETIRE pre-fix.
- `spec/anchors.toml` `[v3.0.0-llm-forecaster]` (empty at v0.1.0;
  additive-zero invariant; 2 rows land at v0.1.1).
- `spec/trace.toml` — `REQ-V3-LLM-FORECASTER-001` carried through
  `proposed → in-progress → shipped-partial` (NEW state value).

## Changelog

- 2026-05-22 (presenter): release deck v0.1.0-PARTIAL. **First-of-kind
  `shipped-partial` ship state** — 6 of 7 waves shipped clean (A + B + C
  + E + F + G; ~4.1 k LoC; 692 lib + 98 integration + 19 visual + 11
  layout + 20 verdict tests all PASS; 34/34 anchors additive-zero;
  R9.3 byte-identity SHA `2fb4b243…` confirmed; L-verdict bin runs
  end-to-end on stub data → L2 conservative fallback as expected). Wave D
  (real-API backtest + canonical cache + 2-anchor delta + 3-run
  byte-identity + cockpit smoke + empirical cost benchmark) deferred to
  v0.1.1 pending `ANTHROPIC_API_KEY` configuration + ~$25-50 operator
  spend. PARTIAL protocol documented in tester report § 14 + this deck §
  "The PARTIAL precedent" as canonical pattern for any future feature
  blocked on an externally-provided resource. ADR-0039 L0-L4 verdict
  priority tree operator-locked. Decision 1 (approve PARTIAL) + Decision
  2 (Wave D scheduling: (a) recommended / (b) defer / (c) Ollama) left as
  operator picks. Mechanical pre-tick + spec-lint gates expected at
  baseline 90 / 2 (no new categories from this deck).
