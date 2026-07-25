---
slug: v2-1-tracing-layer-redactor
kind: verification
date: 2026-06-16
---

# v2-1-tracing-layer-redactor — verification (archived-report pointer + pre-ship re-verify)

The M-FINAL tester report (VERDICT → PASS, 2026-05-29) is archived in
`spec/archive/tester-reports-2026-05-to-06.tar.gz` — the in-repo `reports/` dir was not
retained at the time. This note records the pointer plus the 2026-06-16 pre-ship
re-verification (and gives the shipped feature its `reports/` evidence).

## Tester verdict (2026-05-29, archived)

- **VERDICT → PASS.** T-RED-FINAL.1..5 ticked. 108 unit + 9 integration tests pass; 3
  `#[ignore]` falsification probes (P-RED-1/2/3) ship; anchors 84/84 byte-identical.
- Full report: `spec/archive/tester-reports-2026-05-to-06.tar.gz`.

## Re-verification (2026-06-16, orchestrator, pre-ship)

- `cargo test -p llm` → **108 lib passed + integration tests pass** (1 ignored P-RED-3
  probe), 0 failed.
- `cargo clippy --tests -p llm -- -D warnings` → **clean** (forced 57s recompile).
- Anchors unaffected (tracing-only feature; no backtest-report change).

## Disposition

Shipped 2026-06-16 (commit `b36fdb5`; operator-approved). v0.1.0 ships WARN-mode default
(`REDACT_LAYER_MODE=warn|gate`); the enforcing-gate flip is a later operator-decided v0.2.0.
