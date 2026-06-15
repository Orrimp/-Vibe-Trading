---
slug: ui-contrast-asserter
status: proposed
owner: analyst
updated: 2026-06-15
---

# Tasks — ui-contrast-asserter

> **v0.2.0 close-out tasks appended 2026-06-15 (analyst)** — see
> [§ v0.2.0 close-out](#v020-close-out-tasks) below. The v0.1.0 task tree is
> preserved verbatim; its parked `T-CONT-P1` presenter task is the first
> v0.2.0 deliverable (V2-R1).

# Tasks — ui-contrast-asserter v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick B Wave 1 promotion in
> [`spec/dev-notes/pick-b-cross-cutting-safety-duo-2026-05-29.md`](../dev-notes/archive/2026-Q2/pick-b-cross-cutting-safety-duo-2026-05-29.md).
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

---

## v0.2.0 close-out tasks

> **Analyst V2-M0 2026-06-15.** Three deliverables per
> [feature.md § v0.2.0 close-out](feature.md#v020-close-out): V2-R1 ship the
> parked presenter, V2-R2 flip the gate default, V2-R3 dispose the 6 sub-AA
> pairs (recommended path A = ratify all 6 as opt-out). ~0.25 dev day +
> ~0.25 tester day + presenter deck. **Gated on V-CONT2-1 (A vs B) +
> V-CONT2-2 (flip-now vs flip-next) operator decisions.**

### V2-M0 — Analyst (DONE 2026-06-15)

- [x] V2-CONT-M0.1 — v0.2.0 close-out brief appended to feature.md — _accept: § v0.2.0 close-out with V2-R1/R2/R3 + V2-AC.1-7 + per-pair tune-vs-opt-out table + V-CONT2-1/2 operator decisions + V2-K1-3 + V2-H1-3; baseline-equity-divergence e2e N/A justified_
- [x] V2-CONT-M0.2 — Per-pair tune feasibility re-verified against live theme.rs hex — _accept: WCAG formula run on candidate darkenings; 4 of 6 trivially tunable (#1 up_500 #2 down_500 #3 fg_3-dark #4 fg-on-accent), 2 hard amber (#5 #6 warn_500); pair #3 dark-mode correction logged_
- [x] V2-CONT-M0.3 — Frontmatter bumped + intended trace change reported — _accept: feature.md/tasks.md version 0.1.0→0.2.0, status→proposed, owner→analyst; trace REQ-UI-CONTRAST-ASSERTER-001 intended change reported to orchestrator (NOT edited by analyst per close-out constraint)_

### V2-M-T1 — Architect

- [ ] V2-CONT-T1.1 — Ratify V-CONT2-1 (disposition A vs B) + V-CONT2-2 (gate-flip timing) operator decisions — _accept: § Design records the chosen disposition + timing; if (A), lock the 6→OptOut re-class list + 6 OPT_OUTS manifest rows with per-pair reason strings; if (B), lock the N tuned tokens + visual-rebaseline + operator-color-sign-off plan_
- [ ] V2-CONT-T1.2 — Lock the `current_mode()` default-arm flip + header/doc-comment edits — _accept: D-clause names the exact `_ => Mode::Gate` change at contrast.rs:101-106 + the file-header / all_theme_pairs_meet_wcag doc-block updates (lines 826-834) describing gate-as-default_
- [ ] V2-CONT-T1.3 — Confirm V2-K1 mitigation: gate-mode dry-run shows exactly the known 6 — _accept: architect dry-run (or developer pre-flight) confirms `UI_CONTRAST_MODE=gate` surfaces exactly the 6, zero 7th; git log on theme.rs since 2026-05-29 confirms no color change landed_
- [ ] V2-CONT-T1.4 — Confirm ADR-0048 carry-forward (or no-ADR) for the gate-flip — _accept: § D-clause records whether the WARN→gate promotion needs an ADR-0048 Changelog ride-along row (v0.1.0 § D-CONT-7 set the boundary-test precedent; the flip is within that contract)_
- [ ] V2-CONT-T1.5 — Frontmatter flipped arch-done; HANDOFF → developer — _accept: feature.md/tasks.md status proposed → arch-done, owner analyst → developer_

### V2-M-DEV — Developer (path-A scope; ~0.25 day)

- [x] V2-CONT-D1 — (path A) Re-class the 6 sub-AA pairs `Body → OptOut("<reason>")` in PAIRS — **file: `crates/ui/tests/contrast.rs` (fg_3_on_panel_raised_dark ~line 202, fg_on_accent_on_accent_light ~line 392, up_500_on_canvas_light ~line 426, down_500_on_canvas_light ~line 451, warn_500_on_canvas_light ~line 483, warn_500_on_panel_light ~line 491)** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored`. PAIRS length stays 83.
- [x] V2-CONT-D2 — (path A) Add 6 mirror rows to the `OPT_OUTS` manifest table — **file: `crates/ui/tests/contrast.rs` OPT_OUTS table now 15 entries (was 9)** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test opt_outs_all_have_reasons ... ok`
- [x] V2-CONT-D3 — Flip `current_mode()` default arm to `Mode::Gate` + update header/doc-comments — **file: `crates/ui/tests/contrast.rs` lines 102-110 `Ok("warn") => Mode::Warn, _ => Mode::Gate`; file-header Mode section updated; all_theme_pairs_meet_wcag doc-block updated** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed`
- [x] V2-CONT-D4 — Verify gate-default PASS with env unset — **env UNSET** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s`. 6 opt-out audit lines logged.
- [x] V2-CONT-D5 — Verify WARN escape hatch byte-identical to v0.1.0 — **`UI_CONTRAST_MODE=warn`** | test: `UI_CONTRAST_MODE=warn cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored`. No failures; 6 pairs now OptOut so they go to audit branch (not WARN branch) — back-compat PASS confirmed.
- [x] V2-CONT-D6 — Re-run P-CONT-1.A probe under gate-DEFAULT (env unset) — **probe pair `probe_d6_white_on_pale_grey` (white on 0.9/0.9/0.9) temporarily inserted into PAIRS** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `panicked… probe_d6_white_on_pale_grey = 1.25 < threshold 4.5`; reverted; clean re-run 7/7 PASS.
- [x] V2-CONT-D7 — Verify R-NR.1 holds under path A — **`git diff -- crates/ui/src/` → 0 bytes; `git diff --stat -- '*.png'` → 0 bytes; visual_snapshots 3-failure pre-dates this change (charts_screen_dark_* pre-existing); contrast.rs is the only changed file** | test: `cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed`. `git diff --stat -- crates/ui/src/` empty confirmed.
- [ ] V2-CONT-D8 — _(path B ONLY)_ Tune the N `.light` tokens + rebaseline visuals — N/A: operator chose path (A); skipped.

### V2-M-FINAL — Tester

- [x] V2-CONT-FINAL.1 — Gate-default run (env unset) PASS — **file: `crates/ui/tests/contrast.rs`** | test: `cargo test -p ui --test contrast --no-default-features --features live -- --nocapture` | output: `test result: ok. 7 passed; 0 failed; 2 ignored`. Exactly 6 opt-out audit lines confirmed (fg_3_on_panel_raised_dark=3.75, fg_on_accent_on_accent_light=3.52, up_500_on_canvas_light=4.46, down_500_on_canvas_light=4.33, warn_500_on_canvas_light=2.96, warn_500_on_panel_light=3.11). Zero 7th asserting failure. V2-AC.2 green.
- [x] V2-CONT-FINAL.2 — WARN escape-hatch run byte-identical to v0.1.0 — **`UI_CONTRAST_MODE=warn`** | test: `UI_CONTRAST_MODE=warn cargo test -p ui --test contrast --no-default-features --features live` | output: `test result: ok. 7 passed; 0 failed; 2 ignored`. 6 formerly-WARN pairs now go to audit branch (OptOut class); no WARN lines (correct). V2-AC.3 green.
- [x] V2-CONT-FINAL.3 — Regression-block probe under gate-default — **tester independently inserted `probe_tester_white_on_pale_grey` (WHITE on 0.9/0.9/0.9) into PAIRS with env UNSET** | test: `cargo test -p ui --test contrast --no-default-features --features live -- all_theme_pairs_meet_wcag` | output: `panicked… probe_tester_white_on_pale_grey = 1.25 < threshold 4.5`; exit 101; reverted; clean re-run 7/7 PASS. Gate is actually enforcing. V2-AC.4 green.
- [x] V2-CONT-FINAL.4 — R-NR / anchor / visual non-regression (path A) — **`git diff 61ba42d^ 61ba42d -- crates/ui/src/` → empty; `git diff 61ba42d^ 61ba42d -- '*.png'` → empty. charts_screen_dark 3/3 FAIL confirmed PRE-EXISTING (introduced by 3ba82fc cross-platform chart.rs change, baselines last set 93845af); NOT introduced by 61ba42d. anchors N/A (no strategy/exec/backtest crates touched).** V2-AC.6 green.
- [x] V2-CONT-FINAL.5 — Write v0.2.0 test-final report — **file: `spec/ui-contrast-asserter/reports/test-2026-06-15-1300-v0.2.0-ui-contrast-asserter.md`** | VERDICT → PASS. All V2-AC gates green. Gate-flip + 6-pair disposition confirmed. False alarms (runner.rs clippy + charts_screen_dark) independently confirmed pre-existing and non-reproducing as v0.2.0 issues.

### V2-M-PRESENT — Presenter

- [ ] V2-CONT-P1 — **(V2-R1; also closes the parked v0.1.0 T-CONT-P1)** Deck `spec/ui-contrast-asserter/presentations/ui-contrast-asserter-<date>.md` — _accept: cross-cutting safety-duo recap; M-T1 pair-count + opt-out-count table; the 6 design-intent WARN lines; 2-week WARN observation contract (now elapsed); the v0.2.0 gate-flip + 6-pair disposition outcome; sibling-redactor cross-link; operator-decide-ready. v0.1.0 baseline frontmatter → status: shipped on approval_

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
