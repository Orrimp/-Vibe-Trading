---
slug: ui-contrast-asserter
status: arch-done
owner: developer
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

## M-T1 — Architect (DONE 2026-05-29)

- [x] T-CONT-T1.1 — Ratified Q-CONT-1 (a) WARN-mode default + Q-CONT-2 (a) hand-rolled + Q-CONT-3 (a) in-file OPT_OUTS — _accept: § Design § Operator-decide ratifications records all three on Recommended DURABLE path; fast-skip path taken_
- [x] T-CONT-T1.2 — Test file path + struct shape locked — _accept: `crates/ui/tests/contrast.rs` (NEW) per D-CONT-1; `ContrastPair { pair_id, fg, bg, class }` + `ContrastClass { Body, Equity, OptOut(&'static str) }` enum + `OptOutEntry { pair_id, reason }`_
- [x] T-CONT-T1.3 — One-pass theme.rs audit COMPLETED — _accept: 30 ModeColor constants enumerated (theme.rs:128-404); 83 PAIRS table entries seeded; 9 opt-outs (8 `FG_4` disabled-tier + 1 `BORDER_STRONG` decorative); MIN_PAIRS = 60 floor ratified (23-pair safety margin). H2 INVALIDATED (9 opt-outs vs ≤ 3 estimate) — bounded + reviewable, not a route-back. K1 PARTIALLY TRIGGERED. 2 GENUINE sub-AA defects surfaced: `FG_ON_ACCENT on ACCENT_light = 3.52`, `WARN_500 on CANVAS_light = 2.96`. WARN-mode design contains the signal without blocking CI._
- [x] T-CONT-T1.4 — ADR-0048 carry-forward confirmed — _accept: § D-CONT-7 records no new ADR; ADR-0048 § Changelog + README registry table updated atomically per architect.md § ADR registry atomic-write contract_
- [x] T-CONT-T1.5 — Wave decomposition: single M-DEV wave ~0.5 dev day — _accept: D-CONT-1..D-CONT-7 + D-CONT-1 ref-vector tests cover the developer scope. T-CONT-D1..D9 already enumerated below; no expansion needed._
- [x] T-CONT-T1.6 — Falsification probe P-CONT-1 spec'd — _accept: § Design § Falsification probe P-CONT-1 includes two-recipe variant (deliberately-low-contrast pair WARN-or-gate + MIN_PAIRS floor 25-comment-out trigger); each variant has expected stderr + exit-code expectations_
- [x] T-CONT-T1.7 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer (this commit)_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

**Architect M-T1 notes for developer**:
- D-CONT-1 / D-CONT-2 = the struct + formula sketch — developer
  implements verbatim. Reference vector tests are individual `#[test]
  fn ref_vector_*` per D-CONT-1 (4 vectors).
- D-CONT-3 = 83 PAIRS entries (architect's audit table); developer
  may build them as `const PAIRS: &[ContrastPair]` literal OR build
  via small helper-fns at module-init — architect leaves the
  ergonomic choice to dev BUT requires all 83 entries land literal in
  source (no runtime computation) so a code review can read the table.
- D-CONT-4 = OPT_OUTS list of 9 entries (verbatim in D-CONT-4 code
  block) PLUS keep the chart-line pairs (UP_400, DOWN_400, ACCENT_2..5)
  in PAIRS with `class: ContrastClass::OptOut("chart-line-stroke-not-text")`
  or `…("chart-comparison-stroke-not-text")` per audit table.
- D-CONT-5 = MIN_PAIRS = 60 (explicit named const).
- D-CONT-6 = env var `UI_CONTRAST_MODE`; default `warn`.
- All 5-6 WARN-logging pairs from architect dry-run (light-mode
  WARN_500/UP_500/DOWN_500/FG_ON_ACCENT, dark-mode FG_3 on PANEL_RAISED)
  should EMIT `eprintln!` at v0.1.0 — that's the asserter doing its
  job. Do NOT silence them via opt-out at v0.1.0.

- [ ] T-CONT-D1 — Author `crates/ui/tests/contrast.rs` per § Design D-CONT-1 + D-CONT-3 + D-CONT-4 — _accept: file exists; defines `ContrastPair`, `ContrastClass`, `OptOutEntry` per D-CONT-1; `const PAIRS: &[ContrastPair]` with all 83 entries per D-CONT-3 audit table; `const OPT_OUTS: &[OptOutEntry]` with 9 entries per D-CONT-4_
- [ ] T-CONT-D2 — Implement hand-rolled WCAG 2.1 formula per D-CONT-2 — _accept: `linearize` + `relative_luminance` + `contrast_ratio` pure fns; 4 reference-vector unit tests (`WHITE on BLACK = 21.00`, `BLACK on WHITE = 21.00`, `#777 on #FFF = 4.48`, `#888 on #000 = 5.92`) each `#[test]` PASSing with `< 0.01` f64 tolerance_
- [ ] T-CONT-D3 — Implement pair iteration + per-class assertion per R2.3 — _accept: iterate `PAIRS` table; `Body` class asserts ≥ 4.5; `Equity` class asserts ≥ 7.0; `OptOut(reason)` always logs audit line `eprintln!("opt-out: {pair_id}; reason: {reason}; ratio: {ratio:.2}")` regardless of mode_
- [ ] T-CONT-D4 — Implement WARN-mode + gate-mode per D-CONT-6 — _accept: `enum Mode { Warn, Gate }` + `fn current_mode()` parses env var `UI_CONTRAST_MODE` (default `warn`); WARN failures emit `eprintln!("WARN: contrast pair {pair_id} = {ratio:.2} < threshold {threshold:.1}")` + test PASS; gate failures collect violations + panic with joined message at end of test_
- [ ] T-CONT-D5 — Implement MIN_PAIRS floor per D-CONT-5 — _accept: `const MIN_PAIRS: usize = 60` + separate `#[test] fn pairs_table_meets_minimum_count()` panics with "theme token enumeration detected only {N} pairs; refactor likely broke enumeration (MIN_PAIRS = 60)" message_
- [ ] T-CONT-D6 — Run falsification probe P-CONT-1 (both variants) — _accept: variant A (deliberately-low-contrast pair) — temporarily prepend `ContrastPair { pair_id: "probe_low_contrast_white_on_pale_grey", fg: Color::WHITE, bg: Color::from_rgb(0.9, 0.9, 0.9), class: Body }`; `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast` panics with "contrast assertion failed: probe_low_contrast_white_on_pale_grey = 1.07 < threshold 4.5"; revert. Variant B (MIN_PAIRS floor) — comment out 25 PAIRS entries; same gate command panics with floor violation message; revert. Both variants logged in dev-end report under § Falsification._
- [ ] T-CONT-D7 — Verify R-NR contract — _accept: `git diff -- crates/ui/src/` empty (zero production code touched); `cargo test -p ui --test visual_snapshots` PASS byte-identical; no `[dependencies]` add to `crates/ui/Cargo.toml` (no `[dev-dependencies]` add either — hand-rolled formula has zero new deps)_
- [ ] T-CONT-D8 — Update `spec/trace.toml` `REQ-UI-CONTRAST-ASSERTER-001` row — _accept: `crates = ["crates/ui"]`; `tests` lists fn names (`all_theme_pairs_meet_wcag`, `pairs_table_meets_minimum_count`, `ref_vector_white_on_black_is_21`, `ref_vector_black_on_white_is_21`, `ref_vector_777_on_fff_is_4_48`, `ref_vector_888_on_000_is_5_92`); `state = "dev-done"`_
- [ ] T-CONT-D9 — Dev-side gates — _accept: `cargo fmt -p ui -- --check` clean; `cargo test -p ui --test contrast` PASS in WARN default (6 WARN-log lines expected per D-CONT-3 audit: `fg_3_on_panel_raised_dark`, `fg_on_accent_on_accent_light`, `up_500_on_canvas_light`, `down_500_on_canvas_light`, `warn_500_on_canvas_light`, `warn_500_on_panel_light`); `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast` FAILs with the same 6 lines as the panic violation list (expected at v0.1.0 — operator promotes to gate at v0.2.0 only after upstream hex tune per § Operator decisions deferred to v0.2.0 promotion); `bash scripts/verify_anchors.sh` → 75/75 PASS byte-identical_

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
