# Reproduction recipes for the 14 ungated forecast/report-binary anchors — 2026-09-27

> **A gate that cannot fail is indistinguishable from a gate that passes.** A body-SHA reproduction
> claim is meaningless unless the condition — CWD × feature set × build profile × invocation × input
> corpus — is *declared*. This note declares it for the 14 anchored scenarios that
> `docs/dev-notes/anchor-gate-coverage-audit-2026-09-26.md` § 2 counts inside the "56 with nothing".

## Provenance

**Verified by reading code** (every `file:line` below is a direct read of the file at HEAD
`74910b37`): the clap `Args` structs and their `default_value`s; the `required-features` lines in
`crates/{forecast,backtest}/Cargo.toml`; every `#[cfg(feature = …)]` branch in the six bins and in
the scenario modules they reach; the filename `format!` sites; the CWD-relative path literals; the
predecessor-report reads; `scripts/verify_anchors.sh`'s namespace resolver; `scripts/hash_report.py`'s
body definition.

**Verified by running a command** (three, all read-only): `ls` on `data/binance/` and
`crates/forecast/checkpoints/anchors/`; `cargo build -p forecast --bin forecast_distribution` and
`cargo build -p backtest --bin threshold_sweep` with no `--features` (to capture cargo's *actual*
refusal text, quoted verbatim in § 2); `cargo tree -p backtest` with and without `--features candle`
(to settle whether `candle-core` is linked either way — it is).

**Verified by `ls`/`find`**: the presence of all 10 checkpoint files, all 24 parquet months for
BTCUSDT, `data/binance/REVISION.toml` and its `sha256` line, and the on-disk location of all 14
anchored report bodies.

**Measured, quoted from the artefact**: every runtime in § 5 is the `wall_clock_s` field in the
anchored report's own YAML front-matter — a number the producing run wrote about itself. It is not my
measurement and it excludes `cargo build`. Where a bin measures only part of its own run, § 5 says so.

**Estimates / inferences, labelled as such**: the drift predictions in § 4.3 (derived from dates and
from `anchors.toml`'s own re-emission comments, not from a run); the "build ≈ N min" figures in § 5;
the claim that `vol_verdict` is numerically candle-invariant (a code-reading conclusion from
`features.rs:811-824`, **not** measured).

**Not run at all**: none of the six binaries. No report was emitted. No file under `crates/`,
`evidence/`, `scripts/` or `_bmad-output/` was touched.

---

## 1. The 14 rows, with namespaces verbatim

`evidence/anchors.toml` is keyed by `(scenario, version)`. Twelve of the 14 carry **two** rows each
(the `noop-baseline` pin and the v5 canonical pin); the two `v3.0.0-regime` scenarios carry one. In
every dual case **both rows hold the identical sha256** — consistent with `anchors.toml:390`
("Investigation/analysis reports and operator success reports: SHA unchanged") — and both resolve to
the **same single file on disk**, because the v5 resolver's migration-dir probes look only for
`backtest-*-<scenario>.md` and these reports are named `<scenario>-<date>.md`.

| # | scenario | `version` (verbatim) | anchors.toml line | sha256 |
|---|---|---|---|---|
| 1 | `forecast-distribution-bs1-realdata` | `v2.6.0-alpha-investigation + noop-baseline` | 211 | `ef73cb8d…b216d54` |
| | | `v2.6.0-alpha-investigation + v5-realdata-medium-2026-05` | 561 | same |
| 2 | `forecast-distribution-bs2-realdata` | `v2.6.0-alpha-investigation + noop-baseline` | 216 | `d7cd08e6…c405fc06` |
| | | `v2.6.0-alpha-investigation + v5-realdata-medium-2026-05` | 566 | same |
| 3 | `forecast-distribution-bs1-realdata-recalibrated` | `v2.6.1-alpha-investigation-recalibrated + noop-baseline` | 236 | `8a548042…8a06f80f` |
| | | `v2.6.1-alpha-investigation-recalibrated + v5-realdata-medium-2026-05` | 576 | same |
| 4 | `forecast-distribution-bs2-realdata-recalibrated` | `v2.6.1-alpha-investigation-recalibrated + noop-baseline` | 241 | `d6c1e17c…7deaf4b4151` |
| | | `v2.6.1-alpha-investigation-recalibrated + v5-realdata-medium-2026-05` | 581 | same |
| 5 | `recalibrate-sigma-train-bs1` | `v2.6.1-alpha-investigation-recalibrated + noop-baseline` | 246 | `baa658fb…50336ad9` |
| | | `v2.6.1-alpha-investigation-recalibrated + v5-realdata-medium-2026-05` | 586 | same |
| 6 | `recalibrate-sigma-train-bs2` | `v2.6.1-alpha-investigation-recalibrated + noop-baseline` | 251 | `bfa8104a…954b47e0` |
| | | `v2.6.1-alpha-investigation-recalibrated + v5-realdata-medium-2026-05` | 591 | same |
| 7 | `threshold-sweep-bs1-realdata-recalibrated` | `v2.6.2-threshold-tuning + noop-baseline` | 263 | `551cc2ab…0dacc2f9c` |
| | | `v2.6.2-threshold-tuning + v5-realdata-medium-2026-05` | 596 | same |
| 8 | `threshold-sweep-bs2-realdata-recalibrated` | `v2.6.2-threshold-tuning + noop-baseline` | 268 | `755bc380…c8b54486c3` |
| | | `v2.6.2-threshold-tuning + v5-realdata-medium-2026-05` | 601 | same |
| 9 | `forecast-distribution-patchtst-bs1-realdata` | `v2.5a.0-patchtst + noop-baseline` | 285 | `c55c6c51…bcca7f4646dd` |
| | | `v2.5a.0-patchtst + v5-realdata-medium-2026-05` | 606 | same |
| 10 | `vol-verdict-bs1-realdata` | `v3.0.0-volatility + noop-baseline` | 324 | `99c21892…686cd21` |
| | | `v3.0.0-volatility + v5-realdata-medium-2026-05` | 616 | same |
| 11 | `sharpe-comparison-vol-target-bs1-realdata` | `v3.0.0-volatility + noop-baseline` | 343 | `d21db467…fed701a31` |
| | | `v3.0.0-volatility + v5-realdata-medium-2026-05` | 626 | same |
| 12 | `sharpe-comparison-vol-target-bs1-realbaseline` | `v3.0.0-volatility-rebaseline + noop-baseline` | 372 | `ff2b9349…b3989d95e9` |
| | | `v3.0.0-volatility-rebaseline + v5-realdata-medium-2026-05` | 631 | same |
| 13 | `regime-verdict-bs1-realdata` | `v3.0.0-regime` | 709 | `2d248f4e…a0323a2eda1d` |
| 14 | `sharpe-comparison-regime-dispatcher-bs1-realdata` | `v3.0.0-regime` | 714 | `a9e00139…04e5531ab97` |

### 1.1 Which file each anchor resolves to today

All 14 resolve through `verify_anchors.sh`'s **third** pattern —
`*/reports/${scenario}-[0-9]+\.md$` — and each has **exactly one** match on disk (`find` verified):

| scenario | resolved file |
|---|---|
| 1 | `evidence/v1/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md` |
| 2 | `evidence/v1/v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md` |
| 3 | `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` |
| 4 | `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md` |
| 5 | `evidence/v1/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md` |
| 6 | `evidence/v1/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md` |
| 7 | `evidence/v1/v25-tcn-threshold-tuning/reports/threshold-sweep-bs1-realdata-recalibrated-20260521.md` |
| 8 | `evidence/v1/v25-tcn-threshold-tuning/reports/threshold-sweep-bs2-realdata-recalibrated-20260521.md` |
| 9 | `evidence/v1/v25a-patchtst-overlay/reports/forecast-distribution-patchtst-bs1-realdata-20260521.md` |
| 10 | `evidence/v1/v3-volatility-forecaster/reports/vol-verdict-bs1-realdata-20260522.md` |
| 11 | `evidence/v1/v3-volatility-forecaster/reports/sharpe-comparison-vol-target-bs1-realdata-20260522.md` |
| 12 | `evidence/v1/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md` |
| 13 | `evidence/v1/v3-regime-classifier/reports/regime-verdict-bs1-realdata-20260529.md` |
| 14 | `evidence/v1/v3-regime-classifier/reports/sharpe-comparison-regime-dispatcher-bs1-realdata-20260529.md` |

The `-recalibrated-` files do **not** collide with the plain `forecast-distribution-bs{1,2}-realdata`
rows: `verify_anchors.sh:224` (and the noop/canonical equivalents) filter the glob through
`grep -E "/reports/${scenario}-[0-9]+\.md$"`, and `recalibrated-20260521` is not `[0-9]+`.

---

## 2. Required cargo features — refusal vs silent fallback

**This is the load-bearing distinction.** Three of the six bins *refuse*; three *do not*.

### 2.1 Hard refusal at cargo target-selection (cannot be run wrong)

`crates/forecast/Cargo.toml:53-56` and `:58-61`, `crates/backtest/Cargo.toml:11-14` declare
`required-features`. Cargo refuses before compiling anything. Verified by execution:

```
$ cargo build -p forecast --bin forecast_distribution
error: target `forecast_distribution` in package `forecast` requires the features: `candle`
Consider enabling them by passing, e.g., `--features="candle"`

$ cargo build -p backtest --bin threshold_sweep
error: target `threshold_sweep` in package `backtest` requires the features: `candle`, `realdata`
Consider enabling them by passing, e.g., `--features="candle realdata"`
```

| bin | `required-features` | declared at |
|---|---|---|
| `forecast_distribution` | `["candle"]` | `crates/forecast/Cargo.toml:56` |
| `recalibrate_sigma_train` | `["candle"]` | `crates/forecast/Cargo.toml:61` |
| `threshold_sweep` | `["candle", "realdata"]` | `crates/backtest/Cargo.toml:14` |

`threshold_sweep` additionally carries a belt-and-braces `#[cfg(not(feature = "realdata"))]
anyhow::bail!` at `crates/backtest/src/bin/threshold_sweep.rs:698-701` — unreachable via cargo, and
correctly so.

### 2.2 No refusal — buildable with default features (the hazard)

`crates/forecast/Cargo.toml:40-42` (`vol_verdict`), `:44-46` (`regime_verdict`), `:68-70`
(`sharpe_comparison`) declare **no** `required-features`. All three build and run under bare
`cargo run -p forecast --bin …`.

- **`vol_verdict`** — compiles under BOTH feature sets and silently takes a different code path.
  `crates/forecast/src/bin/vol_verdict.rs:811-831` branches on `candle` purely to extract
  `r_prev` from `window.features`, whose type is `Tensor` with the feature and `Vec<f32>` without
  (`crates/forecast/src/features.rs:229-235`). Reading
  `crates/forecast/src/features.rs:811-824`, the ONLY difference is the container: the same
  `feat_matrix: Vec<f32>` is either wrapped via `Tensor::from_vec(feat_matrix, …)` or used directly.
  **Inference, not measurement:** the numbers are therefore identical and `vol-verdict-bs1-realdata`
  should be byte-reproducible either way. The bin has no GARCH-side candle dependency at all. *This
  has not been empirically confirmed — if a gate is built, it should assert it.* Note the bin's own
  doc comment (`vol_verdict.rs:12`) prescribes `--features candle --release`, so `candle` is the
  declared condition even if it turns out to be immaterial.
- **`regime_verdict`** — needs no features itself; it **shells out** to a `backtest` binary
  (`regime_verdict.rs:827-835`) that must have been built `--features realdata`.
  **Silent-fallback hazard:** if the spawned binary exists but *refuses* (wrong features →
  `main.rs:1456-1463` "requires --features realdata"), `regime_verdict` does **not** propagate the
  failure. `regime_verdict.rs:837-853` logs a warning, substitutes `String::new()` for the report
  body, and `parse_run_stats` then yields zeroed stats → a V-REG-1 "convergence failure" report is
  written and the process exits 0. A wrong-feature build produces a plausible-looking report with a
  different body-SHA and no error. If the binary is *absent*, `.status()` errors and the run does
  fail (`regime_verdict.rs:834`).
- **`sharpe_comparison`** — needs no features itself; also shells out (`sharpe_comparison.rs:404-416`).
  Unlike `regime_verdict`, it **does refuse**: `sharpe_comparison.rs:414-416` `anyhow::bail!`s on a
  non-zero exit from the backtest binary. This is the correct shape.

### 2.3 The `backtest` binary's feature condition, and a decoupling trap

For the three scenarios that shell out, the `backtest` binary is the real condition:

- `--features realdata` is **mandatory and refused if absent**. The realdata scenario arms are
  `#[cfg(feature = "realdata")]`-gated and the fall-through is
  `crates/backtest/src/main.rs:1186` `other => anyhow::bail!("unknown scenario: {other}")`; the
  data-loading path additionally bails at `main.rs:1356-1363` and `:1456-1463` with a message naming
  the feature. Clean refusal.
- `--features candle` is **not** needed for any of the four sub-scenarios these three re-run
  (`top10-2023-1h-momentum`, `top10-2023-fy-momentum-realdata`,
  `top10-2023-fy-vol-target-overlay-realdata`, `top10-2023-fy-regime-dispatcher-realdata`). The
  `candle` cfg in `backtest` appears in exactly three scenario modules —
  `scenarios/tcn_overlay_weights.rs:35`, `scenarios/patchtst_overlay_weights.rs:50`,
  `scenarios/threshold_sweep.rs:63` — and none of the four routes through them. Those three *do*
  refuse cleanly when it is absent (e.g. `tcn_overlay_weights.rs:35-43`: *"requires --features candle
  (real TCN weights)"*, under a comment stating the refusal is deliberate "rather than a silent
  fallback — per the operator-locked decision in the M3 punch list").
- **The trap:** `backtest`'s own `candle` feature is `candle = ["strategy/forecast"]`
  (`crates/backtest/Cargo.toml:38`), while `[dependencies] strategy = { path = "../strategy",
  features = ["forecast"] }` (`:63`) already enables it unconditionally. Verified with
  `cargo tree -p backtest`: `candle-core` appears **3 times with and 3 times without**
  `--features candle`. So the flag buys no capability and saves no compile time — its *only* effect
  is flipping the `#[cfg(feature = "candle")]` blocks in `crates/backtest/src/scenarios/*.rs` and the
  `required-features` on the `threshold_sweep` target. A reader who reasons "candle isn't in the
  build, so the tensor path can't run" is wrong in both directions. Passing `candle,realdata` is
  harmless and the safer default for any `backtest` build used here.

---

## 3. Required CWD — every bin needs the workspace root

None of the six uses `env!("CARGO_MANIFEST_DIR")` in its own source. The single manifest-relative
fallback in reach is `forecast::tcn::resolve_anchors_dir`
(`crates/forecast/src/tcn.rs:1489-1519`), which tries CWD-relative **first**:

```rust
// crates/forecast/src/tcn.rs:1490-1496
const REL: &str = "crates/forecast/checkpoints/anchors";
let cwd_relative = PathBuf::from(REL);
if cwd_relative.is_dir() { … return cwd_relative; }
// CARGO_MANIFEST_DIR is `<workspace>/crates/forecast` for this crate.
let from_manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()) …
```

That fallback only covers `load_anchor`. Every other input in every bin is bare CWD-relative, so
**CWD MUST be the workspace root for all six**:

| bin | CWD-relative resolution | file:line |
|---|---|---|
| `forecast_distribution` | `--data-root` default `"data/binance/"`; and in the `--metadata-path` arm `let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");` | `forecast_distribution.rs:115`, `:835` |
| `recalibrate_sigma_train` | `--data-root` default; and **unconditionally** `let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");` to read the original metadata | `recalibrate_sigma_train.rs:130`, `:506` |
| `threshold_sweep` | `--data-root` default; `PathBuf::from("crates/forecast/checkpoints/anchors")`; `PathBuf::from("config/strategies/top10_momentum_h1.toml")` at three sites (note: the bin does **not** use `backtest::paths::resolve_workspace_path`, unlike `scenarios/threshold_sweep.rs:92`); and the predecessor report path of § 4.1 | `threshold_sweep.rs:142`, `:691`, `:774`, `:807`, `:884`, `:106`, `:109` |
| `vol_verdict` | `--data-root` default; `--anchors-dir` default `"crates/forecast/checkpoints/anchors/"` | `vol_verdict.rs:123`, `:131` |
| `regime_verdict` | `--backtest-bin` default `"target/release/backtest"`; and `read_data_revision_sha()` reads a hardcoded path with no flag: `let rev_path = std::path::Path::new("data/binance/REVISION.toml");` | `regime_verdict.rs:99`, `:770` |
| `sharpe_comparison` | `--backtest-bin` default; `read_data_revision_sha()` hardcodes `"data/binance/REVISION.toml"` | `sharpe_comparison.rs:95`, `:2003` |
| (spawned) `backtest` | **no `--data-root` flag exists**: `crates/backtest/src/main.rs:1298` is `let data_root = PathBuf::from("data/binance");` | `main.rs:1298` |

Consequence for any future gate: `cargo test` sets CWD to the **package** root, so a gate must set
`.current_dir(workspace_root)` exactly as `determinism.rs:1163-1170` already does.

---

## 4. Inputs on disk, output-directory flags, filename shapes

### 4.1 Inputs — all PRESENT

Checkpoints (`ls crates/forecast/checkpoints/anchors/`) — **10/10 PRESENT**, real files, not LFS
pointers:

| file | size | needed by |
|---|---|---|
| `tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors` | 1 676 516 | 1, 3, 5, 7 |
| `tcn-bs1-…d2.metadata.json` | 855 | 1, 3, 5, 7 |
| `tcn-bs1-…d2.metadata.recalibrated.json` | 858 | 3, 7 |
| `tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors` | 1 676 516 | 2, 4, 6, 8 |
| `tcn-bs2-…1d.metadata.json` | 852 | 2, 4, 6, 8 |
| `tcn-bs2-…1d.metadata.recalibrated.json` | 855 | 4, 8 |
| `patchtst-bs1-62520db92f68c1d323f0782bc367c742cf9439631106ddc0fd492188f6d1cd4d.safetensors` | 1 729 220 | 9 |
| `patchtst-bs1-…dd.metadata.json` | 911 | 9 |
| `garch-bs1-991324772ba077355731c2f551e3412430070b76468f6044261161a9160c0c71.json` | 2 993 | 10 |

Corpus: `data/binance/` **PRESENT** — 10 symbol dirs (ADAUSDT AVAXUSDT BNBUSDT BTCUSDT DOGEUSDT
DOTUSDT ETHUSDT LINKUSDT SOLUSDT XRPUSDT), each with `2023/` and `2024/` × 12 monthly parquets
(`ls data/binance/BTCUSDT/{2023,2024}/` → `01.parquet … 12.parquet`), plus
`data/binance/REVISION.toml` (23 144 bytes). Its `sha256` line **matches the pin**:

```
data/binance/REVISION.toml:2:  sha256 = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"
```

That value is asserted at runtime by `threshold_sweep`'s `--expected-revision-sha` default
(`threshold_sweep.rs:157-160`) and by every realdata scenario arm's `expected_revision_sha`
(e.g. `main.rs:1108-1111`) — a mismatch is a hard bail, not a warning. It also lands **inside the
hashed body** of `forecast_distribution` (`forecast_distribution.rs:1137`, the `| Revision SHA |`
row) and of `threshold_sweep` (`threshold_sweep.rs:429`). For `sharpe_comparison` and
`regime_verdict` it is front-matter only (`sharpe_comparison.rs:758,1135,1479,1863`;
`regime_verdict.rs:741-750`) and therefore excluded from the hash by `scripts/hash_report.py:21`.

**Predecessor-report dependency — exactly one, and it is silent on failure.**
`threshold_sweep` reads a recalibrated `forecast_distribution` report to recover gate-survivor counts:

```rust
// crates/backtest/src/bin/threshold_sweep.rs:103-110
fn gate_survivor_report(self) -> &'static str {
    match self {
        ScenarioArg::Bs1 => "evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md",
        ScenarioArg::Bs2 => "evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md",
    }
}
```

consumed at `threshold_sweep.rs:741`. Both files are PRESENT. The path is **hardcoded, including the
`20260521` date** and is CWD-relative. And:

```rust
// crates/backtest/src/bin/threshold_sweep.rs:246-251
fn parse_gate_survivors(report_path: &str) -> [usize; 9] {
    let mut result = [0usize; 9];
    let content = match std::fs::read_to_string(report_path) {
        Ok(c) => c,
        Err(_) => return result,     // ← all zeros, no error, no log
    };
```

A missing or unreadable predecessor yields nine zeros and a completed run. The corpus gate *would*
catch the resulting body drift, but the bin itself says nothing — flag this if a gate is written.

No other predecessor-report reads exist among the 14. The `RecalDelta` pre-recalibration numbers that
`forecast_distribution` cites from the BS-1/BS-2 originals are **compiled-in constants**, not file
reads (`forecast_distribution.rs:624-651`), so runs 3 and 4 have no file dependency on runs 1 and 2.

### 4.2 Output-directory flags — where the landmines are

`scripts/verify_anchors.sh` resolves each anchor to the **newest** matching file (`sort | tail -1`).
Twelve of the 14 embed today's date in the filename, so a default-out-dir run today writes
`…-20260927.md`, which sorts after the locked `…-2026052x.md` and **silently becomes the file the
corpus gate hashes**. The two `threshold-sweep` files are worse: their filename has a hardcoded date,
so a default-out-dir run **overwrites the anchored, byte-immutable body in place**.

| bin | flag | default | file:line | tempdir override possible? |
|---|---|---|---|---|
| `forecast_distribution` | `--out-dir` | `evidence/v1/v25-tcn-alpha-investigation/reports/` | `:119-122` | **YES** — and additionally `--output <full path>` (`:152`) bypasses the filename logic entirely (`:1051-1054`). Note `:859-861` still `create_dir_all`s `--out-dir` even when `--output` is given, so the evidence dir is touched-but-not-written. |
| `recalibrate_sigma_train` | `--out-dir` | `evidence/v1/v25-tcn-recalibrate/reports/` | `:134` | **YES for the report. NO for the second output** — see below. |
| | `--anchor-dir` | `crates/forecast/checkpoints/anchors/` | `:140` | flag exists, but **overriding it changes the body-SHA** — see below. |
| `threshold_sweep` | `--out-dir` | `evidence/v1/v25-tcn-threshold-tuning/reports/` | `:152` | **YES — and MANDATORY.** Filename is date-hardcoded (`:1045`); a default run clobbers the anchored file. |
| `vol_verdict` | `--out-dir` | `evidence/v1/v3-volatility-forecaster/reports/` | `:127` | **YES** |
| `regime_verdict` | `--out-dir` (`Option`) | `evidence/v1/v3-regime-classifier/reports/` | `:95` decl, `:790-793` default | **YES** |
| `sharpe_comparison` | `--out-dir` (`Option`) | per-family: `vol-target-bs1` → `evidence/v1/v3-volatility-forecaster/reports/`; `vol-target-bs1-rebaseline` → `evidence/v1/v3-volatility-forecaster-rebaseline/reports/`; `regime-dispatcher-bs1` → `evidence/v1/v3-regime-classifier/reports/` | `:91` decl, `:2024-2033` defaults | **YES** |

**`recalibrate_sigma_train` is the one bin that cannot be fully sandboxed.** It has two writes
(`recalibrate_sigma_train.rs:20-26` declares both). The second is the `.metadata.recalibrated.json`
overlay, written to `--anchor-dir` (`:626-631`) — whose default is the committed checkpoint
directory, where **both overlay files already exist**. And the overlay's resolved path is passed into
the report renderer and lands in the **hashed body**, not the front-matter:

```rust
// crates/forecast/src/bin/recalibrate_sigma_train.rs:454-458  (inside the body builder at :443)
writeln!(&mut body, "- Read-only against `{overlay_path}` original safetensors.")
// caller: recalibrate_sigma_train.rs:686  →  &overlay_path.display().to_string()
```

So there is no tempdir setting that both leaves the repo untouched **and** reproduces the anchored
SHA. Reproducing rows 5 and 6 requires `--anchor-dir crates/forecast/checkpoints/anchors/` (the
default), which rewrites the two committed overlay JSONs. They are expected to be byte-identical (the
bin is deterministic per ADR-0035 § D1 and the inputs are unchanged), but that is a *hope*, not a
guarantee, and `git status` must be checked afterwards. **Any gate for rows 5/6 needs either a code
change (render the overlay path relative to `--anchor-dir`, i.e. a body change → a D6.b re-emission)
or an accepted repo-mutating step with a `git diff --exit-code` assertion on
`crates/forecast/checkpoints/anchors/`.** This is the single genuine blocker among the 14.

### 4.3 Filename shapes

| bin | `format!` | file:line | date source |
|---|---|---|---|
| `forecast_distribution` (recal) | `"forecast-distribution-{}-realdata-recalibrated-{}.md"` | `:1039-1043` | `today` = `{}{:02}{:02}` from `SystemTime::now()`, `:1030-1037` |
| `forecast_distribution` (default) | `"forecast-distribution-{}-realdata-{}.md"` | `:1045-1049` | same |
| `recalibrate_sigma_train` | `"recalibrate-sigma-train-{scenario_label}-{today}.md"` | `:690` | `today`, `:656-665` |
| `recalibrate_sigma_train` (overlay) | `"{prefix}-{sha}.metadata.recalibrated.json"` | `:628` | — |
| `threshold_sweep` | `"threshold-sweep-{label}-realdata-recalibrated-20260521.md"` | `:1045` | **hardcoded `20260521`** |
| `vol_verdict` | `"vol-verdict-{}-realdata-{}.md"` | `:945-949` | `today_yyyymmdd()`, `:635-643` |
| `regime_verdict` | `"regime-verdict-{scenario_label}-realdata-{today}.md"` | `:921` | `today`, `:891-894` |
| `sharpe_comparison` (regime) | `"sharpe-comparison-regime-dispatcher-bs1-realdata-{today}.md"` | `:2144` | `today`, `:2066-2070` |
| `sharpe_comparison` (rebaseline) | `"sharpe-comparison-vol-target-bs1-realbaseline-{today}.md"` | `:2219` | same |
| `sharpe_comparison` (vol-target) | `"sharpe-comparison-vol-target-bs1-realdata-{today}.md"` | `:2289` | same |

`threshold_sweep`'s front-matter `generated:` is also a fake fixed `2026-05-21T…` built from
epoch-seconds arithmetic (`threshold_sweep.rs:1009-1020`) — harmless, since front-matter is stripped
by `hash_report.py:21`.

### 4.4 A drift prediction for rows 11, 12, 14 — inference, not measurement

The three `sharpe-comparison-*` bodies are *derived* from backtest sub-scenario bodies that have since
changed. Grounds, all read from `anchors.toml` and `determinism.rs`:

- `top10-2023-fy-vol-target-overlay-realdata` was **re-emitted 2026-09-26** under ADR-0038 § D6.b
  (`anchors.toml:328-337`: SUPERSEDED BODY `9fa64d46…` → `91848e23…`); `determinism.rs:1637` pins the
  new value.
- `top10-2023-fy-momentum-realdata` was re-emitted too (`anchors.toml:739-744`: SUPERSEDED
  `0867d232…` → `1fc0e85d…`).
- `top10-2023-1h-momentum` was re-emitted (`anchors.toml:424-429`: SUPERSEDED `0f6f6eb8…` →
  `b655e5e7…`). It is **not** in `REAL_DATA_SCENARIO_IDS` (`main.rs:43-60`), so under Q-D1=(a) it is
  forced to `Linear { bps: 8 }` regardless of CLI flags — a decision dated 2026-05-29, *seven days
  after* rows 11 and 12 were locked (2026-05-22).

So rows 11 and 12 are **predicted to drift**; a re-run measures the drift rather than confirming the
anchor. Row 14 (locked 2026-05-29) rests on `top10-2023-fy-momentum-realdata` +
`top10-2023-fy-regime-dispatcher-realdata`; the latter is measured GREEN at its anchor
(`determinism.rs:1603-1618`, "GREEN — measured 2026-09-26") but the former is not, so row 14 is
**uncertain**. Row 13 (`regime_verdict`) rests only on `top10-2024-fy-regime-dispatcher-realdata`,
which `determinism.rs:1621-1629` measured GREEN on 2026-09-26 — so row 13 is the most likely of the
four shell-out rows to reproduce. **All four of these are predictions from dates and pins. None is a
measurement.** Treat a red result as a D6.b re-emission candidate, not as a gate bug.

---

## 5. The 14 invocations

Everything below assumes `cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading` (the
workspace root — § 3) and `--release` (the profile the anchors were locked under; see
`determinism.rs:1091-1098` for the measured evidence that release-vs-debug does not change realdata
bodies, and `:1094` for the 270 s-vs-over-an-hour reason to use release anyway).
`SWEEP=/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad`
is used as the tempdir root.

### R1 — `forecast-distribution-bs1-realdata`

```bash
cargo run --release -p forecast --features candle --bin forecast_distribution -- \
  --scenario bs1 \
  --out-dir "$SWEEP/repro/fd-bs1"
```

Feature: `candle` REQUIRED, cargo refuses without it. Inputs: `tcn-bs1-…d2.{safetensors,metadata.json}`,
`data/binance/`. No predecessor report. Emits `forecast-distribution-bs1-realdata-20260927.md`.
**Measured runtime 481.0 s** (`…-20260519.md` front-matter `wall_clock_s`) — and that measures only
the forward-pass loop (`forecast_distribution.rs:890` `t_start` is set after checkpoint load), so
wall-clock will be slightly higher.

### R2 — `forecast-distribution-bs2-realdata`

Same as R1 with `--scenario bs2` and `--out-dir "$SWEEP/repro/fd-bs2"`. **Measured 509.5 s.**

### R3 — `forecast-distribution-bs1-realdata-recalibrated`

```bash
cargo run --release -p forecast --features candle --bin forecast_distribution -- \
  --scenario bs1 \
  --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json \
  --out-dir "$SWEEP/repro/fd-bs1-recal"
```

`--metadata-path` is the **only** switch that selects the recalibrated variant: it drives both the
`-recalibrated` filename/slug/scenario suffix (`:679-683`, `:1039-1043`) and the `## Recalibration
delta` body section (`:987-996`). The safetensors still come from the hardcoded anchors dir
(`:835-838`) — hence the workspace-root CWD requirement. **Measured 486.9 s.**

### R4 — `forecast-distribution-bs2-realdata-recalibrated`

As R3 with `--scenario bs2` and
`--metadata-path crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json`.
**Measured 487.6 s.**

### R5 — `forecast-distribution-patchtst-bs1-realdata`

```bash
cargo run --release -p forecast --features candle --bin forecast_distribution -- \
  --scenario patchtst-bs1 \
  --out-dir "$SWEEP/repro/fd-patchtst-bs1"
```

The clap value is `patchtst-bs1` (`#[value(name = "patchtst-bs1")]`, `forecast_distribution.rs:64`).
It dispatches to `PatchTstForecaster::load_anchor` (`:815-826`) and switches `FeatureConfig` to
`context_bars: 336, target_horizon_bars: 24` (`:879-890`). Do **not** pass `--metadata-path`: no
PatchTST overlay exists and `is_patchtst()` guards the recal-delta branch (`:987`). Inputs:
`patchtst-bs1-…dd.{safetensors,metadata.json}`. **Measured 404.8 s.**

### R6 — `recalibrate-sigma-train-bs1`  ⚠ writes into `crates/`

```bash
# git status crates/forecast/checkpoints/anchors/  → must be clean BEFORE
cargo run --release -p forecast --features candle --bin recalibrate_sigma_train -- \
  --scenario bs1 \
  --out-dir "$SWEEP/repro/recal-bs1"
# --anchor-dir MUST stay at its default; see § 4.2. Then:
# git diff --exit-code crates/forecast/checkpoints/anchors/
```

Feature: `candle` REQUIRED (refused otherwise). Span is not a CLI argument — it is read from the
checkpoint's own `metadata.data_span` (`:518-534`). **Measured 487.1 s.**

### R7 — `recalibrate-sigma-train-bs2`

As R6 with `--scenario bs2`, `--out-dir "$SWEEP/repro/recal-bs2"`. **Measured 619.8 s.**

### R8 — `threshold-sweep-bs1-realdata-recalibrated`  ⚠ `--out-dir` MANDATORY

```bash
cargo run --release -p backtest --features candle,realdata --bin threshold_sweep -- \
  --scenario bs1 \
  --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json \
  --out-dir "$SWEEP/repro/ts-bs1"
```

`--metadata-path` has no default and is required by clap (`threshold_sweep.rs:148-149`). Omitting
`--out-dir` **overwrites the anchored report** (date-hardcoded filename, § 4.3). Reads the
`forecast-distribution-bs1-realdata-recalibrated-20260521.md` predecessor (§ 4.1) — silently zeroed
if absent. Runs 45 (τ, ε) cells 4-way in rayon plus a baseline and a default cell.
**Measured 428.8 s.**

### R9 — `threshold-sweep-bs2-realdata-recalibrated`

As R8 with `--scenario bs2`, the `tcn-bs2-…1d.metadata.recalibrated.json` overlay, and
`--out-dir "$SWEEP/repro/ts-bs2"`. **Measured 224.6 s.**

### R10 — `vol-verdict-bs1-realdata`

```bash
cargo run --release -p forecast --features candle --bin vol_verdict -- \
  --scenario bs1 \
  --out-dir "$SWEEP/repro/vol-verdict"
```

`--scenario` defaults to `bs1` and `Bs1` is the only variant (`vol_verdict.rs:57-61`). `candle` is
NOT enforced by cargo — pass it anyway, because it is the bin's own declared condition
(`vol_verdict.rs:12`) and § 2.2's numeric-equivalence argument is unverified. The GARCH checkpoint is
found by `garch-bs1-*.json` prefix glob, lexicographically largest wins
(`vol_verdict.rs:82-105`) — one candidate on disk. **Measured 0.7 s** (no NN forward pass; the
front-matter `wall_clock_s` covers the whole run from `:698`). The report's own body records
`n_predictions_total | 76800`, so this is a real full-year × 10-symbol pass, not a truncated one.

### R11 — `regime-verdict-bs1-realdata`

```bash
cargo build --release -p backtest --bin backtest --features candle,realdata   # prerequisite
cargo run --release -p forecast --bin regime_verdict -- \
  --scenario bs1 \
  --backtest-bin target/release/backtest \
  --out-dir "$SWEEP/repro/regime-verdict"
```

Re-runs `top10-2024-fy-regime-dispatcher-realdata` (`regime_verdict.rs:67-71`) into its own
`tempfile::TempDir` via `--reports-dir` (`:821-835`), so the sub-scenario cannot touch `evidence/`.
Do **not** pass `--skip-rerun`: that reads a pre-existing report out of `--out-dir` (`:812-818`) and
is the #93 shape. See § 2.2 for why a wrong-feature `backtest` binary yields a plausible V-REG-1
report instead of an error. **Measured 306.7 s** (matches `determinism.rs:1092`'s independently
measured 270 s for the same sub-scenario in release).

### R12 — `sharpe-comparison-vol-target-bs1-realdata`

```bash
cargo build --release -p backtest --bin backtest --features candle,realdata   # prerequisite
cargo run --release -p forecast --bin sharpe_comparison -- \
  --scenario vol-target-bs1 \
  --backtest-bin target/release/backtest \
  --out-dir "$SWEEP/repro/sc-vol-target"
```

`--scenario vol-target-bs1` (clap name at `sharpe_comparison.rs:57`) selects the arm at `:2247` and
re-runs `["top10-2023-1h-momentum", "top10-2023-fy-vol-target-overlay-realdata"]` (`:2254-2259`) into
a tempdir. `--skip-rerun` is explicitly unimplemented for this family (`:2249-2251`).
**Measured 10.4 s** — the front-matter value covers the two sub-runs, since `t_start` is at `sharpe_comparison.rs:2046`.
Expected to DRIFT (§ 4.4).

### R13 — `sharpe-comparison-vol-target-bs1-realbaseline`

As R12 with `--scenario vol-target-bs1-rebaseline` (clap name at `:61`), arm at `:2176`, sub-scenarios
`["top10-2023-fy-momentum-realdata", "top10-2023-fy-vol-target-overlay-realdata"]` (`:2183-2188`),
default out-dir `evidence/v1/v3-volatility-forecaster-rebaseline/reports/`.
**Measured 10.3 s.** Expected to DRIFT (§ 4.4).

### R14 — `sharpe-comparison-regime-dispatcher-bs1-realdata`

As R12 with `--scenario regime-dispatcher-bs1` (clap name at `:65`), arm at `:2073`, sub-scenarios
`["top10-2023-fy-momentum-realdata", "top10-2023-fy-regime-dispatcher-realdata"]` (`:2081-2086`),
default out-dir `evidence/v1/v3-regime-classifier/reports/`. **Measured 287.1 s.** Reproduction
uncertain (§ 4.4).

### 5.1 Does any invocation emit more than one of the 14?

**No.** Each of the 14 needs its own invocation. Worth stating explicitly because three near-misses
invite the opposite guess:

- `sharpe_comparison` has four families but writes **one** report per run, one `if` arm per family
  (`:2073`, `:2176`, `:2247`, plus the `Tcn` default fall-through at `:2313`). The `Tcn` family emits
  `sharpe-comparison-patchtst-bs1-realdata`, which is **not anchored** (per `anchors.toml:280-282`).
- `forecast_distribution` covers 5 of the 14 but needs 5 runs: `--scenario` × presence of
  `--metadata-path` are independent switches and each combination emits one file.
- `threshold_sweep` runs 45 cells but renders them into **one** report (`:1025-1045`).

---

## 6. Dependency-ordered plan from a clean state

Only one true ordering edge exists (R8/R9 read the R3/R4 report bodies), and it is already satisfied
on disk by committed files whose filenames R8/R9 hardcode. Two builds gate the rest.

```
── Stage 0: builds (no reports emitted) ────────────────────────────────────────
B1  cargo build --release -p forecast --features candle \
      --bin forecast_distribution --bin recalibrate_sigma_train --bin vol_verdict \
      --bin regime_verdict --bin sharpe_comparison
B2  cargo build --release -p backtest --features candle,realdata \
      --bin backtest --bin threshold_sweep
    # B2's `backtest` binary at target/release/backtest is the --backtest-bin
    # prerequisite for R11, R12, R13, R14.

── Stage 1: independent, no report inputs ──────────────────────────────────────
R1  forecast_distribution --scenario bs1                                   [481 s]
R2  forecast_distribution --scenario bs2                                   [510 s]
R5  forecast_distribution --scenario patchtst-bs1                          [405 s]
R10 vol_verdict --scenario bs1                                             [  1 s]
R6  recalibrate_sigma_train --scenario bs1   ⚠ writes crates/…/anchors/    [487 s]
R7  recalibrate_sigma_train --scenario bs2   ⚠ writes crates/…/anchors/    [620 s]

── Stage 2: needs the overlay JSONs that R6/R7 (re)write ───────────────────────
R3  forecast_distribution --scenario bs1 --metadata-path …bs1…recalibrated  [487 s]
R4  forecast_distribution --scenario bs2 --metadata-path …bs2…recalibrated   [488 s]
    # Edge is on the OVERLAY FILE, not on a report. Both overlays are committed
    # and present, so Stage 2 can run without Stage 1's R6/R7 — but if R6/R7 are
    # run, they must precede R3/R4 or the two stages test different inputs.

── Stage 3: needs BOTH an overlay JSON and a Stage-2 REPORT BODY ───────────────
R8  threshold_sweep --scenario bs1 --metadata-path …bs1…recalibrated       [429 s]
R9  threshold_sweep --scenario bs2 --metadata-path …bs2…recalibrated       [225 s]
    # ↑ the only report→report edge in the set. Each reads
    #   evidence/v1/v25-tcn-recalibrate/reports/
    #     forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md
    #   at a HARDCODED path+date (threshold_sweep.rs:106,109).
    # NOTE the asymmetry: R3/R4 re-emitted into a tempdir do NOT feed R8/R9 —
    # the hardcoded 20260521 path still points at the committed 2026-05 body.
    # So R8/R9 always consume the COMMITTED predecessor, whatever R3/R4 produced.
    # Reproducing the anchor therefore requires leaving those two files in place.

── Stage 4: needs target/release/backtest from B2 ──────────────────────────────
R11 regime_verdict --scenario bs1                                          [307 s]
R12 sharpe_comparison --scenario vol-target-bs1                            [ 10 s]
R13 sharpe_comparison --scenario vol-target-bs1-rebaseline                 [ 10 s]
R14 sharpe_comparison --scenario regime-dispatcher-bs1                     [287 s]
```

Run Stage 1–4 **serially**, not in parallel: R8/R9 already saturate cores with a 4-way rayon pool
(`threshold_sweep.rs:644-654`) and the house rule is that parallel cargo makes load-sensitive runs
red.

After every stage: `bash scripts/verify_anchors.sh` must still print `ANCHORS PASS (119 / 119)`. If a
run leaked a report into `evidence/`, the count stays 119 but a row silently re-resolves — so also
check `git status evidence/` and `git status crates/forecast/checkpoints/anchors/`.

### 6.1 Watch recipe for the long runs

Every run in Stages 1–4 except R10/R12/R13 exceeds two minutes; the full sweep is ≈ 1 h 20 m of
compute plus builds. Paste this in a second pane:

```bash
watch -n 30 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading; \
  echo "== emitted =="; \
  find /private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/repro \
       -name "*.md" -newermt "-8 hours" 2>/dev/null | sort; \
  echo "== evidence/ must be clean =="; git status --porcelain evidence/ | head; \
  echo "== checkpoints must be clean =="; git status --porcelain crates/forecast/checkpoints/anchors/ | head; \
  echo "== running =="; pgrep -fl "forecast_distribution|recalibrate_sigma_train|threshold_sweep|vol_verdict|regime_verdict|sharpe_comparison|release/backtest" | head'
```

To hash an emitted report against its anchor, use the repo's own hasher — never a re-implementation
(`anchor-gate-coverage-audit-2026-09-26.md` § 8):

```bash
python3 scripts/hash_report.py "$SWEEP/repro/fd-bs1/forecast-distribution-bs1-realdata-"*.md
```

---

## 7. Cost estimate

Every "measured" figure is the anchored report's own front-matter `wall_clock_s`, written by the
producing run on host `M022517718D` (an Apple-Silicon laptop, same class as the current machine).
Build times are **guesses** — nothing in the repo measures them.

| run | scenario | runtime | basis |
|---|---|---|---|
| B1 | forecast bins, release + `candle` | **10–25 min guess** on a cold `target/`; ~0 warm | guess — pulls `candle-core`/`candle-nn` 0.9 + deps; no measurement in repo |
| B2 | `backtest` + `threshold_sweep`, release + `candle,realdata` | **5–15 min guess** cold; ~0 warm | guess. Note § 2.3: candle is already linked, so adding `--features candle` costs little |
| R1 | `forecast-distribution-bs1-realdata` | **481.0 s** | measured (`…-20260519.md` front-matter) |
| R2 | `forecast-distribution-bs2-realdata` | **509.5 s** | measured |
| R3 | `…-bs1-realdata-recalibrated` | **486.9 s** | measured |
| R4 | `…-bs2-realdata-recalibrated` | **487.6 s** | measured |
| R5 | `…-patchtst-bs1-realdata` | **404.8 s** | measured |
| R6 | `recalibrate-sigma-train-bs1` | **487.1 s** | measured |
| R7 | `recalibrate-sigma-train-bs2` | **619.8 s** | measured |
| R8 | `threshold-sweep-bs1-realdata-recalibrated` | **428.8 s** | measured |
| R9 | `threshold-sweep-bs2-realdata-recalibrated` | **224.6 s** | measured |
| R10 | `vol-verdict-bs1-realdata` | **0.7 s** | measured. Low but consistent: no NN forward pass, and R1's 481 s is ~5 ms × 90 k TCN inferences |
| R11 | `regime-verdict-bs1-realdata` | **306.7 s** | measured; corroborated by `determinism.rs:1092` (270 s release for the same sub-scenario) |
| R12 | `sharpe-comparison-vol-target-bs1-realdata` | **10.4 s** | measured (covers both sub-runs) |
| R13 | `…-vol-target-bs1-realbaseline` | **10.3 s** | measured |
| R14 | `…-regime-dispatcher-bs1-realdata` | **287.1 s** | measured |
| | **total compute, R1–R14** | **≈ 4 746 s ≈ 79 min** | sum of measured values, excluding builds |

Caveats on reusing these numbers: `forecast_distribution` and `recalibrate_sigma_train` time only
their forward-pass loop (`t_start` after checkpoint load), so wall-clock is a little higher;
`vol_verdict`, `regime_verdict` and `sharpe_comparison` time the whole `main`. None includes the
`cargo build`. All were recorded 2026-05-19 … 2026-05-29 on a machine whose parquet cache state is
unknown; a cold page cache on the 240 parquets will add to the first run of each stage.

---

## 8. Verdict for gate-building

**Safely measurable today, with no repo mutation and no code change: 12 of 14.** R1–R5, R8–R14 all
accept a tempdir `--out-dir` (R8/R9 *require* one), read only committed inputs, and refuse rather
than fall back when mis-invoked — except for the two soft spots called out in § 2.2 (`regime_verdict`
swallowing a refusing `backtest` binary) and § 4.1 (`parse_gate_survivors` returning nine zeros on a
missing predecessor). Both are worth asserting around, not blockers.

**Blocked: 2 of 14 — R6 and R7 (`recalibrate-sigma-train-bs{1,2}`).** The overlay path is rendered
into the hashed body (`recalibrate_sigma_train.rs:457`, fed from `:686`), so the only `--anchor-dir`
that reproduces the anchor is the committed checkpoint directory. Either accept a repo-mutating step
with `git diff --exit-code crates/forecast/checkpoints/anchors/` as the assertion, or change the body
to print the overlay path relative to `--anchor-dir` — which is itself a body change and therefore an
ADR-0038 § D6.b re-emission.

---

## 9. Corrections of record — 2026-10-01 (story 1-29 AC2/AC3/AC4/AC7)

This note is a dated derivation. Rather than edit it in place and lose what it said, the four things
that have since changed or turned out wrong are recorded here.

**9.1 — Rows 7 and 8's SHAs in § 1 are STALE.** `threshold-sweep-bs{1,2}-realdata-recalibrated` were
re-emitted on 2026-09-29 under ADR-0038 § D6.b by story 1-30, after `#129`'s silent cross-symbol drop
was repaired. The live values are `924a51bb…` and `f60d3bdd…`; `551cc2ab…` / `755bc380…` are history.
This is why the gates built on this note **read the SHA out of `anchors.toml` at run time** instead of
copying it: a recipe doc goes stale, the corpus does not.

**9.2 — § 8's "Blocked: 2 of 14" is right, and its reasoning was incomplete.** The blocker is not only
that `--anchor-dir` reaches the hashed body. Bug-log `#134`: the sentence it reaches it *through* is
**false**. `recalibrate_sigma_train.rs:457` renders *"- Read-only against `{overlay_path}` original
safetensors"* — and `overlay_path` (`:628`) is the `.metadata.recalibrated.json` the run **wrote** at
`:631`. The body's sole read-only claim names the only file the run created, and calls a JSON overlay
*safetensors*. § 4.2 quoted the line correctly and read it as a provenance-location problem; it is
also a correctness problem, and that is what decides the disposition. Option (a) — *"run it, it
rewrites X, assert X came back identical"* — would certify a body asserting X is read-only. Ruled (b):
fix the sentence, re-emit, then gate. The fix removes the blocker as a side effect, because the READ
path (`:506`) is a hardcoded relative constant that no flag touches.

**9.3 — § 2.2's `vol_verdict` candle-invariance is now MEASURED, not inferred.** The note labelled it
*"a code-reading conclusion … not measured"* and said a gate should assert it. It does:
`anchored_report_reproduction.rs::vol_verdict_is_candle_invariant` runs the bin both ways and compares
body-SHAs. The result is in that test's output, not in this paragraph — which is the point.

**9.4 — § 4.1's `parse_gate_survivors` note undercounted.** It names two failure paths (unreadable
file → nine zeros). There are **three**: a missing section heading, and a *partial* parse that
zero-fills the tail — `[69085, 60339, 51964, 44375, 0, 0, 0, 0, 0]`, four measured values and five
fabricated ones, indistinguishable once rendered. All three now error with their own diagnosis
(bug-log `#128b`, fixed 2026-09-29).
