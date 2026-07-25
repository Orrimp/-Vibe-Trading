# UI testing & multi-agent workflow direction (2026-05-12)

This is a **strategy reset** after the chart-canvas-overhaul retrospective. The
v1.10.0 pipeline ran analyst → architect → developer → ui-designer → tester →
re-spec analyst → re-architect → re-developer → re-ui-designer and still
shipped a feature where the operator's verification step was a manual
30-second `Cmd+Shift+4` because every agent's "fix" was filtered through the
same agent that wrote it. Then a follow-up tooltip-hover artifact couldn't be
captured at all without granting macOS Accessibility to the host. That isn't a
chart bug — it's a workflow bug. This note captures the corrective direction
on **UI verification** and **multi-agent operating rules** so the next feature
spawns a different pipeline shape from day one.

The recommendations are evidence-driven. Three research passes ran in parallel
to ground this note: a local-codebase audit, an iced/Rust GUI testing
ecosystem survey, and an AI-agent UI workflow survey. Source citations are
inline; the raw research outputs live in this session's agent transcripts.

## TL;DR — five decisions to make

1. **Adopt `iced_test`** (ships in iced 0.14 — we missed it). Use
   `Simulator::snapshot().matches_image()` for chart canvases and
   `matches_hash()` for static layouts. ~1 week to wire in.
   ([iced_test on docs.rs](https://docs.rs/iced_test/0.14.0/iced_test/))
2. **Extend `insta` to binary snapshots** so the existing
   `cargo insta review` workflow covers PNG baselines, not only text.
   ([insta binary snapshots](https://insta.rs/docs/snapshot-types/))
3. **Encode the orchestrator vs. sub-agent capability asymmetry** as a
   project rule. Anything needing a display, GPU, screenshot, or window
   automation **only** runs from the orchestrator's shell. Sub-agents that
   try to verify visually fail silently and rationalize. We saw this three
   times in one session.
4. **Replace the tester agent's "tester-runs-tests" model** with a
   read-only **evaluator subagent** that has no Write/Edit tools and a fresh
   context. The current tester self-grades the developer's work in a context
   that already saw the diff. Anthropic's reference harness explicitly puts
   evaluators on a **default-FAIL** PreToolUse hook.
   ([cwc-long-running-agents](https://github.com/anthropics/cwc-long-running-agents))
5. **No PR / feature ships without a screenshot artifact captured by the
   orchestrator and reviewed by the operator.** Mirrors how Cursor and Devin
   actually work in production. No agent self-approves.
   ([Cursor agent best-practices](https://cursor.com/blog/agent-best-practices),
   [Devin Review](https://cognition.ai/blog/devin-review))

Each decision is detailed below with concrete scope, owner, and adoption
cost. Skip to §8 if you only want the phased plan.

## 1. What broke — evidence

From the chart-canvas-overhaul session and the audit:

| Failure | Evidence |
|---|---|
| Tester PASS verdict on 1280×720 capture, operator sees broken UI at 3360×1890 | `spec/chart-canvas-overhaul/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md` (prior cycle) |
| Architect "iced canvas-scale bug" misdiagnosis | `spec/chart-canvas-overhaul/feature.md ## Diagnostic` (marked SUPERSEDED), corrected via orchestrator's red-rect + cyan-dot empirical disproof |
| Developer rationalized osascript denial as universal | Developer's T3002 spike report claimed iced source was inaccessible; orchestrator read it from `~/.cargo/registry/src/...` |
| Multi-cycle (M6 → M6.2 → M7) on the same complaint | chart-buy-sell-emphasis v1.9 shipped → operator feedback → M6 → still broken → M6.2 → still broken |
| 818 tests green, 0 cover the broken behavior | Local audit confirms zero pixel-level tests; 68 insta snapshots all text-summary; 11 anchors all backtest-report SHA |

The audit's single most damning finding: walking the existing test suite,
**no test would have caught the tooltip-invisible-at-3360×1890 bug**.
`crates/ui/tests/chart_tooltip_hover_fires.rs` exercises hover-event
detection at canvas bounds `(100, 50, 800×600)` — a fixed dimension. Pass.
`crates/ui/tests/chart_tooltip_integration.rs` asserts
`cockpit.chart_tooltip.is_some()`. Pass. Neither test renders a pixel or
varies viewport size.

## 2. The orchestrator vs. sub-agent capability asymmetry

This is the **load-bearing principle** for everything else. Sub-agents in
Claude Code "exist to preserve context… enforce constraints by limiting which
tools a subagent can use… control costs by routing tasks to faster, cheaper
models" ([sub-agents docs](https://code.claude.com/docs/en/sub-agents)).
Nothing in that framing promises capability parity with the orchestrator
session.

In practice we hit three asymmetries this session:

- **Filesystem reach.** Developer agent claimed the iced 0.14 source was
  inaccessible; orchestrator read it from `~/.cargo/registry/src/...` and
  empirically disproved the architect's hypothesis from the actual source.
- **macOS automation.** Developer's `osascript` probe failed in their
  sandbox; orchestrator's `osascript` worked fine (after the operator's
  Accessibility grant, with caveats for Automation).
- **Screen Recording / `screencapture`.** Multiple sub-agents acknowledged
  they could not capture screenshots; orchestrator did so routinely via
  `screencapture -x` once Screen Recording TCC was granted to VS Code.

Sub-agents that try anyway tend to **rationalize the failure** rather than
escalate. Cognition's "Don't Build Multi-Agents" essay names this pattern:
sub-agents have no view of each other's reasoning and silently diverge
([Cognition](https://cognition.ai/blog/dont-build-multi-agents)). It matches
our session: the architect concluded "iced has a canvas-scale bug" from
instrumentation it ran in its own sandbox, never saw the orchestrator's
empirical disproof, and routed the developer down a fake fix path that
escalated back to the orchestrator three rounds later.

**Corollary — what to forbid in agent instructions:**

- Sub-agents must not capture screenshots.
- Sub-agents must not conclude "the bug is X" from instrumentation that
  required a display server or GPU.
- Sub-agents must not determine whether a UI fix "works" — only that it
  type-checks, that pure-Rust tests pass, and that anchors hold.
- Sub-agents must not adjudicate disagreements between sibling sub-agents.
- The orchestrator empirically arbitrates and the operator visually
  approves. Period.

This belongs in [AGENT.md](../../AGENT.md) as a non-negotiable section, not
in this dev-note.

## 3. UI testing — concrete recommended stack

**Layer 1 — `iced_test` headless snapshots.** iced 0.14 ships
[`iced_test`](https://docs.rs/iced_test/0.14.0/iced_test/) (added in
[PR #2698](https://github.com/iced-rs/iced/pull/2698) + [#3059](https://github.com/iced-rs/iced/pull/3059),
released with [iced 0.14.0](https://github.com/iced-rs/iced/releases/tag/0.14.0)).
Three primitives we should use:

- `Simulator::new(program)` — drives the cockpit headlessly with
  `click(selector)`, `tap_key`, `typewrite`, `find(selector)`,
  `into_messages()`.
- `Simulator::snapshot(theme) → Snapshot` — captures the rendered frame.
- `Snapshot::matches_image("path/to.png")` — first run writes baseline,
  subsequent runs byte-compare. `Snapshot::matches_hash("path.sha256")`
  for SHA-256 comparison on stable layouts.

Critically, `iced_test` runs on whatever renderer the program is configured
with. We already pin `iced_tiny_skia` ("expected to match Skia
pixel-for-pixel" per the [tiny-skia README](https://github.com/linebender/tiny-skia)).
That CPU determinism is what makes `matches_hash` viable — a bit-flip in a
chart-line position changes the SHA, and CI runners produce the same bytes
as developer machines.

**Layer 2 — `insta` binary snapshots.**
[`insta`](https://insta.rs/docs/snapshot-types/) supports binary snapshots
(`assert_binary_snapshot!`) that hold any bytes — PNGs included. We already
use `insta` + `cargo insta review` + the [VS Code extension](https://insta.rs/docs/snapshot-types/)
for text snapshots. Extending to PNGs reuses the same review workflow.
Layered on top of `iced_test::Snapshot`, the test code reads:

```rust
let bytes = simulator.snapshot(theme)?.png()?;
insta::assert_binary_snapshot!("charts_screen@3360x1890.png", bytes);
```

`cargo insta review` then opens an interactive diff in VS Code. This breaks
the "agent verifies its own work" cycle: review is human-gated by
construction.

**Layer 3 — viewport matrix.** Every snapshot test runs at three
resolutions:

| Slot | viewport | scale_factor | rationale |
|---|---|---|---|
| floor | 1280 × 720 | 1.0 | min_size |
| typical | 1920 × 1080 | 1.0 | new default per T3022 |
| operator | 3360 × 1890 | 2.0 | actual hardware |

This makes "operator saw something different at 3360×1890" a compile-time
loud test failure rather than a cycle-burning surprise. Parameterize via
`#[rstest]` or a simple test-helper that loops.

**Layer 4 — canvas hit-test harness.** `iced_test::Simulator` operates at
the widget-tree level via accessibility selectors. It does not synthesize
`canvas::Event::Mouse(CursorMoved { ... })` directly into a `Program`'s
`update`. We already have a partial harness at
`crates/ui/src/widgets/chart.rs:742` (`../crates/ui/src/widgets/chart.rs`)
(`dispatch_canvas_event_for_test`). Extend it so a unit test can:

```rust
let (msg, status) = dispatch_canvas_event_for_test(
    bars, markers, signals,
    &mut hover_state,
    canvas::Event::Mouse(mouse::Event::CursorMoved { position }),
    bounds,                                  // viewport-parameterized
    cursor_pos,                              // absolute screen coords
);
assert_eq!(msg, Some(Message::ChartMarkerHovered(Fill(idx))));
```

The single test we needed but never had: **iterate the cursor across a grid
of cursor positions at each viewport size, assert hover detection fires for
every marker centroid**. Would have caught the
`cursor.position_in(bounds)?` early-bail bug. Would have caught any future
gutter-math regression that shifts marker centroids by a few pixels.

**Layer 5 — perceptual diff for fuzzy regions.** Most snapshots use exact
byte comparison via `matches_hash`. For chart canvases where antialiasing
varies, wrap with [`image-compare`](https://docs.rs/image-compare) (MIT/Apache,
hybrid SSIM+RMS, produces a visual-diff PNG). On failure, persist the diff
PNG to `target/visual-diff/<test>.png` so the operator sees the delta, not
just a verdict. Avoid `dssim-core` — it's AGPL/commercial dual-licensed.

**CI shape.** macOS runner only (per egui_kittest's "macOS is source of
truth" stance per [egui kittest.toml](https://github.com/emilk/egui/blob/main/kittest.toml)
and our tiny-skia CPU determinism). No Xvfb, no lavapipe, no GPU drivers —
tiny-skia is pure CPU. Failures upload baseline + actual + diff PNG triple
as GitHub Actions artifacts. The orchestrator (not the tester sub-agent)
launches the CI run; the operator clicks through the artifacts and
approves or rejects.

## 4. Multi-agent workflow corrections

### 4.1 Capability map

Codify in [AGENT.md](../../AGENT.md) which capabilities live where.

| Capability | Owner | Allowed for sub-agents? |
|---|---|---|
| `cargo fmt`, `cargo clippy`, `cargo test` (pure Rust) | sub-agent | yes |
| `verify_anchors.sh` | sub-agent | yes |
| `rust-build`, `rust-validate` skills | sub-agent | yes |
| spec-update writes to `spec/<slug>/` | sub-agent | yes |
| `cargo run --bin cockpit` with live window | **orchestrator** | **no** |
| `screencapture` of running app | **orchestrator** | **no** |
| `osascript`, `cliclick`, Swift CGWarp | **orchestrator** | **no** |
| Concluding "the bug is X" from live-app instrumentation | **orchestrator** | **no** |
| Visual approval / rejection of UI | **operator** | **no** |

The PreToolUse hook pattern from
[cwc-long-running-agents](https://github.com/anthropics/cwc-long-running-agents)
makes this structural rather than aspirational. We have no such hook today.
Adding one for `Bash(screencapture ...)` and `Bash(./target/.../cockpit ...)`
that denies sub-agents and allows the orchestrator costs ~1 day.

### 4.2 Default-FAIL evaluator subagent

Today's tester agent runs `rust-test`, `rust-validate`, `verify_anchors`,
authors a test report, and emits VERDICT → PASS/FAIL/REGRESSION. **It self-grades
the work it just witnessed.** Anthropic explicitly names this anti-pattern:

> "Agents reliably skew positive when grading their own work."
> ([effective-harnesses-for-long-running-agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents))

The cwc-long-running-agents repo's evaluator subagent (a) has **no
Write/Edit tools**, (b) operates in a **fresh context that never saw the
build**, and (c) is gated by a **PreToolUse hook that denies any write to
the results file unless the agent has first opened one with the Read tool**
— a default-FAIL contract.

Concrete change to `.claude/agents/tester.md`:

- Replace "tester runs tests + writes verdict" with two roles:
  - **Test-runner** (writeable): runs `rust-test`, `rust-validate`,
    `verify_anchors`, dumps raw output to
    `spec/<slug>/reports/test-run-<ts>.log`. **No verdict.** No prose.
  - **Evaluator** (read-only): fresh context, only `Read` + `Bash(grep|wc|sha256sum)`,
    no `Write`/`Edit`/`Bash(cargo*)`. Reads the run log + the artifact
    screenshots, writes `spec/<slug>/reports/evaluation-<ts>.md`. PreToolUse
    hook denies the evaluator opening the evaluation file in append mode
    unless its read trace already contains the run log + every cited
    artifact.
- VERDICT → PASS still emits from the evaluator, but it cannot fabricate
  green tests because the hook fail-closes when artifacts are missing.

### 4.3 Architect = hypothesis only

In this session the architect ran instrumentation (`eprintln` traces),
captured screenshots, drew a root-cause conclusion ("iced canvas-scale
bug"). The conclusion was wrong and routed the dev down a 1.5-day dead end.

Architects in this project must:

- Author hypotheses with explicit falsifiers ("if X, then Y measurement").
- **Not** run instrumentation that requires a display.
- **Not** ship a root-cause claim without a citation to an orchestrator-run
  empirical test that refused to falsify.

Hypotheses without orchestrator-run falsification are first-class spec
artifacts — not blockers, just unresolved. The architect can ship five
hypotheses with falsifiers and let the orchestrator pick which to test
first.

### 4.4 Parallelism: keep, but not for sequential reasoning

Developer ‖ ui-designer parallelism worked in v1.9.0 (clean split:
ui-designer owns chart_legend.rs + tokens; developer owns chart.rs +
axes). It failed when the brief had inter-task dependencies the orchestrator
hadn't surfaced (T3017 wire-up needed T3016 to land first; coordination
was via tasks.md and both agents temporarily patched `cockpit.rs:158`).

Anthropic's [multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)
post: *"Most coding tasks involve fewer truly parallelizable tasks than
research."* Default to **sequential** dev → ui-designer → orchestrator
unless the orchestrator can articulate the lane split explicitly in the
spawn brief. Cognition's "Don't Build Multi-Agents" essay reinforces:
parallel sub-agents with no shared context silently diverge.

## 5. Operator-in-the-loop daily flow

Today's operator flow on a single feature: hop into the IDE 8–15 times to
answer Q1/Q2/Q3 from the analyst, review architect Qs, confirm operator
decisions, click approve on the presenter deck, then verify the live binary
once or twice. With per-cycle visual ambiguity (M6 → M6.2 → M7) that
expands to 30+ touchpoints.

Target flow with the recommendations adopted:

- **Pre-spawn alignment (~5 minutes).** Operator reviews the analyst's
  question batch once. No mid-pipeline Q&A unless the architect-decide
  framework genuinely fails.
- **Architect framework review (~3 minutes).** Operator reviews resolved
  Qs + risk register + acceptance criteria. Confirm or revise.
- **Background work.** Sub-agents run in parallel. The orchestrator runs
  CI-equivalent: `cargo test`, screenshot-snapshot-tests at 3 viewport
  sizes, anchor verification. Operator does other work.
- **Artifact review (~5 minutes).** Operator opens the screenshot-diff PR
  artifacts in `target/visual-diff/`, approves or rejects via
  `cargo insta review` (interactive) or a `/approve-ui` comment style
  command. **No live cockpit launch required** unless the operator wants
  one — the snapshot harness already captured the 3360×1890 reality.
- **Live smoke (~3 minutes, optional).** Operator launches the cockpit,
  exercises any "feel" criteria that aren't pixel-equivalent (animation
  smoothness, latency, focus order). This is the only step that can't be
  automated.

Total operator time per feature ship: ~15 minutes vs. today's 60–90.

## 6. Phased adoption — 4-week plan

**Week 1 — `iced_test` smoke.** One snapshot test for the Charts screen at
1280×720 in dark mode. Lands `crates/ui/tests/visual_snapshots.rs` with one
`#[test] fn charts_screen_dark_1280x720()`. Confirms tiny-skia CPU
determinism holds across local + CI. Wires up `cargo insta review` for the
binary snapshot. ~3 dev-days.

**Week 2 — viewport matrix + canvas hit-test harness.** Parameterize the
Week 1 snapshot over the 3-viewport matrix. Extend
`dispatch_canvas_event_for_test` to sweep a cursor grid and assert hover
detection at every marker centroid in each viewport. Lands the test that
would have caught the original session's bug. ~4 dev-days.

**Week 3 — Evaluator subagent + capability hooks.** Split
`.claude/agents/tester.md` into test-runner + evaluator. Add PreToolUse
hooks denying `screencapture`, `osascript`, `./target/release/cockpit` to
sub-agents and allowing them to the orchestrator. Update [AGENT.md](../../AGENT.md)
with the capability map (§4.1). ~3 dev-days.

**Week 4 — CI + presenter integration.** GitHub Actions workflow on macOS
runner: `cargo test`, `cargo insta test`, viewport-matrix snapshot tests,
artifact upload of baseline+actual+diff PNGs. Presenter agent's deck format
gets a fixed "screenshot artifacts" section pointing at the CI artifacts
URL. The operator's approval is a single GitHub comment or the equivalent
in our local workflow. ~5 dev-days.

Total: ~3 calendar weeks if one engineer is dedicated. Spread across
features-in-flight, maybe 5–6 weeks.

## 7. What this won't fix

Honest list of remaining gaps even after adoption:

- **Real macOS NSWindow rendering.** `iced_test` renders in-process; it
  never opens a real window. Bugs that only manifest in OS-composited
  frames — focus stealing, TCC permission interactions, traffic-light
  overlap, dark-mode-from-system-pref bugs — are unreachable. Zed/GPUI's
  [`zed_visual_test_runner`](https://github.com/zed-industries/zed/blob/main/docs/src/development/macos.md)
  captures real OS windows, but at the cost of macOS-only CI with Screen
  Recording TCC permission on every runner. Multi-week project. Defer
  unless one of these classes of bug bites us repeatedly.
- **Retina ambiguity.** `iced_test::screenshot`'s `scale_factor` argument
  controls the iced renderer's logical→physical scaling but does not
  reproduce the macOS compositor's HiDPI behavior fully. Mitigation: the
  viewport matrix enumerates scales explicitly; never rely on a runner's
  "natural" DPI.
- **Font drift.** `iced_test` embeds Fira Sans via PR #2698 so glyph
  shapes are stable, but system emoji fallback and HarfBuzz drift still
  cause sub-pixel shifts. Use `matches_hash` on text-free regions; accept
  OS-suffixed baselines for text-heavy frames.
- **Architect misdiagnosis class.** No test stack stops an agent from
  claiming "iced has a half-scale canvas bug." The procedural mitigation
  is §4.3: hypotheses without orchestrator-run falsification don't become
  fixes. Slint ships an [MCP server](https://github.com/slint-ui/slint/blob/master/docs/testing.md)
  that lets AI agents inspect a live UI to falsify hypotheses; iced does
  not have one. We could write our own for the cockpit; deferred.
- **Computer-use for verification.** Anthropic's docs are blunt: beta,
  slow, Linux-X11 only, not suited for native macOS iced
  ([computer-use](https://platform.claude.com/docs/en/agents-and-tools/computer-use)).
  Keep it in the "experimental" bucket; do not gate UI on it.

## 8. Resolution path for chart-canvas-overhaul v1.10.0

The paused feature still needs to land. With this direction in place, the
clean resolution is:

1. Operator approves the screenshots already in
   `spec/chart-canvas-overhaul/reports/screenshots/m7-*` as **V14
   acceptance** (legend visible, axes visible, layout correct).
2. **V15 tooltip-hover** is deferred to **week-1 of the adoption plan**:
   the first `iced_test` snapshot test we write IS a chart-hover test that
   asserts the tooltip card renders at a known position when the cursor
   is over a marker at 3360×1890. That test, not a manual screenshot,
   becomes the V15 acceptance.
3. Mark T3029 closed with a forward-pointer to the week-1 task. Tick the
   presenter, ship v1.10.0.

This avoids burning more cycles on macOS Accessibility / Automation grants
for an artifact that the adoption plan will replace anyway.

## 9. Open decisions for the operator

Before any of this gets implemented, the operator should pick:

- **D1.** Adopt all 5 TL;DR recommendations as a single block, or sequence
  them? Recommended: as a block — they're load-bearing on each other.
- **D2.** Week-1 owner — solo dev pass (you), or analyst → architect →
  dev pipeline with the new rules baked into the briefs from day one?
  Recommended: solo dev pass for week 1 (it's bootstrap infrastructure, no
  product-feature scope), pipeline from week 2 onward.
- **D3.** macOS-only CI is acceptable for the foreseeable future? Or do we
  need Linux/Windows test runs even if they only run `matches_hash` on
  text-free regions? Recommended: macOS-only — the cockpit is macOS-only.
- **D4.** chart-canvas-overhaul resolution per §8 (approve V14 from existing
  screenshots, defer V15 to week-1 snapshot test, ship)? Or do we want a
  manual V15 capture before ship?
- **D5.** Should we open a follow-up `spec/agent-workflow-rules/` brief
  capturing §4.1–§4.4 as a permanent AGENT.md amendment, or is this
  dev-note enough until the changes prove out?

---

**Sources**

Research outputs cited in this note are session-transcript artifacts.
External URLs:

- iced 0.14: [iced_test docs.rs](https://docs.rs/iced_test/0.14.0/iced_test/),
  [PR #2698 Headless Mode Testing](https://github.com/iced-rs/iced/pull/2698),
  [iced 0.14.0 release](https://github.com/iced-rs/iced/releases/tag/0.14.0)
- Snapshot tooling: [insta binary snapshots](https://insta.rs/docs/snapshot-types/),
  [cargo-insta](https://crates.io/crates/cargo-insta)
- Visual diff: [image-compare](https://docs.rs/image-compare),
  [twenty-twenty](https://docs.rs/twenty-twenty)
- Comparison frameworks: [egui_kittest](https://docs.rs/egui_kittest),
  [Slint testing](https://github.com/slint-ui/slint/blob/master/docs/testing.md),
  [Zed GPUI](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md)
- Anthropic guidance: [Claude Code best-practices](https://code.claude.com/docs/en/best-practices),
  [Claude Code sub-agents](https://code.claude.com/docs/en/sub-agents),
  [effective-harnesses-for-long-running-agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents),
  [computer-use](https://platform.claude.com/docs/en/agents-and-tools/computer-use),
  [multi-agent-research-system](https://www.anthropic.com/engineering/multi-agent-research-system),
  [cwc-long-running-agents](https://github.com/anthropics/cwc-long-running-agents)
- Production patterns: [Cognition — Don't Build Multi-Agents](https://cognition.ai/blog/dont-build-multi-agents),
  [Devin Review](https://cognition.ai/blog/devin-review),
  [Cursor agent best-practices](https://cursor.com/blog/agent-best-practices),
  [Chromatic Review](https://www.chromatic.com/features/review)
