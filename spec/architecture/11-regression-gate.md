---
slug: architecture-11-regression-gate
status: shipped
owner: tester
updated: 2026-06-08
---

# Regression gate

The body-SHA-256 regression gate that prevents silent output drift
in backtest, audit, and report-rendering paths. Every PR that
touches the load-bearing crates passes through this gate before
VERDICT → PASS.

## Dual-anchor system (ADR-0045 § D6 + § D7)

There are TWO complementary anchor systems, not one:

**System 1 — File anchors (`spec/anchors.toml` + `verify_anchors.sh`)**

`spec/anchors.toml` is the **single source of truth** (canonical).
It maps `(scenario, version)` pairs to body-SHA-256 values of SAVED
report files under `spec/*/reports/`. `scripts/verify_anchors.sh`
checks these saved files against the locked SHAs. This system catches
accidental edits to anchored report files; it does NOT catch engine
drift (because the saved files are unchanged when the engine drifts).

**System 2 — In-test re-run anchors (`determinism.rs`)**

`crates/backtest/tests/determinism.rs` contains `const ANCHOR` sites
inside `t622_*`, `t717_*`, and `tt1_*` test functions. These tests
RE-EXECUTE the backtest engine from scratch and compare the live
output body-SHA to the in-test constant. They guard against silent
engine drift: if the engine's output changes, these tests go RED.
The in-test constants for the non-feature-gated suite (default binary,
no `realdata`/`candle` features) mirror the `v5-realdata-medium-2026-05`
rows in `anchors.toml` (D6.1 mapping rule, ADR-0045 § D6).

**D7.1 drift-linter:** `scripts/check_determinism_anchors.py` is a
sub-second static check that asserts the System-2 in-test constants
equal the System-1 canonical SHAs. Run it before every `cargo test`
and as a pre-commit gate.

**D7.2 release re-run gate:** The `verify-anchors` skill procedure
runs `cargo test --release -p backtest --test determinism -- t622_ t717_ tt1_`
AFTER `verify_anchors.sh` exits 0. Release mode is mandatory (debug
builds are multi-minute on the momentum scenarios; release is ~26s).

**D6.1 mapping rule:** the non-cfg-gated `determinism.rs` tests build
the binary with no `realdata`/`candle` feature, so every scenario takes
the `Linear{bps:8}` synthetic fallback in
`build_slippage_model_for_scenario`. Each in-test constant must equal
the `anchors.toml` row `(scenario, version = "<base> + v5-realdata-medium-2026-05")`.
Exception: scenarios whose `v5-realdata-medium-2026-05` anchor was
produced with real Binance data (Parquet auto-detect) require review
before re-locking the tempdir-based tests (see ADR-0045 § D7 pending
items for macd/rsi/bbands).

## Current anchor set

The locked anchors live in [`../anchors.toml`](../anchors.toml).
At Phase 1A close, the set was 11 scenarios (the count grew from 9
during the v1+ operator-reports and v2 LLM ships). The LIVE set has since grown well beyond that (post-Phase-1A: the v2.5/v2.6 TCN-overlay + realdata locks and the `mc-robustness-2026-06` Monte-Carlo θ-surface anchors); [`scripts/verify_anchors.sh`](../../scripts/verify_anchors.sh) is the SOLE source of truth for the current count — do not hard-code it here. The Phase-1A-close snapshot:

| Scenario                            | Version | Owner ADR                                                          |
|-------------------------------------|---------|--------------------------------------------------------------------|
| `btc-2023-1m-sma-cross`             | v0      | [ADR-0005](adr/0005-v0-strategy-trait-no-hotload.md) (v0 trait shape) |
| `btc-2023-1m-sma-baseline-refresh`  | v0      | [ADR-0005](adr/0005-v0-strategy-trait-no-hotload.md)               |
| `btc-2023-1m-macd-trend`            | v0.5    | [ADR-0006](adr/0006-v05-config-driven-composition.md)              |
| `btc-2023-1m-rsi-reversion`         | v0.5    | [ADR-0006](adr/0006-v05-config-driven-composition.md)              |
| `btc-2023-1m-bbands-mean-revert`    | v0.5    | [ADR-0006](adr/0006-v05-config-driven-composition.md)              |
| `top10-2023-1h-momentum`            | v1      | [ADR-0013](adr/0013-v1-cross-sectional-momentum.md)                |
| `top10-2024-h1-momentum`            | v1      | [ADR-0013](adr/0013-v1-cross-sectional-momentum.md)                |
| `pairs-2023-zscore-mr`              | v1.5a   | [ADR-0014](adr/0014-v15a-mean-reversion-pairs.md)                  |
| `pairs-2024-h1-zscore-mr`           | v1.5a   | [ADR-0014](adr/0014-v15a-mean-reversion-pairs.md)                  |
| `report-sample-7d`                  | v2.0.0  | [ADR-0015](adr/0015-operator-success-reports.md) + [ADR-0019](adr/0019-v2-llm-strategy.md) Q11 |
| `report-sample-90d`                 | v2.0.0  | [ADR-0015](adr/0015-operator-success-reports.md) + [ADR-0019](adr/0019-v2-llm-strategy.md) Q11 |

## The body-vs-frontmatter discipline

Every anchored report file is a markdown document with YAML
frontmatter. The body-SHA-256 hash covers everything **after** the
closing `---` of the frontmatter. Run-varying values (timestamps,
host, pid, git commit, wall-clock seconds, data-source path) live
in the frontmatter and are excluded from the hash. The deterministic
content of the run lives in the body.

The HF-1 incident on 2026-04-18 forced this discipline: a
`wall_clock_s: f64` value leaked into the body and broke the
9-anchor gate. The fix had two parts: move the leaking field to
frontmatter, and widen audit-DB timestamps to 6-digit
fractional-second precision so concurrent inserts don't tie on
`ORDER BY ts`. See [ADR-0004](adr/0004-fractional-second-timestamps.md)
for the latter.

The full body-vs-frontmatter table is the developer agent's
authoritative reference at
[`../../.claude/agents/developer.md` § Body-vs-front-matter discipline](../../.claude/agents/developer.md#body-vs-front-matter-discipline).

## Determinism prerequisites

The gate only holds if the upstream determinism invariants hold.
The three load-bearing rules:

- **RNG.** All randomness uses `ChaCha20Rng::from_seed(...)` with
  a seed declared in the feature's `feature.md` frontmatter. No
  `thread_rng()`, no `OsRng`, no `SystemTime`-derived seed. See
  [ADR-0002](adr/0002-rng-chacha20.md).
- **Money math.** `rust_decimal::Decimal` wrapped in `Money<C:
  Currency>` everywhere. No `f64` for money. See
  [ADR-0003](adr/0003-decimal-money-math.md).
- **Audit-DB timestamps.** 6-digit fractional-second format
  (`%Y-%m-%dT%H:%M:%S%.6f`) on every column. See
  [ADR-0004](adr/0004-fractional-second-timestamps.md).

A regression in any of these surfaces as an anchor body-SHA diff,
which the tester's `verify-anchors` skill catches mechanically.

## Mechanical enforcement

The tester runs `scripts/verify_anchors.sh` as a non-negotiable gate
before any `VERDICT → PASS` that touches `crates/strategy/`,
`crates/audit/`, `crates/exec/`, `crates/backtest/`, or report
rendering. The script computes body-SHA-256 for each scenario's
latest report under `spec/<slug>/reports/` and compares against the
locked value in `../anchors.toml`. Exit code 0 = all PASS; non-zero
= at least one FAIL or MISS.

The corresponding skill documentation is at
[`../../.claude/skills/verify-anchors/SKILL.md`](../../.claude/skills/verify-anchors/SKILL.md).
The supporting scripts are `scripts/hash_report.py`,
`scripts/verify_anchors.sh`, `scripts/prune_backtest_duplicates.sh`,
and `scripts/pre_stage_anchors.sh`.

## Locking a new anchor

When a strategy or report ships and produces deterministic byte-
identical output across two `--release` runs at the same seed, the
architect approves the new scenario and the tester locks it once
into `anchors.toml`. The locking discipline:

1. Two sequential `cargo run --release --bin backtest -- --scenario
   <name>` runs.
2. `scripts/hash_report.py spec/<feature>/reports/backtest-*-<name>.md`
   for both runs.
3. If hashes match → append the new `[[anchors]]` row to
   `../anchors.toml` under the new version. If they don't match →
   route `HANDOFF → developer (non-determinism in the new
   scenario)`.

The architect owns approval; the tester owns the mechanical lock.
Neither operates unilaterally.

## Anchor mutation policy

Anchors do not mutate after locking without an explicit ADR. The
two cases that legitimately re-lock an anchor:

- The R6 reflection-memory placeholder lifecycle. The v1+ report
  ships R6 as a fixed placeholder string. When reflection-memory
  ships, the `report-sample-*` anchors re-lock **once** — not
  patched in place. See
  [ADR-0015](adr/0015-operator-success-reports.md) Q9.
- The v2 LLM-spend denominator hot-fix. The `report-sample-7d`
  and `report-sample-90d` anchors re-locked once at
  `T_FINAL_V2_LLM_STRATEGY` per
  [ADR-0019](adr/0019-v2-llm-strategy.md) Q11.

Both are documented one-shot re-locks. The default rule —
"anchors are immutable" — holds otherwise.

## Changelog
- 2026-06-08 (architect, doc-hygiene): "Current anchor set" caveat — replaced
  the hard-coded live anchor count (was `87`; had re-rotted, live was 119) with
  a pure pointer to [`scripts/verify_anchors.sh`](../../scripts/verify_anchors.sh)
  as the SOLE source of truth, so the caveat no longer re-rots on each anchor
  wave (audit-2026-06-08 SC-B / P1). No number restated.
- 2026-05-30 (developer / architect): D7.3 dual-anchor model documented
  (ADR-0045 § D7). Added the "Dual-anchor system" section covering
  System 1 (file anchors, `anchors.toml`) and System 2 (in-test re-run
  anchors, `determinism.rs`), the D7.1 drift-linter, the D7.2 release
  re-run gate, and the D6.1 mapping rule.
- 2026-05-13 (tester / architect): body synthesised from
  `../anchors.toml`, the `verify-anchors` skill, and ADRs 0002 /
  0003 / 0004 / 0015 / 0019 during Phase 1A Session 12. This file
  is the canonical operator-facing description of the regression
  gate; the mechanical implementation lives in the skill + scripts.
