---
slug: visual-fail-html-reporter
mode: release
status: draft
audience: human-operator
updated: 2026-06-16
generated: 2026-06-16T00:00:00Z
---

# Visual-fail HTML reporter — release

## TL;DR

When a cockpit visual-snapshot test fails, it now drops a single
self-contained HTML file you open in a browser to see expected /
actual / pixel-diff side by side — no more hunting three PNGs in
Finder. Ready to ship v0.1.0.

## What changed

- **New failure artifact.** On a visual-assertion FAIL, the test
  harness writes `target/visual-diff/<test>-<timestamp>.html` with the
  baseline, the actual render, and the colour-coded pixel diff all
  inlined as images — open it, see the mismatch in one click.
- **Test-only, additive, zero production impact.** Lives entirely
  under `crates/ui/tests/` (dev-dependency `base64`); on a PASS it
  writes nothing — the existing fast path is byte-for-byte unchanged.
  The old standalone PNGs are still emitted too.
- **Tester-contract trim.** `.claude/agents/tester.md` gained a stanza
  telling the tester to cite the HTML path instead of re-typing what
  the PNGs show (~30 prose lines → ~3 lines + a path).

## Why

When a cockpit screenshot test failed, the operator had to open three
separate files by hand — the baseline ("what should render"), the
actual render, and the diff — and mentally line them up to find the
mismatch (~5 min per failure). This feature closes that gap, surfaced
by the `ui-testability-deep-dive-2026-05-15 § 4.1` agent-contract
analysis: the failing test now emits one browser-openable report with
all three images plus the exact assertion (file:line + message)
inlined, turning a five-minute triangulation into a single click.
See `spec/visual-fail-html-reporter/feature.md` § Why.

## Timely context

The cockpit's visual-regression gate was **de-flaked earlier today**
(commit `730dc5d` — a multithread `set_var` data race in the
visual-regression gate). This reporter is the tool that makes the next
failure of that same load-bearing gate fast to triage: when it trips,
you get the HTML report instead of raw PNG paths.

## What you can do now

| Action | Command |
|--------|---------|
| Run the cockpit visual-snapshot tests (an HTML report appears under `target/visual-diff/` on any FAIL) | `cargo test -p ui --test visual_snapshots` |
| Open the most recent failure report in a browser | `open "$(ls -t target/visual-diff/*.html \| head -1)"` |
| Persist a failure report into the feature's `spec/<slug>/reports/` for an investigation (opt-in; OFF by default) | `EMIT_VISUAL_FAIL_TO_SPEC=1 VISUAL_FAIL_SPEC_SLUG=<slug> cargo test -p ui --test visual_snapshots` |
| Re-verify the reporter's own self-tests | `cargo test -p ui --test visual_fail_html_self_test` |

## Live demo

This is backend test infrastructure with no runnable bin — the
ground-truth demo is the reporter's own self-test pair, which
exercises the real emit path (synthetic baseline/actual/diff PNGs →
HTML → asserts the base64 images + assertion text + section headers
are present). **Re-verified by the orchestrator, 2026-06-16** (this
deck does not re-run cargo):

```
$ cargo test -p ui --test visual_fail_html_self_test
running 2 tests
test fixtures::visual_fail_html::tests::emit_visual_fail_html_spec_persist_writes_byte_identical_copy ... ok
test fixtures::visual_fail_html::tests::emit_visual_fail_html_default_path_inlines_pngs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Notice the two tests. The `default_path` test proves a FAIL emits HTML
with the PNGs inlined as `data:image/png;base64,…`; the `spec_persist`
test proves the opt-in `EMIT_VISUAL_FAIL_TO_SPEC=1` copy is
byte-identical to the `target/` copy. What the emitted HTML looks like
(from the real template in
`crates/ui/tests/fixtures/visual_fail_html.rs`, abbreviated):

```html
<h1>Visual fail &mdash; charts_screen_dark_operator <small>20260616T...Z</small></h1>
<section class="meta"><h2>Assertion</h2>
  <pre>crates/ui/tests/visual_snapshots.rs:148

PNG byte mismatch: 47 of 25920000 pixels differ</pre></section>
<section class="baseline"><h2>Baseline (what should render)</h2>
  <img src="data:image/png;base64,iVBORw0KGgo...">  ...</section>
<section class="actual"><h2>Actual (what rendered instead)</h2>   ...</section>
<section class="diff"><h2>Perceptual diff (image-compare hybrid SSIM)</h2> ...</section>
```

## Screenshots

_n/a — backend test-infrastructure feature (no `## UI` heading, no
`screenshots/` dir). The artifact this feature produces is itself an
HTML file the operator opens in a browser; there is no cockpit
surface to capture._

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| R1 — helper + FAIL-only emission | `emit_visual_fail_html(...)` exists in `crates/ui/tests/fixtures/visual_fail_html.rs`; called only from `matches_screenshot` FAIL branches; PASS writes nothing | VERIFIED | Self-test `emit_visual_fail_html_default_path_inlines_pngs` → ok (orchestrator re-verify 2026-06-16); helper source confirmed present |
| R2 — inline-PNG self-containment | PNGs emitted as `data:image/png;base64,…` `<img>` tags; three stacked sections + assertion `<pre>`; inline `<style>`, zero external CSS/JS | VERIFIED | Self-test asserts `html.contains("data:image/png;base64,")` + base64 of baseline PNG + section `<h2>` headers + `8 &times; 8 px` dims → ok |
| R3 — tester contract amendment | New `## Visual failures — HTML artifact emission` section in `.claude/agents/tester.md`, additive | VERIFIED | Archived tester PASS (2026-05-29): tester.md section count 10 → 11; D-VF-4 stanza present |
| R4 — self-test (V9-style) | Self-test pair under `tempfile::TempDir`; second variant exercises `EMIT_VISUAL_FAIL_TO_SPEC=1` byte-identical copy | VERIFIED | Both self-tests → ok (orchestrator re-verify 2026-06-16: 2 passed, 0 failed); archived tester ran 3× consecutive |
| R-NR — non-regression / anchors | PASS path byte-identical; helper produces only `target/` output on FAIL; anchor gate unchanged | VERIFIED | Archived tester PASS: anchors 75/75 byte-identical; base64 PNG magic-bytes verified in emitted artifacts; D-VF-5 (emit-failure does not mask original `Mismatch`) confirmed by code review |
| K3 — parallel-write race | ISO-8601 timestamp + test_name in filename guarantees unique paths; self-tests serialize env mutation via `static ENV_LOCK: Mutex<()>` | VERIFIED | Self-tests pass under Cargo's default multithread runner; `ENV_LOCK` present in source |

> Note: `feature.md § Verification` was left as a tester-fill stub; the
> matrix above is reconstructed from the feature's R-items, the
> archived 2026-05-29 tester VERDICT → PASS (report archived under
> `spec/archive/tester-reports-2026-05-to-06.tar.gz`; no `test-*.md` in
> `spec/visual-fail-html-reporter/reports/`), and the orchestrator's
> 2026-06-16 re-verification.

## Numbers that matter

- **Self-tests:** 2 passed, 0 failed (orchestrator re-verify 2026-06-16;
  archived tester ran them 3× consecutive at PASS).
- **Anchors:** 75/75 PASS at tester-done (2026-05-29), byte-identical
  pre/post. Feature's anchor delta is **0** by contract — it touches
  no anchored report and writes nothing on a test PASS. (This deck does
  not re-run `verify_anchors.sh`; the gate has since grown to 87 rows
  for unrelated backtest features, but this feature contributes none.)
- **Helper size:** ~80 LoC (hypothesis H1: ≤ 80 — met).
- **Operator time per visual FAIL:** ~5 min triangulation → one click
  (hypothesis H2).
- **Production code changed:** 0 lines — `base64` is a
  `[dev-dependencies]` entry only; zero new production-crate deps, zero
  design tokens, zero iced-widget changes.
- **spec-lint:** 69 violations in 2 categories (carry-forward backlog;
  zero introduced by this deck).

## Open decisions

1. **Ship v0.1.0?** This is the single load-bearing decision. The
   feature is tester-PASS (2026-05-29) and orchestrator-re-verified
   (2026-06-16). Approving commits you to nothing beyond the ship — no
   anchor re-lock, no manual capture, no follow-up cost (PASS path is
   byte-identical; the artifact is gitignored under `target/`).

## Approval

- [x] Approved — ship — operator, 2026-06-16
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-06-16 (presenter): initial release deck. Assembled from
  `spec/visual-fail-html-reporter/feature.md` (§ Why / Requirements /
  Design / Implementation), the emit source
  `crates/ui/tests/fixtures/visual_fail_html.rs`, the archived
  2026-05-29 tester VERDICT → PASS (anchors 75/75; self-tests 2/2 ×3;
  base64 PNG magic-bytes verified), and the orchestrator's 2026-06-16
  re-verification (`cargo test -p ui --test visual_fail_html_self_test`
  → 2 passed, 0 failed). Awaiting operator approval.
