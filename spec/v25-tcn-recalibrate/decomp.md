---
slug: v25-tcn-recalibrate
phase: M-T1
owner: architect
date: 2026-05-21
status: locked
---

# M-T1 — Architect decomposition (v25-tcn-recalibrate v0.1.0)

> Architect lock for the metadata-only σ_train fix. Operator approved
> Q1-Q5 = analyst defaults on 2026-05-21 via the standing "Autoapprove
> all" directive. This decomposition is **read-only against the 22
> existing anchors**; baseline `bash scripts/verify_anchors.sh` reports
> `ANCHORS PASS (22 / 22)` (literal output captured in tasks.md
> T-AR-3 § baseline).

## 1. Architect-decide resolutions

### T-AR-1 — Design lock

**Decision recorded as `## Design` in `spec/v25-tcn-recalibrate/feature.md`
(appended at end of this M-T1 commit; see tasks.md T-AR-1).**

#### D-AR-1.a — Bin name + location

The recalibration tool ships at
[`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs)
(new file; placeholder paths only in this M-T1 commit). Analyst's
suggested name confirmed because:

- Co-locates with the existing investigation bin family
  [`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
  and [`sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs).
  Cross-grep `crates/forecast/src/bin/` enumerates every read-only
  model-introspection bin in one shot.
- Mirrors the verb-noun shape (`forecast_distribution`,
  `sharpe_comparison`, `recalibrate_sigma_train`) — keeps the family
  scannable.
- No conflict with shipped binary names (verified via
  `grep -rn 'name = "recalibrate' crates/`).

Rejected alternatives:

- `tools/recalibrate_sigma_train` — would require a new bin entry +
  no co-location with the existing family. Net negative.
- `calibrate_metadata` — generic enough to be confused with the broader
  metadata canonicaliser at
  [`crates/forecast/src/provenance.rs`](../../crates/forecast/src/provenance.rs).
  Rejected for ambiguity.

#### D-AR-1.b — CLI surface (5 args, mirrors `forecast_distribution`)

```rust
// crates/forecast/src/bin/recalibrate_sigma_train.rs
#[derive(Parser, Debug)]
#[command(
    name = "recalibrate_sigma_train",
    about = "Re-derive σ_train from a converged-model forward pass (read-only against safetensors + original metadata)",
    long_about = "Loads the anchored checkpoint by --scenario, runs the converged \
                  model forward pass over metadata.data_span, computes σ_train as \
                  std(r_hat) per ADR-0033 § D2.a percentile/total-cmp conventions, \
                  and writes a new .metadata.recalibrated.json overlay file next \
                  to the original. Original .metadata.json and .safetensors files \
                  stay byte-identical."
)]
struct Args {
    /// Which anchored checkpoint to inspect.
    #[arg(long, value_enum)]
    scenario: ScenarioArg,                        // Bs1 | Bs2

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Output directory for the recalibration derivation report.
    #[arg(long, default_value = "spec/v25-tcn-recalibrate/reports/")]
    out_dir: PathBuf,

    /// Target directory for the new .metadata.recalibrated.json file.
    /// Defaults to the checkpoint's own anchor dir, co-located with the
    /// original .metadata.json (which is NOT touched).
    #[arg(long, default_value = "crates/forecast/checkpoints/anchors/")]
    anchor_dir: PathBuf,
}
```

Read-only contract enforced at the type level (mirrors ADR-0033 § D1.c):

- Bin invokes `TcnForecaster::load_anchor` only. No `with_cache()`, no
  `with_strict_replay()`.
- Bin's only writes are to `--out-dir` (the derivation report) and to
  exactly **one** new file under `--anchor-dir`:
  `tcn-<bs1|bs2>-<sha>.metadata.recalibrated.json` (path computed via
  `AnchorScenario::file_prefix()` + `::sha_prefix()`).
- No `--retrain`, `--update-original`, `--write-safetensors` flags.
  CLI surface is the 4 args above (clap denies unknown flags by
  default).

#### D-AR-1.c — Forward-pass span (Q1 = (a))

Recalibration span is read from the **original metadata's `data_span`
field**, NOT from `ScenarioArg::default_span` (which is the
forecast_distribution eval span — for BS-2 this differs from the
training span). Concretely:

- BS-1 → `2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z` (training span,
  matches `forecast_distribution::ScenarioArg::Bs1::default_span` of
  `2023-01-01..2024-01-01` modulo the one-hour boundary, so functionally
  the same window).
- BS-2 → `2023-01-01T00:00:00Z .. 2024-03-31T23:00:00Z` (training span;
  differs from `ScenarioArg::Bs2::default_span` which is
  `2024-01-01..2025-01-01` — the OOS eval window). Recalibration uses
  the training span per analyst Q1=(a).

The data_span parse logic is the canonical
`{YYYY-MM-DD}T{HH:MM:SS}Z` shape from
[ADR-0029 § 2 — Canonicalisation rules § 6](../architecture/adr/0029-tcn-checkpoint-provenance.md)
(second-precision RFC 3339). Re-use `time::OffsetDateTime::parse(...,
&time::format_description::well_known::Rfc3339)` mirroring
[`forecast_distribution.rs:667-671`](../../crates/forecast/src/bin/forecast_distribution.rs).

#### D-AR-1.d — Forward-pass call site

Mirrors [ADR-0033 § D1.b](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d1b--forward-pass-call-site)
verbatim:

```rust
let out = forecaster.forward(&x, /*train=*/ false)?;
let val = out.flatten_all()?.to_vec1::<f32>()?[0];
r_hat_all.push(val);
```

- Calls `TcnForecaster::forward()` directly (the same shipped public
  API forecast_distribution uses; train=false means no dropout, no
  BatchNorm running-mean update — pure inference path).
- Iterates `windows_for_symbol(&args.data_root, sym, span,
  &FeatureConfig::default())` from
  [`crates/forecast/src/features.rs:489`](../../crates/forecast/src/features.rs).
- Symbol order locked to the canonical 10 USDT alphabetical list
  (`ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT,
  LINKUSDT, SOLUSDT, XRPUSDT`) per
  [forecast_distribution.rs:694-697](../../crates/forecast/src/bin/forecast_distribution.rs).
- Buffer is a single `Vec<f32>` (no per-epoch reset bug — the buffer
  is constructed once, filled once, std-computed once, then dropped).

#### D-AR-1.e — σ_train formula (matches existing histogram code)

```rust
let n = r_hat_all.len();
let mu = r_hat_all.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
let var = r_hat_all
    .iter()
    .map(|&x| (x as f64 - mu).powi(2))
    .sum::<f64>() / n as f64;
let sigma_train = (var.sqrt().max(1e-8)) as f32;
```

Identical population-variance formula to
[`train_tcn.rs:733-741`](../../crates/forecast/src/bin/train_tcn.rs)
(the bug site), with the load-bearing difference that `r_hat_all`
contains **only converged-model outputs**, never per-epoch
accumulator garbage. The `1e-8` floor matches the existing training
fallback (preserves the existing semantic of "σ_train cannot be zero
even on a degenerate run"). Reduction uses `f64` intermediates per the
[`forecast_distribution.rs:225-232`](../../crates/forecast/src/bin/forecast_distribution.rs)
precedent (avoids f32 catastrophic cancellation on the ~87,590-sample
sum-of-squares).

#### D-AR-1.f — `.metadata.recalibrated.json` overlay schema

The new file lives at the same dir as the original:

```
crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.recalibrated.json
crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe…metadata.recalibrated.json
```

Body is a **full copy of the original metadata JSON**, with **exactly one
field substituted** (`sigma_train`), and canonicalised via the existing
[`crates/forecast/src/provenance.rs::canonicalise`](../../crates/forecast/src/provenance.rs)
serialiser (ADR-0029 § 2 rules: lexicographic key sort, no whitespace,
no trailing newline). All other fields (`architecture`,
`data_span`, `epochs_trained`, `final_train_loss`, `final_val_loss`,
`model_revision`, `tokenisation`, `training`, `weights_sha256`) copied
verbatim from the original. K5 hard invariant — codified as unit test
at T-D-N5 (see Wave A § T-D-N5).

`sigma_train` itself uses `format!("{:.9}", value)` (9 decimals, matches
[ADR-0033 § D2.a percentile format](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d2a--forecast-distribution-bs12-realdata-yyyymmdd-md))
emitted as a JSON **number** (not a string). The original metadata
emits `sigma_train` as a number (`10.95425033569336` / `6.916285514831543`),
so number-format parity preserves the load-bearing schema invariant
that `TcnForecaster::load_anchor` at
[`tcn.rs:534`](../../crates/forecast/src/tcn.rs) reads via
`.as_f64()`.

**Important nuance**: ADR-0029 § 2 specifies float fields as
**string-encoded** with `format!("{:.6}", x)` for the
`model_revision`-hashed canonical form. But the **on-disk metadata file**
emits `sigma_train` as a JSON number (current originals confirm:
`"sigma_train":10.95425033569336`). The recalibrated file matches the
on-disk shape (JSON number) — NOT the ADR-0029 hashed canonical shape
— because the goal is to be a drop-in replacement readable by the
existing `load_anchor` code path. The ADR-0029 canonicaliser is reused
for **key ordering + whitespace** only; the float serialisation matches
the existing on-disk convention (`serde_json::Number::from_f64` via
the standard serialisation path with 9-decimal precision applied at the
serde::Value construction site).

Architect codifies this nuance in ADR-0035 (see T-AR-2).

#### D-AR-1.g — Loader fallback (additive, optional toggle)

For `forecast_distribution` to consume the recalibrated metadata, the
bin gets **one additive CLI flag**:

```rust
/// Optional path to a .metadata.recalibrated.json overlay. If provided,
/// uses this file's sigma_train + model_revision in place of the
/// anchor's default .metadata.json. The safetensors weights still come
/// from the anchor.
#[arg(long)]
metadata_path: Option<PathBuf>,
```

Default behavior (flag omitted) is **byte-identical** to the existing
shipped path — `TcnForecaster::load_anchor(anchor)`. With the flag
provided, the bin uses
[`TcnForecaster::load_from_paths(safetensors_path, &metadata_path)`](../../crates/forecast/src/tcn.rs)
(shipped public API at `tcn.rs:522`). This is the load-bearing reason
the 22 anchor SHAs stay byte-identical: the flag's default path
preserves the original metadata as the source-of-truth for any
non-recalibrate invocation, so the existing
`spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs{1,2}-realdata-20260519.md`
reports are re-runnable verbatim by simply omitting `--metadata-path`.

Rejected alternatives:

- **Loader fallback inside `load_anchor` itself** (auto-prefer
  `.metadata.recalibrated.json` if present) — rejected. Would break
  the predecessor's anchor SHAs the moment the recalibrated file lands
  on disk; F4-evidence reports cease to be reproducible without
  deleting the overlay file. The explicit CLI flag preserves the
  toggle.
- **Environment variable override** — rejected. Hidden state; harder to
  audit from the report frontmatter.

#### D-AR-1.h — Recalibration-derivation report shape

Per R2, the bin emits a markdown report at:

```
spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs{1,2}-20260521.md
```

Body shape (deterministic, anchor-candidate, mirrors ADR-0033 § D2.a
discipline):

```markdown
---
slug: v25-tcn-recalibrate
scenario: recalibrate-sigma-train-bs1   # or bs2
generated: 2026-05-21T12:34:56Z          # advisory (frontmatter, NOT hashed)
wall_clock_s: 47.3                       # advisory
host: <hostname>                         # advisory
git_commit: <40 hex>                     # advisory
model_revision: d1c3696d…                # 64 hex (carries forward unchanged)
sigma_train_original: 10.954250          # f64, %.6f
sigma_train_recalibrated: 0.018015       # f64, %.9f
data_revision_sha: 3a8b96c4…             # 64 hex from data/binance/REVISION.toml
---

# Recalibration report — BS-1 σ_train

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs1                                            |
| model_revision    | d1c3696d…  (UNCHANGED — weights byte-identical) |
| weights_sha256    | 4ed9064a…  (UNCHANGED)                          |
| Training span     | 2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z   |
| Data revision SHA | 3a8b96c4…                                      |
| Inferences        | 87590                                          |

## Result

| Field                       | Value           |
|-----------------------------|-----------------|
| σ_train (original metadata) | 10.954250       |
| σ_train (recalibrated)      | 0.018015573     |
| Ratio (orig / recal)        | 608.012         |
| r_hat mean                  | 0.000123456     |
| r_hat std                   | 0.018015573     |
| r_hat count                 | 87590           |

## Wire-format contrast

\`\`\`diff
- "sigma_train":10.95425033569336
+ "sigma_train":0.018015573
\`\`\`

(All other 8 metadata fields byte-identical; see § Field invariance.)

## Field invariance — recalibrated overlay vs. original

| Field            | Original | Recalibrated | Match |
|------------------|----------|--------------|-------|
| architecture     | (full obj) | (verbatim copy) | ✓ |
| data_span        | (full obj) | (verbatim copy) | ✓ |
| epochs_trained   | 30       | 30           | ✓ |
| final_train_loss | 1.21676e-5 | 1.21676e-5  | ✓ |
| final_val_loss   | 1.53892e-5 | 1.53892e-5  | ✓ |
| model_revision   | d1c3696d… | d1c3696d…   | ✓ |
| tokenisation     | (full obj) | (verbatim copy) | ✓ |
| training         | (full obj) | (verbatim copy) | ✓ |
| weights_sha256   | 4ed9064a… | 4ed9064a…   | ✓ |
| **sigma_train**  | 10.954250 | 0.018015573 | **CHANGED** |

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…safetensors`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`.
- σ_train formula: `std(r_hat)` per ADR-0033 § D2.a (type-7 quantile
  convention does not apply to std; this is population std with f64
  intermediates and the `1e-8` floor inherited from
  `train_tcn.rs:738`).
- Forward-pass call site: `TcnForecaster::forward(&x, false)` per
  ADR-0033 § D1.b.
- Recalibrated metadata canonicalisation: ADR-0035 § D-AR-1.f (this
  ADR).
```

Floating-point format rules:

| Field family                    | Format               |
|---------------------------------|----------------------|
| `sigma_train_original`          | `%.6f`               |
| `sigma_train_recalibrated`      | `%.9f`               |
| `r_hat` mean / std              | `%.9f`               |
| Ratio (orig / recal)            | `%.3f`               |
| `r_hat` count, `inferences`     | `%d`                 |
| Wire-format JSON literal        | verbatim from canonicaliser |

This report is **anchor-candidate** (analyst defers final
anchor-yes/no to architect; M-FINAL determinism gate decides).

#### D-AR-1.i — Carry-forward of feature.md § Design

The above D-AR-1.a … D-AR-1.h decisions land verbatim as the
`## Design` section of `spec/v25-tcn-recalibrate/feature.md` at
T-AR-1's spec-update tick. This decomp.md is the canonical
architecture reference; feature.md § Design is a cross-pointer back
to here.

### T-AR-2 — ADR-0035 (write it)

**Decision: write ADR-0035** under the title
"Post-training σ_train recalibration via metadata overlay (cross-phase
contract for v2.5 / v2.5a / v2.5b)".

Rationale:

- **Cross-phase reusability.** The per-batch accumulation bug in
  [`train_tcn.rs:606,676-678,733-741`](../../crates/forecast/src/bin/train_tcn.rs)
  is a generic training-loop pattern. The PatchTST + Transformer
  phases (v2.5a, v2.5b — both planned under ADR-0028) will reuse the
  same training scaffold if developers copy the existing
  `train_tcn.rs` shape. ADR-0035 codifies "always recalibrate
  σ_train in a frozen-weights post-training pass" as the
  cross-phase contract, with the per-batch accumulator pattern
  flagged as the negative precedent.
- **Metadata-overlay precedent.** D-AR-1.f's "matches on-disk JSON
  number shape, NOT the ADR-0029 string-encoded canonical shape"
  decision is non-obvious and needs an ADR-level citation. Future
  metadata overlays (e.g. PatchTST σ_train recalibration, ε
  recalibration) inherit this convention.
- **σ_train-not-in-safetensors invariant.** ADR-0029 documents the
  schema but does not assert that σ_train is exclusively a metadata
  field; the test at T-D-N5 codifies the invariant and ADR-0035
  references it as the load-bearing reason the metadata-only fix is
  feasible (Q2 = (a) closure).
- **No supersession.** ADR-0035 does **not** supersede ADR-0033 (the
  F-verdict algorithm stays immutable per Q4 = (a)). ADR-0035 sits
  alongside ADR-0033 + ADR-0029 — they form the v2.5 forecaster
  read-path triad.

ADR-0035 sections to write (target ~150 lines, see ADR template):

- **Context** — cites the F4 verdict, the per-batch accumulation bug
  evidence chain (`train_tcn.rs:606,676-678,733-741`), and the
  inference-time read sites (`tcn.rs:534,937`).
- **Decision** — D1 (recalibrate via frozen-weights post-training
  forward pass; never in-loop). D2 (overlay file path naming +
  on-disk JSON number convention vs. ADR-0029 canonicaliser usage).
  D3 (additive `--metadata-path` flag on consumers; default behavior
  byte-identical). D4 (σ_train-not-in-safetensors invariant codified
  as unit test).
- **Alternatives considered** — (a) in-place metadata rewrite
  (rejected: breaks the predecessor's anchor SHAs); (b) safetensors
  tensor for σ_train (rejected: forces retraining); (c) extend
  ADR-0033 § D3 with F3' branch (rejected per Q4 = (a)).
- **Consequences** — files added by this feature; downstream
  applicability to v2.5a/v2.5b; what breaks if a future training
  loop reintroduces the per-batch accumulator pattern.

ADR-0035 path:
[`spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`](../architecture/adr/0035-tcn-sigma-train-recalibration.md).

**Cross-grep note**: ADR-0035 number is currently free (last accepted
ADR is 0034; cockpit-training-control's draft ADR placeholder, if any,
sits in a different feature folder and doesn't collide). The ADR
registry at `spec/architecture/adr/README.md` is updated at the same
M-T1 commit.

### T-AR-3 — Wave decomposition + T-D rows

See § 3 below.

## 2. Module/file change-map

| Path | Action | Lines (estimate) | Notes |
|------|--------|------------------|-------|
| `crates/forecast/src/bin/recalibrate_sigma_train.rs` | **NEW** | ~350 | New bin, mirrors `forecast_distribution.rs` shape. Re-uses `windows_for_symbol`, `TcnForecaster`, `provenance::canonicalise`. |
| `crates/forecast/Cargo.toml` | **MODIFY** | +5 | `[[bin]]` entry for `recalibrate_sigma_train` (mirror existing `[[bin]] name = "forecast_distribution"` block). No new external dep — all crates already pulled by `forecast_distribution.rs`. |
| `crates/forecast/src/bin/forecast_distribution.rs` | **MODIFY** | +12 | Add `metadata_path: Option<PathBuf>` arg + branch in `main()` to call `TcnForecaster::load_from_paths` when `Some`. Default behavior (flag omitted) is byte-identical to current shipped path. |
| `crates/forecast/tests/recalibrate_sigma_train_readonly.rs` | **NEW** | ~120 | Mirror of `forecast_distribution_bin_readonly.rs`: help-surface assertions + checkpoint mtime guard + assertion that no `.safetensors` or original `.metadata.json` mtime moves. |
| `crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs` | **NEW** | ~80 | K5 unit test: assert the recalibrated overlay file has exactly 9 of 10 fields byte-identical to the original (only `sigma_train` differs). Runs against a small synthetic fixture (no full forward pass needed; the JSON-overlay logic is testable in isolation). |
| `crates/forecast/tests/sigma_train_not_in_safetensors.rs` | **NEW** | ~40 | K2/Q2 unit test: load both anchored safetensors files via `safetensors::SafeTensors::deserialize` and assert no tensor named `sigma_train` / `sigma` / `output_scale`. Parses the safetensors header only; no full load. |
| `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.recalibrated.json` | **NEW** (developer-emitted) | 1 line | Output of Wave A bin run. **Original .metadata.json + .safetensors untouched.** |
| `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe…metadata.recalibrated.json` | **NEW** (developer-emitted) | 1 line | Output of Wave A bin run. |
| `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md` | **NEW** (developer-emitted) | ~80 | Wave A derivation report. |
| `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md` | **NEW** (developer-emitted) | ~80 | Wave A derivation report. |
| `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` | **NEW** (orchestrator-emitted via T-D-N7) | ~150 | Wave B re-run; ADR-0033 § D2.a body shape verbatim + standalone `## Recalibration delta` section. |
| `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md` | **NEW** (orchestrator-emitted via T-D-N7) | ~150 | Wave B re-run. |
| `spec/v25-tcn-recalibrate/feature.md` | **MODIFY** | +120 | Append `## Design` section that cross-points back to this decomp.md. Flip frontmatter `status: in-progress`, `owner: developer`. |
| `spec/v25-tcn-recalibrate/tasks.md` | **MODIFY** | +180 | Tick T-AR-1/T-AR-2/T-AR-3 with file:line + cargo invocation + literal output. Append T-D-N1..T-D-N8 rows + Wave A/B/C/D milestones. Flip frontmatter `status: in-progress`, `owner: developer`. |
| `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md` | **NEW** | ~180 | ADR per T-AR-2. |
| `spec/architecture/adr/README.md` | **MODIFY** | +2 | Registry row for ADR-0035. |
| `spec/trace.toml` | **MODIFY** | +3 | Flip `REQ-V25-TCN-RECALIBRATE-001` `state` `proposed → in-progress`. Populate `arch = [...]` with decomp.md + ADR-0035 + ADR-0033 + ADR-0029. |
| `spec/anchors.toml` | **NO CHANGE in M-T1** | — | Tester adds 2 new rows at M-FINAL T-T-1 (`forecast-distribution-bs{1,2}-realdata-recalibrated` under `v2.6.1-alpha-investigation-recalibrated`). All 22 originals byte-identical. |

**Anchor neutrality** (R7): every NEW file lives at a path that no
existing anchor SHA covers. Every MODIFY file is either spec-only
(`feature.md`, `tasks.md`, `trace.toml`, ADR registry) or
non-anchored code (`crates/forecast/Cargo.toml` adds a `[[bin]]`
entry; `forecast_distribution.rs` gets an **additive** CLI flag that
preserves default behavior byte-for-byte).

## 3. Wave A–D ordered decomposition

```
              Wave A  ─────────────►  Wave B  ─────────────►  Wave C  ─────────────►  Wave D
       (new bin + overlay files)    (re-run + reports)        (anchor lock)         (M-FINAL gate)
              developer                orchestrator              tester                tester
```

### Wave A — `recalibrate_sigma_train` bin (developer)

| Row | Description | File:line | Cargo invocation | Expected literal output |
|-----|-------------|-----------|------------------|--------------------------|
| **T-D-N1** | Bin skeleton + CLI surface (D-AR-1.a, D-AR-1.b). Mirrors `forecast_distribution.rs:42-120`. No forward-pass logic yet. Add `[[bin]]` to `crates/forecast/Cargo.toml`. | `crates/forecast/src/bin/recalibrate_sigma_train.rs:1-120` (new) | `cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --help` | Help text containing `--scenario`, `--data-root`, `--out-dir`, `--anchor-dir`; NO `retrain`/`update`/`write-checkpoint` substrings. |
| **T-D-N2** | data_span parser (D-AR-1.c) + symbol loop + forward-pass collector (D-AR-1.d). Iterates `windows_for_symbol()` over `data_span.start..data_span.end` parsed from the original metadata JSON. | `crates/forecast/src/bin/recalibrate_sigma_train.rs:120-220` (new) | `cargo build -p forecast --features candle --bin recalibrate_sigma_train` | `Compiling forecast …` followed by `Finished … profile [optimized] target(s)` — no warnings. |
| **T-D-N3** | σ_train computation + canonical JSON emitter (D-AR-1.e, D-AR-1.f). Reads original `.metadata.json`, mutates only the `sigma_train` field, writes `.metadata.recalibrated.json` overlay via `provenance::canonicalise`. | `crates/forecast/src/bin/recalibrate_sigma_train.rs:220-310` (new) | `cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --scenario bs1` | `INFO recalibrate_sigma_train: σ_train original=10.954250 recalibrated=<f64> ratio=<f64> wrote=<path>.metadata.recalibrated.json wall_clock_s=<f64>` |
| **T-D-N4** | Derivation report emitter (D-AR-1.h). Markdown body matches the canonical shape; deterministic over 2 runs. | `crates/forecast/src/bin/recalibrate_sigma_train.rs:310-400` (new) | `cargo run -p forecast --features candle --bin recalibrate_sigma_train -- --scenario bs2` | Markdown file at `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md`; second run identical body-SHA (see T-T-1 § Determinism). |
| **T-D-N5** | Field-invariance + read-only enforcement tests (R2 + K5). 1× test: every field except `sigma_train` byte-identical. 1× test: original `.metadata.json` + `.safetensors` mtimes unchanged. 1× test: help surface has no forbidden flags (mirror `forecast_distribution_bin_readonly.rs`). | `crates/forecast/tests/recalibrate_sigma_train_readonly.rs:1-120` + `crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs:1-80` (new) | `cargo test -p forecast --features candle --test recalibrate_sigma_train_readonly --test recalibrate_sigma_train_field_invariance` | `running 3 tests … test result: ok. 3 passed; 0 failed` |
| **T-D-N6** | σ_train-not-in-safetensors invariant test (Q2 closure). Parses both BS-1 + BS-2 safetensors headers via `safetensors::SafeTensors::deserialize`; asserts no tensor name contains `sigma`/`output_scale`. | `crates/forecast/tests/sigma_train_not_in_safetensors.rs:1-40` (new) | `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors` | `running 1 test … test result: ok. 1 passed; 0 failed` |

**Wave A acceptance** = T-D-N1..T-D-N6 ticked + both
`tcn-bs{1,2}-<sha>.metadata.recalibrated.json` files on disk + both
derivation reports under `spec/v25-tcn-recalibrate/reports/` + original
`.metadata.json` + `.safetensors` files byte-identical (M-R2 close).

### Wave B — Re-run `forecast_distribution` + new reports (orchestrator)

| Row | Description | File:line | Cargo invocation | Expected literal output |
|-----|-------------|-----------|------------------|--------------------------|
| **T-D-N7** | Additive `--metadata-path` flag on `forecast_distribution.rs` (D-AR-1.g). Default behavior byte-identical to shipped (asserted via `verify_anchors.sh`). | `crates/forecast/src/bin/forecast_distribution.rs:113-120` (modify, +12 lines) | `cargo run -p forecast --features candle --bin forecast_distribution -- --scenario bs1` (verifies default path) AND `bash scripts/verify_anchors.sh` | First cmd: produces report at `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-<DATE>.md` (re-runnable verbatim of the predecessor 2026-05-19 baseline; body-SHA must match `ef73cb8d…`). Second cmd: `ANCHORS PASS (22 / 22)`. |
| **T-D-N8** | Re-run `forecast_distribution` under recalibrated metadata for both BS-1 + BS-2; emit 2 new reports under `spec/v25-tcn-recalibrate/reports/`. Reports include `## Recalibration delta` section per Q4 = (c). | `crates/forecast/src/bin/forecast_distribution.rs` (re-used; no further code change) | (BS-1) `cargo run -p forecast --features candle --bin forecast_distribution -- --scenario bs1 --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json --out-dir spec/v25-tcn-recalibrate/reports/` AND (BS-2) ditto with bs2 SHA | Two markdown files: `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md` with `verdict: F<N>` in frontmatter (per ADR-0033 § D3 algorithm) AND a `## Recalibration delta` body section diffing gate-survival pre-vs-post recalibration. |

**Recalibration-delta section shape** (Q4 = (c) — appended at the
**end** of the body, AFTER `## Verdict` and BEFORE `## Notes`,
hashed by the anchor):

```markdown
## Recalibration delta

| Metric                                    | Pre-recal (orig σ_train) | Post-recal (new σ_train) |
|-------------------------------------------|--------------------------|--------------------------|
| σ_train                                   | 10.954250                | 0.018015573              |
| frac_passes_confidence_gate (τ=0.6)       | 0.000000                 | 0.<6 decimals>           |
| confidence_gate_survival[τ=0.1]           | 0.000000                 | 0.<6 decimals>           |
| confidence_gate_survival[τ=0.5]           | 0.000000                 | 0.<6 decimals>           |
| confidence_gate_survival[τ=0.9]           | 0.000000                 | 0.<6 decimals>           |
| F-verdict                                 | F4                       | F<N>                     |

Pre-recal values are read from the predecessor's anchored report
`forecast-distribution-bs1-realdata-20260519.md` (body-SHA
`ef73cb8d65c1aad8bdcaf1b541f142f02000fbb26d19427899abd4d77b216d54`,
locked in `spec/anchors.toml` row 156).

Routing decision per analyst Q4 default: operator routes on the
**joint signal** (F-verdict + recalibration delta), NOT the F-verdict
alone.
```

The pre-recal values are **read directly from the predecessor's anchored
report body**, not re-computed (the predecessor anchors are
byte-immutable and citeable). The developer wires this via a small
helper `read_predecessor_gate_stats(path)` that grep-parses the
`Confidence-gate survival` table from the anchored body — implementation
detail at T-D-N8.

**Wave B acceptance** = T-D-N7..T-D-N8 ticked + both
`forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md` files
on disk + verify_anchors still `22/22` (no new anchors yet; Wave C
locks them).

### Wave C — Anchor lock (tester)

| Row | Description | Path | Cargo / shell invocation | Expected literal output |
|-----|-------------|------|--------------------------|--------------------------|
| **T-T-1.a** | 2-run determinism check on BOTH recalibrated forecast-distribution reports + recalibrate-derivation reports. | `spec/v25-tcn-recalibrate/reports/{forecast-distribution-bs{1,2}-realdata-recalibrated,recalibrate-sigma-train-bs{1,2}}-20260521.md` | `bash scripts/hash_report.py spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` (run twice; SHAs must match across runs) | `<64hex>  forecast-distribution-bs1-realdata-recalibrated-20260521.md` literal SHA stable. |
| **T-T-1.b** | Anchor-additive lock: append 2 rows to `spec/anchors.toml` under version `v2.6.1-alpha-investigation-recalibrated`. Optionally add 2 more rows for the recalibrate-derivation reports if T-T-1.a passes for those too. | `spec/anchors.toml:170-175` (append) | `bash scripts/verify_anchors.sh` | `ANCHORS PASS (24 / 24)` (or `26 / 26` if recalibrate-derivation reports also anchor). |
| **T-T-1.c** | Anchor-neutrality check: all 22 originals byte-identical to baseline. | `spec/anchors.toml` rows 1-168 (unchanged) | `bash scripts/verify_anchors.sh \| head -22` | All 22 lines `PASS  <scenario>  <sha>` literal-identical to the baseline captured in tasks.md T-AR-3 § baseline. |

**Wave C acceptance** = T-T-1.a..T-T-1.c ticked + new anchor rows in
`spec/anchors.toml` + 22 originals byte-identical (M-R3 close).

### Wave D — M-FINAL handoff prep (tester)

| Row | Description | Path | Cargo / shell invocation | Expected literal output |
|-----|-------------|------|--------------------------|--------------------------|
| **T-T-1.d** | F-verdict re-classification recorded: joint label per ADR-0033 § D3 table. Routing decision recorded in `feature.md § Verification`. | `spec/v25-tcn-recalibrate/feature.md § Verification` (append) | (manual) | `Joint verdict: F<N>` + `Operator disposition: <decision>` lines added to feature.md. |
| **T-T-1.e** | Trace row flipped to `shipped`. `crates`, `tests`, `anchors` columns populated. | `spec/trace.toml:194-204` | `python scripts/spec_brief.py v25-tcn-recalibrate --check-trace` (or manual `grep`) | `state = "shipped"` + non-empty arrays. |
| **T-T-1.f** | Tester report under `spec/v25-tcn-recalibrate/reports/test-<YYYYMMDD-HHMM>-v25-tcn-recalibrate.md` per `.claude/skills/rust-test/templates/test-report.md`. Carries the 22/22 → 24/24 (or 26/26) anchor-progression line literal. | `spec/v25-tcn-recalibrate/reports/test-<YYYYMMDD-HHMM>-v25-tcn-recalibrate.md` (new) | `bash scripts/verify_anchors.sh` (quote literal in report body) | Tester report cites `ANCHORS PASS (24 / 24)` (or `26 / 26`) as the post-lock literal. |

**Wave D acceptance** = T-T-1.d..T-T-1.f ticked + tester emits
`VERDICT → PASS` envelope to presenter (M-FINAL close).

### Parallelism map

```
Wave A:  T-D-N1 → T-D-N2 → T-D-N3 → T-D-N4  (sequential; each row depends on the prior)
                                ↘
                                  T-D-N5  (parallel-after-T-D-N3 — tests the JSON-overlay logic in isolation)
                                  T-D-N6  (parallel-after-T-D-N1 — tests safetensors independent of forward pass)

Wave B:  T-D-N7 (sequential after Wave A; this is the load-bearing CLI-flag change)
         T-D-N8 (BS-1 + BS-2 invocations independent; parallel)

Wave C:  T-T-1.a (BS-1 + BS-2 + recalibrate-derivation all parallel; 4-fan-out)
         T-T-1.b → T-T-1.c (sequential after T-T-1.a)

Wave D:  T-T-1.d → T-T-1.e → T-T-1.f (sequential)
```

Critical path: **T-D-N1 → T-D-N2 → T-D-N3 → T-D-N4 → T-D-N7 → T-D-N8 →
T-T-1.b → T-T-1.f.** Wall-clock estimate ~3-4 hours (analyst's
feature.md § Cost estimate budget of 4-5h confirmed).

### Watch recipe for the long-running rows

T-D-N3 + T-D-N4 + T-D-N8 invocations are each ~8 min (~87,590 forward
passes per scenario). The orchestrator should kick each off in
background and probe via:

```bash
watch -n 30 'tail -n 40 /tmp/recalibrate-bs1.log; \
             ls -la crates/forecast/checkpoints/anchors/*recalibrated* 2>/dev/null; \
             ls -la spec/v25-tcn-recalibrate/reports/ 2>/dev/null'
```

## 4. Spike requirement

**NONE.** The analyst pinned the bug to exact file:line at T-A1
(`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`) and the
inference-time read sites (`tcn.rs:534,937`). The shipped public APIs
needed by the bin are all anchored
([`load_anchor`](../../crates/forecast/src/tcn.rs),
[`load_from_paths`](../../crates/forecast/src/tcn.rs),
[`forward`](../../crates/forecast/src/tcn.rs),
[`windows_for_symbol`](../../crates/forecast/src/features.rs),
[`canonicalise`](../../crates/forecast/src/provenance.rs)). The bin is
a copy-paste-and-adapt of the existing `forecast_distribution.rs`
scaffold + a 90-line metadata-overlay emitter. No experimental crate
selection (zero new deps).

If the developer hits an unexpected at T-D-N3 (e.g. the recalibrated
σ_train lands outside the 0.005–0.025 range — H1 falsification),
**escalate back to analyst, do NOT band-aid in Wave A**. The H1
falsification path is documented in `feature.md § Hypothesis register §
H1`.

## 5. Rollback shape per wave

Every wave is independently revertable. The overlay-file convention
(D-AR-1.f, D-AR-1.g) is the load-bearing reason this is true:

| Wave | Rollback action | Cost |
|------|-----------------|------|
| **A** | Delete the 2 `.metadata.recalibrated.json` files + 2 derivation reports + revert the new bin source. Original `.metadata.json` + `.safetensors` files were never touched → byte-identical. | ~1 minute (`rm` + `git revert <sha>`). |
| **B** | Delete the 2 `forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md` reports + revert the `--metadata-path` CLI flag on `forecast_distribution.rs`. With Wave A still in place, the recalibrated metadata files persist but are not auto-consumed (the default `load_anchor` path stays the source of truth) — F4-evidence reports are still re-runnable verbatim. | ~5 minutes (`rm` + `git revert <sha>`). |
| **C** | Revert the 2 (or 4) new rows in `spec/anchors.toml`. The 22 originals were byte-identical the whole time → no migration. | ~2 minutes (`git revert <sha>`). |
| **D** | Revert `feature.md § Verification` + `trace.toml` state-flip. Tester report stays on disk as a historical artifact. | ~2 minutes. |

**Full-feature rollback** = `git revert` the 4 commits + `rm` the 4
artifact files. Original 22 anchors stay byte-identical throughout
(R7 hard invariant).

## 6. Anchor neutrality baseline

The M-T1 baseline captured at architect-spawn time:

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS PASS  (22 / 22)
```

(Full literal output of all 22 PASS lines is preserved in tasks.md
T-AR-3 § baseline — the literal `ANCHORS PASS  (22 / 22)` line plus
the 22 individual scenario rows.)

## 7. Cross-references

- Analyst feature brief:
  [`spec/v25-tcn-recalibrate/feature.md`](feature.md)
- Predecessor F4 verdict:
  [`spec/v25-tcn-alpha-investigation/feature.md`](../v25-tcn-alpha-investigation/feature.md),
  [presenter deck](../v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md)
- F-verdict algorithm (immutable per Q4):
  [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
- Metadata canonicaliser:
  [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md),
  [`crates/forecast/src/provenance.rs`](../../crates/forecast/src/provenance.rs)
- New ADR (T-AR-2):
  [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
- Bug site:
  [`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`](../../crates/forecast/src/bin/train_tcn.rs)
- Inference read sites:
  [`crates/forecast/src/tcn.rs:534,937`](../../crates/forecast/src/tcn.rs)
- Trace row: `REQ-V25-TCN-RECALIBRATE-001` in
  [`spec/trace.toml`](../trace.toml).

## Changelog

- 2026-05-21 (architect, M-T1): full lock. T-AR-1 (Design),
  T-AR-2 (ADR-0035 = WRITE), T-AR-3 (8 T-D rows across Waves A-D).
  Anchor-additive contract confirmed (22 originals byte-identical);
  recalibrated metadata uses overlay-file convention; F-verdict
  algorithm stays immutable per Q4 = (a); recalibration delta surfaces
  as standalone body section per Q4 = (c). HANDOFF → developer
  (Wave A first; Wave B post-A; Waves C-D tester at M-FINAL).
