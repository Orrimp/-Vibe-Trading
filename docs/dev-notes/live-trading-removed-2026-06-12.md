---
slug: live-trading-removed
status: decided
owner: orchestrator
updated: 2026-06-12
---

# Live trading removed from the project — operator decision, 2026-06-12

## The decision

The operator directed: **"Remove Live Trading from the Project. We can have
real data to Backtest and check trading strategies but there will be no live
trading for a long time."**

The entire live-money execution program built earlier on 2026-06-12 was
**removed from `main`** the same day. Git history preserves every line for
recovery; the working tree no longer carries any live-execution capability.

## What was removed

The live program was a contiguous block of 8 commits (`af9d0a9` … `edbbb10`)
on top of the pre-live commit **`a063d79`**. Removal = restore every modified
file to `a063d79` + delete every added file (byte-exact; verified by an empty
`git diff --cached a063d79`).

- **Code:** `crates/exec/src/live/` (the whole `BinanceSpotExecClient`:
  `mod/sign/clock/filters/cap/error/endpoint/types`), `crates/core/src/secret.rs`
  + `crates/agent/src/secret.rs` (`SecretSource`/`SecretString`), the
  `AccountReader` + two-class reconciliation seam in `crates/agent/src/reconciler.rs`
  (reverted to the paper/research reconciler), the `hmac`/`hex` deps, and the
  three live test suites (`live_exec_adversarial`, `binance_testnet_live`,
  `live_reconcile_adversarial`).
- **Spec:** `spec/live-passive-execution-readiness/` (umbrella),
  `spec/live-exec-client-binance-spot/` (F1: feature/tasks/reports/deck),
  `_bmad-output/planning-artifacts/architecture/decisions/0054-mode-live-boundary.md` (+ its ADR-README registry
  row), the four `REQ-LIVE-*` trace rows, and the `product.md` § Non-goals /
  § Project-scope boundary **amendment** (product.md restored to its clean
  pre-amendment state: real-money execution is once again a flat non-goal).

## What was KEPT (explicitly in scope per the decision)

- **Real market data** — the read-only Binance feed (`crates/data/src/binance.rs`),
  Yahoo (ADR-0040), and the parquet data domain. None of it is live trading;
  none was touched.
- **Backtesting + strategy research** — the operator explicitly wants real data
  to backtest and check trading strategies. The backtest crate, the montecarlo /
  robustness harness, and the 119 anchors are untouched (119/119 PASS).
- **Paper simulation** — `mode = "paper"` (the `paper-mode-equity-wiring`
  feature) is SIMULATED execution (`PaperEngine`, no real exchange) and stays.
  It consumes the read-only live-data feed but places no real orders.
- The read-only **cockpit live view** (`ui::live`, `cockpit_live`) — market-data
  monitoring, never order placement.

## Verification of the removal

`cargo check --workspace --all-targets` clean; `exec/agent/core/audit` 419/0;
`ui --lib` 447/0; `verify_anchors.sh` 119/119; `spec_lint.py` 70 (== baseline,
zero new) + `--self-test` PASS; `adr_registry_check.py` exit 0. The working tree
is byte-identical to `a063d79` for every touched path.

## Notes for future agents

- **Do not re-propose live execution** as a "next step" without an explicit
  fresh operator request. The standing position is: no live trading for a long
  time. Real-data backtesting and strategy-checking ARE wanted — that is the
  channel for "what should the agent trade" questions.
- **ADR number reuse:** `0054` was used for the (now-removed) Mode::Live
  boundary ADR and lives in git history. The next NEW ADR should be **0055** to
  avoid colliding with that withdrawn number in history searches.
- Recovery: the full program is reachable at commit `edbbb10` (and the F1
  implementation at `dc3ef58`) if the decision is ever revisited.
