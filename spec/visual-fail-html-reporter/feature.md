---
slug: visual-fail-html-reporter
version: 0.1.0
status: dev-done
owner: tester
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

> **Architect M-T1 ratification — 2026-05-29.** Q1 + Q2 ratified at
> analyst's (a) DURABLE pick. Q3 **overridden** — `.claude/agents/tester.md`
> has no pre-existing "Visual failures" prose section to append to (see
> [D-VF-4](#d-vf-4--testermd-amendment-stanza-text-q3-overridden)
> rationale). New top-level `## Visual failures — HTML artifact emission`
> section added between `## Tick discipline (T_FINAL ownership)` and
> `## Handoff`. Same DURABLE bias; just no existing stanza to append to.
> No new ADR; ADR-0048 § Changelog amendment only (per
> [D-VF-6](#d-vf-6--adr-amendment-no-new-adr)).

### Q1 ratified — output path default

**Q1 (a) RATIFIED.** Default emission target is
`target/visual-diff/<test_name>-<ts>.html` (gitignored via existing
`target/` rule; no `.gitignore` delta required). Opt-in spec-persist
controlled by env var `EMIT_VISUAL_FAIL_TO_SPEC=1` (locked name per
R1.5; matches analyst's brief verbatim). Spec-persist target path
pattern: `spec/<slug>/reports/visual-fail-<test_name>-<ts>.html`.

**Slug derivation** (analyst left this open): the helper reads env
var `VISUAL_FAIL_SPEC_SLUG` if set; otherwise no spec-persist
fires even if `EMIT_VISUAL_FAIL_TO_SPEC=1` (logs a stderr warning).
The tester sets `VISUAL_FAIL_SPEC_SLUG=<current-feature-slug>` when
deliberately investigating a failure. Rationale: `CARGO_PKG_NAME`
would always resolve to `ui` (the crate being tested), which is NOT
the feature slug — operator wants the report under the FEATURE's
`spec/<slug>/reports/`, not under `spec/ui/reports/`. Explicit env
var is the only correct disambiguator.

**Timestamp format** (analyst said "ISO-8601-ts"): use
`YYYYMMDDTHHMMSSZ` UTC (compact RFC 3339 basic format — no `:`
separators since macOS Finder treats colons specially in filenames).
Helper derives via `chrono::Utc::now().format("%Y%m%dT%H%M%SZ")`.

### Q2 ratified — base64 crate

**Q2 (a) RATIFIED.** `base64 = "0.22"` added under
`crates/ui/Cargo.toml [dev-dependencies]`. Library compatibility
checklist (per `architect.md`):

| Check | Status | Evidence |
|---|---|---|
| Single-binary friendly | PASS | Pure Rust, zero infra |
| No system C deps | PASS | No build.rs, no `*-sys` deps |
| Edition 2024 compatible | PASS | Already transitively present at `0.22.1` in `Cargo.lock` (16 sites; pulled by `reqwest`, `aws-*`, `rustls-*`, `iced_test`'s transitive tree) — workspace compiles on edition 2024 today |
| `[package] name` no stdlib shadow | PASS | `base64` is not a stdlib crate name |
| Maintained | PASS | `0.22.0` released 2024-03; `0.22.1` released 2024-04 — within 18-month freshness window from today (2026-05-29) |
| License compatible | PASS | MIT OR Apache-2.0 (matches workspace policy) |

**Decision**: chosen — `base64 = "0.22"`. Rejected alternatives:
manual hex-encode (Q2 fallback, 2× payload size, raises K1 risk),
`base64-url` crate (less popular, equivalent semantics but redundant
dep when `base64::engine::general_purpose::STANDARD` already supports
data-URI semantics).

**Encoder pick**: `base64::engine::general_purpose::STANDARD.encode(&png_bytes)`
— the standard alphabet (NOT URL-safe) because HTML data-URIs accept
`+/=` and the standard alphabet is the canonical pick across Chromium /
Safari / Firefox.

### Q3 ratified-with-override — tester.md stanza placement

**Q3 (a) OVERRIDDEN to (b).** Analyst recommendation assumed an
existing visual-failure prose sub-section in `.claude/agents/tester.md`.
**Verified 2026-05-29 by reading the file**: tester.md sections are
`Pre-flight: brief and trace`, `Trace.toml: own the anchors column`,
`Your Responsibilities`, `Workflow Position`, `Output Contract`,
`Skills You Use`, `Spec-lint gate`, `Anchor-verification gate`,
`Tick discipline (T_FINAL ownership)`, `Handoff`. **No existing
"Visual failures" stanza exists**. Q3 (a) is therefore physically
impossible.

Override is to **Q3 (b) — new top-level section**. Placement: insert
new `## Visual failures — HTML artifact emission` section
**between `## Tick discipline (T_FINAL ownership)` (ends at line 133)
and `## Handoff` (starts at line 135)**. Exact stanza text in
[D-VF-4](#d-vf-4--testermd-amendment-stanza-text-q3-overridden).

Rationale for override is mechanical (no existing stanza) not a
durability downgrade — the new section is single-purpose, ≤ 15
lines, and follows the same prose-then-code-block shape as
neighbouring sections.

### D-VF-1 — HTML schema + template skeleton

The helper emits a single self-contained HTML file. Layout (top to
bottom):

1. `<head>` with `<meta charset=utf-8>`, page title `"Visual fail —
   <test_name> — <ts>"`, inline `<style>` block (no external CSS).
2. `<h1>` with the test name + timestamp.
3. `<section class="meta">` — assertion location (file:line) and
   assertion body in a `<pre>` block (verbatim, including newlines).
4. `<section class="baseline">` — `<h2>Baseline (what should render)</h2>`
   + `<img src="data:image/png;base64,..." alt="baseline">` +
   `<p class="dim">3360 × 1890 px</p>`.
5. `<section class="actual">` — same shape for "Actual (what rendered
   instead)".
6. `<section class="diff">` — same shape for "Perceptual diff
   (image-compare hybrid SSIM)". The diff section is **optional**:
   if `diff_png_path` is `None` (which happens on `VisualDiffError::
   DimensionMismatch` where no perceptual diff is meaningful) the
   section is omitted.
7. `<section class="vlm">` — only emitted if `optional_vlm_verdict`
   is `Some`; renders verdict text in a `<pre>` block under
   `<h2>VLM verdict (shadow mode)</h2>`.

**Inline `<style>` minimum** (per R2.3):

```css
body { background: #1a1a1a; color: #e0e0e0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 1rem; }
h1 { font-size: 1.5rem; border-bottom: 1px solid #444; padding-bottom: 0.5rem; }
h2 { font-size: 1.1rem; margin-top: 2rem; color: #8ab4f8; }
section { max-width: 100%; }
img { max-width: 100%; height: auto; object-fit: contain; display: block; border: 1px solid #333; }
pre { background: #0f0f0f; padding: 0.75rem; overflow-x: auto; font-size: 0.85rem; white-space: pre-wrap; }
.dim { color: #888; font-size: 0.85rem; margin: 0.25rem 0 0; }
```

**Template skeleton** (Rust-side, building blocks in
`visual_fail_html.rs`):

```text
<!DOCTYPE html>
<html lang="en"><head>
  <meta charset="utf-8">
  <title>Visual fail — {test_name} — {ts}</title>
  <style>{STYLE_BLOCK}</style>
</head><body>
  <h1>Visual fail — {test_name} <small>{ts}</small></h1>
  <section class="meta">
    <h2>Assertion</h2>
    <pre>{assertion_location}\n\n{assertion_body}</pre>
  </section>
  <section class="baseline">
    <h2>Baseline (what should render)</h2>
    <img src="data:image/png;base64,{baseline_b64}" alt="baseline">
    <p class="dim">{baseline_w} × {baseline_h} px</p>
  </section>
  <section class="actual">
    <h2>Actual (what rendered instead)</h2>
    <img src="data:image/png;base64,{actual_b64}" alt="actual">
    <p class="dim">{actual_w} × {actual_h} px</p>
  </section>
  {OPTIONAL diff section}
  {OPTIONAL vlm section}
</body></html>
```

Helper writes via `std::format!` macro into a single `String`, then
`fs::write(path, html_string)`. PNG dimensions read via
`image::ImageReader::open(path)?.into_dimensions()?` (already a
workspace dep via `crates/ui`).

### D-VF-2 — helper location and signature

**Helper module**: new file `crates/ui/tests/fixtures/visual_fail_html.rs`
(matches R1.1 analyst suggestion; analyst already named the path in
the brief). NOT a co-location inside `visual_diff.rs` — keeps the
new helper independently testable and avoids growing `visual_diff.rs`
past its current ~345 LoC single-responsibility footprint.

**Function signature** (extends analyst's R1.3 with a context struct
for ergonomics + future extension):

```rust
use std::path::{Path, PathBuf};

/// Context for one visual-fail HTML emission.
///
/// `diff_png_path` is `Option` because `VisualDiffError::DimensionMismatch`
/// has no meaningful perceptual diff (the comparator refuses unequal
/// dimensions). `optional_vlm_verdict` is the v0.2.0+ hook for
/// `ui-vlm-judge`; v0.1.0 always passes `None`.
pub struct VisualFailContext<'a> {
    pub test_name: &'a str,
    pub assertion_location: &'a str,   // "crates/ui/tests/visual_snapshots.rs:148"
    pub assertion_body: &'a str,       // e.g. VisualDiffError::Display output
    pub baseline_png_path: &'a Path,
    pub actual_png_path: &'a Path,
    pub diff_png_path: Option<&'a Path>,
    pub optional_vlm_verdict: Option<&'a str>,
}

/// Emit a self-contained `visual-fail-<ts>.html` next to
/// `target/visual-diff/`. Returns the path written, or `Err` on any
/// I/O or PNG-decode failure.
///
/// When env var `EMIT_VISUAL_FAIL_TO_SPEC=1` is set AND env var
/// `VISUAL_FAIL_SPEC_SLUG=<slug>` is set, the helper additionally
/// writes a byte-identical copy to
/// `spec/<slug>/reports/visual-fail-<test_name>-<ts>.html`.
/// If only `EMIT_VISUAL_FAIL_TO_SPEC=1` is set without
/// `VISUAL_FAIL_SPEC_SLUG`, a warning is logged via `eprintln!` and
/// only the `target/` copy is written.
pub fn emit_visual_fail_html(
    ctx: VisualFailContext<'_>,
) -> Result<PathBuf, VisualFailHtmlError>;

#[derive(Debug)]
pub enum VisualFailHtmlError {
    Io(std::io::Error),
    Image(image::ImageError),
}
```

LoC budget H1: ≤ 80 LoC. Realistic shape: ~30 LoC for the HTML
`format!` template, ~10 LoC for base64 encode + dimensions read,
~15 LoC for path derivation + env var branching, ~15 LoC for error
enum + Display impl. Target ~70 LoC.

### D-VF-3 — trigger contract

**Wire-up site**: extend `crates/ui/tests/fixtures/visual_diff.rs`
`write_diff_artifacts(...)` (line 178) — call
`emit_visual_fail_html(...)` AFTER `write_rgb_diff_artifacts(...)`
succeeds (or fails), so the HTML always emits even when the
perceptual-diff write errors.

Concretely, inside the fail-path branches of `matches_screenshot` —
two call sites:

1. `DimensionMismatch` branch (visual_diff.rs:115) — call
   `emit_visual_fail_html(...)` with `diff_png_path: None`.
2. `Mismatch` branch (visual_diff.rs:131) — call
   `emit_visual_fail_html(...)` with `diff_png_path: Some(&diff_path(test_name))`.

Both call sites assemble `assertion_location` from
`std::panic::Location::caller()` available via the existing
`#[track_caller]` propagation OR (simpler) from the `test_name`
argument and a `file!()` / `line!()` macro at the call site within
`matches_screenshot` itself (the test author's call site is the
caller of `matches_screenshot`, but the failure location string is
acceptable as "matches_screenshot internal call at visual_diff.rs:<line>"
for v0.1.0; the assertion body Display string includes the baseline
path the operator originally passed, which IS the test author's call
site context).

For `matches_rgb_buffers` (the V9 self-test path), the same
emission applies — `emit_visual_fail_html` is reachable from
`write_rgb_diff_artifacts(...)` (visual_diff.rs:198).

**Failure-mode contract** (per D-VF-5): if
`emit_visual_fail_html(...)` returns `Err`, the existing
`VisualDiffError::Mismatch { baseline, diff, actual }` return value
is UNCHANGED. The HTML emission failure logs to stderr via
`eprintln!("warning: visual-fail HTML emission failed: {err}")` and
the helper continues to its existing return value. This preserves
the original failure semantics.

### D-VF-4 — tester.md amendment stanza text (Q3 overridden)

**Placement** (Q3 ratified-with-override per Q3 section above):
new section inserted between `## Tick discipline (T_FINAL ownership)`
(ends at line 133) and `## Handoff` (starts at line 135) of
`.claude/agents/tester.md`. Exact text:

```markdown
## Visual failures — HTML artifact emission

When any test under `crates/ui/tests/` fails a visual assertion
(via the `fixtures::visual_diff::matches_screenshot` or
`matches_rgb_buffers` helpers), the helper automatically emits a
self-contained `visual-fail-<test_name>-<ts>.html` report next to
the existing forensic PNG triple at `target/visual-diff/`. The HTML
inlines the baseline, actual, and perceptual-diff PNGs as
base64 data URIs alongside the assertion location and body — the
operator opens it in Safari/Chrome and sees the full triage view
in one click.

- **Cite the HTML path in your test-final report's "Visual failures"
  section** rather than re-describing what the PNGs show. Example:
  `Visual fail report: target/visual-diff/charts_screen_dark_operator-20260529T143012Z.html`.
- **Opt-in spec-persist**: when the operator wants a durable artifact
  for an investigation, set `EMIT_VISUAL_FAIL_TO_SPEC=1` AND
  `VISUAL_FAIL_SPEC_SLUG=<feature-slug>` before re-running the test;
  the helper writes a byte-identical copy to
  `spec/<slug>/reports/visual-fail-<test_name>-<ts>.html`. Default
  OFF — do NOT set these in CI workflows (per K2 falsifier, spec
  bloats fast otherwise).
- **The HTML is additive.** The existing forensic PNG triple
  (`<test>.png`, `<test>-actual.png`, baseline under
  `crates/ui/tests/visual-baselines/`) continues to be emitted so
  the operator can open each standalone if needed.
```

LoC: ~22 lines including blank lines. Wave 1 sibling
`ui-test-harness-viewport-matrix` INHERITS this stanza without
amendment per the trifecta-direction § Risk R1 mitigation.

### D-VF-5 — failure-mode contract (don't mask original test failure)

If `emit_visual_fail_html(...)` returns `Err` for ANY reason (base64
encode panic, fs::write I/O error, image decode error reading PNG
dimensions), the caller MUST:

1. Log the HTML-emission error to stderr via `eprintln!(...)` (NOT
   `tracing::error!` — tests don't initialise tracing subscribers
   by default and the error would silently disappear).
2. **Continue with the original `VisualDiffError::Mismatch { ... }`
   return value verbatim** — no swallowing, no replacing the
   `Mismatch` with the HTML error.
3. The original `.expect(...)` panic message at the test call site
   reports the standard baseline/diff/actual PNG paths exactly as
   today. The HTML failure becomes a stderr-only warning the
   operator may notice but does NOT affect the test outcome.

This is the **falsification probe P-VF-1 contract**: stub
`emit_visual_fail_html` to return `Err` early; assert the
calling test's failure message is byte-identical to the
pre-helper-wire-up behaviour. Implementation note: the dev
implements this probe by manually editing the helper to
`return Err(VisualFailHtmlError::Io(io::Error::other("test")));`
at function entry, runs the existing
`visual_diff_helper_writes_diff_png_on_mismatch` self-test from
`visual_diff.rs:320`, and confirms the test still asserts
`Err(VisualDiffError::Mismatch { .. })` with no behavioral change.

### D-VF-6 — ADR amendment (no new ADR)

**Decision**: amend ADR-0048 § Changelog with a one-line row;
no new ADR.

**Rationale**: this feature is a forensic-artifact augmentation of
the existing visual-diff helper. ADR-0048's D1-D6 contract covers
the "boundary test + per-Recipe mock + FAIL-only emission" shape
that this helper extends. No new architectural decision surfaces —
the HTML emission is structurally identical to the existing
`target/visual-diff/<test>.png` + `<test>-actual.png` emission
pattern, just wrapped in a single HTML container.

**Amendment shape** (architect appended at end of ADR-0048 § Changelog
during M-T1 close — already committed; developer does NOT re-amend):

```
- 2026-05-29 (architect, visual-fail-html-reporter v0.1.0
  M-T1 close): forensic-artifact emission pattern from D6 anchor-
  additivity extended to include `target/visual-diff/<test>-<ts>.html`
  alongside the existing `<test>.png` + `<test>-actual.png` triple
  on visual-assertion FAIL only. PASS path byte-identical; 71/71
  anchors unaffected (helper produces zero output on PASS). No
  D1-D6 row revised. See
  [`spec/visual-fail-html-reporter/feature.md`](../../visual-fail-html-reporter/feature.md)
  § Design D-VF-1..D-VF-6. Wave 1 sibling
  `ui-test-harness-viewport-matrix` inherits the
  `.claude/agents/tester.md` "Visual failures — HTML artifact
  emission" stanza (D-VF-4) without further amendment.
```

`spec/architecture/adr/README.md` frontmatter `updated:` field
bumped to `2026-05-29 (ADRs 0048-0049 added; ADR-0048 § Changelog
amended for visual-fail-html-reporter v0.1.0 M-T1 close)` — already
committed in architect M-T1 commit. Table row for ADR-0048 unchanged.

### Wave decomposition (D-VFH-5 per task T-VFH-T1.4)

Single M-DEV wave; no sub-waves. Estimated total ~80-100 LoC + ~22
LoC tester.md amendment + ~5 LoC ADR Changelog row. The three
M-DEV bullets (helper / wire-up / self-test) are sequential within
one developer session; no parallel sub-wave needed.

### Falsification probe P-VF-1 (T-T1 self-falsification)

Per architect.md, M-T1 includes a fast-skip check. Probe:

**P-VF-1 — emit_visual_fail_html stub-Err probe.** Developer dry-runs
the failure-mode contract before final wire-up by editing the helper
to return `Err(VisualFailHtmlError::Io(io::Error::other("synthetic
probe")))` at function entry. Then runs:

```bash
cargo test -p ui --test visual_diff visual_diff_helper_writes_diff_png_on_mismatch
```

**Expected**: the existing self-test still asserts `Err(VisualDiffError::
Mismatch { .. })` and PASSes (the original failure semantics are
preserved). Developer also greps the test stderr for the warning
line: `warning: visual-fail HTML emission failed: I/O error: synthetic
probe`. If observed → D-VF-5 contract is honoured; revert the synthetic
Err and ship. If NOT observed → either the warning is swallowed (route
back to architect for D-VF-5 amendment) OR the test outcome changed
(BUG — original `Mismatch` was masked; route back to architect
immediately).

Probe runtime ≤ 5 seconds. No fixture mutation needed.

## Implementation

**Developer M-DEV close — 2026-05-29.**

### Files changed

- `crates/ui/Cargo.toml` — added `base64 = "0.22"` and `chrono = { version = "0.4", default-features = false, features = ["clock"] }` under `[dev-dependencies]` (T-VFH-D1). Chrono was already in `Cargo.lock` at 0.4.44 via transitive deps; the explicit dep adds the `clock` feature for `Utc::now()`.
- `crates/ui/tests/fixtures/visual_fail_html.rs` — NEW file (T-VFH-D2 + D3 + D6). Exports `VisualFailContext<'_>`, `VisualFailHtmlError`, and `emit_visual_fail_html(ctx) -> Result<PathBuf, VisualFailHtmlError>`. HTML template matches D-VF-1 skeleton with inline `<style>` block. Base64 encoding via `base64::engine::general_purpose::STANDARD`. PNG dimensions via `image::ImageReader::open(path)?.into_dimensions()?`. Env-var-gated spec-persist (D-VF-3). Self-test pair in `#[cfg(test)] mod tests` guarded by a static `Mutex<()>` to prevent env var races in parallel test execution. `workspace_root()` uses `std::fs::canonicalize` to resolve `..` components so TempDir-overridden paths match what the test asserts.
- `crates/ui/tests/fixtures/mod.rs` — added `pub mod visual_fail_html;` (T-VFH-D2).
- `crates/ui/tests/fixtures/visual_diff.rs` — wired 3 call sites (T-VFH-D5): (a) `matches_screenshot` DimensionMismatch branch (~line 120), (b) `matches_screenshot` Mismatch branch (~line 155), (c) `matches_rgb_buffers` Mismatch branch (~line 205). The in-memory-buffer case (c) saves a temp baseline PNG, emits HTML, then cleans up. PASS path byte-identical — zero new code reachable on `Ok(())`.
- `crates/ui/tests/visual_fail_html_self_test.rs` — NEW integration test entry point that imports `fixtures/mod.rs` and lets Cargo discover the `#[test]` fns inside `visual_fail_html::tests`.
- `.claude/agents/tester.md` — inserted `## Visual failures — HTML artifact emission` section at line 135, between `## Tick discipline (T_FINAL ownership)` and `## Handoff` (T-VFH-D7, D-VF-4 verbatim stanza, 22 lines, section count 10 → 11).
- `spec/trace.toml` — row `REQ-VISUAL-FAIL-HTML-REPORTER-001` updated: `crates = ["crates/ui"]`, `tests = [...]` two self-test fn names, `state = "dev-done"` (T-VFH-D8).

### Falsification probe P-VF-1 outcome

Per D-VF-5: `return Err(VisualFailHtmlError::Io(io::Error::other("synthetic probe")))` inserted at function entry. Test `visual_diff_helper_writes_diff_png_on_mismatch` still PASS (`Err(VisualDiffError::Mismatch { .. })`). Stderr shows `warning: visual-fail HTML emission failed: I/O error: synthetic probe`. D-VF-5 contract honoured. Stub reverted.

### Deviations from architect spec

1. **`chrono` added as dev-dep** — D-VF-2 specified `chrono::Utc::now().format(...)` for the timestamp but did not explicitly list chrono as a dep to add. Since chrono is not a direct dep of `crates/ui` (only transitive), it was added under `[dev-dependencies]` alongside `base64`.
2. **`matches_rgb_buffers` call site (c)** — D-VF-3 said "diff PNG available" for this branch. However, since the baseline here is in-memory (no source file), a temp PNG is written to `target/visual-diff/<test>-baseline-tmp.png`, used for the HTML, then deleted. This is additive (no change to the existing `Mismatch` return).
3. **Parallel-safety mutex** — The self-tests use a `static Mutex<()>` to serialize env var mutations. This is a test-layer implementation detail not in the spec but required for correctness.

### Dev gates summary

- `cargo fmt -p ui -- --check` → EXIT:0
- `cargo test -p ui --test visual_fail_html_self_test --no-default-features --features live` → `test result: ok. 2 passed; 0 failed` (3× consecutive)
- `bash scripts/verify_anchors.sh` → `ANCHORS PASS (75 / 75)`
- Clippy: pre-existing failures in `crates/ui/src/lab/runner.rs` + `crates/agent/` (Wave B WIP, not introduced by this feature). Zero new violations in `visual_fail_html.rs` or `visual_diff.rs`.

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
- 2026-05-29 (architect, M-T1 close): § Design D-VF-1..D-VF-6 authored;
  Q1 (a) ratified (`target/` default + `EMIT_VISUAL_FAIL_TO_SPEC=1`
  opt-in + `VISUAL_FAIL_SPEC_SLUG` for disambiguation); Q2 (a)
  ratified (`base64 = "0.22"` dev-dep — already at `0.22.1` in
  Cargo.lock via transitive deps; library-compatibility checklist
  6/6 PASS); Q3 (a) **overridden to (b)** because tester.md has NO
  pre-existing visual-failure stanza to append to — new top-level
  `## Visual failures — HTML artifact emission` section inserted
  between `## Tick discipline (T_FINAL ownership)` and `## Handoff`.
  Helper at `crates/ui/tests/fixtures/visual_fail_html.rs`
  with `VisualFailContext<'_>` struct argument + `Result<PathBuf,
  VisualFailHtmlError>` return. Trigger wire-up at FAIL branches of
  `matches_screenshot` + `matches_rgb_buffers` in
  `visual_diff.rs:115/131/198`. Failure-mode contract: HTML emission
  errors log to stderr via `eprintln!` and do NOT alter the original
  `VisualDiffError::Mismatch` return value (P-VF-1 falsification
  probe spec'd). ADR-0048 § Changelog amended with one row (no new
  ADR per analyst direction); ADR README frontmatter `updated:`
  bumped. Wave decomposition: single M-DEV wave ~80-100 LoC helper +
  ~22 LoC tester.md amendment + ~5 LoC ADR row. Frontmatter flipped
  `status: draft → arch-done`, `owner: analyst → developer`. HANDOFF
  → developer (single wave; ~1 dev day).
