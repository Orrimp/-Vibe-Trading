---
slug: architecture-09-performance-budget
status: shipped
owner: architect
updated: 2026-05-13
---

# Performance budget

Latency and throughput targets each crate must respect. Regression-tested
in the criterion bench suite (`rust-bench` skill).

| Path                          | Budget        | Notes                                                 |
|-------------------------------|---------------|-------------------------------------------------------|
| Bar-close → signal (no LLM)   | < 5 ms p99    | Regression-tested in benches                          |
| Bar-close → signal (with LLM) | < 500 ms p95  | Only on regime-change triggers                        |
| Backtest throughput           | > 100k bars/s | per symbol, single thread                             |

Budgets land as `#[cfg(bench)]` assertions; a benchmark that exceeds its
budget fails the tester's `rust-bench` step. Loosening a budget requires
an architect-approved follow-up explaining why.

## Changelog
- 2026-05-13 (architect): content migrated from `spec/architecture.md` §
  Performance budget during Phase 1A Session 3.
