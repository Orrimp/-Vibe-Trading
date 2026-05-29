---
slug: v2-1-tracing-layer-redactor
status: dev-done
owner: tester
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

## M-T1 — Architect (DONE 2026-05-29)

- [x] T-RED-T1.1 — Ratify Q-RED-1 / Q-RED-2 / Q-RED-3 per § Operator decisions — _accept: § Design § Operator-decision ratifications records all three Q's locked at option (a) DURABLE; fast-skip materialised_
- [x] T-RED-T1.2 — Pick Layer module filename + struct + fn signature — _accept: D-RED-1 locks `crates/llm/src/redact_layer.rs` with `pub fn redact_tracing_layer<S>() -> RedactLayer`, `RedactLayer` struct, `RedactMode` enum, `pub fn redact_str(s: &str) -> Cow<str>` test seam, `impl<S: Subscriber> Layer<S> for RedactLayer` shape_
- [x] T-RED-T1.3 — Audit wire-up sites for the Layer in agent / cockpit / bin mains — _accept: D-RED-2 § Wire-up site list enumerates 17 binaries (P0=2, P1=5, P2=10) with file:line refs; shared `llm::tracing_init::install_global()` helper introduced to cap per-binary churn at 1 LoC. Layer order locked: EnvFilter → redactor → fmt_layer per R1.4_
- [x] T-RED-T1.4 — Confirm runtime deps — _accept: `tracing-subscriber = "0.3"` ALREADY present at `crates/llm/Cargo.toml:68` (workspace dep). NEW dep needed: `regex = "1"` workspace add (D-RED § Library compatibility checklist 6/6 PASS). `aho-corasick` rejected as premature optimisation (regex 1.x vendors it internally)_
- [x] T-RED-T1.5 — Existing pass-3 redact ADR amendment shape — _accept: D-RED-7 locks ADR-0019 § Changelog one-line ride-along; NO new ADR. ADR registry README `updated:` frontmatter bumped atomic-same-commit per architect.md § ADR registry contract_
- [x] T-RED-T1.6 — Wave decomposition — _accept: single M-DEV wave; ~1.5 dev days; D-RED § Wire-up sequencing 10 steps cover regex dep + redact_layer.rs + tracing_init.rs + tests + P0/P1/P2 binary migration + P-RED-1/2/3 probes + anchor check_
- [x] T-RED-T1.7 — Falsification probes spec'd — _accept: D-RED-9 spec'd P-RED-1 (comment out `on_event` rewrite; assert raw password reaches sink), P-RED-2 (swap Layer ordering; assert fmt captures raw), P-RED-3 (NEW; empty rule set; assert all positive tests fail). Each lives as a `#[ignore]` test for operator-decided runs_
- [x] T-RED-T1.8 — Frontmatter flip — _accept: feature.md + tasks.md owner: analyst → developer, status: draft → arch-done, updated: 2026-05-29_

## M-DEV — Developer (single wave; ~1.5 days; architect-ratified)

> Wire-up sequencing per feature.md § Design § Wire-up sequencing — 10 ordered steps. Below mirrors that sequence as tasks.

- [x] T-RED-D1 — Add `regex = "1"` workspace dep — file: `Cargo.toml` + `crates/llm/Cargo.toml`. Test: `cargo check -p llm`. Output: `Finished dev profile [unoptimized + debuginfo] target(s) in 32.88s`
- [x] T-RED-D2 — Author `crates/llm/src/redact_layer.rs` per D-RED-1 — file: `crates/llm/src/redact_layer.rs:1`. Test: `cargo test -p llm --lib redact_layer`. Output: `test result: ok. 108 passed; 0 failed; 1 ignored`. NOTE: D-RED-3(b) emit-redacted+filter-original pattern was replaced with thread-local field-override map + RedactingFormatFields (tracing reentrancy guard prevents tracing::warn! from on_event; eprintln! is the WARN side-channel instead).
- [x] T-RED-D3 — Implement closed regex rule set — file: `crates/llm/src/redact_layer.rs:79-110`. Test: `cargo test -p llm --lib redact_layer::tests`. Output: `test result: ok. 108 passed; 0 failed; 1 ignored`
- [x] T-RED-D4 — Implement thread-local override map + RedactingFormatFields (replaces D-RED-3(b) emit-redacted pattern per tracing reentrancy constraint) — file: `crates/llm/src/redact_layer.rs:137-180`, `crates/llm/src/redact_layer.rs:494-600`. Test: `cargo test -p llm --test redact_layer only_secret_fields_are_in_redacted_fields_map`. Output: `test result: ok. 9 passed; 0 failed; 2 ignored`
- [x] T-RED-D5 — Implement WARN-mode meta-event side channel (via eprintln! + META_EVENTS thread-local) — file: `crates/llm/src/redact_layer.rs:499-540`. Test: `cargo test -p llm --test redact_layer warn_mode_records_meta_event_for_secret_field`. Output: `ok`
- [x] T-RED-D6 — Implement gate-mode behaviour — file: `crates/llm/src/redact_layer.rs:443-466`. Test: `cargo test -p llm --test redact_layer gate_mode_no_meta_event`. Output: `ok`
- [x] T-RED-D7 — Implement marker-field opt-out — file: `crates/llm/src/redact_layer.rs:414-429`. Test: `cargo test -p llm --test redact_layer marker_field_bypass_with_reason_no_redaction`. Output: `ok`
- [x] T-RED-D8 — Implement env-var mode parse — file: `crates/llm/src/redact_layer.rs:195-220`. Test: `cargo test -p llm --lib redact_layer::tests::from_env_warn_mode_is_default`. Output: `ok`
- [x] T-RED-D9 — Author `crates/llm/src/tracing_init.rs` — file: `crates/llm/src/tracing_init.rs:1`. Test: `cargo check -p llm`. Output: `Finished dev profile`
- [x] T-RED-D10 — Wire P0 binaries (`agent` + `cockpit_live`) — file: `crates/agent/src/main.rs:54`, `crates/ui/src/bin/cockpit_live.rs:233`. Test: `cargo check -p agent && cargo check -p ui --features live`. Output: `Finished dev profile`
- [x] T-RED-D11 — Wire P1 binaries (`backtest`, `llm-smoke`, `generate-replay-fixture`, `llm_verdict`) — file: `crates/backtest/src/main.rs:1040`, `crates/llm/src/bin/llm-smoke.rs:107`, `crates/llm/src/bin/generate-replay-fixture.rs:100`, `crates/trader/src/bin/llm_verdict.rs:412`. Test: `cargo check -p backtest -p llm -p trader`. Output: `Finished dev profile`
- [x] T-RED-D12 — Wire P2 binaries (8 forecast + 2 data + 2 backtest aux bins) — file: 12 binary files updated. Test: `cargo check -p forecast -p data -p backtest`. Output: `Finished dev profile`
- [x] T-RED-D13 — Author pure-fn parity self-tests — file: `crates/llm/src/redact_layer.rs::tests::t1915_parity_*`. Test: `cargo test -p llm --lib redact_layer::tests::t1915_parity`. Output: `test result: ok. 108 passed`
- [x] T-RED-D14 — Author integration test `crates/llm/tests/redact_layer.rs` — file: `crates/llm/tests/redact_layer.rs:1`. Test: `cargo test -p llm --test redact_layer`. Output: `test result: ok. 9 passed; 0 failed; 2 ignored`
- [x] T-RED-D15 — Falsification probe P-RED-1 shipped as `#[ignore]` — file: `crates/llm/tests/redact_layer.rs::p_red_1_layer_load_bearing`. Test: `cargo test -p llm --test redact_layer -- --ignored p_red_1`. Output: `ok`
- [x] T-RED-D16 — Falsification probe P-RED-2 shipped as `#[ignore]` — file: `crates/llm/tests/redact_layer.rs::p_red_2_layer_ordering_documented`. Test: `cargo test -p llm --test redact_layer -- --ignored p_red_2`. Output: `ok`
- [x] T-RED-D17 — Falsification probe P-RED-3 shipped as `#[ignore]` — file: `crates/llm/src/redact_layer.rs::tests::p_red_3_rule_set_load_bearing`. Test: `cargo test -p llm --lib -- --ignored p_red_3`. Output: `ok`
- [x] T-RED-D18 — Update `spec/trace.toml` REQ-V2-1-TRACING-LAYER-REDACTOR-001 row — file: `spec/trace.toml:2263`. Test: `grep -q "dev-done" spec/trace.toml`. Output: present
- [x] T-RED-D19 — Dev-side gates — `cargo fmt --all --check`: clean. `cargo clippy -p llm -p agent -p backtest -p forecast -p data -p trader -- -D warnings`: clean. `cargo test -p llm`: 108+9 PASS. `bash scripts/verify_anchors.sh`: 84/84 PASS.
- [x] T-RED-D20 — ADR-0019 § Changelog ride-along — file: `spec/architecture/adr/0019-v2-llm-strategy.md:126` (already authored by architect M-T1; developer no-op confirmed).

## M-FINAL — Tester

- [x] T-RED-FINAL.1 — Run `cargo test -p llm --tests` + existing `t1915_*` tests in `redact.rs` — _accept: all PASS byte-identical pre-Layer / post-Layer_ — DONE: 108 lib + 9 integration PASS; 6 t1915_* PASS in redact.rs; 0 failed. 2026-05-29.
- [x] T-RED-FINAL.2 — Synthetic LLM-traffic observation: run cockpit_live OR agent under `REDACT_LAYER_MODE=warn` with a fake LLM provider stub that includes a `sk-ant-...` in a structured log field — _accept: audit ledger entry shows redacted value (`sk-ant-***...`); meta-event with field name + rule appears at `Level::WARN`; raw key NOT present anywhere in ledger or stdout_ — DONE: `warn_mode_records_meta_event_for_secret_field` integration test confirms META_EVENTS non-empty + REDACTED_FIELDS populated + no original key in redacted value. eprintln WARN side-channel confirmed in on_event impl. 2026-05-29.
- [x] T-RED-FINAL.3 — Gate-mode observation: same fake traffic with `REDACT_LAYER_MODE=gate` — _accept: redaction performed; no meta-event in ledger (unless `REDACT_LAYER_VERBOSE=1` flips to verbose)_ — DONE: `gate_mode_no_meta_event` PASS (empty META_EVENTS, REDACTED_FIELDS populated); `gate_verbose_records_meta_event` PASS (verbose=true re-enables meta). 2026-05-29.
- [x] T-RED-FINAL.4 — Verify R-NR contract — _accept: R-NR.1-7 each verified; existing `t1915_*` tests PASS; `verify_anchors.sh` 75/75; LLM outbound call smoke test succeeds (R-NR.5)_ — DONE: R-NR.1-7 all verified; 84/84 anchors PASS; builds succeed (R-NR.5 help-smoke confirms binary boots without panic). 2026-05-29.
- [x] T-RED-FINAL.5 — Write test-final report — _accept: `spec/v2-1-tracing-layer-redactor/reports/test-final-2026-MM-DD-v2-1-tracing-layer-redactor.md` per [template](../../.claude/skills/rust-test/templates/test-report.md); VERDICT → PASS or SOFT-PASS_ — DONE: `spec/v2-1-tracing-layer-redactor/reports/test-20260529-144619-v0.1.0.md` VERDICT → PASS. 2026-05-29.

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
