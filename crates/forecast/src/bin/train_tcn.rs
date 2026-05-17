//! `train_tcn` — TCN training binary (M2).
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p forecast --bin train_tcn --features candle -- \
//!   --config crates/forecast/train_tcn.toml \
//!   --output-dir crates/forecast/checkpoints/
//!
//! # Dry-run (random-init checkpoint only, no training):
//! cargo run -p forecast --bin train_tcn --features candle -- \
//!   --config crates/forecast/train_tcn.toml \
//!   --output-dir crates/forecast/checkpoints/ \
//!   --dry-run
//!
//! # 1-epoch BTCUSDT smoke (T-D-10):
//! cargo run -p forecast --bin train_tcn --features candle -- \
//!   --config crates/forecast/train_tcn.toml \
//!   --output-dir crates/forecast/checkpoints/ \
//!   --epochs 1 --symbols BTCUSDT
//! ```
//!
//! ## Determinism contract
//!
//! - Seed: `0x00C0FFEE` via `ChaCha20Rng` (ADR-0002).
//! - No `SystemTime` / `Instant` / `chrono::Utc::now()` on any
//!   backtest-replay path (the training loop does NOT touch live clocks).
//! - Two runs with the same seed + same data produce byte-identical
//!   `<sha>.metadata.json` files.  The safetensors weights may differ on
//!   Metal (known limitation per D2), but the metadata JSON is deterministic
//!   because it is computed from the config, not from the weights.
//!
//! ## Cross-references
//!
//! - `spec/v25-tcn-overlay/feature.md § R7` — training schedule
//! - `spec/v25-tcn-overlay/feature.md § R8` — checkpoint provenance
//! - `spec/v25-tcn-overlay/feature.md § D4` — metadata-JSON canonicalisation
//! - `crates/forecast/src/provenance.rs` — canonicaliser
//! - `ADR-0029` — cross-phase provenance contract

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::Deserialize;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use forecast::{
    features::{FeatureConfig, FeatureWindow, TimeSpan, windows_for_symbol},
    provenance::{CheckpointMetadata, DataSpan},
    tcn::{TcnModel, INPUT_FEATURES},
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "train_tcn",
    about = "Train the TCN forecaster (v2.5)",
    long_about = "Train a Temporal Convolutional Network on OHLCV parquet data.\n\
                  Use --dry-run to write a random-init checkpoint without training."
)]
struct Cli {
    /// Path to the train_tcn.toml config file.
    #[arg(long, default_value = "crates/forecast/train_tcn.toml")]
    config: PathBuf,

    /// Directory to write checkpoint files into.
    #[arg(long, default_value = "crates/forecast/checkpoints/")]
    output_dir: PathBuf,

    /// Write a random-init checkpoint without running any training.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Override number of training epochs (default: from config).
    #[arg(long)]
    epochs: Option<u32>,

    /// Override symbols (comma-separated, e.g. "BTCUSDT,ETHUSDT").
    #[arg(long)]
    symbols: Option<String>,

    /// Override parquet root directory.
    #[arg(long)]
    parquet_root: Option<PathBuf>,

    /// Training window start (ISO-8601, e.g. "2023-01-01").
    /// Overrides [data].train_start in the config file.
    #[arg(long)]
    train_start: Option<String>,

    /// Training window end (ISO-8601, e.g. "2023-09-30").
    /// Overrides [data].train_end in the config file.
    #[arg(long)]
    train_end: Option<String>,

    /// Validation window start (ISO-8601, e.g. "2023-10-01").
    /// Overrides [data].val_start in the config file.
    #[arg(long)]
    val_start: Option<String>,

    /// Validation window end (ISO-8601, e.g. "2023-12-31").
    /// Overrides [data].val_end in the config file.
    #[arg(long)]
    val_end: Option<String>,

    /// Scenario name embedded in checkpoint metadata (e.g. "bs1", "bs2").
    /// Used to label the checkpoint without changing the provenance SHA.
    #[arg(long)]
    scenario: Option<String>,
}

// ── Config structs ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TomlConfig {
    architecture: TomlArchitecture,
    tokenisation: TomlTokenisation,
    training: TomlTraining,
    data: TomlData,
}

#[derive(Debug, Deserialize)]
struct TomlArchitecture {
    blocks: u32,
    channels: u32,
    kernel: u32,
    dilations: Vec<u32>,
    dropout: f64,
}

#[derive(Debug, Deserialize)]
struct TomlTokenisation {
    context_bars: usize,
    #[allow(dead_code)]
    features: Vec<String>,
    vol_z_lookback: usize,
}

#[derive(Debug, Deserialize)]
struct TomlTraining {
    #[allow(dead_code)]
    optimiser: String,
    lr_max: f64,
    #[allow(dead_code)]
    schedule: String,
    batch: usize,
    epochs: u32,
    #[allow(dead_code)]
    loss: String,
    huber_delta: f64,
    seed: u64,
    patience: u32,
}

#[derive(Debug, Deserialize)]
struct TomlData {
    parquet_root: PathBuf,
    train_start: String,
    train_end: String,
    val_start: String,
    val_end: String,
    symbols: Vec<String>,
    interval: String,
    source: String,
}

// ── Huber loss ────────────────────────────────────────────────────────────────

/// Huber loss: `δ²*(sqrt(1+(x/δ)²)-1)` or equivalently the common form.
/// We use the "smooth L1" variant: `0.5*x² if |x|<δ, δ*(|x| - 0.5*δ) else`.
fn huber_loss(pred: &Tensor, target: &Tensor, delta: f32) -> candle_core::Result<Tensor> {
    let diff = (pred - target)?;
    let abs_diff = diff.abs()?;
    let delta_t = Tensor::full(delta, pred.shape(), pred.device())?;
    let quadratic = (diff.sqr()? * 0.5_f64)?;
    let linear = ((abs_diff.clone() - (&delta_t * 0.5_f64)?)? * delta as f64)?;
    // Where |x| < delta: quadratic; else: linear (smooth-L1 / Huber).
    let mask = abs_diff.lt(&delta_t)?;
    let loss = mask.where_cond(&quadratic, &linear)?;
    loss.mean_all()
}

// ── OneCycle LR ───────────────────────────────────────────────────────────────

/// Simple OneCycle LR schedule: linear warm-up to `lr_max`, then cosine decay.
fn onecycle_lr(step: usize, total_steps: usize, lr_max: f64, pct_start: f64) -> f64 {
    let warmup = (total_steps as f64 * pct_start) as usize;
    if step < warmup {
        // Linear warm-up.
        lr_max * (step + 1) as f64 / warmup as f64
    } else {
        // Cosine annealing.
        let cos_progress = (step - warmup) as f64 / (total_steps - warmup).max(1) as f64;
        let cos = (std::f64::consts::PI * cos_progress).cos();
        lr_max * 0.5 * (1.0 + cos)
    }
}

// ── Time parsing ──────────────────────────────────────────────────────────────

fn parse_ts(s: &str) -> Result<time::OffsetDateTime> {
    // Parse ISO-8601 "YYYY-MM-DDTHH:MM:SSZ" or "YYYY-MM-DDTHH:MM:SS+00:00".
    let format = time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(s, &format)
        .with_context(|| format!("invalid timestamp: {s}"))
}

/// Normalise a bare "YYYY-MM-DD" or full RFC-3339 string to
/// "YYYY-MM-DDTOO:OO:OOZ" (midnight UTC) for window-start positions.
fn normalise_date_to_rfc3339(s: &str) -> String {
    if s.len() == 10 {
        format!("{s}T00:00:00Z")
    } else {
        s.to_string()
    }
}

/// Normalise a bare "YYYY-MM-DD" or full RFC-3339 string to
/// "YYYY-MM-DDT23:00:00Z" (last complete hourly bar UTC) for window-end.
fn normalise_date_to_rfc3339_end(s: &str) -> String {
    if s.len() == 10 {
        format!("{s}T23:00:00Z")
    } else {
        s.to_string()
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // Initialise tracing subscriber (respects RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("train_tcn=info".parse()?))
        .init();

    let cli = Cli::parse();

    // Load config.
    let config_str = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("reading config {:?}", cli.config))?;
    let mut cfg: TomlConfig =
        toml::from_str(&config_str).with_context(|| "parsing train_tcn.toml")?;

    // Apply CLI overrides.
    if let Some(epochs) = cli.epochs {
        cfg.training.epochs = epochs;
    }
    if let Some(symbols_str) = &cli.symbols {
        cfg.data.symbols = symbols_str.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Some(pr) = cli.parquet_root {
        cfg.data.parquet_root = pr;
    }
    // Per-scenario date window overrides (T-D-11 / T-D-12).
    // Bare "YYYY-MM-DD" dates are normalised to RFC-3339 with midnight UTC.
    if let Some(ts) = cli.train_start {
        cfg.data.train_start = normalise_date_to_rfc3339(&ts);
    }
    if let Some(ts) = cli.train_end {
        cfg.data.train_end = normalise_date_to_rfc3339_end(&ts);
    }
    if let Some(vs) = cli.val_start {
        cfg.data.val_start = normalise_date_to_rfc3339(&vs);
    }
    if let Some(ve) = cli.val_end {
        cfg.data.val_end = normalise_date_to_rfc3339_end(&ve);
    }
    // Scenario label (used in output file prefix for human readability).
    let scenario_label = cli.scenario.unwrap_or_else(|| "default".to_string());

    info!(
        config = %cli.config.display(),
        output_dir = %cli.output_dir.display(),
        dry_run = cli.dry_run,
        epochs = cfg.training.epochs,
        symbols = ?cfg.data.symbols,
        scenario = %scenario_label,
        train_start = %cfg.data.train_start,
        train_end = %cfg.data.train_end,
        val_start = %cfg.data.val_start,
        val_end = %cfg.data.val_end,
        "train_tcn starting"
    );

    // Ensure output dir exists.
    std::fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("creating output dir {:?}", cli.output_dir))?;

    // Build device (Metal if feature enabled, else CPU).
    #[cfg(feature = "metal")]
    let device = Device::new_metal(0).unwrap_or_else(|e| {
        warn!("Metal device failed ({e}); falling back to CPU");
        Device::Cpu
    });
    #[cfg(not(feature = "metal"))]
    let device = Device::Cpu;

    info!(device = ?device, "using device");

    // Seeded RNG (ADR-0002: ChaCha20Rng from fixed seed).
    // DETERMINISM: no SystemTime/Instant on the training path.
    let _rng = ChaCha20Rng::seed_from_u64(cfg.training.seed);

    // Build the model.
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);

    let dilations: Vec<usize> = cfg.architecture.dilations.iter().map(|&d| d as usize).collect();
    let model = TcnModel::with_config(
        INPUT_FEATURES,
        cfg.architecture.channels as usize,
        cfg.architecture.kernel as usize,
        &dilations,
        cfg.architecture.dropout,
        vb,
    )
    .context("building TcnModel")?;

    info!(
        blocks = cfg.architecture.blocks,
        channels = cfg.architecture.channels,
        "model built"
    );

    if cli.dry_run {
        info!("--dry-run: writing random-init checkpoint (no training)");
        write_checkpoint(
            &model,
            &varmap,
            &cfg,
            &cli.output_dir,
            TrainingMetrics {
                sigma_train: 0.0,
                final_train_loss: 0.0,
                final_val_loss: 0.0,
                epochs_trained: 0,
                scenario: scenario_label.clone(),
            },
        )?;
        info!("--dry-run: done");
        return Ok(());
    }

    // Parse time spans.
    let train_span = TimeSpan::new(
        parse_ts(&cfg.data.train_start)?,
        parse_ts(&cfg.data.train_end)?,
    );
    let val_span = TimeSpan::new(
        parse_ts(&cfg.data.val_start)?,
        parse_ts(&cfg.data.val_end)?,
    );

    let feat_cfg = FeatureConfig {
        context_bars: cfg.tokenisation.context_bars,
        vol_z_lookback: cfg.tokenisation.vol_z_lookback,
        ..Default::default()
    };

    // Load training windows.
    info!("loading training windows...");
    let mut train_windows: Vec<FeatureWindow> = Vec::new();
    for symbol in &cfg.data.symbols {
        let iter = windows_for_symbol(
            &cfg.data.parquet_root,
            symbol,
            train_span.clone(),
            &feat_cfg,
        );
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
        anyhow::bail!("No training windows loaded. Check parquet_root and data spans.");
    }

    // Load validation windows.
    info!("loading validation windows...");
    let mut val_windows: Vec<FeatureWindow> = Vec::new();
    for symbol in &cfg.data.symbols {
        let iter = windows_for_symbol(
            &cfg.data.parquet_root,
            symbol,
            val_span.clone(),
            &feat_cfg,
        );
        for w in iter {
            match w {
                Ok(window) => val_windows.push(window),
                Err(e) => warn!(symbol, %e, "skipping val window"),
            }
        }
    }
    info!(
        train_windows = train_windows.len(),
        val_windows = val_windows.len(),
        "data loaded"
    );

    // Training loop: AdamW + OneCycle.
    let adamw_params = ParamsAdamW {
        lr: cfg.training.lr_max,
        beta1: 0.9,
        beta2: 0.999,
        eps: 1e-8,
        weight_decay: 1e-4,
    };
    let mut optimizer = AdamW::new(varmap.all_vars(), adamw_params)?;

    let batch_size = cfg.training.batch;
    let total_steps_per_epoch = train_windows.len() / batch_size;
    let total_steps = cfg.training.epochs as usize * total_steps_per_epoch;
    let delta = cfg.training.huber_delta as f32;
    let context = cfg.tokenisation.context_bars;

    let mut best_val_loss = f32::MAX;
    let mut patience_counter = 0u32;
    let mut step = 0usize;
    let mut final_train_loss = 0.0_f32;
    let mut final_val_loss = 0.0_f32;
    let mut all_r_hats: Vec<f32> = Vec::new(); // for sigma_train
    let mut epochs_actually_trained = 0u32;

    // Seeded shuffle order (deterministic).
    let mut rng = ChaCha20Rng::seed_from_u64(cfg.training.seed);
    let n_train = train_windows.len();

    for epoch in 0..cfg.training.epochs {
        epochs_actually_trained = epoch + 1;
        // Deterministic shuffle using Fisher-Yates with seeded RNG.
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

            // Build batch tensors: [batch, context, 5] → transpose to [batch, 5, context].
            let mut feat_data: Vec<f32> = Vec::with_capacity(actual_batch * 5 * context);
            let mut target_data: Vec<f32> = Vec::with_capacity(actual_batch);

            for &idx in batch_indices {
                let w = &train_windows[idx];
                // Extract flat feature data from the window.
                // features layout (row-major [context, 5]) — transpose to [5, context].
                let flat: Vec<f32> = w.features.flatten_all()
                    .and_then(|t| t.to_vec1::<f32>())
                    .unwrap_or_default();
                // Transpose: flat[row=t, col=c] → feat_data[c * context + t]
                for c in 0..5 {
                    for t in 0..context {
                        let val = flat.get(t * 5 + c).copied().unwrap_or(0.0);
                        feat_data.push(val);
                    }
                }
                target_data.push(w.target_logret);
            }

            let x = Tensor::from_vec(feat_data, (actual_batch, 5, context), &device)
                .context("building input tensor")?;
            let y = Tensor::from_vec(target_data, (actual_batch, 1), &device)
                .context("building target tensor")?;

            // Update LR via OneCycle schedule.
            let lr = onecycle_lr(step, total_steps.max(1), cfg.training.lr_max, 0.3);
            optimizer.set_learning_rate(lr);

            // Forward + backward.
            let pred = model.forward(&x, true).context("forward pass")?;
            let loss = huber_loss(&pred, &y, delta).context("huber loss")?;

            optimizer.backward_step(&loss).context("backward step")?;

            let loss_val = loss.to_scalar::<f32>().unwrap_or(f32::NAN);
            epoch_loss += loss_val;
            n_batches += 1;
            step += 1;

            // Collect r_hats for sigma_train.
            if let Ok(r_hats) = pred.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
                all_r_hats.extend_from_slice(&r_hats);
            }
        }

        let avg_train_loss = if n_batches > 0 {
            epoch_loss / n_batches as f32
        } else {
            f32::NAN
        };
        final_train_loss = avg_train_loss;

        // Validation loss.
        let avg_val_loss = compute_val_loss(
            &model,
            &val_windows,
            &device,
            context,
            batch_size,
            delta,
        );
        final_val_loss = avg_val_loss;

        info!(
            epoch = epoch + 1,
            total_epochs = cfg.training.epochs,
            train_loss = avg_train_loss,
            val_loss = avg_val_loss,
            lr = optimizer.learning_rate(),
            "epoch complete"
        );

        // Early stopping.
        if avg_val_loss < best_val_loss {
            best_val_loss = avg_val_loss;
            patience_counter = 0;
        } else {
            patience_counter += 1;
            if patience_counter >= cfg.training.patience {
                info!(
                    epoch = epoch + 1,
                    patience = cfg.training.patience,
                    best_val_loss,
                    "early stopping triggered"
                );
                break;
            }
        }
    }

    // Compute sigma_train.
    let sigma_train = if all_r_hats.len() > 1 {
        let n = all_r_hats.len() as f32;
        let mu = all_r_hats.iter().sum::<f32>() / n;
        let var = all_r_hats.iter().map(|&x| (x - mu).powi(2)).sum::<f32>() / n;
        var.sqrt().max(1e-8)
    } else {
        1.0_f32
    };

    info!(
        final_train_loss,
        final_val_loss,
        sigma_train,
        "training complete"
    );

    write_checkpoint(
        &model,
        &varmap,
        &cfg,
        &cli.output_dir,
        TrainingMetrics {
            sigma_train,
            final_train_loss,
            final_val_loss,
            epochs_trained: epochs_actually_trained,
            scenario: scenario_label.clone(),
        },
    )?;

    Ok(())
}

// ── Validation loss helper ────────────────────────────────────────────────────

fn compute_val_loss(
    model: &TcnModel,
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

        let mut feat_data: Vec<f32> = Vec::with_capacity(actual_batch * 5 * context);
        let mut target_data: Vec<f32> = Vec::with_capacity(actual_batch);

        for w in &val_windows[batch_start..batch_end] {
            let flat: Vec<f32> = w.features.flatten_all()
                .and_then(|t| t.to_vec1::<f32>())
                .unwrap_or_default();
            for c in 0..5 {
                for t in 0..context {
                    let val = flat.get(t * 5 + c).copied().unwrap_or(0.0);
                    feat_data.push(val);
                }
            }
            target_data.push(w.target_logret);
        }

        let x = match Tensor::from_vec(feat_data, (actual_batch, 5, context), device) {
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

// ── Checkpoint write ──────────────────────────────────────────────────────────

/// Training metrics to embed in checkpoint metadata.
struct TrainingMetrics {
    sigma_train: f32,
    final_train_loss: f32,
    final_val_loss: f32,
    epochs_trained: u32,
    scenario: String,
}

fn write_checkpoint(
    _model: &TcnModel,
    varmap: &VarMap,
    cfg: &TomlConfig,
    output_dir: &Path,
    metrics: TrainingMetrics,
) -> Result<()> {
    use forecast::provenance::{ArchitectureConfig, TokenisationConfig, TrainingConfig};

    // Write safetensors to a temp file to compute SHA.
    let temp_path = output_dir.join("_tmp_checkpoint.safetensors");
    varmap
        .save(&temp_path)
        .context("saving safetensors")?;

    let weights_bytes = std::fs::read(&temp_path).context("reading temp safetensors")?;
    let w_sha = forecast::provenance::weights_sha256(&weights_bytes);

    // Build metadata.
    let mut meta = CheckpointMetadata {
        architecture: ArchitectureConfig {
            blocks: cfg.architecture.blocks,
            channels: cfg.architecture.channels,
            kernel: cfg.architecture.kernel,
            dilations: cfg.architecture.dilations.clone(),
            dropout: format!("{:.6}", cfg.architecture.dropout),
        },
        tokenisation: TokenisationConfig {
            context_bars: cfg.tokenisation.context_bars as u32,
            features: vec![
                "logret".into(),
                "logrange".into(),
                "logvol_z".into(),
                "hour_sin".into(),
                "hour_cos".into(),
            ],
        },
        training: TrainingConfig {
            optimiser: "adamw".into(),
            lr_max: format!("{:.6}", cfg.training.lr_max),
            schedule: "onecycle".into(),
            batch: cfg.training.batch as u32,
            epochs: cfg.training.epochs,
            loss: "huber".into(),
            huber_delta: format!("{:.6}", cfg.training.huber_delta),
            seed: cfg.training.seed,
        },
        data_span: DataSpan {
            start: cfg.data.train_start.clone(),
            end: cfg.data.val_end.clone(),
            symbols: cfg.data.symbols.iter().map(|s| {
                // Strip "USDT" suffix for the canonical schema (matches R8 example).
                s.trim_end_matches("USDT").to_string()
            }).collect(),
            interval: cfg.data.interval.clone(),
            source: cfg.data.source.clone(),
        },
        weights_sha256: w_sha,
        model_revision: String::new(),
        sigma_train: metrics.sigma_train,
        final_train_loss: metrics.final_train_loss,
        final_val_loss: metrics.final_val_loss,
        epochs_trained: metrics.epochs_trained,
    };

    // Finalise model_revision (SHA-256 over canonical metadata bytes).
    meta.finalise();

    let sha = &meta.model_revision;
    info!(model_revision = sha, "checkpoint model_revision computed");

    // Build filename prefix: "tcn-<scenario>-<sha>" when scenario is
    // provided (e.g. "tcn-bs1-<sha>"), otherwise just "<sha>".
    let prefix = if metrics.scenario.is_empty() || metrics.scenario == "default" {
        sha.clone()
    } else {
        format!("tcn-{}-{sha}", metrics.scenario)
    };

    // Rename safetensors to <prefix>.safetensors.
    let weights_path = output_dir.join(format!("{prefix}.safetensors"));
    std::fs::rename(&temp_path, &weights_path)
        .with_context(|| format!("renaming safetensors to {weights_path:?}"))?;

    // Write metadata JSON.
    let meta_path = output_dir.join(format!("{prefix}.metadata.json"));
    let meta_bytes = meta.to_canonical_bytes();
    std::fs::write(&meta_path, &meta_bytes)
        .with_context(|| format!("writing metadata to {meta_path:?}"))?;

    info!(
        safetensors = %weights_path.display(),
        metadata_json = %meta_path.display(),
        model_revision = sha,
        scenario = %metrics.scenario,
        "checkpoint written"
    );

    Ok(())
}
