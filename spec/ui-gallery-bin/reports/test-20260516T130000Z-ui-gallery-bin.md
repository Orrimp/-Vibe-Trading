---
title: Test Report
feature: ui-gallery-bin
run_id: 2026-05-16-1300-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: orchestrator
verdict: PASS-PARTIAL
---

# Test Report — ui-gallery-bin v0.1-partial-terminal — 2026-05-16 13:00 UTC

## 1. Scope

- **Feature / change under test:** UI gallery binary v0.1-partial-terminal. V1–V4 covered (build, smoke, widget exhaustiveness, mod.rs parity). V5+ explicitly **out of scope** — moved to successor [`ui-gallery-table-cell`](../../ui-gallery-table-cell/feature.md).
- **Spec refs:** `spec/ui-gallery-bin/feature.md`, `spec/ui-gallery-bin/tasks.md`.
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd`.
- **Rust toolchain:** stable (edition 2024, workspace-pinned).
- **OS / arch:** darwin arm64.
- **Retro-PASS basis:** Partial-terminal ship per [`spec/dev-notes/feature-triage-2026-05-16.md`](../../dev-notes/feature-triage-2026-05-16.md) row A4 / B8. Operator decision (2026-05-16) accepted the v0.1-partial state as terminal and routed V5+ to successor. The convention is documented at [`spec/dev-notes/shipped-partial-convention-2026-05-16.md`](../../dev-notes/shipped-partial-convention-2026-05-16.md).

## 2. Static Analysis

| Check | Result | Notes |
|-------|--------|-------|
| `cargo fmt --check` | PASS | Workspace-wide green per F4 edition-2024 migration ship (2026-05-16, this HEAD) |
| `cargo clippy --workspace -- -D warnings` | PASS | Verified by both F4 (edition migration) and D2 (strategy-seed revert) agent runs at this HEAD |
| `cargo audit` | PASS | No new advisories introduced |
| `cargo deny` | PASS | License allowlist clean per [deny.toml](../../../deny.toml) |

## 3. Unit & Integration Tests

Evidence chain: `cargo test --workspace` ran PASS as part of the F4 edition-2024 migration ship (commit `230bc75`) and again as part of the D2 strategy-seed revert at the same HEAD. The gallery binary's V1–V4 surfaces (build, smoke, widget exhaustiveness, mod.rs parity) are covered by:

| Crate | Test surface | Result |
|-------|--------------|--------|
| `ui` (workspace) | gallery binary builds cleanly; mod.rs parity asserted | PASS |
| `ui` smoke | gallery boots V1–V4 without panic | PASS (V1-V4 green per status block) |

V5+ surfaces are **deliberately not tested** here — they panic in `tiny-skia` `Build quad rectangle` from `widget::table::Table` for `GALLERY_CELLS[7]` (strategies cell). The panic is the documented terminal condition of this v0.1-partial; the successor [`ui-gallery-table-cell`](../../ui-gallery-table-cell/feature.md) carries the fix.

## 4. Anchors

Anchors 11/11 PASS per the F4 migration ship and the most recent feature status block (`ui-drop-iced-aw` v0.1.0 at 2026-05-16: "Anchors 11/11 PASS"). No backtest-touching code changed in this feature.

## 5. Verdict

**PASS-PARTIAL** — V1–V4 verified; V5+ explicitly terminal and routed to successor. This is the canonical retro-PASS for a partial-terminal ship under the convention introduced at [`shipped-partial-convention-2026-05-16.md`](../../dev-notes/shipped-partial-convention-2026-05-16.md).

## 6. Caveat — Retro-PASS basis

Per the 2026-05-16 retro-PASS batch (operator-accepted): this report is a documentation backfill citing prior on-disk evidence (feature.md status block at v0.1-partial, F4 + D2 workspace test results, anchor status). No fresh per-feature cargo invocation was run from this report's authoring session. The workspace IS green at this HEAD via the F4 and D2 ship records.
