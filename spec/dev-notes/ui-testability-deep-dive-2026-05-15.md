---
slug: ui-testability-deep-dive
status: living
owner: analyst
updated: 2026-05-15
---

# UI testability deep-dive (2026-05-15)

This is a research dev-note, not a feature brief. Its audience is the
operator deciding what to schedule next, and any future analyst who
opens a `ui-test-*` feature brief from `spec/backlog.md`. It builds on
top of and where necessary critiques the canonical
[`ui-testing-direction-2026-05-12.md`](ui-testing-direction-2026-05-12.md)
strategy doc and the v0.1 retrospective at
[`spec/ui-test-harness-bootstrap/reports/evaluation-2026-05-12T13-15Z.md`](../ui-test-harness-bootstrap/reports/evaluation-2026-05-12T13-15Z.md).

The operator's framing — **"it is hard for the agents to test the UI"** —
is the load-bearing problem. The v0.1 bootstrap proves that with
discipline a screenshot harness can ship under the new agent regime, but
it leaves most of the failure modes the chart-canvas-overhaul retro
surfaced still uncovered: interaction sequencing, perceptual regressions,
agent-readable assertions about meaning, accessibility/legibility,
operator-grade integration flows, and feature-completeness signal. This
note critiques the existing four-week plan, brings back fresh ideas from
a web sweep, and proposes a reordered rollout that the operator can
sequence feature-by-feature.

## 0. TL;DR — for the operator's tick

The existing plan is structurally sound but priced one dimension wrong:
**it overweights pixel-perfect screenshots and underweights
agent-readable structure**. The single highest-ROI change is to add a
**widget-tree assertion layer that does not render pixels at all** —
[`kittest`](https://docs.rs/kittest/) over an AccessKit adapter, plus a
pure-Rust state-machine harness over `ui::state::update`. Pixels become
the **failure forensics layer**, not the primary oracle.

Three ideas the existing dev-note does not mention that I want on the
backlog:

1. **iced live-inspect MCP shim (`ui-inspect-mcp`)** — bolt a minimal
   MCP server onto the live cockpit so the orchestrator (not a sub-agent)
   can query "is the tooltip card on-screen and >32 px tall?" at the
   3360×1890 viewport without a manual screencap. Slint already ships
   this; iced does not, but the cockpit can host its own
   ([Slint testing backend](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/)).
   This is the structural answer to "the operator does a 30-second
   `Cmd+Shift+4`."
2. **VLM-as-oracle, scoped to one job** (`ui-vlm-judge`) — Claude
   Sonnet 4.6 vision as a **second opinion** on three deliberate
   assertions ("the tooltip is visible", "no element overlaps another
   element by >50%", "every chart label has ≥4.5:1 contrast against
   what's behind it"). Not a global oracle (cost + flakiness); a
   targeted oracle that runs after a `matches_image` PASS to catch the
   class of failure pixel-diff misses (a uniformly broken layout that
   matches the broken baseline).
3. **Storybook-equivalent widget gallery** (`ui-gallery-bin`) — a
   `cargo run --bin ui-gallery` that renders every widget × every state
   × every viewport on one scrolling page. Multiplies the harness ROI
   by ~10× — one snapshot test gets 50+ cells of coverage instead of
   one screen.

Drop-or-replace recommendation against the existing weeks 2-4 plan:
**keep week 2 (full-widget viewport matrix) but reframe it**, **replace
week 3 (evaluator hooks)** with the AccessKit/kittest pivot, **defer
week 4 (CI integration)** by one cycle so the kittest pivot lands first.
Reasoning below.

## 1. Reading the existing plan with fresh eyes

The four-week plan in
[`ui-testing-direction-2026-05-12.md §6`](ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan)
ships:

- Week 1: `iced_test` smoke + 1 snapshot at 3 viewports. **SHIPPED v0.1**.
- Week 2: viewport matrix across all widgets + canvas hit-test sweep.
- Week 3: test-runner/evaluator split + PreToolUse hooks.
- Week 4: CI on macOS runner + presenter integration.

The v0.1 retrospective surfaces three real signals that re-shape the
remaining weeks.

### 1.1 The H2 "tooltip render gap" is not an edge case — it is the model breaking

[`feature.md ## Hypothesis register`](../ui-test-harness-bootstrap/feature.md#hypothesis-register)
H2 is logged "RESOLVED-WITH-CAVEAT". The caveat is structural: the V15
acceptance test for chart-canvas-overhaul cannot render its acceptance
condition because **the canvas widget owns hover state inside
`ChartProgram::State`, and `Cockpit::chart_tooltip` is a shadow copy that
the canvas does not read**. The fix is queued as the
`ui-test-harness-canvas-state-seeding` backlog candidate.

This is not a "we need a seed helper for one widget" problem. It is the
**iced architecture telling us "view-from-state" is not the whole truth**:
canvas widgets keep `Program::State` that the harness cannot seed
through the iced 0.14 public API. Any widget that uses a `canvas::Program`
(today: chart only; future likely: equity curve, drawdown band,
sparkline if they migrate from `tiny_skia` direct-draw to
`canvas::Program`) has the same gap.

The strategy plan does not name this. The Layer 1..N model in
[`dev-note §3`](ui-testing-direction-2026-05-12.md#3-ui-testing--concrete-recommended-stack)
assumes `for_charts_screen_test_program()` exposes everything the
operator sees. It doesn't, and won't, for canvas-owning widgets.

**Implication.** Either the test harness reaches into `Program::State`
(the queued `ui-test-harness-canvas-state-seeding` path), OR the harness
**dispatches synthetic mouse events to drive state into existence**
(Slint's MCP server approach, kittest's AccessKit approach). The first
is one fix per canvas widget; the second is one harness, every canvas
gets it for free. The dev-note does not consider option two.

### 1.2 The "agent can't see pixels" sandbox is a strength, not just a constraint

The
[AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)
amendment correctly forbids sub-agents from running `screencapture` or
launching the cockpit. The bootstrap proves the discipline works. But
the plan's Layer 1..N is **still pixels-first**: snapshot + diff +
operator approval is the v0.1 architecture and the proposed weeks 2-4
architecture.

Pixels are the wrong oracle for half the failures we see:

| Failure class | Pixel diff catches it? |
|---|---|
| Tooltip invisible at 3360×1890 (the founding bug) | Yes — if a baseline exists at that viewport. |
| `cursor.position_in(bounds)?` early-bail (T3022 root cause) | No — pixels are identical; the message flow is broken. |
| Text contrast drops below 4.5:1 in a future palette refactor | No — pixel SHA is byte-identical to itself. |
| `Message::KillConfirmed` fires when the typed phrase is wrong | No — pixels never re-render between keypress and assertion. |
| Strategy row sorts by wrong column after T1507 refactor | Maybe — only if the screenshot covers the sorted region. |
| Loading spinner stuck because subscription never completes | No — pixel is "valid loading state" — looks fine. |
| Inline hex literal sneaks into a widget (`theme.rs` bypass) | No — pixel is identical; consistency.rs catches it. |
| Reachable widget never exercised by any test (coverage gap) | No — pixel-diff only knows about tests that exist. |

**Half of these are answerable from the widget tree alone, no pixels
needed.** `kittest` over AccessKit (or any tree-shaped accessor)
answers them in milliseconds per test, runs in the sandboxed sub-agent,
needs no operator review, and produces a stable text diff that the
evaluator can read.

### 1.3 The plan's §7 "what this won't fix" is honest but incomplete

`§7` ([dev-note §7](ui-testing-direction-2026-05-12.md#7-what-this-wont-fix))
correctly lists: macOS NSWindow rendering, Retina ambiguity, font
drift, architect misdiagnosis class, computer-use. It misses:

- **Reachability coverage.** Nothing tells the team that
  `crates/ui/src/widgets/override_risk_veto.rs` (278 LOC, shipped under
  human-in-the-loop) has zero test references outside its own module.
  A widget the operator can reach but no test touches is a future
  v1.10.0-class incident waiting.
- **`update` correctness.** `crates/ui/src/state.rs` is 2472 lines; the
  `update` function dispatches ~60 `Message` variants. v0.1 ships
  panel/visual snapshots and one canvas hit-test sweep — none of these
  prove that, say, `Message::AgentHaltedExternally` actually halts the
  agent display the way `Message::KillConfirmed` does. The fact that
  Elm-style `update` is a **pure function** is gold-standard for
  testing and the harness ignores it.
- **Voice/copy drift.** A future ui-designer mistakes `WARN_500` for
  `DOWN_500` on a status pill. Pixels capture it. Insta text snapshots
  capture it. Nothing tells us "the status pill's
  Lumen-rules-of-three pairing
  ([ui-design-principles.md § Status pill colors](../ui-design-principles.md#status-pill-colors))
  was violated." That's a structural assertion against the theme
  contract.
- **Real perf and input responsiveness.** The just-promoted
  [`cockpit-performance-and-input-responsiveness`](../backlog.md#queue)
  backlog item is exactly this gap: snapshots are layout-determinism
  tests, not runtime-event-flow tests. The harness has no way to
  measure "every click landed."

The plan also implicitly assumes **D3 macOS-only is permanent**. That
decision is worth revisiting; see § Decisions to revisit below.

## 2. Web research — what's out there in 2026

This section captures what a real WebSearch sweep surfaces today.
Citations inline. Unverified claims marked `[unverified]`.

### 2.1 iced ecosystem testing — current state

**iced 0.14 (shipped December 2025).** Reactive rendering, time-travel
debugging, headless-mode testing
([PR #2698](https://github.com/iced-rs/iced/pull/2698) +
[#3059](https://github.com/iced-rs/iced/pull/3059)), animation API,
hot reloading, the `comet` debugger
([CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md),
[Iced 0.14 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0)).
The dev-note's §3 Layer 1 captures `iced_test::Simulator` / `screenshot`
/ `matches_image` — those are correct and shipped in v0.1.

**iced 0.14 limits relevant to us.** The
[`iced_test 0.14` docs](https://docs.rs/iced_test/0.14.0/iced_test/)
expose `click`, `find`, `tap_key`, `typewrite`, `into_messages`, and
`screenshot(...)` returning `iced::window::Screenshot { rgba, size,
scale_factor }`. Selectors are `&str`-based text selectors only — no
public AccessKit selector, no canvas-event injection. The bootstrap's
own developer pass confirmed (`tasks.md ## M1 T4014`):
`iced_test::Snapshot` exposes only `matches_image` / `matches_hash`
with no public byte accessor. So **today's iced_test cannot select a
widget by role, query the rendered tree, or fire `canvas::Event::Mouse`
directly** — it can only click text and screenshot.

**iced master and AccessKit.** iced has an open
[issue #552 — Implement accessibility support](https://github.com/iced-rs/iced/issues/552)
since 2020, plus
[issue #282](https://github.com/iced-rs/iced/issues/282). As of this
writing (May 2026) **AccessKit is not merged into iced** `[unverified — confirmed via web search, but no merged PR found]`.
egui has had AccessKit since 2022
([PR #2294](https://github.com/emilk/egui/pull/2294)); Bevy has it
([PR #18706](https://github.com/bevyengine/bevy/pull/18706)); Slint
ships an AccessKit-backed MCP server in
[`i-slint-backend-testing`](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/).
**iced is the laggard.** This shapes the "agent-readable DOM" design
space: we can't use AccessKit through iced today; if we want a widget
tree we either (a) wait for iced upstream, (b) PR it ourselves, or
(c) build a parallel "structural shadow tree" inside `ui::state` that
the harness reads.

**Community testing patterns.** A scan of squidowl/halloy, cosmic-comp,
iced_aw, and the iced examples surfaces **no published higher-level
interaction harness** beyond what `iced_test` ships. Halloy uses
plain `cargo test` ([release notes](https://github.com/squidowl/halloy/releases/tag/2026.4));
cosmic-comp tests at the compositor layer not the widget layer
([System76 blog](https://blog.system76.com/post/cosmic-epoch-2-and-3-roadmap/)).
The bootstrap brief's "we are the first ones doing this" framing is
correct.

### 2.2 kittest — the framework-agnostic AccessKit harness

[`kittest`](https://docs.rs/kittest/) is rerun-io's GUI testing library
inspired by JS Testing Library, built on AccessKit, **framework-agnostic
by design**. egui has the
[`egui_kittest`](https://docs.rs/egui_kittest) adapter; the kittest
[`NodeT` trait](https://docs.rs/kittest/latest/kittest/trait.NodeT.html)
documents "creating new kittest integrations is simple and
straightforward". `egui_kittest`'s docs explicitly say *"prefer regular
Rust tests or insta snapshot tests over image comparison tests… they
are brittle since unrelated side effects (like a change in color) can
cause the test to fail"* — that's the same lesson the bootstrap H2
caveat surfaced.

The implication is structural: even without iced AccessKit merging,
**a thin `iced` adapter that emits accessibility nodes for the
cockpit's widget tree** would let us query "is the kill button
focused", "is the tooltip card rendered", "what's the row count of the
positions table" *without rendering pixels*. That adapter is ~500 LOC
of analyzed work, not a multi-week project.

### 2.3 Property-based and model-based testing

[`proptest-state-machine`](https://crates.io/crates/proptest-state-machine)
is the canonical Rust crate for stateful property testing. The
[blog post by Tomáš Zemanovič](https://tzemanovic.gitlab.io/posts/state-machine-testing-with-proptest/)
walks through it; Nikos Baxevanis has a
[2025 worked example](https://blog.nikosbaxevanis.com/2025/01/10/state-machine-testing-proptest/).
ReadyT ships an alternative [`proptest-stateful`](https://github.com/readysettech/proptest-stateful)
for cases where the reference model and SUT diverge intentionally.

For iced, the relevant insight is: `ui::state::update` is a **pure
function**. Given `(Cockpit, Message) → Cockpit`. That's the ideal
shape for a state-machine property test:

```text
ReferenceStateMachine: a hand-written invariant model of valid Cockpit transitions.
StateMachineTest:      drives Cockpit via update() and asserts invariants hold
                       after every transition.
```

Invariants we can encode:

- **Kill switch monotonicity.** Once `KillConfirmed` fires, no
  subsequent `Message` returns the cockpit to a non-halted state.
- **Subscription channel safety.** A `PnlError(_)` message followed
  by `PnlRefreshed(_)` must leave the panel in `Ready`, not stuck
  in error.
- **No state cross-talk between screens.** Switching screens
  `Charts → Audit → Charts` must leave the chart's hover/tooltip
  state unchanged.
- **PanelState exhaustiveness.** Every panel reaches all four
  `PanelState<T>` arms via some sequence of `Message`s.

The plan does not mention this. It is the cheapest test in the
catalogue — no rendering, no fonts, no determinism caveats — and it
covers the third of failures pixel-diff cannot reach.

### 2.4 Perceptual comparison beyond pixel diff

The bootstrap shipped `image-compare`'s `rgb_hybrid_compare` for
forensics on failures. Three orthogonal upgrades exist:

- **pHash / dHash perceptual hashing.** Rust crates:
  [`image_hasher`](https://crates.io/crates/image_hasher),
  [`img_hash`](https://github.com/abonander/img_hash) (multiple forks),
  [`visual-hash`](https://github.com/archer884/visual-hash). pHash
  reduces a frame to a 64-bit hash robust to minor antialiasing /
  compression / subpixel drift. Tolerance is a Hamming distance
  threshold. **Useful as a "did anything meaningful change" gate
  before the byte-exact diff** — agent CI runs pHash first, only
  surfaces the operator on Hamming > N. Cost: tiny (sub-millisecond
  per image).
- **SSIM / MS-SSIM.** [`image-compare`](https://docs.rs/image-compare)
  is shipped. [`dssim`](https://github.com/kornelski/dssim) is more
  perceptually accurate but is AGPL/commercial dual-licensed — we
  already exclude it in the bootstrap's design (Q8 lock). For
  text-heavy frames where antialiasing varies, [jest-image-snapshot
  thread #201](https://github.com/americanexpress/jest-image-snapshot/issues/201)
  documents SSIM "significantly reduces false positives." The
  trade-off is **false negatives** — SSIM can pass a frame where one
  small but operationally important region (kill button label) has
  shifted.
- **OCR-based assertions.** [`leptess`](https://github.com/houqp/leptess)
  (Rust binding for Tesseract) or
  [`ocrs`](https://github.com/robertknight/ocrs) (pure-Rust OCR by
  Robert Knight). The assertion shape: *"after rendering, OCR the
  baseline PNG, assert the string 'PnL' is found in the inner-rect
  area with bounding box ≥ X×Y px."* This is the **agent-readable
  legibility test** — the one assertion that would have caught the
  3360×1890 tooltip bug *even without a baseline at that viewport*.
  Limitations: Tesseract requires ≥300 DPI for reliable recognition
  ([Transloadit](https://transloadit.com/devtips/recognize-text-in-images-ocr-in-rust/),
  [thomasgruebl/rusty-tesseract](https://github.com/thomasgruebl/rusty-tesseract)),
  preprocessing is brittle, and adding Tesseract is a non-trivial
  C++ dep. `ocrs` is pure-Rust but newer, less battle-tested
  `[unverified]`.

**Recommendation in priority order:** pHash gate before byte-diff →
SSIM for text-heavy regions only (per
[`image-compare`](https://docs.rs/image-compare) hybrid mode, already
shipped) → OCR deferred until the cost is justified by a real
legibility regression.

### 2.5 Contrast / WCAG / colorblindness assertions

[`ui-design-principles.md`](../ui-design-principles.md#accessibility-minimums)
mandates "contrast ≥ 4.5:1 (AA) for body text, ≥ 7:1 (AAA) for the
equity display" but there is **no automated check**. The current
audit is "verified against the table in `Color palette`" — i.e.
human-eye-balled.

Mechanically, contrast ratio is a simple function of two RGB tuples
([W3C WCAG 2.1 understanding](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html)).
There is no obvious Rust crate
([crates.io search](https://crates.io/search?q=wcag+contrast) returns
few, mostly unmaintained `[unverified]`). A **20-LOC helper inside
`crates/ui/tests/`** is enough:

```text
fn contrast_ratio(fg: Color, bg: Color) -> f32 { /* WCAG 2.1 formula */ }
```

Wire it into the consistency-tests pattern that's already in
`crates/ui/tests/consistency.rs`. Every `theme.rs` token pair flagged
"foreground/background" gets a compile-time assertion. **This costs
half a day and closes an entire class of regressions without a
single pixel.**

Colorblindness simulation is one step further: transform RGB →
[Brettel-Vienot-Mollon-simulated dichromat space](https://en.wikipedia.org/wiki/Color_blindness)
and re-run the contrast assertion. Justifies its weight when v3+ i18n
ships; today's all-English, single-operator surface doesn't. Mark as
stretch.

### 2.6 Headless / virtual display

iced_test already runs headless on tiny-skia CPU — no display server
needed. The bootstrap's H1 confirmed byte-determinism across two
consecutive runs. The dev-note's §7 caveats (NSWindow, Retina,
font drift) are correct.

**The operator's D3 ("macOS-only CI") decision deserves a fresh look.**
The argument for re-litigating:

- iced_test is CPU-bound on tiny-skia, which is "expected to match
  Skia pixel-for-pixel"
  ([tiny-skia README](https://github.com/linebender/tiny-skia)). For
  **non-text regions**, byte-determinism likely holds across macOS /
  Linux / Windows runners `[unverified — needs falsifier]`.
- For text regions, [cosmic-text](https://github.com/pop-os/cosmic-text)
  + HarfRust shaping is the source of drift. But cosmic-text is also
  cross-platform and font-cache-driven; with the **font embedded**
  (`include_bytes!`), shaping is deterministic per-cosmic-text-version
  regardless of host OS `[unverified — testable]`.
- A cross-platform CI lets a Linux server run paper-trading
  validation without macOS hardware. That's been operator-flagged in
  the
  [`cockpit-cross-platform` backlog candidate](../backlog.md#queue).

**Concrete falsifier the operator can fund.** Spend 1 dev-day adding
GitHub Actions Linux + Windows runners to the v0.1 harness. Either
they produce byte-identical baselines (D3 was overcautious; revisit),
or they drift (D3 confirmed; document the drift class for future
reference). Either outcome retires uncertainty.

### 2.7 Accessibility as a testing surface — the load-bearing pivot

Tying together § 2.2 and § 2.5: an AccessKit tree is **the agent's
DOM**. The test assertion *"the kill button is labeled 'STOP
TRADING' and is enabled"* needs no pixels and no `theme.rs` knowledge:

```text
let tree = ui::a11y_tree(&cockpit);
let kill_btn = tree.find_by_label("Stop trading").unwrap();
assert_eq!(kill_btn.role, Role::Button);
assert!(kill_btn.is_enabled());
assert!(kill_btn.bounds().h >= 32.0);  // tap target minimum
```

Since iced has not merged AccessKit, we cannot use the upstream path.
Two viable approaches:

- **Approach A — PR AccessKit support to iced.** Big project,
  community win. Months of calendar time. Not v1 path.
- **Approach B — In-repo shadow tree.** Author a
  `crates/ui/src/a11y.rs` module that walks `Cockpit` + the current
  screen's `view()` and emits an
  [`accesskit::TreeUpdate`](https://docs.rs/accesskit) by hand. The
  `view()` tree shape is small and stable (~5 screens × ~10 widgets
  each = ~50 nodes total). The cost is one annotation per widget
  ("this is a `Role::Button` with label X"). One developer-week.

Approach B is what I recommend. It builds an
**agent-readable structural shadow** that the harness queries with
zero pixels involved, ratifies the iced 0.14 limit (no upstream
AccessKit), and creates a forcing function for future widgets:
every new widget added must publish its semantic role. The same tree
later becomes the basis for the real iced AccessKit PR when it lands.

### 2.8 Recorded / replayed sessions

The bootstrap fires synthetic `cursor_positions` through
`dispatch_canvas_event_for_test`. A natural extension is
**input-event journals**: capture an operator's real session as a
`Vec<(timestamp, Message)>` (or upstream `iced::Event`), commit it
as a fixture, replay it deterministically against `ui::state::update`,
assert end-state invariants.

The closest analogue in the JS world is [Playwright trace
viewer](https://playwright.dev/docs/trace-viewer) and
[Replay.io](https://docs.replay.io/) (deterministic browser). For
iced, the journal is one struct per message — `serde_json` over the
existing `Message` enum is enough to serialize/deserialize.

The single highest-value session to record: **the chart-canvas-overhaul
incident**. Replaying that session against the current `state.rs`
would let us assert "this regression class cannot return"
mechanically.

### 2.9 VLM / LLM-as-oracle — surfacing the sharp downside

Anthropic's
[Vision docs](https://platform.claude.com/docs/en/build-with-claude/vision)
support PNG inputs; Claude Sonnet 4.6 input is $3/MTok with prompt
caching saving up to 90%
([Anthropic pricing](https://www.anthropic.com/claude/sonnet)). A
single 3360×1890 PNG is ~1500 input tokens worst case `[unverified]`.
At 50 snapshots × 3 viewports × 1 judge call = 150 calls/feature,
~225k input tokens, ~$0.68 of input cost per feature run. Output is
short ("PASS/FAIL + one-line reason"), negligible.

Cost is **not** the bottleneck. The bottleneck is **flakiness**:

- **Even with temperature 0 + fixed seed, VLM outputs are not
  guaranteed identical**
  ([Vincent Schmalbach](https://www.vincentschmalbach.com/does-temperature-0-guarantee-deterministic-llm-outputs/),
  [vLLM determinism discussion](https://github.com/vllm-project/vllm/discussions/17166),
  [Anthropic docs](https://platform.claude.com/docs/en/build-with-claude/vision)
  themselves note even temperature 0 is "not fully deterministic").
- **VLMs hallucinate on low-quality, rotated, or very small images
  under 200 px** — direct Anthropic guidance
  ([Vision](https://platform.claude.com/docs/en/build-with-claude/vision)).
- **Categorical questions trip on synonyms** ("bicycle" vs "bike" —
  [VLM evaluation false-negative](https://learnopencv.com/vlm-evaluation-metrics/)).
- The
  ["Beyond Screenshots" arxiv paper](https://arxiv.org/html/2604.26148)
  explicitly mitigates flakiness with **10-trial averaging + answer-
  order randomization**. We could do the same: each judge call is N=3
  to N=5 with majority vote. Cost stays under $5/feature.

**The class of failure VLM-as-oracle uniquely catches** is the
"uniformly broken layout matches the broken baseline" case — i.e.
the regression class where the broken state is committed as the
baseline once, and pixel-diff defends it forever.

**Failure modes to surface:**

- **Prompt drift.** Anthropic updates Sonnet; oracle's behavior
  changes mid-feature. Pin the model version (`claude-sonnet-4-6`
  not `claude-sonnet-latest`) and lock prompts via
  `crates/llm/src/prompts/` (existing pattern in
  [crates/llm](../../crates/llm)) hashed into the test fixture so a
  prompt change shows up as a baseline change.
- **Cost blowout.** Cap each feature at $2 of judge spend; fail the
  test on overflow.
- **Operator-trust corrosion.** A judge that flakes 1% of runs
  trains the operator to ignore it. **Mitigation: judge runs in
  shadow mode for two weeks before its FAIL gates anything.** Log
  disagreements between byte-diff PASS and judge FAIL; review at the
  end; only then promote judge to gating.
- **Reach-around abuse.** A future agent learns to "talk the judge
  out of" failing. Mitigation: judge is read-only relative to the
  agent. The agent cannot edit the prompt; the prompt is locked.

The project already has [`crates/llm`](../../crates/llm) with three
provider impls (Anthropic / OpenAI / Ollama), prompt caching, and a
`BudgetedProvider` decorator. **Bolting a VLM judge onto the harness
costs ~2 dev-days** plus the prompt design.

**Concrete shape:**

```text
trait VisualJudge {
    fn assert(&self, screenshot: &Screenshot, claim: &str) -> Verdict;
}

// In a test:
let snap = iced_test::screenshot(&program, ...);
let judge = AnthropicVisualJudge::new("claude-sonnet-4-6", temperature=0.0, n_samples=3);
judge.assert(&snap, "The tooltip card is visible near the centered marker.")?;
```

The judge call ships only on failure of the pixel-diff (i.e. as a
**second-opinion forensic**, not the primary oracle). On PASS-PASS,
nothing extra runs. That keeps cost in proportion to test failures.

### 2.10 State-invariant tests vs view tests — quantifying the gap

Cockpit's `ui::state::update` lives at
[`crates/ui/src/state.rs:1`](../../crates/ui/src/state.rs) (2472 LOC).
The Message enum at line 969 has ~60 variants `[grep count: pub enum Message and counting variants, approximate]`.

The bootstrap and prior feature briefs ship:

- 68 panel/text insta snapshots (pure state-to-string, doesn't render).
- 4 visual snapshots (renders + pixels).
- 7 canvas hit-test grid tests.
- 13 integration tests in `tests/*.rs` (mix of update-driven and
  rendering-driven).

That's ~92 tests, of which ~20 exercise `update`. **For ~60 Message
variants, ~20 update-driven tests are ~33% direct coverage and
nothing tells us which are which.**

Mutation testing with [`cargo-mutants`](https://mutants.rs/) over
`ui::state::update` would surface this mechanically: mutate the
arm-body of every `Message` match arm, run the test suite, report
which arms have surviving mutants. A surviving mutant means the
behaviour of that arm is not actually constrained by any test.
Expected first-run output `[unverified — testable]`: 30+ surviving
mutants. That's a clear ROI signal for the analyst/architect of any
follow-on `update`-coverage feature.

Cost: cargo-mutants is slow (runs the test suite per mutant). For
the `update` function specifically, scoping it via `--in-place`
+ `--package ui --file crates/ui/src/state.rs` keeps the run
bounded (estimate: 30 minutes on the developer's machine
`[unverified]`).

### 2.11 Determinism — the failure-mode catalogue

The bootstrap's H1 (tiny-skia byte-determinism) holds for the
Charts-only fixture. Extrapolation to the full screen surface is
weak. Sources of non-determinism the harness has not yet hit:

- **System fonts.** The bootstrap embeds fonts via PR #2698; the
  failure mode the dev-note's §7 names (system emoji fallback,
  HarfBuzz drift) is real. Mitigation: the cockpit ships
  `theme::FONT_SANS = Inter`, `theme::FONT_MONO = JetBrains Mono` —
  these are bundled in `assets/`. **Add a test asserting
  `iced::Settings::default_font` resolves to a bundled font
  fingerprint**, not a system fallback. One-line fixture.
- **Locale.** Number formatting uses the runtime locale separator
  (per
  [`ui-design-principles.md § Voice and copy`](../ui-design-principles.md#voice-and-copy)).
  A test machine in `de_DE.UTF-8` renders `1.234,56`; in `en_US.UTF-8`
  it renders `1,234.56`. Same numbers, different pixels. Mitigation:
  `crates/ui/tests/setup.rs` sets `setlocale(LC_ALL, "C.UTF-8")` at
  test entry. Three-line fixture.
- **Animation.** Per
  [`ui-design-principles.md § Motion`](../ui-design-principles.md#motion),
  bounded animations exist (fade-in on first paint, focus-ring pulse,
  panel slide, spinner). `iced_test::screenshot(..., Duration::ZERO)`
  ostensibly skips animation; the H2 caveat shows it sometimes does
  not (the chart tooltip's canvas-internal hover state). **Make
  animation-time injectable** under `#[cfg(test)]` — the test
  override pattern that already works for
  `time::UtcOffset::current_local_offset` in
  `crates/ui/src/widgets/chart.rs:125-160` (cited in
  [bootstrap feature.md R4.2](../ui-test-harness-bootstrap/feature.md#r4--determinism-contract-cross-cutting)).
- **Renderer-tier drift.** v0.1 ships tiny-skia; the
  `cockpit-performance-and-input-responsiveness` backlog item
  ([backlog.md](../backlog.md)) flags H-PERF-4 (revisit wgpu).
  Anything that bumps the renderer will reshape every baseline.
  Mitigation: tie baselines to a renderer-fingerprint string in
  the PNG metadata; CI rejects mismatches loudly.

### 2.12 Mutation testing — the focused ask

Per § 2.10. Tactical scope:

```bash
cargo mutants --package ui --file crates/ui/src/state.rs --no-shuffle
```

Cost: one feature-week. Expected output: a punch-list of `update`-
arm tests to write. Pairs naturally with property-based testing
(§ 2.3) — proptest covers correctness under arbitrary sequences;
mutants covers coverage gaps for individual arms.

### 2.13 Storybook-equivalent — the highest-ROI agent artifact

A `cargo run --bin ui-gallery` that renders every widget × every
state × every viewport. The output is **one scrolling page** the
agent (or the operator) screenshots once. The screenshot contains
50+ cells. One snapshot test, 50+ baselines, one operator review.

This is what
[Chromatic + Storybook](https://www.chromatic.com/storybook) sell
for the JS world. The Rust ecosystem has nothing equivalent for
iced
([2025 Rust GUI survey](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html));
egui ships its
[demo app](https://github.com/emilk/egui) (`cargo run --release -p
egui_demo_app`) which is structurally the same idea. We have
**`crates/ui/src/fixtures.rs` (1096 LOC of state builders)**
already — the gallery is `fixtures` × `widgets` × `viewports` × a
trivial dispatch loop. Estimated cost: one dev-week for the bin +
two weeks of iterative state expansion as ui-designer adds new
widgets.

Why it is the highest-ROI agent artifact: an agent that doesn't
need to navigate a live binary can capture-screenshot one URL-
shaped path and ask "is anything visually wrong with this whole
gallery?". The VLM-judge cost amortizes over 50 cells per call.

### 2.14 Operator-grade integration tests — generalizing what we already have

`crates/ui/tests/cockpit_live_kill_button_writes_audit.rs` is the
exemplar: build an in-memory `audit::Ledger`, drive the kill
button through `ui::state::update`, assert the audit row landed.
Tests **end-to-end behavior with no rendering**.

The pattern generalizes to:

- **Tape-row click → audit modal** ([backlog.md Lumen
  Phase 6](../backlog.md#queue) — already promoted from
  ui-design-principles.md Q2).
- **Strategy row click → strategy-events history.**
- **Chart marker click → fill modal** (already exists at
  [`chart_marker_click_opens_modal.rs`](../../crates/ui/tests/chart_marker_click_opens_modal.rs)).

Each follows the shape: ledger fixture in memory, drive `Message`
through `update`, assert the resulting `Cockpit` state matches the
expected post-condition. **Zero pixels, zero VLM, zero
flakiness.** Each test is ~50 LOC and ~50 ms to run.

The dev-note's Layer 1..N model does not name "integration tests
through `update` without rendering" as its own layer. Promoting
this to a first-class layer (Layer 0 — pure update; renders
nothing) reorders the value pyramid:

```
Layer 0 — Pure update + property tests (NEW — promote)
Layer 1 — Insta text snapshots of state (existing — 68 baselines)
Layer 2 — Widget-tree accesskit assertions (NEW — kittest pivot)
Layer 3 — Pixel snapshots at viewport matrix (existing — 4 baselines)
Layer 4 — Canvas hit-test grid (existing — 7 tests)
Layer 5 — Perceptual diff forensics on Layer 3 failure (existing — image-compare)
Layer 6 — VLM judge on a small set of high-value claims (NEW — Sonnet judge)
Layer 7 — Live-app inspection via MCP shim (NEW — stretch)
```

Note "live-app inspection" is at the top — most expensive,
least-frequently run, only when nothing else has surfaced a real
incident. The current plan flattens this hierarchy and overweights
Layer 3.

### 2.15 Feature-completeness scaffolding — the reachability question

The operator's framing includes "feature-completeness." A widget
or screen is "feature-complete" when every reachable user
interaction has a test.

Three mechanical coverage signals to wire up:

- **`cargo llvm-cov` per crate.** Existing tool, easy to add.
  Surfaces unhit branches in `update` and `view`. Coverage
  thresholds become CI gates.
- **`Message::*` exhaustiveness check.** A test that does
  `let _ = match Message::dummy() { /* every arm */ }; ` is what
  [`ui-design-principles.md § Consistency`](../ui-design-principles.md#consistency-enforcement)
  already mandates ("`Message::*` is exhaustive — No `_ => {}`
  catch-all"). Extend: a `grep`-based test that fails if a
  `Message::Variant` is added without a test reference to it
  elsewhere in `crates/ui/tests/`. ~30 LOC of shell.
- **Widget-tree coverage.** Once Layer 2 (accesskit shadow tree)
  lands, the gallery enumerates every widget. A test asserts the
  set of widgets in the gallery equals the set of widgets exported
  from `crates/ui/src/widgets/mod.rs`. Drift fails loud.

### 2.16 CI ergonomics — solving "agent proposes, human approves"

The bootstrap's "no `cargo insta review` for binary snapshots" gap
(per
[feature.md ## Design § cargo insta review integration gap](../ui-test-harness-bootstrap/feature.md#cargo-insta-review-integration-gap))
is the friction point. `insta` does support binary snapshots
([insta docs](https://insta.rs/docs/snapshot-types/)) and CI mode
fails on out-of-date snapshots
([Snapshot Testing primer](https://www.rustprojectprimer.com/testing/snapshot.html)).
The gap is `iced_test::Screenshot::rgba` has no public byte
accessor.

Two paths:

- **Path A — upstream PR.** Send an `into_bytes() -> Vec<u8>`
  method to `iced_test::Snapshot`. Calendar time uncertain.
- **Path B — adapter crate.** A `crates/iced-test-bytes/` thin
  crate that decomposes `Screenshot { rgba }` into a `Vec<u8>` we
  pass to `insta::assert_binary_snapshot!`. The rgba field is
  already public (per the bootstrap's developer pass T4014); no
  upstream change needed. ~30 LOC.

Path B is what the bootstrap deferred to "week 2." It's worth
~half a day and closes the human-approval loop: `cargo insta
review` opens the PNG diff in the operator's VS Code, one
keystroke accepts.

For multi-agent "approval comments" — the [Chromatic
approach](https://www.chromatic.com/storybook) — we're single-
operator and self-hosted (per
[`ui-design-principles.md`](../ui-design-principles.md#whats-not-in-scope)):
no team-share need. The local `insta review` IS the workflow.

## 3. New proposals — three+ ideas not in the existing dev-note

Each proposal carries (a) what, (b) why it solves operator pain,
(c) cheapest credible MVP, (d) what could go wrong.

### 3.1 Live-inspect MCP shim — `ui-inspect-mcp`

**(a) What.** Embed a minimal Model Context Protocol server into
`cockpit_live` (and `cockpit --features fixtures`) that exposes a
read-only API the orchestrator queries: `get_widget_tree()`,
`screenshot()`, `find_by_label(s)`, `get_widget_bounds(id)`. Mirrors
Slint's
[`i-slint-backend-testing`](https://docs.rs/i-slint-backend-testing/)
MCP server. Listens on `localhost:<port>` only when
`COCKPIT_MCP=1`.

**(b) Why.** The
[AGENT.md ## Capability boundaries](../../AGENT.md#capability-boundaries)
forbids sub-agents from launching the cockpit. The orchestrator
*can* — but doing so is currently a manual screencap. An MCP shim
lets the orchestrator answer "is the tooltip visible at the
operator's real viewport" by reading a structured response, not
eyeballing a PNG. This is the structural cure for the founding
v1.10.0 bug.

**(c) MVP.**

- `crates/ui/src/inspect/mod.rs` — 200 LOC of `axum`-or-`hyper`
  HTTP server that exposes 4 endpoints.
- Lifetime: server starts when env var set; tied to the cockpit's
  iced runtime. Process exit kills it.
- Read-only by construction: no `set_widget(...)` endpoint, no
  `Message::*` injection (yet). Inspection only.
- Hook into the `inspect-mcp` feature gate so production binaries
  never compile it in.
- ~4 dev-days.

**(d) Risks.**

- **Security.** Even `localhost`-only HTTP is a risk on a shared
  machine. Mitigation: bind to `127.0.0.1` only + require an
  env-var-supplied token in every request.
- **Compatibility with iced's reactive rendering.** iced 0.14
  reactive-renders only on state change; the inspect server needs
  to access state outside the iced render cycle. Mitigation: snapshot
  state via `Arc<Mutex<...>>` shared with the iced main loop; reads
  always non-blocking.
- **Feature creep.** "While we have an MCP shim, why not let the
  agent click buttons?" That violates capability boundaries. Lock
  the shim to read-only in the brief; bump to v2 only with explicit
  operator decision.

### 3.2 VLM-as-second-opinion judge — `ui-vlm-judge`

**(a) What.** A `Judge` trait with an Anthropic implementation
backed by the existing [`crates/llm`](../../crates/llm) provider.
Three locked claims:

1. *"The tooltip card is visible somewhere in the rendered frame."*
2. *"No two opaque foreground elements overlap by more than 50% of
   either's bounding box."*
3. *"Every text label has at least 4.5:1 contrast against whatever
   is behind it."*

Runs **on test failure only**, as a forensic second opinion that
either confirms the pixel diff is meaningful or flags the failure as
"baseline is itself broken."

**(b) Why.** Two failure classes pixel-diff cannot catch:

- The committed baseline IS the broken state (silent corruption).
- A future palette refactor flips contrast below 4.5:1 (the kind of
  regression we already had to add `consistency.rs` for).

The judge is the second opinion. It does not gate by default; it
**produces a second-opinion artifact** that the evaluator and
presenter reference.

**(c) MVP.**

- `crates/ui/tests/fixtures/visual_judge.rs` — wraps
  `crates/llm::AnthropicProvider`.
- Pin model `claude-sonnet-4-6`, temperature 0, N=3 samples
  majority vote.
- Budget cap: $0.50 per test run via `crates/llm::BudgetedProvider`.
- Shadow-mode period: 2 weeks. Log disagreements; do not gate. After
  shadow period, operator reviews disagreements and decides
  promotion.
- ~3 dev-days incl. prompt iteration.

**(d) Risks.**

- **Flakiness.** Mitigated by N=3 majority vote. Track per-claim
  flake rate; demote any claim above 1% flake.
- **Cost.** Capped per run. Budget exhaustion fails loud.
- **Prompt drift.** Pin model + prompt hash; prompt change ≡
  baseline change.
- **Operator-trust corrosion.** Shadow mode for 2 weeks before any
  gate; explicit operator approval to promote.
- **Reach-around abuse.** Prompt is read-only-relative-to-agent;
  baked into `crates/ui/tests/fixtures/visual_judge.rs` as a const
  string with its SHA committed.

### 3.3 Widget gallery binary — `ui-gallery-bin`

**(a) What.** `cargo run --bin ui-gallery` opens a single scrolling
window rendering every widget × every state × every viewport on
one page. Drives all `crates/ui/src/fixtures.rs` state-builders
through every screen in turn, captured as named sections in the
scroll.

**(b) Why.** The agent's coverage problem is "I don't know what
exists." A gallery enumerates the surface. One screenshot
captures 50+ cells. The VLM judge cost amortizes; the operator's
visual review is "scroll this one page and confirm." The Rust GUI
world has no equivalent today
([2025 Rust GUI survey](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html));
egui's demo app is the closest analogue
([egui repo](https://github.com/emilk/egui)).

**(c) MVP.**

- New `[[bin]] name = "ui-gallery" required-features = ["fixtures"]`.
- ~200 LOC dispatching across `fixtures::*` × widget × viewport.
- Reuses the existing `theme.rs` + `widgets/*` exports.
- Pairs with snapshot tests: each gallery section is one named
  insta binary snapshot.
- ~3 dev-days for v0, 1 dev-day per quarter of maintenance.

**(d) Risks.**

- **Drift between gallery and real cockpit.** A widget rendered in
  isolation may behave differently inside the real screen-grid
  composition. Mitigation: each gallery cell is annotated with the
  screen it lives in and the constraints of that screen; tests run
  both the gallery cell AND the real screen and compare.
- **Maintenance burden.** A new widget = a new gallery entry.
  Mitigation: a `crates/ui/src/widgets/mod.rs`-exhaustiveness test
  (per § 2.15) fails when a widget exists but no gallery entry
  exists.

### 3.4 Stretch — Update + property-based state-machine harness — `ui-update-proptest`

**(a) What.**
[`proptest-state-machine`](https://crates.io/crates/proptest-state-machine)
over `ui::state::update`. A `ReferenceCockpit` invariant model;
randomized `Message` sequences; assertions after every step.

**(b) Why.** Quantified in § 2.10: ~40 untested `Message` variants
out of ~60. Property tests reach the long tail no example-based
test covers.

**(c) MVP.**

- `crates/ui/tests/update_proptest.rs` — one
  `prop_state_machine!` macro invocation.
- 5 invariants to start: kill monotonicity, no-cross-screen-leakage,
  PanelState arm reachability, subscription-error recoverability,
  audit-write idempotency.
- ~5 dev-days for the harness + invariant authoring; ~1 day/quarter
  for new-message-variant maintenance.

**(d) Risks.**

- **Shrinking interacting with iced runtime state.** No iced
  runtime is involved (pure `update`), so shrinking is just
  shrinking the message sequence. Safe.
- **Invariant authoring is the work.** The first 5 invariants
  carry most of the value; later ones diminish.
- **proptest test-time blowup.** Cap with `proptest_config!(.. .cases = 256)`.

### 3.5 Stretch — AccessKit shadow tree — `ui-a11y-shadow`

**(a) What.** Author `crates/ui/src/a11y.rs` that walks the current
`Cockpit` + the active screen and emits an
[`accesskit::TreeUpdate`](https://docs.rs/accesskit). Wire to
[`kittest`](https://docs.rs/kittest/) for assertion-driven tests
that operate purely on the tree.

**(b) Why.** § 2.7. The agent's "DOM." Closes the
contrast/legibility/coverage classes. Establishes the foundation
for the eventual iced upstream AccessKit merge.

**(c) MVP.**

- `a11y.rs` covers the cockpit's existing widget surface
  (~50 nodes).
- `crates/ui/tests/a11y_tree.rs` asserts the tree shape.
- `kittest` integration via the documented `NodeT` trait.
- ~7 dev-days (the work is per-widget annotation, plus one
  integration test class).

**(d) Risks.**

- **Drift between `view()` and `a11y()`.** Two render functions to
  keep in sync. Mitigation: clippy lint that flags any new widget
  added without a matching `a11y_node()` impl.
- **Doesn't help upstream-merging iced AccessKit.** True. The work
  is harness-local; the iced PR is a separate, larger lift.
- **Performance.** Building a tree every test step has cost. For
  ~50 nodes, negligible (sub-millisecond).

### 3.6 Stretch — Recorded session journal — `ui-session-journal`

**(a) What.** Add `cockpit_live --record-journal <path>` that
serializes every `Message` (with the dispatched-at timestamp) into
a TOML file. Add `cargo test --test journal_replay -- <path>` that
deserialises, replays, asserts the final state matches a committed
golden snapshot.

**(b) Why.** Future regressions of the chart-canvas-overhaul shape
can be reproduced from the operator's real session bytes-for-bytes.
The audit ledger already captures most events for post-mortem; the
journal is the input-side equivalent. Comparable to
[Playwright trace viewer](https://playwright.dev/docs/trace-viewer)
and [Replay.io](https://docs.replay.io/) — but for our message
event log instead of browser traces.

**(c) MVP.**

- `Message` already derives `Serialize` / `Deserialize` in most
  arms; gaps audited.
- New `recorder` middleware in `ui::live` that taps the event
  stream before `update`.
- ~3 dev-days for the recorder, ~1 dev-day for the replay test
  scaffold.

**(d) Risks.**

- **Time-of-day non-determinism.** Replays must inject a fake clock.
  Pattern already exists at
  [`chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs).
- **Bus-side state.** A journal only captures messages, not the
  outside world's state. Mitigation: an "external-state" fixture
  saved alongside the journal (`journal.toml` + `fixtures.toml`).
- **Privacy.** Recorded sessions may capture sensitive data
  (positions, P&L). Treat journal files like audit DB excerpts —
  outside `git`, in `target/`.

### 3.7 Stretch — Mutation testing pass — `ui-mutants-pass`

**(a) What.** One-time `cargo mutants --package ui --file
crates/ui/src/state.rs` run. Output: a triage report of surviving
mutants in `update`. Feeds back into § 3.4 (proptest authors
target the gaps).

**(b) Why.** § 2.12. Surfaces test-coverage gaps mechanically.

**(c) MVP.** ~1 dev-day for the run + report. Operator picks the
top 10 mutants worth covering; that's an analyst follow-up.

**(d) Risks.**

- **Slow first run.** Acceptable for a one-shot.
- **Mutants that survive but aren't real bugs.** Manual triage;
  no automation here. Time spent on triage may exceed value.
- **Recurring runs in CI are too slow.** Quarterly cadence at
  most; not a CI gate.

### 3.8 Stretch — Pure-Rust WCAG contrast asserter — `ui-contrast-asserter`

**(a) What.** A `crates/ui/tests/contrast.rs` test that enumerates
every `(fg, bg)` token pair in
[`theme.rs`](../../crates/ui/src/theme.rs) and asserts WCAG 2.1
ratios per the table in
[`ui-design-principles.md`](../ui-design-principles.md#accessibility-minimums).

**(b) Why.** § 2.5. Catches the entire "contrast regression on
palette refactor" class for half a dev-day.

**(c) MVP.**

- 20 LOC of WCAG formula.
- ~50 LOC of pair enumeration.
- Lives next to `crates/ui/tests/consistency.rs`.
- ~0.5 dev-days.

**(d) Risks.**

- **Light-mode vs dark-mode.** Two pairings per token. Already
  modelled via `ModeColor`. Easy.
- **Brand-color compromise.** A palette change with WCAG-failing
  on-brand color → operator decision, not a test bug. Add the
  test in WARN mode for two weeks first.

## 4. Agent contract changes

Concrete changes to the `.claude/agents/*.md` files and to the
[AGENT.md](../../AGENT.md) workflow that the proposals above
require.

### 4.1 `tester.md` — emit a structured fail artifact, not just prose

Today's test report template is markdown
([test-report.md template](../../.claude/skills/rust-test/templates/test-report.md)).
On a `matches_image` failure, the report cites the diff PNG path.
The operator opens it in `Preview.app` separately.

Proposed stanza addition (~10 lines in `.claude/agents/tester.md`):

```text
## Visual failure artifacts

On any matches_image / kittest assertion / visual_judge FAIL, the
test-runner additionally writes:

  spec/<slug>/reports/visual-fail-<ts>.html

— a single self-contained HTML page embedding the baseline /
actual / diff PNG triple, the assertion that fired, the relevant
file:line, and (if Layer 6 VLM judge is enabled) the judge's
verbatim verdict + per-sample disagreement.

The evaluator's read trace MUST include this HTML for the
default-FAIL contract to pass.
```

Cost: ~1 dev-day to author the HTML template + a 50-LOC helper in
`crates/ui/tests/fixtures/`.

### 4.2 `ui-designer.md` — render-preview before handoff

Today's ui-designer handoff is "I wrote the code, here's the
test cite." Per the new
[capability boundaries](../../AGENT.md#capability-boundaries) the
ui-designer cannot launch the cockpit. But it CAN run the gallery
bin's snapshot test and cite the gallery section it touched.

Proposed stanza addition to `.claude/agents/ui-designer.md`:

```text
## Render-preview gate

Before HANDOFF → tester, the ui-designer MUST cite the gallery
snapshot section corresponding to the widget(s) it modified, and
must have run the gallery snapshot test (`cargo test -p ui --test
gallery_snapshots <section>`) at least once.

If the gallery has no section for the widget being modified, the
ui-designer adds one in the same diff. Adding to the gallery is
mandatory for any new widget under `crates/ui/src/widgets/`.
```

Cost: depends on `ui-gallery-bin` shipping first (§ 3.3).

### 4.3 `presenter.md` — embed the VLM judge verdict

Today's presenter deck cites test cite + screenshots. Proposed
stanza addition to `.claude/agents/presenter.md`:

```text
## Visual-judge verdict (when Layer 6 is enabled)

For any feature whose verification stack includes the visual_judge
Layer 6, the presenter's deck includes:

  ## Visual judge

  - Claim 1 — <claim text> — N=3 majority: <PASS/FAIL>, per-sample [v1, v2, v3]
  - Claim 2 — ...
  - Claim 3 — ...

  Operator acks both byte-diff PASS AND visual-judge PASS in the
  approval block.
```

Cost: ~0.5 dev-day for the template update.

### 4.4 `analyst.md` — write reachability hypotheses

The analyst already authors `## Hypothesis register` rows. Add a
new convention: any feature that adds a widget surface includes
**at least one reachability hypothesis**:

```text
- H-REACH-N — every operator-reachable interaction on this widget
  is covered by at least one test in crates/ui/tests/.
  - Falsifier: the gallery-snapshot test exists; the kittest tree
    assertion finds the widget by label; at least one update-driven
    integration test fires the widget's Message variant.
```

This pulls reachability into the agent contract rather than a
post-ship audit.

### 4.5 The orchestrator — owns the inspect-mcp queries

Per § 3.1, the orchestrator (not the sub-agents) runs the
inspect-mcp client. AGENT.md's
[capability map](../../AGENT.md#capability-map) gets one new row:

| Capability | Owner | Allowed for sub-agents? |
|---|---|---|
| `inspect-mcp` HTTP queries to a running cockpit | **orchestrator** | **no** |

Same reasoning as `screencapture` / `osascript`. The shim runs in
the cockpit; the cockpit is operator-territory.

## 5. Prioritized rollout

The plan reorders the existing weeks 2-4 by ROI and dependency.

### 5.1 Idea table

| # | Idea | Layer (new model) | Prereq | Effort | ROI | Risk | Source |
|---|---|---|---|---|---|---|---|
| A | Pure-Rust contrast asserter (`ui-contrast-asserter`) | L0 — pure state | none | S (0.5d) | Med | Low | § 3.8 |
| B | Update + proptest harness (`ui-update-proptest`) | L0 — pure state | none | M (5d) | High | Low | § 3.4 |
| C | Storybook-equivalent gallery bin (`ui-gallery-bin`) | L1 + L3 | none | M (3d) | **High** | Low | § 3.3 |
| D | Canvas-state seeding (queued in backlog) | L3 — pixel | bootstrap v0.1 | S (1d) | Med | Low | [backlog.md `ui-test-harness-canvas-state-seeding`](../backlog.md#queue) |
| E | Full-widget viewport matrix (existing week-2) | L3 — pixel | C (gallery) | M (4d) | Med | Med | existing dev-note §6 |
| F | Insta binary-snapshot adapter (`iced-test-bytes`) | L3 — pixel | none | S (0.5d) | Med | Low | § 2.16 |
| G | Locale + font determinism fixtures | L3 — pixel | none | S (1d) | High | Low | § 2.11 |
| H | AccessKit shadow tree + kittest (`ui-a11y-shadow`) | L2 — widget tree | none | L (7d) | **High** | Med | § 3.5 |
| I | Mutation testing one-shot (`ui-mutants-pass`) | L0 → input to B | B in shadow | S (1d) | Med | Low | § 3.7 |
| J | Recorded session journal (`ui-session-journal`) | L0 — pure state | none | M (4d) | Med | Low | § 3.6 |
| K | VLM-judge shadow mode (`ui-vlm-judge`) | L6 — judge | E or H landed | M (3d) | High after shadow | High flakiness | § 3.2 |
| L | Inspect-MCP shim (`ui-inspect-mcp`) | L7 — live | feature-gated | M (4d) | Med | Med (security) | § 3.1 |
| M | Evaluator subagent + PreToolUse hooks (existing week-3) | workflow | adoption of A-H | M (3d) | Med | Low | existing dev-note §4.2 |
| N | CI on macOS runner (existing week-4) | workflow | E + F + G | M (5d) | High | Low | existing dev-note §6 week 4 |
| O | Cross-platform CI falsifier (revisits D3) | workflow | N | S (1d) | Med | Low | § 2.6 |

### 5.2 Recommended sequencing

I'd run these in **three two-week cycles** rather than the original
four sequential weeks. Each cycle ends with one shipped feature
the operator approves.

**Cycle 1 (weeks 2-3 of the bootstrap timeline) — "L0 + tooling"**

Land A, B, C, D, F, G in parallel where lanes allow.

- Lane 1 — A + G (small fixtures, one dev-day total).
- Lane 2 — B (proptest harness — independent).
- Lane 3 — C (gallery bin — independent).
- Lane 4 — D + F (canvas-state seeding + insta binary adapter —
  both close existing bootstrap gaps).

This cycle ships: WCAG asserter, proptest invariants, gallery,
canvas-state seeding closure of V15 render half, `cargo insta
review` reopened for binary snapshots, locale-fixed test setup.

**Cycle 2 (weeks 4-5) — "L2 + L3 expansion"**

Land H, E in sequence.

- Lane 1 — H (accesskit shadow tree + kittest) — 7d.
- Lane 2 — E (full-widget viewport matrix) — 4d, blocks on C.

Cycle 2 ships: widget-tree assertion layer, viewport matrix
across every widget, two new layers of the test pyramid.

Mid-cycle, run I (mutation testing one-shot) to grade B + H
coverage. Operator picks the top-10 surviving mutants; analyst
files a follow-up brief.

**Cycle 3 (weeks 6-7) — "Forensics, judge, CI"**

Land K, N, O in sequence.

- Lane 1 — K (VLM judge) — 3d + 2 weeks shadow mode.
- Lane 2 — N (macOS CI) — 5d.
- Lane 3 — O (Linux + Windows CI falsifier to revisit D3) — 1d.

L (inspect-MCP shim) and J (session journal) and M (evaluator
hooks) defer to **cycle 4 or later** — they are workflow/operator-
experience improvements that depend on operator load not the
harness's mechanical coverage.

### 5.3 Keep / drop / replace against the existing weeks 2-4 plan

| Existing item | My recommendation | Reasoning |
|---|---|---|
| Week 2 — viewport matrix across all widgets + canvas hit-test sweep | **KEEP, REORDER** as Cycle 2 / item E. **Add** the gallery (C) as a prerequisite. | E is good. But E without a gallery means one snapshot per widget per viewport ≈ 50 baselines per cycle, hard to review. Gallery (C) compresses the review to one scroll. |
| Week 3 — evaluator subagent + PreToolUse hooks | **DEFER to cycle 4+** (item M, after harness mechanical coverage matures). | The evaluator-split workflow already shipped via the bootstrap's AGENT.md ## Test-runner / evaluator split. The PreToolUse hook is "structural enforcement of an already-procedural rule." Other items (A through K) close real coverage gaps; M closes a procedural hardening one. Schedule M after the harness is solid; before then, the procedural rule is enough. |
| Week 4 — CI workflow + presenter integration | **KEEP, MOVE to cycle 3 / item N**. **Add** the cross-platform falsifier (O) as a 1-day follow-up. | CI is high-ROI but only meaningful once the harness has enough breadth that "CI green" means something. Cycles 1+2 must land first. |

### 5.4 Decisions to revisit

Per the operator's framing ("if you want to revisit a D-block decision,
flag it explicitly"):

- **D3 — macOS-only CI.** Revisit per § 2.6 + item O. Cost: 1 dev-day
  added to the CI brief. Outcome: either D3 was overcautious (saved
  the project a cross-platform follow-up) or D3 was right (documented
  for future). Either outcome is worth the dollar.

The other D1-D5 decisions (adopt as block, dev pass for week 1,
chart-canvas-overhaul resolution, AGENT.md amendments) are
already proven by the bootstrap ship and need no revisit.

## 6. Open questions for the operator

> **DECIDED 2026-05-15 (operator):** All 6 analyst defaults
> accepted as a block. Each Q-* below is marked `LOCKED → <default>`
> inline. See Changelog entry of the same date for the lock event.

These shape the briefs of whichever items the operator schedules
next.

1. **Q-VLM — Adopt VLM-as-judge at all?** It has a real cost
   ($, flake, trust) and a real benefit (catches "broken baseline
   committed" class). Two-week shadow mode would prove it for ~$10.
   *Default if operator silent: adopt for shadow mode only; promote
   only on operator review of disagreement log.*
   **LOCKED 2026-05-15 → adopt for shadow mode only; no gating
   promotion without operator review of the disagreement log.**

2. **Q-ACCESSKIT — Approach A (PR iced upstream) or Approach B
   (in-repo shadow)?** Approach A unblocks community; Approach B
   ships in 7 days. *Default: B. Operator can revisit later if iced
   upstream lands AccessKit.*
   **LOCKED 2026-05-15 → Approach B (in-repo shadow tree). Revisit
   if upstream iced lands AccessKit.**

3. **Q-MCP — `inspect-mcp` shim now or defer?** It's the most
   architecturally interesting proposal and the most security-
   sensitive. *Default: defer to cycle 4. Cycle 1-3 closes more
   tactical coverage gaps.*
   **LOCKED 2026-05-15 → defer to cycle 4 or later.**

4. **Q-GALLERY-SCOPE — Does the gallery render the LIVE cockpit
   state-builders or a frozen fixtures snapshot?** Live = drifts
   with the cockpit; frozen = matches its baselines forever. *Default:
   reuse `crates/ui/src/fixtures.rs` directly so the gallery and the
   cockpit see the same state-builders.*
   **LOCKED 2026-05-15 → reuse `crates/ui/src/fixtures.rs` (live
   state-builders, shared with cockpit).**

5. **Q-D3-REVISIT — Run the cross-platform CI falsifier in cycle 3?**
   Cheap; either retires D3 uncertainty or confirms it. *Default:
   yes — 1 dev-day spike adds ~zero risk.*
   **LOCKED 2026-05-15 → yes, run the 1-day falsifier in cycle 3.**

6. **Q-MUTANTS-CADENCE — One-shot mutation testing or quarterly
   cadence?** *Default: one-shot now (item I), then quarterly only
   on `update` arms that grew since the last run.*
   **LOCKED 2026-05-15 → one-shot now, quarterly delta-only after.**

## 7. Failure modes I want surfaced

This dev-note's load-bearing proposals can themselves fail. Honest
list of "if these go wrong, here's how:"

- **VLM judge in shadow becomes a slow-walk that never promotes.**
  The shadow log accumulates, no one reviews it, operator trust
  hardens against the idea. Mitigation: hard deadline at the end of
  the 2-week shadow window. Either promote with one of the operator
  decision-block patterns we already use, or kill the judge entirely.
- **AccessKit shadow tree drifts from `view()`.** Two render
  functions; both have to update on every widget change. Mitigation:
  the clippy lint per § 3.5. If lint enforcement turns out
  unreliable, kill the shadow tree and wait for upstream iced
  AccessKit.
- **MCP shim becomes a security incident.** Defaults that "look
  fine" turn into a live cockpit binding 0.0.0.0 on a customer-
  shared machine. Mitigation: localhost-only + token + feature-gated
  off in production. Document as a non-negotiable in the brief.
- **Gallery diverges from live cockpit.** Adding a widget to the
  gallery but not to a real screen, or vice versa. Mitigation: the
  exhaustiveness test in § 2.15.
- **proptest invariants encode the wrong rules.** A property test
  passes because the invariant is too weak. Mitigation: prefer
  invariants that fail loudly (assertion panics) over invariants
  that compute a tolerance.
- **Mutation testing surfaces 200 mutants and no one triages.**
  Time-box the triage to 1 day. Anything not covered after the
  time-box gets a future-feature label.
- **The operator stops using `cargo insta review` because the binary
  diff is hard to read.** Mitigation: ship the visual-fail HTML
  page (§ 4.1) regardless. The HTML page IS the operator surface
  even when `insta review` exists.

## 8. Sources

External URLs cited inline above; aggregated for the operator's
sweep.

- **iced ecosystem:**
  [iced 0.14 release](https://github.com/iced-rs/iced/releases/tag/0.14.0),
  [iced master CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md),
  [Iced 0.14 Phoronix coverage](https://www.phoronix.com/news/Iced-0.14-Rust-GUI-LIbrary),
  [Iced 0.14 byteiota coverage](https://byteiota.com/iced-0-14-rust-gui-gets-reactive-rendering-time-travel/),
  [Iced 0.14 HN thread](https://news.ycombinator.com/item?id=46185323),
  [iced_test 0.14 docs](https://docs.rs/iced_test/0.14.0/iced_test/),
  [iced PR #2698 headless testing](https://github.com/iced-rs/iced/pull/2698),
  [iced PR #3059 e2e testing](https://github.com/iced-rs/iced/pull/3059),
  [iced issue #552 accessibility](https://github.com/iced-rs/iced/issues/552),
  [iced issue #282 accessibility](https://github.com/iced-rs/iced/issues/282).
- **Property and model-based testing:**
  [proptest-state-machine](https://crates.io/crates/proptest-state-machine),
  [proptest-state-machine docs](https://docs.rs/proptest-state-machine/),
  [Tomáš Zemanovič — State machine testing](https://tzemanovic.gitlab.io/posts/state-machine-testing-with-proptest/),
  [Nikos Baxevanis — Model-based stateful testing](https://blog.nikosbaxevanis.com/2025/01/10/state-machine-testing-proptest/),
  [proptest-stateful](https://github.com/readysettech/proptest-stateful).
- **Visual / perceptual:**
  [image-compare crate](https://docs.rs/image-compare),
  [image_hasher](https://crates.io/crates/image_hasher),
  [abonander/img_hash](https://github.com/abonander/img_hash),
  [archer884/visual-hash](https://github.com/archer884/visual-hash),
  [dssim](https://github.com/kornelski/dssim),
  [jest-image-snapshot SSIM issue](https://github.com/americanexpress/jest-image-snapshot/issues/201).
- **OCR:**
  [leptess](https://github.com/houqp/leptess),
  [ocrs](https://github.com/robertknight/ocrs),
  [rusty-tesseract](https://github.com/thomasgruebl/rusty-tesseract),
  [Transloadit — Rust OCR](https://transloadit.com/devtips/recognize-text-in-images-ocr-in-rust/).
- **Accessibility:**
  [kittest](https://docs.rs/kittest/),
  [kittest crate](https://crates.io/crates/kittest),
  [rerun-io/kittest](https://github.com/rerun-io/kittest),
  [egui_kittest](https://docs.rs/egui_kittest),
  [egui PR #2294 AccessKit](https://github.com/emilk/egui/pull/2294),
  [AccessKit repo](https://github.com/AccessKit/accesskit),
  [AccessKit.dev](https://accesskit.dev/),
  [Bevy PR #18706 AccessKit](https://github.com/bevyengine/bevy/pull/18706),
  [Slint i-slint-backend-testing](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/),
  [WCAG 2.1 contrast minimum](https://www.w3.org/WAI/WCAG21/Understanding/contrast-minimum.html).
- **Cross-platform & determinism:**
  [tiny-skia](https://github.com/linebender/tiny-skia),
  [cosmic-text](https://github.com/pop-os/cosmic-text).
- **VLM / LLM-as-oracle:**
  [Anthropic Vision docs](https://platform.claude.com/docs/en/build-with-claude/vision),
  [Anthropic pricing](https://www.anthropic.com/claude/sonnet),
  [Beyond Screenshots arxiv](https://arxiv.org/html/2604.26148),
  [Prometheus-Vision](https://prometheus-eval.github.io/prometheus-vision/),
  [VLM evaluation metrics — LearnOpenCV](https://learnopencv.com/vlm-evaluation-metrics/),
  [vincentschmalbach — temperature 0 determinism](https://www.vincentschmalbach.com/does-temperature-0-guarantee-deterministic-llm-outputs/),
  [vLLM determinism discussion](https://github.com/vllm-project/vllm/discussions/17166).
- **Snapshot / mutation / coverage tooling:**
  [insta](https://docs.rs/insta),
  [insta binary snapshots](https://insta.rs/docs/snapshot-types/),
  [Rust Project Primer — snapshot testing](https://www.rustprojectprimer.com/testing/snapshot.html),
  [cargo-mutants](https://mutants.rs/),
  [cargo-mutants design](https://github.com/sourcefrog/cargo-mutants/blob/main/DESIGN.md).
- **Production-class workflows:**
  [Chromatic](https://www.chromatic.com/),
  [Chromatic for Storybook](https://www.chromatic.com/storybook),
  [Playwright Trace Viewer](https://playwright.dev/docs/trace-viewer),
  [Replay.io](https://docs.replay.io/),
  [Storybook](https://qaskills.sh/blog/storybook-component-testing-guide),
  [boringcactus 2025 Rust GUI survey](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html),
  [MCP Inspector](https://github.com/modelcontextprotocol/inspector),
  [Chrome DevTools MCP — Chrome for Developers](https://developer.chrome.com/blog/chrome-devtools-mcp).
- **Anthropic agent + harness docs:**
  [Cognition — Don't Build Multi-Agents](https://cognition.ai/blog/dont-build-multi-agents),
  [Devin Review](https://cognition.ai/blog/devin-review),
  [Cursor agent best-practices](https://cursor.com/blog/agent-best-practices),
  [cwc-long-running-agents](https://github.com/anthropics/cwc-long-running-agents),
  [Anthropic — effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents),
  [Anthropic — multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system),
  [Anthropic — computer-use](https://platform.claude.com/docs/en/agents-and-tools/computer-use).

## Changelog

- 2026-05-15 (operator, accepted as block): locked all 6 open
  questions at their analyst defaults — Q-VLM (shadow only),
  Q-ACCESSKIT (Approach B in-repo), Q-MCP (defer to cycle 4),
  Q-GALLERY-SCOPE (reuse `fixtures.rs`), Q-D3-REVISIT (yes, 1-day
  cycle-3 spike), Q-MUTANTS-CADENCE (one-shot now, quarterly
  delta-only after). § 6 stanzas annotated `LOCKED 2026-05-15`.
- 2026-05-15 (analyst): initial draft. Critiques the existing
  [`ui-testing-direction-2026-05-12.md`](ui-testing-direction-2026-05-12.md)
  4-week plan; surfaces 3 blind spots (canvas-state ownership,
  pixels-only oracle, no reachability coverage); proposes 8 new
  ideas across layers L0..L7 of a re-shaped pyramid (pure state /
  text snapshot / widget tree / pixel / hit-test / perceptual /
  judge / live-inspect); recommends three two-week cycles
  replacing the original sequential weeks 2-4; flags D3 (macOS-
  only CI) as the one prior decision worth revisiting via a
  1-day falsifier in cycle 3; opens 6 operator-input questions
  (Q-VLM, Q-ACCESSKIT, Q-MCP, Q-GALLERY-SCOPE, Q-D3-REVISIT,
  Q-MUTANTS-CADENCE). Per the brief's "no code changes, pure spec
  output" constraint, this dev-note is the only output — backlog
  items below queue the schedulable features for operator pick.
