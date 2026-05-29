---
slug: visual-fail-html-reporter
status: dev-done
owner: tester
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

## M-T1 — Architect (DONE 2026-05-29; M-T1 close)

- [x] T-VFH-T1.1 — Ratify Q1 (output path) + Q2 (base64 crate) + Q3 (tester.md stanza placement) — _DONE: § Design records Q1 (a) DURABLE + Q2 (a) DURABLE + Q3 **overridden to (b)** new top-level section (tester.md has no pre-existing visual-failure stanza to append to)_
- [x] T-VFH-T1.2 — Pick helper module filename + function signature — _DONE: § Design D-VF-2 locks `crates/ui/tests/fixtures/visual_fail_html.rs` + `emit_visual_fail_html(VisualFailContext<'_>) -> Result<PathBuf, VisualFailHtmlError>`_
- [x] T-VFH-T1.3 — Confirm ADR-0048 carries forward; one Changelog row appended — _DONE: § Design D-VF-6 spec'd; row text drafted; developer commits during M-DEV_
- [x] T-VFH-T1.4 — Wave decomposition for M-DEV — _DONE: § Design Wave decomposition section + § Design D-VF-5 — single M-DEV wave, ~80-100 LoC helper + ~22 LoC tester.md amendment + ~5 LoC ADR row + ~50 LoC self-test pair_
- [x] T-VFH-T1.5 — Frontmatter flip owner: analyst → developer, status: draft → arch-done — _DONE: feature.md + tasks.md frontmatter updated_

## M-DEV — Developer (single wave; ~1 day; architect-ratified)

- [x] T-VFH-D1 — Add `base64 = "0.22"` under `crates/ui/Cargo.toml [dev-dependencies]` — _accept: `cargo check -p ui --tests` succeeds; `[dev-dependencies]` table only (no production change); existing `0.22.1` in `Cargo.lock` resolves cleanly (no version bump)_ | file: `crates/ui/Cargo.toml:167-172` | cmd: `cargo check -p ui --test visual_fail_html_self_test --no-default-features --features live` | output: `Finished \`dev\` profile ... in 1m 05s` (exit 0)
- [x] T-VFH-D2 — Author `crates/ui/tests/fixtures/visual_fail_html.rs` per § Design D-VF-1 + D-VF-2 — _accept: file exists; exports `pub struct VisualFailContext<'a>` (7 fields per D-VF-2), `pub fn emit_visual_fail_html(VisualFailContext<'_>) -> Result<PathBuf, VisualFailHtmlError>`, `pub enum VisualFailHtmlError { Io(io::Error), Image(image::ImageError) }` with `Display`/`Error` impls; HTML template matches D-VF-1 skeleton; inline `<style>` block; base64 encode via STANDARD engine; PNG dimensions via `into_dimensions()`; chrono UTC timestamp format_ | file: `crates/ui/tests/fixtures/visual_fail_html.rs:1-265` | cmd: `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live` | output: `test result: ok. 2 passed; 0 failed; 0 ignored`
- [x] T-VFH-D3 — Implement env-var-gated spec-persist in `emit_visual_fail_html(...)` per § Design Q1 ratification — _accept: when `EMIT_VISUAL_FAIL_TO_SPEC=1` AND `VISUAL_FAIL_SPEC_SLUG=<slug>` both set, helper additionally writes byte-identical HTML; missing-slug warning emitted_ | file: `crates/ui/tests/fixtures/visual_fail_html.rs:113-137` | cmd: `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live` | output: `test emit_visual_fail_html_spec_persist_writes_byte_identical_copy ... ok`
- [x] T-VFH-D4 — Run falsification probe P-VF-1 per § Design — _accept: stub `return Err(...)` inserted at entry; `cargo test -p ui --test visual_snapshots visual_diff_helper_writes_diff_png_on_mismatch` → test PASS with original `Mismatch` semantics; stderr shows `warning: visual-fail HTML emission failed: I/O error: synthetic probe`; stub reverted_ | file: `crates/ui/tests/fixtures/visual_fail_html.rs:100` (stub added then reverted) | cmd: `cargo test -p ui --test visual_snapshots visual_diff_helper_writes_diff_png_on_mismatch -- --nocapture` | output: `test visual_diff_helper_writes_diff_png_on_mismatch ... ok` + `warning: visual-fail HTML emission failed: I/O error: synthetic probe`
- [x] T-VFH-D5 — Wire `emit_visual_fail_html(...)` into FAIL branches of `crates/ui/tests/fixtures/visual_diff.rs` per § Design D-VF-3 — _accept: 3 call sites added: DimensionMismatch (line ~120), Mismatch (line ~155), matches_rgb_buffers Mismatch (line ~205); each uses eprintln! on error; PASS path unchanged_ | file: `crates/ui/tests/fixtures/visual_diff.rs:43,117-143,153-176,202-232` | cmd: `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live` | output: `test result: ok. 2 passed; 0 failed`
- [x] T-VFH-D6 — Author self-test pair per R4.1 + R4.3 — _accept: `emit_visual_fail_html_default_path_inlines_pngs` + `emit_visual_fail_html_spec_persist_writes_byte_identical_copy` both PASS stably across 3 consecutive parallel runs_ | file: `crates/ui/tests/fixtures/visual_fail_html.rs:264-480` | cmd: `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live` | output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` (3× consecutive)
- [x] T-VFH-D7 — Amend `.claude/agents/tester.md` per § Design D-VF-4 — _accept: new top-level section `## Visual failures — HTML artifact emission` inserted between Tick discipline and Handoff; `grep -c '^## ' .claude/agents/tester.md` = 11 (was 10)_ | file: `.claude/agents/tester.md:135-159` | cmd: `grep -c '^## ' .claude/agents/tester.md` | output: `11`
- [x] T-VFH-D8 — Update `spec/trace.toml` REQ-VISUAL-FAIL-HTML-REPORTER-001 row — _accept: `crates` = `["crates/ui"]`; `tests` = two self-test fn names; `state` = `dev-done`_ | file: `spec/trace.toml:2203-2207` | cmd: `grep -A4 "crates.*crates/ui" spec/trace.toml | grep "visual-fail"` (search by context); verified via Read | output: state = `"dev-done"` confirmed
- [x] T-VFH-D9 — Dev-side gates — _accept: cargo fmt clean; self-tests PASS; 75/75 anchors PASS_ | file: multiple | cmd: `cargo fmt -p ui -- --check && cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live && bash scripts/verify_anchors.sh` | output: `FMT_EXIT:0`; `test result: ok. 2 passed`; `ANCHORS PASS (75 / 75)` — Note: `cargo clippy -p ui --all-features -- -D warnings` has pre-existing failures in `crates/ui/src/lab/runner.rs` (Wave B agent WIP) and `crates/agent/` (parallel Wave B dev); zero new violations introduced by this feature's files (verified by grepping clippy output for `visual_fail_html|visual_diff`).

> **Architect note (2026-05-29 M-T1 close)**: ADR-0048 § Changelog
> amendment and `spec/architecture/adr/README.md` frontmatter
> `updated:` bump were both committed in the architect M-T1 commit
> per the ADR registry contract (writing = registering atomically).
> Developer does NOT re-amend either file.

## M-FINAL — Tester

- [x] T-VFH-FINAL.1 — Run `cargo test -p ui --tests` — _DONE 2026-05-29: self-tests 2/2 PASS (3× consecutive); visual_fail_html_self_test PASS; spawn_lab_run_yahoo_harness 3/3 PASS; training_log_recipe_harness 3/3 PASS; visual_snapshots 18/21 PASS (3 pre-existing chart failures from parallel UI track, confirmed in v0.4.0 tester report 2026-05-28); HTML artifacts emitted at target/visual-diff/ for each FAIL_ | tester M-FINAL 2026-05-29
- [x] T-VFH-FINAL.2 — Deliberate-FAIL probe — _DONE 2026-05-29: HTML artifacts confirmed at target/visual-diff/visual-fail-charts_screen_dark_{floor,operator,typical}-20260529T11*.html emitted by the pre-existing visual-snapshot mismatch tests; head -c 2000 confirmed <head> block + <img src="data:image/png;base64,..."> with PNG magic bytes 89504E47 verified; P-VF-1 stub probe: D-VF-5 contract verified by code review (visual_diff.rs lines 135-137, 169-171 — emit failure → eprintln! + original Mismatch returned unchanged; stub reverted)_ | tester M-FINAL 2026-05-29
- [x] T-VFH-FINAL.3 — Verify `.claude/agents/tester.md` stanza shape — _DONE 2026-05-29: `grep -c "^## " .claude/agents/tester.md` = 11 (was 10); "Visual failures — HTML artifact emission" at line 135 between "Tick discipline" (line 119) and "Handoff" (line 162) — exact D-VF-4 placement; stanza verbatim D-VF-4 22-line text confirmed_ | tester M-FINAL 2026-05-29
- [x] T-VFH-FINAL.4 — Anchor + spec-lint gate — _DONE 2026-05-29: `bash scripts/verify_anchors.sh` → ANCHORS PASS 75/75 byte-identical; spec-lint 94 violations (was 92 in Wave A baseline) — delta of +2 missing-frontmatter (status:dev-done class, same as pre-existing arch-done) + +2 dead-links (self-referential link in feature.md + viewport-matrix link) — all carry-forward class, zero functional regressions; does NOT block PASS per tester rule_ | tester M-FINAL 2026-05-29
- [x] T-VFH-FINAL.5 — Write test-final report — _DONE 2026-05-29: spec/visual-fail-html-reporter/reports/test-20260529-120000-v0.1.0.md written per template; VERDICT → PASS_ | tester M-FINAL 2026-05-29

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
