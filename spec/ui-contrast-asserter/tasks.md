---
slug: ui-contrast-asserter
status: shipped
owner: presenter
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

- [x] T-CONT-D1 — Author `crates/ui/tests/contrast.rs` per § Design D-CONT-1 + D-CONT-3 + D-CONT-4 — **file: `crates/ui/tests/contrast.rs:1-960`** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored`. File defines `ContrastPair`, `ContrastClass`, `OptOutEntry`; `const PAIRS: &[ContrastPair]` with 83 entries; `const OPT_OUTS: &[OptOutEntry]` with 9 entries.
- [x] T-CONT-D2 — Implement hand-rolled WCAG 2.1 formula per D-CONT-2 — **file: `crates/ui/tests/contrast.rs:47-68`** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test ref_vector_white_on_black_is_21 ... ok`, `test ref_vector_black_on_white_is_21 ... ok`, `test ref_vector_777_on_fff_is_4_48 ... ok`, `test ref_vector_888_on_000_is_5_92 ... ok`. All 4 reference vectors PASS with `< 0.01` f64 tolerance.
- [x] T-CONT-D3 — Implement pair iteration + per-class assertion per R2.3 — **file: `crates/ui/tests/contrast.rs:849-910`** | test: `cargo test -p ui --test contrast --no-default-features --features live -- --nocapture` | output: `test all_theme_pairs_meet_wcag ... ok`. Opt-out audit lines confirmed via `--nocapture` run.
- [x] T-CONT-D4 — Implement WARN-mode + gate-mode per D-CONT-6 — **file: `crates/ui/tests/contrast.rs:94-106`** | test: `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast --no-default-features --features live` | output: `test all_theme_pairs_meet_wcag ... FAILED` with 6 violation lines in panic message. WARN default: `test result: ok. 7 passed`.
- [x] T-CONT-D5 — Implement MIN_PAIRS floor per D-CONT-5 — **file: `crates/ui/tests/contrast.rs:116`** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test pairs_table_meets_minimum_count ... ok`. P-CONT-1.B probe (MIN_PAIRS=200 temporarily) confirmed floor fires with "theme token enumeration detected only 83 pairs; refactor likely broke enumeration (MIN_PAIRS = 200)".
- [x] T-CONT-D6 — Run falsification probe P-CONT-1 (both variants) — **file: `crates/ui/tests/contrast.rs:919-960`** | Variant A: probe pair inserted → gate panic "probe_low_contrast_white_on_pale_grey = 1.25 < threshold 4.5"; PASS → reverted. Variant B: MIN_PAIRS=200 → gate panic "theme token enumeration detected only 83 pairs; refactor likely broke enumeration (MIN_PAIRS = 200)"; PASS → reverted. Both probes confirmed.
- [x] T-CONT-D7 — Verify R-NR contract — **`git diff -- crates/ui/src/`** empty (zero production code); visual_snapshots 51/51 PASS; no new Cargo.toml deps added. | test: `cargo test -p ui --test visual_snapshots --no-default-features --features live` | output: `test result: ok. 51 passed; 0 failed`.
- [x] T-CONT-D8 — Update `spec/trace.toml` `REQ-UI-CONTRAST-ASSERTER-001` row — **file: `spec/trace.toml:2288-2291`** | `crates = ["crates/ui"]`; `tests` lists 7 fn names; `state = "dev-done"`. | verified by grep: `grep "dev-done" spec/trace.toml | grep CONTRAST`.
- [x] T-CONT-D9 — Dev-side gates — **all gates GREEN**: `cargo fmt -p ui -- --check` clean; WARN default 7/7 PASS (6 eprintln WARN lines observed); gate mode panic with same 6 lines; 75/75 anchors PASS. | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s`.

## M-FINAL — Tester

- [x] T-CONT-FINAL.1 — Run `cargo test -p ui --test contrast` with default `UI_CONTRAST_MODE=warn` — **file: `crates/ui/tests/contrast.rs`** | test: `cargo test -p ui --test contrast --no-default-features --features live -- --nocapture` | output: `test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s`. Exactly 6 WARN eprintln lines confirmed: `fg_3_on_panel_raised_dark=3.75`, `fg_on_accent_on_accent_light=3.52`, `up_500_on_canvas_light=4.46`, `down_500_on_canvas_light=4.33`, `warn_500_on_canvas_light=2.96`, `warn_500_on_panel_light=3.11`.
- [x] T-CONT-FINAL.2 — Run with `UI_CONTRAST_MODE=gate` — **file: `crates/ui/tests/contrast.rs`** | test: `UI_CONTRAST_MODE=gate cargo test -p ui --test contrast --no-default-features --features live` | output: `test all_theme_pairs_meet_wcag ... FAILED` with panic listing all 6 violations verbatim. 6 passed; 1 failed; 2 ignored.
- [x] T-CONT-FINAL.3 — Verify the floor assertion fires correctly — **Probe P-CONT-1.B**: temporarily set MIN_PAIRS=200 (equivalent to commenting out 25 entries — floor fires at same threshold; reverts to 60). `pairs_table_meets_minimum_count ... FAILED` with "theme token enumeration detected only 83 pairs; refactor likely broke enumeration (MIN_PAIRS = 200)". Reverted; clean PASS confirmed.
- [x] T-CONT-FINAL.4 — Verify visual-snapshot tests unaffected — test: `cargo test -p ui --test visual_snapshots --no-default-features --features live` | output: `test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.10s`. Also visual_fail_html_self_test 2/2, spawn_lab_run_yahoo_harness 3/3, training_log_recipe_harness 3/3 — all PASS.
- [x] T-CONT-FINAL.5 — Verify R-NR contract — `bash scripts/verify_anchors.sh` → 75/75 PASS; `git diff -- crates/ui/src/` → zero output; `git diff HEAD~1 -- crates/ui/Cargo.toml` → zero output. R-NR.1/R-NR.4/R-NR.5 all confirmed.
- [x] T-CONT-FINAL.6 — Write test-final report — **file: `spec/ui-contrast-asserter/reports/test-20260529-v0.1.0-ui-contrast-asserter.md`** | VERDICT → PASS. All 8 gates green; probes P-CONT-1.A and P-CONT-1.B both confirmed; anchors 75/75 byte-identical; sibling regressions zero; spec-lint delta all carry-forward class.

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
