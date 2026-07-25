---
title: Pick B — Cross-cutting safety duo strategic direction
date: 2026-05-29
authors: [analyst]
status: direction
tags: [strategy, process, tooling, route-c, safety, bundle, duo]
related:
  - docs/dev-notes/process-tooling-survey-2026-05-29.md
  - docs/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md
  - docs/dev-notes/post-v3-strategy-direction-2026-05-29.md
  - docs/dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md
  - spec/v2-1-tracing-layer-redactor/feature.md
  - spec/ui-contrast-asserter/feature.md
  - spec/v2-llm-strategy/feature.md
  - _bmad-output/planning-artifacts/architecture/decisions/0048-lab-recipe-test-harness.md
  - docs/ui-design-principles.md
  - spec/backlog.md
---

# Pick B — Cross-cutting safety duo strategic direction

> **Strategic dev-note, NOT a feature brief.** Frames the bundle
> rationale, sequencing, acceptance, risks, and operator-decide list
> for two Top-5 candidates the architect's
> [`process-tooling-survey-2026-05-29.md § Pick B`](process-tooling-survey-2026-05-29.md#pick-b--tracing-redactor-split-from-3--contrast-asserter-8)
> framed under "cross-cutting safety nets that prevent classes of
> regression without per-feature wiring." Two feature briefs
> (`v2-1-tracing-layer-redactor`, `ui-contrast-asserter`) get authored
> alongside and promoted Queue → Active under this direction.

## § Why bundle these two — same class, not same surface

Per [`process-tooling-survey-2026-05-29.md § Pick B`](process-tooling-survey-2026-05-29.md#pick-b--tracing-redactor-split-from-3--contrast-asserter-8):
the **v2.1 tracing-Layer redactor** (Top-5 rank 3) and the
**ui-contrast-asserter** (Top-5 rank 4) target SUPERFICIALLY UNRELATED
surfaces — one runs at the audit/llm boundary in `crates/llm` +
`crates/audit`; the other at the design-token layer in `crates/ui`.
The bundle framing is **not** "they touch the same files"; it is
**"both prevent classes of regression without per-feature wiring,
both are data-driven (regex rule-set / token-pair enumeration), both
ship in a few dev-days, both pay forward for every future change in
their respective domains."**

Empirical precedent for the bundle-shaped analyst output:
[`pick-a-test-infra-trifecta-2026-05-29.md`](pick-a-test-infra-trifecta-2026-05-29.md)
(commit `0cea301`) shipped three Top-5 picks under one strategic
direction because they all targeted the SAME test-infra failure class
(default-FAIL visual + state oracles surviving future iced upgrades).
Pick B inherits that pattern: ship two together so the
**"cross-cutting safety net"** mental model gets a name, a Queue
position, and a dev-note operators can point at when the next
candidate of this class surfaces.

**Durable-over-quick framing per AGENT.md 2026-05-28.** Ship the duo
under one strategic direction so:

1. The shared **WARN → gate** maturity ladder is ratified ONCE in this
   dev-note (both features start in WARN mode for ~2 weeks, then
   promote to gating). Sequencing the two features instead would mean
   re-deciding the WARN duration twice.
2. The shared **"data-driven rule set + per-feature opt-out marker"**
   shape (the redactor's regex set has a programmable allowlist;
   the asserter has an opt-out marker for tokens that physically
   can't meet WCAG) gets one operator-decide pass, not two.
3. **Maintenance follow-through**: once both ship, future safety-class
   candidates (e.g. a "linter for unfetched audit-row state" or a
   "panic-bound asserter for proptest mutators") have a Queue lane
   labeled `cross-cutting-safety-duo follow-ons` to plug into.

**The anti-pattern to avoid.** Ship contrast-asserter alone first
(~0.5d, "trivial win"), then ship redactor 2-3 weeks later. Two
operator approval cycles for what is structurally one decision class.
Worse, the WARN-mode duration gets re-decided differently between the
two (e.g. asserter ships 2-week WARN, redactor ships 1-month WARN by
operator drift), and the future "third safety net" candidate inherits
an inconsistent precedent.

The bundle framing is **NOT "ship both in one PR"** — it's "design
both under one direction so the WARN-mode ladder and opt-out shape
mesh." Sequencing below.

## § Sequencing — which goes first, which parallelizes, which gates

Both features have **independent file-scope** per the AGENT.md
§ Parallelism rules conflict matrix:

| Pair | Same file? | Same Cargo.toml? | Same artifact? | Same operator-decide Q? | Verdict |
|------|------------|-------------------|----------------|---------------------------|---------|
| `v2-1-tracing-layer-redactor` × `ui-contrast-asserter` | NO (`crates/llm/src/redact.rs` extension + `crates/audit` wiring vs `crates/ui/tests/contrast.rs` NEW) | NO (`crates/llm/Cargo.toml` adds `tracing-subscriber` runtime dep vs `crates/ui/Cargo.toml` adds `wcag-contrast-ratio` or hand-rolled dev-dep) | NO (audit ledger vs theme.rs tokens) | NO (each has own WARN-mode + allowlist Qs; bundle dev-note declares shared WARN duration default) | **PARALLEL SAFE** |
| Either × any in-flight agent | NO (5 agents currently running per orchestrator status board work on visual-fail-html-reporter Wave 1, viewport-matrix Wave 1, lab-recipe-test-harness v0.2.0 Wave A, yahoo-realdata-v0.1.4 Wave A, post-v3-trail-ui-cleanup — none of those touch redact.rs, theme.rs, or audit-write boundary) | NO | NO | NO | **PARALLEL SAFE** |

So architect M-T1 passes can spawn concurrently for both promoted
briefs. The orchestrator should kick architect on both in the same
tool-use block — same parallel-spawn pattern Pick A used for
visual-fail-html-reporter + ui-test-harness-viewport-matrix.

### Wave 1 (NOW, parallel)

The two promoted features run in parallel. Cost summary per the
process-tooling-survey:

| Feature | Investment | Wall-clock |
|---------|------------|------------|
| `v2-1-tracing-layer-redactor v0.1.0` | ~1.5 dev days + ~0.5 tester day | ~2 days |
| `ui-contrast-asserter v0.1.0` | ~0.5 dev days + ~0.25 tester day | ~1 day |
| **Bundle total** | **~2 dev days + ~0.75 tester day** | **~3 days wall-clock (parallel)** |

Sequential would be ~3+ wall-clock plus two operator approval cycles;
parallel is one operator approval cycle covering both.

### Wave 2 (none — duo ships at v1.0)

Unlike Pick A's trifecta which has an explicit Wave 2 (harness
v0.3.0+ Recipe extensions deferred until v0.2.0 ships), the safety
duo has NO Wave 2. Both features ship at v0.1.0 and immediately enter
WARN mode for the 2-week observation window before promotion to gate.
The "next safety net" candidate (e.g. proptest panic asserter,
audit-row unfetched-state linter) gets its own strategic dev-note when
it surfaces — this dev-note does not pre-position one.

## § Acceptance — what "cross-cutting safety duo v1.0 SHIPPED" means

The bundle is **SHIPPED** when ALL of the following hold:

1. **`v2-1-tracing-layer-redactor v0.1.0` SHIPPED** (operator-approved
   presentation; trace row state = `passed`). `crates/llm/src/redact.rs`
   carries the new `pub fn install_tracing_redactor() -> Result<...>`
   helper (or equivalent) that installs a `tracing_subscriber::Layer`
   field-visitor. Layer redacts API keys, JWTs, AWS-style secrets,
   password-like field values, and high-entropy strings BEFORE they
   hit the audit ledger or stdout. **WARN mode** active by default
   (Layer emits redaction events at `tracing::Level::WARN` whenever it
   masks a field, so the operator can observe false-positive rate
   during the observation window).

2. **`ui-contrast-asserter v0.1.0` SHIPPED** (operator-approved; trace
   row state = `passed`). `crates/ui/tests/contrast.rs` enumerates
   every `(fg, bg)` token pair in `crates/ui/src/theme.rs` and asserts
   WCAG 2.1 contrast ratios per
   [`docs/ui-design-principles.md ## Accessibility minimums`](../ui-design-principles.md#accessibility-minimums)
   (4.5:1 AA body, 7:1 AAA equity). **WARN mode** active by default
   (assertions emit `eprintln!` warnings on failure but PASS the test —
   the gate-vs-warn flag becomes mandatory after the 2-week observation
   window per Q-DUO-WARN below).

3. **Shared WARN-mode → gate ladder**. Both features include an
   environment-variable or `#[cfg(feature = "...")]`-gated mode flag
   that controls WARN vs gate behavior. Default at v0.1.0 ship is
   WARN. After ~2 weeks of empirical observation (no false-positive
   spikes; both produce signal-without-noise), operator flips the
   default to gate via a follow-on `v0.2.0` patch. This dev-note
   carries the **observation contract**: gather false-positive count,
   true-positive count (if any), operator feedback on opt-out
   adequacy.

4. **Shared opt-out shape**. Both features support a per-rule opt-out
   marker:
   - Redactor: caller-side `#[tracing::instrument(skip_redaction = true)]`
     equivalent OR a per-call-site `redact_layer::bypass()` guard for
     fields the LLM provider's own headers need raw (Anthropic-Version,
     etc.).
   - Asserter: per-token `#[contrast_opt_out("reason")]` attribute or
     equivalent table-driven exclusion list for tokens that physically
     can't meet WCAG (low-priority annotation grey on canvas; iconography
     fills under accent rings).
   Both opt-out markers carry a mandatory `reason: &str` so the next
   audit catches stale opt-outs.

5. **No new ADRs.** Both features are forensic-safety augmentations
   of existing surfaces. ADR-0048 (lab-recipe-test-harness) covers the
   "boundary test + FAIL-only emission" precedent for the asserter's
   shape; the v2-llm-strategy pass-3 ADR (ADR-0033 or equivalent — the
   pure-fn `redact()` ADR) covers the redactor's. Each feature only
   amends a § Changelog row.

6. **One single bundle-level operator-decide closed**. Q-DUO-WARN
   (this dev-note) ratifies the 2-week WARN-mode default for both
   features. Each feature's own brief has its own per-feature
   operator-decides; this dev-note carries the only shared one.

**Counter-example — not SHIPPED**: either feature at FAIL,
SOFT-PASS-with-deferred-rework, or skipped WARN-mode entirely (gate-
from-day-1 with empirical false-positive risk unmitigated). The
bundle does NOT ship partial.

## § Risks

### R1 — Redactor false-positives masking legitimate audit detail

The redactor's regex set (API keys, JWTs, AWS-style secrets,
password-like field values, high-entropy strings) will fire on
legitimate forensic content — Anthropic's `model_id` field
(`claude-opus-4-7` looks high-entropy-ish if entropy threshold is
too low); base64-encoded prompt cache keys; debug dumps containing
JSON with field names like `password_hash_test` (test fixture).

**Mitigation**: WARN mode for 2 weeks before promotion to gate (per
Q-DUO-WARN below). During WARN, the Layer logs every redaction at
`tracing::Level::WARN` so operator observes the false-positive rate
and tunes the regex set or expands the opt-out allowlist BEFORE gate
flip. Feature.md R-NR-1 carries the contract that any v0.1.x patch to
add new patterns MUST stay in WARN mode for an additional week.

**Falsifier**: operator observes ≥ 10 false-positive redactions per
day during WARN observation → route back to analyst for regex
tuning or expanded opt-out shape. Tracked via daily `tracing::warn!`
event count grep in audit ledger.

### R2 — Contrast asserter blocking legitimate palette evolution

The asserter enumerates `(fg, bg)` token pairs — but the cockpit theme
evolves. The lumen-design-adoption master roadmap (Phase 6 Assistant
still in progress) introduces new tokens regularly. An asserter that
gates from day 1 on a tight 4.5:1 / 7:1 threshold may block a
legitimate token-pair the operator deliberately accepted as
sub-threshold (e.g. low-priority annotation grey on canvas where the
text is purely decorative).

**Mitigation**: per-token `#[contrast_opt_out("reason")]` marker with
mandatory reason string (R-DUO-2 below). WARN mode for 2 weeks
before gate. The asserter ships with an architect-ratified opt-out
list seeded from a one-pass theme audit at M-T1.

**Falsifier**: architect M-T1 audit finds ≥ 5 tokens that need
opt-out at v0.1.0 ship → either tune the WCAG threshold per token
class (body text 4.5:1 vs annotation 3:1) or expand the opt-out
mechanism scope. Route back to analyst with audit findings.

### R3 — Redactor breaks Anthropic SDK provider headers

The Anthropic Rust SDK and equivalents send headers like
`anthropic-version: 2023-06-01` whose values can superficially match
the redactor's `anthropic-*` pattern. If the Layer redacts these BEFORE
they get sent on the wire, the LLM call fails with a 400 from
Anthropic.

**Mitigation**: the redactor is a **`tracing` Layer**, not an HTTP
middleware. It only redacts fields entering structured log events —
NOT the values about to be sent on the network. Architect M-T1
confirms wire-up site is `tracing_subscriber::Registry::with(layer)`,
NOT `reqwest::ClientBuilder::with(layer)`. This separation is the
strongest mitigation; Q-RED-2 in the redactor feature brief surfaces
this as an operator-decide to lock the contract.

**Falsifier**: redactor accidentally wired at the HTTP layer in
v0.1.0 → first LLM call after ship fails with 400 → route back to
architect for surface re-wire.

### R4 — Contrast asserter brittle to theme refactors

The asserter enumerates `(fg, bg)` pairs by reading
`crates/ui/src/theme.rs` token definitions. If a future theme refactor
(e.g. lumen Phase 7 hypothetical re-org) changes the token storage
shape from `pub const FG_3: ColorPair` to `pub const FG: [ColorPair; 4]`,
the asserter's enumeration breaks silently — it counts zero pairs and
passes vacuously.

**Mitigation**: the asserter enumerates via a **minimum-count
assertion** — `assert!(pairs.len() >= 30, "theme token enumeration
detected < 30 pairs; refactor likely broke enumeration")`. Architect
M-T1 ratifies the floor based on the M-T1 theme audit count.
Falsification probe in feature.md: developer deliberately renames one
token before final wire-up, asserts the asserter fails with the
floor-violation message.

### R5 — Cross-feature WARN-mode ladder drift

If the two features ship with different WARN-mode default durations
(e.g. asserter ships 2-week, redactor ships 4-week by analyst drift),
operators have an inconsistent mental model for "how long do
cross-cutting safety nets observe before gating?" The next safety-
net candidate (e.g. proptest panic asserter) inherits ambiguity.

**Mitigation**: Q-DUO-WARN below locks the shared default (2 weeks)
at the bundle level. Each feature inherits the default; no per-feature
override unless an operator-decide override surfaces in the feature
brief explicitly.

## § Operator-decide questions

**One bundle-level operator-decide.** Each promoted feature's own
brief has its own internal operator-decides (see Q1-Q3 in
[`v2-1-tracing-layer-redactor/feature.md`](../v2-1-tracing-layer-redactor/feature.md)
and Q1-Q2 in
[`ui-contrast-asserter/feature.md`](../ui-contrast-asserter/feature.md)),
but the bundle-level choice is the shared WARN-mode ladder.

### Q-DUO-WARN — shared WARN-mode duration before gate promotion

**Q.** How long does each feature stay in WARN mode (assertions /
redactions logged but not blocking) before promoting to gate (blocking
on false-positive-tuned threshold)?

**(Recommended — DURABLE) Option A: 2 weeks WARN per feature, shared
default.** Both features ship v0.1.0 with WARN default; operator
flips to gate via a v0.2.0 patch after 2 weeks of observation. The
2-week window matches the empirical precedent of
[`lab-recipe-test-harness v0.1.0` 2026-05-28 → v0.2.0 2026-05-29 single-week
cycle](../lab-recipe-test-harness/feature.md) for fast-feedback safety
nets, doubled because both Pick B features touch user-facing or audit-
ledger surfaces that justify a longer observation. Bundle ladder is
consistent; next safety-net candidate inherits "2 weeks WARN" as the
default precedent.

**Cost.** ~0 (the WARN flag is a one-line env var or feature flag in
each feature's brief; gate flip is a one-line patch).

**Rationale.** Per AGENT.md 2026-05-28 durable-over-quick framing: the
2-week shared ladder eliminates per-feature WARN-duration re-decision
and gives every future safety-net candidate a precedent to point at.
Cheaper to ship is "gate from day 1" (Option B fallback) but the
false-positive risk in R1 + R2 + the inability to tune the regex set
or opt-out list under load makes day-1 gate fragile.

**Option B (cheap fallback — REJECTED at analyst level).** Gate from
day 1 with no WARN mode. Saves ~0 dev-cost (no flag at all) and ~0.5d
of v0.2.0 patch work. **Rejected** per R1 + R2 risks: false-positives
that block legitimate audit content or palette evolution surface as
test failures the operator must hot-patch under pressure. The cheap
path's risk profile is "cheap until something breaks, then expensive."

**Option C (operator escape valve).** 1-week WARN per feature
(half the recommended duration). Acceptable if operator observes
clean signal during week 1 and wants the gate sooner. This is NOT a
sub-option to recommend; it's an escape valve the operator can pick
on the v0.2.0 patch trigger.

**Default**: A (Recommended DURABLE) per AGENT.md 2026-05-28.

---

**No other bundle-level operator-decide questions.** Each promoted
feature's brief carries its own per-feature operator-decides as usual
(redactor: regex set vs configurable allowlist; provider header
bypass shape; asserter: per-token opt-out marker shape).

## § What's NOT in the duo (despite looking like one)

Honest accounting — these look like Route C cross-cutting safety but
fail the bundle's "ship in a few dev-days" or "pay forward every
cycle" criterion:

- **`v2x-trading-state-bus` (#1)** — large benefit IF v2 LLM lane
  re-activates; zero benefit if dormant. Not a safety net — a
  refactor. Defer to next v2 LLM activation per process-tooling
  survey § What's NOT a compounder.
- **`ui-update-proptest` (#9) + `ui-mutants-pass` (#15)** — both
  cross-cutting safety, but combined cost ~6 dev-days (3× the duo
  budget). Promote at Month-2 cycle after the duo lands per Pick C
  framing.
- **`ui-iced-table-panic-upstream` (#17)** — one-shot bug report
  (~0.5d). Safety-adjacent but single-event; not regression-class.
  Land alongside `ui-gallery-table-cell` (#11) when that promotes.
- **Pending-operator-verifications ledger (Pick C item)** — process
  hygiene, not regression safety. Belongs in Pick C orchestrator
  hygiene bundle, not this duo.

## § ADR readiness flag

Per the 2026-05-29 codified architect contract (writing ADR =
registering atomically in `architecture/adr/README.md`), **no Pick B
feature requires a new ADR.** The redactor's pass-3 ADR (the existing
ADR covering `pub fn redact()`) carries forward; the asserter's shape
is a thin extension of ADR-0048's "boundary test" precedent for tests
that gate on cross-cutting properties.

**Possible ride-along amendment**: if architect M-T1 finds the
WARN-mode ladder warrants an `AGENT.md` stanza ("cross-cutting safety
nets default to 2-week WARN before gate"), that's a one-line addition
under AGENT.md § Communication contract or similar — NOT a new ADR.

## § Cross-references

- [`process-tooling-survey-2026-05-29.md`](process-tooling-survey-2026-05-29.md) — Top-5 ranking (Pick B)
- [`pick-a-test-infra-trifecta-2026-05-29.md`](pick-a-test-infra-trifecta-2026-05-29.md) — bundle-pattern precedent (commit `0cea301`)
- [`weekly-retro-2026-05-27-to-2026-05-29.md`](weekly-retro-2026-05-27-to-2026-05-29.md) — Route C compounder context
- [`spec/v2-llm-strategy/feature.md`](../v2-llm-strategy/feature.md) — pure-fn `redact()` (pass 3) — predecessor for the redactor brief
- [`docs/ui-design-principles.md ## Accessibility minimums`](../ui-design-principles.md#accessibility-minimums) — WCAG 4.5:1 / 7:1 contract for the asserter brief
- [`_bmad-output/planning-artifacts/architecture/decisions/0048-lab-recipe-test-harness.md`](../architecture/adr/0048-lab-recipe-test-harness.md) — boundary-test precedent for the asserter
- [`crates/llm/src/redact.rs`](../../crates/llm/src/redact.rs) — existing pure-fn `redact()` (the redactor brief extends with a Layer wire-up)
- [`docs/dev-notes/post-v3-strategy-direction-2026-05-29.md`](post-v3-strategy-direction-2026-05-29.md) — Route C compounder argument
- [`spec/backlog.md`](../backlog.md) — promotions Queue → Active

## Closing

Pick B's durable framing is **"bundle the two cross-cutting safety
nets under one strategic direction; ship parallel Wave 1; both enter
WARN mode for 2 weeks; promote to gate via v0.2.0 patches."** The
operator decides nothing at the strategic level beyond Q-DUO-WARN
(Recommended A = 2-week WARN). Each promoted feature's brief carries
the per-feature operator-decides as usual.

The duo's pay-forward shape is what distinguishes it from Pick A's
test infra (which compounds linearly with feature count) — the duo
compounds **per code change** in its respective domain. Every future
LLM call inherits redaction. Every future theme token inherits
contrast assertion. Both run zero-config across the entire surface
they cover.

## Changelog

- 2026-05-29 (analyst): direction authored under Route C Pick B
  framing per `process-tooling-survey-2026-05-29.md` architect
  recommendation. Two Wave 1 features promoted via parallel feature.md
  + tasks.md authoring (`v2-1-tracing-layer-redactor`,
  `ui-contrast-asserter`). The v2.1-redactor portion split off from
  the `v2-llm-strategy-v21-followups` Queue entry (#3); LLM-budget
  tile + clippy items stay Queue per process-tooling-survey § What's
  NOT a compounder honorable mentions. One bundle-level operator-
  decide Q-DUO-WARN (Recommended DURABLE = 2-week WARN) surfaced.
  Shared WARN → gate ladder + shared opt-out shape locked at bundle
  level. No Wave 2 — both ship at v0.1.0 and ride the v0.2.0
  patch-to-gate cycle.
