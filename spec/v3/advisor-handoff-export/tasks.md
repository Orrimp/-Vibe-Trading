---
slug: advisor-handoff-export
status: arch-done
owner: architect
updated: 2026-07-10
---

# Tasks — advisor-handoff-export (P5, the SUGGEST → manual hand-off export)

Analyst-authored skeleton for the **architect** to refine at M-T1. **The build is
GATED behind the operator's `DECISION-P5-WORDING` ratification** (the remediation-plan
"wording operator-ratified before build" gate) — T0 below is that gate, and no code task
starts until it is checked. See [`feature.md`](feature.md) for the brief, the not-advice
boundary analysis, the § Draft wording template (the thing being ratified), the shorts
finding, and Q-HE-1..6.

Hard invariants for every task (from the CLAUDE.md non-negotiables + the P5 non-goals):
**NO order placement / NO venue API (register B-2 intact); NO new engine computation; NO
LLM in the export path (deterministic + offline); NO new numbers (verbatim from the
mirror); NO prescriptive voice in the content ('the plan says X', never 'you should X');
anchors 119/119 before AND after; spec-lint PASS(0).**

## Gate

- [x] **T0 — operator ratifies `DECISION-P5-WORDING`.** ✅ RATIFIED AS DRAFTED (operator, 2026-07-10; orchestrator-recorded — the § Draft wording Variant A+B is the binding serialiser contract). The operator reviews § Draft
  wording in `feature.md` (instantiated on the golden `(BTCUSDT, €200, 2024 H1)` case) and
  ratifies or redlines: (a) the BenchmarkWins-first ordering; (b) the "following this plan
  manually would mean X" not-advice frame; (c) the three `«NEW»` honesty lines (the ~1/5
  note, the era-qualified thesis one-liner, the "your decision, your account" footer);
  (d) any redlines. — _blocking: no build task starts until this is checked._

## Architect (M-T1, after T0)

- [x] **T1 — decided Q-HE-1..6 + took ADR-0088.** ✅ DONE (architect, 2026-07-10). See
  [`feature.md`](feature.md) § Design + [ADR-0088](../../architecture/adr/0088-suggest-manual-handoff-export.md).
  Q-HE-1 = md `.md`; Q-HE-2 = the SUGGEST/`Screen::ForwardPlan` screen; Q-HE-3 = one-per-plan
  + seed/`last_bar_ts` stamp (NO wall-clock); Q-HE-4 = `ui` owns it, REFINED to serialise the
  pure-`ui` `ForwardPlanView` mirror not the `#[cfg(live)]`-gated `agent::config::ForwardPlan`;
  Q-HE-5 = golden-text + byte-determinism + filename tests **plus a rendered-pixel button
  proof** (the button is a new visible journey control); Q-HE-6 = the action is absent until
  crowned. R-HE.12 divergence-e2e recorded N/A-not-skipped. Wording locked into the serialiser
  design (27 new consts, verbatim). — _acceptance MET: ADR-0088 accepted + registered
  atomically in `adr/README.md` (Registry row + frontmatter same edit pass); took 0088 (0087 =
  the sibling P4 `advisor-lot-realism`, verified free first); `adr_registry_check.py --pre-commit`
  clean._

## Developer lane (after T1)

- [x] **T2 — the pure serialiser + the 27 strings.** Add `crates/ui/src/export/plan_export.rs`
  with `serialize_plan_export(plan: &ForwardPlanView, report: &BakeoffReportMirror, narration:
  &NarrationState, fx: Option<&FxNote>) -> String` + `export_filename(report) -> String`, per
  the ratified § Draft wording. Walk the SAME predicate tree as `forward_plan.rs::view` +
  `leaderboard.rs::recommendation_block`, reusing the SAME `crate::strings` consts +
  `crate::widgets::num` formatters. Add the ~27 new `crate::strings` consts (feature § Design
  "New strings"), each carrying the ratified text VERBATIM, wired into `strings::all()`. —
  _acceptance: no network/model call, no re-derived number; `BenchmarkWins` leads the verdict
  block; every «SOURCE» line is an existing const, every «NEW» line a new verbatim const._
  ✅ DONE (developer, 2026-07-10). `crates/ui/src/export/mod.rs` (new, re-exports) +
  `crates/ui/src/export/plan_export.rs:167` (`serialize_plan_export`) + `:203`
  (`export_filename`); `crates/ui/src/lib.rs` `pub mod export;`. Signature uses
  `trading_core::FxNote` (the exact type `screens/forward_plan.rs` already imports) rather
  than the non-existent `crate::state::FxNote` path the ADR's prose names — zero semantic
  change, zero new dep edge (`trading_core` was already a dep; `cargo tree -p ui` verified
  byte-identical before/after via `git diff --stat -- Cargo.toml Cargo.lock crates/ui/Cargo.toml`
  = empty). The 27 `PLAN_EXPORT_*` consts land at `crates/ui/src/strings.rs:4114` onward
  (verbatim ratified text, transcribed byte-exact via a Python `repr()` read of the ratified
  `feature.md` template — see the handoff summary) + registered in `strings::all()` (7 lines
  after the `FORWARD_PLAN_CONFIDENCE_NOTE` entry). Box-drawing rule lines are
  serialiser-emitted (`push_rule`, `plan_export.rs:112`), not consts, per the ADR's own "New
  strings" note. Test: `cargo test -p ui --lib export::` → `test result: ok. 13 passed; 0
  failed; 0 ignored; 0 measured; 602 filtered out`.
- [x] **T3 — the shared resolver + the mirror seed echo.** Move `crown_credibility` +
  `CrownCredibility` (behaviour-preserving) to `pub(crate)` in `leaderboard/state.rs` so the
  serialiser reuses the EXACT screen resolver. Add an additive `run_seed: [u8; 32]` field to
  `BakeoffReportMirror`, set in `from_report` from `report.request.seed` (value-echo, no
  engine computation); update the ~6 `fake_bakeoff_report_mirror*` fixtures. — _acceptance:
  the screen's `crown_credibility_render.rs` proof still passes; `run_seed` reads an existing
  value; no `crates/backtest` diff._
  ✅ DONE (developer, 2026-07-10). `crates/ui/src/leaderboard/state.rs:444`
  (`pub(crate) enum CrownCredibility`) + `:476` (`pub(crate) fn crown_credibility`), moved
  verbatim from `screens/leaderboard.rs` (which now imports both — `crown_credibility_element`
  stays there). `crates/ui/src/leaderboard/state.rs:609` (`pub run_seed: [u8; 32]` field) +
  `:666` (`run_seed: report.request.seed,` in `from_report`) + `:1230` (`ready_mirror()` test
  helper). All 6 `fake_bakeoff_report_mirror*` literal sites in `crates/ui/src/fixtures.rs`
  gained a distinct deterministic `run_seed` (mechanical). `git diff --stat -- crates/backtest`
  = empty (no engine diff). Test: `cargo test -p ui --test crown_credibility_render` → `test
  result: ok. 3 passed; 0 failed` (all 3 pixel-render assertions unaffected by the move).
- [x] **T4 — the variant branches + new fixtures.** Wire the decision table (feature § Design):
  credibility in-body (WeakEvidence/Passes/NotApplicable), the always-present survivorship
  caveat + venue/provenance/trust + warnings (from `DataQualityView`), the confidence block
  (only when `plan.confidence` is `Some`) + the ~1/5 note, the F9 narration (only when
  `NarrationState::Ready`), and — when short-capable — the short IF/THEN rules + liquidation +
  `SHORT_UNBOUNDED_LOSS_DISCLAIMER` verbatim (R-HE.7). Add fixtures `fake_forward_plan_short()`
  + a `crown_clears_dsr=true` mirror. — _acceptance: BenchmarkWins/AllFragile emit NO
  credibility badge (ADR-0085 NotApplicable); a short crown emits the unbounded-loss
  disclaimer._
  ✅ DONE (developer, 2026-07-10). Decision table wired in `crates/ui/src/export/plan_export.rs`:
  `push_measured_answer` (credibility in-body), `push_data_source` (survivorship + venue/
  provenance/trust + warnings), `push_trust` (confidence block, only called when
  `plan.confidence.is_some()`) + `PLAN_EXPORT_ONE_IN_FIVE_NOTE`, `push_provenance` (F9 narration
  embedded only on `NarrationState::Ready`), `push_short_rules_append` + `push_short_risk`
  (short IF/THEN + liquidation + `SHORT_UNBOUNDED_LOSS_DISCLAIMER`, gated on
  `plan.is_short_capable()`). `fake_forward_plan_short()` ALREADY existed (advisor-short-selling
  T-U3, `crates/ui/src/fixtures.rs`) — reused, not re-added. The genuinely-new fixture is
  `crates/ui/src/fixtures.rs:2108` `fake_bakeoff_report_mirror_five_arm_passes_dsr()` (clones
  `fake_bakeoff_report_mirror_five_arm()`, sets `crown_clears_dsr = true` /
  `deflated_sharpe = 0.97`, mirroring the `scorecard_with_dsr(true)` test-helper convention in
  `screens/leaderboard.rs`). Test: `cargo test -p ui --lib export::` → `test result: ok. 13
  passed; 0 failed` (incl. `benchmark_wins_carries_headline_and_bridge_but_no_credibility_badge`
  and `non_short_capable_plan_never_carries_short_risk_section` — the two negative controls
  this task names).
- [x] **T5 — golden-text + determinism tests.** Golden-text tests over the 4 variants
  (BenchmarkWins / ActiveWins+WeakEvidence on `fake_bakeoff_report_mirror_five_arm` + €200 /
  ActiveWins+Passes / short-crowned) asserting the EXACT ratified text + negative controls
  (BenchmarkWins ⇒ NO credibility badge; short-crowned ⇒ `SHORT_UNBOUNDED_LOSS_DISCLAIMER`);
  a byte-determinism test (same inputs ⇒ identical output; no wall-clock); a
  filename-determinism test. — _acceptance: green; the negative controls prove the branches
  track the state, not a tautology._
  ✅ DONE (developer, 2026-07-10). `crates/ui/src/export/plan_export.rs:800` `mod tests` — 13
  inline tests: 4 golden-text variants (`benchmark_wins_carries_headline_and_bridge_but_no_
  credibility_badge`, `active_wins_weak_evidence_carries_the_credibility_caveat_in_body`,
  `active_wins_passes_dsr_carries_the_passes_line_not_weak_evidence`,
  `short_capable_plan_carries_the_unbounded_loss_disclaimer_and_liquidation_line`) each
  asserting the EXACT ratified consts are present/absent; 2 negative controls
  (`non_short_capable_plan_never_carries_short_risk_section`,
  the BenchmarkWins test's own no-credibility-badge assertions); 1 F9-narration-lifecycle test;
  2 byte-determinism tests (`same_inputs_produce_byte_identical_output` incl. a
  different-seed-changes-output check, `short_capable_variant_is_also_byte_deterministic`); 3
  filename tests (`export_filename_matches_the_adr_illustrative_example` — asserts the EXACT
  `plan-BTCUSDT-2024-h1-a1b2c3d4.md` ADR-0088 § Design gives as its own illustrative example,
  `export_filename_is_deterministic_slug_safe_and_wall_clock_free`,
  `export_filename_differs_when_run_seed_differs`) + 1 slug unit test. Test:
  `cargo test -p ui --lib export::` → `test result: ok. 13 passed; 0 failed; 0 ignored; 0
  measured; 602 filtered out`. Full-suite regression: `cargo test -p ui --lib` → `test result:
  ok. 615 passed; 0 failed` (602 baseline + these 13).

## ui-designer lane (after T1, ‖ developer against the agreed signature)

- [ ] **T6 — the export trigger + write + gitignore + pixel proof.** The "Export this plan"
  button (`PLAN_EXPORT_BUTTON`) on the `Screen::ForwardPlan` header, rendered ONLY when
  `forward_plan_screen_state.plan == PanelState::Ready` (Q-HE-6). A `Message::ExportPlan`
  handler assembles inputs from `forward_plan_screen_state.plan` + `leaderboard_screen_state`
  (mirror + narration) + `forward_fx`, calls `serialize_plan_export`, and does the single
  `std::fs::write` to a git-ignored workspace-root `plan-exports/` dir; add `/plan-exports/`
  to `.gitignore`. A rendered-PIXEL proof (macOS-gated): the button paints in `Ready`, ABSENT
  in `Empty`/`Loading`/`Error` (the Q-HE-6 negative control). — _acceptance: no export action
  on the empty-plan state; the button passes the render walk; the artifact lands outside every
  `spec/**` anchor glob._

## Tester (closes the loop)

- [ ] **T7 — gates + lifecycle.** `cargo build`/`test -p ui` + clippy `-D warnings` + `fmt
  --check` + the golden/determinism tests + the render-pixel button proof + `verify_anchors.sh`
  **119/119** before AND after + `spec_lint.py` **PASS(0)**; confirm NO `crates/backtest` diff
  (no engine computation) and register B-2 intact (no order/venue code); confirm the artifact
  lands in the git-ignored `plan-exports/` dir (never `spec/*/reports/`). Own the `feature.md
  status` + trace `state` flip to `shipped` only on `VERDICT → PASS` (ADR-0082). — _acceptance:
  the export is a deterministic text artifact, adds no anchor, and the honesty context
  (credibility + survivorship + short disclaimer) is present in the golden output._
