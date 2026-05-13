# Session handoff — 2026-05-13

Resumption breadcrumb for the next orchestrator. **Read this first.**

## Branch state

- `main` is **11 commits ahead of origin/main**, unpushed by operator's call.
- Working tree clean (`git status --short` empty).
- Anchors PASS `11/11` byte-identical.
- Workspace tests `1203+ passing / 0 failed`.

## What shipped 2026-05-12 / 2026-05-13 (most recent first)

| Feature | Commits | One-line |
|---|---|---|
| `iced-native-widgets v0.1.0` (Brief A) | `3077425` `970e857` `9e5bd65` `9027a0d` `1431409` `77d3a89` | 4 hand-rolled widgets → native iced 0.14 (table / grid / float); shared Catalog adapter scaffold at `crates/ui/src/theme/iced_widget_catalogs.rs` |
| `v2-llm-strategy v2.0.0` | `d0bcad2`→`faaaec1` + `8a41b47` `d8c3a99` | LLM substrate (trait + 3 providers + retry + cache + budget + strict replay + smoke + runbooks); foundation-only per Q1=A |
| `ui-test-harness-bootstrap v0.1` | `55c46a0` | `iced_test` smoke + canvas hit-test grid + image-compare diff + 3 viewport PNG baselines |
| `chart-canvas-overhaul v1.10.0` | `96ba58b` | Axes + gutters + legend + viewer parity + window-size bump |
| Workflow + tooling commits | `c264ced` `a34e702` | AGENT.md `## Capability boundaries`, UI testing strategy dev-note, `scripts/orch_*` |

## Backlog at handoff

- **Active**: empty
- **Queue / Process / tooling** (next natural candidates):
  - **Brief B — `iced_aw` cherry-pick** (date_picker + spinner + badge). Consumes today's `cockpit_table_style_fn` Catalog adapter. Filed per architect's adoption priority A→B→C→D in `spec/iced-ecosystem-evaluation/feature.md`.
  - **M5 — in-cockpit markdown viewer**. Needs **one Cargo.toml feature flag**: `iced = { ..., features = [..., "markdown"] }` at `crates/ui/Cargo.toml:69`. Operator-approved via Q-O2.
  - **`v2-llm-strategy-v21-followups`** — T1938 cockpit "LLM budget" tile + T1915 tracing-Layer redactor + T1910 pedantic clippy. All deferred from v2.0.0.
  - **`ui-test-harness-canvas-state-seeding`** — closes the render-half of chart-canvas-overhaul V15 (V8 in bootstrap). Operator decision 2026-05-12: "Commit — V14 covered, V15 partial-accept".
  - **Week 2 / 3 / 4** of the UI testing 4-week plan (viewport matrix / evaluator PreToolUse hooks / GitHub Actions CI).
- **Queue / Strategy** (unblocked by v2-llm-strategy ship):
  - **v2.5 Kronos** foundation-model forecast overlay
  - **Lumen Phase 6** Assistant slot
  - **reflection-memory-llm-enrichment** + **reflection-memory-trader-wiring**
- **Queue / UI**: `chart-x-axis-local-time` (Q4 deferral) / `cockpit-cross-platform` (Windows/Linux future) / `cockpit-app-bundle` (macOS .app)

## Workflow rules now in force (read `AGENT.md` end-to-end)

1. **`AGENT.md ## Capability boundaries`** (added 2026-05-12, commit `c264ced`):
   - Orchestrator owns display/GPU/cursor/screencapture/cockpit-binary launch
   - Test-runner / evaluator split replaces single `tester` role
   - Architect = hypothesis only (with falsifiers)
   - Default to sequential when in doubt
2. **`AGENT.md ## Structured handoff envelope`** (added 2026-05-13, just before this handoff):
   - Sub-agents emit a TOML envelope in a fenced ` ```toml ` block alongside prose
   - Schema includes `[handoff]` `[inputs]` `[outputs]` sections
   - **None of today's sub-agent briefs included this** — they predate the amendment. Next session's spawns SHOULD include TOML-envelope instructions in their prompts.

## Orchestrator tooling shipped 2026-05-12

In `scripts/orch_*` (committed `a34e702`):

- `orch_crop.sh <png> <x> <y> <w> <h> <out>` — PNG crop with sane arg order
- `orch_probe_tcc.sh` — macOS Screen-Recording / Accessibility / Automation status
- `orch_supplement_log.sh <log> "<title>" -- <cmd...>` — append verbatim cmd output to a test-runner log (used twice now for sandbox-denied checks)
- `orch_determinism_check.sh -p <crate> --test <name> -- <glob>` — twice-run shasum diff
- `orch_cockpit_on_screen.sh <Screen>` + `orch_cockpit_off.sh` — patch cockpit.rs default screen + build + run + revert
- `orch_hover_screenshot.sh <x> <y> <out.png>` — cursor warp + CGEvent + screencapture
- `orch_cursor_move.swift` — CGWarp + CGEvent primitive (used by hover-screenshot)

Documented in [`spec/dev-notes/orchestrator-tooling-2026-05-12.md`](orchestrator-tooling-2026-05-12.md). All allowlisted in `.claude/settings.local.json`.

## Observed operator preferences (from this session)

- **Commits over push** — operator commits per logical chunk, defers `git push` until explicit "push" instruction.
- **Architect's defaults** — when architect surfaces operator Qs with a recommended default, operator typically picks it.
- **Honest framing** — operator values when an agent corrects an earlier overpromise (e.g. "LOC retirement was actually +154 net, not −900-1100").
- **Multi-pass dev with operator confirmations between** — v2-llm-strategy ran 6 dev passes, each with an operator "continue / split / stop" question between.
- **Resumption breadcrumbs** — operator paused v2-llm-strategy with a written breadcrumb (`spec/v2-llm-strategy/orchestrator-scope-check-2026-05-10.md`); pattern worked, resumed cleanly 2 days later.
- **Bounded transitions allowed** in `spec/ui-design-principles.md:62` per Q-O1 amendment 2026-05-13.

## Failure modes corrected this session — don't redo

- Architect ghost APIs caught by M0 falsifier batch: `Float::new(1 arg)` (architect said 2); orphan-rule violation on `impl Catalog`. **Pattern**: orchestrator runs 5-grep falsifier batch from unsandboxed shell after architect synthesis, before dev fan-out.
- Sub-agent sandboxes block `~/.cargo/registry/` reads, `cargo doc`, `screencapture`, `osascript`. **Pattern**: use `scripts/orch_supplement_log.sh` to fill those gaps into the test-runner log.
- LOC framing in research briefs was wrong twice (ecosystem-evaluation, Brief A analyst). **Pattern**: presenter section "Architectural divergences (honest)" + "Numbers that matter" with LOC table — operator wants the truth, not the original promise.
- `plotters-iced` is iced 0.13-pinned (analyst SKIP). `iced_plot` is wgpu-only (we pin tiny-skia). `iced-anim` family is forbidden by design constitution. Don't re-suggest these.

## Things I'd recommend the next session do first

1. Read `AGENT.md` end-to-end — especially the new ## Structured handoff envelope section (lines 134-160).
2. Skim `spec/backlog.md ## Recent (shipped)` top 4 entries for context on what just landed.
3. Decide with operator: push the 11 unpushed commits OR open the next candidate brief (Brief B is the natural next per architect's A→B→C→D priority).
4. When spawning sub-agents going forward, **include TOML-envelope instructions in the prompts** — none of my prompts this session did. The receiving-agent first-pass-parse benefit only kicks in if senders emit it.

## Anchors snapshot

```
PASS  btc-2023-1m-sma-cross
PASS  btc-2023-1m-sma-baseline-refresh
PASS  btc-2023-1m-macd-trend
PASS  btc-2023-1m-rsi-reversion
PASS  btc-2023-1m-bbands-mean-revert
PASS  top10-2023-1h-momentum
PASS  top10-2024-h1-momentum
PASS  pairs-2023-zscore-mr
PASS  pairs-2024-h1-zscore-mr
PASS  report-sample-7d      520b1f2968…  (re-locked to v2.0.0 by v2-llm-strategy T_FINAL 2026-05-12)
PASS  report-sample-90d     c656414ebf…  (re-locked to v2.0.0 by v2-llm-strategy T_FINAL 2026-05-12)
---
ANCHORS PASS  (11 / 11)
```
