---
date: 2026-09-26
author: sub-agent (independent cross-check derivation)
slug: relock-row-mapping-derivation-2026-09-26
status: DERIVATION COMPLETE — 8 CERTAIN / 6 PROBABLE / 1 CANNOT DECIDE
scope: story 1-27 ADR-0038 § D6.b re-lock — which anchors.toml row each measured body replaces
---

# Row mapping for the 1-27 D6.b re-lock — independent derivation

15 scenarios were measured producing a body-SHA under the default invocation. Each of them is
anchored 1–3 times in [`evidence/anchors.toml`](../../evidence/anchors.toml) under different
namespaces. This note derives, per scenario, **which single row the produced body replaces**, and
says how that was established.

This is an independent derivation. It was written without reading any other derivation of the same
mapping. Where it lands on the same answer as an existing bug-log entry (`#122`), the route is given
so the agreement can be checked rather than taken on trust.

---

## 0. The one thing everything turns on

The three namespaces are **three CLI friction conditions**, and nothing else. Established from the
frozen evidence's own front-matter:

| namespace | invocation | source |
|---|---|---|
| `… + noop-baseline` | no `--sim-*` flags — zero sim slippage | `evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/sharpe-delta-2026-05-29.md:17` — *"Noop baseline = pre-v5 zero-sim-slippage report (PaperEngine spread still applies)"* |
| `… + v5-realdata-medium-2026-05` | `--sim-latency-ms-min 30 --sim-latency-ms-max 80 --sim-slippage-bps 8` | `evidence/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/sharpe-delta-table-2026-05-28.md:5` (`canonical_config:`) |
| `v5-sqrt-impact-2026-05` | `--sim-slippage-sqrt-alpha 1.0 --sim-slippage-sqrt-lookback-days 90` | `evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/sharpe-delta-2026-05-29.md:6-8` (`namespace_sqrt`, `alpha`, `volume_lookback_days`) |

Four dimensions that could otherwise confound the comparison were each closed:

1. **Latency is body-neutral by construction, not by luck.** `crates/backtest/src/main.rs` threads
   `args.sim_latency_ms_min/max` into `LatencySlippageSimConfig` at 12 sites (`:1645`, `:1720`,
   `:1792`, `:1814`, `:1922`, `:1943`, `:2047`, `:2068`, `:2170`, `:2191`, `:2288`, `:2404`) — and
   **no scenario path ever reads the field.** The only consumer of a latency value in the repo is
   `crates/exec/src/latency.rs::apply_latency`, whose sole callers are
   `crates/exec/benches/latency_slippage.rs` and its own unit tests (verified by grep over
   `crates/**/*.rs`). So the `30..=80 ms` half of the canonical config cannot move a body, and the
   friction dimension collapses to `slippage_model` alone.
2. **Data revision unchanged.** `data/binance/REVISION.toml` still reads
   `sha256 = "3a8b96c4…bfc7"`, byte-equal to the `data_revision_sha:` front-matter of every anchored
   `-realdata` body across all three generations. Last touched at `ae9b415a`.
3. **The candle feature is config-driven, so it is constant across a scenario's generations.**
   `crates/backtest/src/scenarios/tcn_overlay.rs:397` hard-codes
   `forecaster_label: "passthrough (no-candle mode — degrades to v1 momentum)"` in the *non-weights*
   runner. Consequence, checked on disk: `top10-2023-fy-tcn-overlay-realdata`'s **v0.4.0** body
   prints "passthrough (no-candle mode)" even though the v0.4.0 binary was built
   `--features "candle realdata"` (that dir's table, § Group F/G). Every generation of every one of
   the 15 prints the *same* `Forecaster:` line — so candle cannot distinguish two generations of one
   scenario, and cannot be a drift source either.
4. **Build profile measured-neutral.** `crates/backtest/tests/determinism.rs:1097-1105` records 4
   scenarios × 8 runs producing byte-identical SHAs debug-vs-release, corroborated by the anchored
   `wall_clock_s: 3.2` matching the release run (bug-log `#121`).

### The rule this yields

> **The target row is the row whose namespace-condition equals the `slippage_model` that
> `build_slippage_model_for_scenario` returns for that scenario under default flags.**

`crates/backtest/src/main.rs:195-221`, with the CLI defaults at `:111-132`
(`sim_latency_ms_min=0`, `sim_latency_ms_max=0`, `sim_slippage_bps=0`, `sim_slippage_sqrt_alpha=0`):

- **Real-data scenario** (one of the 9 ids at `main.rs:43-59`) → `build_slippage_model(args)`
  (`:168-181`) → `sqrt_alpha == 0` → **`Linear { bps: 0 }`**.
  `crates/backtest/src/scenarios/sim.rs:64` returns `Decimal::ZERO` for that arm, documented at
  `:40-42` as *"byte-identical to the pre-feature code path"*; `cli_types.rs:104-115` says the same
  of the noop default. **⇒ the `+ noop-baseline` condition.**
- **Synthetic scenario** (anything else, or `realdata` not compiled in) → Q-D1=(a) fallback at
  `main.rs:214-220` → **`Linear { bps: 8 }` regardless of CLI flags**.
  **⇒ the `+ v5-realdata-medium-2026-05` condition.**

`venue_filter` is `None` at every CLI scenario arm (`cli_types.rs:130-137` keeps `None` as
`Default` precisely so anchored bodies don't move), and `VolScaledSpread` is never selected in the
CLI path (grep: only `sim.rs:97` handles the arm, nothing constructs it).

### Two positive controls, both measured, one on each side of the split

- **Synthetic side.** bug-log `#111`'s bisect (`docs/dev-notes/bug-log.md:1984-1987`): at
  `b3332d35` the default invocation of `top10-2023-1h-momentum` produced `0f6f6eb8…`, the
  `v1 + v5-realdata-medium-2026-05` row, *"reproduces the pin exactly"*. Cross-checked from the
  evidence side: `#111`'s "pinned (contaminated)" final equity `$50 922.49`
  (`bug-log.md:1996`) is exactly the **Canon Equity** cell for that scenario in the v0.4.0 table
  (`sharpe-delta-table-2026-05-28.md`, § Group B). The default run was hitting the canonical row.
- **Real-data side.** bug-log `#120` (`bug-log.md:2520-2545`): at HEAD the default invocation of
  `top10-{2023,2024}-fy-regime-dispatcher-realdata` reproduces `f37bbb8d…` / `691a7056…` **exactly**
  — the `v3.0.0-regime` rows, which are that scenario's pre-v5 / no-`--sim-*`-flags generation
  (their bodies live in `evidence/v1/v3-regime-classifier/reports/`, outside every v5 dir, so they
  resolve through `verify_anchors.sh`'s legacy-default branch at `:207-222`). A zero-bps run
  reproducing them proves they *are* zero-bps bodies. Same helper
  (`assert_reproduces_or_report_unmeasured` → `run_realdata_scenario_once`, which passes only
  `--scenario`, `--seed`, `--reports-dir` — `determinism.rs:1059-1062`) as five of the seven
  `-realdata` scenarios below.

---

## 1. The mapping

`produced` = the SHA measured under the default invocation. `target row` = the row whose `sha256`
this body replaces, with that row's **current** value.

| # | scenario | produced sha | target row (namespace) | target row's current sha | anchors.toml line | how established | confidence |
|---|---|---|---|---|---|---|---|
| 1 | `top10-2023-1h-momentum` | `b655e5e7…` | `v1 + v5-realdata-medium-2026-05` | `0f6f6eb8d943fefa…` | `:364` (sha `:366`) | synthetic ⇒ Q-D1=(a) `Linear{bps:8}` (`main.rs:214-220`); **machine-enforced** by `scripts/check_determinism_anchors.py:21-24` (every non-cfg-gated `const ANCHOR` must equal the `v5-realdata-medium-2026-05` row) — the pin at `determinism.rs:770` is already that row and the linter is green; bisect control above | **CERTAIN** |
| 2 | `top10-2024-h1-momentum` | `37ce69e9…` | `v1 + v5-realdata-medium-2026-05` | `78976062cf3d62b9…` | `:369` (`:371`) | as #1; pin `determinism.rs:786` | **CERTAIN** |
| 3 | `top10-2023-fy-tcn-overlay` | `64f51802…` | `v2.5.0 + v5-realdata-medium-2026-05` | `1460fcc70029746b…` | `:394` (`:396`) | as #1; pin `determinism.rs:813`. Resolver-confirmed: this sha lives *only* in `evidence/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/`, the 8-bps re-emit dir | **CERTAIN** |
| 4 | `top10-2024-fy-tcn-overlay` | `908b66e1…` | `v2.5.0 + v5-realdata-medium-2026-05` | `b8e9186bb36abe65…` | `:399` (`:401`) | as #3; pin `determinism.rs:829` | **CERTAIN** |
| 5 | `pairs-2023-zscore-mr` | `ac647a59…` | `v1.5a + v5-realdata-medium-2026-05` | `01c9da4d4c5ce268…` | `:374` (`:376`) | synthetic ⇒ `Linear{bps:8}`; existing R-REPRO pin `multi_pair_determinism.rs:141` is already this row, with the reasoning stated at `:123-126`; resolver-confirmed (sha only in the v0.3.0 dir, while the noop sha `90591a0e…` sits in both `evidence/v1/v15a-mean-reversion-pairs/` **and** the v0.2.0 dir — i.e. the v0.2.0 canonical run was a no-op for this path, wiring landed at v0.3.0) | **CERTAIN** |
| 6 | `pairs-2024-h1-zscore-mr` | `5bee5e9c…` | `v1.5a + v5-realdata-medium-2026-05` | `6252819b4f719ce4…` | `:379` (`:381`) | as #5; pin `multi_pair_determinism.rs:149` | **CERTAIN** |
| 7 | `top10-2023-fy-tcn-overlay-weights` | `175173b6…` | `v2.5.0-tcn-weights + v5-realdata-medium-2026-05` | `28379df8913e987b…` | `:404` (`:406`) | synthetic (`Data source: synthetic (seeded RNG, v2.5 tcn-overlay-weights)`) ⇒ `Linear{bps:8}`; **directory decides the generation**: `28379df8…` exists only in `evidence/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/`, `7cb1357c…` in `evidence/v1/v25-tcn-overlay/reports/` (×5) + the v0.2.0 dir. The gate builds `--features candle` (`determinism.rs:861,870-873`) so it must pin the candle re-emit. Current pin `determinism.rs:931` = `7cb1357c…` = **the noop row — WRONG**. Independently reaches bug-log `#122`'s conclusion | **CERTAIN** |
| 8 | `top10-2024-fy-tcn-overlay-weights` | `3c1178fb…` | `v2.5.0-tcn-weights + v5-realdata-medium-2026-05` | `0c13ed0bd5e7d4e5…` | `:409` (`:411`) | as #7; current pin `determinism.rs:949` = `23c24dae…` = **the noop row — WRONG** | **CERTAIN** |
| 9 | `top10-2023-fy-tcn-overlay-realdata` | `b6d88fa6…` | `v2.6.0-realdata + noop-baseline` | `8fa47f49e887df48…` | `:149` (`:151`) | real-data id (`main.rs:47`) ⇒ `Linear{bps:0}` ⇒ zero-sim-slippage = the noop condition; `#120` control. Directory corroboration: noop `8fa47f49…` sits in `evidence/v1/backtest-real-binance-data/` (×9) **and** the v0.2.0 dir (path not yet wired), canonical `10fd4502…` only in v0.4.0, sqrt `1157af76…` only in v0.5.0 — equity `$113 479.98 → $77 001.73 → $10 105.53`, three cleanly separated conditions. Current pin `determinism.rs:1500` = the **sqrt** row — wrong condition | **PROBABLE** |
| 10 | `top10-2024-fy-tcn-overlay-realdata` | `b5efe7b6…` | `v2.6.0-realdata + noop-baseline` | `fd8191dff1ca106c…` | `:154` (`:156`) | as #9; pin `determinism.rs:1509` = sqrt row — wrong condition | **PROBABLE** |
| 11 | `top10-2023-fy-tcn-overlay-weights-realdata` | `fa09a769…` | `v2.6.0-realdata + noop-baseline` | `552d7df294bc93ff…` | `:159` (`:161`) | as #9 (`main.rs:50`). Extra: this scenario **cannot run without candle** (bug-log `#111`), so its 2026-05-18 noop body was necessarily emitted by a candle build — the feature dimension matches today's run exactly. Pin `determinism.rs:1518-1523` = sqrt row — wrong condition | **PROBABLE** |
| 12 | `top10-2024-fy-tcn-overlay-weights-realdata` | `0d6cc994…` | `v2.6.0-realdata + noop-baseline` | `2a65c4347964a074…` | `:164` (`:166`) | as #11; pin `determinism.rs:1531-1536` = sqrt row — wrong condition | **PROBABLE** |
| 13 | `top10-2023-fy-momentum-realdata` | `0fc591e5…` | **no row exists for the produced condition** — see § 2 | (only row: `v5-sqrt-impact-2026-05` = `0867d232b5d4e381…`) | `:593` (`:595`) | real-data id (`main.rs:45`) ⇒ `Linear{bps:0}` ⇒ noop condition, but this scenario has **exactly one** anchored row and it is the sqrt one (grep: `momentum-realdata` appears at `anchors.toml:580` in a comment and `:593` as the only row). Its noop body (`70b8d442…`, `$113 479.98`) and its 8-bps body (`ed5717f8…`, `$77 001.73`) are both on disk and **neither is anchored** | **CANNOT DECIDE** |
| 14 | `top10-2023-fy-patchtst-overlay-realdata` | `f704c4f2…` | `v2.5a.0-patchtst + noop-baseline` | `5f303cc0812d421e…` | `:253` (`:255`) | as #9 (`main.rs:53`). Directory: noop `5f303cc0…` in `evidence/v1/v25a-patchtst-overlay/` + v0.2.0; canonical `55c5b715…` only v0.4.0; sqrt `b015b564…` only v0.5.0. Pin `determinism.rs:1632` = sqrt row — wrong condition | **PROBABLE** |
| 15 | `top10-2023-fy-vol-target-overlay-realdata` | `91848e23…` (16 hex) | `v3.0.0-volatility + noop-baseline` | `9fa64d467f357979…` | `:283` (`:285`) | as #9 (`main.rs:55`). Note the noop resolver picks the **newest** body outside the v5 dirs: `evidence/v1/v3-volatility-forecaster/reports/` holds both the pre-noop-fix `66cd69ad…` (08:29) and the post-fix `9fa64d46…` (12:33); `tail -1` lands on the latter, which is the anchored value. Pin `determinism.rs:1669` = sqrt row — wrong condition | **PROBABLE** |

Compact list:

```
top10-2023-1h-momentum                      -> v1 + v5-realdata-medium-2026-05            (0f6f6eb8)
top10-2024-h1-momentum                      -> v1 + v5-realdata-medium-2026-05            (78976062)
top10-2023-fy-tcn-overlay                   -> v2.5.0 + v5-realdata-medium-2026-05        (1460fcc7)
top10-2024-fy-tcn-overlay                   -> v2.5.0 + v5-realdata-medium-2026-05        (b8e9186b)
pairs-2023-zscore-mr                        -> v1.5a + v5-realdata-medium-2026-05         (01c9da4d)
pairs-2024-h1-zscore-mr                     -> v1.5a + v5-realdata-medium-2026-05         (6252819b)
top10-2023-fy-tcn-overlay-weights           -> v2.5.0-tcn-weights + v5-realdata-medium…   (28379df8)
top10-2024-fy-tcn-overlay-weights           -> v2.5.0-tcn-weights + v5-realdata-medium…   (0c13ed0b)
top10-2023-fy-tcn-overlay-realdata          -> v2.6.0-realdata + noop-baseline            (8fa47f49)
top10-2024-fy-tcn-overlay-realdata          -> v2.6.0-realdata + noop-baseline            (fd8191df)
top10-2023-fy-tcn-overlay-weights-realdata  -> v2.6.0-realdata + noop-baseline            (552d7df2)
top10-2024-fy-tcn-overlay-weights-realdata  -> v2.6.0-realdata + noop-baseline            (2a65c434)
top10-2023-fy-momentum-realdata             -> CANNOT DECIDE (no row for this condition)
top10-2023-fy-patchtst-overlay-realdata     -> v2.5a.0-patchtst + noop-baseline           (5f303cc0)
top10-2023-fy-vol-target-overlay-realdata   -> v3.0.0-volatility + noop-baseline          (9fa64d46)
```

**8 CERTAIN · 6 PROBABLE · 1 CANNOT DECIDE.**

### Why the six real-data rows are PROBABLE and not CERTAIN

The *rule* is certain — `Linear{bps:0}` is forced by `main.rs:195-221` with no escape, and `#120`
is a positive control taken with the same helper on the same family. What is not certain is that no
unexamined per-scenario dimension breaks it, and **the body cannot be asked**: every generation of
every one of these scenarios prints the identical `- Slippage: 2 bps, Taker fee: 4 bps` line, so
there is no friction line to read back (this is the `#112`-requirement-2 gap for the whole
`-realdata` family, and the same observation `#122` makes for the m3 pair). The upgrade to CERTAIN
is in § 2.

---

## 2. CANNOT DECIDE, and what would settle it

### `top10-2023-fy-momentum-realdata` — the produced condition has no row

The default invocation produces the **zero-sim-slippage** body. This scenario's only anchor row is
`v5-sqrt-impact-2026-05` (`anchors.toml:593-595`). Writing the produced SHA there would leave a row
labelled *square-root market impact, α = 1.0* holding a body produced with no market-impact model at
all — a mislabel of exactly the kind `#112` requirement (2) exists to prevent, and it would make the
row's own comment block (`anchors.toml:583`, *"Slippage model: SquareRoot { alpha=1.0,
volume_lookback_days=90 }"*) false.

The three options, and none of them is a measurement:

- **(a) Re-run under the sqrt invocation and land *that* body.** `--sim-slippage-sqrt-alpha 1.0
  --sim-slippage-sqrt-lookback-days 90`, from the workspace root with `--features realdata`. This is
  the only option that keeps the row honest, and `0fc591e5…` is then **not** the body to land. This
  is my recommendation, and it generalises — see § 3.
- **(b) Add a `v3.0.0-volatility-rebaseline + noop-baseline` row** for the produced body. Blocked as
  written: ADR-0038 § D6.b step 5's negative invariant, as executed for story 1-26
  (`docs/dev-notes/1-26-d6b-re-emission-2026-09-25.md:108`), requires *"rows before / after equal,
  none added, none deleted"*. Needs an operator ruling, and the anchor count 119 moves.
- **(c) Leave the row and relabel it historical.** The `#118` disposition shape. Then this scenario
  keeps no reproduction claim and its R-REPRO gate (`determinism.rs:1621`) should be deleted rather
  than re-pinned.

**What settles it:** an operator/architect ruling between (a), (b) and (c). Nothing further can be
measured that changes the choice — the measurement is already unambiguous, it is the *corpus shape*
that has no slot.

### The six PROBABLE rows — one measurement closes them

Re-run each of the six `-realdata` scenarios three times from the workspace root under
`--features candle,realdata` (release — bug-log `#121`), with `--reports-dir` to a tempdir
(bug-log `#113`), and hash each body:

| run | flags | prediction |
|---|---|---|
| A | *(none)* | **equals the handed-in produced SHA** |
| B | `--sim-slippage-bps 8` | a third, distinct SHA (the re-emission for the `v5-realdata-medium-2026-05` row) |
| C | `--sim-slippage-sqrt-alpha 1.0 --sim-slippage-sqrt-lookback-days 90` | a fourth, distinct SHA (the re-emission for the `v5-sqrt-impact-2026-05` row) |

If A reproduces the handed-in SHA, the mapping in § 1 is CERTAIN for all six. If A does **not**, the
handed-in numbers were taken under an undeclared condition and the whole table needs redoing —
which is the `#112` failure mode and the reason this run is worth its wall-clock.

Run A is nearly free: it is what the existing R-REPRO gates already do. It is only the *expectation*
they compare against that this note says is wrong.

---

## 3. Does one rule cover all 15?

**Yes — one rule covers 14 of 15, and it explains why the 15th fails.**

> Target row = the row whose namespace-condition equals the `slippage_model` that
> `build_slippage_model_for_scenario` (`main.rs:195-221`) returns for that scenario under default
> flags. Synthetic ⇒ `Linear{bps:8}` ⇒ `… + v5-realdata-medium-2026-05`. Real-data (the 9 ids at
> `main.rs:43-59`) ⇒ `Linear{bps:0}` ⇒ the zero-sim-slippage (pre-v5) row.

It holds for every case:

| scenario | path | model under default flags | condition-matching row | exists? |
|---|---|---|---|---|
| `top10-{2023-1h,2024-h1}-momentum` | synthetic | `Linear{bps:8}` | `v1 + v5-realdata-medium…` | ✅ |
| `top10-{2023,2024}-fy-tcn-overlay` | synthetic | `Linear{bps:8}` | `v2.5.0 + v5-realdata-medium…` | ✅ |
| `pairs-{2023,2024-h1}-zscore-mr` | synthetic | `Linear{bps:8}` | `v1.5a + v5-realdata-medium…` | ✅ |
| `top10-{2023,2024}-fy-tcn-overlay-weights` | synthetic | `Linear{bps:8}` | `v2.5.0-tcn-weights + v5-realdata-medium…` | ✅ |
| `top10-{2023,2024}-fy-tcn-overlay-realdata` | real-data | `Linear{bps:0}` | `v2.6.0-realdata + noop-baseline` | ✅ |
| `top10-{2023,2024}-fy-tcn-overlay-weights-realdata` | real-data | `Linear{bps:0}` | `v2.6.0-realdata + noop-baseline` | ✅ |
| `top10-2023-fy-patchtst-overlay-realdata` | real-data | `Linear{bps:0}` | `v2.5a.0-patchtst + noop-baseline` | ✅ |
| `top10-2023-fy-vol-target-overlay-realdata` | real-data | `Linear{bps:0}` | `v3.0.0-volatility + noop-baseline` | ✅ |
| `top10-2023-fy-momentum-realdata` | real-data | `Linear{bps:0}` | *(zero-sim-slippage row)* | ❌ **only the sqrt row is anchored** |

The single exception is a **corpus gap, not a rule failure**: the rule identifies the condition
correctly and there is no row carrying it.

### The "newest namespace" heuristic does NOT hold — checked, not assumed

| | newest-namespace answer | derived answer | agree? |
|---|---|---|---|
| 8 synthetic | `… + v5-realdata-medium-2026-05` | same | ✅ 8/8 — **by coincidence**: there is no synthetic `v5-sqrt-impact` row, because Q-D1=(a) deliberately froze the synthetic paths out of the sqrt re-emit (`main.rs:187-191`) |
| 6 real-data | `v5-sqrt-impact-2026-05` | `… + noop-baseline` | ❌ 0/6 |
| `momentum-realdata` | `v5-sqrt-impact-2026-05` | no row | vacuous — one candidate |

So the heuristic is right on the easy half for a reason that has nothing to do with newness, and
wrong on every case where the scenario actually has three rows. This is bug-log `#120` exactly —
*"a scenario's canonical namespace is a property of the scenario"* — and `#120`'s fix was applied
only to the two scenarios whose measurement forced it. **The same error still stands in seven more
pins.**

### Two things the corpus says and the derivation contradicts

Both are worth reading before landing the re-lock:

1. **`anchors.toml:16-17`** — *"the in-test constants for the default (no-feature) binary mirror the
   `v5-realdata-medium-2026-05` rows here (D6.1 mapping rule)"*, mechanised by
   `check_determinism_anchors.py:21-24`. Correct, and correctly scoped: *no-feature* binary ⇒
   synthetic paths ⇒ Q-D1=(a) ⇒ 8 bps. It says nothing about the realdata binary, and reading it as a
   corpus-wide rule is what produces the sqrt mis-pins.
2. **`check_determinism_anchors.py:25-28`** — R3 skips the m3 pair because *"the m3_\* candle pair …
   has **no default-binary `v5-realdata-medium` mapping**"*. It does have one: `28379df8…` /
   `0c13ed0b…`, in the v0.4.0 **candle** re-emit dir, which is the generation a candle-built gate
   should pin. The cfg-gate skip is the reason a wrong pin survived four months
   (bug-log `#115` × `#122`: *"two findings that only become visible together"*).
3. **`main.rs:1641-1643`** carries the comment *"Real-data: SquareRoot model + universe-avg V map.
   Synthetic: Linear{bps:8} fallback"*. That describes the **v0.5.0 re-emission invocation**, not the
   default: real-data gets `SquareRoot` only when `--sim-slippage-sqrt-alpha > 0`, and the flag
   defaults to `0` (`main.rs:127-128`). A reader who trusts the comment concludes the default
   invocation targets the sqrt row. That comment is the most likely origin of the nine sqrt R-REPRO
   pins, and it should be corrected in the same pass.

---

## 4. Landing constraint: the resolver decides where the re-emitted bodies may be written

`scripts/verify_anchors.sh` resolves each namespace to a **different directory set**. Story 1-26's
precedent was *"copied into the same anchored directory as the report each supersedes"*
(`1-26-d6b-re-emission-2026-09-25.md:94-97`). For this re-lock that is not optional — two of the
three branches ignore anything written elsewhere:

| target namespace | branch | where a new body MUST land |
|---|---|---|
| `… + v5-realdata-medium-2026-05` (the 8 synthetic) | `verify_anchors.sh:119-145` | `evidence/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit/reports/` **(checked first — any file there wins)**, else the v0.3.0 dir with a later stamp. A body written under `evidence/v2/…` is **never seen**: the global-newest fallback at `:131-135` is only reached when all three migration dirs miss, and for all 8 of these scenarios v0.3.0 or v0.4.0 hits. |
| `v5-sqrt-impact-2026-05` (if § 2 option (a) is chosen) | `:146-149` | **only** `evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/`. No fallback at all. |
| `… + noop-baseline` (the 6 real-data) | `:105-118` | newest *outside* every `v5-latency-slippage-sim-v0*` dir, selected by `find … \| sort \| tail -1` — **a lexicographic sort of full paths, so the directory name outranks the timestamp.** `evidence/v2/…` or `evidence/v3/…` wins over the current `evidence/v1/…` bodies; a dir sorting before `v1` (e.g. `evidence/lumen-…`) would silently lose. |

---

## 5. Scorecard for the 15 current in-test pins

All 15 scenarios already have an in-test `const ANCHOR`. On this derivation, **9 of the 15 point at
the wrong row**:

| pin site | scenario | pins | should pin | status |
|---|---|---|---|---|
| `determinism.rs:770` | `top10-2023-1h-momentum` | canonical | canonical | ✅ |
| `determinism.rs:786` | `top10-2024-h1-momentum` | canonical | canonical | ✅ |
| `determinism.rs:813` | `top10-2023-fy-tcn-overlay` | canonical | canonical | ✅ |
| `determinism.rs:829` | `top10-2024-fy-tcn-overlay` | canonical | canonical | ✅ |
| `multi_pair_determinism.rs:141` | `pairs-2023-zscore-mr` | canonical | canonical | ✅ |
| `multi_pair_determinism.rs:149` | `pairs-2024-h1-zscore-mr` | canonical | canonical | ✅ |
| `determinism.rs:931` | `top10-2023-fy-tcn-overlay-weights` | **noop** | canonical | ❌ (`#122`) |
| `determinism.rs:949` | `top10-2024-fy-tcn-overlay-weights` | **noop** | canonical | ❌ (`#122`) |
| `determinism.rs:1500` | `top10-2023-fy-tcn-overlay-realdata` | **sqrt** | noop | ❌ new |
| `determinism.rs:1509` | `top10-2024-fy-tcn-overlay-realdata` | **sqrt** | noop | ❌ new |
| `determinism.rs:1518` | `top10-2023-fy-tcn-overlay-weights-realdata` | **sqrt** | noop | ❌ new |
| `determinism.rs:1531` | `top10-2024-fy-tcn-overlay-weights-realdata` | **sqrt** | noop | ❌ new |
| `determinism.rs:1622` | `top10-2023-fy-momentum-realdata` | **sqrt** | *(no row)* | ❌ unfixable as-is |
| `determinism.rs:1632` | `top10-2023-fy-patchtst-overlay-realdata` | **sqrt** | noop | ❌ new |
| `determinism.rs:1669` | `top10-2023-fy-vol-target-overlay-realdata` | **sqrt** | noop | ❌ new |

Six of those seven `-realdata` failures are the *same* mistake `#120` caught and fixed for the two
regime-dispatcher gates. The assertion messages also hard-code the wrong namespace name in prose —
`assert_reproduces_canonical_anchor` (`determinism.rs:1590-1592`) and
`assert_reproduces_or_report_unmeasured` (`:1708-1710`) both say *"no longer reproduces its canonical
`v5-sqrt-impact-2026-05` anchor"*, which is already false for the two green regime gates that call
the second one.

---

## 6. Two smaller corrections found on the way

- **The task's stated condition is inexact for the m3 pair.** The eight non-`-realdata` scenarios did
  *not* all run "from a tempdir with the no-feature binary". `run_scenario_once`
  (`determinism.rs:435-491`) and `run_pairs_scenario_once` (`multi_pair_determinism.rs:13-60`) are
  tempdir + no-feature, but `run_scenario_once_candle` (`determinism.rs:861-916`) runs
  **`--features candle` from the workspace root** with `--reports-dir` to a tempdir — the weights
  scenarios cannot run otherwise. It makes no difference to the answer (the `Data source:` line on
  every weights generation says `synthetic (seeded RNG, v2.5 tcn-overlay-weights)`, and multi-symbol
  scenarios branch on the static `ScenarioDataSource` enum and never reach the CWD-sensitive
  auto-detect site — `main.rs:137-140`), but the condition should be recorded as it is.
- **A factual error inside frozen evidence.**
  `evidence/…-v0.5.0-…/reports/sharpe-delta-2026-05-29.md:64` states *"Momentum-realdata noop is from
  v3-volatility-forecaster-rebaseline (same SHA as v0.2.0 noop)"*. The two bodies differ:
  `evidence/v1/v3-volatility-forecaster-rebaseline/reports/backtest-20260522-095222-…` hashes
  `70b8d442…` at `$113 479.98` / `+13.48 %`, while
  `evidence/…-v0.2.0-anchor-migration/reports/backtest-20260527-065715-…` hashes `ed5717f8…` at
  `$77 001.73` / `−23.00 %` — the 8-bps body. That table's "Noop return −23.00 %" column for this
  scenario is therefore the *linear* number, which is why its Noop and Linear columns agree. The file
  is anchored-adjacent and byte-immutable; the correction belongs here and in the bug-log, not in it.

---

## Provenance

Derived from: `crates/backtest/src/main.rs`, `crates/backtest/src/cli_types.rs`,
`crates/backtest/src/scenarios/sim.rs`, `crates/backtest/src/scenarios/tcn_overlay.rs`,
`crates/exec/src/latency.rs`, `crates/backtest/tests/determinism.rs`,
`crates/backtest/tests/multi_pair_determinism.rs`, `scripts/verify_anchors.sh`,
`scripts/check_determinism_anchors.py` (run: `OK — 14 literal(s) match … 0 skipped`),
`scripts/hash_report.py` (applied to all 87 committed report bodies for the 16 scenarios in scope),
`evidence/anchors.toml`, the v0.4.0 and v0.5.0 sharpe-delta tables,
`docs/dev-notes/archive/2026-Q2/engine-drift-diagnosis-2026-05-30.md`,
`docs/dev-notes/1-26-d6b-re-emission-2026-09-25.md`, ADR-0038 § D6 / § D6.b, and bug-log
`#93` `#111` `#112` `#113` `#114` `#115` `#116` `#118` `#120` `#121` `#122`.
No `cargo` command was run; no file outside this one was modified.
