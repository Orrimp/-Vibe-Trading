---
slug: ui-contrast-asserter
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — ui-contrast-asserter v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick B Wave 1 promotion in
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md).
> ~0.5 dev day + ~0.25 tester day total. Bias DURABLE per
> [AGENT.md § Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-CONT-M0.1 — Feature brief authored — _accept: feature.md R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-CONT-1/2/3 + pre-drawn 4-cell verdict tree_
- [x] T-CONT-M0.2 — Bundle direction dev-note authored — _accept: `spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md` ships with bundle framing + Q-DUO-WARN_
- [x] T-CONT-M0.3 — Backlog Active row appended under § Process / tooling — _accept: PROMOTED Queue → Active 2026-05-29 annotation_
- [x] T-CONT-M0.4 — Trace row `REQ-UI-CONTRAST-ASSERTER-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect

- [ ] T-CONT-T1.1 — Ratify Q-CONT-1 (WARN-mode default) + Q-CONT-2 (formula impl: hand-rolled vs crate) + Q-CONT-3 (opt-out marker placement) per § Operator decisions — _accept: § Design records each Q ratified or operator-decided; fast-skip if all DURABLE; Q-CONT-1 inherits from Q-DUO-WARN bundle ratification_
- [ ] T-CONT-T1.2 — Pick test file path + struct shape — _accept: `crates/ui/tests/contrast.rs` (or M-T1-ratified path); `ContrastPair { pair_id, fg, bg, class }` struct + `ContrastClass { Body, Equity, OptOut(&str) }` enum_
- [ ] T-CONT-T1.3 — Run one-pass theme.rs audit — _accept: enumerate `(fg, bg)` token pairs via theme.rs; compute contrast per pair via hand-rolled or crate formula; seed initial opt-out list (≤ 3 per H2); ratify MIN_PAIRS floor (≥ 30 per R1.4)_
- [ ] T-CONT-T1.4 — Confirm ADR-0048 carries forward — _accept: § Design D-CONT-N spec'd; one Changelog row drafted for architect-commit; no new ADR_
- [ ] T-CONT-T1.5 — Wave decomposition — _accept: single M-DEV wave; ~0.5 dev day; D-CONT bullets cover pair table + WCAG formula + opt-out table + WARN/gate mode + 4-5 reference vector tests_
- [ ] T-CONT-T1.6 — Falsification probe P-CONT-1 (R1.4 enumeration floor) spec'd — _accept: § Design includes the comment-out-a-token + cargo test recipe; expected: floor-violation panic message_
- [ ] T-CONT-T1.7 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _accept: feature.md + tasks.md frontmatter updated_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

- [ ] T-CONT-D1 — Author `crates/ui/tests/contrast.rs` per § Design D-CONT-1+2 — _accept: file exists; defines `ContrastPair` struct + `ContrastClass` enum + `const PAIRS: &[ContrastPair]` table (per M-T1 audit) + `const OPT_OUTS: &[(&str, &str)]` table (per M-T1 seed)_
- [ ] T-CONT-D2 — Implement hand-rolled WCAG 2.1 formula per Q-CONT-2 (a) ratification — _accept: `fn contrast_ratio(fg: Rgb, bg: Rgb) -> f64` returns ratio in `[1.0, 21.0]`; reference vector tests for `#FFFFFF on #000000 = 21:1`, `#777 on #FFF ≈ 4.48:1`, `#000 on #FFF = 21:1`, and 1-2 more PASS_
- [ ] T-CONT-D3 — Implement pair iteration + per-class assertion per R2.3 — _accept: iterate `PAIRS` table; `Body` class asserts ≥ 4.5; `Equity` class asserts ≥ 7.0; `OptOut(reason)` skipped with audit log_
- [ ] T-CONT-D4 — Implement WARN-mode + gate-mode per R3 + Q-CONT-3 ratification — _accept: env var `UI_CONTRAST_MODE` parsed; default `warn`; WARN failures emit `eprintln!` + test PASS; gate failures panic_
- [ ] T-CONT-D5 — Implement MIN_PAIRS floor per R1.4 — _accept: assertion `PAIRS.len() >= MIN_PAIRS` PASS at v0.1.0 (per M-T1 audit count); custom panic message "theme token enumeration detected < {MIN_PAIRS} pairs; refactor likely broke enumeration"_
- [ ] T-CONT-D6 — Run falsification probe P-CONT-1 — _accept: comment out 5+ token pairs from `PAIRS` table; `cargo test -p ui --test contrast` panics with the MIN_PAIRS floor violation; revert and ship_
- [ ] T-CONT-D7 — Verify R-NR contract: zero production code touched, visual-snapshot tests PASS byte-identical, no new runtime deps — _accept: `git diff -- crates/ui/src/` empty; `cargo test -p ui --test visual_snapshots` PASS; no `[dependencies]` add (only `[dev-dependencies]` if any)_
- [ ] T-CONT-D8 — Update `spec/trace.toml` `REQ-UI-CONTRAST-ASSERTER-001` row — _accept: `crates` populated (`["crates/ui"]`); `tests` lists test fn names; `state` = `dev-done`_
- [ ] T-CONT-D9 — Dev-side gates — _accept: `cargo fmt -p ui -- --check` clean; `cargo test -p ui --test contrast` PASS (WARN default OR gate with all PASS); `bash scripts/verify_anchors.sh` → 75/75 PASS byte-identical_

## M-FINAL — Tester

- [ ] T-CONT-FINAL.1 — Run `cargo test -p ui --test contrast` with default `UI_CONTRAST_MODE=warn` — _accept: test PASS; cargo test stderr output shows any WARN-mode failure logs (0 expected per H2)_
- [ ] T-CONT-FINAL.2 — Run with `UI_CONTRAST_MODE=gate` — _accept: test PASS (no opt-outs triggered or all opt-outs reason-stringed); WARN-mode logs absent in gate mode_
- [ ] T-CONT-FINAL.3 — Verify the floor assertion fires correctly: temporarily comment out 5 pairs from the `PAIRS` table; rerun test — _accept: panic with MIN_PAIRS floor violation; revert_
- [ ] T-CONT-FINAL.4 — Verify visual-snapshot tests unaffected — _accept: `cargo test -p ui --test visual_snapshots --no-default-features --features live` PASS byte-identical_
- [ ] T-CONT-FINAL.5 — Verify R-NR contract — _accept: 75/75 anchors PASS via `verify_anchors.sh`; zero `git diff -- crates/ui/src/` (no production code touched); no new runtime deps_
- [ ] T-CONT-FINAL.6 — Write test-final report — _accept: `spec/ui-contrast-asserter/reports/test-final-2026-MM-DD-ui-contrast-asserter.md` per [template](../../.claude/skills/rust-test/templates/test-report.md); VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter

- [ ] T-CONT-P1 — Deck `spec/ui-contrast-asserter/presentations/ui-contrast-asserter-<date>.md` — _accept: cross-cutting safety duo framing recap; pair-count + opt-out-count table from M-T1 audit; sample WARN-mode failure log (if any); 2-week WARN observation contract with explicit v0.2.0 promotion-to-gate plan; sibling redactor cross-link; operator-decide-ready_

## Notes

- **Anchor contract**: 75/75 byte-identical pre/post. Test
  infrastructure addition only; zero production code touched.
  Same shape as ADR-0048 D6 anchor-additivity contract.
- **Bundle ownership**: this feature is the CHEAP pillar of Pick B
  Wave 1 (~0.5 dev days; the sibling
  `v2-1-tracing-layer-redactor` is ~1.5 dev days). PARALLEL-SAFE
  with the sibling per the bundle direction § Sequencing.
- **WARN observation contract**: v0.1.0 ships in WARN mode by
  default per the bundle Q-DUO-WARN ratification. After 2 weeks
  of observation, operator promotes default to gate via a v0.2.0
  patch. WARN-mode failure count + opt-out list growth recorded
  in the v0.2.0 brief.
- **New tokens auto-asserted**: future lumen Phase X+ tokens added
  to `crates/ui/src/theme.rs` AND included in the `PAIRS` table
  inherit contrast assertion. The M-T1 audit + dev wire-up is the
  ONE-TIME cost; per-token wiring is ZERO ongoing.
- **Per-cycle benefit (Rank 4 in process-tooling-survey)**: MEDIUM
  — closes the palette-refactor regression class without rendering
  a pixel. Best-cheap-pick framing per the survey.
