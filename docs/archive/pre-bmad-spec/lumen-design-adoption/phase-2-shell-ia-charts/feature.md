---
slug: lumen-phase-2-shell-ia-charts
status: shipped
owner: architect
updated: 2026-06-17
version: 2.1.0
<!-- last-edited: 2026-05-05 (tester): status active → shipped on T_FINAL_LUMEN_PHASE_2 PASS. All 8 gates green first-pass; report `spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`. HANDOFF → presenter. -->
<!-- last-edited: 2026-05-04 (architect): appended `## Design` resolving Q1–Q11 (11/11 ratified, zero deviations). Cockpit state diff (Screen enum × 6, ChartBuffer cap 60, three new Message variants); sidebar nav widget contract; chart widget contract (canvas, line series, single-symbol); recent_fills_filtered (since: Timestamp, until: Timestamp); synthetic_candles per-symbol seed via DefaultHasher; right-rail Length::Fixed(0.0). TD-1 deferred — iced still =0.14.0 on disk. Task list at spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md (T1601–T1616 + T_FINAL). HANDOFF → developer ‖ ui-designer. -->
---

# Lumen Phase 2 — Shell IA + Charts (sidebar nav · Home/Debug/Charts screens · price chart)

**shipped — compressed 2026-06-17.** One-line description and version: see [CHANGELOG.md](../../../CHANGELOG.md). Full narrative history: `git log -- spec/lumen-design-adoption/phase-2-shell-ia-charts/`. Backtest evidence (if any) is preserved under `reports/`.
