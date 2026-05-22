---
slug: iced-ecosystem-evaluation
status: candidate
owner: architect
updated: 2026-05-13
version: 0.2.0
---

<!-- M0 falsifier sub-agent pass 2026-05-13: H-arch-0 partially falsified
     (markdown gated behind feature flag — table/grid/float/pin reachable
     under current feature set); H-arch-2 RESOLVED-UNFALSIFIED (CONFIRM —
     chart_tooltip is canvas-internal); H-arch-7 partially confirmed (no
     direct table.rs grep available under sandbox; lazy is a separate OFF
     feature in iced_widget, consistent with eager-only table). See
     ## Hypothesis register entries for evidence + grep output. -->


# iced ecosystem evaluation

> **Status:** research + scoping brief. **No code changes, no crate adds.**
> Analyst surveys candidate crates and maps each to a concrete cockpit
> hand-rolled widget or roadmap pain point; architect picks up this brief
> and turns it into falsifiable adoption hypotheses (per
> [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only)).

## Why

**Operator's prompt (2026-05-13):** "Are there iced-adjacent crates we should
adopt instead of hand-rolling more widget code?"

The cockpit's hand-rolled widget surface has grown to **22 widgets totalling
~5.2k LOC** across `crates/ui/src/widgets/` (count from
[`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) directory listing;
LOC totals from `wc -l`). The largest single file is
[`chart.rs`](../../crates/ui/src/widgets/chart.rs) at **1,539 lines** — a
canvas `Program` covering axes, gutters, gridlines, candles/lines, buy/sell
markers, and hover dispatch. The hand-roll trajectory is itself the
operator's concern: each new feature has tended to grow a fresh widget
(volume_histogram, focus_ring, override_risk_veto, journal_transaction_modal
all landed in the last six features). If iced 0.14's native surface or the
community ecosystem can absorb some of that volume **without** sacrificing
the determinism contract (tiny-skia byte-stable snapshots, 11 anchors
byte-identical, no wgpu dep weight), we should know which crates are
adoption candidates **before** the next widget is hand-rolled.

What we hope to learn:

- Which native iced 0.14 widgets we **are not yet using** but could.
- Which third-party iced 0.14 crates are mature, MIT/Apache-licensed,
  tiny-skia-compatible, and map to a concrete hand-rolled widget or queued
  roadmap item.
- Which crates **look** attractive but are SKIP (lag iced version, AGPL,
  hard-require wgpu, abandoned).

This is **scoping only** — the architect spawns adoption hypotheses; the
operator approves which (if any) to convert into adoption features.

## Baseline — what we already pull in

Orchestrator-verified survey 2026-05-12 (commit `d8c3a99` in operator's
research thread; see Section "Out of scope" for why this is not re-surveyed).
`crates/ui` directly declares **2 iced crates**:

- `iced = "=0.14.0"` with features `tiny-skia, thread-pool, advanced, canvas`
  ([`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml))
- `iced_test = "=0.14.0"` (dev-dep, dev-only — landed in
  [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md))

The lockfile carries **34 crates transitively**:

| Family | Count | Members |
|---|---|---|
| iced family | 14 | iced, iced_core, iced_debug, iced_futures, iced_graphics, iced_program, iced_renderer, iced_runtime, iced_selector, iced_test, iced_tiny_skia, iced_wgpu, iced_widget@0.14.2, iced_winit |
| Text / font | 2 | cosmic-text 0.15.0, fontdb 0.23.0 |
| Lyon (vector tess.) | 5 | lyon, lyon_algorithms, lyon_geom, lyon_path, lyon_tessellation |
| Renderers | 3 | tiny-skia 0.11.4 (active), wgpu 27.x family (**unused, ~10–15 MB dead weight** per chart-canvas-overhaul retrospective), softbuffer 0.4.8 |
| Windowing | 3 | winit 0.30.13, raw-window-handle 0.6.2, core-graphics 0.23.2 |
| Plotters (criterion only) | 3 | plotters 0.3.7, plotters-backend, plotters-svg — pulled by **criterion bench-report rendering**, NOT a chart-UI dep |
| Other | 4 | (rounding to 34 total) |

We currently do **NOT** depend on `plotters-iced`, `iced_aw`, `iced-anim`,
or any other extra-widget library. Every cockpit widget is hand-rolled.

## Native iced 0.14 widgets we are not yet using

Critical finding from the [iced 0.14 release notes](https://github.com/iced-rs/iced/releases/tag/0.14.0)
and [CHANGELOG](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md):
**five new widgets shipped in iced 0.14.0 that we have not adopted.** These
are first-party, already in the lockfile, zero new deps:

| Native iced 0.14 widget | PR | Maps to (file:line) | Use case |
|---|---|---|---|
| `table` | [#3018](https://github.com/iced-rs/iced/pull/3018) | [`agent_feed.rs:1`](../../crates/ui/src/widgets/agent_feed.rs) (153 LOC), [`positions.rs:1`](../../crates/ui/src/widgets/positions.rs), [`strategies.rs:1`](../../crates/ui/src/widgets/strategies.rs) | Tabular rows with headers, currently `column!`+`row!` macros |
| `grid` | [#2885](https://github.com/iced-rs/iced/pull/2885) | [`kpi_strip.rs:1`](../../crates/ui/src/widgets/kpi_strip.rs) (264 LOC) | KPI strip layout, currently hand-rolled `row!` |
| `markdown` | (in 0.14 core, with `markdown` feature) | None yet | Renders reports in-cockpit; supports paragraphs, headings, code, quotes, rules, lists, tables, links, syntax highlighting via `highlighter` feature ([docs.rs](https://docs.rs/iced/latest/iced/widget/markdown/index.html)) |
| `float` | [#2916](https://github.com/iced-rs/iced/pull/2916) | [`chart_tooltip.rs:1`](../../crates/ui/src/widgets/chart_tooltip.rs) (562 LOC), [`journal_transaction_modal.rs:1`](../../crates/ui/src/widgets/journal_transaction_modal.rs) (571 LOC) | Positions an element floating over another — exactly the tooltip / modal overlay primitive we hand-built |
| `pin` | [#2673](https://github.com/iced-rs/iced/pull/2673) | None yet | Pins a widget to prevent it from being dropped during reactive renders |
| `sensor` | [#2751](https://github.com/iced-rs/iced/pull/2751) | None yet | Reactive value sensor; niche, mostly for devtools |
| `Animation` API | (0.14 core) | None yet | Application-level animation primitive — see Q4 |

This is the **single biggest finding of the survey**. Adopting native `table`
+ `grid` + `float` could plausibly retire 500–1500 LOC of hand-rolled
glue without adding a single transitive dep.

## Candidate matrix — third-party crates

Every crate the survey turned up, mapped to a cockpit pain point or
hand-rolled widget. Verdict legend: **ADOPT** (recommend in next 1–2
features); **EVALUATE** (architect should consider; non-obvious tradeoff);
**SKIP** (lag, license, or fit problem). License column: MIT/Apache/MPL =
fine; AGPL = no-go per
[`ui-test-harness-bootstrap` feature.md`](../ui-test-harness-bootstrap/feature.md)
precedent on `dssim-core`.

| Crate | Latest | License | iced 0.14? | Maps to (file:line OR roadmap) | Verdict | Rationale |
|---|---|---|---|---|---|---|
| [`iced_aw`](https://github.com/iced-rs/iced_aw) | 0.14.1 (2026-04-27) | MIT | ✅ yes | Multiple — date_picker → v2.5 Kronos backtest range / Phase 4 viewer; menu → screens routing; tab_bar → screens nav; sidebar → [`sidebar_nav.rs:1`](../../crates/ui/src/widgets/sidebar_nav.rs); badge → status chips; spinner → `panel_state::Loading`; number_input → [`override_risk_veto.rs:1`](../../crates/ui/src/widgets/override_risk_veto.rs) typed-confirm; card → frame replacement; context_menu → audit row right-click | **ADOPT (cherry-pick)** | Architect-resolved Q5 = cherry-pick. Brief B scope = `date_picker` + `spinner` + `badge` only, feature-gated. Block-adoption of `full` rejected per Q5 rationale (surface-area dump). 21,998 downloads/month, last commit 2026-04-27, official iced-rs org repo. |
| [`iced_anim`](https://crates.io/crates/iced_anim) | 0.3.1 (2026-01-01) | MIT | ✅ yes (0.3.x → iced 0.14.x) | None directly; could power motion tokens in [`spec/ui-design-principles.md`](../ui-design-principles.md) `DUR_1..DUR_4` ladder | **SKIP** | [`ui-design-principles.md:62`](../ui-design-principles.md) explicitly says "Not animation-rich. Trading UIs that move when nothing has happened are…" — the design constitution forbids the use case this crate solves. Iced 0.14 also ships a built-in `Animation` API that covers the bounded cases ([changelog](https://github.com/iced-rs/iced/blob/master/CHANGELOG.md)). Re-evaluate only if a Phase 6 Assistant slot (LLM streaming token reveal) lands. |
| [`iced_dialog`](https://crates.io/crates/iced_dialog) | 0.14.0 (2025-12-08) | MIT | ✅ yes | [`journal_transaction_modal.rs:1`](../../crates/ui/src/widgets/journal_transaction_modal.rs) (571 LOC) | **DEFER (Brief C, gated on H6)** | Architect-resolved Q6: API-shape diff IS architect work. Native `iced::widget::float` (Brief A) covers the overlay primitive — `iced_dialog` only buys us the *modal chrome* (header / button row / focus-trap). Defer until H6 falsification (native `float` doesn't compose with typed-confirm focus path). |
| [`iced_toasts`](https://crates.io/crates/iced_toasts) | 0.1.1 (2025-08-22) | MIT | ⚠️ iced 0.13 only | New surface (no current toast widget) | **SKIP** | Lags one iced version. No current toast surface in the cockpit, and the operator hasn't surfaced this as a pain point — adoption would be inventing a need. Re-evaluate if upstream bumps to 0.14, or wait until the cockpit needs ephemeral notifications (likely never — see `ui-design-principles.md` quiet aesthetic). |
| [`iced_drop`](https://crates.io/crates/iced_drop) | 0.2.26 (2026-05-11) | MIT | ✅ yes (`^0.14`) | None — no current drag-drop surface | **SKIP** | Live and maintained, but cockpit has no drag-drop use case. Could become relevant if Phase 6 Assistant adds card-rearrangement, but that's hypothetical. |
| [`iced_fonts`](https://crates.io/crates/iced_fonts) | 0.3.0 (2025-12-08) | MIT | ✅ yes (0.3.x → iced 0.14.x) | None directly; principles section [`Iconography`](../ui-design-principles.md) currently silent on icon-font source | **DEFER (wait for Phase 6)** | Architect-resolved Q9: pre-adoption manufactures a need. No current icon surface; cockpit ships zero system icons per `ui-design-principles.md` quiet aesthetic. Re-evaluate when Phase 6 Assistant brief opens — pulling 8 font families now would inflate snapshot baselines for zero present-tense win. |
| [`iced_drop`](https://crates.io/crates/iced_drop) (dup) | — | — | — | — | — | (duplicate row — ignore) |
| [`iced_plot`](https://crates.io/crates/iced_plot) | 0.4.0 (2026-03-20) | MIT | ✅ yes | [`chart.rs:1`](../../crates/ui/src/widgets/chart.rs) (1,539 LOC), [`equity_curve.rs:1`](../../crates/ui/src/widgets/equity_curve.rs) (386 LOC), [`drawdown_band.rs:1`](../../crates/ui/src/widgets/drawdown_band.rs) (353 LOC), [`sparkline.rs:1`](../../crates/ui/src/widgets/sparkline.rs) (180 LOC) | **SKIP** | Description: "GPU-accelerated plotting widget for Iced, handles up to millions of points." **Hard-requires wgpu** for GPU acceleration — exactly the dep weight (~10–15 MB) we are NOT paying today. Tiny-skia-only renderer is non-negotiable per chart-canvas-overhaul retrospective. Re-evaluate only if we ever flip to wgpu. |
| [`plotters-iced`](https://github.com/Joylei/plotters-iced) | 0.11.0 (2024-09-18) | MIT | ❌ iced 0.13 only | [`chart.rs:1`](../../crates/ui/src/widgets/chart.rs), `equity_curve.rs`, `drawdown_band.rs`, `sparkline.rs` | **SKIP** | Stuck at iced 0.13, last commit ~20 months ago. Original-author maintenance has stalled. |
| [`plotters-iced2`](https://github.com/GyulyVGC/plotters-iced2) | 0.14 (~2026) | MIT | ✅ yes (community fork) | Same as above | **SPIKE-only (Brief D, deferrable)** | Architect-resolved Q8: research spike on `sparkline.rs` only (180 LOC, smallest blast radius). Bus-factor-1 risk is the dominant SKIP signal; analyst's H5 fallback is "confirms SKIP." Adoption commitment forbidden — spike outcome routes back to operator. **NOT in Brief A/B/C scope.** |
| [`iced_color_picker`](https://github.com/iced-rs/awesome-iced) | 0.14 | MIT | ✅ yes | None — no color-picker surface | **SKIP** | No use case (operator chooses dark or light mode only — see `ui-design-principles.md`). |
| [`iced_audio`](https://github.com/iced-rs/awesome-iced) | 0.9 | MIT | ❌ iced 0.9 | None — VST/audio knobs | **SKIP** | Wrong domain (audio plugin UIs) and four versions behind. |
| [`iced_gif`](https://github.com/iced-rs/awesome-iced) | 0.10 | MIT | ❌ iced 0.10 | None | **SKIP** | Lag + no use case. |
| [`iced_moving_picture`](https://github.com/iced-rs/awesome-iced) | 0.14 | MIT | ✅ yes | None — GIF/APNG decoder | **SKIP** | No use case; operator prefers static screenshots in presenter decks. |
| [`iced_video_player`](https://github.com/iced-rs/awesome-iced) | 0.13 | MIT | ❌ iced 0.13 | None | **SKIP** | Wrong domain (GStreamer video) + lag. |
| [`iced_term`](https://lib.rs/crates/iced_term) | 0.13 | MIT | ❌ iced 0.13 | None | **SKIP** | Wrong domain (terminal emulator). |
| [`iced_code_editor`](https://crates.io/crates/iced-code-editor) | master | MIT | ⚠️ unreleased master | None today; could host JSON/TOML config editing if the cockpit ever ships a config screen | **SKIP** | Pre-release; no current fit. iced 0.14 also ships a `text_editor` widget natively. |
| [`iced-loading-indicator`](https://github.com/BB-301/iced-loading-indicator) | (single-commit GitHub, no crates.io) | MIT | ❌ iced 0.10 | `panel_state::Loading` (currently rendered as plain text in multiple panels) | **SKIP** | One-commit repo, four iced versions behind, not on crates.io. iced_aw's `spinner` feature is a better path. |
| [`iced_font_awesome`](https://crates.io/crates/iced_font_awesome) | 0.4.0 | MIT | ✅ yes (`^0.14`) | Subset of iced_fonts | **SKIP** | Strict subset of `iced_fonts` (which ships FA as one of 8 families). If we want fonts, prefer `iced_fonts`. |
| [`libcosmic`](https://github.com/pop-os/libcosmic) | active | MPL-2.0 (with notes) | maintains a separate fork of iced | New theme system / advanced widget set | **SKIP** | System76's COSMIC desktop toolkit, built on a **fork of iced** (not upstream 0.14). Pulling libcosmic means inheriting their iced-fork pin, which would break our `iced = "=0.14.0"` lockstep and the Lumen design system we already built. Not portable to our tiny-skia + macOS-only target. |
| [`bevy_iced`](https://github.com/iced-rs/awesome-iced) | 0.10 | MIT/Apache | ❌ iced 0.10 | None | **SKIP** | Wrong domain (game engine integration). |
| [`prettygooey`](https://github.com/iced-rs/awesome-iced) | 0.10 | MIT | ❌ iced 0.10 | Theme/UI styling | **SKIP** | Lag + we have Lumen (which post-dates this crate). |
| [`anim-rs`](https://github.com/iced-rs/awesome-iced) | 0.3 | MIT | ⚠️ framework-independent | None — see `iced_anim` row | **SKIP** | Same reason as `iced_anim` (design constitution forbids motion-heavy UI). |
| [`Cosmic Time`](https://github.com/iced-rs/awesome-iced) | 0.9 | MPL-2.0 | ❌ iced 0.9 | Animation | **SKIP** | Lag + animation use case forbidden. |

## Anti-candidates section

Crates I found that **look** attractive at first glance but I recommend
SKIP, with the precise reason:

1. **`plotters-iced` (original Joylei)** — looks like the obvious chart
   crate (#1 search result for "iced plotters"), but stuck at iced 0.13 with
   no commit since 2024-09. Adopting it means downgrading our entire iced
   stack — non-starter.
2. **`iced_plot`** — looks like a modern alternative ("millions of points,
   GPU-accelerated"), but **hard-requires wgpu** — exactly the renderer we
   stripped to ship tiny-skia byte determinism for snapshot tests
   ([`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md)).
   Adopting would invalidate the entire snapshot harness investment.
3. **`iced-anim` / `Cosmic Time` / `anim-rs`** — animation crates are
   attractive because animations look polished, but
   [`ui-design-principles.md:62`](../ui-design-principles.md) explicitly
   says "Not animation-rich. Trading UIs that move when nothing has happened
   are…". Adopting any of these would require an operator-locked
   constitutional amendment first.
4. **`libcosmic`** — looks like a "batteries-included widget toolkit on
   iced" (which would be a dream for our hand-rolled widget volume), but it
   ships on a **System76 fork of iced**, not upstream — pulling it would
   break our `iced = "=0.14.0"` lockstep, the Lumen tokens we already
   shipped, and the tiny-skia determinism contract.
5. **`iced_toasts`** — clean MIT crate, but iced 0.13 only, AND no current
   toast surface in the cockpit (Lumen aesthetic is quiet — notifications
   would be a net-new design decision, not a port).

## Open questions for architect

These are gaps where I need architect judgment before adoption hypotheses
land. The operator may need to ratify some of these.

- **Q1.** Should we adopt iced 0.14's **native `table` widget** to retire
  the hand-rolled `row!`+`column!` table glue in
  [`agent_feed.rs`](../../crates/ui/src/widgets/agent_feed.rs),
  [`positions.rs`](../../crates/ui/src/widgets/positions.rs), and
  [`strategies.rs`](../../crates/ui/src/widgets/strategies.rs)? Zero new
  deps. Risk: native `table` doesn't appear to support virtualization (per
  [docs.rs](https://docs.rs/iced/latest/iced/widget/table/index.html)
  showing only headers + rows + styling), so `agent_feed` at high event
  volume may still need the lazy-scrollable path. Acceptance gate:
  re-render perf at 1000+ rows.
- **Q2.** Same question for **native `grid`** vs hand-rolled `row!` in
  [`kpi_strip.rs`](../../crates/ui/src/widgets/kpi_strip.rs).
- **Q3.** Should we adopt **native `float`** to back tooltip and modal
  overlays, retiring the bespoke overlay code in
  [`chart_tooltip.rs`](../../crates/ui/src/widgets/chart_tooltip.rs) (562
  LOC) and
  [`journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (571 LOC)? This is the highest-ROI native-widget candidate by LOC
  potentially retired. The chart-canvas-overhaul retrospective specifically
  called out tooltip-overlay-clamp behaviour ([`spec/chart-canvas-overhaul/feature.md`](../chart-canvas-overhaul/feature.md));
  understand whether native `float` exposes equivalent clamp/anchor APIs.
- **Q4.** Adopt iced 0.14's **built-in `Animation` API** for the bounded
  motion tokens in [`ui-design-principles.md`](../ui-design-principles.md)
  (`DUR_1..DUR_4`)? This is zero-new-dep and aligned with the constitution
  ("Not animation-rich" forbids 60fps movement, NOT bounded transitions).
  If yes, the constitution may need a clarification sentence distinguishing
  motion-decoration (forbidden) from state-transition (allowed).
- **Q5.** Adopt **`iced_aw`** as a single block (feature `full`), or
  cherry-pick? Recommendation: cherry-pick (`date_picker` for v1.11 /
  Phase 4 backtest range, `spinner` for `panel_state::Loading`,
  `badge` for status chips). Block-adoption would dump 14 widget surfaces
  on the architect at once.
- **Q6.** Adopt **`iced_dialog`** in place of
  [`journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs)?
  Trade-off: single-purpose crate, well-scoped, but our modal carries
  cockpit-specific behaviour (typed-confirm `OVERRIDE`, focus-ring
  integration, journal-row click-through). API-shape match is the open
  question; architect should diff the two surfaces.
- **Q7.** Native iced 0.14 ships **markdown rendering**
  ([`iced::widget::markdown`](https://docs.rs/iced/latest/iced/widget/markdown/index.html)).
  Should the `viewer` binary embed in-cockpit markdown rendering of the 11
  committed backtest reports (currently rendered as text only)? Adoption
  cost: one feature flag (`markdown` + optional `highlighter`); zero new
  deps. Roadmap fit: matches the
  [`v2-llm-strategy`](../v2-llm-strategy/feature.md) follow-up brief and
  potential Phase 6 Assistant chat surface. **This is the most operator-
  visible candidate.**
- **Q8.** **`plotters-iced2`** as a chart-stack consolidation: would it let
  us retire the four hand-rolled chart-family widgets (chart.rs 1539,
  equity_curve.rs 386, drawdown_band.rs 353, sparkline.rs 180 = ~2.5k LOC)
  in favor of a Plotters-backed unified path? Risks: (a) single-maintainer
  fork, bus-factor 1; (b) Plotters' API model differs from our
  domain-specific (bars+markers+signals overlay) chart; (c) tiny-skia
  parity unconfirmed. **Architect should treat this as a research spike,
  not an adoption commitment.**
- **Q9.** Is `iced_fonts` adoption a precondition for any near-term
  feature? Today the cockpit ships zero system icons (per
  `ui-design-principles.md` quiet aesthetic). If Phase 6 Assistant ships a
  chat surface, we'll want at least 4-6 lucide icons. Pre-adopt or wait?
- **Q10.** Beyond the surveyed crates, is there an iced-adjacent **MCP
  server / accessibility / introspection tool** the architect would want?
  The HN discussion on iced 0.14 (cited in
  [`ui-testing-direction-2026-05-12.md ## 7`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#7-what-this-wont-fix))
  flagged accessibility as "WIP in iced." [AccessKit](https://accesskit.dev/)
  is the standard cross-toolkit a11y crate but iced 0.14 does not yet ship
  a first-class adapter. Out of scope for this survey, but worth raising.

## Hypothesis register (analyst seeds; architect ratifies / falsifies)

Per [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only),
the analyst seeds **measurable** adoption hypotheses; the architect picks
which to falsify first; the orchestrator runs the empirical test. None of
these are conclusions.

- **H1 — native `table` migration is byte-neutral.** *If* we replace the
  `row!`+`column!` macros in `agent_feed.rs` with iced 0.14's native
  `table` widget, *then* the `agent_feed_*` insta snapshots stay
  byte-identical (or change deterministically with a one-shot baseline
  refresh). Falsifier: snapshot byte-diff that survives baseline refresh
  indicates the native widget renders differently from our manual
  composition.
- **H2 — native `float` cleanly absorbs chart-tooltip overlay.** *If* we
  port [`chart_tooltip.rs`](../../crates/ui/src/widgets/chart_tooltip.rs)
  to iced 0.14's native `float`, *then* the `inner_rect_with_gutters`
  clamp logic from chart-canvas-overhaul v1.10.0 is expressible as `float`
  anchor + offset config without a custom `overlay::Overlay` impl.
  Falsifier: we need a custom overlay impl anyway, so the migration only
  saves the trivial half of the widget.
- **H3 — native `markdown` widget shipping in viewer is a one-day feature.**
  *If* we enable the `markdown` feature on the `iced` dep and add a
  `MarkdownReport` panel to the `viewer` bin, *then* the existing 11
  committed reports render correctly (paragraphs, tables, code blocks,
  links). Falsifier: report fixtures use rendering features outside
  `iced::widget::markdown::Item`'s supported variants (per
  [docs](https://docs.rs/iced/latest/iced/widget/markdown/index.html)
  explicit limitation).
- **H4 — `iced_aw::date_picker` is a drop-in for v1.11 / Phase 4 backtest
  range selection.** *If* we add `iced_aw = "0.14"` with the `date_picker`
  feature only, *then* the `viewer` bin's hard-coded date range becomes
  operator-selectable without breaking the snapshot harness. Falsifier:
  date_picker uses a font/glyph our snapshot baseline doesn't have, or its
  overlay collides with the Lumen tier model.
- **H5 — `plotters-iced2` is NOT a viable chart consolidation in 2026.**
  *If* an architect spends 1 dev-day spiking a port of
  [`sparkline.rs`](../../crates/ui/src/widgets/sparkline.rs) (smallest of
  the four chart widgets, 180 LOC) to plotters-iced2, *then* either (a)
  the spike succeeds and snapshot baselines refresh cleanly, suggesting a
  larger consolidation is viable, OR (b) the bus-factor-1 maintenance
  state surfaces breakage that confirms SKIP. Strong recommendation:
  architect picks the falsifier here BEFORE any chart-stack work.

## Design — architect synthesis

> **Architect pass, 2026-05-13.** Resolves Q1-Q10 against the analyst's
> candidate matrix; authors falsifiable hypotheses for every recommended
> adoption (per [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only));
> groups adoptions into four briefs (A/B/C/D) with explicit priority order;
> isolates a narrow operator-input set (3 Qs). **No code changes, no crate
> adds in this pass.** Each brief becomes its own future feature with its
> own analyst → architect → dev → tester loop.

### Q-resolutions (Q1-Q10)

Architect-decide are resolved inline with rationale. Operator-routed Qs
appear in [`## Operator-input questions`](#operator-input-questions) below.

- **Q1 (table) — RESOLVE: ADOPT native `table` for `positions.rs` (100 LOC)
  and `strategies.rs` (344 LOC); HOLD on `agent_feed.rs` (153 LOC) pending
  H7 perf falsifier.** Rationale: positions + strategies are bounded-row
  surfaces (≤20 strategies, ≤symbol-count positions) — virtualization is
  irrelevant. `agent_feed` is the volume tail (operator decisions emit one
  row per fill — 1000+ rows plausible at multi-strategy steady state); if
  native `table` lacks virtualization (analyst's open caveat) we keep the
  hand-rolled `Scrollable<Column>` path until iced ships `table` with
  lazy children. Threshold: ≥500 visible rows at steady state → keep
  hand-roll; <500 → migrate. See H7.

- **Q2 (grid) — RESOLVE: ADOPT native `grid` for `kpi_strip.rs:123-180`.**
  Rationale: KPI strip is a bounded 4–6 cell layout currently using
  `Row::new().push(...)` chains (file:line cited above). Native `grid`
  gives implicit column-alignment without our hand-rolled width calc.
  Zero virtualization concern. See H8.

- **Q3 (float) — RESOLVE: PARTIAL ADOPT. Use native `float` for
  `journal_transaction_modal.rs` (571 LOC); DEFER for `chart_tooltip.rs`
  (562 LOC) pending H2 falsifier.** Rationale: `journal_transaction_modal`
  is a standard widget-tree overlay (button-row + form fields) — `float`
  is the right primitive. `chart_tooltip.rs` is **canvas-draw based**
  (`draw_tooltip()` called from `ChartProgram::draw` at
  [`chart_tooltip.rs:65-110`](../../crates/ui/src/widgets/chart_tooltip.rs#L65)),
  not a widget-tree overlay — porting requires either (a) lifting tooltip
  out of canvas into the widget tree, or (b) accepting that `float`
  doesn't apply. H2 (analyst's original) covers this.

- **Q4 (Animation API) — ROUTE TO OPERATOR.** This is a constitutional
  reading: does [`ui-design-principles.md:62`](../ui-design-principles.md)
  "Not animation-rich" forbid bounded state transitions (e.g. focus-ring
  fade-in, panel-state cross-fade)? Architect's view: bounded transitions
  are NOT what the line forbids (it targets ambient motion-decoration).
  But the line is operator-authored constitution; operator ratifies. See
  [`## Operator-input questions`](#operator-input-questions) Q4.

- **Q5 (iced_aw block vs cherry-pick) — RESOLVE: CHERRY-PICK only.**
  Rationale: block-adopting 14+ widget surfaces dumps adoption decisions
  on the architect at once and inflates snapshot baselines for unused
  widgets. Brief B scope locked to `date_picker` + `spinner` + `badge`
  feature flags. Each subsequent iced_aw widget becomes its own
  adoption-feature decision. See H4 (date_picker), H9 (spinner), H10
  (badge).

- **Q6 (iced_dialog) — RESOLVE: DEFER, gated on H6 outcome.** Rationale:
  native `float` (Brief A) covers the overlay primitive; `iced_dialog`
  only adds the modal *chrome* (header/button-row/focus-trap). If H6
  falsifies "native `float` composes with our typed-confirm focus path,"
  THEN `iced_dialog` re-enters consideration as Brief C. Otherwise SKIP.

- **Q7 (markdown viewer in `viewer` bin) — ROUTE TO OPERATOR.** Product
  direction: IS in-cockpit rendering of the 11 committed backtest reports
  on the roadmap? Analyst calls this "the most operator-visible
  candidate." Architect cannot adjudicate product priority. See
  [`## Operator-input questions`](#operator-input-questions) Q7.

- **Q8 (plotters-iced2 chart-stack consolidation) — RESOLVE: SPIKE-only
  on `sparkline.rs` (180 LOC, smallest blast radius).** Treat as Brief D,
  research-only, deferrable indefinitely. Bus-factor-1 risk is
  load-bearing; the spike's job is to *confirm or deny* H5, not to land
  a port. Adoption commitment forbidden in this brief — spike outcome
  routes back to operator for go/no-go on a broader consolidation.

- **Q9 (iced_fonts pre-adoption) — RESOLVE: WAIT.** Rationale: cockpit
  ships zero system icons per `ui-design-principles.md` quiet aesthetic.
  Pre-adoption manufactures a need + inflates the asset baseline for the
  ui-test-harness PNG triples. Re-evaluate when Phase 6 Assistant brief
  opens (operator-flagged precondition).

- **Q10 (AccessKit / a11y) — RESOLVE: TRACK, not adopt.** Rationale:
  iced 0.14 does not yet ship a first-class AccessKit adapter (analyst-
  confirmed). Adopting upstream-incomplete a11y plumbing would either
  fork iced or live as a feature flag with no behaviour. Track via a
  dedicated `awaiting-upstream` queue entry; revisit when iced 0.15+
  ships an adapter or AccessKit publishes its own iced backend.

### Adoption priority — four briefs, recommended ordering

The five ADOPT/EVALUATE candidates from the analyst's matrix collapse to
four briefs after Q-resolution. Recommended ordering reflects (a) risk
profile, (b) operator visibility, and (c) blast radius. **Operator's
opening preference welcomed** — see Q-O3 below.

| Brief | Scope | Estimated retired LOC | New deps | Snapshot diff | Risk |
|---|---|---|---|---|---|
| **A — Native iced 0.14 widgets** | `table` (positions, strategies), `grid` (kpi_strip), `float` (journal_transaction_modal) | ~900-1100 LOC | **0** (already in lockfile) | ~25 panel snapshots refreshed | LOW — first-party, zero new transitive deps |
| **B — `iced_aw` cherry-pick** | `date_picker`, `spinner`, `badge` (feature-gated) | ~50-100 LOC + new surfaces | **1 direct dep** (`iced_aw = "0.14"` with 3 features); transitive estimate: +2-3 crates (per feature gate manifest) | ~5-10 panel snapshots refreshed | MEDIUM — third-party but well-maintained (last commit 2026-04-27, official `iced-rs` org) |
| **C — `iced_dialog` chrome wrapper** | Only if H6 falsifies; replaces modal chrome around `journal_transaction_modal.rs` | ~150-200 LOC | **1 direct dep** (transitive: +1-2 crates) | ~3 modal snapshots refreshed | MEDIUM — single-feature crate, depends on H6 |
| **D — `plotters-iced2` SPIKE** | `sparkline.rs` only (180 LOC); research-only | ~180 LOC IF spike succeeds, ELSE 0 | **1 direct dep** (transitive estimate: +8-12 crates incl. plotters + plotters-backend chain) | 1 sparkline snapshot refreshed | HIGH — bus-factor-1 maintainer; analyst's H5 default expectation is SKIP |

**Recommended order:** **A → B → C (gated) → D (gated).**

- **A first** — zero new deps, largest LOC retirement, validates the
  hypothesis pattern with the safest candidate set.
- **B next** — controlled cherry-pick, three discrete falsifiers (H4 /
  H9 / H10), each independently shippable.
- **C deferred** — only opens IF H6 (native `float` chrome composes with
  typed-confirm focus path) falsifies. Default expectation: don't open.
- **D research-only** — spike spawned only after A ships; H5's analyst-
  authored default is SKIP; the spike's job is to either confirm SKIP
  or surface a falsification that warrants operator re-engagement.

**Markdown viewer (Q7) is operator-gated** — if approved, it becomes its
own brief slotted between A and B (independent of all other briefs;
viewer-bin only, no cockpit risk).

### Per-candidate cost analysis

#### Brief A — Native iced 0.14 widget adoption

| Sub | Widget | Retire (file:line) | Wire-in target | Snapshot impact | iced 0.14 compat risk |
|---|---|---|---|---|---|
| A1 | `table` → `positions` | [`positions.rs:1-100`](../../crates/ui/src/widgets/positions.rs) (100 LOC `Row::new()`/`Column::new()` glue) | `positions.rs::view()` returns `iced::widget::table::Table` | ~6 panel snapshots (positions_*) | LOW (H1) |
| A2 | `table` → `strategies` | [`strategies.rs:1-344`](../../crates/ui/src/widgets/strategies.rs) (344 LOC — includes sparkline-cell wiring; only the table glue retires, sparkline cell stays) | `strategies.rs::view()` returns `Table` | ~8 panel snapshots (strategies_*) | LOW (H1) |
| A3 | `table` → `agent_feed` (HELD) | [`agent_feed.rs:21,53-82`](../../crates/ui/src/widgets/agent_feed.rs) (153 LOC) | DEFERRED pending H7 perf falsifier | ~9 panel snapshots (agent_feed_*) IF migrated | MEDIUM — virtualization caveat |
| A4 | `grid` → `kpi_strip` | [`kpi_strip.rs:17,123-180`](../../crates/ui/src/widgets/kpi_strip.rs) (~80 LOC of layout glue inside the 264-LOC file; row dispatch + sentiment-color logic stays) | `kpi_strip.rs::view()` returns `Grid` | ~2 panel snapshots (kpi_*) | LOW (H8) |
| A5 | `float` → `journal_transaction_modal` | [`journal_transaction_modal.rs:1-571`](../../crates/ui/src/widgets/journal_transaction_modal.rs) (overlay-positioning portion only — ~150 LOC; typed-confirm logic + focus-ring integration stays) | `journal_transaction_modal.rs::view()` wraps `float` | ~3 modal snapshots | MEDIUM (H6) |

**Transitive crate delta: 0.** All five sub-adoptions reach widgets
already in the iced lockfile.

**License: MIT/Apache** (iced upstream).

**PNG baseline impact (`ui-test-harness-bootstrap` v0.1):** The 3 PNG
baselines at [`crates/ui/tests/visual-baselines/charts_screen_dark_*.png`](../../crates/ui/tests/visual-baselines/)
render the Charts screen, NOT positions/strategies/kpi_strip/journal_modal —
**Brief A leaves all 3 PNGs byte-identical.**

#### Brief B — `iced_aw` cherry-pick

| Sub | Widget | New surface | Retire | Snapshot impact | iced 0.14 compat |
|---|---|---|---|---|---|
| B1 | `date_picker` | viewer bin's hard-coded date range becomes operator-selectable (v1.11 / Phase 4 follow-up) | — (new surface, retires nothing) | +1 panel snapshot (viewer-bin) | LOW (H4) |
| B2 | `spinner` | `panel_state::Loading` rendered as visual spinner | retires plain-text "Loading…" rendering in ~8 panels | ~8 panel snapshots (`*_loading.snap`) refreshed | LOW (H9) |
| B3 | `badge` | status chips on Strategies / Risk screens | replaces hand-rolled `container().style(badge_style)` patterns; estimate ~50 LOC across 3 files | ~5 panel snapshots | LOW (H10) |

**Transitive crate delta: +2-3** (one direct dep `iced_aw`, plus its
feature-gated dependencies for `date_picker` / `spinner` / `badge`).
Architect to confirm via `cargo tree --features iced_aw/date_picker
iced_aw/spinner iced_aw/badge` during dev pass.

**License: MIT.**

**PNG baseline impact: 0** (B1-B3 don't touch Charts screen).

#### Brief C — `iced_dialog` (gated on H6 falsification)

| Sub | Widget | New surface | Retire | Snapshot impact | iced 0.14 compat |
|---|---|---|---|---|---|
| C1 | `iced_dialog::Dialog` | wraps `journal_transaction_modal.rs` chrome | retires header/button-row/focus-trap chrome ~150-200 LOC | ~3 modal snapshots | MEDIUM (H11) |

**Transitive crate delta: +1-2.** **License: MIT.** **PNG baseline impact: 0.**

#### Brief D — `plotters-iced2` SPIKE (research-only)

| Sub | Widget | Scope | Retire IF spike succeeds | Snapshot impact | iced 0.14 compat |
|---|---|---|---|---|---|
| D1 | `plotters-iced2` integration spike | port `sparkline.rs` (180 LOC) to plotters-iced2 backend; render byte-comparable output | 180 LOC IF spike succeeds; 0 IF SKIP confirmed | 1 sparkline snapshot (intentional refresh during spike) | HIGH — community fork, bus-factor 1 |

**Transitive crate delta IF adopted: +8-12** (plotters + plotters-backend
+ plotters-svg + plotters-iced2 + their transitives). License: MIT. PNG
baseline impact: 0 (sparkline lives on Strategies-detail, not Charts).

**Spike-only commitment:** Brief D does NOT open an adoption — it opens
a 1 dev-day port spike against H5. Outcome routes to operator.

### Hypothesis register (architect, 2026-05-13)

Per [`AGENT.md ## Architect = hypothesis only`](../../AGENT.md#architect--hypothesis-only),
each hypothesis carries an orchestrator-runnable falsifier requiring
NO display server / GPU / live window — only `cargo test`, `cargo doc`,
`cargo build`, `grep`, or `cargo tree`.

**H-prefix `arch-`** distinguishes architect-authored from analyst-seeded
(`H1`-`H5` analyst seeds are re-stated here under their architect-pass
falsifier where re-scoped).

**H-arch-0 (architect, 2026-05-13) — iced 0.14 native widgets `table`,
`grid`, `float`, `markdown`, `pin` are reachable from our existing
`iced = "=0.14.0"` feature set `["tiny-skia", "thread-pool", "advanced",
"canvas"]` without enabling additional iced feature flags.**
- *Statement:* Analyst's "five new widgets shipped in iced 0.14" claim
  is load-bearing for Briefs A and (operator-gated) Q7. If any of these
  widgets requires a feature flag we don't carry, the cost analysis
  shifts (and may surface transitive deps).
- *Falsifier:* Orchestrator runs `cargo doc -p iced --no-deps --features
  "tiny-skia,thread-pool,advanced,canvas"` and greps the generated
  doc-index for `pub mod table`, `pub mod grid`, `pub mod float`, `pub mod
  markdown`, `pub mod pin`. If any module is absent, that widget either
  needs an iced feature we don't carry OR the analyst's claim is wrong
  → falsified, STOP and re-scope toward a Cargo.toml feature audit.
- *Status:* **RESOLVED-FALSIFIED-partial — `markdown` requires enabling
  the `markdown` feature (NOT in current Cargo.toml); `table`, `grid`,
  `float`, `pin` are reachable under current `["advanced", "canvas"]`
  feature set.**
- *Evidence (M0 falsifier sub-agent, 2026-05-13, sandbox-constrained
  read-only inspection):*
  - **Method.** `cargo doc` was blocked by the sandbox; substituted the
    architect's `Alternative falsifier` path (inspect built artifacts).
    The build-artifact-derived equivalent is rustc's per-crate `.d`
    dependency manifest, which lists exactly which `.rs` files were
    compiled into the present `iced_widget-0.14.2` artifact under our
    `iced = "=0.14.0"` feature flags. A `.rs` file present in the `.d`
    manifest ≡ its module is compiled into the current build (i.e. NOT
    cfg-gated away by an OFF feature). A `.rs` file absent ≡ either
    feature-gated and OFF, or not part of the crate. Fingerprint JSONs
    cross-check declared-vs-enabled features.
  - **Fingerprint** (`target/debug/.fingerprint/iced_widget-c9c66a946f64d83f/lib-iced_widget.json:1`):
    `"features":"[\"advanced\", \"canvas\"]"`,
    `"declared_features":"[\"advanced\", \"canvas\", \"crisp\", \"highlighter\", \"image\", \"lazy\", \"markdown\", \"ouroboros\", \"qr_code\", \"svg\", \"wgpu\"]"`.
    Note `markdown` and `lazy` are declared but OFF.
  - **Compiled sources** (verbatim from
    `target/debug/deps/iced_widget-e0d51c2a6d696d3b.d:1`, source paths
    under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/`):
    `table.rs` present, `grid.rs` present, `float.rs` present, `pin.rs`
    present, `sensor.rs` present; `markdown.rs` ABSENT; `lazy.rs` ABSENT.
  - **Per-widget verdict:**
    | Widget | Source file in .d manifest | Verdict |
    |---|---|---|
    | `table` | `table.rs` present | REACHABLE |
    | `grid` | `grid.rs` present | REACHABLE |
    | `float` | `float.rs` present | REACHABLE |
    | `pin` | `pin.rs` present | REACHABLE |
    | `markdown` | `markdown.rs` ABSENT (feature `markdown` OFF) | NOT-REACHABLE under current features — requires `iced = {... features=[..., "markdown"]}` |
  - **Impact:** Brief A (table+grid+float adoption) is CLEAR for adoption
    under current feature set. Q-O2 / M5 markdown viewer requires the
    Cargo.toml change `features=[..., "markdown"]` as a precondition
    (1-line edit, zero new transitive crates per `markdown` feature
    being internal to `iced` / `iced_widget`).
  - **Caveat.** The `.d`-manifest method confirms the source file
    compiled; it does not directly grep the `pub mod` re-export in
    `iced/src/lib.rs` (registry-read blocked under sub-agent sandbox).
    Orchestrator can confirm the final `pub mod` plumbing by running
    the original `cargo doc -p iced` falsifier if needed; per build-
    artifact ground truth this is expected to PASS for table/grid/float/pin.

**H-arch-1 (architect, 2026-05-13; re-scoped from analyst H1) — Native
`table` migration of `positions.rs` produces snapshot diffs that are
byte-identical OR deterministically refreshable via `cargo insta
review`.**
- *Statement:* `positions.rs` is a bounded-row tabular surface; replacing
  `Row`/`Column` glue with `iced::widget::table` should produce
  equivalent visual output modulo expected one-shot baseline refresh.
- *Falsifier:* Dev sub-agent ports `positions.rs::view()` to native
  `table`, runs `cargo test -p ui --test panel_snapshots positions`, and
  records the diff. If after baseline refresh the snapshot is unstable
  across two consecutive runs → falsified, native `table` has internal
  non-determinism we cannot accept.
- *Status:* unresolved.

**H-arch-2 (architect, 2026-05-13; re-scoped from analyst H2) — Native
`float` does NOT cleanly absorb `chart_tooltip.rs`, because the tooltip
is canvas-draw based (`draw_tooltip()` in
[`chart_tooltip.rs:65-110`](../../crates/ui/src/widgets/chart_tooltip.rs#L65)),
not widget-tree based.**
- *Statement:* `chart_tooltip` renders inside the canvas's `Program::draw`
  call, not as a sibling widget — `float` operates on widget-tree
  elements. The port requires lifting tooltip out of canvas first.
- *Falsifier:* `grep -nE "draw_tooltip|ChartProgram::draw" crates/ui/src/widgets/chart_tooltip.rs crates/ui/src/widgets/chart.rs`
  — if the call-graph shows canvas-internal dispatch (expected), H2
  CONFIRMED (we keep tooltip canvas-rendered, OR open a separate brief
  to lift it). If the call-graph shows widget-tree dispatch, H2
  FALSIFIED and native `float` is a candidate after all.
- *Status:* **RESOLVED-UNFALSIFIED — chart_tooltip is canvas-internal
  dispatch; native `float` does NOT apply without first lifting
  tooltip out of the canvas.**
- *Evidence (M0 falsifier sub-agent, 2026-05-13, verbatim grep output):*
  - `crates/ui/src/widgets/chart_tooltip.rs:68` —
    `pub(crate) fn draw_tooltip(` (free function, NOT a `Widget` impl).
  - `crates/ui/src/widgets/chart_tooltip.rs` — supplementary grep
    `grep -nE "^impl|^pub" chart_tooltip.rs` returned ONLY the
    `draw_tooltip` line above; there is **no** `impl Widget for ...`
    or `impl Overlay for ...` anywhere in the file.
  - `crates/ui/src/widgets/chart.rs:226` —
    `impl canvas::Program<Message> for ChartProgram {` (canvas-program,
    not a widget-tree node).
  - `crates/ui/src/widgets/chart.rs:306` — `fn draw(` (start of
    `ChartProgram::draw`).
  - `crates/ui/src/widgets/chart.rs:468` —
    `chart_tooltip::draw_tooltip(&mut frame, bounds, anchor, &view, self.mode);`
    (caller is `ChartProgram::draw`, dispatch is canvas-internal).
  - Verdict: call-graph is canvas-internal as architect predicted.
    Brief A scope excludes `chart_tooltip` for native `float` adoption.

**H-arch-3 (architect, 2026-05-13; absorbs analyst H3) — Operator-gated
Q7 markdown viewer: the iced 0.14 `markdown` widget can render the 11
committed backtest reports without missing variants.**
- *Statement:* Pre-condition for Q7 = ADOPT. If markdown renders cleanly
  in a viewer-bin smoke test, the operator decision is well-supported.
- *Falsifier:* Dev sub-agent enables iced's `markdown` feature, writes
  a minimal viewer-bin smoke test loading
  [`spec/reports/success-fixed-report-sample-7d.md`](../reports/),
  asserts the test compiles and runs to completion. If `markdown::Item`
  variants don't cover all features (tables, code-blocks, links per
  analyst H3 fallback), falsified → operator decides whether to ship
  partial coverage or defer.
- *Status:* unresolved. **Only spawn IF operator answers Q-O2 = ADOPT.**

**H-arch-4 (architect, 2026-05-13; re-scoped from analyst H4) —
`iced_aw::date_picker` 0.14.1 is feature-flag-isolatable in Cargo.toml.**
- *Statement:* Pre-condition for Brief B is that `iced_aw`'s feature
  graph permits `date_picker` without dragging in `menu`/`tab_bar`/
  `sidebar`/etc.
- *Falsifier:* Orchestrator (or sandbox) runs `cargo tree -p iced_aw
  --features date_picker --no-default-features` (read-only inspection
  via a scratch Cargo.toml — NOT a workspace edit) and counts new
  transitive crates. If the count exceeds 5 new crates OR pulls a
  forbidden license, falsified → re-scope to "vendor the date-picker
  source" or HOLD.
- *Status:* unresolved.

**H-arch-5 (architect, 2026-05-13; ratifies analyst H5 default) —
`plotters-iced2` sparkline-only port produces a one-shot snapshot
refresh that's byte-stable across two consecutive `cargo test` runs.**
- *Statement:* If unstable across runs, plotters' internal rendering
  has non-determinism we cannot accept (rules out the entire chart-stack
  consolidation argument).
- *Falsifier:* Dev spike runs `cargo test -p ui sparkline` twice in
  succession; diffs snapshot bytes. If `cmp -s` fails between the two
  runs → falsified, SKIP locked.
- *Status:* unresolved. **Spike-only — DO NOT spawn before Brief A
  ships and operator approves opening Brief D.**

**H-arch-6 (architect, 2026-05-13) — Native `float` overlay composes
with our typed-confirm `OVERRIDE` focus path in
`override_risk_veto.rs` / `journal_transaction_modal.rs` without
requiring a custom `overlay::Overlay` impl.**
- *Statement:* If native `float` exposes focus-trap + dismissal-event
  hooks compatible with our typed-confirm pattern, Brief C (iced_dialog)
  stays deferred. If NOT, Brief C re-opens.
- *Falsifier:* `cargo doc -p iced --no-deps` + grep the `float` module's
  public surface for focus-handling APIs (`on_dismiss`, `focus`, or
  equivalent). If absent → H6 falsified, Brief C unblocks.
- *Status:* unresolved.

**H-arch-7 (architect, 2026-05-13) — Native `table` in iced 0.14 lacks
row-virtualization (no lazy-children API).**
- *Statement:* Gates A3 (`agent_feed.rs` migration). If `table` is eager
  (renders all rows up-front), at 1000+ steady-state rows the cockpit's
  re-render cost grows linearly with history — unacceptable for
  `agent_feed`.
- *Falsifier:* `cargo doc -p iced --no-deps` + grep the `table` module
  for `lazy`/`virtual`/`with_offset`/`row_provider` patterns. If any
  virtualization API surfaces → falsified, A3 unblocks. If absent
  (expected) → H7 confirmed, A3 stays held, `agent_feed` keeps
  `Scrollable<Column>` glue.
- *Status:* **RESOLVED-UNFALSIFIED-partial — indirect build-artifact
  evidence is consistent with native `table` lacking virtualization;
  direct table.rs source grep remains an orchestrator-only step
  because the sub-agent sandbox blocks access to
  `~/.cargo/registry/`. A3 stays HELD for `agent_feed.rs` pending
  orchestrator-run direct grep.**
- *Evidence (M0 falsifier sub-agent, 2026-05-13):*
  - **Method.** `cargo doc -p iced` was blocked by the sandbox; the
    brief's `Alternative falsifier` of grepping
    `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_widget-0.14.2/src/table.rs`
    was also blocked by sandbox. Substituted: inspect the iced_widget
    feature graph and compiled-source manifest for whether a separate
    `lazy` widget surface exists (a sibling-virtualization pattern
    would imply table itself is eager-only).
  - **Fingerprint** (`target/debug/.fingerprint/iced_widget-c9c66a946f64d83f/lib-iced_widget.json:1`):
    `"declared_features":"[\"advanced\", \"canvas\", \"crisp\", \"highlighter\", \"image\", \"lazy\", \"markdown\", \"ouroboros\", \"qr_code\", \"svg\", \"wgpu\"]"`
    — `lazy` is a separate, feature-gated module distinct from `table`.
  - **Compiled sources** (`target/debug/deps/iced_widget-e0d51c2a6d696d3b.d:1`):
    `table.rs` present (compiled), `lazy.rs` ABSENT (feature `lazy` OFF).
    iced_widget partitions table-the-widget and lazy-children into
    separate modules; turning on `lazy` enables a separate `lazy` widget
    (per
    [docs.rs/iced/0.14/iced/widget/lazy](https://docs.rs/iced/0.14/iced/widget/lazy/)
    naming convention), not virtualization inside `table`.
  - **Interpretation.** The architect's expected outcome (table is
    eager-only; virtualization, if any, lives in the separate `lazy`
    feature/module) is consistent with the iced_widget feature-graph
    partition. **NOT a direct grep of `table.rs`'s public API for
    `with_offset` / `virtual` / `row_provider`** — that remains an
    orchestrator action because sub-agent sandbox blocks
    `~/.cargo/registry/` reads even for `grep`.
  - **Action.** A3 (`agent_feed.rs` migration) **stays HELD** per
    architect decision; if/when the orchestrator runs the direct
    `table.rs` grep and confirms no virtualization API, this status
    converts to a full RESOLVED-UNFALSIFIED. Brief A's A1/A2/A4/A5
    proceed independently — none of them depend on H-arch-7.

**H-arch-8 (architect, 2026-05-13) — Native `grid` accepts the
KPI-strip's 4-cell layout with implicit column alignment, retiring
the hand-rolled width-calc in `kpi_strip.rs:146-180`.**
- *Statement:* Brief A4 hinges on `grid` exposing a column-template
  API (column count + per-cell content) that matches our 4-KPI
  layout shape.
- *Falsifier:* `cargo doc -p iced --no-deps` + grep `grid` module for
  `column_count` / `with_columns` / `cell` patterns. If the API requires
  manual cell-position math, the migration buys less than expected →
  re-scope A4 to "skip; hand-roll is already minimal."
- *Status:* unresolved.

**H-arch-9 (architect, 2026-05-13) — `iced_aw::spinner` renders
deterministically (no internal animation frame counter or wall-clock
read) for snapshot determinism.**
- *Statement:* Brief B2 must not introduce non-determinism. If
  `iced_aw::spinner` reads wall-clock to compute its rotation angle,
  every snapshot of `*_loading.snap` becomes flaky.
- *Falsifier:* Grep `iced_aw` source (vendored via `cargo doc` or
  crates.io view) for `Instant::now`/`SystemTime::now`/`elapsed` in
  the spinner module. If present → falsified, B2 SKIP, fall back to
  static "Loading…" text. Also covered by
  `scripts/check_no_clocks_in_ui_tests.sh` from
  [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md).
- *Status:* unresolved.

**H-arch-10 (architect, 2026-05-13) — `iced_aw::badge` renders with our
Lumen surface tokens via its custom-style hook.**
- *Statement:* Brief B3 requires `badge` accept our `theme::ModeColor`
  ramp; if it hard-codes its own palette, badges look out of place and
  Lumen brand-bleed grep fails.
- *Falsifier:* `cargo doc -p iced_aw --features badge --no-deps` + grep
  the badge module for a `style`/`Catalog`/`StyleFn` public surface
  taking `Color` or `Theme`. If absent → falsified, B3 falls back to
  hand-rolled `container().style(badge_style)`.
- *Status:* unresolved.

**H-arch-11 (architect, 2026-05-13; conditional) — `iced_dialog` exposes
a typed-confirm focus-trap API hook compatible with our
`OVERRIDE`-string requirement.**
- *Statement:* Only meaningful IF H6 falsifies (native `float` lacks
  focus-trap). If `iced_dialog` lacks a hook for "block dismissal until
  text input matches a literal," Brief C falls back to keeping
  `journal_transaction_modal`'s hand-rolled focus path.
- *Falsifier:* `cargo doc -p iced_dialog --no-deps` + grep for
  `on_dismiss` / `focus` / `confirm_text` / equivalent. If absent →
  falsified, Brief C closes.
- *Status:* PENDING H6 (do not falsify until H6 resolves).

### Snapshot-impact summary

Sum of `.snap` files expected to diff across briefs (panel_snapshots
prefix; total baseline count = 68 per
[`crates/ui/tests/snapshots/`](../../crates/ui/tests/snapshots/)):

| Brief | `.snap` files refreshed (estimate) | PNG baselines refreshed |
|---|---|---|
| A | ~20 (positions ×6 + strategies ×8 + kpi ×2 + journal-modal ×3 + audit-modal ×1) | 0 |
| B | ~13 (loading states ×8 + badges ×5) | 0 |
| C | ~3 (journal-modal chrome) | 0 |
| D | ~1 (sparkline only) | 0 |

**No PNG visual baseline is expected to diff in any brief** — Brief A's
changes are confined to panels rendered outside the Charts screen, B
changes loading/badge surfaces also outside Charts, C is modal-only, D
is sparkline-only. Re-blessing happens via `cargo insta review` (per
ui-test-harness-bootstrap precedent).

### Operator-input questions

Three Qs that genuinely require operator decision before architect spawns
the next adoption brief. **Architect's recommendation in parens; operator
ratifies, overrides, or defers.**

- **Q-O1 (constitutional clarification of `ui-design-principles.md:62`):**
  Does "Not animation-rich" forbid bounded state transitions (focus-ring
  fade-in, panel-state cross-fade, ~150ms `DUR_2` motion-token uses), or
  only ambient motion-decoration? Architect's read: bounded transitions
  are allowed; the line targets liar-by-motion-when-nothing-happened
  (steady-state idle animation). Operator: ratify (allows iced 0.14
  Animation API as zero-new-dep adoption) OR over-rule (locks the
  cockpit to fully static rendering — also fine). **Recommended:
  ratify the bounded-transitions reading + amend the constitution
  sentence to make it explicit.**

- **Q-O2 (in-cockpit markdown viewer roadmap fit):** Should the `viewer`
  bin render the 11 committed backtest reports as live in-cockpit
  markdown (paragraphs, headings, code blocks, links, tables, syntax
  highlighting) using iced 0.14 native `markdown`? Cost: 1 feature flag,
  zero new deps; one new viewer-bin panel. Architect's read: the analyst
  called this the most operator-visible candidate; v2 LLM ship unblocks
  related surfaces (Phase 6 Assistant chat). Operator: **ADOPT** (slot
  a brief between A and B), **DEFER** (queue with no ship-date), or
  **SKIP** (out of product scope). **Recommended: ADOPT — single feature
  flag, high operator-visible payoff, blocks no other work.**

- **Q-O3 (adoption brief ordering preference):** Architect recommends
  **A → B → C (gated) → D (gated)**. Does the operator have a competing
  preference (e.g. open Brief B first to validate third-party adoption
  workflow before Brief A's larger LOC swap, or open Q-O2's
  markdown-viewer first as the most visible win)? **Recommended:
  proceed in architect's order unless operator has a specific reason
  to re-prioritize.**

### Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| H-arch-0 falsifies (native widgets need additional iced features) | LOW | Brief A reshape — possibly +1-2 iced features in Cargo.toml | Falsify H-arch-0 FIRST, before any other dev work |
| H-arch-1 snapshot drift uncovers iced rendering non-determinism | LOW (Lumen Phase 1-5 already shipped 28 widgets without it) | Brief A halts, escalates to upstream iced issue | One-shot baseline refresh + two-consecutive-run determinism gate |
| H-arch-7 confirms `table` lacks virtualization | HIGH (analyst flagged this) | A3 stays held; `agent_feed` keeps hand-roll indefinitely | Accept; document threshold (≥500 visible rows → hand-roll); revisit when iced ships `table` lazy children |
| Brief B `iced_aw` transitive bloat exceeds budget | LOW | Cherry-pick falls back to vendoring specific widgets | H-arch-4 / H-arch-9 / H-arch-10 each falsifiable independently |
| Brief D plotters-iced2 spike confirms SKIP (analyst's default expectation) | HIGH (analyst-flagged bus-factor-1) | Chart-stack consolidation closes; ~2.5k LOC stays hand-rolled | Accept; the spike's value is the falsification itself, not adoption |
| Operator over-rules constitutional reading (Q-O1) | MEDIUM | iced 0.14 Animation API stays out of scope | Architect's recommendation is non-binding; operator owns the constitution |

## Out of scope

Explicit non-goals for this survey:

- **Non-iced Rust GUI alternatives** (egui, slint, gpui, dioxus, makepad,
  floem, ribir). The
  [`ui-testing-direction-2026-05-12.md`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md)
  dev-note covered that territory. This brief is iced-ecosystem-only.
- **The wgpu vs tiny-skia decision.** Locked to tiny-skia per chart-canvas-
  overhaul retrospective and [`ui-test-harness-bootstrap` v0.1](../ui-test-harness-bootstrap/feature.md);
  any candidate that hard-requires wgpu is SKIP by construction (see
  `iced_plot` row).
- **Cross-platform (Windows/Linux).** The cockpit is macOS-only per
  operator decision D3 in
  [`ui-testing-direction-2026-05-12.md ## 9`](../dev-notes/archive/2026-Q2/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator).
  Candidate evaluation does not score cross-platform reach.
- **The 34-crate transitive baseline.** Orchestrator-verified at commit
  `d8c3a99`; analyst does NOT re-survey.
- **Adoption commitment.** This brief recommends architect spawn; no crate
  enters `crates/ui/Cargo.toml` from this brief. Each adopt-candidate
  becomes its own narrow feature (e.g. `iced-native-table-adoption`,
  `iced-markdown-viewer-integration`) with architect-led adoption
  hypotheses + tester-locked snapshot gates.
- **Performance benchmarks.** No criterion runs from this brief. Architect
  may spawn a `rust-bench` spike per H1/H5 if needed.

## Implementation

_See [`## Design — architect synthesis`](#design--architect-synthesis)
above. This research-brief itself ships no implementation. Each
adoption brief (A/B/C/D) opens its own feature folder
(`spec/iced-native-widget-adoption-brief-a/`, etc.) with its own
analyst → architect → developer loop after operator answers Q-O1 / Q-O2 /
Q-O3 in [`## Operator-input questions`](#operator-input-questions)._

_Task stubs at [`tasks.md`](tasks.md) carry only the M0 architect-
diagnostic work (falsify H-arch-0 first, then H-arch-7) plus stubs for
the four downstream brief spawns._

## Verification
_tester links to reports here once any adoption feature ships. This
research-brief itself has no test — its verification is operator approval
of the candidate matrix + architect uptake of H1-H5._

## Changelog

- 2026-05-13 (M0 falsifier sub-agent — orchestrator-routed, read-only
  sandbox): Ran T-M0-1 (H-arch-0), T-M0-2 (H-arch-7), T-M0-3 (H-arch-2)
  in a single pass. **H-arch-0 → RESOLVED-FALSIFIED-partial:** `table`,
  `grid`, `float`, `pin` reachable under current `["advanced",
  "canvas"]` features (build-artifact `.d` manifest at
  `target/debug/deps/iced_widget-e0d51c2a6d696d3b.d` lists all four
  `.rs` files as compiled); `markdown` requires enabling the
  feature-gated `markdown` flag in iced/iced_widget Cargo.toml (NOT
  currently enabled — `target/debug/.fingerprint/iced_widget-c9c66a946f64d83f/lib-iced_widget.json`
  shows `markdown` in `declared_features` but NOT in active `features`).
  Brief A scope (table+grid+float+pin) is CLEAR-TO-SPAWN. M5 markdown
  viewer needs a 1-line Cargo.toml feature flag addition as precondition.
  **H-arch-2 → RESOLVED-UNFALSIFIED:** `chart_tooltip::draw_tooltip` is
  a free function (`crates/ui/src/widgets/chart_tooltip.rs:68`) called
  inside `ChartProgram::draw` (`crates/ui/src/widgets/chart.rs:468`);
  `ChartProgram` implements `canvas::Program<Message>`
  (`crates/ui/src/widgets/chart.rs:226`); no `impl Widget` exists in
  `chart_tooltip.rs`. Canvas-internal dispatch confirmed; native `float`
  does NOT apply to chart_tooltip without first lifting it out of the
  canvas. **H-arch-7 → RESOLVED-UNFALSIFIED-partial:** sub-agent sandbox
  blocked `~/.cargo/registry/` reads, so direct `grep` of
  `iced_widget-0.14.2/src/table.rs` for `with_offset` / `virtual` /
  `row_provider` API was not executable. Indirect evidence is
  consistent with H-arch-7: `lazy` is a separate iced_widget feature
  (declared but OFF), with `lazy.rs` ABSENT from the compiled `.d`
  manifest — i.e. iced_widget partitions virtualization into a sibling
  module, not inside `table`. A3 (`agent_feed.rs`) stays HELD per
  architect decision; orchestrator may run the direct `table.rs` grep
  outside the sub-agent sandbox to convert to a full UNFALSIFIED.
  Sandbox caveat: T-M0-1 and T-M0-2 used a build-artifact-derived
  equivalent of `cargo doc` (per-crate rustc `.d` manifest + fingerprint
  JSON) because the sub-agent sandbox blocked both `cargo doc` and
  `~/.cargo/registry/` reads; the architect's primary falsifier path is
  unchanged for orchestrator-run confirmation if desired.
  HANDOFF → orchestrator (Brief A CLEAR-TO-SPAWN for table+grid+float+pin
  scope; markdown deferred pending Cargo.toml feature flag addition;
  H-arch-7 partial — agent_feed stays HELD).

- 2026-05-13 (operator, Q-O1 / Q-O2 / Q-O3 resolved at architect
  defaults):
  - **Q-O1 = bounded transitions allowed.** Constitutional amendment
    landed at
    [`spec/ui-design-principles.md:62`](../ui-design-principles.md):
    fade-in, focus-ring pulse, panel slide, spinner-during-real-I/O
    ALLOWED when motion signals an event; continuous motion without
    an event stays forbidden. Unblocks `iced_aw::spinner` (Brief B)
    and iced 0.14's Animation API for any future opt-in transitions.
  - **Q-O2 = ADOPT in-cockpit markdown viewer.** New M5 brief slot
    for the `viewer` bin: iced 0.14's native `markdown` widget
    renders `spec/operator-success-reports/reports/*.md` + backtest
    reports in-cockpit. Single feature flag, zero new deps.
  - **Q-O3 = brief order A → B → C → D unchanged.** Plus M5
    markdown viewer slots after Brief A (shares the iced feature
    flag dance).
  Status: research-brief candidate → research-brief operator-approved.
  Next step: M0 falsifier sub-agent runs H-arch-0 + H-arch-7 +
  H-arch-2 in a single read-only sandbox; Brief A's analyst gates on
  H-arch-0 PASS.

- 2026-05-13 (architect, synthesis pass — bumps version 0.1.0 → 0.2.0):
  Resolved Q1-Q10 per [`## Design — architect synthesis`](#design--architect-synthesis):
  Q1 = ADOPT native `table` for positions + strategies, HOLD agent_feed
  pending H-arch-7; Q2 = ADOPT native `grid` for kpi_strip; Q3 = ADOPT
  native `float` for journal_transaction_modal, DEFER for chart_tooltip
  (canvas-draw based, see H-arch-2); Q4 → operator (Q-O1
  constitutional clarification); Q5 = cherry-pick `iced_aw`
  (date_picker + spinner + badge); Q6 = DEFER `iced_dialog` gated on
  H-arch-6; Q7 → operator (Q-O2 product direction); Q8 = SPIKE-only on
  `sparkline.rs`; Q9 = WAIT on `iced_fonts`; Q10 = TRACK only.
  Authored 12 falsifiable hypotheses (H-arch-0 through H-arch-11)
  with orchestrator-runnable falsifiers (cargo doc + grep / cargo
  tree, no display server / live window). Grouped adoptions into 4
  briefs (A native widgets, B `iced_aw` cherry-pick, C `iced_dialog`
  gated, D plotters-iced2 spike) with recommended order A → B → C → D.
  Three operator-input Qs (Q-O1 / Q-O2 / Q-O3). No code changes; no
  crate adds. Candidate matrix `Verdict` column updated in-place (5
  cells: iced_aw → ADOPT cherry-pick; iced_dialog → DEFER; iced_fonts
  → DEFER; plotters-iced2 → SPIKE-only). Owner flipped analyst →
  architect.
  HANDOFF → orchestrator (route Q-O1 / Q-O2 / Q-O3 to operator, then
  spawn the M0 falsifier sub-agent for H-arch-0 + H-arch-7, then spawn
  Brief A's analyst on operator approval).
- 2026-05-13 (analyst, initial draft): Authored brief in response to
  operator's iced-ecosystem-evaluation prompt 2026-05-13. Surveyed 22+
  candidate crates via WebSearch + WebFetch (iced-rs/awesome-iced,
  crates.io, lib.rs, docs.rs, HN). **Five native iced 0.14 widgets
  (table, grid, markdown, float, pin)** identified as unused-but-already-
  in-lockfile. Top ADOPT/EVALUATE candidates: native table, native grid,
  native float, native markdown, `iced_aw` (cherry-picked), `iced_dialog`.
  Top SKIP candidates: `iced_plot` (wgpu-only), `plotters-iced`
  (iced 0.13), `iced-anim` family (forbidden by design constitution),
  `libcosmic` (iced fork), `iced_toasts` (iced 0.13). 10 open questions
  for architect (Q1-Q10), 5 seeded falsifiable hypotheses (H1-H5).
  HANDOFF → orchestrator (architect spawns next per the workflow
  contract). No code changes. No crate adds.
