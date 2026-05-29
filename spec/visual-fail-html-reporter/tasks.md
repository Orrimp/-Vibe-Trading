---
slug: visual-fail-html-reporter
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — visual-fail-html-reporter v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick A Wave 1 promotion in
> [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
> ~1 dev day total. Bias DURABLE per
> [AGENT.md ## Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-VFH-M0.1 — Recipient contract gap surfaced at R1-R4 — _accept: feature.md R1-R4 + R-NR + K1-K4 + H1-H3 + Q1-Q3 + pre-drawn verdict tree_
- [x] T-VFH-M0.2 — backlog Active row appended under § Process / tooling — _accept: PROMOTED Idea → Active 2026-05-29 annotation_
- [x] T-VFH-M0.3 — trace row `REQ-VISUAL-FAIL-HTML-REPORTER-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (M-T1 fast-skip expected; no new ADR)

- [ ] T-VFH-T1.1 — Ratify Q1 (output path) + Q2 (base64 crate) + Q3 (tester.md stanza placement) — _accept: § Design D-VFH-1..3 records Q1/Q2/Q3 picks (analyst recommends all (a) DURABLE)_
- [ ] T-VFH-T1.2 — Pick helper module filename (analyst suggested `crates/ui/tests/fixtures/visual_fail_html.rs`) + function signature (per R1.3) — _accept: § Design D-VFH-4 names the file + signature_
- [ ] T-VFH-T1.3 — Confirm ADR-0048 carries forward; one Changelog row appended — _accept: ADR-0048 § Changelog gets 2026-MM-DD architect M-T1 row; no new ADR_
- [ ] T-VFH-T1.4 — Wave decomposition for M-DEV (analyst expects 1 wave; M-T1 may split into helper + tester.md stanza + self-test) — _accept: § Design D-VFH-5 names wave count + per-wave LoC budget_
- [ ] T-VFH-T1.5 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _accept: feature.md + tasks.md frontmatter updated_

## M-DEV — Developer (single wave expected; ~1 day)

- [ ] T-VFH-D1 — Add `base64 = "0.22"` (or architect-ratified crate version) under `crates/ui/Cargo.toml [dev-dependencies]` — _accept: cargo check -p ui --tests succeeds; no production dep change_
- [ ] T-VFH-D2 — Author `crates/ui/tests/fixtures/visual_fail_html.rs` (or architect-ratified filename) with `emit_visual_fail_html(...)` per R1.3 signature — _accept: function exposes 7-arg signature; inline `<style>` HTML template + base64 PNG encode + `fs::write` to default `target/visual-diff/<test>-<ts>.html` path; emits second copy to spec/<slug>/reports/ iff env var EMIT_VISUAL_FAIL_TO_SPEC=1 set; ~50-80 LoC budget_
- [ ] T-VFH-D3 — Wire `emit_visual_fail_html(...)` invocation into FAIL branch of `crates/ui/tests/fixtures/visual_diff.rs::matches_screenshot` — _accept: helper called ONLY on `Err(VisualDiffError::...)` return path; PASS path byte-identical to today; existing visual-snapshot tests stay PASS_
- [ ] T-VFH-D4 — Author self-test pair per R4.1 + R4.3 — _accept: two `#[test] fn` in fixtures/visual_fail_html.rs (or sibling test file): (1) default emit path produces HTML with inlined base64 PNGs + assertion text under `tempfile::TempDir`; (2) `EMIT_VISUAL_FAIL_TO_SPEC=1` path produces byte-identical second file; both clean up via `TempDir` drop_
- [ ] T-VFH-D5 — Amend `.claude/agents/tester.md` per R3.1 — _accept: new ~5-10 line stanza titled "## Visual failures — HTML artifact emission" (architect-ratified placement per Q3); additive only, no removed prose; cites the helper + the env var_
- [ ] T-VFH-D6 — Dev-side gates — _accept: cargo test -p ui --tests PASS; cargo clippy -p ui --all-features -- -D warnings clean; bash scripts/verify_anchors.sh 71/71 PASS byte-identical_

## M-FINAL — Tester

- [ ] T-VFH-FINAL.1 — Run `cargo test -p ui --tests` — _accept: all pre-existing visual-snapshot tests PASS byte-identical; new self-tests PASS_
- [ ] T-VFH-FINAL.2 — Deliberate-FAIL probe — _accept: temporarily perturb a baseline PNG (e.g. flip one pixel via `convert` or `image` CLI), re-run the test that uses it, confirm helper emits `target/visual-diff/<test>-<ts>.html`, open in Safari/Chrome, eyeball the three PNG triple + assertion-text rendering, restore baseline, confirm test goes back to PASS; report § Visual failures cites the screenshot path_
- [ ] T-VFH-FINAL.3 — Verify `.claude/agents/tester.md` stanza shape — _accept: grep tester.md for "Visual failures — HTML artifact emission", confirm presence at architect-ratified location, no contract regression in surrounding stanzas_
- [ ] T-VFH-FINAL.4 — Anchor + spec-lint gate — _accept: `bash scripts/verify_anchors.sh` → 71/71 PASS byte-identical; `uv run scripts/spec_lint.py` → exit 0 (no new violations)_
- [ ] T-VFH-FINAL.5 — Write test-final report — _accept: spec/visual-fail-html-reporter/reports/test-final-2026-MM-DD-visual-fail-html-reporter.md per [template](../../.claude/skills/rust-test/templates/test-report.md); VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter

- [ ] T-VFH-P1 — Deck `spec/visual-fail-html-reporter/presentations/visual-fail-html-reporter-<date>.md` — _accept: per-cycle benefit numbers (operator review time before/after); sample HTML artifact embedded as screenshot OR linked URL; trifecta-direction cross-ref; tester contract amendment recap; operator-decide-ready_

## Notes

- **Anchor contract**: 71/71 byte-identical pre/post. Zero file output
  from the helper on test PASS. Same shape as ADR-0048 D6 anchor-
  additivity contract for Recipe tests.
- **Tester contract amendment ownership**: this feature OWNS the
  `.claude/agents/tester.md` amendment per trifecta direction
  § Risk R1 mitigation. The Wave 1 sibling (`ui-test-harness-viewport-
  matrix`) inherits the stanza without amendment.
- **Per-cycle benefit**: LARGE per `process-tooling-survey-2026-05-29.md`
  Rank 2. Operator review time on visual FAILs drops from ~5 min
  (open three PNGs + cross-reference test source) to ~30 s (open one
  HTML in Safari/Chrome, read assertion text inline with the rendered
  visual diff).
