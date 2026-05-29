---
slug: visual-fail-html-reporter
status: arch-done
owner: developer
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

- [ ] T-VFH-D1 — Add `base64 = "0.22"` under `crates/ui/Cargo.toml [dev-dependencies]` — _accept: `cargo check -p ui --tests` succeeds; `[dev-dependencies]` table only (no production change); existing `0.22.1` in `Cargo.lock` resolves cleanly (no version bump)_
- [ ] T-VFH-D2 — Author `crates/ui/tests/fixtures/visual_fail_html.rs` per § Design D-VF-1 + D-VF-2 — _accept: file exists; exports `pub struct VisualFailContext<'a>` (7 fields per D-VF-2), `pub fn emit_visual_fail_html(VisualFailContext<'_>) -> Result<PathBuf, VisualFailHtmlError>`, `pub enum VisualFailHtmlError { Io(io::Error), Image(image::ImageError) }` with `Display`/`Error` impls; HTML template matches D-VF-1 skeleton (head + meta + assertion + baseline + actual + optional diff + optional vlm); inline `<style>` block per D-VF-1 CSS minimum; base64 encode via `base64::engine::general_purpose::STANDARD.encode(&png_bytes)`; PNG dimensions read via `image::ImageReader::open(path)?.into_dimensions()?`; default output path `target/visual-diff/<test_name>-YYYYMMDDTHHMMSSZ.html` via `chrono::Utc::now().format("%Y%m%dT%H%M%SZ")`; ≤ 80 LoC excluding error enum + Display impl_
- [ ] T-VFH-D3 — Implement env-var-gated spec-persist in `emit_visual_fail_html(...)` per § Design Q1 ratification — _accept: when `EMIT_VISUAL_FAIL_TO_SPEC=1` AND `VISUAL_FAIL_SPEC_SLUG=<slug>` both set, helper additionally writes byte-identical HTML to `spec/<slug>/reports/visual-fail-<test_name>-<ts>.html`; when `EMIT_VISUAL_FAIL_TO_SPEC=1` set but `VISUAL_FAIL_SPEC_SLUG` missing, helper emits `eprintln!("warning: EMIT_VISUAL_FAIL_TO_SPEC=1 set but VISUAL_FAIL_SPEC_SLUG missing; spec-persist skipped")` and writes only the `target/` copy; default (neither var set) writes only `target/` copy_
- [ ] T-VFH-D4 — Run falsification probe P-VF-1 per § Design — _accept: temporarily edit `emit_visual_fail_html` to `return Err(VisualFailHtmlError::Io(io::Error::other("synthetic probe")))` at function entry; run `cargo test -p ui --test visual_diff visual_diff_helper_writes_diff_png_on_mismatch`; confirm test STILL asserts `Err(VisualDiffError::Mismatch { .. })` PASS (original `Mismatch` semantics preserved); grep stderr for `warning: visual-fail HTML emission failed`; revert synthetic Err; record probe outcome in feature.md § Implementation_
- [ ] T-VFH-D5 — Wire `emit_visual_fail_html(...)` into FAIL branches of `crates/ui/tests/fixtures/visual_diff.rs` per § Design D-VF-3 — _accept: 3 call sites added: (a) `matches_screenshot` `DimensionMismatch` branch at `visual_diff.rs:115` with `diff_png_path: None`; (b) `matches_screenshot` `Mismatch` branch at `visual_diff.rs:131` with `diff_png_path: Some(&diff_path(test_name))`; (c) `matches_rgb_buffers` Mismatch branch at `visual_diff.rs:198` similar shape; each call uses `eprintln!` to log emission errors but does NOT alter the `VisualDiffError` return; PASS branch of `matches_screenshot` at line 127 unchanged (zero new code reachable on `Ok(())`)_
- [ ] T-VFH-D6 — Author self-test pair per R4.1 + R4.3 (§ Design D-VF-1 schema assertions) — _accept: new `#[test] fn` in `crates/ui/tests/fixtures/visual_fail_html.rs` under `#[cfg(test)] mod tests`: (1) `emit_visual_fail_html_default_path_inlines_pngs` — drives helper with synthetic 8×8 baseline/actual/diff PNGs under `tempfile::TempDir`, asserts emitted HTML contains `data:image/png;base64,`, the assertion-location string, all three section `<h2>` headers, and PNG dimension `8 × 8 px`; (2) `emit_visual_fail_html_spec_persist_writes_byte_identical_copy` — sets `EMIT_VISUAL_FAIL_TO_SPEC=1` + `VISUAL_FAIL_SPEC_SLUG=test-slug` via `std::env::set_var` in test (scoped to test thread), invokes helper, asserts second file at `<TempDir>/spec/test-slug/reports/visual-fail-...html` exists AND is byte-identical to the `target/`-side copy; uses `std::env::remove_var` cleanup or scoped guard pattern_
- [ ] T-VFH-D7 — Amend `.claude/agents/tester.md` per § Design D-VF-4 — _accept: new top-level section `## Visual failures — HTML artifact emission` inserted between `## Tick discipline (T_FINAL ownership)` (ends line 133) and `## Handoff` (currently starts line 135); stanza text matches D-VF-4 verbatim (~22 lines); no other tester.md prose touched; `grep -c '^## ' .claude/agents/tester.md` increases by exactly 1_
- [ ] T-VFH-D8 — Update `spec/trace.toml` REQ-VISUAL-FAIL-HTML-REPORTER-001 row — _accept: `crates` array populated with `crates/ui` (Cargo.toml dev-dep + fixtures helper + visual_diff.rs wire-up + .claude/agents/tester.md amendment); `tests` array populated with self-test fn names (`emit_visual_fail_html_default_path_inlines_pngs`, `emit_visual_fail_html_spec_persist_writes_byte_identical_copy`); state transitions `arch-done → dev-done` (developer M-DEV close) — actual transition `dev-done → passed` ticked by tester at M-FINAL_
- [ ] T-VFH-D9 — Dev-side gates — _accept: `cargo test -p ui --tests` PASS (existing + new self-tests); `cargo clippy -p ui --all-features -- -D warnings` clean; `bash scripts/verify_anchors.sh` → 71/71 PASS byte-identical pre/post; `python3 scripts/spec_lint.py` exit 0 (no new violations)_

> **Architect note (2026-05-29 M-T1 close)**: ADR-0048 § Changelog
> amendment and `spec/architecture/adr/README.md` frontmatter
> `updated:` bump were both committed in the architect M-T1 commit
> per the ADR registry contract (writing = registering atomically).
> Developer does NOT re-amend either file.

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
