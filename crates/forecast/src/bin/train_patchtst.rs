//! `train_patchtst` — PatchTST training binary (v2.5a Wave B).
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p forecast --release --features candle --bin train_patchtst -- \
//!   --scenario bs1 \
//!   --target-horizon-bars 24 \
//!   --span-start 2023-01-01 \
//!   --span-end 2023-12-31 \
//!   --patch-len 16 --stride 8 \
//!   --d-model 128 --n-heads 4 --d-ff 256 --n-layers 3 --dropout 0.2 \
//!   --context-len 336 \
//!   --epochs 30 --batch-size 128 \
//!   --seed 0x00C0FFEE
//!
//! # 1-epoch smoke run (T-D-N11):
//! cargo run -p forecast --release --features candle --bin train_patchtst -- \
//!   --scenario bs1 --epochs 1 --batch-size 4 \
//!   --span-start 2023-01-01 --span-end 2023-01-07 --seed 0x00C0FFEE
//! ```
//!
//! ## Watch recipe (per MEMORY.md — emit when kicking off Wave B training)
//!
//! ```bash
//! watch -n 60 '
//! PID=$(pgrep -f train_patchtst | head -1)
//! [ -z "$PID" ] && echo "train_patchtst not running" && exit
//! N=$(grep -c "epoch complete" /tmp/train_patchtst-bs1.log 2>/dev/null || echo 0)
//! LAST=$(grep "epoch complete" /tmp/train_patchtst-bs1.log 2>/dev/null | tail -1 | grep -oE "epoch=[0-9]+" | cut -d= -f2 || echo 0)
//! ELAPSED=$(ps -o etime= -p $PID 2>/dev/null | awk "{gsub(/^ +/,\"\"); n=split(\$0,a,/[-:]/); if(n==2)print a[1]*60+a[2]; else if(n==3)print a[1]*3600+a[2]*60+a[3]; else if(n==4)print a[1]*86400+a[2]*3600+a[3]*60+a[4]}")
//! [ "$N" -gt 0 ] && echo "epoch $LAST/30 ($((N*100/30))%), elapsed ${ELAPSED}s, remaining ~$(((30-N)*ELAPSED/N/60)) min" || echo "warmup: 0 epochs (elapsed=${ELAPSED}s)"
//! '
//! ```
//!
//! ## σ_train contract (ADR-0035 § D1 + ADR-0036 § D3)
//!
//! σ_train is computed via a **post-training frozen-weights forward pass** over
//! the training-data span — NOT via an in-loop accumulator across training epochs.
//!
//! The deprecated in-loop accumulator at `train_tcn.rs:606,676-678,733-741`
//! is the canonical **negative precedent** (per ADR-0035 § Negative precedent).
//! This file deliberately does NOT replicate that pattern.
//!
//! ## Cost tripwire (ADR-0036 § D4)
//!
//! `assert_epoch_budget(epoch_n, wall_clock_sec, history)` fires if:
//! - Single epoch > 24 h (hard limit)
//! - Epoch N > 3× rolling median of epochs 1..N-1
//!
//! On fire: log error + write `/tmp/train_patchtst-bs1-tripwire-epoch{N}.txt` +
//! emit `train_events` row with kind = "tripwire_warning" + **continue** training.
//!
//! ## Determinism
//!
//! - Seed: `--seed` CLI arg → `ChaCha20Rng::from_seed` (no SystemTime/OsRng).
//! - `Instant::now()` is used ONLY for wall-clock audit fields (not model bytes).
//! - Two runs with the same seed + data produce byte-identical safetensors.
//!
//! ## Cross-references
//!
//! - `spec/v1/v25a-patchtst-overlay/feature.md § R2` — CLI spec
//! - `spec/architecture/adr/0036-patchtst-training-contract.md § D3,D4,D5`
//! - `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md § D1`
//! - `crates/forecast/src/patchtst.rs` — model definition

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tracing::{error, info, warn};
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).
use uuid::Uuid;

use forecast::{
    features::{FeatureConfig, FeatureWindow, TimeSpan, windows_for_symbol},
    patchtst::{CHANNELS, PatchTstModel},
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "train_patchtst",
    about = "Train the PatchTST forecaster (v2.5a)",
    long_about = "Train a PatchTST (Patch Time Series Transformer) on OHLCV parquet data.\n\
                  Mirrors train_tcn.rs but with post-training σ_train derivation (ADR-0035 § D1).\n\
                  NO in-loop σ_train accumulator — see ADR-0036 § D3 for rationale."
)]
struct Cli {
    /// Scenario name for checkpoint file prefix (e.g. "bs1").
    #[arg(long, default_value = "bs1")]
    scenario: String,

    /// Parquet data root directory.
    #[arg(long, default_value = "data/binance")]
    data_root: PathBuf,

    /// Output directory for checkpoint files.
    #[arg(long, default_value = "crates/forecast/checkpoints/anchors")]
    out_dir: PathBuf,

    /// Training span start (ISO-8601 date, e.g. "2023-01-01").
    #[arg(long, default_value = "2023-01-01")]
    span_start: String,

    /// Training span end (ISO-8601 date, e.g. "2023-12-31").
    #[arg(long, default_value = "2023-12-31")]
    span_end: String,

    /// Validation span start (ISO-8601 date).
    /// Defaults to None (no validation set); when absent, val loss is NaN.
    #[arg(long)]
    val_start: Option<String>,

    /// Validation span end (ISO-8601 date).
    #[arg(long)]
    val_end: Option<String>,

    /// Comma-separated symbol list.
    #[arg(
        long,
        default_value = "ADAUSDT,AVAXUSDT,BNBUSDT,BTCUSDT,DOGEUSDT,DOTUSDT,ETHUSDT,LINKUSDT,SOLUSDT,XRPUSDT"
    )]
    symbols: String,

    /// Number of training epochs.
    #[arg(long, default_value_t = 30)]
    epochs: u32,

    /// Mini-batch size.
    #[arg(long, default_value_t = 128)]
    batch_size: usize,

    /// Maximum learning rate (OneCycle peak).
    #[arg(long, default_value_t = 1e-3)]
    lr_max: f64,

    /// Huber loss δ.
    #[arg(long, default_value_t = 1e-3)]
    huber_delta: f64,

    /// RNG seed (determinism contract).
    #[arg(long, value_parser = parse_hex_or_dec_u64, default_value = "12648430")]
    seed: u64,

    /// Target horizon in bars (24 for PatchTST 24h, 1 for TCN compat).
    #[arg(long, default_value_t = 24)]
    target_horizon_bars: usize,

    /// Patch length.
    #[arg(long, default_value_t = 16)]
    patch_len: usize,

    /// Patch stride.
    #[arg(long, default_value_t = 8)]
    stride: usize,

    /// Context length in bars (336 = ~14 days hourly).
    #[arg(long, default_value_t = 336)]
    context_len: usize,

    /// Model dimension.
    #[arg(long, default_value_t = 128)]
    d_model: usize,

    /// Number of attention heads.
    #[arg(long, default_value_t = 4)]
    n_heads: usize,

    /// Feed-forward dimension.
    #[arg(long, default_value_t = 256)]
    d_ff: usize,

    /// Number of encoder layers.
    #[arg(long, default_value_t = 3)]
    n_layers: usize,

    /// Dropout rate.
    #[arg(long, default_value_t = 0.2)]
    dropout: f64,

    /// Path to audit SQLite database for training-event emission (ADR-0034).
    /// When omitted, no SQLite handle is opened.
    #[arg(long)]
    audit_db: Option<PathBuf>,
}

/// Parse hex (0x…) or decimal u64.
fn parse_hex_or_dec_u64(s: &str) -> std::result::Result<u64, String> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        s.parse::<u64>().map_err(|e| e.to_string())
    }
}

// ── CostTripwireError (ADR-0036 § D4 + T-AR-8) ───────────────────────────────

/// Error emitted by `assert_epoch_budget` (ADR-0036 § D4).
///
/// On fire the bin logs an error, writes a diagnostic file, and **continues**
/// training (the operator owns the stop/continue decision).
#[derive(Debug, thiserror::Error)]
pub enum CostTripwireError {
    #[error("epoch {epoch} wall-clock {wall_clock_sec}s exceeds 24h hard limit")]
    HardLimit { epoch: usize, wall_clock_sec: u64 },
    #[error("epoch {epoch} wall-clock {wall_clock_sec}s exceeds 3× rolling median {median}s")]
    MedianMultiple {
        epoch: usize,
        wall_clock_sec: u64,
        median: u64,
    },
}

/// Assert that a single epoch's wall-clock is within budget (ADR-0036 § D4).
///
/// - `HARD_LIMIT_SEC = 24h`: any epoch exceeding this fires immediately.
/// - `3× median`: if epoch N > 3× median of epochs 1..N-1, fires.
///
/// **Returns `Err` on fire; caller logs + writes diagnostic + continues.**
pub fn assert_epoch_budget(
    epoch_n: usize,
    epoch_wall_clock_sec: u64,
    history: &[u64],
) -> std::result::Result<(), CostTripwireError> {
    const HARD_LIMIT_SEC: u64 = 24 * 3_600; // 24 h per ADR-0036 § D4

    if epoch_wall_clock_sec > HARD_LIMIT_SEC {
        return Err(CostTripwireError::HardLimit {
            epoch: epoch_n,
            wall_clock_sec: epoch_wall_clock_sec,
        });
    }

    if epoch_n > 0 && !history.is_empty() {
        let mut sorted = history.to_vec();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        if median > 0 && epoch_wall_clock_sec > 3 * median {
            return Err(CostTripwireError::MedianMultiple {
                epoch: epoch_n,
                wall_clock_sec: epoch_wall_clock_sec,
                median,
            });
        }
    }

    Ok(())
}

// ── Huber loss ────────────────────────────────────────────────────────────────

fn huber_loss(pred: &Tensor, target: &Tensor, delta: f32) -> candle_core::Result<Tensor> {
    let diff = (pred - target)?;
    let abs_diff = diff.abs()?;
    let delta_t = Tensor::full(delta, pred.shape(), pred.device())?;
    let quadratic = (diff.sqr()? * 0.5_f64)?;
    let linear = ((abs_diff.clone() - (&delta_t * 0.5_f64)?)? * delta as f64)?;
    let mask = abs_diff.lt(&delta_t)?;
    let loss = mask.where_cond(&quadratic, &linear)?;
    loss.mean_all()
}

// ── OneCycle LR ───────────────────────────────────────────────────────────────

fn onecycle_lr(step: usize, total_steps: usize, lr_max: f64, pct_start: f64) -> f64 {
    let warmup = (total_steps as f64 * pct_start) as usize;
    if step < warmup {
        lr_max * (step + 1) as f64 / warmup as f64
    } else {
        let cos_progress = (step - warmup) as f64 / (total_steps - warmup).max(1) as f64;
        let cos = (std::f64::consts::PI * cos_progress).cos();
        lr_max * 0.5 * (1.0 + cos)
    }
}

// ── Date helpers ──────────────────────────────────────────────────────────────

fn normalise_date_start(s: &str) -> String {
    if s.len() == 10 {
        format!("{s}T00:00:00Z")
    } else {
        s.to_string()
    }
}

fn normalise_date_end(s: &str) -> String {
    if s.len() == 10 {
        format!("{s}T23:00:00Z")
    } else {
        s.to_string()
    }
}

fn parse_ts(s: &str) -> Result<time::OffsetDateTime> {
    let format = time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(s, &format).with_context(|| format!("invalid timestamp: {s}"))
}

// ── Audit-DB writer (ADR-0034) ────────────────────────────────────────────────

/// Lazily opened audit-DB writer for `train_patchtst` instrumentation.
///
/// Mirrors `train_tcn.rs`'s `AuditWriter` pattern verbatim.
/// When `--audit-db` is absent, every method is a no-op (zero overhead).
struct AuditWriter {
    inner: Option<AuditInner>,
    model_family: &'static str,
}

struct AuditInner {
    rt: tokio::runtime::Runtime,
    ledger: audit::Ledger,
}

impl AuditWriter {
    fn disabled() -> Self {
        Self {
            inner: None,
            model_family: "patchtst",
        }
    }

    fn open(path: &Path) -> Result<Self, String> {
        let db_path = path
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 audit-db path: {}", path.display()))?
            .to_owned();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("tokio runtime: {e}"))?;

        let ledger = rt
            .block_on(audit::Ledger::open(&db_path))
            .map_err(|e| format!("ledger open at {db_path}: {e}"))?;

        Ok(Self {
            inner: Some(AuditInner { rt, ledger }),
            model_family: "patchtst",
        })
    }

    fn write_start(&self, run_id: &str, scenario: &str, seed: i64) {
        let Some(AuditInner { rt, ledger }) = &self.inner else {
            return;
        };
        let pid = std::process::id() as i64;
        // Prefix scenario with model_family for cockpit surfacing.
        let labelled_scenario = format!("{}:{}", self.model_family, scenario);
        if let Err(e) = rt.block_on(audit::journal::post_training_start(
            ledger,
            run_id,
            &labelled_scenario,
            seed,
            Some(pid),
        )) {
            warn!(run_id, %e, "audit write_start failed (non-fatal)");
        }
    }

    fn write_epoch(
        &self,
        run_id: &str,
        epoch: i64,
        total_epochs: i64,
        train_loss: f32,
        val_loss: f32,
        wall_clock_ms: i64,
    ) {
        let Some(AuditInner { rt, ledger }) = &self.inner else {
            return;
        };
        if let Err(e) = rt.block_on(audit::journal::post_training_epoch(
            ledger,
            run_id,
            epoch,
            total_epochs,
            train_loss,
            val_loss,
            wall_clock_ms,
        )) {
            warn!(run_id, epoch, %e, "audit write_epoch failed (non-fatal)");
        }
    }

    fn write_finish(
        &self,
        run_id: &str,
        model_revision: &str,
        final_train_loss: f32,
        final_val_loss: f32,
        total_wall_clock_ms: i64,
    ) {
        let Some(AuditInner { rt, ledger }) = &self.inner else {
            return;
        };
        if let Err(e) = rt.block_on(audit::journal::post_training_finish(
            ledger,
            run_id,
            model_revision,
            final_train_loss,
            final_val_loss,
            total_wall_clock_ms,
        )) {
            warn!(run_id, model_revision, %e, "audit write_finish failed (non-fatal)");
        }
    }

    fn write_failed(&self, run_id: &str, error_message: &str) {
        let Some(AuditInner { rt, ledger }) = &self.inner else {
            return;
        };
        if let Err(e) = rt.block_on(audit::journal::post_training_failed(
            ledger,
            run_id,
            error_message,
        )) {
            warn!(run_id, %e, "audit write_failed failed (non-fatal)");
        }
    }
}

// ── Validation loss ───────────────────────────────────────────────────────────

fn compute_val_loss(
    model: &PatchTstModel,
    val_windows: &[FeatureWindow],
    device: &Device,
    context: usize,
    batch_size: usize,
    delta: f32,
) -> f32 {
    if val_windows.is_empty() {
        return f32::NAN;
    }
    let mut total_loss = 0.0_f32;
    let mut n_batches = 0usize;

    for batch_start in (0..val_windows.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(val_windows.len());
        if batch_end - batch_start < 1 {
            break;
        }
        let actual_batch = batch_end - batch_start;

        let mut feat_data: Vec<f32> = Vec::with_capacity(actual_batch * CHANNELS * context);
        let mut target_data: Vec<f32> = Vec::with_capacity(actual_batch);

        for w in &val_windows[batch_start..batch_end] {
            let flat: Vec<f32> = w
                .features
                .flatten_all()
                .and_then(|t| t.to_vec1::<f32>())
                .unwrap_or_default();
            // flat is [context, CHANNELS] row-major — transpose to [CHANNELS, context].
            for c in 0..CHANNELS {
                for t in 0..context {
                    let val = flat.get(t * CHANNELS + c).copied().unwrap_or(0.0);
                    feat_data.push(val);
                }
            }
            target_data.push(w.target_logret);
        }

        let x = match Tensor::from_vec(feat_data, (actual_batch, CHANNELS, context), device) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let y = match Tensor::from_vec(target_data, (actual_batch, 1), device) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if let Ok(pred) = model.forward(&x, false)
            && let Ok(loss) = huber_loss(&pred, &y, delta)
        {
            total_loss += loss.to_scalar::<f32>().unwrap_or(0.0);
            n_batches += 1;
        }
    }

    if n_batches > 0 {
        total_loss / n_batches as f32
    } else {
        f32::NAN
    }
}

// ── σ_train post-training derivation (ADR-0035 § D1 + ADR-0036 § D3) ────────

/// Compute σ_train via a frozen-weights forward pass over the training span.
///
/// **ADR contract:** this is called AFTER the training loop completes, with
/// frozen model weights. The `Vec<f32>` accumulator is declared INSIDE this
/// function's scope — NOT outside the per-epoch training loop.
///
/// Two runs with the same checkpoint + same data produce an identical σ_train
/// scalar (the frozen-pass is deterministic on CPU).
fn compute_sigma_train_post_training(
    model: &PatchTstModel,
    train_windows: &[FeatureWindow],
    device: &Device,
    context: usize,
    batch_size: usize,
) -> f32 {
    // The Vec<f32> is declared INSIDE the post-training block scope (ADR-0036 § D3).
    // This is the canonical correct location — NOT outside the per-epoch loop.
    let mut r_hats: Vec<f32> = Vec::with_capacity(train_windows.len());

    for batch_start in (0..train_windows.len()).step_by(batch_size.max(1)) {
        let batch_end = (batch_start + batch_size).min(train_windows.len());
        if batch_end <= batch_start {
            break;
        }
        let actual_batch = batch_end - batch_start;

        let mut feat_data: Vec<f32> = Vec::with_capacity(actual_batch * CHANNELS * context);

        for w in &train_windows[batch_start..batch_end] {
            let flat: Vec<f32> = w
                .features
                .flatten_all()
                .and_then(|t| t.to_vec1::<f32>())
                .unwrap_or_default();
            for c in 0..CHANNELS {
                for t in 0..context {
                    let val = flat.get(t * CHANNELS + c).copied().unwrap_or(0.0);
                    feat_data.push(val);
                }
            }
        }

        let x = match Tensor::from_vec(feat_data, (actual_batch, CHANNELS, context), device) {
            Ok(t) => t,
            Err(e) => {
                warn!(%e, "sigma_train batch tensor build failed (skipping batch)");
                continue;
            }
        };

        // Frozen forward pass (train=false → no dropout).
        match model.forward(&x, false) {
            Ok(pred) => {
                if let Ok(v) = pred.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
                    r_hats.extend_from_slice(&v);
                }
            }
            Err(e) => {
                warn!(%e, "sigma_train batch forward failed (skipping batch)");
            }
        }
    }

    // Compute population std with f64 intermediates + 1e-8 floor (ADR-0035 § D1).
    if r_hats.len() > 1 {
        let n = r_hats.len() as f64;
        let mu = r_hats.iter().map(|&x| x as f64).sum::<f64>() / n;
        let var = r_hats.iter().map(|&x| (x as f64 - mu).powi(2)).sum::<f64>() / n;
        (var.sqrt().max(1e-8)) as f32
    } else {
        1.0_f32
    }
}

// ── Checkpoint write ──────────────────────────────────────────────────────────

struct PatchTstCheckpointMetrics {
    sigma_train: f32,
    final_train_loss: f32,
    final_val_loss: f32,
    epochs_trained: u32,
    scenario: String,
    // Architecture fields for provenance schema (ADR-0036 § D2).
    patch_len: usize,
    stride: usize,
    d_model: usize,
    n_heads: usize,
    d_ff: usize,
    n_layers: usize,
    dropout: f64,
    context_len: usize,
    target_horizon_bars: usize,
    lr_max: f64,
    huber_delta: f64,
    batch: usize,
    epochs: u32,
    seed: u64,
    span_start: String,
    span_end: String,
    symbols: Vec<String>,
}

/// Write checkpoint files (safetensors + metadata JSON) and return `model_revision` SHA.
fn write_checkpoint(
    varmap: &VarMap,
    out_dir: &Path,
    metrics: PatchTstCheckpointMetrics,
) -> Result<String> {
    use serde_json::{Value, json};

    // Write safetensors to temp file.
    let temp_path = out_dir.join("_tmp_patchtst_checkpoint.safetensors");
    varmap.save(&temp_path).context("saving safetensors")?;

    let weights_bytes = std::fs::read(&temp_path).context("reading temp safetensors")?;
    let w_sha = forecast::provenance::weights_sha256(&weights_bytes);

    // Build ADR-0036 § D2 provenance JSON.
    // Keys sorted lexicographically per ADR-0029 canonicalisation.
    let meta_value = json!({
        "architecture": {
            "context_len": metrics.context_len,
            "d_ff": metrics.d_ff,
            "d_model": metrics.d_model,
            "dropout": format!("{:.6}", metrics.dropout),
            "model_family": "patchtst",
            "n_heads": metrics.n_heads,
            "n_layers": metrics.n_layers,
            "patch_len": metrics.patch_len,
            "stride": metrics.stride
        },
        "data_span": {
            "end": metrics.span_end,
            "interval": "1h",
            "source": "binance",
            "start": metrics.span_start,
            "symbols": metrics.symbols
        },
        "metrics": {
            "epochs_run": metrics.epochs_trained,
            "final_train_huber": format!("{:.6}", metrics.final_train_loss),
            "final_val_huber": format!("{:.6}", metrics.final_val_loss)
        },
        "model_revision": "",
        "sigma_train": metrics.sigma_train,
        "tokenisation": {
            "context_bars": metrics.context_len,
            "features": ["logret","logrange","logvol_z","hour_sin","hour_cos"],
            "target_horizon_bars": metrics.target_horizon_bars
        },
        "training": {
            "batch": metrics.batch,
            "epochs": metrics.epochs,
            "huber_delta": format!("{:.6}", metrics.huber_delta),
            "loss": "huber",
            "lr_max": format!("{:.6}", metrics.lr_max),
            "optimiser": "adamw",
            "schedule": "onecycle",
            "seed": metrics.seed
        },
        "weights_sha256": w_sha
    });

    // Canonicalise (sort keys, no whitespace) for model_revision computation.
    let canonical_bytes = forecast::provenance::canonicalise(&meta_value);
    let model_revision = forecast::provenance::model_revision(&canonical_bytes);

    // Insert model_revision into the value.
    let mut meta_final = meta_value.clone();
    meta_final["model_revision"] = Value::String(model_revision.clone());

    // Re-canonicalise with model_revision set.
    let final_canonical = forecast::provenance::canonicalise(&meta_final);

    // Build filename prefix: "patchtst-<scenario>-<sha>".
    let sha = &model_revision;
    let prefix = format!("patchtst-{}-{sha}", metrics.scenario);

    // Rename safetensors.
    let weights_path = out_dir.join(format!("{prefix}.safetensors"));
    std::fs::rename(&temp_path, &weights_path)
        .with_context(|| format!("renaming safetensors to {weights_path:?}"))?;

    // Write metadata JSON (canonical bytes).
    let meta_path = out_dir.join(format!("{prefix}.metadata.json"));
    std::fs::write(&meta_path, &final_canonical)
        .with_context(|| format!("writing metadata to {meta_path:?}"))?;

    info!(
        safetensors = %weights_path.display(),
        metadata_json = %meta_path.display(),
        model_revision = %sha,
        scenario = %metrics.scenario,
        sigma_train = metrics.sigma_train,
        "checkpoint written"
    );

    Ok(model_revision)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&["train_patchtst=info"], false)?;

    let cli = Cli::parse();

    // ── Audit-DB writer ───────────────────────────────────────────────────────
    let audit = match &cli.audit_db {
        Some(path) => match AuditWriter::open(path) {
            Ok(w) => {
                info!(path = %path.display(), "audit-db enabled");
                w
            }
            Err(e) => {
                warn!(%e, "audit-db open failed; continuing without audit (non-fatal)");
                AuditWriter::disabled()
            }
        },
        None => AuditWriter::disabled(),
    };

    let run_id = Uuid::new_v4().to_string();

    let symbols: Vec<String> = cli
        .symbols
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let span_start = normalise_date_start(&cli.span_start);
    let span_end = normalise_date_end(&cli.span_end);

    info!(
        scenario = %cli.scenario,
        model_family = "patchtst",
        data_root = %cli.data_root.display(),
        out_dir = %cli.out_dir.display(),
        span_start = %span_start,
        span_end = %span_end,
        epochs = cli.epochs,
        batch_size = cli.batch_size,
        seed = cli.seed,
        target_horizon_bars = cli.target_horizon_bars,
        %run_id,
        "train_patchtst starting"
    );

    let training_start_instant = Instant::now();

    audit.write_start(&run_id, &cli.scenario, cli.seed as i64);

    std::fs::create_dir_all(&cli.out_dir)
        .with_context(|| format!("creating out_dir {:?}", cli.out_dir))?;

    // ── Device selection ──────────────────────────────────────────────────────
    #[cfg(feature = "metal")]
    let device = Device::new_metal(0).unwrap_or_else(|e| {
        warn!("Metal device failed ({e}); falling back to CPU");
        Device::Cpu
    });
    #[cfg(not(feature = "metal"))]
    let device = Device::Cpu;

    info!(device = ?device, "using device");

    // Seeded RNG (ADR-0002: ChaCha20Rng from fixed seed — no SystemTime/OsRng).
    let _rng_init = ChaCha20Rng::seed_from_u64(cli.seed);

    // ── Build model ───────────────────────────────────────────────────────────
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = PatchTstModel::new(vb).context("building PatchTstModel")?;

    let n_params = model.num_parameters();
    info!(
        n_params,
        patch_len = cli.patch_len,
        stride = cli.stride,
        d_model = cli.d_model,
        n_heads = cli.n_heads,
        d_ff = cli.d_ff,
        n_layers = cli.n_layers,
        dropout = cli.dropout,
        context_len = cli.context_len,
        "PatchTstModel built"
    );

    // ── Load training windows ─────────────────────────────────────────────────
    let train_span = TimeSpan::new(parse_ts(&span_start)?, parse_ts(&span_end)?);

    let feat_cfg = FeatureConfig {
        context_bars: cli.context_len,
        vol_z_lookback: 720,
        direction_epsilon: 0.0005,
        target_horizon_bars: cli.target_horizon_bars,
        vol_target_kind: None,
    };

    info!("loading training windows...");
    let mut train_windows: Vec<FeatureWindow> = Vec::new();
    for symbol in &symbols {
        let iter = windows_for_symbol(&cli.data_root, symbol, train_span.clone(), &feat_cfg);
        let mut count = 0usize;
        for w in iter {
            match w {
                Ok(window) => {
                    train_windows.push(window);
                    count += 1;
                }
                Err(e) => warn!(symbol, %e, "skipping symbol due to feature error"),
            }
        }
        info!(symbol, windows = count, "loaded training windows");
    }

    if train_windows.is_empty() {
        let msg = "No training windows loaded. Check data_root and span.";
        audit.write_failed(&run_id, msg);
        anyhow::bail!(msg);
    }

    info!(
        total_training_windows = train_windows.len(),
        "Loaded {} training windows for span {}..{} (overlapping {}h targets, {} symbols)",
        train_windows.len(),
        cli.span_start,
        cli.span_end,
        cli.target_horizon_bars,
        symbols.len()
    );

    // ── Load optional validation windows ─────────────────────────────────────
    let mut val_windows: Vec<FeatureWindow> = Vec::new();
    if let (Some(vs), Some(ve)) = (&cli.val_start, &cli.val_end) {
        let val_start = normalise_date_start(vs);
        let val_end = normalise_date_end(ve);
        let val_span = TimeSpan::new(parse_ts(&val_start)?, parse_ts(&val_end)?);
        for symbol in &symbols {
            let iter = windows_for_symbol(&cli.data_root, symbol, val_span.clone(), &feat_cfg);
            for w in iter {
                match w {
                    Ok(window) => val_windows.push(window),
                    Err(e) => warn!(symbol, %e, "skipping val window"),
                }
            }
        }
        info!(val_windows = val_windows.len(), "loaded validation windows");
    }

    // ── Training loop: AdamW + OneCycle + Huber ───────────────────────────────
    let adamw_params = ParamsAdamW {
        lr: cli.lr_max,
        beta1: 0.9,
        beta2: 0.999,
        eps: 1e-8,
        weight_decay: 1e-4,
    };
    let mut optimizer = AdamW::new(varmap.all_vars(), adamw_params)?;

    let batch_size = cli.batch_size;
    let context = cli.context_len;
    let total_steps_per_epoch = train_windows.len() / batch_size;
    let total_steps = cli.epochs as usize * total_steps_per_epoch;
    let delta = cli.huber_delta as f32;
    let n_train = train_windows.len();

    let mut best_val_loss = f32::MAX;
    let mut step = 0usize;
    let mut final_train_loss = 0.0_f32;
    let mut final_val_loss = f32::NAN;
    let mut epochs_actually_trained = 0u32;

    // Per-epoch wall-clock history for the cost tripwire (ADR-0036 § D4).
    let mut epoch_wall_clock_history: Vec<u64> = Vec::new();

    // Seeded shuffle RNG (deterministic: same seed → same shuffle order).
    let mut rng = ChaCha20Rng::seed_from_u64(cli.seed);

    for epoch in 0..cli.epochs {
        epochs_actually_trained = epoch + 1;
        let epoch_start = Instant::now();

        // Deterministic Fisher-Yates shuffle.
        let mut indices: Vec<usize> = (0..n_train).collect();
        use rand::seq::SliceRandom;
        indices.shuffle(&mut rng);

        let mut epoch_loss = 0.0_f32;
        let mut n_batches = 0usize;

        for batch_start in (0..n_train).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(n_train);
            if batch_end - batch_start < 2 {
                break;
            }
            let batch_indices = &indices[batch_start..batch_end];
            let actual_batch = batch_end - batch_start;

            // Build [actual_batch, CHANNELS, context] feature tensor.
            let mut feat_data: Vec<f32> = Vec::with_capacity(actual_batch * CHANNELS * context);
            let mut target_data: Vec<f32> = Vec::with_capacity(actual_batch);

            for &idx in batch_indices {
                let w = &train_windows[idx];
                // features: [context, CHANNELS] row-major → transpose to [CHANNELS, context].
                let flat: Vec<f32> = w
                    .features
                    .flatten_all()
                    .and_then(|t| t.to_vec1::<f32>())
                    .unwrap_or_default();
                for c in 0..CHANNELS {
                    for t in 0..context {
                        let val = flat.get(t * CHANNELS + c).copied().unwrap_or(0.0);
                        feat_data.push(val);
                    }
                }
                target_data.push(w.target_logret);
            }

            let x = Tensor::from_vec(feat_data, (actual_batch, CHANNELS, context), &device)
                .context("building input tensor")?;
            let y = Tensor::from_vec(target_data, (actual_batch, 1), &device)
                .context("building target tensor")?;

            // OneCycle LR update.
            let lr = onecycle_lr(step, total_steps.max(1), cli.lr_max, 0.3);
            optimizer.set_learning_rate(lr);

            // Forward + backward.
            let pred = model.forward(&x, true).context("forward pass")?;
            let loss = huber_loss(&pred, &y, delta).context("huber loss")?;
            optimizer.backward_step(&loss).context("backward step")?;

            let loss_val = loss.to_scalar::<f32>().unwrap_or(f32::NAN);
            epoch_loss += loss_val;
            n_batches += 1;
            step += 1;
        }
        // ── END OF PER-EPOCH SCOPE ────────────────────────────────────────────
        // NOTE: No σ_train accumulator lives outside this loop.
        // ADR-0035 § D1 + ADR-0036 § D3 — σ_train is computed post-training.

        let avg_train_loss = if n_batches > 0 {
            epoch_loss / n_batches as f32
        } else {
            f32::NAN
        };
        final_train_loss = avg_train_loss;

        let avg_val_loss =
            compute_val_loss(&model, &val_windows, &device, context, batch_size, delta);
        final_val_loss = avg_val_loss;

        let epoch_wall_ms = epoch_start.elapsed().as_millis() as u64;
        let epoch_wall_sec = epoch_wall_ms / 1000;

        info!(
            epoch = epoch + 1,
            total_epochs = cli.epochs,
            train_loss = avg_train_loss,
            val_loss = avg_val_loss,
            lr = optimizer.learning_rate(),
            wall_clock_ms = epoch_wall_ms,
            model_family = "patchtst",
            scenario = %cli.scenario,
            "epoch complete"
        );

        // Emit audit epoch row (ADR-0034; model_family encoded in scenario prefix).
        audit.write_epoch(
            &run_id,
            (epoch + 1) as i64,
            cli.epochs as i64,
            avg_train_loss,
            avg_val_loss,
            epoch_wall_ms as i64,
        );

        // Cost tripwire check (ADR-0036 § D4).
        if let Err(tripwire_err) =
            assert_epoch_budget(epoch as usize, epoch_wall_sec, &epoch_wall_clock_history)
        {
            error!(
                epoch = epoch + 1,
                wall_clock_sec = epoch_wall_sec,
                error = %tripwire_err,
                "COST TRIPWIRE FIRED — continuing training (operator decision)"
            );
            // Write diagnostic file per ADR-0036 § D4.
            let diag_path = format!("/tmp/train_patchtst-bs1-tripwire-epoch{}.txt", epoch + 1);
            let diag_content = format!(
                "PatchTST BS-1 training cost tripwire fired at epoch {}.\n\
                 Error: {tripwire_err}\n\
                 wall_clock_sec: {epoch_wall_sec}\n\
                 history_secs: {epoch_wall_clock_history:?}\n\
                 run_id: {run_id}\n",
                epoch + 1,
            );
            if let Err(e) = std::fs::write(&diag_path, &diag_content) {
                warn!(%e, path = %diag_path, "failed to write tripwire diagnostic file (non-fatal)");
            } else {
                info!(path = %diag_path, "tripwire diagnostic written");
            }
            // Continue training (do NOT stop).
        }

        epoch_wall_clock_history.push(epoch_wall_sec);

        // Simple early stopping (no patience config at v0.1.0 — run all epochs).
        if !avg_val_loss.is_nan() && avg_val_loss < best_val_loss {
            best_val_loss = avg_val_loss;
        }
    }

    info!(
        final_train_loss,
        final_val_loss,
        epochs_trained = epochs_actually_trained,
        model_family = "patchtst",
        "training loop complete; computing sigma_train post-training..."
    );

    // ── σ_train post-training derivation (ADR-0035 § D1 + ADR-0036 § D3) ─────
    // This is the canonical correct pattern: frozen forward pass AFTER training.
    // The Vec<f32> accumulator is inside compute_sigma_train_post_training.
    let sigma_train =
        compute_sigma_train_post_training(&model, &train_windows, &device, context, batch_size);

    info!(
        sigma_train,
        "sigma_train derived via post-training frozen pass"
    );

    // ── Write checkpoint ──────────────────────────────────────────────────────
    let model_revision = write_checkpoint(
        &varmap,
        &cli.out_dir,
        PatchTstCheckpointMetrics {
            sigma_train,
            final_train_loss,
            final_val_loss,
            epochs_trained: epochs_actually_trained,
            scenario: cli.scenario.clone(),
            patch_len: cli.patch_len,
            stride: cli.stride,
            d_model: cli.d_model,
            n_heads: cli.n_heads,
            d_ff: cli.d_ff,
            n_layers: cli.n_layers,
            dropout: cli.dropout,
            context_len: cli.context_len,
            target_horizon_bars: cli.target_horizon_bars,
            lr_max: cli.lr_max,
            huber_delta: cli.huber_delta,
            batch: batch_size,
            epochs: cli.epochs,
            seed: cli.seed,
            span_start: span_start.clone(),
            span_end: span_end.clone(),
            symbols: symbols
                .iter()
                .map(|s| s.trim_end_matches("USDT").to_string())
                .collect(),
        },
    )?;

    let total_wall_ms = training_start_instant.elapsed().as_millis() as i64;

    audit.write_finish(
        &run_id,
        &model_revision,
        final_train_loss,
        final_val_loss,
        total_wall_ms,
    );

    info!(
        model_family = "patchtst",
        scenario = %cli.scenario,
        epochs = epochs_actually_trained,
        final_train_huber = final_train_loss,
        final_val_huber = final_val_loss,
        sigma_train,
        safetensors = %format!("crates/forecast/checkpoints/anchors/patchtst-{}-{model_revision}.safetensors", cli.scenario),
        "Training complete"
    );

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T-D-N12: epoch_budget hard limit fires at >24h.
    #[test]
    fn epoch_budget_hard_limit() {
        // Under the limit: OK.
        let ok = assert_epoch_budget(0, 3600, &[]);
        assert!(ok.is_ok(), "1h epoch should not trigger hard limit");

        // At 24h exactly: allowed (limit is strictly >, not >=).
        let at_limit = assert_epoch_budget(0, 24 * 3600, &[]);
        assert!(at_limit.is_ok(), "exactly 24h epoch should not trigger");

        // Over hard limit by 1 second: fires.
        let over = assert_epoch_budget(0, 24 * 3600 + 1, &[]);
        assert!(
            matches!(over, Err(CostTripwireError::HardLimit { .. })),
            "epoch 1 second over 24h should fire HardLimit"
        );

        // 3× median test: epoch with 3001s when history has median 1000s → 3001 > 3*1000 fires.
        let history = vec![900u64, 1000u64, 1100u64];
        let over_median = assert_epoch_budget(3, 3001, &history);
        assert!(
            matches!(over_median, Err(CostTripwireError::MedianMultiple { .. })),
            "3001s with median 1000s should fire MedianMultiple"
        );

        // Exactly 3× median: allowed (strictly >).
        let at_median = assert_epoch_budget(3, 3000, &history);
        assert!(at_median.is_ok(), "exactly 3× median should not trigger");

        // Empty history: no median check, no fire.
        let no_history = assert_epoch_budget(1, 5000, &[]);
        assert!(no_history.is_ok(), "no history: no median check");
    }

    /// T-D-N12: parse_hex_or_dec_u64 handles both formats.
    #[test]
    fn parse_hex_or_dec() {
        assert_eq!(parse_hex_or_dec_u64("12648430").unwrap(), 12648430u64);
        assert_eq!(parse_hex_or_dec_u64("0x00C0FFEE").unwrap(), 0x00C0FFEE_u64);
        assert_eq!(parse_hex_or_dec_u64("0xC0FFEE").unwrap(), 0xC0FFEE_u64);
        assert!(parse_hex_or_dec_u64("not_a_number").is_err());
    }
}
