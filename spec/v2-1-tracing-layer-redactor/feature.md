---
slug: v2-1-tracing-layer-redactor
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# v2.1 tracing-Layer redactor — v0.1.0

> **Pick B Wave 1 promoted feature (cross-cutting safety duo).** Per
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md)
> this is the more-expensive of the two duo pillars (~1.5 dev days),
> biased toward DURABLE: a `tracing_subscriber::Layer` field-visitor
> that redacts API keys / JWTs / AWS-style secrets / password-like
> field values / high-entropy strings BEFORE they hit the audit ledger
> or stdout — cross-cutting safety net every future LLM call and
> structured log emit inherits automatically.

## Why

Per [`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 3`](../dev-notes/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
the **pure-fn `redact()`** shipped at v2-llm-strategy v2.0.0 pass 3 (see
[`crates/llm/src/redact.rs`](../../crates/llm/src/redact.rs)) covers
explicit `redact(secret)` call sites — every error message + audit-memo
formatter that's TYPED OUT today. But:

1. **Structured `tracing` events bypass the pure-fn surface.** A
   developer who writes `tracing::info!(api_key = key, "outbound LLM
   request")` (or worse, the `tracing::debug!(?request, ...)`
   Debug-derive shorthand) never invokes `redact()`. The field value
   goes straight into the audit ledger / stdout / file appender at
   `Level::INFO` / `Level::DEBUG`.
2. **No cross-cutting safety net exists today.** Every new LLM
   provider integration, every new `crates/audit` aggregator, every
   new structured log call across the codebase has to **remember** to
   redact. Per the v2.0.0 pass-3 deferral note in
   [`redact.rs:18-26`](../../crates/llm/src/redact.rs#L18): the Layer
   half is "deferred to a pass-4 follow-up". This brief is that pass-4.
3. **HIGH pay-forward** (per survey Rank 3): once the Layer is wired
   at `tracing_subscriber::Registry` level, every future LLM call,
   every future agent thread, every future audit aggregator inherits
   redaction WITHOUT per-call wiring. The Layer pattern is the same
   as the `lab-recipe-test-harness` boundary test pattern (ADR-0048):
   one place to wire, everywhere benefits.

Three layered consequences:

- **Audit-ledger safety**: structured-log entries written to
  `crates/audit`'s sink (file / SQLite / stream) carry redacted
  field values by default. Forensic value preserved (prefix + suffix
  visible per `redact()`'s sanitization shape); secret material
  scrubbed.
- **stdout / stderr safety**: dev-loop `tracing::debug!` traces in
  local cargo runs no longer leak keys to the operator's terminal
  scrollback.
- **Cross-feature safety for the v2 LLM lane** (when re-activated per
  process-tooling-survey § Honorable mentions for `v2x-trading-state-bus`):
  any new agent that hits Anthropic / OpenAI / future provider
  inherits the Layer at startup — no per-agent re-wiring.

Per process-tooling-survey: **MEDIUM per-cycle benefit, SMALL
investment (~1.5d), LOW maintenance**. HIGH pay-forward because the
Layer lives at the `tracing_subscriber::Registry::init()` site,
which is initialised ONCE per process and inherited by every span
and event across the program lifecycle.

## Requirements

### R1 — `tracing_subscriber::Layer` redactor wired at process init

- **R1.1** A new module `crates/llm/src/redact_layer.rs` (or
  equivalent path the architect M-T1 ratifies; recommended
  alongside the existing `redact.rs`) exports
  `pub fn redact_tracing_layer() -> impl tracing_subscriber::Layer<S>`
  (or the equivalent shape per the chosen `tracing_subscriber`
  version) implementing a `Visit`-side field-redaction interceptor.
- **R1.2** The Layer's `on_event(...)` hook intercepts every event
  whose target is below the redaction threshold (default: ALL targets).
  For each event field, the Layer's `Visit` impl checks the field
  name + field value against the rule set (per R3) and rewrites
  high-confidence matches to the `redact(value)` output before the
  event is recorded into the downstream subscriber.
- **R1.3** Wire-up site: `crates/agent/src/main.rs` (or the canonical
  `tracing_subscriber::Registry::init()` site for the cockpit /
  agent binaries). Architect M-T1 ratifies the exact site list —
  likely the agent main + cockpit main + any standalone bin.
- **R1.4** Layer order: redactor MUST be installed BEFORE any other
  Layer that emits to a persistent sink (file / SQLite / audit-write
  Layer). Architect M-T1 verifies the Registry chain ordering at
  wire-up sites.

### R2 — WARN-mode default per Pick B Q-DUO-WARN

- **R2.1** The redactor Layer ships with a `cfg`-feature OR env-var
  default that controls **WARN vs gate** behavior. WARN mode is the
  v0.1.0 default per
  [`pick-b-cross-cutting-safety-duo-2026-05-29.md § Q-DUO-WARN`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md#-q-duo-warn--shared-warn-mode-duration-before-gate-promotion).
- **R2.2** In WARN mode: every redaction the Layer performs ALSO
  emits a meta-event at `tracing::Level::WARN` via a side-channel
  (a separate Layer or direct stderr) with the field name (NOT the
  value), the rule that matched, and the count-so-far in this process
  lifetime. Operator can grep audit ledger for `redact_layer.warn`
  events to count false-positive rate.
- **R2.3** In gate mode (v0.2.0+ default after operator-flip): the
  meta-event is suppressed by default (only emitted if operator sets
  `REDACT_LAYER_VERBOSE=1`). The redaction itself is unchanged.
- **R2.4** Mode controlled by feature flag `redact_layer_warn` (cargo
  default `["redact_layer_warn"]` at v0.1.0) OR env var
  `REDACT_LAYER_MODE=warn|gate`. Architect M-T1 picks the durable
  shape per Q-RED-3 below.

### R3 — Rule set for redaction

- **R3.1** The Layer matches fields against a **regex rule set** OR
  a **configurable allowlist+denylist** per Q-RED-1 below. Operator
  decides; analyst recommends DURABLE = regex rule set with a
  per-process opt-out allowlist (so the rule set is **closed by
  default** and per-site opens are explicit + reviewable).
- **R3.2** Patterns matched in v0.1.0:
  - API key shapes: `sk-[A-Za-z0-9-]{16,}`, `sk-ant-[A-Za-z0-9-]{16,}`,
    `sk-proj-[A-Za-z0-9-]{16,}`, OpenAI-style.
  - Bearer tokens: `Bearer\s+[A-Za-z0-9.\-_=]{20,}` in field values.
  - JWTs: 3-segment `eyJ[A-Za-z0-9.\-_]+\.eyJ[A-Za-z0-9.\-_]+\.[A-Za-z0-9.\-_]+`.
  - AWS-style: `AKIA[0-9A-Z]{16}` (access key) +
    `[A-Za-z0-9/+=]{40}` in fields named `*secret*|*access*|*token*`.
  - Password-like field NAMES: `password`, `pwd`, `passwd`,
    `secret`, `api_key`, `apikey`, `auth_token`, `bearer`.
  - High-entropy strings ≥ 32 chars in fields named `*key*|*token*|*secret*`
    AND whose Shannon entropy ≥ 4.5 bits/char (high-entropy threshold;
    architect M-T1 ratifies threshold).
- **R3.3** Match on field VALUE: rewrite to `redact(value)` output
  (existing pure-fn).
- **R3.4** Match on field NAME (password-like names): rewrite VALUE
  to `redact(value)` regardless of value shape.
- **R3.5** Per-site opt-out: callers can mark a field as
  redaction-exempt via a side-band marker (Q-RED-2: provider header
  bypass shape). Default = NO bypass; opt-out is explicit + carries
  a `reason: &str`.

### R4 — Tests + falsification probes

- **R4.1** A new test in `crates/llm/tests/redact_layer.rs` (or
  `crates/llm/src/redact_layer.rs` `#[cfg(test)]` module) exercises
  each rule pattern (R3.2 list) via synthetic `tracing::info!(...)`
  events captured by a `tracing_subscriber::registry::Registry` with
  the redactor Layer installed + a test-only sink Layer that records
  events into a `Vec<RecordedEvent>`. Asserts each pattern is
  rewritten; asserts non-matching fields pass through unchanged.
- **R4.2** A WARN-mode self-test: with `REDACT_LAYER_MODE=warn`,
  emit a synthetic `tracing::info!(api_key = "sk-ant-...")` event;
  assert the test sink records BOTH the redacted event AND the WARN-
  level meta-event with field name `api_key`.
- **R4.3** A gate-mode self-test: with `REDACT_LAYER_MODE=gate`,
  emit the same event; assert only the redacted event is recorded
  (no meta-event).
- **R4.4** A pure-fn parity self-test: every input the existing
  `crates/llm/src/redact.rs` test suite (`t1915_*` tests) covers MUST
  produce the SAME output when routed through the Layer. The Layer
  reuses `redact()` verbatim — no separate sanitization logic.
- **R4.5** Falsification probe P-RED-1 (per K1 below): comment out
  the Layer's `on_event` registration; assert at least one R4.1
  pattern test FAILs with "expected redaction not applied". Confirms
  the Layer wire-up is load-bearing.

### R-NR — Non-regression contract

- **R-NR.1** Existing `redact()` pure-fn surface stays byte-identical
  (no signature change, no behavioral change). The Layer USES it; no
  re-implementation.
- **R-NR.2** Existing `t1915_*` tests in
  [`crates/llm/src/redact.rs`](../../crates/llm/src/redact.rs) all
  PASS byte-identical pre/post-merge.
- **R-NR.3** Anchored backtest reports under `spec/*/reports/` carry
  no diff (the Layer affects only tracing emit; backtest reports are
  not tracing-emitted artifacts). `bash scripts/verify_anchors.sh`
  → 75/75 PASS byte-identical.
- **R-NR.4** No production Cargo.toml dependency added to crates
  OTHER than `crates/llm` (which gains `tracing-subscriber` as a
  runtime dep) and `crates/agent` (which gains the Layer init call —
  no new dep, just usage). Architect M-T1 audits the exact deps.
- **R-NR.5** No LLM provider HTTP wire affected (per R3 risk
  mitigation: the Layer wraps `tracing`, NOT `reqwest`). First-call
  smoke test post-wire-up confirms outbound LLM calls succeed
  byte-identical to pre-wire-up.
- **R-NR.6** WARN-mode meta-events are gitignored / not committed
  to anchored reports (they live in operator's local audit ledger
  during observation only).
- **R-NR.7** Zero new design tokens, zero strings.rs adds, zero UI
  changes — backend `tracing` infra only.

## Falsifiers (K)

- **K1 — Layer's `on_event` hook misses high-entropy keys with
  unusual prefixes.** E.g. a future provider issues keys without
  `sk-` / `sk-ant-` / `Bearer` prefixes. Rule set entropy threshold
  catches if entropy is high; otherwise leaks. **Mitigation**:
  R3.2 entropy fallback rule for fields named `*key*|*token*|*secret*`;
  operator observes WARN-mode meta-events to tune the threshold.
- **K2 — Anthropic header bypass list grows unbounded.** Provider
  releases new API version → header `anthropic-version: 2026-XX-XX`
  needs bypass → operator adds to allowlist → bypass list is a
  permanent maintenance burden. **Mitigation**: Q-RED-2 ratifies
  bypass shape; the bypass is at the WIRE layer (reqwest middleware
  exempts headers from log capture), NOT at the redactor layer.
  Redactor only sees fields that already entered tracing.
- **K3 — Layer ordering bug breaks audit-write Layer.** If the
  redactor Layer is installed AFTER the audit-write Layer in the
  Registry chain, audit writes capture raw values BEFORE redaction.
  **Mitigation**: R1.4 enforces ordering; architect M-T1 verifies
  Registry chain at wire-up sites; falsification probe P-RED-2 below.
- **K4 — Tracing subscriber init is called multiple times.** If
  `tracing_subscriber::Registry::init()` is called more than once in
  test harnesses or process restarts, the Layer may double-install
  or fail to install. **Mitigation**: wire-up uses
  `tracing_subscriber::registry().try_init()` (non-panicking) AND
  the Layer is idempotent (re-installing on a Registry with the
  Layer already attached is a no-op).

## Hypotheses (H)

- **H1 — Layer impl ≤ 200 LoC** (Visit impl ~100 LoC for the rule
  set + ~50 LoC for the WARN-mode meta-event + ~50 LoC for setup).
  Matches the analyst's ~1.5d estimate at the survey level.
- **H2 — Zero false-positives observed during 2-week WARN window
  for non-LLM call sites.** The redactor's regex set is precise
  enough that audit-ledger structured logs from `crates/strategy` /
  `crates/backtest` / `crates/data` don't trigger redaction
  (verified by WARN-mode meta-event grep). LLM call sites (the
  intended target) trigger redaction at the expected rate (~1 per
  outbound request).
- **H3 — Zero existing tracing-driven tests break.** Wire-up
  Layer at process init; existing `cargo test --all` passes
  byte-identical. Falsification probe P-RED-1 confirms.

## Operator decisions

### Q-RED-1 — Rule set shape: closed regex set vs configurable allowlist

**Q.** Does the redactor ship with a **closed regex rule set** the
operator can extend via opt-out only, OR a **configurable
allowlist + denylist** the operator can mutate freely at process init?

**(Recommended — DURABLE) Option A — closed regex set + per-site
opt-out only.** v0.1.0 ships the R3.2 pattern list verbatim. Adding
new patterns requires a v0.1.x patch (analyst → architect → developer
loop). Removing patterns (false-positive tuning) is via the per-site
opt-out marker (R3.5) — explicit + reviewable + carries a `reason: &str`.

**Cost.** ~0 dev (the rule set is a `const REDACT_RULES: &[Rule]`).
v0.1.x patches as needed for new patterns. No runtime config surface.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28 durable-over-quick
framing: a closed rule set is the **safe-by-default** choice. The
configurable allowlist (Option B) lets operators silently weaken
redaction at process init (e.g. allowlist `sk-ant-` for a debug
session, forget to remove, ship to production with leaks). The
closed set forces every weakening to be explicit, code-reviewed,
and traceable to a feature.md operator-decide.

**Option B (cheap fallback).** Configurable allowlist + denylist
loaded from env var or config file at process init. Saves a v0.1.x
patch cycle for new patterns. **Rejected** per the rationale above
— the cheap path's risk profile is "operator overrides redaction
in a hurry, forgets to revert, secret leaks in audit ledger." The
closed-set choice is strictly more durable.

**Default**: A (Recommended DURABLE).

### Q-RED-2 — Provider header bypass shape (Anthropic-Version etc)

**Q.** Where in the stack does the LLM provider's OWN headers
(`anthropic-version: 2023-06-01`, `x-api-version: ...`) get exempted
from redaction?

**(Recommended — DURABLE) Option A — wire-layer exemption (reqwest
middleware), not redactor-layer bypass.** The provider's HTTP
client (`crates/llm/src/anthropic/client.rs` or equivalent)
configures `reqwest`'s middleware to NOT emit the provider header
values into structured tracing events at all. The redactor Layer
never sees them. The wire-layer is the canonical source of truth for
"this is a provider header, not a secret."

**Cost.** ~5-10 LoC per HTTP-client crate (one config line in the
existing tracing-instrumentation site). Zero impact on the redactor
Layer's surface.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: the wire-layer
exemption is the correct architectural separation. Provider headers
are PROVIDER-OWNED metadata (not secrets), and the wire layer is the
place that knows the difference. Routing through the redactor
Layer's bypass mechanism (Option B) would mean every new provider
integration touches both files (HTTP client + redactor allowlist);
the wire-layer-only choice means new providers touch ONE file.

**Option B (cheap fallback — REJECTED).** Redactor Layer carries an
internal allowlist of provider header names (`anthropic-version`,
`x-api-version`, etc); fields matching those names bypass redaction
regardless of value. **Rejected** — every new provider needs the
allowlist updated; allowlist drift risk; the bypass mechanism becomes
a permanent maintenance burden (K2 falsifier).

**Default**: A (Recommended DURABLE).

### Q-RED-3 — WARN-mode flag shape: feature flag vs env var

**Q.** How does the operator switch between WARN and gate mode?
Cargo feature flag (`redact_layer_warn`) OR runtime env var
(`REDACT_LAYER_MODE=warn|gate`)?

**(Recommended — DURABLE) Option A — runtime env var
`REDACT_LAYER_MODE=warn|gate`** with default = `warn` at v0.1.0,
operator-flippable to `gate` after the 2-week observation window
without re-compile. v0.2.0 patch changes the default to `gate`.

**Cost.** ~5 LoC for env var parse + branch in the Layer's
`on_event` hook.

**Rationale (DURABLE).** Per AGENT.md 2026-05-28: env-var control
is **operator-decideable at runtime without re-compile**. The
operator can flip a single agent process to gate mode while other
processes stay in WARN (e.g. CI runs gate, dev runs WARN), which
is the durable shape for multi-environment hygiene. Cargo feature
flag (Option B) would require re-compile per environment.

**Option B (cheap fallback — REJECTED).** Cargo feature flag
`redact_layer_warn` (default at v0.1.0; operator flips by toggling
the feature in `Cargo.toml`). Saves ~5 LoC env var parse.
**Rejected** — the per-environment re-compile cost is durably more
expensive than the env var parse cost.

**Default**: A (Recommended DURABLE).

## Verdict tree (pre-drawn)

| Q-RED-1 \ Q-RED-2 | Q-RED-2=(a) wire-layer exemption | Q-RED-2=(b) redactor allowlist |
|---|---|---|
| **Q-RED-1=(a) closed rule set** | **DURABLE — Recommended.** Closed-by-default redaction; provider headers exempted at wire layer; redactor surface stays narrow + safe; new provider = one-file change. | INCONSISTENT — closed redactor surface but a soft-mutable allowlist for provider headers. Operator-override only. |
| **Q-RED-1=(b) configurable allowlist** | INCONSISTENT — operator can weaken redactor at process init, but provider headers handled correctly. Drift risk on the redactor side. | REJECTED — both surfaces soft-mutable; every operator process can silently weaken redaction. Worst maintenance + safety profile. |

Q-RED-3 is orthogonal to Q-RED-1 / Q-RED-2 and overlays on either
verdict cell.

## Design

_Architect M-T1 fills this. Expected DURABLE-fast-skip path:
Q-RED-1 (a) ratified + Q-RED-2 (a) ratified + Q-RED-3 (a) ratified;
Layer module at `crates/llm/src/redact_layer.rs`; wire-up sites
audited and locked; `tracing-subscriber = "0.3"` runtime dep added
to `crates/llm/Cargo.toml`; existing ADR (pass-3 redact ADR) amended
with one Changelog row; no new ADR. Single M-DEV wave (~1.5 dev days)
covering rule set + Layer impl + meta-event side channel + 3-4 unit
tests + WARN/gate parity tests + wire-up at agent main._

## Backtest Scenarios

_N/A — backend tracing infrastructure feature; no backtest scenarios
attach. The R-NR.3 anchor contract carries the equivalent regression
guarantee (75/75 anchors byte-identical pre/post)._

## Implementation

_Developer fills at M-DEV._

## Verification

_Tester M-FINAL links the test-final report + observability evidence
of WARN-mode meta-events appearing in the audit ledger under
synthetic LLM call traffic + the falsification probe P-RED-1 outcome
+ `bash scripts/verify_anchors.sh` 75/75 PASS byte-identical
pre/post._

## Changelog

- 2026-05-29 (analyst): M0 brief authored under Pick B Wave 1
  promotion per [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
  R1 Layer wire-up + R2 WARN-mode + R3 rule set + R4 tests + R-NR
  (7 clauses) + K1-K4 falsifiers + H1-H3 hypotheses + Q-RED-1/2/3
  all bias DURABLE + pre-drawn 4-cell verdict tree. ~1.5 dev day +
  ~0.5 tester day estimate. Trace row
  `REQ-V2-1-TRACING-LAYER-REDACTOR-001` opened at `proposed`. Split
  from `v2-llm-strategy-v21-followups` Queue entry (#3) — LLM-budget
  tile + clippy items stay Queue per process-tooling-survey § What's
  NOT a compounder (deferred with v2 LLM lane activation). HANDOFF →
  architect (M-T1 fast-skip likely if Q-RED-1/2/3 all Recommended
  durable; pass-3 redact ADR carries forward, no new ADR).
