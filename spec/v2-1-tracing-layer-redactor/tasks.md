---
slug: v2-1-tracing-layer-redactor
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — v2-1-tracing-layer-redactor v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick B Wave 1 promotion in
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
> ~1.5 dev day + ~0.5 tester day total. Bias DURABLE per
> [AGENT.md § Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-RED-M0.1 — Feature brief authored — _accept: feature.md R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H3 + Q-RED-1/2/3 + pre-drawn 4-cell verdict tree_
- [x] T-RED-M0.2 — Bundle direction dev-note authored — _accept: `spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md` ships with bundle framing + Q-DUO-WARN_
- [x] T-RED-M0.3 — Backlog Active row appended under § Process / tooling — _accept: PROMOTED Queue → Active 2026-05-29 annotation; Queue #3 (v2-llm-strategy-v21-followups) annotated with redactor-portion split-off_
- [x] T-RED-M0.4 — Trace row `REQ-V2-1-TRACING-LAYER-REDACTOR-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect

- [ ] T-RED-T1.1 — Ratify Q-RED-1 (rule set shape) + Q-RED-2 (provider header bypass) + Q-RED-3 (WARN-mode flag shape) per § Operator decisions — _accept: § Design records each Q ratified or operator-decided; fast-skip if all DURABLE_
- [ ] T-RED-T1.2 — Pick Layer module filename + struct + function signature — _accept: `crates/llm/src/redact_layer.rs` (or M-T1-ratified path); `pub fn redact_tracing_layer() -> impl tracing_subscriber::Layer<S>` or equivalent shape; Visit impl signature locked_
- [ ] T-RED-T1.3 — Audit wire-up sites for the Layer in agent / cockpit / bin mains — _accept: § Design lists the canonical `tracing_subscriber::Registry::init()` sites (likely `crates/agent/src/main.rs` + `crates/ui/src/bin/cockpit_live.rs`); Registry-chain ordering verified (redactor BEFORE audit-write Layer)_
- [ ] T-RED-T1.4 — Add `tracing-subscriber = "0.3"` to `crates/llm/Cargo.toml` `[dependencies]` (runtime) — _accept: cargo check passes; library-compatibility checklist 6/6 PASS_
- [ ] T-RED-T1.5 — Confirm existing pass-3 redact ADR amendment shape (one Changelog row; no new ADR) — _accept: § Design D-RED-N spec'd; row text drafted for architect-commit_
- [ ] T-RED-T1.6 — Wave decomposition — _accept: single M-DEV wave; ~1.5 dev days; D-RED bullets cover rule set + Layer impl + WARN-mode meta-event + 4 unit tests + WARN/gate parity tests + wire-up at agent main_
- [ ] T-RED-T1.7 — Falsification probes P-RED-1 (Layer wire-up load-bearing) + P-RED-2 (Layer ordering) spec'd — _accept: § Design lists each probe with comment-out-then-cargo-test recipe_
- [ ] T-RED-T1.8 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _accept: feature.md + tasks.md frontmatter updated_

## M-DEV — Developer (single wave; ~1.5 days; architect-ratified)

- [ ] T-RED-D1 — Add `tracing-subscriber = "0.3"` to `crates/llm/Cargo.toml [dependencies]` per T-RED-T1.4 — _accept: `cargo check -p llm` succeeds; existing version (if any) in `Cargo.lock` resolves cleanly_
- [ ] T-RED-D2 — Author `crates/llm/src/redact_layer.rs` per § Design D-RED-1+2 — _accept: file exists; exports `pub fn redact_tracing_layer()` (or M-T1 signature); `Visit` impl per R3 rule set; uses existing `crate::redact::redact()` for sanitization (R-NR.1)_
- [ ] T-RED-D3 — Implement R3.2 pattern list: `sk-`, `sk-ant-`, `sk-proj-`, `Bearer`, JWT, AWS access+secret, password-like field names, high-entropy ≥ 32 chars threshold per architect ratification — _accept: unit tests per pattern PASS_
- [ ] T-RED-D4 — Implement WARN-mode meta-event side channel per R2.2 — _accept: with `REDACT_LAYER_MODE=warn`, each redaction emits a `redact_layer.warn` event with field name + rule + count; test asserts event recorded by sink_
- [ ] T-RED-D5 — Implement gate-mode behavior per R2.3 — _accept: with `REDACT_LAYER_MODE=gate`, redaction performed but no meta-event recorded (unless `REDACT_LAYER_VERBOSE=1`); test asserts no meta-event_
- [ ] T-RED-D6 — Implement env-var mode flag parse per Q-RED-3 ratification — _accept: `REDACT_LAYER_MODE` parse at process init; default = `warn`; invalid value → log warning + default to `warn`_
- [ ] T-RED-D7 — Wire `redact_tracing_layer()` into `Registry::init()` sites per T-RED-T1.3 — _accept: Layer installed BEFORE audit-write Layer; existing logs continue to flow; smoke test confirms LLM outbound call succeeds (R-NR.5)_
- [ ] T-RED-D8 — Author pure-fn parity self-test per R4.4 — _accept: every `t1915_*` input from `crates/llm/src/redact.rs` produces identical output through Layer; test PASS_
- [ ] T-RED-D9 — Run falsification probe P-RED-1 — _accept: comment out Layer registration in `crates/llm/src/redact_layer.rs`; cargo test for at least one R4.1 pattern FAILs with "expected redaction not applied"; revert and ship_
- [ ] T-RED-D10 — Run falsification probe P-RED-2 — _accept: swap Layer ordering in `Registry::init()`; verify audit-write Layer captures raw values (FAIL); revert to correct ordering_
- [ ] T-RED-D11 — Update `spec/trace.toml` `REQ-V2-1-TRACING-LAYER-REDACTOR-001` row — _accept: `crates` populated; `tests` lists test fn names; `state` = `dev-done`_
- [ ] T-RED-D12 — Dev-side gates — _accept: `cargo fmt -p llm -- --check` clean; `cargo test -p llm` PASS; `bash scripts/verify_anchors.sh` → 75/75 PASS byte-identical pre/post_

## M-FINAL — Tester

- [ ] T-RED-FINAL.1 — Run `cargo test -p llm --tests` + existing `t1915_*` tests in `redact.rs` — _accept: all PASS byte-identical pre-Layer / post-Layer_
- [ ] T-RED-FINAL.2 — Synthetic LLM-traffic observation: run cockpit_live OR agent under `REDACT_LAYER_MODE=warn` with a fake LLM provider stub that includes a `sk-ant-...` in a structured log field — _accept: audit ledger entry shows redacted value (`sk-ant-***...`); meta-event with field name + rule appears at `Level::WARN`; raw key NOT present anywhere in ledger or stdout_
- [ ] T-RED-FINAL.3 — Gate-mode observation: same fake traffic with `REDACT_LAYER_MODE=gate` — _accept: redaction performed; no meta-event in ledger (unless `REDACT_LAYER_VERBOSE=1` flips to verbose)_
- [ ] T-RED-FINAL.4 — Verify R-NR contract — _accept: R-NR.1-7 each verified; existing `t1915_*` tests PASS; `verify_anchors.sh` 75/75; LLM outbound call smoke test succeeds (R-NR.5)_
- [ ] T-RED-FINAL.5 — Write test-final report — _accept: `spec/v2-1-tracing-layer-redactor/reports/test-final-2026-MM-DD-v2-1-tracing-layer-redactor.md` per [template](../../.claude/skills/rust-test/templates/test-report.md); VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter

- [ ] T-RED-P1 — Deck `spec/v2-1-tracing-layer-redactor/presentations/v2-1-tracing-layer-redactor-<date>.md` — _accept: cross-cutting safety duo framing recap; rule set table + WARN-mode meta-event example; provider-header wire-layer exemption confirmation; 2-week WARN observation contract with explicit v0.2.0 promotion-to-gate plan; operator-decide-ready_

## Notes

- **Anchor contract**: 75/75 byte-identical pre/post. The Layer
  affects only `tracing` emit; backtest reports under
  `spec/*/reports/` are unaffected. Same shape as ADR-0048 D6
  anchor-additivity contract.
- **Bundle ownership**: this feature is the EXPENSIVE pillar of
  Pick B Wave 1 (~1.5 dev days; the sibling `ui-contrast-asserter`
  is ~0.5 dev days). PARALLEL-SAFE with the sibling per the
  bundle direction § Sequencing.
- **WARN observation contract**: v0.1.0 ships in WARN mode by
  default per the bundle Q-DUO-WARN ratification. After 2 weeks
  of observation, operator promotes default to gate via a
  v0.2.0 patch. False-positive count + true-positive count
  recorded in the v0.2.0 brief.
- **Future provider integrations**: every new LLM provider
  (Gemini, local LLama, etc.) inherits redaction at process init
  with ZERO per-provider wiring. Provider header bypass is at the
  wire layer per Q-RED-2; no allowlist update needed in the
  redactor for new providers.
