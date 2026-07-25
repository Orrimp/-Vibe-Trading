---
adr: 0023
title: iced is the single UI stack across the project
status: accepted
date: 2026-04-17
supersedes: none
superseded-by: none
---

# ADR-0023: iced is the single UI stack across the project

## Context

The project ships two user-facing surfaces: a live ops cockpit
(real-time agent state, kill switch, P&L) and an offline backtest
viewer (markdown report shell + equity curve + drawdown band). Both
share types from `trading_core` and a read-only audit query surface.
The choice of UI stack determines both how those surfaces are built
and how cross-cutting concerns (subscriptions to the agent's event
bus, accessibility, theming) compose.

## Decision

Single UI stack across the project: [iced](https://github.com/iced-rs/iced).
No mixing with `egui` / `tauri` / `dioxus`. Both binaries (`cockpit`
/ `cockpit_live` / `viewer`) live in the `ui` crate.

- `iced` — Elm-architecture (`Model` / `Message` / `update` / `view`
  / `Subscription`); GPU-accelerated via wgpu; multi-window.
- `iced_aw` — community widgets (date pickers, modals, tabs, badges).
- `plotters` with `plotters-iced` backend — equity curves, indicator
  overlays, drawdown plots in the backtest viewer. Architect-side
  spike of `plotters` vs hand-rolled `iced::widget::Canvas` for the
  live candlestick view ran during chart-canvas-overhaul (v1.9 —
  [ADR-0020](0020-chart-buy-sell-emphasis.md)); custom Canvas won.

## Why iced fits

- **Subscriptions** wrap our `tokio::sync::mpsc` and
  `BroadcastStream` feeds — the existing actor pattern composes
  directly. No bespoke glue code.
- **Pure `update` functions** make every state mutation reviewable;
  matches the auditability goal in
  [`../product.md`](../../../../docs/archive/pre-bmad-spec/product.md).
- **Multi-window** lets ops cockpit (real-time) and backtest viewer
  (offline) run as separate top-level apps in the same crate,
  sharing widgets.

## Alternatives considered

- **`egui`.** Immediate-mode; simpler for prototypes but no
  Elm-architecture purity, no first-class Subscription concept, and
  rebuilds the whole UI per frame. Conflicts with the
  "every state mutation reviewable" goal. Rejected.
- **`tauri`.** Web-stack frontend over a Rust backend. Forces
  JS/TS/HTML/CSS into the project; doubles the toolchain. Rejected.
- **`dioxus`.** React-style hooks in Rust. Younger; ecosystem
  smaller; less mature for a project that wants to ship today.
  Rejected.
- **Web app + remote agent.** Out of scope; cockpit is a single-box
  desktop binary by design. Rejected.

## Consequences

- The UI crate's dependency graph is locked: `ui → {trading_core,
  audit}`. Direct UI imports of `strategy` / `exec` / `models` /
  `llm` are reject-on-review. This is what makes UI swappable
  without touching trading logic.
- The Lumen design system ([ADR-0018](0018-lumen-phase-1-foundation.md))
  exists because iced gives us a real theme/styling surface to build
  it against. The detailed UI architecture (cockpit screen routing,
  `audit::query` API, KPI strip, status bar) lives in
  [`../../../../docs/archive/pre-bmad-spec/architecture/06-ui-and-cockpit.md`](../../../../docs/archive/pre-bmad-spec/architecture/06-ui-and-cockpit.md), not in this
  ADR.
- Framework version gaps (e.g. iced 0.14.2 lacking
  `button::Status::Focused`, ADR-0018 Q11) are addressed via
  bounded best-effort with documented upgrade triggers — never by
  switching to a different stack.

## Changelog
- 2026-04-17 (architect): initial accept.
- 2026-05-13 (architect): extracted from `docs/archive/pre-bmad-spec/architecture.md` §
  Foundation libraries — Frontend — iced during Phase 1A Session 11.
  Detailed UI architecture body (cockpit screen routing,
  audit::query API surface, KPI strip widget contracts, etc.) moved
  to `06-ui-and-cockpit.md`.
