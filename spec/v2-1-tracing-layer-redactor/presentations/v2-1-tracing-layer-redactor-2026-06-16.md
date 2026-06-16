# v2.1 tracing-Layer redactor — operator deck (v0.1.0)

- **Slug**: `v2-1-tracing-layer-redactor`
- **Version**: 0.1.0
- **Mode**: release
- **Date**: 2026-06-16
- **Upstream verdict**: tester `VERDICT → PASS` (2026-05-29; report archived to
  `spec/archive/tester-reports-2026-05-to-06.tar.gz`)
- **Re-verified for this deck**: 2026-06-16 (orchestrator re-ran the test +
  clippy gates fresh on current `main` — see § Verification matrix)

---

## TL;DR

A process-wide safety net that scrubs API keys, tokens and passwords out of
**every** log line and audit-ledger entry before it is written — ship it in
observe-only ("WARN") mode now; the enforcing flip is a separate decision later.

## What changed

- A new `tracing` Layer (`crates/llm/src/redact_layer.rs`) inspects every
  structured log event and rewrites secret-shaped field values (API keys, Bearer
  tokens, JWTs, AWS-style secrets, password-like field names, high-entropy
  strings) to a masked form **before** the event reaches stdout, a file, or the
  audit ledger.
- One shared installer (`llm::tracing_init::install_global()`) is now called by
  **17 binaries** in place of the old `tracing_subscriber::fmt().init()`. Every
  current and future binary, LLM call, agent thread and audit aggregator inherits
  redaction with zero per-call wiring.
- Ships in **WARN mode by default** (`REDACT_LAYER_MODE=warn`): nothing is
  blocked, but each redaction emits a meta-line to stderr naming the field and
  the rule that fired (never the secret value) so you can watch for
  false-positives before any enforcing flip.

## Why it matters

Today a developer who writes `tracing::info!(api_key = key, ...)` leaks that key
straight into the audit ledger and your terminal scrollback. The pure-function
redactor that shipped earlier (`crates/llm/src/redact.rs`) only covers code that
explicitly *types out* `redact(secret)` — structured-log fields bypass it
entirely. This feature closes that gap structurally: the secret is masked at the
subscriber, so the leak is caught no matter who wrote the log call or whether
they remembered to sanitize. It is the deferred "pass-4" half of the v2 LLM
secret-redaction work, now delivered.

## What the operator can do now

| Action | Command |
|---|---|
| Run a binary in observe-only mode (default — meta-lines on stderr) | `REDACT_LAYER_MODE=warn cargo run -p agent` |
| Preview the enforcing behaviour on one process without recompiling | `REDACT_LAYER_MODE=gate cargo run -p agent` |
| Keep meta-event diagnostics on while in gate mode | `REDACT_LAYER_MODE=gate REDACT_LAYER_VERBOSE=1 cargo run -p agent` |
| Count what got redacted during an observation window | `<run a binary> 2>&1 \| grep 'llm::redact_layer::meta'` |

Note: switching modes is a runtime env var — no recompile, and you can run one
process in `gate` while others stay in `warn` (e.g. CI strict, dev permissive).

## Live demo

I cannot run `cargo` from the presenter sandbox (hard constraint for this fire),
so this section shows the redaction rule set executing against its own checked-in
unit-test vectors rather than a fresh binary run. Each row below is a real
assertion in `crates/llm/src/redact_layer.rs` `#[cfg(test)] mod tests`, re-run
green by the orchestrator on 2026-06-16 (108/108 lib tests pass — see
§ Verification matrix). The left column is the input field; the right column is
what reaches the sink.

```text
input field value                                   →  what the sink sees
--------------------------------------------------- -- -----------------------
sk-ant-api03-ABCDEFGHIJKLMNOP...0123456789abc        →  sk-a***9abc  (rule: anthropic_key)
sk-proj-AbCdEf...0123456789AB                        →  sk-p***89AB  (rule: openai_proj_key)
Bearer eyJhbGci...JXVCJ9abc123456789                 →  Bearer ey***6789 (rule: bearer_token)
eyJhbGci....eyJzdWIi....SflKxwRJ...sw5c (a JWT)      →  ey***w5c     (rule: jwt)
AKIAIOSFODNN7EXAMPLE                                 →  AKIA***MPLE  (rule: aws_access)
field "password" = hunter2                           →  ***          (rule: password_field_name)
field "api_key"  = somevalue12345678                 →  ***          (rule: password_field_name)
field "session_token_value" = aB3#kP9@...(36 chars)  →  ***          (rule: entropy_fallback)
"The quick brown fox jumps over the lazy dog"        →  (unchanged — no rule fires)
"123456789"                                          →  (unchanged — no rule fires)
```

The masked shape (prefix + suffix visible, middle scrubbed) is produced by the
existing `redact()` pure-fn — the Layer reuses it verbatim, so forensic value is
preserved while the secret material is gone. The last two rows are the
negative-case assertions: plain prose and short numerics pass through untouched
(false-positive guard).

## Verification matrix

The feature's `## Verification` section in `feature.md` is a stub and the tester
report was archived after PASS, so the evidence below is the orchestrator's
**fresh re-run on 2026-06-16** against current `main`, plus the documented M-DEV
delivery results in `feature.md § Implementation`.

| ID | Claim | Status | Evidence |
|---|---|---|---|
| V1 | Layer + helper + rule set are present in the tree | VERIFIED | `crates/llm/src/redact_layer.rs`, `crates/llm/src/tracing_init.rs`, `crates/llm/tests/redact_layer.rs` all present (re-confirmed 2026-06-16) |
| V2 | Library unit tests pass | VERIFIED | `cargo test -p llm` → 108 lib passed, 1 ignored (P-RED-3 probe), 2026-06-16 |
| V3 | Layer integration tests pass | VERIFIED | `cargo test -p llm` integration tests pass (WARN/gate/verbose/marker-bypass/thread-local), 2026-06-16 |
| V4 | No clippy regressions | VERIFIED | `cargo clippy --tests -p llm -- -D warnings` → clean (forced 57s recompile), 2026-06-16 |
| V5 | Existing `redact()` pure-fn unchanged (R-NR.1/2) | VERIFIED | Parity self-tests (`t1915_parity_*`) included in the 108 lib pass; Layer reuses `redact()` verbatim |
| V6 | Anchored reports byte-identical (R-NR.3) | VERIFIED (by argument) | Tracing-only feature touches no anchored report bodies; M-DEV recorded `verify_anchors.sh` 84/84 PASS at ship; orchestrator confirms anchors unaffected, shipped harnesses untouched |
| V7 | Falsification probes load-bearing (P-RED-1/2/3) | VERIFIED | All 3 `#[ignore]` probes documented PASS at M-DEV; P-RED-3 ships ignored in the lib suite (the "1 ignored" above) |
| V8 | WARN-mode meta-events observable under live LLM traffic | N/A — deferred | _The 14-day WARN observation window is the operator-run step AFTER ship; live-traffic meta-event grep is collected during that window, not pre-ship._ |

## Numbers that matter

- **Tests**: 108 lib + integration suite green; 1 ignored (P-RED-3 rule-set probe,
  ships ignored by design); 2 further ignored falsification probes (P-RED-1/2) in
  the integration file.
- **Clippy**: clean under `-D warnings` for `-p llm --tests` (re-verified
  2026-06-16, 57s recompile).
- **Anchors**: 0 delta. 84/84 byte-identical at M-DEV; unaffected by this fire
  (tracing-only; no anchored report body touched).
- **Rule set**: 9 closed regex/heuristic rules; entropy fallback at ≥ 4.5
  bits/char over ≥ 32 chars in `*key*|*token*|*secret*` fields.
- **Wire-up surface**: 17 binaries migrated to the single `install_global()`
  helper (1 line each).
- **Default mode**: `warn` (observe-only). Enforcing (`gate`) is opt-in this
  version and becomes the default only at a later operator-decided v0.2.0.
- **spec-lint**: 70 violations on current `main` (65 dead-link + 1
  shipped-no-tests + 4 trace-broken-path) — all pre-existing and unrelated to
  this feature; **this deck adds zero**.

## Safety posture (why this is hard to weaken by accident)

- **Closed rule set.** The 9 rules are a `const`/`OnceLock<RegexSet>` in source.
  Adding a pattern needs a reviewed patch (analyst → architect → developer); the
  operator cannot silently widen *or* weaken it at runtime.
- **Provider headers exempted at the wire, not via a mutable allowlist.**
  Provider-owned metadata (`anthropic-version`, etc.) is kept out of tracing at
  the HTTP-client layer, so the redactor never has to carry a drift-prone bypass
  list.
- **Per-site opt-out is explicit and fail-safe-closed.** A field can be marked
  exempt only with both `__redact_skip` and a non-empty `__redact_reason`; a
  missing reason → the field is still redacted *and* a meta-event flags it.
- **14-day WARN window before any enforcing flip.** Every redaction emits a
  field-name + rule meta-line (never the value) so you can measure the
  false-positive rate before deciding to enforce.

## Open decisions

**One decision only — approve the v0.1.0 ship (WARN-mode default), or route back.**

The enforcing gate flip (`gate` as default) is explicitly **NOT** part of this
approval. It is a separate, later, operator-decided v0.2.0 follow-on authored at
the end of the 14-day observation window. Approving here commits you to:

- the WARN-mode meta-lines appearing on stderr for the binaries you run during
  the observation window (cosmetic; greppable; never contain secret values), and
- a later, separate decision on whether/when to flip to enforcing.

There is no anchor re-lock cost and no manual capture required for this approval.

## Approval block

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

_Notes / reason:_

---

### Intended trace change (orchestrator applies — NOT this deck)

`spec/trace.toml` row `REQ-V2-1-TRACING-LAYER-REDACTOR-001`:
`state = "tester-done"` → `state = "presenter-done"`, applied atomically with the
deck commit. The deck does not write `trace.toml`.
