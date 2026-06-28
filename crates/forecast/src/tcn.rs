//! TCN (Temporal Convolutional Network) forecaster implementation.
//!
//! ## Architecture (feature.md § D1, R1)
//!
//! 8 stacked `TemporalBlock` residual blocks with dilation schedule
//! `[1, 2, 4, 8, 16, 32, 64, 128]`, kernel size k=3, H=96 channels,
//! dropout 0.1.
//!
//! ### Receptive field (BKK18 formula)
//!
//! `RF = 1 + 2 * (k-1) * sum(dilations) = 1 + 2*2*255 = 1021 bars`
//!
//! At hourly cadence: ~42 days.  Context window is 256 bars — well within RF.
//!
//! ### Block layout (D1)
//!
//! ```text
//! conv1: Conv1d(in_ch  → out_ch, k=3, dilation=d, padding=(k-1)*d)
//! conv2: Conv1d(out_ch → out_ch, k=3, dilation=d, padding=(k-1)*d)
//! skip:  Identity if in_ch == out_ch, else 1×1 Conv1d(in_ch → out_ch)
//! dropout: 0.1
//!
//! forward(x):
//!   y = conv1(x);  y = causal_trim(y, (k-1)*d);  y = relu(y);  y = dropout(y)
//!   y = conv2(y);  y = causal_trim(y, (k-1)*d);  y = relu(y);  y = dropout(y)
//!   out = relu(y + skip(x))
//! ```
//!
//! ### Weight-norm decision (T-D-5 developer call)
//!
//! The architect spec calls for `WeightNormConv1d`. As of candle 0.9 there is
//! no built-in weight-norm; landing a 30-line `g·v/||v||` reparameterisation
//! helper is feasible but adds ~50 lines of non-trivial tensor indexing. The
//! developer's call (per T-D-5): **drop weight-norm for this wave** and use
//! plain `candle_nn::Conv1d`.
//!
//! Rationale:
//! - Weight-norm is an optimisation aid (stabilises gradient scale), not a
//!   correctness requirement. The network trains without it; convergence may
//!   be marginally slower on hard crypto return distributions.
//! - Adding it introduces a custom parameter-reparameterisation path that
//!   complicates checkpoint serialisation and makes T-D-9 (provenance) more
//!   fragile.
//! - The v2.5a (PatchTST) phase will use a plain Linear head anyway; keeping
//!   the training infrastructure uniform across phases reduces cross-phase
//!   divergence.
//! - Wave C can add a `WeightNormConv1d` wrapper as a pure optimisation
//!   without any API change (same forward signature).
//!
//! This deviation is recorded here and flagged to the architect per the
//! "honest tick" rule.
//!
//! ## M4 — Full inference path (T-D-13)
//!
//! `load_anchor(scenario)` loads the LFS-anchored safetensors checkpoint and
//! returns a fully-initialised `TcnForecaster` with `sigma_train` from the
//! companion `.metadata.json`.
//!
//! `forecast()` (the `ForecastProvider` impl) now:
//! 1. Builds a proper 5-feature window from `OhlcvBar` using the same
//!    formula as `features.rs` (logret, logrange, logvol_z, hour_sin, hour_cos).
//! 2. Checks the replay-cache (`crates/replay-cache/`, namespace `"forecast"`)
//!    keyed by `SHA-256(canonical-JSON(model_revision, ohlcv_bars, sampling))`.
//! 3. On cache miss in live mode: runs inference; stores result. On cache miss
//!    in strict-replay mode: returns `ForecastError::ReplayMiss`.
//! 4. Emits one `tracing::info!` event that downstream audit wiring can
//!    consume (full audit ledger write is the caller's responsibility since
//!    `forecast()` is async-trait and has no ledger handle).
//!
//! ## Cross-references
//!
//! - `spec/v1/v25-tcn-overlay/feature.md § D1` — block layout spec
//! - `spec/v1/v25-tcn-overlay/feature.md § R1` — topology
//! - `spec/v1/v25-tcn-overlay/feature.md § R2` — model size
//! - `spec/v1/v25-tcn-overlay/feature.md § R10` — strict-replay determinism
//! - `spec/v1/v25-tcn-overlay/feature.md § R11` — audit emission
//! - `spec/v1/v25-tcn-overlay/feature.md § R12` — cost telemetry
//! - `ADR-0029` — checkpoint provenance contract

use candle_core::{DType, Device, Module, Result as CResult, Tensor};
use candle_nn::{Conv1d, Conv1dConfig, Dropout, VarBuilder};
use std::path::{Path, PathBuf};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default number of residual blocks.
pub const N_BLOCKS: usize = 8;

/// Default kernel size for all Conv1d layers.
pub const KERNEL_SIZE: usize = 3;

/// Default channel count (H) per layer.
pub const CHANNELS: usize = 96;

/// Default input feature dimension (5 features per bar).
pub const INPUT_FEATURES: usize = 5;

/// Context window length in bars.
pub const CONTEXT_LEN: usize = 256;

/// Dilation schedule for the 8 blocks.
pub const DILATIONS: [usize; N_BLOCKS] = [1, 2, 4, 8, 16, 32, 64, 128];

/// Dropout rate.
pub const DROPOUT: f32 = 0.1;

// ── Skip projection ────────────────────────────────────────────────────────────

/// Skip connection: identity (in_ch == out_ch) or 1×1 projection.
enum SkipProjection {
    Identity,
    Projection(Conv1d),
}

impl SkipProjection {
    fn forward(&self, x: &Tensor) -> CResult<Tensor> {
        match self {
            Self::Identity => Ok(x.clone()),
            Self::Projection(conv) => conv.forward(x),
        }
    }
}

// ── TemporalBlock ──────────────────────────────────────────────────────────────

/// One residual block of the TCN.
///
/// Forward input shape: `[batch, in_channels, seq_len]`.
/// Forward output shape: `[batch, out_channels, seq_len]` (shape-preserving).
///
/// Padding `(k-1)*d` is applied on the left (via `Conv1dConfig::padding`)
/// and the right tail is trimmed after each conv to maintain causal alignment.
pub struct TemporalBlock {
    conv1: Conv1d,
    conv2: Conv1d,
    skip: SkipProjection,
    dropout: Dropout,
}

impl TemporalBlock {
    /// Construct a `TemporalBlock`.
    ///
    /// # Arguments
    ///
    /// - `in_ch`: input channel count.
    /// - `out_ch`: output channel count.
    /// - `kernel`: kernel size (3 for TCN default).
    /// - `dilation`: dilation factor for this block.
    /// - `dropout`: dropout probability.
    /// - `vb`: `VarBuilder` scope to register parameters into.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error` on tensor allocation failure.
    pub fn new(
        in_ch: usize,
        out_ch: usize,
        kernel: usize,
        dilation: usize,
        dropout: f64,
        vb: VarBuilder,
    ) -> CResult<Self> {
        let trim = (kernel - 1) * dilation;
        let padding = trim; // left-pad only — causal trim restores seq_len

        let conv1_cfg = Conv1dConfig {
            padding,
            dilation,
            stride: 1,
            groups: 1,
            ..Conv1dConfig::default()
        };
        let conv2_cfg = Conv1dConfig {
            padding,
            dilation,
            stride: 1,
            groups: 1,
            ..Conv1dConfig::default()
        };

        let conv1 = candle_nn::conv1d(in_ch, out_ch, kernel, conv1_cfg, vb.pp("conv1"))?;
        let conv2 = candle_nn::conv1d(out_ch, out_ch, kernel, conv2_cfg, vb.pp("conv2"))?;

        let skip = if in_ch == out_ch {
            SkipProjection::Identity
        } else {
            // 1×1 convolution for channel projection (no dilation, no padding).
            let skip_cfg = Conv1dConfig {
                padding: 0,
                dilation: 1,
                stride: 1,
                groups: 1,
                ..Conv1dConfig::default()
            };
            let proj = candle_nn::conv1d(in_ch, out_ch, 1, skip_cfg, vb.pp("skip"))?;
            SkipProjection::Projection(proj)
        };

        let dropout = Dropout::new(dropout as f32);

        Ok(Self {
            conv1,
            conv2,
            skip,
            dropout,
        })
    }

    /// Forward pass.
    ///
    /// Input shape: `[batch, in_ch, seq_len]`.
    /// Output shape: `[batch, out_ch, seq_len]`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error` on shape mismatches or tensor ops.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        let seq_len = x.dim(2)?;

        // Branch 1: two dilated causal conv layers.
        let y = self.conv1.forward(x)?;
        // Causal trim: drop the rightmost `trim` elements.
        let y = causal_trim(&y, seq_len)?;
        let y = y.relu()?;
        let y = self.dropout.forward(&y, train)?;

        let y = self.conv2.forward(&y)?;
        let y = causal_trim(&y, seq_len)?;
        let y = y.relu()?;
        let y = self.dropout.forward(&y, train)?;

        // Residual: add skip connection then apply ReLU (locuslab/BKK18 default).
        let skip_x = self.skip.forward(x)?;
        (y + skip_x)?.relu()
    }
}

/// Trim the output of a Conv1d to restore the original sequence length.
///
/// After left-padding by `(k-1)*d`, `conv1d` produces output of length
/// `seq_len + (k-1)*d`. We narrow to `[0, seq_len)` to restore causality.
fn causal_trim(t: &Tensor, seq_len: usize) -> CResult<Tensor> {
    t.narrow(2, 0, seq_len)
}

// ── TcnModel (8-block stack + head) ───────────────────────────────────────────

/// The full TCN: 8 stacked `TemporalBlock` layers + a 1×1 linear head.
///
/// Forward input shape: `[batch, INPUT_FEATURES, CONTEXT_LEN]`.
/// Forward output shape: `[batch, 1]` (one scalar `r_hat` per sample).
pub struct TcnModel {
    blocks: Vec<TemporalBlock>,
    /// Final 1×1 conv: `[batch, CHANNELS, CONTEXT_LEN] → [batch, 1, CONTEXT_LEN]`.
    head: Conv1d,
}

impl TcnModel {
    /// Construct a `TcnModel` with the architect-locked defaults (D1).
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        Self::with_config(
            INPUT_FEATURES,
            CHANNELS,
            KERNEL_SIZE,
            &DILATIONS,
            DROPOUT as f64,
            vb,
        )
    }

    /// Construct with explicit configuration (for tests with smaller shapes).
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn with_config(
        in_features: usize,
        channels: usize,
        kernel: usize,
        dilations: &[usize],
        dropout: f64,
        vb: VarBuilder,
    ) -> CResult<Self> {
        let mut blocks = Vec::with_capacity(dilations.len());
        for (i, &d) in dilations.iter().enumerate() {
            let in_ch = if i == 0 { in_features } else { channels };
            let block = TemporalBlock::new(
                in_ch,
                channels,
                kernel,
                d,
                dropout,
                vb.pp(format!("block_{i}")),
            )?;
            blocks.push(block);
        }

        let head_cfg = Conv1dConfig {
            padding: 0,
            dilation: 1,
            stride: 1,
            groups: 1,
            ..Conv1dConfig::default()
        };
        let head = candle_nn::conv1d(channels, 1, 1, head_cfg, vb.pp("head"))?;

        Ok(Self { blocks, head })
    }

    /// Forward pass.
    ///
    /// Input shape: `[batch, in_features, seq_len]`.
    /// Output shape: `[batch, 1]`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        let seq_len = x.dim(2)?;

        // Pass through all residual blocks.
        let mut h = x.clone();
        for block in &self.blocks {
            h = block.forward(&h, train)?;
        }

        // 1×1 head: [batch, channels, seq_len] → [batch, 1, seq_len].
        let h = self.head.forward(&h)?;

        // Last-timestep: [batch, 1, seq_len] → [batch, 1, 1] → [batch, 1].
        let h = h.narrow(2, seq_len - 1, 1)?;
        // Squeeze the time dimension: [batch, 1, 1] → [batch, 1].
        let (b, _c, _t) = h.dims3()?;
        h.reshape((b, 1))
    }
}

// ── TcnForecaster ──────────────────────────────────────────────────────────────

// ── AnchorScenario ─────────────────────────────────────────────────────────────

/// Identifies which LFS-anchored checkpoint to load.
///
/// - `Bs1`: `tcn-bs1-<sha>` — trained on Jan–Sep 2023, evaluated Oct–Dec 2023.
/// - `Bs2`: `tcn-bs2-<sha>` — trained on 2023 full year, evaluated Q2–Q4 2024.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorScenario {
    /// BS-1 anchor: model trained on Jan–Sep 2023.
    Bs1,
    /// BS-2 anchor: model trained on 2023 full year.
    Bs2,
}

impl AnchorScenario {
    /// SHA prefix used to locate the anchor checkpoint files.
    ///
    /// The full file names are:
    /// - `tcn-bs1-<SHA>.safetensors` + `tcn-bs1-<SHA>.metadata.json`
    /// - `tcn-bs2-<SHA>.safetensors` + `tcn-bs2-<SHA>.metadata.json`
    pub fn sha_prefix(&self) -> &'static str {
        match self {
            AnchorScenario::Bs1 => {
                "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2"
            }
            AnchorScenario::Bs2 => {
                "3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d"
            }
        }
    }

    /// Canonical identifier string used as `model_revision` in forecasts.
    pub fn model_revision(&self) -> &'static str {
        self.sha_prefix()
    }

    /// The file-name prefix for this anchor scenario (e.g. `"tcn-bs1"`).
    ///
    /// Used to construct checkpoint paths:
    /// `<anchors_dir>/<file_prefix>-<sha_prefix>.{safetensors,metadata.json}`.
    pub fn file_prefix(&self) -> &'static str {
        match self {
            AnchorScenario::Bs1 => "tcn-bs1",
            AnchorScenario::Bs2 => "tcn-bs2",
        }
    }
}

// ── TcnForecasterError ─────────────────────────────────────────────────────────

/// Error type for `TcnForecaster` construction and inference.
#[derive(Debug, thiserror::Error)]
pub enum TcnForecasterError {
    /// Checkpoint file not found at the expected path.
    #[error("checkpoint not found: {path}")]
    CheckpointNotFound { path: String },

    /// Failed to load safetensors weights.
    #[error("safetensors load failed: {0}")]
    SafetensorsLoad(String),

    /// Failed to parse metadata JSON.
    #[error("metadata parse failed: {0}")]
    MetadataParse(String),

    /// Candle tensor error during model init or inference.
    #[error("candle error: {0}")]
    Candle(String),
}

impl From<candle_core::Error> for TcnForecasterError {
    fn from(e: candle_core::Error) -> Self {
        TcnForecasterError::Candle(e.to_string())
    }
}

/// The `TcnForecaster` wraps a `TcnModel` and implements `ForecastProvider`.
///
/// M4: full inference path — load anchor checkpoint, build feature window,
/// replay-cache lookup, emit `ForecastOverlay` with correct `sigma_train`.
pub struct TcnForecaster {
    pub model: TcnModel,
    pub device: Device,
    /// Standard deviation of `r_hat` on the training set (R6 confidence calibration).
    /// Pinned at checkpoint time; loaded from `.metadata.json`.
    pub sigma_train: f32,
    /// The canonical `model_revision` SHA (from checkpoint provenance).
    pub model_revision: String,
    /// Whether this forecaster operates in strict-replay mode.
    /// In strict-replay mode, a cache miss returns `ForecastError::ReplayMiss`.
    pub strict_replay: bool,
    /// Optional path to the replay-cache SQLite file.
    /// When `None`, replay-cache is disabled (live inference only).
    pub cache_path: Option<PathBuf>,
    /// Optional audit ledger for `ForecastEmitted` tick emission (T-D-13 /
    /// decomp §5A). Set via `with_ledger(ledger)`. Guarded by the
    /// `audit-tick` feature so training bins never carry the tick path.
    #[cfg(feature = "audit-tick")]
    pub(crate) ledger: Option<audit::Ledger>,
    /// Optional strategy id for the `post_forecast_event` SQL writer
    /// (Phase D R1.4 — ui-rethink-phase-d-trail). Set alongside `ledger`
    /// via `with_ledger`; left `None` in backtest/training paths.
    #[cfg(feature = "audit-tick")]
    pub(crate) forecast_strategy_id: Option<String>,
    /// Optional symbol for the `post_forecast_event` SQL writer (Phase D R1.4).
    #[cfg(feature = "audit-tick")]
    pub(crate) forecast_symbol: Option<String>,
}

impl TcnForecaster {
    /// Construct with a random-initialised model on the given device.
    ///
    /// `sigma_train` is set to 1.0 (placeholder for tests).
    /// `model_revision` is set to `"random-init"`.
    /// Replay-cache is disabled.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn random_init(device: Device) -> CResult<Self> {
        let vb = VarBuilder::zeros(DType::F32, &device);
        let model = TcnModel::new(vb)?;
        Ok(Self {
            model,
            device,
            sigma_train: 1.0,
            model_revision: "random-init".to_string(),
            strict_replay: false,
            cache_path: None,
            #[cfg(feature = "audit-tick")]
            ledger: None,
            #[cfg(feature = "audit-tick")]
            forecast_strategy_id: None,
            #[cfg(feature = "audit-tick")]
            forecast_symbol: None,
        })
    }

    /// Load an LFS-anchored checkpoint by scenario identifier.
    ///
    /// Looks for files at:
    /// `crates/forecast/checkpoints/anchors/tcn-{bs1|bs2}-{sha}.{safetensors,metadata.json}`
    ///
    /// `sigma_train` and `model_revision` are read from the companion
    /// `.metadata.json`.
    ///
    /// # Errors
    ///
    /// Returns `TcnForecasterError::CheckpointNotFound` if the file is absent.
    /// Returns `TcnForecasterError::SafetensorsLoad` on weight load failure.
    /// Returns `TcnForecasterError::MetadataParse` on JSON parse failure.
    pub fn load_anchor(scenario: AnchorScenario) -> Result<Self, TcnForecasterError> {
        // Resolve the checkpoint directory relative to the workspace root.
        // In tests and binaries, the CWD is the workspace root.
        let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");
        let prefix = scenario.file_prefix();
        let sha = scenario.sha_prefix();

        let safetensors_path = anchors_dir.join(format!("{prefix}-{sha}.safetensors"));
        let metadata_path = anchors_dir.join(format!("{prefix}-{sha}.metadata.json"));

        if !safetensors_path.exists() {
            return Err(TcnForecasterError::CheckpointNotFound {
                path: safetensors_path.display().to_string(),
            });
        }
        if !metadata_path.exists() {
            return Err(TcnForecasterError::CheckpointNotFound {
                path: metadata_path.display().to_string(),
            });
        }

        Self::load_from_paths(&safetensors_path, &metadata_path)
    }

    /// Load a `TcnForecaster` from explicit checkpoint + metadata paths.
    ///
    /// Used by `load_anchor` and tests.
    ///
    /// # Errors
    ///
    /// Returns `TcnForecasterError` on file I/O or parse failure.
    pub fn load_from_paths(
        safetensors_path: &Path,
        metadata_path: &Path,
    ) -> Result<Self, TcnForecasterError> {
        // Parse metadata JSON.
        let metadata_bytes =
            std::fs::read(metadata_path).map_err(|e| TcnForecasterError::CheckpointNotFound {
                path: format!("{}: {e}", metadata_path.display()),
            })?;
        let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes)
            .map_err(|e| TcnForecasterError::MetadataParse(e.to_string()))?;

        let sigma_train = metadata["sigma_train"].as_f64().unwrap_or(1.0_f64) as f32;
        let model_revision = metadata["model_revision"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        // Load safetensors weights.
        let bytes = std::fs::read(safetensors_path)
            .map_err(|e| TcnForecasterError::SafetensorsLoad(e.to_string()))?;

        let device = Device::Cpu;
        // Build a VarBuilder from the raw safetensors bytes using
        // candle_nn::VarBuilder::from_buffered_safetensors.
        let vb = VarBuilder::from_buffered_safetensors(bytes, DType::F32, &device)
            .map_err(|e| TcnForecasterError::SafetensorsLoad(e.to_string()))?;

        let model = TcnModel::new(vb).map_err(TcnForecasterError::from)?;

        tracing::info!(
            model_revision = %model_revision,
            sigma_train = sigma_train,
            "TcnForecaster loaded from checkpoint"
        );

        Ok(Self {
            model,
            device,
            sigma_train,
            model_revision,
            strict_replay: false,
            cache_path: None,
            #[cfg(feature = "audit-tick")]
            ledger: None,
            #[cfg(feature = "audit-tick")]
            forecast_strategy_id: None,
            #[cfg(feature = "audit-tick")]
            forecast_symbol: None,
        })
    }

    /// Enable strict-replay mode with the given cache database path.
    ///
    /// In strict-replay mode, a cache miss returns `ForecastError::ReplayMiss`.
    #[must_use]
    pub fn with_strict_replay(mut self, cache_path: PathBuf) -> Self {
        self.strict_replay = true;
        self.cache_path = Some(cache_path);
        self
    }

    /// Attach an audit ledger so `ForecastEmitted` ticks are emitted on the
    /// broadcast tick bus (T-D-13 / decomp §5A). Only available when the
    /// `audit-tick` feature is enabled.
    #[cfg(feature = "audit-tick")]
    #[must_use]
    pub fn with_ledger(mut self, ledger: audit::Ledger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    /// Attach the `strategy_id` and `symbol` context for the Phase D
    /// `post_forecast_event` SQL writer (R1.4 — ui-rethink-phase-d-trail).
    ///
    /// Called alongside `with_ledger`; only available when the `audit-tick`
    /// feature is enabled. Both fields are `None` in backtest/training paths,
    /// meaning the SQL writer branch is skipped (H2 anchor invariant holds).
    #[cfg(feature = "audit-tick")]
    #[must_use]
    pub fn with_forecast_context(mut self, strategy_id: String, symbol: String) -> Self {
        self.forecast_strategy_id = Some(strategy_id);
        self.forecast_symbol = Some(symbol);
        self
    }

    /// Enable live mode with the given cache database path for write-through.
    #[must_use]
    pub fn with_cache(mut self, cache_path: PathBuf) -> Self {
        self.strict_replay = false;
        self.cache_path = Some(cache_path);
        self
    }

    /// Forward pass.  Input shape: `[batch, 5, 256]`. Output shape: `[batch, 1]`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        self.model.forward(x, train)
    }
}

// ── ForecastProvider impl (T-D-6) ─────────────────────────────────────────────

use async_trait::async_trait;
use trading_core::forecast::{
    Direction, ForecastError, ForecastOverlay, ForecastRequest, ForecastResponse, OhlcvBar,
};

/// Convert a scalar `r_hat` prediction to `Direction` using epsilon threshold.
///
/// - `r_hat > +ε` → `Up`
/// - `r_hat < -ε` → `Down`
/// - else → `Flat`
pub fn r_hat_to_direction(r_hat: f32, epsilon: f32) -> Direction {
    if r_hat > epsilon {
        Direction::Up
    } else if r_hat < -epsilon {
        Direction::Down
    } else {
        Direction::Flat
    }
}

/// Default epsilon (5 bps per R6/D5).
pub const DIRECTION_EPSILON: f32 = 0.000_5_f32;

/// Build a proper 5-feature row from an `OhlcvBar` for inference.
///
/// This is the inference-time counterpart of `features.rs::build_features()`.
/// Features per bar:
/// - `logret`:   `ln(close_t / close_{t-1})` — requires prior close
/// - `logrange`: `ln(1 + (high - low) / close)`
/// - `logvol_z`: `(ln(1 + volume) - mu) / sigma` — requires rolling stats
/// - `hour_sin`: `sin(2π * hour_of_week / 168)`
/// - `hour_cos`: `cos(2π * hour_of_week / 168)`
///
/// For a window of `N` bars, this function takes a slice of N bars and
/// computes simplified in-window logvol_z (using window mean/std) and
/// logret from consecutive bars.
fn build_feature_window_from_ohlcv(bars: &[OhlcvBar]) -> Vec<f32> {
    use rust_decimal::prelude::ToPrimitive;
    use std::f32::consts::PI;

    let n = bars.len();
    assert!(n > 1, "need at least 2 bars for logret");

    // Pre-compute log-volumes for z-scoring.
    let log_vols: Vec<f32> = bars
        .iter()
        .map(|b| (1.0_f32 + b.volume.to_f32().unwrap_or(0.0)).ln())
        .collect();

    let mu_vol = log_vols.iter().sum::<f32>() / n as f32;
    let sigma_vol = {
        let var = log_vols
            .iter()
            .map(|v| (v - mu_vol) * (v - mu_vol))
            .sum::<f32>()
            / n as f32;
        var.sqrt().max(1e-6)
    };

    // Output: [5, n] in channel-first order (filled below).
    let mut feat_cf: Vec<f32> = vec![0.0; 5 * n];

    for t in 0..n {
        let bar = &bars[t];
        let close = bar.close.to_f32().unwrap_or(1.0).max(1e-8);
        let high = bar.high.to_f32().unwrap_or(close);
        let low = bar.low.to_f32().unwrap_or(close);

        // logret: use previous close (or 0 for first bar).
        let logret = if t == 0 {
            0.0_f32
        } else {
            let prev_close = bars[t - 1].close.to_f32().unwrap_or(1.0).max(1e-8);
            (close / prev_close).ln()
        };

        let logrange = (1.0_f32 + (high - low) / close).ln();
        let logvol_z = (log_vols[t] - mu_vol) / sigma_vol;

        // Hour-of-week seasonality.
        let hour_of_week = {
            use time::Weekday;
            let ts = bar.ts;
            let weekday_offset = match ts.weekday() {
                Weekday::Monday => 0,
                Weekday::Tuesday => 24,
                Weekday::Wednesday => 48,
                Weekday::Thursday => 72,
                Weekday::Friday => 96,
                Weekday::Saturday => 120,
                Weekday::Sunday => 144,
            };
            (weekday_offset + ts.hour() as usize) as f32
        };
        let hour_sin = (2.0 * PI * hour_of_week / 168.0).sin();
        let hour_cos = (2.0 * PI * hour_of_week / 168.0).cos();

        // Channel-first indexing: feat_cf[c * n + t].
        // The multiplications by 0 and 1 are intentional — they express the
        // general formula `channel * n + t` with the channel index written
        // explicitly so adding new channels (c=2…4 below) remains uniform.
        #[allow(clippy::erasing_op, clippy::identity_op)]
        {
            feat_cf[0 * n + t] = logret;
            feat_cf[1 * n + t] = logrange;
        }
        feat_cf[2 * n + t] = logvol_z;
        feat_cf[3 * n + t] = hour_sin;
        feat_cf[4 * n + t] = hour_cos;
    }

    feat_cf
}

/// Build a canonical cache-key string for a forecast request.
///
/// Excluded: `correlation_id` (per the replay-cache contract: same request
/// at different times shares a cache entry).
///
/// Key fields: `model_revision`, `ohlcv_window` (close prices as strings for
/// stability), `sampling`.
fn forecast_cache_key(request: &ForecastRequest) -> String {
    use sha2::{Digest, Sha256};
    // Build a compact canonical representation: model_revision + closes + sampling seed.
    let mut h = Sha256::new();
    h.update(request.model_revision.as_bytes());
    h.update(b"|");
    for bar in &request.ohlcv_window {
        h.update(bar.close.to_string().as_bytes());
        h.update(b",");
        h.update(bar.ts.unix_timestamp().to_string().as_bytes());
        h.update(b";");
    }
    h.update(b"|");
    h.update(request.sampling.sampling_seed.to_le_bytes());
    let digest = h.finalize();
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[async_trait]
impl crate::ForecastProvider for TcnForecaster {
    /// Run the TCN forward pass over the OHLCV window and return a
    /// `ForecastOverlay`.
    ///
    /// ## M4 full inference path (T-D-13)
    ///
    /// 1. Build a proper 5-feature window from the `ohlcv_window`.
    /// 2. Check the replay-cache (if configured) keyed by the canonical hash.
    /// 3. On cache miss in strict-replay mode: return `ForecastError::ReplayMiss`.
    /// 4. Run the TCN forward pass on CPU.
    /// 5. Emit `tracing::info!` with forecast details (R11 audit hook).
    /// 6. Store result in cache (if cache is configured and not strict-replay).
    ///
    /// ## Cost telemetry (R12)
    ///
    /// One `tracing::info!` event with `target = "forecast.cost"` carrying
    /// `line = "forecast_inference"` and `usd = 0`. The caller may attach a
    /// `tracing::Subscriber` that converts this into a `CostEvent::Infra`.
    ///
    /// # Errors
    ///
    /// Returns `ForecastError::InvalidInput` if the window is shorter than
    /// `CONTEXT_LEN`. Returns `ForecastError::ReplayMiss` on cache miss in
    /// strict-replay mode. Returns `ForecastError::Inference` on candle errors.
    async fn forecast(&self, request: ForecastRequest) -> Result<ForecastResponse, ForecastError> {
        let window = &request.ohlcv_window;

        if window.len() < CONTEXT_LEN {
            return Err(ForecastError::InvalidInput(format!(
                "ohlcv_window has {} bars; need {}",
                window.len(),
                CONTEXT_LEN
            )));
        }

        let t_start = std::time::Instant::now();

        // Compute the cache key (model_revision + window closes + sampling seed).
        let cache_key = forecast_cache_key(&request);

        // ── Replay-cache lookup ───────────────────────────────────────────────
        if let Some(cache_path) = &self.cache_path {
            // Try to load from cache.
            if let Ok(cache) = replay_cache::ReplayCache::<
                trading_core::forecast::ForecastRequest,
                ForecastResponse,
            >::open_readonly(cache_path, "forecast")
            .await
            {
                match cache.load(&cache_key).await {
                    Ok(Some(cached)) => {
                        tracing::debug!(
                            cache_key = %cache_key,
                            model_revision = %self.model_revision,
                            "forecast_cache_hit"
                        );
                        // R11 audit hook.
                        tracing::info!(
                            target: "forecast.audit",
                            kind = "forecast_emitted",
                            correlation_id = %cached.correlation_id,
                            model_revision = %cached.model_revision,
                            direction = ?cached.overlay.direction,
                            confidence = %cached.overlay.confidence,
                            cache_hit = true,
                            inference_ms = 0u64,
                        );
                        // R12 cost hook.
                        tracing::info!(
                            target: "forecast.cost",
                            line = "forecast_inference",
                            usd = 0u64,
                        );
                        // T-D-13 — ForecastEmitted tick (cache-hit, decomp §5A).
                        #[cfg(feature = "audit-tick")]
                        if let Some(l) = self.ledger.as_ref() {
                            audit::tick::emit_public(
                                l,
                                audit::tick::AuditEvent::ForecastEmitted {
                                    overlay: cached.overlay.clone(),
                                    cache_hit: true,
                                },
                            );
                            // Phase D R1.4 — persist to forecast_events alongside tick.
                            if let (Some(sid), Some(sym)) = (
                                self.forecast_strategy_id.as_deref(),
                                self.forecast_symbol.as_deref(),
                            ) && let Err(e) = audit::journal::post_forecast_event(
                                l,
                                &cached.overlay,
                                sid,
                                sym,
                                true,
                            )
                            .await
                            {
                                tracing::warn!(
                                    error = %e,
                                    "post_forecast_event cache-hit failed (non-fatal)"
                                );
                            }
                        }
                        return Ok(cached);
                    }
                    Ok(None) => {
                        if self.strict_replay {
                            return Err(ForecastError::ReplayMiss {
                                hash: cache_key.clone(),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            cache_key = %cache_key,
                            error = %e,
                            "forecast cache read error — running inference"
                        );
                        if self.strict_replay {
                            return Err(ForecastError::ReplayMiss {
                                hash: cache_key.clone(),
                            });
                        }
                    }
                }
            } else if self.strict_replay {
                return Err(ForecastError::ReplayMiss {
                    hash: cache_key.clone(),
                });
            }
        } else if self.strict_replay {
            // strict_replay = true but no cache path configured.
            return Err(ForecastError::ReplayMiss {
                hash: cache_key.clone(),
            });
        }

        // ── Run inference ─────────────────────────────────────────────────────
        // Take the last CONTEXT_LEN bars.
        let bars = &window[window.len() - CONTEXT_LEN..];

        // Build the 5-feature tensor using the proper feature pipeline.
        let feat_cf = build_feature_window_from_ohlcv(bars);

        let x = Tensor::from_vec(feat_cf, (1, 5, CONTEXT_LEN), &self.device)
            .map_err(|e| ForecastError::Inference(e.to_string()))?;

        let y = self
            .forward(&x, false)
            .map_err(|e| ForecastError::Inference(e.to_string()))?;

        let r_hat = y
            .flatten_all()
            .map_err(|e| ForecastError::Inference(e.to_string()))?
            .to_vec1::<f32>()
            .map_err(|e| ForecastError::Inference(e.to_string()))?[0];

        let direction = r_hat_to_direction(r_hat, DIRECTION_EPSILON);

        // R6: confidence = clamp(|r_hat| / sigma_train, 0, 1).
        let confidence_f = (r_hat.abs() / self.sigma_train).clamp(0.0, 1.0);
        let confidence = rust_decimal::Decimal::try_from(f64::from(confidence_f))
            .unwrap_or(rust_decimal::Decimal::ZERO);

        let inference_ms = t_start.elapsed().as_millis() as u64;

        // Use the forecaster's own model_revision, not the request's.
        let effective_model_revision = if self.model_revision == "random-init" {
            request.model_revision.clone()
        } else {
            self.model_revision.clone()
        };

        let overlay = ForecastOverlay {
            correlation_id: request.correlation_id,
            confidence,
            direction,
            horizon_bars: 1,
            model_revision: effective_model_revision.clone(),
            sampled_at: time::OffsetDateTime::now_utc(),
        };

        let response = ForecastResponse {
            correlation_id: request.correlation_id,
            model_revision: effective_model_revision.clone(),
            overlay: overlay.clone(),
            samples: vec![],
        };

        // R11 audit hook — downstream subscribers convert this to JournalEntry.
        tracing::info!(
            target: "forecast.audit",
            kind = "forecast_emitted",
            correlation_id = %request.correlation_id,
            model_revision = %effective_model_revision,
            direction = ?direction,
            confidence = %confidence,
            cache_hit = false,
            inference_ms = inference_ms,
        );

        // R12 cost hook — usd=0 by default (energy_cost_per_kwh=0 per R12).
        tracing::info!(
            target: "forecast.cost",
            line = "forecast_inference",
            usd = 0u64,
        );

        // T-D-13 — ForecastEmitted tick (post-inference, decomp §5A).
        #[cfg(feature = "audit-tick")]
        if let Some(l) = self.ledger.as_ref() {
            audit::tick::emit_public(
                l,
                audit::tick::AuditEvent::ForecastEmitted {
                    overlay: overlay.clone(),
                    cache_hit: false,
                },
            );
            // Phase D R1.4 — persist to forecast_events alongside tick.
            if let (Some(sid), Some(sym)) = (
                self.forecast_strategy_id.as_deref(),
                self.forecast_symbol.as_deref(),
            ) && let Err(e) =
                audit::journal::post_forecast_event(l, &overlay, sid, sym, false).await
            {
                tracing::warn!(
                    error = %e,
                    "post_forecast_event post-inference failed (non-fatal)"
                );
            }
        }

        // ── Write to cache if configured ─────────────────────────────────────
        if let Some(cache_path) = &self.cache_path
            && !self.strict_replay
            && let Ok(cache) = replay_cache::ReplayCache::<
                trading_core::forecast::ForecastRequest,
                ForecastResponse,
            >::open_readwrite(cache_path, "forecast")
            .await
        {
            let req_json = serde_json::to_string(&request).unwrap_or_else(|_| "{}".to_string());
            if let Err(e) = cache.store(&cache_key, &req_json, &response).await {
                tracing::warn!(
                    cache_key = %cache_key,
                    error = %e,
                    "forecast cache write error — result not cached"
                );
            }
        }

        Ok(response)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;

    fn cpu_vb(name: &str) -> VarBuilder<'static> {
        VarBuilder::zeros(DType::F32, &Device::Cpu).pp(name)
    }

    // ── TemporalBlock tests ────────────────────────────────────────────────────

    /// Shape test: block with identity skip (in_ch == out_ch).
    /// Input [1, 96, 256] → Output [1, 96, 256].
    #[test]
    fn temporal_block_shape_identity_skip() {
        let block = TemporalBlock::new(96, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb")).unwrap();
        let x = Tensor::zeros((1, 96, 256), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).unwrap();
        assert_eq!(
            y.dims(),
            [1, 96, 256],
            "identity skip: shape must be preserved"
        );
    }

    /// Shape test: block with 1×1 projection skip (in_ch != out_ch).
    /// Input [1, 5, 256] → Output [1, 96, 256].
    #[test]
    fn temporal_block_shape_projection_skip() {
        let block = TemporalBlock::new(5, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb_proj")).unwrap();
        let x = Tensor::zeros((1, 5, 256), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).unwrap();
        assert_eq!(
            y.dims(),
            [1, 96, 256],
            "projection skip: output shape must be [1, 96, 256]"
        );
    }

    /// Skip identity test: the skip path is not applied through conv layers.
    /// Zero input → all-zero output after relu (zeros + zeros = zeros).
    #[test]
    fn temporal_block_zero_input_identity_skip() {
        let block = TemporalBlock::new(96, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb_zero")).unwrap();
        let x = Tensor::zeros((1, 96, 256), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).unwrap();
        let sum = y.sum_all().unwrap().to_scalar::<f32>().unwrap();
        assert_eq!(sum, 0.0, "zero input through zero-init block → zero output");
    }

    /// Receptive field arithmetic at d=128: BKK18 formula
    /// `RF per block = 1 + 2*(k-1)*d`, total RF = sum over all blocks + 1.
    #[test]
    fn receptive_field_arithmetic() {
        // BKK18 formula: RF = 1 + 2*(k-1) * sum(dilations)
        let sum_dilations: usize = DILATIONS.iter().sum();
        let rf = 1 + 2 * (KERNEL_SIZE - 1) * sum_dilations;
        assert_eq!(sum_dilations, 255, "sum of dilations must be 1+2+…+128=255");
        assert_eq!(rf, 1021, "receptive field must be 1021 bars per BKK18");
        // At hourly cadence: ~42 days (> v1 momentum lookback of 20 bars).
        assert!(rf > 20 * 24, "RF must exceed v1 momentum lookback");
    }

    /// Large dilation (d=128) preserves sequence length.
    #[test]
    fn temporal_block_large_dilation_shape() {
        let block = TemporalBlock::new(96, 96, KERNEL_SIZE, 128, 0.0, cpu_vb("tb_d128")).unwrap();
        let x = Tensor::zeros((2, 96, 256), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [2, 96, 256]);
    }

    // ── TcnModel tests ────────────────────────────────────────────────────────

    /// Full model forward pass: [2, 5, 256] → [2, 1].
    #[test]
    fn tcn_model_forward_shape() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = TcnModel::new(vb).unwrap();
        let x = Tensor::zeros((2, 5, 256), DType::F32, &Device::Cpu).unwrap();
        let y = model.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [2, 1], "TcnModel output must be [2, 1]");
    }

    /// Single-sample forward pass: [1, 5, 256] → [1, 1].
    #[test]
    fn tcn_model_single_sample_shape() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = TcnModel::new(vb).unwrap();
        let x = Tensor::zeros((1, INPUT_FEATURES, CONTEXT_LEN), DType::F32, &Device::Cpu).unwrap();
        let y = model.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [1, 1]);
    }

    /// Trait object boxing test: `TcnForecaster` can be boxed without
    /// object safety issues in the owning struct.
    #[test]
    fn tcn_forecaster_forward_compiles() {
        let forecaster = TcnForecaster::random_init(Device::Cpu).unwrap();
        let x = Tensor::zeros((1, INPUT_FEATURES, CONTEXT_LEN), DType::F32, &Device::Cpu).unwrap();
        let y = forecaster.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [1, 1]);
    }

    /// Mini-config forward test (smaller tensor for faster CI).
    #[test]
    fn tcn_model_mini_config_shape() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = TcnModel::with_config(
            5,          // in_features
            16,         // channels (small)
            3,          // kernel
            &[1, 2, 4], // 3 blocks
            0.0,
            vb,
        )
        .unwrap();
        let x = Tensor::zeros((2, 5, 64), DType::F32, &Device::Cpu).unwrap();
        let y = model.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [2, 1], "mini-config output must be [2, 1]");
    }

    /// Training mode forward (dropout active) should not panic.
    #[test]
    fn tcn_model_train_mode_no_panic() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = TcnModel::with_config(5, 16, 3, &[1, 2], 0.1, vb).unwrap();
        let x = Tensor::zeros((1, 5, 32), DType::F32, &Device::Cpu).unwrap();
        let _y = model.forward(&x, true).unwrap();
    }

    // ── ForecastProvider trait-object boxing test (T-D-6) ─────────────────────

    /// T-D-6: `TcnForecaster` implements `ForecastProvider` and can be boxed.
    #[tokio::test]
    async fn tcn_forecaster_forecast_provider_boxed() {
        use rust_decimal::Decimal;
        use trading_core::forecast::{OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let forecaster: Box<dyn crate::ForecastProvider> =
            Box::new(TcnForecaster::random_init(Device::Cpu).unwrap());

        // Build a window of CONTEXT_LEN bars.
        let bar = OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(102, 0),
            low: Decimal::new(99, 0),
            close: Decimal::new(101, 0),
            volume: Decimal::new(1000, 0),
            ts: time::OffsetDateTime::UNIX_EPOCH,
        };
        let window = vec![bar; CONTEXT_LEN];

        let req = trading_core::forecast::ForecastRequest {
            model_revision: "test-m2".into(),
            ohlcv_window: window,
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };

        let resp = forecaster.forecast(req).await.unwrap();
        assert_eq!(resp.overlay.horizon_bars, 1);
        assert_eq!(resp.correlation_id, Uuid::nil());
    }

    /// T-D-6: forward pass with `[2, 5, 256]` input returns `[2, 1]`.
    #[test]
    fn tcn_forecaster_batch2_shape() {
        let forecaster = TcnForecaster::random_init(Device::Cpu).unwrap();
        let x = Tensor::zeros((2, INPUT_FEATURES, CONTEXT_LEN), DType::F32, &Device::Cpu).unwrap();
        let y = forecaster.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [2, 1], "batch=2 output shape must be [2, 1]");
    }

    // ── T-D-13: M4 inference + replay-cache tests ──────────────────────────────

    /// T-D-13a: Load BS-1 anchor checkpoint, run forecast, assert ForecastOverlay
    /// shape: direction ∈ {Up, Down, Flat}, confidence ∈ [0, 1], horizon_bars = 1,
    /// model_revision matches checkpoint SHA.
    #[tokio::test]
    async fn td13_load_bs1_anchor_forecast_shape() {
        use crate::ForecastProvider;
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;
        use trading_core::forecast::{Direction, OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let forecaster = match TcnForecaster::load_anchor(AnchorScenario::Bs1) {
            Ok(f) => f,
            Err(TcnForecasterError::CheckpointNotFound { path }) => {
                // Checkpoint not present in test env — skip.
                eprintln!(
                    "SKIP td13_load_bs1_anchor_forecast_shape: checkpoint not found at {path}"
                );
                return;
            }
            Err(e) => panic!("unexpected error loading BS-1 anchor: {e}"),
        };

        // Verify model_revision matches the known BS-1 SHA.
        assert_eq!(
            forecaster.model_revision,
            AnchorScenario::Bs1.sha_prefix(),
            "model_revision must match BS-1 SHA"
        );
        assert!(
            forecaster.sigma_train > 0.0,
            "sigma_train must be positive; got {}",
            forecaster.sigma_train
        );

        // Build a fixture OHLCV window of CONTEXT_LEN bars.
        let base_ts = time::OffsetDateTime::UNIX_EPOCH;
        let window: Vec<OhlcvBar> = (0..CONTEXT_LEN)
            .map(|i| {
                let close = Decimal::new(16_500 + i as i64 * 10, 0);
                OhlcvBar {
                    open: close,
                    high: close + dec!(50),
                    low: close - dec!(50),
                    close,
                    volume: Decimal::new(1000, 0),
                    ts: base_ts + time::Duration::hours(i as i64),
                }
            })
            .collect();

        let req = trading_core::forecast::ForecastRequest {
            model_revision: forecaster.model_revision.clone(),
            ohlcv_window: window,
            sampling: SamplingParams::default(),
            correlation_id: Uuid::new_v4(),
        };

        let resp = forecaster
            .forecast(req)
            .await
            .expect("forecast should succeed");

        // ForecastOverlay shape assertions.
        assert_eq!(resp.overlay.horizon_bars, 1, "horizon_bars must be 1");
        assert!(
            matches!(
                resp.overlay.direction,
                Direction::Up | Direction::Down | Direction::Flat
            ),
            "direction must be one of Up/Down/Flat"
        );
        assert!(
            resp.overlay.confidence >= Decimal::ZERO,
            "confidence must be >= 0"
        );
        assert!(
            resp.overlay.confidence <= Decimal::ONE,
            "confidence must be <= 1"
        );
        assert_eq!(
            resp.overlay.model_revision,
            AnchorScenario::Bs1.sha_prefix(),
            "response model_revision must match BS-1 SHA"
        );
    }

    /// T-D-13b: Strict-replay mode returns ReplayMiss on a cache miss.
    #[tokio::test]
    async fn td13_strict_replay_miss_on_empty_cache() {
        use crate::ForecastProvider;
        use rust_decimal::Decimal;
        use trading_core::forecast::{ForecastError, OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let td = tempfile::tempdir().unwrap();
        let cache_path = td.path().join("forecast_test.db");

        // Use a random-init model in strict-replay mode (simulates no inference needed).
        let forecaster = TcnForecaster::random_init(Device::Cpu)
            .unwrap()
            .with_strict_replay(cache_path);

        let bar = OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(102, 0),
            low: Decimal::new(98, 0),
            close: Decimal::new(101, 0),
            volume: Decimal::new(1000, 0),
            ts: time::OffsetDateTime::UNIX_EPOCH,
        };
        let window = vec![bar; CONTEXT_LEN];

        let req = trading_core::forecast::ForecastRequest {
            model_revision: "test-strict-replay".into(),
            ohlcv_window: window,
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };

        let err = forecaster.forecast(req).await.unwrap_err();
        assert!(
            matches!(err, ForecastError::ReplayMiss { .. }),
            "strict_replay mode must return ReplayMiss on empty cache, got {err:?}"
        );
    }

    /// T-D-13c: Same request twice → cache hit on second call.
    #[tokio::test]
    async fn td13_cache_hit_on_second_call() {
        use crate::ForecastProvider;
        use rust_decimal::Decimal;
        use trading_core::forecast::{OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let td = tempfile::tempdir().unwrap();
        let cache_path = td.path().join("forecast_test.db");

        // Use a random-init model with a write-through cache.
        let forecaster = TcnForecaster::random_init(Device::Cpu)
            .unwrap()
            .with_cache(cache_path.clone());

        let bar = OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(102, 0),
            low: Decimal::new(98, 0),
            close: Decimal::new(101, 0),
            volume: Decimal::new(1000, 0),
            ts: time::OffsetDateTime::UNIX_EPOCH,
        };
        let window = vec![bar.clone(); CONTEXT_LEN];

        let make_req = || trading_core::forecast::ForecastRequest {
            model_revision: "test-cache-hit".into(),
            ohlcv_window: window.clone(),
            sampling: SamplingParams::default(),
            correlation_id: Uuid::new_v4(), // different correlation_id each time
        };

        // First call: cache miss → inference runs.
        let resp1 = forecaster.forecast(make_req()).await.expect("first call");

        // Second call with same logical request (same model + window + seed).
        let resp2 = forecaster.forecast(make_req()).await.expect("second call");

        // Both responses should have the same overlay fields (direction, confidence, horizon_bars).
        assert_eq!(
            resp1.overlay.direction, resp2.overlay.direction,
            "direction must be identical on cache hit"
        );
        assert_eq!(
            resp1.overlay.confidence, resp2.overlay.confidence,
            "confidence must be identical on cache hit"
        );
        assert_eq!(resp1.overlay.horizon_bars, 1);
        assert_eq!(resp2.overlay.horizon_bars, 1);
    }

    /// T-D-13d: Different model_revision → different cache key (cache miss).
    #[tokio::test]
    async fn td13_different_model_revision_cache_miss() {
        use crate::ForecastProvider;
        use rust_decimal::Decimal;
        use trading_core::forecast::{OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let td = tempfile::tempdir().unwrap();
        let cache_path = td.path().join("forecast_test.db");

        // First forecaster with model_revision "rev-a".
        let mut forecaster_a = TcnForecaster::random_init(Device::Cpu).unwrap();
        forecaster_a.model_revision = "rev-a".to_string();
        let forecaster_a = forecaster_a.with_cache(cache_path.clone());

        let bar = OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(102, 0),
            low: Decimal::new(98, 0),
            close: Decimal::new(101, 0),
            volume: Decimal::new(1000, 0),
            ts: time::OffsetDateTime::UNIX_EPOCH,
        };
        let window = vec![bar; CONTEXT_LEN];

        let req_a = trading_core::forecast::ForecastRequest {
            model_revision: "rev-a".into(),
            ohlcv_window: window.clone(),
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };

        // Cache rev-a.
        forecaster_a
            .forecast(req_a)
            .await
            .expect("first call rev-a");

        // Second forecaster with model_revision "rev-b" — different cache key.
        let mut forecaster_b = TcnForecaster::random_init(Device::Cpu).unwrap();
        forecaster_b.model_revision = "rev-b".to_string();
        let forecaster_b = forecaster_b.with_strict_replay(cache_path.clone());

        let req_b = trading_core::forecast::ForecastRequest {
            model_revision: "rev-b".into(),
            ohlcv_window: window,
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };

        // rev-b is not in cache → strict_replay returns ReplayMiss.
        let err = forecaster_b.forecast(req_b).await.unwrap_err();
        assert!(
            matches!(
                err,
                trading_core::forecast::ForecastError::ReplayMiss { .. }
            ),
            "different model_revision must yield a cache miss, got {err:?}"
        );
    }

    /// T-D-13e: AnchorScenario::Bs2 SHA prefix matches the known value.
    #[test]
    fn td13_anchor_scenario_sha_prefix() {
        assert_eq!(
            AnchorScenario::Bs1.sha_prefix(),
            "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2"
        );
        assert_eq!(
            AnchorScenario::Bs2.sha_prefix(),
            "3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d"
        );
    }
}
