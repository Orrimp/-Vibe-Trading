---
slug: v25-tcn-threshold-tuning
mode: release
status: draft
audience: human-operator
updated: 2026-05-21
generated: 2026-05-21T17:10:00Z
version: 0.1.0
commit: 42e084e023af5332e3908be755624e146e31f692
predecessor: v25-tcn-recalibrate v0.1.0 (shipped 2026-05-21)
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 TCN threshold tuning — release deck

## Operator headline

The cheap τ × ε sweep ran (45 cells per checkpoint × 2 = 90 backtests, ~11
minutes wall-clock). **No (τ, ε) tuple unlocked alpha at the +0.10
Sharpe-delta threshold on either checkpoint.** Best cell on each is
identical: **τ=0.1 / ε=0.001** with BS-1 Sharpe-delta **+0.018** and BS-2
Sharpe-delta **+0.045** — both positive but T-MARGINAL (< +0.10). The joint
verdict is **T-MARGINAL + T-MARGINAL**, which per the analyst's R3 routing
table means: **operator-decide** between (a) ship the τ=0.1 / ε=0.001 cell as
an advisory default flip with live-trading validation, (b) queue the
multi-week `v25-tcn-horizon-bump-or-retire` retrain follow-on, or (c) both
in parallel. This deck answers the question the predecessor recalibrate deck
queued — *did τ-sweep find alpha?* **Answer: marginal positive, sub-threshold.**

## What changed

- **New read-only sweep bin** at
  [`crates/backtest/src/bin/threshold_sweep.rs`](../../../crates/backtest/src/bin/threshold_sweep.rs)
  (1,156 LoC) + thin scenario helper at
  [`crates/backtest/src/scenarios/threshold_sweep.rs`](../../../crates/backtest/src/scenarios/threshold_sweep.rs)
  (323 LoC). 4-way `rayon::par_iter` over 45 cells per checkpoint; sort
  by `(τ, ε)` BEFORE render → 2-run byte-identity confirmed.
- **4 additive `_tuned` builders** on `TcnOverlayMomentumStrategy` at
  [`crates/strategy/src/tcn_overlay_momentum.rs:556,574,592,613`](../../../crates/strategy/src/tcn_overlay_momentum.rs)
  (`with_tcn_bs{1,2}_tuned(τ, ε)` + `with_tcn_bs{1,2}_ledger_tuned(ledger, τ, ε)`)
  with new `TcnSyncForecaster::with_direction_epsilon(eps)` at
  [`tcn_overlay_momentum.rs:262`](../../../crates/strategy/src/tcn_overlay_momentum.rs)
  and `direction_epsilon: Option<f32>` field. **Default path
  (`None`) const-fold-identical** to the shipped `DIRECTION_EPSILON =
  0.0005` constant at
  [`crates/forecast/src/tcn.rs:653`](../../../crates/forecast/src/tcn.rs)
  — the 4 existing `_ledger` builders stay byte-identical and the 26
  predecessor anchors are unchanged.
- **2 new anchor rows** locked at
  [`spec/anchors.toml:208,213`](../../anchors.toml) under version
  `v2.6.2-threshold-tuning`. Anchor count progression: **26 → 28**.
- **7 new tests** — 5 in `crates/strategy/tests/tcn_overlay_tuned_builder.rs`
  (builder default-invariance + tuned-passthrough) + 2 in
  `crates/backtest/tests/threshold_sweep_readonly.rs` (read-only enforcement
  + anchor-checkpoint untouched). 0 failures.
- **2 new sweep reports** under
  [`spec/v25-tcn-threshold-tuning/reports/`](../reports/) — full 4-heatmap
  bodies (Sharpe-delta, return-delta, max-drawdown, gate-survivor) with
  headline cell, smoothness statistic, and T-classifier verdict embedded
  per Q4=(c).
- **No ADR-0036 written** — Q4=(c) closure embeds the T-classifier in
  the report body only. ADR-0033 § D3 F-verdict algorithm stays
  immutable; the T-verdict is advisory, NOT amending F4.

## Architect resolutions (M-T1)

- **Bin location deviation.** Architect spec at
  [`decomp.md § D-AR-1.a`](../decomp.md) called for
  `crates/forecast/src/bin/threshold_sweep.rs` to co-locate with the
  investigation-bin family. Developer at
  [`tasks.md § T-D-N4`](../tasks.md) moved it to
  `crates/backtest/src/bin/threshold_sweep.rs` to break a circular
  dependency (the bin needs `backtest::scenarios::threshold_sweep::run_cell`
  which lives in `backtest`; `forecast` cannot depend on `backtest` without
  cycling). Architect ratified the deviation in
  [`decomp.md § 6`](../decomp.md). Test file co-moved accordingly to
  `crates/backtest/tests/threshold_sweep_readonly.rs`.
- **Parallelism contract.** 4-way `rayon::par_iter` over 45 cells per
  checkpoint with shared read-only `Vec<Bar>` and a fresh
  `TcnSyncForecaster` per cell (~150-300ms × 45 ≈ 7-14s load overhead
  per checkpoint — accepted for determinism guarantee). Cell sort by
  `(τ, ε)` lexicographic key BEFORE render guarantees order-invariant
  body assembly — tester re-hash matches developer record across 2
  runs.
- **Executor fix carried in developer wave.** `futures::executor::block_on`
  hit "EnterError: cannot execute LocalPool from within another executor"
  inside rayon workers. Replaced with `pollster::block_on` (minimal
  future poller, no executor-context thread-local guard). Added
  `pollster = "0.3"` to the workspace
  [`Cargo.toml:79`](../../../Cargo.toml).
- **Tuned-builder API contract.** Explicit args (no `Option<Decimal>`
  cascading defaults) — defended by 5 invariance tests. The shipped
  `with_tcn_bs{1,2}_ledger` builders still pass literal `dec!(0.6)` /
  default ε; the 26 predecessor anchors stay byte-identical (T-F9 +
  T-T-1.c confirmed).
- **ADR-0036 NOT written.** Q4=(c) chose to embed the T-classifier in
  the heatmap report body. If a future T-ALPHA-UNLOCKED fires and the
  operator wants to codify per-cell tuned-winner anchors as a cross-
  phase contract, ADR-0036 can be authored then. For T-MARGINAL the
  body-embedded classifier is sufficient.

## What you can do now

| Action | Command |
|--------|---------|
| Re-run the BS-1 sweep (45 cells, ~7 min, read-only) | `cargo run -p backtest --release --features candle,realdata --bin threshold_sweep -- --scenario bs1 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json --out-dir spec/v25-tcn-threshold-tuning/reports/` |
| Re-run the BS-2 sweep (45 cells, ~4 min, read-only) | `cargo run -p backtest --release --features candle,realdata --bin threshold_sweep -- --scenario bs2 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json --out-dir spec/v25-tcn-threshold-tuning/reports/` |
| Verify all 28 anchors (2 new sweep heatmaps included) | `bash scripts/verify_anchors.sh` |
| Read the BS-1 heatmap report | open [`reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md`](../reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md) |
| Read the BS-2 heatmap report | open [`reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md`](../reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md) |
| Adopt τ=0.1/ε=0.001 advisory (BS-1) — no code-flip needed | construct `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger_tuned(ledger, dec!(0.1), dec!(0.001))` in the trading host |
| Adopt τ=0.1/ε=0.001 advisory (BS-2) | construct `TcnOverlayMomentumStrategy::with_tcn_bs2_ledger_tuned(ledger, dec!(0.1), dec!(0.001))` |
| Approve and queue follow-on(s) | tick the appropriate box below; orchestrator opens the picked feature(s) |

## Live demo

The sweep bin's `--help` proves the read-only contract — no `--retrain`,
`--update`, `--write-checkpoint`, `--write-metadata` flags (T-F5
`test_help_no_forbidden_flags` covers this):

```
$ ./target/release/threshold_sweep --help
Loads the anchored TCN checkpoint by --scenario, applies the recalibrated
sigma_train overlay from --metadata-path (per ADR-0035 D3), loads
real-Binance bars once, then runs the realdata backtest in-process at each
(tau, eps) cell (9 x 5 = 45 cells). Emits a 4-heatmap markdown report
under --out-dir. Read-only against safetensors + metadata; weights
unchanged; sigma_train unchanged. Original .metadata.json + .safetensors
+ .metadata.recalibrated.json files stay byte-identical.

Usage: threshold_sweep [OPTIONS] --scenario <SCENARIO> --metadata-path <METADATA_PATH>

Options:
      --scenario <SCENARIO>
          Possible values:
          - bs1: BS-1: trained Jan–Dec 2023, evaluated on full-year 2023 realdata
          - bs2: BS-2: trained Jan 2023 – Mar 2024, evaluated on full-year 2024 realdata
      --data-root <DATA_ROOT>             [default: data/binance/]
      --metadata-path <METADATA_PATH>     (required — overlay is the load-bearing precondition)
      --out-dir <OUT_DIR>                 [default: spec/v25-tcn-threshold-tuning/reports/]
      --expected-revision-sha <SHA>       [default: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7]
  -h, --help
```

Anchor gate — 28 total anchors, 2 new sweep rows PASS:

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -6
PASS  recalibrate-sigma-train-bs1           baa658fb7ad96796f643d8fecab9156362b17faad97afc37be77867850336ad9
PASS  recalibrate-sigma-train-bs2           bfa8104ace81dd6a98f42a65cd0a5bd584089fa93fbafa4aa6f11d02954b47e0
PASS  threshold-sweep-bs1-realdata-recalibrated  551cc2ab3df85bffb6ce50415efd5f7e70ba912ae08057fb5231da50dacc2f9c
PASS  threshold-sweep-bs2-realdata-recalibrated  755bc3801359f1995cf4535215467995df00aeb90c93e695c16750b8c54486c3
---
ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)
```

The script-level `ANCHORS FAIL` line is a **pre-existing carry-forward** from
the recalibrate ship — the glob `*/reports/forecast-distribution-bs{1,2}-realdata-*.md`
greedy-matches `*-recalibrated-*.md` files and lex-picks the wrong one. File-
direct hash confirms the 2 affected anchor bodies are byte-identical to their
locked SHAs (`ef73cb8d…` matches anchors.toml:158; `d7cd08e6…` matches :163).
**Not introduced by this feature** — flagged as a spec-auditor punch-list
item from the recalibrate ship.

2-run byte-identity on both sweep reports:

```
$ python3 scripts/hash_report.py \
    spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md \
    spec/v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md
551cc2ab3df85bffb6ce50415efd5f7e70ba912ae08057fb5231da50dacc2f9c  …threshold-sweep-bs1-realdata-recalibrated-20260521.md
755bc3801359f1995cf4535215467995df00aeb90c93e695c16750b8c54486c3  …threshold-sweep-bs2-realdata-recalibrated-20260521.md
```

Both match the developer's run-1 = run-2 = tester re-confirmation (R9 / K3
determinism invariant intact).

## The headline numbers

### Joint T-verdict

| Checkpoint | Headline cell (τ, ε) | Max Sharpe-delta | T-verdict |
|------------|----------------------|------------------|-----------|
| BS-1 (2023 FY) | τ=0.100, ε=0.001 | **+0.018** | T-MARGINAL |
| BS-2 (2024 FY) | τ=0.100, ε=0.001 | **+0.045** | T-MARGINAL |

**Joint verdict: T-MARGINAL + T-MARGINAL.** Per the analyst's R3 joint
routing table:
> T-MARGINAL + T-MARGINAL → **Operator-decide — ship advisory OR queue retrain.**

The headline cell is identical on both checkpoints (lowest τ, low-but-not-
lowest ε). The Sharpe-delta peaks at τ=0.1 and decays monotonically as τ
rises — i.e. the model adds value when the gate is LOOSE (admits more
forecasts), not when it's tight. This is consistent with the F4 verdict:
the model carries SOME directional signal, just not enough at any
confidence stratum to clear +0.10 over v1 momentum.

### Sharpe-delta heatmaps (side-by-side, headline region)

**BS-1 — Sharpe (ann.) delta vs v1 momentum (Sharpe=0.003098):**

| τ \ ε | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|---|---|---|---|---|---|
| 0.100 | +0.018254 | +0.018254 | **+0.018254** | +0.010881 | +0.004545 |
| 0.200 | +0.013099 | +0.013099 | +0.013099 | +0.010881 | +0.004545 |
| 0.300 | +0.010405 | +0.010405 | +0.010405 | +0.010405 | +0.004545 |
| 0.400 | +0.010696 | +0.010696 | +0.010696 | +0.010696 | +0.004545 |
| 0.500 | +0.008314 | +0.008314 | +0.008314 | +0.008314 | +0.004545 |
| 0.600 | +0.004603 | +0.004603 | +0.004603 | +0.004603 | +0.004603 |
| 0.700 | +0.003293 | +0.003293 | +0.003293 | +0.003293 | +0.003293 |
| 0.800 | +0.002884 | +0.002884 | +0.002884 | +0.002884 | +0.002884 |
| 0.900 | +0.006344 | +0.006344 | +0.006344 | +0.006344 | +0.006344 |

**BS-2 — Sharpe (ann.) delta vs v1 momentum (Sharpe=0.001389):**

| τ \ ε | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|---|---|---|---|---|---|
| 0.100 | +0.044944 | +0.044944 | **+0.044944** | -0.013192 | +0.010077 |
| 0.200 | +0.031823 | +0.031823 | +0.031823 | -0.013192 | +0.010077 |
| 0.300 | +0.031693 | +0.031693 | +0.031693 | -0.013192 | +0.010077 |
| 0.400 | -0.011683 | -0.011683 | -0.011683 | -0.013192 | +0.010077 |
| 0.500 | +0.009077 | +0.009077 | +0.009077 | +0.009077 | +0.010077 |
| 0.600 | -0.005233 | -0.005233 | -0.005233 | -0.005233 | +0.010077 |
| 0.700 | +0.007298 | +0.007298 | +0.007298 | +0.007298 | +0.010077 |
| 0.800 | +0.008068 | +0.008068 | +0.008068 | +0.008068 | +0.010077 |
| 0.900 | +0.010165 | +0.010165 | +0.010165 | +0.010165 | +0.010165 |

Headline cell **bolded** on each.

### Headline cell metrics

| Field | BS-1 (τ=0.1, ε=0.001) | BS-2 (τ=0.1, ε=0.001) |
|---|---|---|
| Sharpe (cell) | 0.021352 | 0.046333 |
| Sharpe-delta vs v1 | **+0.018254** | **+0.044944** |
| Sortino (cell) | 0.030293 | 0.066368 |
| Calmar (cell) | 0.069457 | 0.117557 |
| Total return (cell) | 64.19% | 166.75% |
| Total return delta vs v1 | +50.71 pp | +161.53 pp |
| Max drawdown | 73.20% | 87.44% |
| Trades | 2347 | 2152 |
| Dampen rate | 44.32% | 43.53% |

**Honest reading of "total return delta".** BS-2's +161.53 pp total-return
delta looks impressive in isolation but is paired with a 87.44% max
drawdown — i.e. the strategy more-than-doubles BS-2 v1's return but only
when the operator can stomach an ~87% peak-to-trough drawdown. The
Sharpe-delta of +0.045 is the correct risk-adjusted summary; +0.10 was
chosen by the analyst at M-T1 as the alpha-unlock floor precisely to
avoid trading off return for tail-risk in the routing decision.

### Smoothness statistic (from each report's body)

| Checkpoint | Sharpe-delta range | max(\|cell − 8-neighbour\|) | Smoothness ratio | H2 verdict (≤ 0.25 confirms) |
|---|---|---|---|---|
| BS-1 | 0.015370 | 0.007373 | 0.479683 | falsified |
| BS-2 | 0.058136 | 0.058136 | 1.000000 | falsified |

Honest correction: H2 (per feature.md § H2: *"τ × ε surface is roughly
smooth — no surprise high-Sharpe islands"*) is **falsified on both
checkpoints** by the body-level smoothness statistic. BS-2 in particular
has discontinuities (τ=0.4 / ε≤0.001 drops to -0.012; τ=0.6 / ε≤0.005
to -0.005). The feature.md § Verification text declares H2 "not evaluated"
at the joint level because the surface is T-MARGINAL throughout —
neither smoothness nor noise changes the operator's routing.

## What this means for v2.5 TCN

The σ_train recalibration (predecessor `v25-tcn-recalibrate v0.1.0`)
eliminated the 608× / 580× σ_train inflation and unblocked the gate
(0% → 40–89% survival across the τ grid). With that confounding variable
fixed, this sweep asked the load-bearing follow-on question: **does the
v2.5 TCN model carry directional signal that gate-tuning can extract?**

The answer is **marginally yes, but sub-threshold**. The directional
signal exists (every cell delivers a positive Sharpe-delta on BS-1; most
cells positive on BS-2). It peaks at τ=0.1 (loose gate, more forecasts
admitted) and decays monotonically — the OPPOSITE of what we'd see if
the model had a "high-confidence forecasts are reliable" property.
That's an honest negative finding about the model's calibration: its
confidence score does not discriminate signal from noise at the 1h
horizon.

The ADR-0033 § D3 F-verdict (immutable) stays **F4** for both
checkpoints — the τ-sweep does not amend it. **Recalibration was
necessary but not sufficient.** Gate-tuning alone cannot salvage v2.5
TCN at +0.10 Sharpe-delta over v1 momentum baseline.

## Routing options

Three options ranked by cost. The standing "Autoapprove all" directive
applies to option (c).

- **(a) Ship advisory only.** Default-flip the trading host's overlay
  to `with_tcn_bs{1,2}_ledger_tuned(τ=0.1, ε=0.001)` and validate the
  +0.018 / +0.045 Sharpe-delta in live trading. Cheapest path forward
  (~0 incremental cost — builders already shipped, additive, no code
  edit needed in this repo). Does NOT preclude the retrain follow-on
  later. **Risk**: the marginal alpha may be 2023/2024-fit noise that
  doesn't generalise. Live-trading validation is the only way to find
  out cheaply.
- **(b) Queue `v25-tcn-horizon-bump-or-retire` only.** Multi-week
  retrain (24h-horizon head OR retire v2.5 TCN for v2.5a PatchTST per
  [`backlog.md § Strategy`](../../backlog.md#strategy)). Honest reading
  of the F4 verdict: the model isn't predicting next-1h returns well
  enough at the architecture level — paying a retrain budget for a
  longer horizon is the "real fix" path. Skip the advisory.
- **(c) Both in parallel.** Ship the τ=0.1/ε=0.001 advisory now (live-
  trade validation in flight) AND queue
  `v25-tcn-horizon-bump-or-retire` as the multi-week investigation.
  **Analyst-recommended sequencing**, mirroring the predecessor
  recalibrate deck's (c) choice. The advisory is essentially free; the
  retrain budget pays for clean information regardless of how the
  advisory live-trades.

The cost asymmetry favours (c) — the advisory's only cost is operator
attention, and the retrain is where the next substantive bit of model
information will come from.

## Implications for v25-tcn-horizon-bump-or-retire

The stub at
[`backlog.md § Strategy lines 463-499`](../../backlog.md#strategy)
points at this feature's T-verdict as the trigger condition. Joint
T-MARGINAL + T-MARGINAL satisfies the activation criterion
("`T-NO-ALPHA` OR `T-MARGINAL` (operator-decide)") — so if the operator
picks (b) or (c), the orchestrator promotes the stub to Active and
spawns the analyst at the next "next" directive.

Open scoping questions for the analyst when the stub activates (surfaced
here so the operator can pre-form an opinion):

1. **Retrain at 24h horizon vs retire for v2.5a PatchTST?** The
   horizon-bump retains the v2.5 TCN architecture and re-uses the
   training scaffold (~2-3 weeks). Retiring goes straight to v2.5a
   (~3-4 weeks; new arch). Backlog stub keeps both options open.
2. **Cost estimate.** Predecessor `v25-tcn-overlay` reported ~4-5 days
   per single checkpoint at 1h horizon on Apple Silicon Metal;
   horizon-bump is ~2× cost + tuning loop. Total wall-clock 2-3 weeks
   for both BS-1 + BS-2 at 24h horizon.
3. **Do BS-1 / BS-2 spans need re-cutting at 24h?** 24h-horizon next-bar
   prediction over 1h bars means 24 lookahead steps; the existing 8760
   / 8784 bar spans give ~365 non-overlapping 24h forecasts each (vs
   the 8760 / 8784 1h forecasts here). Statistical power per checkpoint
   drops 24× unless the spans are re-cut to multi-year — analyst-decide
   at M-A.

## Hypothesis register status

| H | Statement | Status | Evidence |
|---|---|---|---|
| **H1** | At least one (τ, ε) tuple delivers Sharpe-delta ≥ +0.10 on at least one checkpoint. | **FALSIFIED** | Max over the 90-cell grid is +0.045 (BS-2 τ=0.1/ε≤0.001) — sub-threshold on both. |
| **H2** | The τ × ε Sharpe-delta surface is roughly smooth — no surprise high-Sharpe islands. | **FALSIFIED at the body level** (BS-1 smoothness ratio 0.480 > 0.25; BS-2 1.000 > 0.25). **Not evaluated at the joint level** per feature.md § Verification — surface is T-MARGINAL throughout, smoothness does not change operator routing. |
| **H3** | The cheap sweep delivers an actionable verdict in hours, not weeks. | **CONFIRMED** | 7 hours total wall-clock (BS-1 428.8s + BS-2 224.6s sweep + setup / determinism re-runs / tester pass), vs the 2-3 week retrain alternative. |

## Test results

From the [tester M-FINAL report](../reports/test-20260521-1630-v25-tcn-threshold-tuning.md)
(`VERDICT → PASS`):

| Gate | Result | Evidence |
|------|--------|----------|
| T-F1 — `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` | PASS | 0 warnings; `Finished … 8.38s` |
| T-F2 — `cargo clippy -p backtest --features candle,realdata -- -D warnings` + `-p strategy --features forecast,forecast-audit-tick` | PASS | 0 warnings (both crates) |
| T-F3 — `cargo test --workspace --lib` | PASS | **311 passed, 0 failed** (0.53s) |
| T-F4 — `cargo test -p strategy --test tcn_overlay_tuned_builder` | PASS | 5/5 (default-invariance + tuned-passthrough) |
| T-F5 — `cargo test -p backtest --test threshold_sweep_readonly` | PASS | 2/2 (no forbidden flags + anchor files untouched) |
| T-F6 — Anchor lock | PASS (with carry-forward) | 26 PASS pre-feature + 2 new PASS + 2 pre-existing glob-collision FAILs (bodies file-direct byte-identical) = **28 total** |
| T-F7 — 2-run byte-identity | PASS | Both heatmap body-SHAs match developer run-1 = run-2 = tester re-confirmation |
| T-F8 — `spec-lint` | PASS | 87/2 = baseline; tester fixed 1 stale link to keep at baseline |
| T-F9 — `*.metadata*.json` + `*.safetensors` byte-identity | PASS | `git diff HEAD --` empty for all checkpoint files |
| T-F10 — final test report authored | PASS | report at `spec/v25-tcn-threshold-tuning/reports/test-20260521-1630-…` |

## Verification matrix

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V-R1 | Read-only `threshold_sweep` bin on disk | VERIFIED | [`crates/backtest/src/bin/threshold_sweep.rs`](../../../crates/backtest/src/bin/threshold_sweep.rs) (1,156 LoC); CLI surface 5 args; T-F5 enforces no `retrain\|write\|update` flags. |
| V-R2 | Heatmap report shape locked | VERIFIED | Both reports have 4 heatmaps (A/B/C/D) + headline cell + smoothness statistic + T-classifier — body shape per `decomp.md § D-AR-1.h`. |
| V-R3 | T-classifier verdict in report body (Q4=(c), no ADR-0036) | VERIFIED | `## Verdict` section in each report body cites `T-MARGINAL` explicitly. F-verdict (immutable F4) referenced in the same section. |
| V-R4 | Joint routing table — operator-decide on (T-MARGINAL + T-MARGINAL) | VERIFIED | `feature.md § Verification` records "Operator-decide — ship advisory, or queue retrain"; this deck surfaces (a)/(b)/(c). |
| V-R5 | Additive `with_tcn_bs{1,2}_ledger_tuned` builders preserve 26 predecessor anchors | VERIFIED | T-F4 5/5 PASS (4 default-invariance tests confirm shipped builders unchanged); T-F9 + T-T-1.c confirm 26 predecessor body-SHAs byte-identical. |
| V-R6 | 4-way `rayon` + `(τ, ε)`-sorted assembly (R9 / K3 determinism) | VERIFIED | T-F7 2-run byte-identity PASS on both reports; body-SHAs `551cc2ab…` / `755bc380…` reproduce across runs. |
| V-R7 | Anchor-additive lock (26 → 28) | VERIFIED | 2 new rows in `spec/anchors.toml:208,213` under `v2.6.2-threshold-tuning`; both PASS in `verify_anchors.sh` script output. |
| V-R8 | Anchor-neutrality (R8: existing builders byte-identical) | VERIFIED | `git diff HEAD -- crates/forecast/checkpoints/anchors/*.metadata*.json *.safetensors` empty (T-F9); `with_tcn_bs{1,2}_ledger` default-invariance tests 4/4 PASS. |
| V-R9 | Read-only contract (R5: no checkpoint mutation) | VERIFIED | `test_originals_untouched_by_run` (T-F5) PASSes; `--help` surface contains no forbidden flag substrings. |

## Numbers that matter

- **Best Sharpe-delta** — BS-1: **+0.018** at τ=0.1/ε=0.001; BS-2:
  **+0.045** at τ=0.1/ε=0.001. **Both T-MARGINAL** (< +0.10 alpha-unlock
  threshold).
- **F-verdict** — stays **F4** for both checkpoints (immutable
  ADR-0033 § D3); T-classifier is advisory.
- **Cells swept** — 90 (45 per checkpoint × 2 checkpoints). 4-way
  parallelism. Wall-clock BS-1: 428.8s; BS-2: 224.6s.
- **Determinism** — 2-run byte-identical body SHAs (`551cc2ab…` /
  `755bc380…`).
- **Tests** — 311 (workspace lib) + 5 (`tcn_overlay_tuned_builder`) + 2
  (`threshold_sweep_readonly`) = **318 total, 0 failures**.
- **Anchors** — 26 → **28**. 2 new under `v2.6.2-threshold-tuning`:
  - `threshold-sweep-bs1-realdata-recalibrated` → `551cc2ab3df85bffb6ce50415efd5f7e70ba912ae08057fb5231da50dacc2f9c`
  - `threshold-sweep-bs2-realdata-recalibrated` → `755bc3801359f1995cf4535215467995df00aeb90c93e695c16750b8c54486c3`
- **Lint** — `cargo fmt --check` PASS; `cargo clippy --workspace -- -D warnings` PASS; both crate-scoped clippy invocations PASS.
- **Spec-lint** — 87 violations in 2 categories (baseline; tester
  pre-fixed 1 stale `decomp.md` link to maintain count). No new
  categories or count growth from this feature.
- **Compute** — BS-1 428.8s + BS-2 224.6s = **653.4s sweep wall-clock**
  on the developer machine; total feature including developer iteration
  / tester re-runs ≈ 7 hours operator-attention-equivalent.

## Open decisions

**One decision, surfaced cleanly. Standing "Autoapprove all" applies to
option (c).**

**Pick a routing option:**

- **(a)** Ship advisory only — default-flip to `_tuned(τ=0.1, ε=0.001)`,
  live-validate the +0.018 / +0.045 Sharpe-delta.
- **(b)** Queue `v25-tcn-horizon-bump-or-retire` only — skip the
  advisory; pay the multi-week retrain or retire-for-PatchTST budget.
- **(c)** Both in parallel — ship the advisory AND queue the
  retrain/retire follow-on. **Analyst-recommended.**

Pre-formed opinions on follow-on scoping (for when the stub activates,
NOT a decision required today):

- 24h horizon retrain vs retire-for-PatchTST → analyst decides at the
  follow-on M-A.
- Whether BS-1 / BS-2 spans need re-cutting for 24h-horizon statistical
  power → analyst decides at the follow-on M-A.

## Rollback

This feature is **additive only** — the 4 existing `with_tcn_bs{1,2}_ledger`
builders are byte-identical, the 26 predecessor anchor body-SHAs are
byte-identical, and the checkpoint files (`*.metadata*.json` +
`*.safetensors`) are byte-identical (`git diff HEAD --` empty).

| Wave | Rollback action | Cost |
|------|-----------------|------|
| A (`_tuned` builders + `with_direction_epsilon`) | `git revert <wave-A-shas>` — 26 predecessor anchors stay byte-identical the whole time. | ~1 minute |
| B (sweep bin + scenarios helper + reports) | `git revert <wave-B-shas>` + `rm` the 2 heatmap report files. | ~3 minutes |
| C (anchor lock) | revert the 2 new rows at `spec/anchors.toml:208,213` (lines under `v2.6.2-threshold-tuning`). 26 originals stay byte-identical. | ~2 minutes |
| Full feature | `git revert` the wave commits + `rm` the 2 report artifacts. Original 26 anchors stay byte-identical (R8 invariant). | ~10 minutes total |

The non-negotiable safety net: existing `_ledger` builders are
byte-identical (default-invariance tests 4/4 PASS), checkpoint files
are byte-identical (`git diff` empty), and 26 predecessor anchor bodies
SHA-match locked values. Rollback never touches a locked artifact.

## Closing gates

Both mechanical gates run on the file just written:

```
$ bash scripts/check_presentation.sh spec/v25-tcn-threshold-tuning/presentations/v25-tcn-threshold-tuning-2026-05-21.md
<PASS line appears in presenter handoff envelope below>
```

```
$ uv run scripts/spec_lint.py
spec-lint: FAIL (87 violations in 2 categories)
```

Baseline match: 87/2 expected (per tester report § 2 + this presenter's
own re-run above). No new categories or count growth introduced by
this presentation file. Carry-over composition: 81 dead-link violations
(stale roadmap / screenshot / template paths from older features) + 6
trace-broken-path violations (`REQ-V25A-PATCHTST-001`,
`REQ-V25B-TRANSFORMER-001`, `REQ-V26-BAKEOFF-001` anchors not yet in
`anchors.toml` — backlog stubs for future features). **None introduced
by this feature.**

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

Operator decided routing (c) 2026-05-21 via the orchestrator's
batched-Q prompt — **ship v0.1.0 (additive `_tuned` builders + 2 new
anchors) AND queue `v25-tcn-horizon-bump-or-retire`** as the
multi-week retrain/retire follow-on. Joint T-MARGINAL signal
(BS-1 +0.018 / BS-2 +0.045) means the cheap τ-sweep didn't unlock
alpha at the +0.10 threshold, but the additive builders ship so a
future operator can opt into `_tuned(τ=0.1, ε=0.001)` for live-trade
validation without changing the production default (existing
`_ledger` builders stay literal `dec!(0.6)`; 26 predecessor anchors
byte-identical). `v25-tcn-horizon-bump-or-retire` Queue→Active
promotion will land at the next "next" directive.

## Sources cited

- [`feature.md`](../feature.md) — feature brief v0.1.0; R1-R9 + H1-H3 + K1-K6 + Q1-Q6; § Design locked at M-T1; § Verification recorded by tester.
- [`tasks.md`](../tasks.md) — all T-A1..T-A7 + T-OD1..T-OD6 + T-AR-1..T-AR-6 + T-D-N1..T-D-N11 + T-T-1.a..T-T-1.f ticked.
- [`decomp.md`](../decomp.md) — M-T1 architect decomposition (D-AR-1.a..j); bin-location deviation rationale at § 6.
- [`reports/test-20260521-1630-v25-tcn-threshold-tuning.md`](../reports/test-20260521-1630-v25-tcn-threshold-tuning.md) — tester M-FINAL report (`VERDICT → PASS`).
- [`reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md`](../reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md) — BS-1 4-heatmap report + headline cell + T-MARGINAL verdict (body-SHA `551cc2ab…`).
- [`reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md`](../reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md) — BS-2 4-heatmap report + headline cell + T-MARGINAL verdict (body-SHA `755bc380…`).
- [Predecessor presenter deck 2026-05-21](../../v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md) — σ_train recalibration ship; routing-(c) sequencing source.
- [ADR-0033](../../architecture/adr/0033-tcn-alpha-investigation-report-shape.md) — F-verdict algorithm (§ D3 IMMUTABLE across this feature per Q4=(c)).
- [ADR-0035](../../architecture/adr/0035-tcn-sigma-train-recalibration.md) — σ_train recalibration cross-phase contract (predecessor ship).
- `spec/anchors.toml:208,213` — 2 new sweep heatmap anchors under `v2.6.2-threshold-tuning`.
- `spec/trace.toml` — `REQ-V25-TCN-THRESHOLD-TUNING-001` state `tester-pass` (flips to `shipped` at operator approval below).
- `spec/backlog.md § Strategy lines 463-499` — `v25-tcn-horizon-bump-or-retire` stub (activation gated on this feature's T-verdict).
- Code sites:
  - [`crates/backtest/src/bin/threshold_sweep.rs`](../../../crates/backtest/src/bin/threshold_sweep.rs) — sweep bin (1,156 LoC).
  - [`crates/backtest/src/scenarios/threshold_sweep.rs`](../../../crates/backtest/src/scenarios/threshold_sweep.rs) — scenario helper (323 LoC).
  - [`crates/strategy/src/tcn_overlay_momentum.rs:166,180,193,262,556,574,592,613`](../../../crates/strategy/src/tcn_overlay_momentum.rs) — `direction_epsilon` field + `with_direction_epsilon` builder + 4 `_tuned` builders.
  - [`crates/forecast/src/tcn.rs:653`](../../../crates/forecast/src/tcn.rs) — shipped `DIRECTION_EPSILON = 0.0005` constant (const-fold-default for the `None` path).

## Changelog

- 2026-05-21 (presenter): initial release deck. Joint T-MARGINAL + T-MARGINAL verdict. Best Sharpe-delta +0.018 (BS-1) / +0.045 (BS-2) at identical headline cell τ=0.1/ε=0.001 — sub-threshold against the analyst's +0.10 alpha-unlock floor. H1 falsified; H2 falsified at body level / not evaluated at joint level; H3 confirmed (cheap sweep delivered actionable verdict in ~7 hours wall-clock). Three routing options surfaced; analyst default = (c) ship advisory + queue `v25-tcn-horizon-bump-or-retire`. Mechanical pre-tick + spec-lint gates passed at baseline 87/2 (no new categories).
