# Documentation

Human-facing documentation for the **Single-Coin Investment Advisor (paper)** — a Rust
decision-support + paper-simulation tool that answers one question: *"I have €200 for one
crypto — which strategy should I use, and what should I do over the next few days?"*

It works by **baking off every strategy** on your `(coin, window)`, **ranking** them under a
**frozen robustness gate** (with buy-and-hold always in the field as the benchmark), and then
**paper-trading** the pick forward on real data. It is **paper/sim only** and **not financial
advice** — and its honest, repeatedly-validated finding is that *no active strategy robustly
beats just holding*, so the product sells **measured honesty, not asserted alpha**.

> These docs describe the product as of the 2026-06-19 advisor pivot and the work shipped since.
> For the authoritative sources see [`spec/product.md`](../spec/product.md) (product),
> [`spec/architecture.md`](../spec/architecture.md) (design), [`CHANGELOG.md`](../CHANGELOG.md)
> (the per-feature shipped index), and [`AGENT.md`](../AGENT.md) (the dev workflow).

## The five documents

| Doc | What it covers |
|-----|----------------|
| [**Architecture**](architecture.md) | System design — the advisor journey as architecture, the layered 17-crate design + invariants, the core domain model, the backtest engine, the robustness gate (the credibility layer), the agent runtime, LLM integration, determinism & body-SHA anchoring, and the iced cockpit. |
| [**Structure**](structure.md) | Repository + code layout — the annotated top-level tree, the 17 crates (responsibility + entry points + a verified dependency graph), a module tour of the load-bearing crates, the `spec/` tree, and config/data/scripts/`.claude`. |
| [**Processes**](processes.md) | How work gets done — the spec-driven multi-agent pipeline (analyst → architect → developer ‖ ui-designer → tester → presenter → operator), the per-feature lifecycle + status vocabulary, the quality gates (clippy `--all-targets`, fmt, 119/119 body-SHA anchors, spec-lint, cockpit-smoke, pixel verification), and the non-negotiables. |
| [**End to End**](end-to-end.md) | The journey, user + system — Pick → Bake off → Rank → [Inspect / Tune] → Plan → [Promote] → Watch, with the engine path behind each step (`run_bakeoff` → `classify_verdict` → `rank_candidates` → the UI mirror → `ForwardCommand::Launch` → the ledger), and a worked example. |
| [**User Manual**](user-manual.md) | How to run + use the cockpit — build/run commands (both bins, `--release`), all 19 screens, the step-by-step guided journey (Leaderboard → Lab → Tune → ForwardPlan → Live), how to read the honesty signals (FRAGILE, "just hold", the promotion lock), and troubleshooting. |

## Diagrams

The docs use **Mermaid** diagrams (rendered by GitHub + most Markdown viewers): the journey
flow, the layered architecture, the crate dependency graph, the dev pipeline + gate flow, the
bake-off/promote sequence diagrams, and the cockpit navigation map.

## Quick start

```bash
# Build + run the cockpit (native window — run from your own terminal, in release)
cargo run -p ui --release --bin cockpit_live --features live
```

Then open the **Leaderboard**, pick a coin + €200 + a window, press **Run bake-off**, and follow
[the User Manual](user-manual.md). Full setup is in the [README](../README.md) § Quickstart.
