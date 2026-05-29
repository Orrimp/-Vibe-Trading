---
slug: lab-recipe-test-harness-v0.2.0-cross-surface-extension
status: in-progress
owner: analyst
updated: 2026-05-29
---

# Tasks — lab-recipe-test-harness v0.2.0 cross-surface extension

## M0 — Analyst (DONE 2026-05-29)

- [x] T-M0.1 — Recipe inventory at R1 (9 surfaces enumerated) — _accept: each Recipe has file:line + regression class + coverage status_
- [x] T-M0.2 — feature.md R1-R4 + R-NR + K1-K4 + H1-H4 + Q1-Q2 + 4-cell verdict tree — _accept: <200 lines, durable contract framing_
- [x] T-M0.3 — backlog Active row appended under § Process / tooling — _accept: PROMOTED Idea → Active 2026-05-29 annotation_
- [x] T-M0.4 — trace row `REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (ratify Q1+Q2; lock R3 mock pattern; ADR-0048 amendment iff needed)

- [ ] T-T1.1 — ratify Q1 + Q2 (default: Q1=(a), Q2=(a) durable-recommended; record overrides if operator chose (b)) — _accept: M-OD outcome locked into § Design_
- [ ] T-T1.2 — lock R3 mock-pattern decision: single shared trait family vs per-Recipe (analyst default: per-Recipe) — _accept: § Design records choice + rationale_
- [ ] T-T1.3 — decompose M-DEV into per-Recipe waves (one wave per Recipe — Wave A=TrainingLog, B=TrainingPoller, C=ToastDismiss, D=ActivityAuditAggregator, E=Surface-2 ServerTime + TrailMirror, F=Surface-1 ActivityRecipe) — _accept: 4-6 dev waves, each ≤ ~80 LoC + falsification probe_
- [ ] T-T1.4 — ADR-0048 § Changelog amendment iff R3 changed the contract (likely NO — pattern carries forward verbatim) — _accept: ADR-0048 either untouched or one-line Changelog row only_

## M-DEV — Developer (per-Recipe waves, T-D1.x → T-D6.x sequential)

- [ ] T-D1 — Wave A: `TrainingLogRecipe` boundary test (Surface 1) — _accept: new `crates/ui/tests/training_log_recipe_stream.rs`; ≥ 3 tests; per-Recipe T-T4 probe stub in module docstring_
- [ ] T-D2 — Wave A: `TrainingLogRecipe` gating test (Surface 2) — _accept: new `crates/ui/tests/training_log_inflight_gating.rs` (or extend `cockpit_training_pressed_wiring.rs` if architect prefers); lifecycle assertion on `training_inflight` predicate_
- [ ] T-D3 — Wave B: `TrainingPoller` boundary + gating tests — _accept: new file(s); MockAuditLedger uses existing `Ledger::in_memory()`; per-Recipe T-T4 probe stub_
- [ ] T-D4 — Wave C: `ToastDismissRecipe` boundary + gating tests — _accept: new file(s); `tokio::time::pause()` + `advance()` drives interval deterministically_
- [ ] T-D5 — Wave D: `ActivityAuditAggregator` `tokio::select!`-arm-survival boundary test — _accept: new test file in `crates/agent/tests/`; asserts channel still receives ticks AFTER N interval boundaries_
- [ ] T-D6 — Waves E + F: ServerTime S2 + TrailMirror S2 + Activity S1 — _accept: per-Recipe test files; all per-Recipe T-T4 probes documented_

## M-FINAL — Tester (per-Recipe T-T4 falsification table)

- [ ] T-T-FINAL — run all new tests + falsification probes; emit per-Recipe FAIL → restore → PASS table; verify anchors 71/71 byte-identical pre/post; verify v0.1.0 harness tests stay PASS — _accept: test-final-2026-MM-DD-<slug>.md with per-Recipe T-T4 evidence; VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter (operator review deck)

- [ ] T-P1 — deck `presentations/<slug>-<date>.md`: per-Recipe T-T4 evidence; durable-coverage outcome statement; v0.3.0 backlog row planted iff Q1=(b) chosen — _accept: operator-decide-ready deck_

## Notes

- **Anchor contract**: 71/71 byte-identical pre/post. Zero file
  output from any new test. Same as v0.1.0 D6.
- **Falsification stub per Recipe**: each new test file MUST include
  a module docstring section "T-T4 falsification probe" that names the
  exact source line to comment out + the expected FAIL assertion.
  This is the v0.1.0 lesson made durable per Q2=(a).
- **Wave parallelism**: Waves A-D are independent (different Recipes,
  different test files). Waves E-F can run alongside any of A-D.
  Architect may schedule them concurrently if dev-bench available.
- **Cargo build budget**: per K4, total new test LoC ≤ 800 (8 surfaces
  × ~100 LoC each); cargo test wall-clock budget per Recipe ≤ 1.5 s
  per ADR-0048 D4.
