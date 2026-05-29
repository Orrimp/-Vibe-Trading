---
slug: ui-test-harness-viewport-matrix
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — ui-test-harness-viewport-matrix v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick A Wave 1 promotion in
> [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
> ~3-4 dev days. Bias DURABLE per
> [AGENT.md ## Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).
> Predecessor: [`ui-test-harness-bootstrap v0.1.0`](../ui-test-harness-bootstrap/feature.md)
> (Charts-only three-viewport baselines, shipped 2026-05-12).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-VPM-M0.1 — feature.md R1-R5 + R-NR + K1-K4 + H1-H4 + Q1-Q3 + 4-cell verdict tree — _accept: bias DURABLE at every Q_
- [x] T-VPM-M0.2 — backlog Active row appended under § Process / tooling — _accept: PROMOTED Idea → Active 2026-05-29 annotation_
- [x] T-VPM-M0.3 — trace row `REQ-UI-TEST-HARNESS-VIEWPORT-MATRIX-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (inventory + dry-run + ratification)

- [ ] T-VPM-T1.1 — Audit existing widget test files per R1.2 — _accept: § Design D-VPM-1 enumerates every test file under `crates/ui/tests/` invoking `iced_test::screenshot` or `matches_screenshot`, with current viewport + per-file `#[test] fn` count; H1 confirmed or revised_
- [ ] T-VPM-T1.2 — Dry-run baseline PNG generation for 3-widget representative sample at all 3 viewport slots — _accept: § Design D-VPM-2 records actual PNG sizes (floor / typical / operator) for the sample; H3 size projection confirmed at 50-100 MB total OR routed to operator-decide per K2_
- [ ] T-VPM-T1.3 — Ratify Q1 (coverage scope) + Q2 (helper shape) + Q3 (`.gitattributes` rule) — _accept: § Design D-VPM-3 records all three picks (analyst recommends all (a) DURABLE); confirm git `exif` driver availability for Q3 default_
- [ ] T-VPM-T1.4 — Author opt-out list per K1 falsifier — _accept: § Design D-VPM-4 enumerates any widget × operator-slot pair that CANNOT render (with empirical evidence: dry-run failure trace); opt-outs documented at `// VIEWPORT-MATRIX-OPT-OUT: <reason>` markers; opt-out list ≤ 3 widgets per K1 mitigation_
- [ ] T-VPM-T1.5 — Confirm ADR-0048 + bootstrap § Design carry forward — _accept: ADR-0048 § Changelog gets 2026-MM-DD architect M-T1 row (or single row covers entire Wave 1 bundle); no new ADR_
- [ ] T-VPM-T1.6 — Wave decomposition for M-DEV — _accept: § Design D-VPM-5 records wave count + per-wave widget scope + LoC budget (~50-80 LoC helper + ~300 LoC test expansion across all files)_
- [ ] T-VPM-T1.7 — Confirm visual-fail-HTML stanza inheritance per R3.1 — _accept: § Design D-VPM-6 confirms the visual-fail-HTML stanza (authored by Wave 1 sibling) covers the matrix case OR amends stanza per R3.3_
- [ ] T-VPM-T1.8 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _accept: feature.md + tasks.md frontmatter updated_

## M-DEV — Developer (waves per M-T1 decomposition; ~3-4 days)

- [ ] T-VPM-D1 — Author shared helper at `crates/ui/tests/fixtures/viewport_matrix.rs` (or architect-ratified filename) per R1.3 + Q2 — _accept: function-with-closure shape (default Q2=(a)) exposing `run_all_slots(|viewport, scale| { ... })`; SLOTS constant table mirrors bootstrap precedent; ~50-80 LoC_
- [ ] T-VPM-D2 — Per-test expansion across all existing widget test files (count per M-T1 D-VPM-1) — _accept: each existing `#[test] fn` becomes three discrete `#[test] fn` per slot (floor / typical / operator) named e.g. `widget_panels_dark_floor`, `widget_panels_dark_typical`, `widget_panels_dark_operator`; PASSes at all three slots OR has architect-ratified opt-out comment; bootstrap Charts baselines stay byte-identical_
- [ ] T-VPM-D3 — Baseline PNG first-run generation — _accept: 90-120 new baseline PNGs at `crates/ui/tests/visual-baselines/<widget>_<theme>_<slot>.png`; first-run helper auto-write per R2.2; operator visually reviews PNGs before commit_
- [ ] T-VPM-D4 — `.gitattributes` rule per R2.4 + Q3 — _accept: rule added at workspace root `.gitattributes`; covers `crates/ui/tests/visual-baselines/**`; architect-ratified shape (Q3=(a) `binary diff=exif` or fallback `binary`)_
- [ ] T-VPM-D5 — Dev-side gates — _accept: cargo test -p ui --tests PASS (all expanded tests + opt-outs); cargo clippy -p ui --all-features -- -D warnings clean; bash scripts/verify_anchors.sh 71/71 PASS byte-identical_
- [ ] T-VPM-D6 — Operator-side PNG review request — _accept: develop emits a six-section recipe per [memory/feedback_human_verification_recipe.md](../../.claude/projects/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/memory/feedback_human_verification_recipe.md) — Command (open Finder at `crates/ui/tests/visual-baselines/`) + Steps (eyeball each `_operator.png` for rendering sanity) + Timing (~10-20 min for 90 PNGs) + Expected (no obviously-broken PNGs — clipping, blank canvas, etc.) + Failure mode (route back to dev with operator-flagged PNG names) + Cleanup (none)_

## M-FINAL — Tester

- [ ] T-VPM-FINAL.1 — Run `cargo test -p ui --tests` — _accept: all expanded tests PASS; existing Charts baselines from bootstrap byte-identical; new baselines stable across 2 consecutive runs (H1 cross-run stability re-confirmed)_
- [ ] T-VPM-FINAL.2 — Deliberate-FAIL probe per R3.1 inheritance — _accept: perturb one baseline PNG (e.g. flip pixel via image CLI); re-run test; confirm `target/visual-diff/<test>-<ts>.html` emits via visual-fail-HTML helper (assumes that feature shipped first per trifecta direction); open in Safari/Chrome; eyeball; restore baseline; test goes back to PASS; report § Visual failures cites the HTML path_
- [ ] T-VPM-FINAL.3 — `.gitattributes` verification — _accept: `git check-attr binary crates/ui/tests/visual-baselines/foo.png` returns `binary: set`; `git log -p` on a representative baseline PNG does not dump raw bytes (or shows exif metadata per Q3=(a))_
- [ ] T-VPM-FINAL.4 — Repo size delta check — _accept: `git diff --stat HEAD~ HEAD -- crates/ui/tests/visual-baselines/` reports net new PNG size ≤ 100 MB per K2 ceiling; if > 100 MB, route to operator-decide (oxipng recompress or scope phase)_
- [ ] T-VPM-FINAL.5 — Anchor + spec-lint gate — _accept: `bash scripts/verify_anchors.sh` → 71/71 PASS byte-identical; `uv run scripts/spec_lint.py` → exit 0 (no new violations)_
- [ ] T-VPM-FINAL.6 — Write test-final report — _accept: spec/ui-test-harness-viewport-matrix/reports/test-final-2026-MM-DD-ui-test-harness-viewport-matrix.md per [template](../../.claude/skills/rust-test/templates/test-report.md); per-widget × per-slot PASS table; opt-out list with architect-approval evidence; baseline PNG count + total size; VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter

- [ ] T-VPM-P1 — Deck `spec/ui-test-harness-viewport-matrix/presentations/ui-test-harness-viewport-matrix-<date>.md` — _accept: per-cycle benefit (every UI feature inherits 3-viewport snapshot coverage from this point); coverage stats (N test files extended, M baseline PNGs added, K opt-outs with rationale); H4 confirmation (≥ 1 new regression caught OR documented as "no regressions surfaced — confidence buffer"); trifecta-direction cross-ref; operator-decide-ready_

## Notes

- **Anchor contract**: 71/71 byte-identical pre/post. Visual baselines
  PNG growth is the only artifact delta — checked at FINAL.4 against
  K2 ceiling.
- **Tester contract amendment**: this feature INHERITS the visual-fail
  HTML stanza authored by the Wave 1 sibling `visual-fail-html-reporter`.
  Does not amend `.claude/agents/tester.md` independently. Per
  trifecta direction § Risk R1 mitigation, if the stanza needs
  matrix-specific tweaks, architect M-T1 amends in T-VPM-T1.7.
- **Cross-platform baseline determinism (K3)**: explicitly out of
  scope at v0.1.0; this brief assumes the bootstrap's single-canonical-
  Apple-Silicon-box contract (per bootstrap H1 RESOLVED-WITH-CAVEAT).
  Cross-platform falsifier is a separate `ui-test-harness-ci` Queue
  item.
- **Wall-clock estimate per architect M-T1 dry-run**: ~3-4 dev days
  (helper ~0.5d + per-test expansion ~2-2.5d + baseline review +
  gates ~0.5-1d) + ~0.5 tester day + ~0.5 presenter day ≈ 1 week
  wall-clock total.
