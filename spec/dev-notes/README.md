---
slug: dev-notes-index
status: living
owner: architect
updated: 2026-07-10
---

# Dev-notes — categorized index

Cross-cutting memos: standing decisions, dated audits, analyses/postmortems, and
how-tos. This index is the front door — scan it first, then open the one note you
need. **Note bodies are dated history and are not rewritten** (only this index is
maintained). Notes tagged **LOAD-BEARING** encode a standing decision, contract, or
verification recipe that current work depends on — do not treat them as stale.

> For *what the system is now*, read
> [`../architecture/00-current-state.md`](../architecture/00-current-state.md)
> (crate map + invariants + advisor spine). For *what shipped*, read
> [`../../CHANGELOG.md`](../../CHANGELOG.md). This folder is the *working memory*
> behind those.

## Standing decisions & contracts

| Note | One line |
|---|---|
| [`do-not-build-register.md`](do-not-build-register.md) | **LOAD-BEARING.** The binding register of settled dead-ends (combination-search engine, live trading, band-loosening, the DSR crown-veto E-1, …) — do NOT re-propose these. |
| [`dsr-report-only-decision-2026-07-09.md`](dsr-report-only-decision-2026-07-09.md) | **LOAD-BEARING.** Operator decision: deflated-Sharpe stays *report-only* (informational `crown_clears_dsr`), never a crown veto (do-not-build E-1). |
| [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md) | **LOAD-BEARING.** The frozen, pre-registered block-bootstrap robustness rule (`classify_verdict` bands + seed rule) — the moat the whole gate rests on. |
| [`venue-trust-map-2026-07-01.md`](venue-trust-map-2026-07-01.md) | **LOAD-BEARING.** Which venues/feeds are trusted for reconciliation + cost realism (Binance/Coinbase/Kraken); the P2 second-venue basis. |
| [`live-trading-removed-2026-06-12.md`](live-trading-removed-2026-06-12.md) | **LOAD-BEARING.** Standing scope decision: live trading removed 2026-06-12 (paper/sim only) — do NOT re-propose live execution. |
| [`post-v2-scoping-2026-07-09.md`](post-v2-scoping-2026-07-09.md) | **LOAD-BEARING.** Scoping verdict: there is no coherent add-more-features v3 (honest-null); the product is feature-complete. |
| [`shipped-partial-convention-2026-05-16.md`](shipped-partial-convention-2026-05-16.md) | The `shipped-partial` status convention (code gates clean, one wave deferred for an external-dependency reason). |

## How-tos & references

| Note | One line |
|---|---|
| [`iced-ui-render-verification.md`](iced-ui-render-verification.md) | **LOAD-BEARING.** How to verify cockpit/UI at the rendered-PIXEL layer (`iced_test::Emulator::screenshot` harnesses + negative control). Cited by CLAUDE.md. |
| [`v3-vol-overlay-noop-discovery-2026-05-22.md`](v3-vol-overlay-noop-discovery-2026-05-22.md) | **LOAD-BEARING.** The computed-but-never-applied overlay no-op → the day-1 baseline-equity-divergence e2e non-negotiable. Cited by CLAUDE.md. |
| [`codegraph.md`](codegraph.md) | CodeGraph setup + the opt-in MCP wiring (dev/agent code-navigation aid; zero product/build effect). |
| [`operator-side-pending-ledger.md`](operator-side-pending-ledger.md) | Ledger of operator-side pending items (out-of-band actions the operator owes). Cited by AGENT.md. |
| [`lumen-accent-palette-extension-2026-05-17.md`](lumen-accent-palette-extension-2026-05-17.md) | Lumen design-system accent-palette extension (design tokens); consumed by `crates/ui/src/theme.rs`. |
| [`retired-surface-inventory-2026-05-22.md`](retired-surface-inventory-2026-05-22.md) | Inventory of retired code surfaces kept in-tree with anchors locked. Cited by README. |
| [`qlib-feature-gap-2026-06-17.md`](qlib-feature-gap-2026-06-17.md) | Gap analysis vs microsoft/qlib: the only scope-fitting gap was structural point-in-time data (→ ADR-0058/0086); model-zoo/RL/HFT out of scope. |

## Audits (dated series)

The recurring spec/integrity audit series + one-off buildout/triage audits. Each is
an immutable dated record; the newest is the current state of the audit trail.

| Note | One line |
|---|---|
| [`audit-2026-07-06.md`](audit-2026-07-06.md) | Weekly spec/integrity audit — trace-lifecycle story closed (ADR-0082 follow-through). |
| [`audit-2026-06-29.md`](audit-2026-06-29.md) | Weekly audit — surfaced the trace lifecycle-drift finding that became ADR-0082. |
| [`audit-2026-06-22-post-b1.md`](audit-2026-06-22-post-b1.md) | Post-B1 (benchmark-robustness) follow-up audit. |
| [`audit-2026-06-22.md`](audit-2026-06-22.md) | Weekly spec/integrity audit. |
| [`audit-2026-06-15.md`](audit-2026-06-15.md) | Weekly spec/integrity audit. |
| [`audit-2026-06-12.md`](audit-2026-06-12.md) | Weekly audit — origin of the `spec_lint` status-drift enforcement hook. |
| [`audit-2026-06-08.md`](audit-2026-06-08.md) | Weekly spec/integrity audit. |
| [`cockpit-buildout-audit-2026-06-08.md`](cockpit-buildout-audit-2026-06-08.md) | One-off cockpit build-out audit. |
| [`feature-triage-2026-05-16.md`](feature-triage-2026-05-16.md) | Feature-triage audit (origin of the `shipped-partial` convention). |

## Analyses & postmortems

| Note | One line |
|---|---|
| [`p2-wobble-thesis-analysis-2026-07-10.md`](p2-wobble-thesis-analysis-2026-07-10.md) | P2 corpus-expansion wobble decomposition + thesis-framing options → the era-qualified thesis. |
| [`robustness-gate-allfragile-analysis-2026-06-22.md`](robustness-gate-allfragile-analysis-2026-06-22.md) | Why the gate returned `AllFragile` on real crypto → the ADR-0066 benchmark exemption. |
| [`robustness-gate-allfragile-technical-2026-06-22.md`](robustness-gate-allfragile-technical-2026-06-22.md) | Technical companion to the `AllFragile` analysis (the `rank.rs` mechanics). |
| [`robustness-verdict-adversarial-review-2026-05-30.md`](robustness-verdict-adversarial-review-2026-05-30.md) | Adversarial review of the robustness verdict before it was frozen. |
| [`analysis-2026-06-15-simple-strategy-bear-survey.md`](analysis-2026-06-15-simple-strategy-bear-survey.md) | Simple-strategy survey on the 2021-22 bear corpus. |
| [`analysis-2026-06-15-simple-strategy-overfit-guard.md`](analysis-2026-06-15-simple-strategy-overfit-guard.md) | Overfit-guard analysis for the simple-strategy survey. |
| [`realdata-simple-strategy-survey-2026-06-13.md`](realdata-simple-strategy-survey-2026-06-13.md) | Real-data simple-strategy survey (precursor to the bear survey). |
| [`onchain-vs-conclude-fork-2026-06-08.md`](onchain-vs-conclude-fork-2026-06-08.md) | The on-chain-vs-conclude decision fork — why the active-edge hunt concluded. |
| [`onchain-netflow-spike-2026-06-08.md`](onchain-netflow-spike-2026-06-08.md) | On-chain netflow-spike signal analysis. |
| [`repo-cleanup-plan-2026-05-22.md`](repo-cleanup-plan-2026-05-22.md) | Repo-cleanup plan (retired-surface consolidation). |

## Historical / archived

Clearly-historical, **unreferenced** one-off notes moved out of the live listing on
2026-07-10 (remediation-plan P6b) — reference-checked to zero across the repo before
moving. Bodies are preserved verbatim.

- [`../archive/dev-notes/backlog-staleness-audit-2026-06-15.md`](../archive/dev-notes/backlog-staleness-audit-2026-06-15.md) — one-off backlog-staleness audit, superseded by the current backlog + later audits.
- [`../archive/dev-notes/fetcher-idempotency-fix-2026-06-16.md`](../archive/dev-notes/fetcher-idempotency-fix-2026-06-16.md) — postmortem of the (shipped) fetcher idempotency fix.
- [`../archive/dev-notes/flaky-charts-visual-test-fix-2026-06-15.md`](../archive/dev-notes/flaky-charts-visual-test-fix-2026-06-15.md) — postmortem of the (shipped) flaky visual-test de-flake.
- [`../archive/dev-notes/engine-drift-fix-handoff-2026-05-30.toml`](../archive/dev-notes/engine-drift-fix-handoff-2026-05-30.toml) — a transient dev handoff envelope.

The earlier bulk archive (session notes, scoping memos, retrospectives from Q2)
lives in `spec/dev-notes/archive/2026-Q2/` (a few of those are still referenced by
`README.md`'s "Key dev-notes" list and by ADRs; they are frozen in place).

## Changelog
- 2026-07-10 (architect): created as the remediation-plan **P6b** dev-notes
  consolidation — categorized index (standing decisions / how-tos / audits /
  analyses / historical); archived 4 unreferenced one-off notes to
  `spec/archive/dev-notes/` after a whole-repo reference check (all refs: 0). Note
  bodies untouched; the index is the deliverable.
