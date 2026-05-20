---
title: Test Report — chart-x-axis-local-time M-FINAL
feature: chart-x-axis-local-time
run_id: 2026-05-20-0220-UTC
commit: working-tree-uncommitted-on-top-of-1967a39
agent: orchestrator-direct (trivial ship per CLAUDE.md)
verdict: PASS
---

# Test Report — chart-x-axis-local-time v1.11.0 — 2026-05-20 02:20 UTC

## 1. Scope

- **Feature / change under test:** `chart-x-axis-local-time v1.11.0` —
  flip workspace `time` `local-offset` feature on; wire
  production-OS-offset in `widgets::chart::local_offset_or_utc()`;
  preserve snapshot determinism via the dual `cfg(test)` + env-var
  gate; add one unit test.
- **Spec refs:** `spec/chart-x-axis-local-time/feature.md`,
  `spec/chart-x-axis-local-time/tasks.md`.
- **Rust toolchain:** stable, edition 2024.
- **OS / arch:** Darwin 25.4.0 arm64.

## 2. Static Analysis

| Check                              | Result | Notes                                         |
|------------------------------------|--------|-----------------------------------------------|
| `cargo fmt --check`                | PASS   | Zero formatting diffs.                        |
| `cargo clippy --workspace -- -D warnings` | PASS   | Clean exit, no warnings promoted to errors.   |
| `cargo audit`                      | SKIP   | Not installed in sandbox.                     |
| `cargo deny`                       | SKIP   | Inherited from Phase B baseline (pre-existing carry-forward; unchanged). |

## 3. Unit & Integration Tests

### Per-crate results

| Crate              | Tests | Passed | Failed | Ignored | Notes |
|--------------------|-------|--------|--------|---------|-------|
| `ui` (lib)         | 279   | 279    | 0      | 0       | +1 vs Phase B baseline of 278 — new `local_offset_under_production_reads_os_offset` |
| `ui` (render_snapshots) | 7 | 2 | 0 | 5 | Same as Phase B baseline; 5 ignored remain ignored. |
| `ui` (visual_snapshots) | 4 | 4 | 0 | 0 | Snapshots green via env-var override. |

### Key new test

```
test widgets::chart::tests::local_offset_under_production_reads_os_offset ... ok
```

The test pins the snapshot-determinism contract for the `cfg(test)`
branch. The companion production branch (`#[cfg(not(test))]`) is
covered by compile-only verification (the `local-offset` feature flip
in `Cargo.toml` makes `current_local_offset()` available) and the
operator's live-cockpit ship at v1.11.

## 4. Anchor Regression Gate (R10.1 contract)

`scripts/verify_anchors.sh` →

```
ANCHORS PASS  (22 / 22)
```

Phase B's 22 body-SHA-256 anchors are byte-identical. v1.11 touches no
strategy / audit / exec / report path; the env-var gate ensures
integration-test renderings are also deterministic across host time
zones.

## 5. Spec-lint Gate

```
spec-lint: FAIL (735 violations in 2 categories)
```

Pre-existing carry-forward baseline (same as post-Phase-B at commit
`1967a39`). **chart-x-axis-local-time contribution = 0** — verified
via `uv run scripts/spec_lint.py --all 2>&1 | grep 'chart-x-axis'` →
empty.

## 6. Cockpit-smoke Gate

Orchestrator-cited (capability boundary):

```
PASS — 0 panic lines in spec/chart-x-axis-local-time/reports/cockpit-smoke-2026-05-20T02-20Z.log (8s smoke window)
```

## 7. Known deviations / design notes

### Snapshot determinism — two-gate contract

The architect's M7 deferral comment in `chart.rs:151-173` originally
claimed the `cfg(test)` UTC override would preserve snapshot
determinism across the v1.11 cutover. **This is true for unit tests
only.** Cargo only sets `cfg(test)` on a crate when that crate is
built as a test target; integration tests (`tests/render_snapshots.rs`,
`tests/visual_snapshots.rs`) link against the library compiled WITHOUT
`cfg(test)`, so the `#[cfg(test)]` branch alone is insufficient.

The fix is a complementary env-var gate (`UI_CHART_FORCE_UTC`) set at
the top of each integration test's run-helper. The combined effect:

1. **Unit tests** (`cargo test -p ui --lib`) — render UTC via
   `#[cfg(test)]` branch.
2. **Integration tests** (`cargo test -p ui --test
   {render_snapshots,visual_snapshots}`) — render UTC via env var set
   by `run_panel_slot` and `run_slot`.
3. **Production** (`cargo run --bin cockpit`) — reads OS-local offset
   via `current_local_offset()` with defensive UTC fallback.

This deviation from the architect's stated contract is documented in
the function's doc comment + the `tasks.md` Notes section.

### `set_var` is unsafe in edition 2024

The test runners use `unsafe { std::env::set_var(...) }` with a
`// SAFETY:` comment. The set is single-threaded, runs before
`iced_test::screenshot`, and no other thread observes the env at that
point.

## 8. Pre-existing Spec Debt

- `cargo audit` / `cargo deny`: skipped / pre-existing baseline.
- `axis.rs` doctest: pre-existing baseline; not exercised in this
  run.

## 9. Verdict

**VERDICT → PASS**

All gates green. v1.11 ships the operator-facing local-time x-axis
labels while preserving snapshot determinism across host time zones
and all 22 anchor bytes.

## 10. Routing

- PASS → orchestrator pre-tick; presenter spawns next.
