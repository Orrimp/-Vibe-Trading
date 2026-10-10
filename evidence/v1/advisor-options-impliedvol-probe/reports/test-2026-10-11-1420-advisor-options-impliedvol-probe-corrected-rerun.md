---
title: Test Report — corrected re-run
feature: advisor-options-impliedvol-probe
run_id: 2026-10-11-1420-UTC
commit: 0fec9a1c87036be11f1648e9c56c3b8ea58bc2c4
agent: orchestrator
verdict: PASS
supersedes_conclusion_of: test-2026-06-27-0610-advisor-options-impliedvol-probe.md
---

# Test Report — advisor-options-impliedvol-probe — corrected re-run — 2026-10-11 14:20 UTC

## 0. Why this report exists

The 2026-08-13 code review of story 3-15 returned **FAIL** and **withdrew the conclusion**
published on 2026-06-27. The code defects it found were all fixed in that same pass, but the
*number* was left withdrawn, and the CHANGELOG has said *"the conclusion awaits a corrected
re-run"* ever since. This is that re-run.

It also closes a second problem the review surfaced only implicitly. The withdrawn figures were
produced by two throwaway artifacts: the BTC half from a pre-assertion stdout capture of a test
that then failed on an unrelated stale count, and the ETH half from
`crates/backtest/tests/_tester_dvol_eth_bootstrap.rs` — **a tester-only file deleted in the same
pass that shipped the story**. Nothing in the repository could reproduce either number. A
published figure whose producer is not in the tree is not a measurement; it is a memory. Both
halves now have durable, named, declared homes:

| half | durable vehicle | mode |
|------|-----------------|------|
| BTCUSDT | `crates/backtest/tests/bakeoff_e2e.rs::bakeoff_full_wired_advisor::t7_1_full_wired_advisor_bakeoff_real_data` | `Bootstrap { paths: 1000 }` |
| ETHUSDT | `crates/backtest/tests/dvol_bakeoff_path_gate.rs::dvol_regime_decisive_eth_bootstrap_verdict` (**new, this pass**) | `Bootstrap { paths: 1000 }` |

The ETH vehicle is declared in that file's `CORPUS_GATED_TESTS`, so its skip is counted and
announced by `corpus_gated_tests_are_declared_and_counted` rather than vanishing silently.

## 1. Scope

- **Under test:** ADR-0072 `v0.dvol_regime` — the Deribit DVOL implied-vol bake-off arm, as
  corrected by the 11 patches landed under the 2026-08-13 review (including the structural fix
  that deleted the contradictory `weight`/`is_long` state field responsible for the warm-up
  holding **cash** instead of the coin).
- **Question:** does the corrected arm change the pre-registered verdict?
- **Commit:** `0fec9a1c87036be11f1648e9c56c3b8ea58bc2c4`
- **Toolchain:** rustc 1.94.1 (e408947bf 2026-03-25) · macOS 26.7 / arm64
- **Seeds (identical to the 2026-06-27 run, deliberately):** advisor seed
  `[0xC0, 0xFF, 0xEE, 0, …]`; bootstrap seed `0x0000000000eeffc0`.
- **Field (both coins):** `default_field() ∪ default_ensemble_field() ∪ default_macro_field()`
  → 19 candidates. `v0.macro_riskon` is correctly ABSENT (bug-log #81: structurally inert in
  any build that does not enable `backtest/yahoo`, which nothing does).

## 2. The control that makes this attributable

Both runs reproduce **every non-DVOL row of the 2026-06-27 tables exactly**. That is the
control: if the configuration had drifted, or the corpus had moved, or the engine had changed
under the story, these rows would have moved too. They did not. The only row that moved is the
row belonging to the arm that was fixed.

**BTCUSDT H1_2024 — every row the original report listed, then vs now:**

| strategy | Sharpe then | Sharpe now | Return% then | Return% now | MaxDD% then | MaxDD% now | Trades then | Trades now |
|----------|------------:|-----------:|-------------:|------------:|------------:|-----------:|------------:|-----------:|
| v0.buyhold | 1.486 | **1.486** | +47.78 | **+47.78** | 22.69 | **22.69** | 0 | **0** |
| v0.donchian_floor | 1.232 | **1.232** | +4.42 | **+4.42** | 3.59 | **3.59** | 1 | **1** |
| v0.8.vote.trend_pair | 1.075 | **1.075** | +1.07 | **+1.07** | 0.82 | **0.82** | 167 | **167** |
| v0.5.macd | 0.966 | **0.966** | +1.06 | **+1.06** | 0.86 | **0.86** | 189 | **189** |
| v0.8.vote.k2of4 | 0.495 | **0.495** | +0.62 | **+0.62** | 1.83 | **1.83** | 337 | **337** |
| v0.5.rsi | 0.478 | **0.478** | +0.38 | **+0.38** | 0.97 | **0.97** | 120 | **120** |
| **v0.dvol_regime** | **−0.190** | **+0.386** | **−0.30** | **+0.77** | **2.11** | **2.28** | **15** | **17** |

**ETHUSDT H1_2024 — same control:**

| strategy | Sharpe then | Sharpe now | Return% then | Return% now | MaxDD% then | MaxDD% now | Trades then | Trades now |
|----------|------------:|-----------:|-------------:|------------:|------------:|-----------:|------------:|-----------:|
| v0.buyhold | 1.297 | **1.297** | +49.78 | **+49.78** | 29.63 | **29.63** | 0 | **0** |
| v0.roc_momentum | 1.685 | **1.685** | +0.61 | **+0.61** | 0.10 | **0.10** | 4 | **4** |
| v0.5.macd | 1.611 | **1.611** | +2.41 | **+2.41** | 1.38 | **1.38** | 185 | **185** |
| v0.sma | 1.169 | **1.169** | +2.77 | **+2.77** | 2.39 | **2.39** | 99 | **99** |
| v0.donchian_floor | 1.114 | **1.114** | +4.70 | **+4.70** | 4.80 | **4.80** | 1 | **1** |
| **v0.dvol_regime** | **+0.397** | **+0.684** | **+0.76** | **+1.58** | **2.33** | **2.54** | **17** | **19** |

## 3. Full corrected leaderboards

### BTCUSDT H1_2024 — Bootstrap 1000 (T7.1, 29.05s)

| Rank | Strategy | Sharpe | Return% | MaxDD% | Trades | Flag |
|-----:|----------|-------:|--------:|-------:|-------:|------|
| 1 | v0.buyhold | 1.486 | +47.78 | 22.69 | 0 | Fragile **(CROWN)** |
| 2 | v0.donchian_floor | 1.232 | +4.42 | 3.59 | 1 | Fragile |
| 3 | v0.8.vote.trend_pair | 1.075 | +1.07 | 0.82 | 167 | Fragile |
| 4 | v0.5.macd | 0.966 | +1.06 | 0.86 | 189 | Fragile |
| 5 | v0.8.vote.k2of4 | 0.495 | +0.62 | 1.83 | 337 | Fragile |
| 6 | v0.5.rsi | 0.478 | +0.38 | 0.97 | 120 | Fragile |
| 7 | **v0.dvol_regime** | **0.386** | **+0.77** | **2.28** | **17** | **Fragile** |
| 8 | v0.8.vote.majority | 0.316 | +0.19 | 0.79 | 100 | Fragile |
| 9 | v0.8.vote.any1of4 | 0.041 | +0.09 | 3.09 | 237 | Fragile |
| 10 | v0.8.vote.tr_mr_macd_rsi | 0.000 | +0 | 0 | 0 | Fragile |
| 11 | v0.8.vote.unanimous | 0.000 | +0 | 0 | 0 | Fragile |
| 12 | v0.roc_momentum | 0.000 | +0 | 0 | 0 | Fragile |
| 13 | v0.sma | −0.038 | −0.07 | 3.57 | 115 | Fragile |
| 14 | v0.donchian_break | −1.083 | −1.06 | 1.76 | 523 | Fragile |
| 15 | v0.obv | −1.242 | −1.91 | 2.66 | 499 | Fragile |
| 16 | v0.5.bbands | −1.423 | −1.09 | 1.30 | 206 | Fragile |
| 17 | v0.8.vote.k3of4 | −1.431 | −0.31 | 0.48 | 32 | Fragile |
| 18 | v0.vol_breakout | −1.478 | −0.96 | 1.35 | 246 | Fragile |
| 19 | v0.8.vote.tr_mr_sma_bb | −2.761 | −1.22 | 1.30 | 110 | Fragile |

**Crowned:** `v0.buyhold` · **Outcome:** `BenchmarkWins` · 19/19 Fragile.

### ETHUSDT H1_2024 — Bootstrap 1000 (new ETH gate, 28.12s)

Return and MaxDD are emitted as **fractions** by this gate (`0.497776` = +49.78%); the
percentage column below is that fraction × 100, and the fraction is given so the figure is
citable without a unit guess.

| Rank | Strategy | Sharpe | Return (frac) | Return% | MaxDD (frac) | Trades | Flag |
|-----:|----------|-------:|--------------:|--------:|-------------:|-------:|------|
| 1 | v0.sma | 1.169 | 0.027737 | +2.77 | 0.023941 | 99 | Fragile |
| 2 | v0.5.macd | 1.611 | 0.024142 | +2.41 | 0.013762 | 185 | Fragile |
| 3 | v0.5.rsi | −0.592 | −0.005879 | −0.59 | 0.016486 | 102 | Fragile |
| 4 | v0.5.bbands | −0.026 | −0.000240 | −0.02 | 0.012666 | 214 | Fragile |
| 5 | v0.donchian_break | −2.890 | −0.038504 | −3.85 | 0.049551 | 540 | Fragile |
| 6 | v0.donchian_floor | 1.114 | 0.047041 | +4.70 | 0.048012 | 1 | Fragile |
| 7 | v0.vol_breakout | −2.579 | −0.019703 | −1.97 | 0.020976 | 220 | Fragile |
| 8 | v0.roc_momentum | 1.685 | 0.006063 | +0.61 | 0.000986 | 4 | Fragile |
| 9 | v0.obv | 0.060 | 0.001157 | +0.12 | 0.023068 | 469 | Fragile |
| 10 | **v0.dvol_regime** | **0.684** | **0.015784** | **+1.58** | **0.025440** | **19** | **Fragile** |
| 11 | v0.8.vote.majority | −0.311 | −0.002418 | −0.24 | 0.011872 | 80 | Fragile |
| 12 | v0.8.vote.unanimous | 0.000 | 0.000000 | +0.00 | 0.000000 | 0 | Fragile |
| 13 | v0.8.vote.trend_pair | 1.636 | 0.023617 | +2.36 | 0.012467 | 169 | Fragile |
| 14 | v0.8.vote.tr_mr_macd_rsi | 0.000 | 0.000000 | +0.00 | 0.000000 | 0 | Fragile |
| 15 | v0.8.vote.tr_mr_sma_bb | 1.425 | 0.006723 | +0.67 | 0.002740 | 88 | Fragile |
| 16 | v0.8.vote.any1of4 | 0.669 | 0.017412 | +1.74 | 0.029409 | 247 | Fragile |
| 17 | v0.8.vote.k2of4 | 1.358 | 0.022824 | +2.28 | 0.015731 | 325 | Fragile |
| 18 | v0.8.vote.k3of4 | 1.980 | 0.006320 | +0.63 | 0.001525 | 12 | Fragile |
| 19 | v0.buyhold | 1.297 | 0.497776 | +49.78 | 0.296278 | 0 | Fragile **(CROWN)** |

**Crowned:** `v0.buyhold` · **Outcome:** `BenchmarkWins` · 19/19 Fragile · 18 active arms faced
the benchmark.

## 4. Verdict

| Symbol | v0.dvol_regime Sharpe | Flag | Crowned | Outcome |
|--------|----------------------:|------|---------|---------|
| BTCUSDT H1_2024 | +0.386 | **Fragile** | v0.buyhold | BenchmarkWins |
| ETHUSDT H1_2024 | +0.684 | **Fragile** | v0.buyhold | BenchmarkWins |

**The 2026-06-27 conclusion is RESTORED, and the correction strengthens rather than weakens it.**

The review predicted the direction of the bias precisely: the warm-up-holds-cash defect *"biases
the measurement **toward** the pre-declared null"*, because on a window where BTC returned
+47.78% an arm sitting in cash is penalised for the wrong reason. That prediction is now
confirmed numerically — on both coins the corrected arm scores **higher** than the withdrawn
figure (BTC −0.190 → +0.386; ETH +0.397 → +0.684) and trades slightly more (15 → 17; 17 → 19).

The arm now gets a fair run on the strategy that was actually pre-registered, and it is **still
Fragile on both coins**, still ineligible to crown, and buy-and-hold is still crowned on both.
The null is the same null, reached on better evidence. A conclusion that survives the removal of
a bias that was pushing it in its own favour is worth more than the one that was withdrawn.

### What is NOT claimed

- **No alpha claim, in either direction.** This was pre-registered as an honest-coverage probe
  whose expected outcome was Fragile/null; that is what it returned. The deliverable is the
  coverage plus the reusable exogenous-series seam.
- **No cost-robustness claim.** The arm pays the same 4bps-a-leg as its siblings on this path,
  but this report does not re-derive the cost annex.
- **Scope is two coins × one 6-month window.** The over-scoping finding filed against the sibling
  story (3-16) applies to any attempt to read these two rows as a statement about the
  implied-vol channel in general. They are a statement about BTCUSDT and ETHUSDT on H1-2024.
- **No DSR certification.** Report-only, per the ratified decision; the frozen gate decides
  crown eligibility and it declined to crown this arm.

## 5. Standing floor

| Gate | Result |
|------|--------|
| `scripts/verify_anchors.sh` | **PASS 119/119** (before and after; `write_report=false` on both runs, no anchored body touched) |
| `scripts/spec_lint.py` | **PASS (0 violations)** |
| `cargo fmt -p backtest` | clean |
| `dvol_bakeoff_path_gate` (non-ignored) | 3/3 PASS; 4 corpus-gated skips declared and announced |
| `macro_regime_overlay_end_to_end` | 5/5 PASS |
| bug-log #78 absence guard | **PROBED RED** — disabling the `continue` at `crates/backtest/src/bakeoff/mod.rs:1280` resurfaced the arm at `trade_count=1, total_return%=0.0077`, confirming the guard is measured rather than merely present |

## 6. Anchor safety

Both runs use `write_report=false`. No anchored report body was created, modified or re-emitted.
This file is a **new** report alongside the 2026-06-27 one, which is left byte-untouched: it
remains the record of what was published and withdrawn, and overwriting it would destroy the
evidence that the withdrawal happened.
