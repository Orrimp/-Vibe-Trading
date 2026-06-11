---
slug: v2-1-tracing-layer-redactor
version: 0.1.0
status: tester-done
owner: presenter
priority: P2
updated: 2026-05-29
---

# v2.1 tracing-Layer redactor — v0.1.0

> **Pick B Wave 1 promoted feature (cross-cutting safety duo).** Per
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md)
> this is the more-expensive of the two duo pillars (~1.5 dev days),
> biased toward DURABLE: a `tracing_subscriber::Layer` field-visitor
> that redacts API keys / JWTs / AWS-style secrets / password-like
> field values / high-entropy strings BEFORE they hit the audit ledger
> or stdout — cross-cutting safety net every future LLM call and
> structured log emit inherits automatically.

## Why

Per [`process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 3`](../dev-notes/archive/2026-Q2/process-tooling-survey-2026-05-29.md#-top-5-deep-dives-condensed):
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
  [`pick-b-cross-cutting-safety-duo-2026-05-29.md § Q-DUO-WARN`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md#-q-duo-warn--shared-warn-mode-duration-before-gate-promotion).
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

**Architect M-T1 ratification 2026-05-29.** Fast-skip path
materialised: Q-RED-1 = (a) closed regex set ratified, Q-RED-2 = (a)
wire-layer exemption ratified, Q-RED-3 = (a) runtime env var
ratified. One material structural finding amends the analyst's
framing (see D-RED-8 below): every existing binary uses the
single-Layer `tracing_subscriber::fmt().init()` shape — the redactor
requires migrating those init sites to
`tracing_subscriber::registry().with(...)` composition. This grows
the wire-up surface from 1-2 files to ~17 binaries; the ~1.5 dev-day
estimate stays valid because the migration is mechanical (the
existing `fmt::Subscriber` becomes a `fmt::Layer` inside the
registry chain with byte-identical output).

### Operator-decision ratifications

#### Q-RED-1 (a) — Closed regex rule set ratified

The v0.1.0 rule set is locked as a `const REDACT_RULES: &[Rule]` in
`crates/llm/src/redact_layer.rs`. Concrete patterns (ratifying R3.2
verbatim plus tightened entropy clause):

| Rule key | Pattern (regex) | Match scope | Action |
|---|---|---|---|
| `anthropic_key` | `sk-ant-[A-Za-z0-9_\-]{16,}` | Field VALUE | `redact(value)` |
| `openai_proj_key` | `sk-proj-[A-Za-z0-9_\-]{16,}` | Field VALUE | `redact(value)` |
| `openai_key` | `sk-[A-Za-z0-9_\-]{16,}` (matches AFTER the two above fail) | Field VALUE | `redact(value)` |
| `bearer_token` | `Bearer\s+[A-Za-z0-9._\-=]{20,}` | Field VALUE | `redact(extracted-token)` |
| `jwt` | `eyJ[A-Za-z0-9._\-]+\.eyJ[A-Za-z0-9._\-]+\.[A-Za-z0-9._\-]+` | Field VALUE | `redact(value)` |
| `aws_access` | `AKIA[0-9A-Z]{16}` | Field VALUE | `redact(value)` |
| `aws_secret_context` | `[A-Za-z0-9/+=]{40}` IFF field name matches `*secret*\|*access*\|*token*` (case-insensitive `contains`) | Field VALUE | `redact(value)` |
| `password_field_name` | (no value regex) | Field NAME `password\|pwd\|passwd\|secret\|api_key\|apikey\|auth_token\|bearer` (case-insensitive exact-match) | `redact(value)` regardless of shape |
| `entropy_fallback` | Shannon entropy ≥ **4.5 bits/char** over ≥ **32 chars** AND field name matches `*key*\|*token*\|*secret*` (case-insensitive `contains`) | Field VALUE | `redact(value)` |

The entropy threshold of 4.5 (R3.2 architect-ratify hook) is
operator-tuned during the 2-week WARN window; v0.1.0 ships at 4.5
per analyst-recommend; v0.2.0 patches based on WARN-mode
meta-event grep evidence. Rule evaluation order is the table order
above; first match wins (so `sk-ant-` wins before the broader `sk-`
fallback). Adding new patterns requires v0.1.x patch (analyst →
architect → developer); removing patterns is via the per-site
opt-out marker (D-RED-7 below).

#### Q-RED-2 (a) — Wire-layer exemption ratified (the durable shape)

**Architect adopts the analyst's wire-layer-exemption framing
verbatim.** The redactor Layer does NOT carry an internal bypass
allowlist. Provider headers (`anthropic-version`, `anthropic-beta`,
`openai-api-version`, `user-agent`, `content-type`,
`x-request-id`) are exempted at the `reqwest` middleware site in
each provider impl (`crates/llm/src/providers/anthropic.rs` and
siblings) — those headers never enter `tracing` events. The
redactor only sees fields that already entered `tracing`, so the
bypass list is implicit + structurally enforced (a future provider
that doesn't honour the middleware contract is the bug, not the
redactor).

**Cost-side note from the architect.** This contradicts the brief
preamble's "bypass list = Anthropic-Version, OpenAI-API-Version,
User-Agent, Content-Type, X-Request-Id, anthropic-beta" framing
under Q-RED-2 in the orchestrator's M-T1 dispatch — that framing
described **Option B (rejected)**. The DURABLE shape per the
feature.md § Operator decisions table cell `(a, a)` carries an
empty bypass list inside the redactor; the equivalent exempt-list
lives at the `reqwest` middleware site as an out-of-tracing
filter. Net effect identical; durable maintenance profile better
(new provider = one-file change).

**Audit of current provider sites for compliance.** The architect
notes for the developer that `crates/llm/src/providers/` exists
but the wire-layer exemption is not yet structurally enforced.
The v0.1.0 ship contract is: **redactor Layer + redact_str
helper land first; provider-side wire-layer audit is a v0.1.x
follow-up** if the WARN-mode meta-event grep surfaces provider
headers triggering false-positives. Per K2 falsifier mitigation,
the redactor entropy threshold (4.5) is set high enough that
short structured-header values do not match — first-line defence.

#### Q-RED-3 (a) — Runtime env var `REDACT_LAYER_MODE=warn|gate` ratified

Default at v0.1.0 = `warn`. Invalid env-var value → log a
one-time `tracing::warn!` at process init + default to `warn`
(fail-safe-closed). v0.2.0 patch flips default to `gate` after
the 2-week WARN observation window closes. The `VERBOSE` knob
per R2.3 (`REDACT_LAYER_VERBOSE=1`) preserves meta-event emission
in gate mode for operator-decided diagnostics.

**14-day WARN-mode duration ratified** per the bundle Q-DUO-WARN
shape (precedent: `ui-contrast-asserter` sibling adopting the same
14-day window). v0.2.0 patch authored by analyst end-of-window
records the false-positive count + true-positive count from the
WARN-mode meta-event grep.

### D-clauses

#### D-RED-1 — Layer module location: `crates/llm/src/redact_layer.rs` (NEW)

**Architect chooses `crates/llm/src/redact_layer.rs` over the
brief-line `crates/audit/src/redactor.rs`.** Rationale:

- The pure-fn `redact()` lives at `crates/llm/src/redact.rs`. The
  Layer reuses it verbatim per R-NR.1 (R-NR contract). Co-located
  module preserves the dependency direction `llm → llm::redact_layer`
  (one crate, no inter-crate edge added).
- The brief framing of "redactor lives at the audit/llm boundary"
  is semantic, not structural. The Layer intercepts **events** in
  the global `tracing` subscriber chain — it intercepts events
  emitted by `crates/audit` AND by `crates/llm` AND by every other
  crate. The Layer's tap point is the global subscriber, not the
  audit ledger sink directly.
- `crates/audit/Cargo.toml` does NOT currently pull in
  `tracing-subscriber`. Adding it to audit would expand the audit
  crate's surface for a feature that's primarily LLM-secret-shaped.
  `crates/llm/Cargo.toml` already pulls in `tracing-subscriber` (line
  68: `tracing-subscriber = { workspace = true }`). Zero new dep.

Module shape:

```rust
// crates/llm/src/redact_layer.rs
//! Tracing-subscriber Layer that redacts secrets from event fields
//! BEFORE they hit downstream sinks (audit ledger, stdout, file).
//!
//! Wraps the pure-fn [`crate::redact::redact`] (T1915) — no separate
//! sanitisation logic (R-NR.1).

use std::borrow::Cow;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

pub fn redact_tracing_layer<S>() -> RedactLayer
where
    S: Subscriber,
{
    RedactLayer::from_env()
}

pub struct RedactLayer {
    mode: RedactMode,
    verbose: bool,
}

enum RedactMode { Warn, Gate }

impl RedactLayer { /* ctor + from_env */ }
impl<S: Subscriber> Layer<S> for RedactLayer { /* on_event */ }

/// Test seam (D-RED-6). Pure-fn boundary, no Subscriber required.
#[must_use]
pub fn redact_str(s: &str) -> Cow<'_, str> { /* run rule set; reuse redact::redact */ }
```

Wired into `crates/llm/src/lib.rs` exports:

```rust
pub mod redact_layer;
pub use redact_layer::{redact_tracing_layer, redact_str, RedactLayer};
```

#### D-RED-2 — Subscriber composition: registry chain, NOT fmt::Subscriber

**Material structural change from the brief framing.** The redactor
is a `tracing_subscriber::Layer` impl (NOT a `MakeWriter` wrapper) —
this matches the analyst's R1.1 recommendation and the `Layer`
shape already established in
`crates/reports/tests/mark_unavailable_warns_capture.rs:178`
(`tracing_subscriber::registry().with(layer)` pattern).

Required composition at every wire-up site:

```rust
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt};

let fmt_layer = fmt::layer()
    .json()
    .with_writer(std::io::stderr);

let env_filter = tracing_subscriber::EnvFilter::from_default_env()
    .add_directive("agent=info".parse()?);

registry()
    .with(env_filter)
    .with(llm::redact_tracing_layer::<_>())   // ← BEFORE fmt_layer
    .with(fmt_layer)
    .try_init()?;
```

**Layer ordering contract (R1.4).** The redactor MUST be installed
**before** the `fmt::Layer` (and before any future audit-sink Layer)
in the registry chain. In `tracing-subscriber` 0.3.x, the Layer
chain processes events top-to-bottom by registration order; the
redactor's `on_event` rewrites the event's field visitor BEFORE
downstream Layers see it. The redactor's `Visit` impl runs over the
original event values, computes redacted values, and surfaces them
via a `Visit`-based shim to downstream Layers (see D-RED-3 sketch).

**Wire-up site list (architect-locked).** Per the grep audit:

| Binary | File | Action |
|---|---|---|
| `agent` (P0) | `crates/agent/src/main.rs:54` | Migrate `fmt().init()` → `registry().with(...).try_init()` |
| `cockpit_live` (P0) | `crates/ui/src/bin/cockpit_live.rs:236` | Same |
| `cockpit` (P1) | `crates/ui/src/bin/cockpit.rs:133` | Same |
| `backtest` (P1) | `crates/backtest/src/main.rs:864` | Same |
| `llm-smoke` (P1) | `crates/llm/src/bin/llm-smoke.rs:108` | Same |
| `generate-replay-fixture` (P2) | `crates/llm/src/bin/generate-replay-fixture.rs:100` | Same |
| `llm_verdict` (P2) | `crates/trader/src/bin/llm_verdict.rs:412` | Same |
| forecast bins (P2 — 7 bins) | `crates/forecast/src/bin/*.rs` | Same |
| backtest aux bins (P2 — 2 bins) | `crates/backtest/src/bin/{threshold_sweep,run_yahoo_sma}.rs` | Same |
| data bins (P2 — 2 bins) | `crates/data/src/bin/fetch_{binance,yahoo}_klines.rs` | Same |

**Developer M-DEV scope decision.** The P0 sites
(`agent`, `cockpit_live`) MUST migrate in v0.1.0 — they're the
LLM-call-bearing binaries. The P1+P2 sites (15 binaries) MAY ship
in v0.1.0 with an **`llm::tracing_init::install_global()` helper**
that all binaries call (one-line replacement of the `fmt().init()`
block) — this caps the per-binary migration to 1 LoC per site
and 17 file touches but keeps the per-binary churn minimal.
**Architect ratifies the helper-fn approach** (better than 17
hand-rolled `registry().with(...)` blocks) — see D-RED-2 helper
sketch below.

```rust
// crates/llm/src/tracing_init.rs (NEW; sibling of redact_layer.rs)
//!
//! Workspace-wide subscriber installer. Every binary calls this
//! INSTEAD of `tracing_subscriber::fmt().init()`. Centralises the
//! redactor + fmt Layer ordering (R1.4) at a single audit point.

pub fn install_global(
    extra_directives: &[&str],
    json: bool,
) -> Result<(), tracing_subscriber::util::TryInitError> { /* ... */ }
```

Per-binary replacement (worked example, `crates/agent/src/main.rs`):

```rust
// before
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("trading=info".parse()?)
            .add_directive("agent=info".parse()?),
    )
    .json()
    .init();

// after
llm::tracing_init::install_global(&["trading=info", "agent=info"], true)?;
```

#### D-RED-3 — Visitor pattern: rewrite-on-record via wrapper Visit

The Layer's `on_event` hook constructs a `RedactingVisitor` that
wraps the downstream `Visit` and rewrites string-shaped field
values before forwarding them. Sketch (developer fills the full
impl at M-DEV):

```rust
impl<S> Layer<S> for RedactLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // tracing-subscriber 0.3 does not expose mutable event-value
        // rewriting at the Layer boundary. Strategy:
        //
        //  1. The redactor Layer captures the event's field values via a
        //     `Visit` impl that computes redacted strings into a
        //     side-channel `RedactedFields` map keyed on field name.
        //  2. Downstream sink Layers (fmt::Layer, audit-write Layer)
        //     query the side-channel via a `tracing::span::Extensions`-
        //     stashed `RedactedFields` BEFORE rendering the event.
        //
        // For v0.1.0 the simpler shape: the redactor Layer emits a
        // SECOND event at the same Level with redacted values and the
        // ORIGINAL event is filtered out by a paired filter Layer.
        // This double-emit + filter-original is the documented
        // tracing-subscriber 0.3.x pattern for content rewriting
        // (see tracing/issues/2570).
        //
        // Developer M-DEV picks ONE of:
        //   (a) Extensions side-channel — preserves single event,
        //       requires every downstream sink to know about
        //       RedactedFields (audit-write Layer when it lands).
        //   (b) Emit-redacted + filter-original — single Layer
        //       responsibility; downstream sinks see only the
        //       redacted event. Simpler for v0.1.0.
        //
        // ARCHITECT RECOMMENDS (b) for v0.1.0; (a) becomes correct
        // when the audit-write Layer lands at v0.2.0+ and needs
        // single-pass rendering.
    }
}
```

**Developer M-DEV decision recorded here:** ship (b)
emit-redacted + filter-original at v0.1.0. The redacting flow:

1. `on_event` calls `event.record(&mut RedactingVisitor)`.
2. `RedactingVisitor` collects each field name + Display-rendered
   value into a `Vec<(name, redacted_value)>`.
3. For each match, `redact_str(value)` is called; the rule set
   evaluates per D-RED-1 ratification table.
4. The Layer re-emits the event using `tracing::event!` with the
   redacted values and a span-extension marker
   `__redact_layer_emitted = true` that downstream filters drop
   the ORIGINAL event for.

Open implementation question (developer M-DEV records the
choice): whether the filter-original lives in the same Layer
(via `enabled()` returning `false` for events without the
marker) or as a sibling Layer. Architect leaves this to the
developer — both are equivalent + small.

#### D-RED-4 — Bypass mechanism: per-site opt-out via marker field (NOT a bypass allowlist)

Per Q-RED-2 ratification, there is NO bypass allowlist for
provider headers in the redactor. The per-site opt-out (R3.5) for
legitimate "this field is not a secret even though it shape-matches"
cases uses a marker-field convention:

```rust
tracing::info!(
    api_key_doc = "sk-ant-EXAMPLE-FOR-DOCS",
    __redact_skip = "api_key_doc",
    __redact_reason = "documentation example value; not a real key",
    "API key field name doc",
);
```

The Layer's `RedactingVisitor` checks for the `__redact_skip` field
naming the to-skip field + a non-empty `__redact_reason`. Missing
`reason` → no skip applied + a `tracing::warn!` meta-event
`redact_layer.warn` records the missing-reason. The bypass list as
a `const BYPASS_FIELDS: &[&str] = &[...]` (brief D-RED-4 framing)
is NOT shipped — it would conflict with Q-RED-1 (a) closed-rule-set
durable shape (operator could silently weaken the redactor by
extending the const).

Marker-field names are reserved (the Layer drops them from the
re-emitted redacted event so downstream sinks never see them).

#### D-RED-5 — WARN-mode emit: meta-event side channel

Per R2.2 + Q-RED-3 ratification, every redaction in `warn` mode
emits a `tracing::warn!` meta-event:

```rust
tracing::warn!(
    target: "redact_layer",
    field_name = %field.name(),
    rule = %rule_key,         // e.g. "anthropic_key", "entropy_fallback"
    count_so_far = process_counter.fetch_add(1, Ordering::Relaxed) + 1,
    "redacted field matched rule",
);
```

The meta-event carries the field name + rule key + a per-process
monotonic counter (atomic). The secret VALUE is NEVER part of the
meta-event payload (mitigates a meta-event-leak K-class risk).

In `gate` mode the meta-event is suppressed UNLESS
`REDACT_LAYER_VERBOSE=1`. The redaction itself is identical in
both modes.

**Meta-event recursion guard.** The meta-event itself is a
`tracing::warn!` call that flows through the subscriber chain.
The redactor checks `event.metadata().target() == "redact_layer"`
and bypasses field rewriting on its own emissions (no infinite
loop; matches the documented tracing pattern).

#### D-RED-6 — Test seam: `pub fn redact_str(s: &str) -> Cow<str>`

Per the M-T1 brief contract, a pure-fn `redact_str` ships
alongside the Layer:

```rust
#[must_use]
pub fn redact_str(s: &str) -> Cow<'_, str> {
    // Run all 9 rules from the D-RED-1 table over the input.
    // Return Borrowed(s) on no match; Owned(redacted) on match.
}
```

This is the unit-test surface. The Layer wraps `redact_str` with
span/event semantics. Unit tests at `crates/llm/src/redact_layer.rs`
`#[cfg(test)] mod tests` exercise:

- Each rule produces the expected redaction on the canonical
  positive input (R4.1).
- Each rule does NOT match on the canonical negative input (no
  false-positive on plain prose / numeric fields / short hashes).
- Pure-fn parity (R4.4): every `t1915_*` input from `redact.rs`
  routed through `redact_str` produces identical output to
  `redact::redact()` directly.

Layer-level tests at `crates/llm/tests/redact_layer.rs` (NEW
file in `crates/llm/tests/`) install the redactor + a test sink
Layer (mirroring `mark_unavailable_warns_capture.rs:178`) and
assert end-to-end:

- WARN-mode self-test (R4.2): meta-event recorded AND redacted event recorded.
- Gate-mode self-test (R4.3): redacted event only.
- Gate + verbose self-test: redacted event + meta-event.
- Marker-field bypass (D-RED-4): field with `__redact_skip` + reason passes through; field with `__redact_skip` + missing reason still redacted + warn meta-event.

#### D-RED-7 — ADR contract: amend ADR-0019 § Changelog; NO new ADR

**Architect ratifies the analyst's "no new ADR" framing.** The
redactor closes ADR-0019 (v2 LLM strategy foundation) § Q-pass-3
deferred half — the pure-fn `redact()` shipped under R8.3 in v2.0.0
pass 3, and the Layer half was deferred per `redact.rs:18-26`. This
is an additive close of a known-deferred surface, not a new design
decision. Recording shape:

```
ADR-0019 § Changelog (one line appended at architect-commit):
- 2026-05-29 (architect): v2.1 tracing-Layer redactor M-T1 ratified
  (REQ-V2-1-TRACING-LAYER-REDACTOR-001). Closes the pass-3 deferred
  half of R8.3 secret-redaction (`crates/llm/src/redact.rs:18-26`).
  Layer shape: closed regex set + per-site opt-out + 14-day WARN
  mode (REDACT_LAYER_MODE=warn|gate env var) before v0.2.0 gate
  flip. Wire-up via new `llm::tracing_init::install_global()`
  helper called by every binary. Anchor contract 0 delta (Layer
  affects tracing emit only; 75/75 byte-identical).
```

The ADR registry README `updated:` frontmatter is bumped same
commit (per architect.md § ADR registry contract atomicity).

#### D-RED-8 — Material structural finding: binary subscriber init migration (NEW from M-T1 audit)

The analyst brief assumed wire-up at "`crates/agent/src/main.rs` +
`crates/ui/src/bin/cockpit_live.rs`" — two sites. The grep audit
finds **17 binary entry points** each calling
`tracing_subscriber::fmt().init()`. Adding a Layer requires
migrating to `registry().with(...)`. The architect's response:

- Ship a **shared `llm::tracing_init::install_global()` helper**
  (D-RED-2). One LoC replacement per binary.
- **P0 binaries** (`agent`, `cockpit_live`) MUST migrate at v0.1.0
  ship.
- **P1 binaries** (`cockpit`, `backtest`, `llm-smoke`,
  `generate-replay-fixture`, `llm_verdict`) MUST migrate at v0.1.0
  ship — these are LLM-adjacent and operator-exposed.
- **P2 binaries** (10 forecast/data/aux bins) are
  non-LLM-bearing; the redactor would be a no-op (no secrets in
  their fields). They MAY migrate at v0.1.0 ship for hygiene OR
  defer to v0.1.x — developer M-DEV decision; architect
  recommends ship-all-at-once for the durable "every binary
  inherits redaction" framing per process-tooling-survey Rank 3
  HIGH pay-forward.

**Anchor impact**: zero. The migration is byte-identical output
(same `EnvFilter`, same `fmt::Layer` JSON shape). Falsification
probe P-RED-1 (D-RED-9 below) catches any drift.

#### D-RED-9 — Falsification probes ratified

**P-RED-1 (analyst-spec'd; architect-ratifies).** Comment out the
Layer's `on_event` field-rewrite logic; send a span with
`password=hunter2`; assert the test sink receives
`password=hunter2` UNREDACTED. Confirms the Layer's tap point is
correct. Revert. Recipe lives in
`crates/llm/tests/redact_layer.rs ## P-RED-1 falsification` as a
`#[ignore]` test the developer un-ignores during the probe pass.

**P-RED-2 (analyst-spec'd; architect-ratifies).** Swap the
Registry chain ordering — put `fmt::Layer` BEFORE
`redact_tracing_layer()`. Assert fmt output captures
`password=hunter2` UNREDACTED. Confirms R1.4 ordering is
load-bearing. Revert. Recipe lives in
`crates/llm/tests/redact_layer.rs ## P-RED-2 falsification` as a
`#[ignore]` test.

**P-RED-3 (NEW architect-add).** Replace `REDACT_RULES` with `&[]`
(empty rule set); assert all 9 positive-rule test cases FAIL
("expected redaction not applied"). Confirms the rule set is
load-bearing — catches a future refactor that accidentally drops
the rule-evaluation loop. Recipe lives at
`crates/llm/src/redact_layer.rs #[cfg(test)] mod p_red_3` as a
`#[ignore]` test.

### Library compatibility checklist

The redactor needs a `regex` dep at `crates/llm/Cargo.toml`. The
audit:

- [x] **Single-binary friendly** — `regex` is a pure-Rust crate,
  no system C deps, no external services. Standard workspace
  dependency.
- [x] **No system C deps** — none.
- [x] **Edition 2024 compatible** — `regex` 1.10+ supports edition
  2024 (verified upstream).
- [x] **`[package] name` does NOT shadow stdlib** — `regex` is not
  a stdlib name.
- [x] **Maintained** — `regex` is a `rust-lang/regex` BurntSushi
  crate, weekly downloads in the millions, last release < 6 months.
- [x] **License compatible** — `MIT OR Apache-2.0`, matches workspace.

The architect rejects `aho-corasick` as a separate dep for the
bypass-prefix fast-path:

- The bypass mechanism per D-RED-4 is per-site marker fields, not
  a static prefix list. No fast-path needed.
- `regex` 1.x already vendors `aho-corasick` internally for its
  `RegexSet` literal optimisation. Adding `aho-corasick` as a
  direct dep is a premature optimisation for the v0.1.0
  rule-set size (9 rules, all short patterns).

**Workspace dep already present**: `tracing-subscriber = "0.3"` is
declared at the workspace root (`Cargo.toml:48` —
`tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }`)
AND already a runtime dep of `crates/llm`
(`crates/llm/Cargo.toml:68 — tracing-subscriber = { workspace = true }`).
Zero new dep edge there.

**New dep edge to add**: `regex = { workspace = true }` at
`crates/llm/Cargo.toml` `[dependencies]` (after a corresponding
workspace declaration at `Cargo.toml [workspace.dependencies]`).

**Open architect decision (developer M-DEV picks):** whether
`once_cell::sync::Lazy<RegexSet>` (well-known pattern, requires
adding `once_cell`) or `std::sync::OnceLock<RegexSet>` (stdlib,
edition 2024 stable) — architect recommends `std::sync::OnceLock`
to avoid the new dep. Free choice for the developer if the
ergonomic gap matters.

### Wire-up sequencing (developer M-DEV gate)

1. Add `regex = "1"` workspace dep at root `Cargo.toml`
   `[workspace.dependencies]`; add `regex = { workspace = true }` at
   `crates/llm/Cargo.toml [dependencies]`. `cargo check -p llm`
   must pass.
2. Author `crates/llm/src/redact_layer.rs` with `redact_str` +
   `RedactLayer` + `RedactMode` + `redact_tracing_layer<S>()`.
3. Author `crates/llm/src/tracing_init.rs` with
   `install_global(extra_directives, json) -> Result<...>`.
4. Wire `pub mod redact_layer; pub mod tracing_init;` +
   re-exports at `crates/llm/src/lib.rs`.
5. Author `crates/llm/tests/redact_layer.rs` integration test +
   `#[cfg(test)] mod tests` in `redact_layer.rs` for unit tests.
6. Migrate P0 binaries (`agent`, `cockpit_live`) to
   `llm::tracing_init::install_global(...)`.
7. Migrate P1 binaries (`cockpit`, `backtest`, `llm-smoke`,
   `generate-replay-fixture`, `llm_verdict`).
8. Migrate P2 binaries (10 sites; bulk replacement).
9. Run P-RED-1, P-RED-2, P-RED-3 falsification probes per D-RED-9.
10. Run `cargo test --workspace` + `bash scripts/verify_anchors.sh`
    (must be 75/75 byte-identical pre/post; anchor contract is the
    R-NR.3 hard gate).

### Risk register update

K1-K4 inherit from the analyst brief; no architect-added K-class
risks. Material risk callouts from M-T1:

- **K-arch-1**: The double-emit + filter-original (D-RED-3 (b))
  pattern doubles the event-count on every redacted event before
  the filter Layer drops the original. Memory + CPU impact is
  bounded (1 extra event per redaction; the filter is O(1) on a
  metadata target check). At LLM-call rates (~1 redaction per
  outbound request, ~10 req/min sustained, ~600 redactions/hour)
  this is negligible. **Re-evaluated at v0.2.0** if the audit-write
  Layer requires single-pass rendering (then migrate to D-RED-3 (a)
  Extensions side-channel).

- **K-arch-2**: The shared `install_global()` helper concentrates
  the wire-up bug. If the helper is wrong, ALL 17 binaries are
  wrong. **Mitigation**: P-RED-1 + P-RED-2 integration tests run
  against the helper, not a hand-rolled chain — catches drift.

- **K-arch-3**: 17-binary migration risks JSON-output drift
  (someone re-types `with_writer(stderr)` differently). **Mitigation**:
  one-line per binary; the `install_global()` helper owns all
  JSON / stderr / EnvFilter shape; per-binary call passes only
  `extra_directives` + `json: bool`.

## Backtest Scenarios

_N/A — backend tracing infrastructure feature; no backtest scenarios
attach. The R-NR.3 anchor contract carries the equivalent regression
guarantee (75/75 anchors byte-identical pre/post)._

## Implementation

### M-DEV delivery 2026-05-29

**New files:**
- `crates/llm/src/redact_layer.rs` — `RedactLayer` + `RedactingFormatFields` + `redact_str()` + thread-local `REDACTED_FIELDS` / `META_EVENTS` state. 9-rule closed regex rule set (OnceLock<RegexSet> per D-RED-1 table). Shannon entropy fallback rule (ENTROPY_THRESHOLD=4.5, ENTROPY_MIN_LEN=32).
- `crates/llm/src/tracing_init.rs` — `install_global(extra_directives, json)` shared installer. Registry chain: `EnvFilter → RedactLayer → fmt::Layer<RedactingFormatFields>`.
- `crates/llm/tests/redact_layer.rs` — 9 integration tests + 2 `#[ignore]` falsification probes (P-RED-1, P-RED-2).

**Modified files (17 binary entry points migrated):**
- `crates/agent/src/main.rs` (P0)
- `crates/ui/src/bin/cockpit_live.rs` (P0) + `crates/ui/Cargo.toml` (added `llm` optional dep under `live` feature)
- `crates/backtest/src/main.rs` (P1) + `crates/backtest/Cargo.toml`
- `crates/llm/src/bin/llm-smoke.rs` (P1)
- `crates/llm/src/bin/generate-replay-fixture.rs` (P1)
- `crates/trader/src/bin/llm_verdict.rs` (P1)
- 8 `crates/forecast/src/bin/*.rs` (P2) + `crates/forecast/Cargo.toml`
- 2 `crates/backtest/src/bin/*.rs` (P2)
- 2 `crates/data/src/bin/*.rs` (P2) + `crates/data/Cargo.toml`

**Design deviation from D-RED-3(b):**
The spec's "emit-redacted + filter-original" pattern cannot work in tracing-subscriber 0.3.x: `tracing::info!()` called from inside `on_event` is silently dropped by tracing's reentrancy guard (confirmed by empirical testing). The implementation uses instead:
1. `RedactLayer::on_event` populates `REDACTED_FIELDS` thread-local with `(field_name, redacted_value)` pairs.
2. `RedactingFormatFields` (a custom `FormatFields` impl) reads `REDACTED_FIELDS` during field formatting and substitutes redacted values before they reach stdout/stderr.
3. WARN meta-events are written via `eprintln!` to stderr (inside `on_event`, bypassing the reentrancy guard) AND recorded in `META_EVENTS` thread-local for integration test assertions.

This approach satisfies R1 (Layer intercepts events before downstream sinks) and R2 (WARN-mode observability) with correct tracing-subscriber semantics.

**Test results:**
- `cargo test -p llm --lib`: 108 passed, 0 failed, 1 ignored (P-RED-3 probe)
- `cargo test -p llm --test redact_layer`: 9 passed, 0 failed, 2 ignored (P-RED-1, P-RED-2 probes)
- `cargo test -p llm -- --ignored`: all 3 probes PASS in documented state
- `cargo fmt --all --check`: clean
- `cargo clippy -p llm -p agent -p backtest -p forecast -p data -p trader -- -D warnings`: clean
- `bash scripts/verify_anchors.sh`: 84/84 PASS (zero anchor delta)

## Verification

_Tester M-FINAL links the test-final report + observability evidence
of WARN-mode meta-events appearing in the audit ledger under
synthetic LLM call traffic + the falsification probe P-RED-1 outcome
+ `bash scripts/verify_anchors.sh` 75/75 PASS byte-identical
pre/post._

## Changelog

- 2026-05-29 (analyst): M0 brief authored under Pick B Wave 1
  promotion per [`pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
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
- 2026-05-29 (architect): M-T1 design pass ratified. Q-RED-1 (a)
  closed regex set + Q-RED-2 (a) wire-layer exemption + Q-RED-3 (a)
  env var all locked verbatim from analyst recommendation. D-clauses
  D-RED-1..D-RED-9 authored (Layer at `crates/llm/src/redact_layer.rs`
  NOT `crates/audit/`; co-located with pure-fn `redact()` per R-NR.1
  reuse contract). Material structural finding D-RED-8: 17 binary
  entry points use `tracing_subscriber::fmt().init()` and must
  migrate to `registry().with(...)` shape — shared
  `llm::tracing_init::install_global()` helper introduced to cap
  per-binary churn at 1 LoC. D-RED-3 picks emit-redacted +
  filter-original pattern for v0.1.0 (simpler) over Extensions
  side-channel (correct at v0.2.0 when audit-write Layer lands).
  D-RED-9 adds P-RED-3 falsification probe (empty rule set → all
  positive tests fail) alongside analyst-spec'd P-RED-1 / P-RED-2.
  Library checklist 6/6 PASS for `regex = "1"` workspace add;
  rejected `aho-corasick` direct dep as premature optimisation;
  recommended `std::sync::OnceLock` over `once_cell`. ADR contract
  ratified — ADR-0019 § Changelog ride-along (one line); NO new ADR
  (additive close of pass-3 deferred half per `redact.rs:18-26`).
  ADR registry README `updated:` frontmatter bumped atomic-same-commit.
  Trace row state `proposed` → `arch-done`. HANDOFF → developer
  (single M-DEV wave; ~1.5 dev days; sequencing locked at D-RED §
  Wire-up sequencing 10 steps).
