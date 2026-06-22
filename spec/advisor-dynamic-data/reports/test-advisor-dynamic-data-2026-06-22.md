---
title: Test Report — Tester Verification
feature: advisor-dynamic-data
run_id: 2026-06-22-1100-UTC
commit: c16a37ca507e8c8d5a37bf7598cdec819b4a3c25
agent: tester
verdict: PASS
---

# Test Report — advisor-dynamic-data — 2026-06-22 11:00 UTC

## 1. Scope

- **Feature / change under test:** Advisor dynamic on-demand market-data loading. `dynamic_cache::load_or_fetch_with` with mock and real (ignored) paths; anchor-safety proof that the dynamic cache never mutates the pinned Binance corpus.
- **Spec refs:** `spec/advisor-dynamic-data/feature.md`
- **Commit SHA:** `c16a37ca507e8c8d5a37bf7598cdec819b4a3c25`
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25)
- **OS / arch:** Darwin arm64

## 2. Static Analysis

| Check              | Result | Notes                                           |
|--------------------|--------|-------------------------------------------------|
| `cargo fmt --check`| PASS   | clean, exit 0                                   |
| `cargo clippy`     | PASS   | 0 warnings workspace-wide; forced re-lint via `touch crates/*/src/lib.rs` |
| `cargo audit`      | n/a    | no CVE-sensitive change in this feature         |
| `cargo deny`       | n/a    | no new deps added                               |

## 3. Unit & Integration Tests

### Data lib unit tests (`cargo test -p data --lib`)

| Crate / module | Passed | Failed | Ignored |
|----------------|-------:|-------:|--------:|
| `data` (all inline tests) | 102 | 0 | 3 |

**102 passed; 0 failed; 3 ignored** (the 3 ignored tests are real-fetch integration tests requiring `--features realdata` and live network access — correctly gated).

### Anchor-safety test (`cargo test -p data --features fixtures --test dynamic_cache_anchor_safety`)

| Test | Result |
|------|--------|
| `load_or_fetch_does_not_touch_pinned_corpus` | PASS |

**1 passed; 0 failed; 0 ignored.**

This test is the ADR-0061 D4 mandatory gate: proves `load_or_fetch_with` (a) does not create or modify any file under the pinned corpus fixture, (b) does not write a `REVISION.toml` under the dynamic root, and (c) leaves `read_and_verify_revision_manifest` on the pinned fixture returning the same aggregate SHA. Verified with a `MockFetcher` — no live network required.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no proptest/fuzz suites for this feature.

## 5. Backtest Results

_n/a_ — this feature is a data-layer addition; no new anchored backtest scenario.

Anchor regression gate re-verified this session:

```
bash scripts/verify_anchors.sh
ANCHORS PASS  (119 / 119)
```

## 6. Benchmarks

_n/a_ — no latency-sensitive hot path changed.

## 7. Environment / Infrastructure Issues

The 3 ignored tests (`binance_live_*` and similar) require `--features realdata` and live Binance API access. They are correctly `#[ignore]`-gated and excluded from CI. The feature spec documents these as operator-runnable verification only.

## 8. Verdict

**PASS**

102 data lib unit tests and the ADR-0061 D4 anchor-safety test all pass with 0 failures. The dynamic cache is proven not to touch the pinned corpus. Static analysis clean. Anchor gate 119/119 confirmed.

## 9. Routing

`VERDICT → PASS` — ready; feature.md status bumped to `shipped`.
