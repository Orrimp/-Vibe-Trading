---
slug: v2-1-tracing-layer-redactor
status: arch-done
owner: developer
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

- [ ] T-RED-D1 — Add `regex = "1"` workspace dep — _accept: root `Cargo.toml [workspace.dependencies]` gains `regex = "1"`; `crates/llm/Cargo.toml [dependencies]` gains `regex = { workspace = true }`; `cargo check -p llm` succeeds; library checklist 6/6 PASS per D-RED § Library compatibility_
- [ ] T-RED-D2 — Author `crates/llm/src/redact_layer.rs` per D-RED-1 — _accept: file exists; exports `pub fn redact_tracing_layer<S>() -> RedactLayer`, `RedactLayer` struct, `RedactMode` enum, `pub fn redact_str(s: &str) -> Cow<str>` test seam, `impl<S: Subscriber> Layer<S> for RedactLayer`; uses existing `crate::redact::redact()` verbatim for sanitisation (R-NR.1)_
- [ ] T-RED-D3 — Implement closed regex rule set per D-RED-1 ratification table — _accept: 9 rules (`anthropic_key` / `openai_proj_key` / `openai_key` / `bearer_token` / `jwt` / `aws_access` / `aws_secret_context` / `password_field_name` / `entropy_fallback`) wired in a `const REDACT_RULES: &[Rule]` (or `std::sync::OnceLock<RegexSet>` per D-RED architect note); rule evaluation order matches table order (first-match-wins so `sk-ant-` precedes `sk-`); unit tests per rule PASS_
- [ ] T-RED-D4 — Implement emit-redacted + filter-original Layer per D-RED-3 (b) — _accept: `on_event` runs `RedactingVisitor`, re-emits event with redacted fields + `__redact_layer_emitted` span-extension marker; downstream filter (same Layer or sibling per developer pick) drops the original event; meta-event recursion guard via `event.metadata().target() == "redact_layer"` short-circuit_
- [ ] T-RED-D5 — Implement WARN-mode meta-event side channel per D-RED-5 — _accept: with `REDACT_LAYER_MODE=warn`, each redaction emits `tracing::warn!(target: "redact_layer", field_name = ..., rule = ..., count_so_far = ...)`; per-process `AtomicU32` monotonic counter; secret VALUE never in meta-event payload; integration test in `crates/llm/tests/redact_layer.rs` asserts WARN event captured by test sink_
- [ ] T-RED-D6 — Implement gate-mode behaviour per D-RED-5 — _accept: with `REDACT_LAYER_MODE=gate` (no `REDACT_LAYER_VERBOSE`), redaction performed + meta-event SUPPRESSED; integration test asserts redacted event only. With `REDACT_LAYER_VERBOSE=1`, redacted event + meta-event both present_
- [ ] T-RED-D7 — Implement marker-field opt-out per D-RED-4 — _accept: `__redact_skip = "field_name"` + `__redact_reason = "..."` together pass the named field through unredacted; marker fields stripped from the re-emitted event; missing `reason` → no skip + `tracing::warn!` meta-event records the missing-reason; no static `BYPASS_FIELDS` const_
- [ ] T-RED-D8 — Implement env-var mode parse per Q-RED-3 — _accept: `REDACT_LAYER_MODE` parse in `RedactLayer::from_env()` at first install; default = `warn`; invalid value → one-time `tracing::warn!` + default to `warn` (fail-safe-closed)_
- [ ] T-RED-D9 — Author `crates/llm/src/tracing_init.rs` per D-RED-2 — _accept: `pub fn install_global(extra_directives: &[&str], json: bool) -> Result<(), TryInitError>` composes `EnvFilter::from_default_env()` + redactor Layer + `fmt::Layer` in registry chain; called by every binary instead of `tracing_subscriber::fmt().init()`_
- [ ] T-RED-D10 — Wire P0 binaries (`agent` + `cockpit_live`) to `llm::tracing_init::install_global` per D-RED-8 — _accept: `crates/agent/src/main.rs:54-61` block replaced with one-line `llm::tracing_init::install_global(&["trading=info", "agent=info"], true)?;`; same for `crates/ui/src/bin/cockpit_live.rs:236-248`; JSON output byte-identical to pre-migration per K-arch-3 mitigation; smoke test confirms LLM outbound call succeeds (R-NR.5)_
- [ ] T-RED-D11 — Wire P1 binaries per D-RED-8 — _accept: `cockpit`, `backtest`, `llm-smoke`, `generate-replay-fixture`, `llm_verdict` (5 binaries) migrated to `install_global`; cargo check + cargo test -p llm pass_
- [ ] T-RED-D12 — Wire P2 binaries per D-RED-8 — _accept: 10 forecast/data/aux binaries (`train_patchtst` / `forecast_distribution` / `train_garch` / `vol_verdict` / `recalibrate_sigma_train` / `regime_verdict` / `train_tcn` / `sharpe_comparison` / `threshold_sweep` / `run_yahoo_sma` / `fetch_binance_klines` / `fetch_yahoo_klines`) migrated to `install_global`; non-LLM-bearing so redaction is a no-op but hygiene durable_
- [ ] T-RED-D13 — Author pure-fn parity self-test per R4.4 — _accept: `crates/llm/src/redact_layer.rs #[cfg(test)] mod tests` includes a `t1915_parity` test routing every `t1915_*` input from `redact.rs` through `redact_str()` and asserting byte-identical output to `redact::redact()`; test PASS_
- [ ] T-RED-D14 — Author integration test `crates/llm/tests/redact_layer.rs` — _accept: installs `RedactLayer` + test sink Layer (mirroring `mark_unavailable_warns_capture.rs:178` pattern); test cases for (i) each of 9 rules positive + negative, (ii) WARN-mode meta-event, (iii) gate-mode no meta-event, (iv) gate + verbose, (v) marker-field bypass, (vi) marker-field missing reason → meta-event_
- [ ] T-RED-D15 — Run falsification probe P-RED-1 per D-RED-9 — _accept: un-ignore `p_red_1_layer_load_bearing` test; comment out `on_event` field-rewrite; cargo test FAILs with "expected redaction not applied"; revert + cargo test PASSes; ship as `#[ignore]`_
- [ ] T-RED-D16 — Run falsification probe P-RED-2 per D-RED-9 — _accept: un-ignore `p_red_2_layer_ordering`; swap `redact_tracing_layer()` after `fmt_layer` in `install_global`; cargo test FAILs (fmt captures raw); revert + cargo test PASSes; ship as `#[ignore]`_
- [ ] T-RED-D17 — Run falsification probe P-RED-3 per D-RED-9 (NEW architect-add) — _accept: un-ignore `p_red_3_rule_set_load_bearing`; replace `REDACT_RULES` with empty slice; all 9 positive-rule tests FAIL; revert + cargo test PASSes; ship as `#[ignore]`_
- [ ] T-RED-D18 — Update `spec/trace.toml` `REQ-V2-1-TRACING-LAYER-REDACTOR-001` row — _accept: `crates` populated with `crates/llm/src/redact_layer.rs`, `crates/llm/src/tracing_init.rs`, plus the 17 binary file paths; `tests` lists test fn names from D13 + D14 + D15 + D16 + D17; `state` = `dev-done`_
- [ ] T-RED-D19 — Dev-side gates — _accept: `cargo fmt --all -- --check` clean; `cargo clippy --workspace -- -D warnings` clean; `cargo test --workspace` PASS (excluding the 3 `#[ignore]`'d probe tests); `bash scripts/verify_anchors.sh` → 75/75 PASS byte-identical pre/post (D-RED-8 anchor impact zero per K-arch-3)_
- [ ] T-RED-D20 — ADR-0019 § Changelog ride-along per D-RED-7 — _accept: `spec/architecture/adr/0019-v2-llm-strategy.md` § Changelog gains one row "2026-05-29 (architect): v2.1 tracing-Layer redactor M-T1 ratified ..."; `spec/architecture/adr/README.md` frontmatter `updated:` field bumped same-commit per atomic registry contract (NOTE: this row already authored by architect M-T1; developer no-op unless un-committed)_

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
