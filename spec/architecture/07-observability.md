---
slug: architecture-07-observability
status: shipped
owner: architect
updated: 2026-05-13
---

# Observability

Tracing, metrics, structured logs — the operational surface the operator
sees during live runs and the tester sees during backtests.

- `tracing` with JSON output. Library code uses `tracing::{debug, info,
  warn, error}`; no `println!`.
- Metrics via `metrics` + `metrics-exporter-prometheus` (default picks
  per [10-foundation-libraries.md](10-foundation-libraries.md)). Each
  crate exposes its own metric set; agent aggregates.
- Structured logs to `logs/` plus stdout. Log rotation handled at the
  process layer, not in-band.
- No clocks in UI tests (`scripts/check_no_clocks_in_ui_tests.sh`
  enforces). Live cockpit reads from the bus, never from `Instant::now`
  in `view`.

## Changelog
- 2026-05-13 (architect): content migrated from `spec/architecture.md` §
  Observability during Phase 1A Session 3. Added explicit pointer to the
  no-clocks-in-UI-tests guardrail since it lives at the boundary between
  observability and UI testability.
