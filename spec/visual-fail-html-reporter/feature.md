---
slug: visual-fail-html-reporter
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-29
---

# Visual-fail HTML reporter — v0.1.0

> **Pick A Wave 1 promoted feature.** Per
> [`spec/dev-notes/pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md)
> this is the cheapest of the three trifecta pillars (~1 dev day),
> biased toward DURABLE: a single tester-contract amendment + ~50 LoC
> helper that lights up agent-facing failure artifacts for every visual
> assertion FAIL across the project.

## Why

The
[`ui-testability-deep-dive-2026-05-15.md § 4.1`](../dev-notes/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose)
analysis named a **direct agent-contract gap**: when a visual assertion
fails inside `crates/ui/tests/*.rs` (via the existing
`tests/fixtures/visual_diff.rs::matches_screenshot` helper), the
operator sees prose like "PNG mismatch at viewport 1920×1080" but has
to manually open three separate files in Finder/Preview
(`target/visual-diff/<test>-actual.png`,
`target/visual-diff/<test>.png` diff, baseline PNG under
`crates/ui/tests/visual-baselines/`) to triangulate the failure. The
tester then writes prose into the test-final report repeating what the
PNGs already show.

This brief closes that gap with **default-FAIL emission of a
self-contained `visual-fail-<ts>.html` artifact** showing:

1. The baseline PNG (top of page; "what should render").
2. The actual PNG (middle; "what rendered instead").
3. The perceptual-diff PNG from `image-compare` (bottom; "where the
   mismatch is, colour-coded").
4. The assertion that fired (file:line, message body).
5. Optional VLM verdict (if `ui-vlm-judge` ever lands and is enabled
   shadow-mode).

Three layered consequences:

- **Operator turns one click on the HTML into the same artefact the
  tester would have written by hand** — saves ~5 min per visual FAIL
  cycle.
- **Tester contract amendment** (`.claude/agents/tester.md`) replaces
  the ad-hoc prose-only `## Visual failures` stanza with a "default-
  emit the HTML; cite the file path in the test-final report" rule.
  Tester writes ~3 lines instead of ~30.
- **Cross-cuts every future UI feature.** Once shipped, any new
  `matches_screenshot(...)` call site inherits the HTML emission
  without per-call wiring — the helper does it.

Per `process-tooling-survey-2026-05-29.md § Top-5 deep-dives Rank 2`:
LARGE per-cycle benefit, SMALL investment cost (~1 day), LOW
maintenance burden (single helper + `.claude/agents/tester.md` stanza
amended once).

## Requirements

### R1 — Helper module + emission on FAIL only

- **R1.1** A new helper function `emit_visual_fail_html(...)` (or
  `write_visual_fail_html(...)`) lives in
  [`crates/ui/tests/fixtures/`](../../crates/ui/tests/fixtures/) —
  most likely as a new sibling to
  [`visual_diff.rs`](../../crates/ui/tests/fixtures/visual_diff.rs).
  Module path: `crates/ui/tests/fixtures/visual_fail_html.rs`
  (architect M-T1 ratifies filename).
- **R1.2** The helper is called ONLY from the FAIL path of
  `matches_screenshot(...)`. On PASS, ZERO file output (no HTML emit,
  no perceptual-diff PNG either — preserves the existing `Ok(())`
  fast path).
- **R1.3** The helper signature accepts:
  - `test_name: &str` (e.g. `"charts_screen_dark_operator"`)
  - `assertion_location: &str` (e.g. `"crates/ui/tests/visual_snapshots.rs:148"` — file:line of the calling assertion)
  - `assertion_body: &str` (e.g. `"PNG byte mismatch: 47 of 25920000 pixels differ"`)
  - `baseline_png_path: &Path`
  - `actual_png_path: &Path`
  - `diff_png_path: &Path`
  - `optional_vlm_verdict: Option<&str>` (default `None`; future `ui-vlm-judge` hook)
- **R1.4** Output path default:
  `target/visual-diff/<test_name>-<ISO-8601-ts>.html` (gitignored
  via existing `target/` rule — no `.gitignore` delta).
- **R1.5** **Opt-in spec-persist:** when env var
  `EMIT_VISUAL_FAIL_TO_SPEC=1` is set, helper additionally writes a
  byte-identical copy to
  `spec/<slug>/reports/visual-fail-<test_name>-<ts>.html`. Default OFF
  to keep repo size bounded (per Risk R4 in trifecta direction).
  Architect M-T1 ratifies slug-derivation: most likely
  `CARGO_PKG_NAME` or an `SPEC_SLUG` env var the tester sets.

### R2 — Inline-PNG self-containment

- **R2.1** PNGs are inlined as `data:image/png;base64,...` in `<img
  src=...>` tags so the HTML file opens correctly when copied/moved
  away from the original PNG locations. No external PNG references.
- **R2.2** Page layout: three sections stacked top-to-bottom
  (baseline → actual → diff), each labeled `<h2>` with the section
  name + PNG dimensions; below all three, a `<pre>` block with
  assertion-location + assertion-body verbatim.
- **R2.3** Page is self-contained — zero external CSS/JS. Inline
  `<style>` block at minimum: 100% page width, max PNG width 1920px
  with `object-fit: contain`, dark background to make rendered text
  visible regardless of system theme.
- **R2.4** Optional VLM verdict block appears between assertion
  section and end-of-page when supplied; absent when `None`.

### R3 — Tester contract amendment

- **R3.1** `.claude/agents/tester.md` gains a new stanza (~5-10 lines)
  titled **`## Visual failures — HTML artifact emission`** explaining:
  (a) the `matches_screenshot` helper emits HTML on FAIL automatically;
  (b) tester cites the path in the test-final report `## Visual
  failures` section rather than re-describing the PNG content;
  (c) `EMIT_VISUAL_FAIL_TO_SPEC=1` is the opt-in for spec-persist.
- **R3.2** The amendment is **additive** — no removed prose, no
  contract weakening. Tester still names the failing test + file:line
  + assertion body in the report's prose. Only the visual-content
  description gets delegated to the HTML artifact.

### R4 — Test self-test (V9-style)

- **R4.1** A new test in `crates/ui/tests/fixtures/visual_fail_html.rs`
  (or a sibling test file) exercises the emit path with synthetic
  baseline/actual/diff PNG triples and asserts the emitted HTML
  contains the inlined base64 + assertion text + section headers.
- **R4.2** The self-test uses `tempfile::TempDir` so it leaves no
  state behind.
- **R4.3** A second self-test variant exercises `EMIT_VISUAL_FAIL_TO_SPEC=1`
  with a fake slug path under `tempfile::TempDir`; asserts the second
  HTML file appears byte-identical to the first.

### R-NR — Non-regression contract

- **R-NR.1** Existing `matches_screenshot(...)` PASS path stays
  byte-identical — zero new code reachable on the `Ok(())` branch.
  Acceptance: existing visual-snapshot tests in
  `crates/ui/tests/visual_snapshots.rs` stay PASS with zero new files
  emitted on PASS.
- **R-NR.2** `bash scripts/verify_anchors.sh` → 71/71 PASS byte-identical
  pre/post-merge. Helper produces only `target/` outputs on FAIL; on PASS
  produces nothing.
- **R-NR.3** Zero new design tokens, zero `strings.rs` adds, zero
  iced widget code changes — backend test infrastructure only.
- **R-NR.4** Zero new `Cargo.toml` dependency on production crates. The
  helper may use `base64 = "0.22"` (or equivalent) under
  `[dev-dependencies]` if needed for inline PNG encoding. Architect
  M-T1 picks crate (stdlib hex is the cheap option but produces 2× the
  payload).
- **R-NR.5** Existing `target/visual-diff/<test>.png` and
  `<test>-actual.png` continue to be emitted as today (forensic PNGs
  the operator can open standalone). The HTML is **additive**, not
  replacing.

## Falsifiers (K)

- **K1 — HTML payload exceeds 30 MB per file at 3360×1890 × 2.0 scale.**
  Three PNG triples each ~10 MB → ~30 MB inline-base64. If observed,
  helper compresses with `image::codecs::png` settings tweak or
  switches diff PNG to JPEG. Architect M-T1 sets quality vs size
  ceiling.
- **K2 — `EMIT_VISUAL_FAIL_TO_SPEC=1` accidentally enabled in CI.**
  Spec/ folder bloats by tens of MB per regression run. Mitigation:
  env var is **explicit operator action only**, default OFF, tester
  never sets it in CI workflows.
- **K3 — Helper's emit-on-FAIL races with cargo test parallelism.**
  Two failing tests writing HTML to the same path simultaneously
  garble the file. Mitigation: ISO-8601 timestamp + test_name in
  filename ensures uniqueness (test_name is per-test; ts is per-run);
  no collision possible.
- **K4 — `.claude/agents/tester.md` stanza drift with viewport-matrix
  feature's parallel amendment.** Both Wave 1 features may touch the
  same file. Mitigation per trifecta direction § Risk R1: visual-fail-
  HTML ships FIRST, viewport-matrix inherits the stanza without
  amendment.

## Hypotheses (H)

- **H1 — Helper LoC ≤ 80** (single function + inline `<style>` HTML
  template + base64 encode loop + `fs::write`). Matches the analyst's
  "~50 LoC helper" estimate at the survey level.
- **H2 — One operator visual review per visual FAIL** replaces the
  current 5-min triangulation across three PNG files. Per-cycle benefit
  measurable as "tester report § Visual failures shrinks from ~30
  lines per FAIL to ~3 lines + HTML path".
- **H3 — Zero existing tests break.** PASS path is structurally
  unchanged; FAIL path only adds emission, doesn't change the existing
  `Err(VisualDiffError::...)` return.

## Operator decisions

### Q1 — HTML output path default

**Q.** Where do failed-test HTML reports land by default?

**(Recommended — DURABLE) Option A — `target/visual-diff/<test>-<ts>.html`,
opt-in spec-persist.** Mirrors existing forensic PNG location; gitignored
by default; `EMIT_VISUAL_FAIL_TO_SPEC=1` env var promotes to
`spec/<slug>/reports/`. No repo-size risk; operator picks persist when
investigation warrants.

**Cost.** Zero `.gitignore` delta; one helper-fn branch on env var.

**Option B (cheap fallback).** Always emit to
`spec/<slug>/reports/visual-fail-<ts>.html`. Cheaper to wire (no env
branch). **Rejected** per Risk R4 — repo grows fast on visual-test
churn; every spurious FAIL leaves a 30 MB artifact committed somewhere.

**Default**: A (Recommended DURABLE).

### Q2 — Base64 encoding crate

**Q.** What encodes the PNG bytes to inline-base64?

**(Recommended — DURABLE) Option A — `base64 = "0.22"` dev-dep.**
Industry-standard crate; ~3 lines of code; URL-safe alphabet by
default; zero maintenance burden (crate is stable).

**Cost.** One `[dev-dependencies]` entry in `crates/ui/Cargo.toml`.

**Option B (cheap fallback).** Hex-encode (`std::fmt::Write`,
`hex::encode`-equivalent). 2× the payload size; HTML inline-data URIs
support hex but base64 is canonical. **Rejected** — saves a dep but
doubles K1's payload risk. Not durable.

**Default**: A (Recommended DURABLE).

### Q3 — Tester contract stanza placement

**Q.** Where in `.claude/agents/tester.md` does the new "Visual
failures — HTML artifact emission" stanza go?

**(Recommended — DURABLE) Option A — append to existing visual-
failure prose section** as a sub-stanza. Preserves existing tester
prose contract; analyst doesn't need to refactor anything around it.

**Option B (cheap fallback).** New top-level section. Slightly higher
visibility but risks contract amendment scope-creep into a re-org of
the whole file. **Rejected.**

**Default**: A (Recommended DURABLE).

## Verdict tree (pre-drawn)

| Q1 \ Q2 | Q2=(a) base64 dev-dep | Q2=(b) hex |
|---|---|---|
| **Q1=(a) target/ + opt-in spec** | **DURABLE — Recommended.** Zero repo bloat; standard encoding; ships clean. | INCONSISTENT — durable path with cheap encoding. Operator-override only. |
| **Q1=(b) always-spec** | REJECTED — repo bloat at scale. | REJECTED — repo bloat AND payload bloat. |

## Design

_(architect M-T1 fills D-VFH-1 through D-VFH-N here. Expected fast-skip
— no new ADR needed; ADR-0048 carries forward for visual-fail emission
shape, and the existing bootstrap visual-diff helper at
`crates/ui/tests/fixtures/visual_diff.rs` is the structural precedent.)_

## Implementation

_(developer fills after architect M-T1 ratifies the helper module
location + base64 crate pick + tester.md stanza shape.)_

## Verification

_(tester M-FINAL links the test-final report + the self-test outputs
+ confirms `.claude/agents/tester.md` stanza appears as ratified +
confirms `verify_anchors.sh` 71/71 PASS byte-identical pre/post.)_

## Changelog

- 2026-05-29 (analyst): M0 brief authored under Pick A Wave 1 promotion
  per [`pick-a-test-infra-trifecta-2026-05-29.md`](../dev-notes/pick-a-test-infra-trifecta-2026-05-29.md).
  R1 helper contract + R2 self-containment + R3 tester.md stanza +
  R4 self-test + R-NR (5 clauses including K-class K1-K4 + H1-H3 +
  Q1-Q3 + pre-drawn verdict tree. ~1 dev day estimate. Trace row
  `REQ-VISUAL-FAIL-HTML-REPORTER-001` opened at `proposed`. HANDOFF →
  architect (M-T1 fast-skip expected; ADR-0048 carries forward + the
  existing `crates/ui/tests/fixtures/visual_diff.rs` is the structural
  precedent for FAIL-only emission).
