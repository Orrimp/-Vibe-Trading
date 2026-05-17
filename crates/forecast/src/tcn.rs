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
//! ## Cross-references
//!
//! - `spec/v25-tcn-overlay/feature.md § D1` — block layout spec
//! - `spec/v25-tcn-overlay/feature.md § R1` — topology
//! - `spec/v25-tcn-overlay/feature.md § R2` — model size
//! - `ADR-0029` — checkpoint provenance contract

use candle_core::{DType, Device, Module, Result as CResult, Tensor};
use candle_nn::{Conv1d, Conv1dConfig, Dropout, VarBuilder};

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

/// The `TcnForecaster` wraps a `TcnModel` and implements `ForecastProvider`.
///
/// At M2 this is a random-init-only stub for smoke-testing the forward pass
/// and checkpoint write.  Full inference (load anchor checkpoint, build
/// feature window, emit `ForecastOverlay`) lands at M4.
pub struct TcnForecaster {
    pub model: TcnModel,
    pub device: Device,
}

impl TcnForecaster {
    /// Construct with a random-initialised model on the given device.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn random_init(device: Device) -> CResult<Self> {
        let vb =
            VarBuilder::zeros(DType::F32, &device);
        let model = TcnModel::new(vb)?;
        Ok(Self { model, device })
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

/// Convert `OhlcvBar` (Decimal fields) to a `[5, 1]` row for the feature
/// pipeline.  This is a simplified version for the M2 stub — the full feature
/// pipeline (logret, logrange, logvol_z, hour_sin, hour_cos) is used in
/// `features.rs` for training.
fn ohlcv_to_feature_row(bar: &OhlcvBar) -> [f32; 5] {
    use rust_decimal::prelude::ToPrimitive;
    let close = bar.close.to_f32().unwrap_or(0.0);
    let high = bar.high.to_f32().unwrap_or(0.0);
    let low = bar.low.to_f32().unwrap_or(0.0);
    let vol = bar.volume.to_f32().unwrap_or(0.0);
    // Simplified features for M2 stub (full feature construction in features.rs)
    let logrange = if close > 0.0 {
        (1.0 + (high - low) / close).ln()
    } else {
        0.0
    };
    let log_vol = (1.0 + vol).ln();
    [0.0_f32, logrange, log_vol, 0.0, 0.0]
}

#[async_trait]
impl crate::ForecastProvider for TcnForecaster {
    /// Run the TCN forward pass over the OHLCV window and return a
    /// `ForecastOverlay`.
    ///
    /// At M2 this is a smoke-test-only stub: it builds a simplified feature
    /// tensor from the `ohlcv_window` (not using the full `features.rs`
    /// pipeline), runs the forward pass, and emits a `ForecastOverlay`.
    ///
    /// Full inference (load anchor checkpoint, full feature construction,
    /// replay-cache lookup, audit emission) lands at M4.
    ///
    /// # Errors
    ///
    /// Returns `ForecastError::InvalidInput` if the window is empty or shorter
    /// than `CONTEXT_LEN`.  Returns `ForecastError::Inference` on candle
    /// tensor errors.
    async fn forecast(
        &self,
        request: ForecastRequest,
    ) -> Result<ForecastResponse, ForecastError> {
        let window = &request.ohlcv_window;

        if window.len() < CONTEXT_LEN {
            return Err(ForecastError::InvalidInput(format!(
                "ohlcv_window has {} bars; need {}",
                window.len(),
                CONTEXT_LEN
            )));
        }

        // Take the last CONTEXT_LEN bars.
        let bars = &window[window.len() - CONTEXT_LEN..];

        // Build a [1, 5, CONTEXT_LEN] feature tensor (simplified M2 stub).
        let mut feat: Vec<f32> = Vec::with_capacity(5 * CONTEXT_LEN);
        for bar in bars {
            let row = ohlcv_to_feature_row(bar);
            for &f in &row {
                feat.push(f);
            }
        }

        // Transpose from [CONTEXT_LEN, 5] to [5, CONTEXT_LEN] (channel-first).
        let mut feat_cf: Vec<f32> = vec![0.0; 5 * CONTEXT_LEN];
        for t in 0..CONTEXT_LEN {
            for c in 0..5 {
                feat_cf[c * CONTEXT_LEN + t] = feat[t * 5 + c];
            }
        }

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

        // Confidence = clamp(|r_hat| / sigma_train, 0, 1).
        // At M2 sigma_train = 1.0 (placeholder; pinned at training time per R6).
        let sigma_train = 1.0_f32;
        let confidence_f = (r_hat.abs() / sigma_train).clamp(0.0, 1.0);
        let confidence = rust_decimal::Decimal::try_from(f64::from(confidence_f))
            .unwrap_or(rust_decimal::Decimal::ZERO);

        let overlay = ForecastOverlay {
            correlation_id: request.correlation_id,
            confidence,
            direction,
            horizon_bars: 1,
            model_revision: request.model_revision.clone(),
            sampled_at: time::OffsetDateTime::now_utc(),
        };

        // M2 stub: no distribution samples.
        Ok(ForecastResponse {
            correlation_id: request.correlation_id,
            model_revision: request.model_revision,
            overlay,
            samples: vec![],
        })
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
        let block =
            TemporalBlock::new(96, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb")).unwrap();
        let x = Tensor::zeros((1, 96, 256), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [1, 96, 256], "identity skip: shape must be preserved");
    }

    /// Shape test: block with 1×1 projection skip (in_ch != out_ch).
    /// Input [1, 5, 256] → Output [1, 96, 256].
    #[test]
    fn temporal_block_shape_projection_skip() {
        let block =
            TemporalBlock::new(5, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb_proj")).unwrap();
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
        let block =
            TemporalBlock::new(96, 96, KERNEL_SIZE, 1, 0.0, cpu_vb("tb_zero")).unwrap();
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
        let block =
            TemporalBlock::new(96, 96, KERNEL_SIZE, 128, 0.0, cpu_vb("tb_d128")).unwrap();
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
        let x = Tensor::zeros((1, INPUT_FEATURES, CONTEXT_LEN), DType::F32, &Device::Cpu)
            .unwrap();
        let y = forecaster.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [1, 1]);
    }

    /// Mini-config forward test (smaller tensor for faster CI).
    #[test]
    fn tcn_model_mini_config_shape() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = TcnModel::with_config(
            5,  // in_features
            16, // channels (small)
            3,  // kernel
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
        use crate::ForecastProvider;
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
        let x = Tensor::zeros((2, INPUT_FEATURES, CONTEXT_LEN), DType::F32, &Device::Cpu)
            .unwrap();
        let y = forecaster.forward(&x, false).unwrap();
        assert_eq!(y.dims(), [2, 1], "batch=2 output shape must be [2, 1]");
    }
}
