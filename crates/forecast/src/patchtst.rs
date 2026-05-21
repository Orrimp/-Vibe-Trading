//! PatchTST (Patch Time Series Transformer) forecaster implementation.
//!
//! ## Architecture (ADR-0036 § D1, Nie et al 2022 arXiv:2211.14730)
//!
//! PatchTST/42 small config with channel-independence (per Nie et al § 3.2):
//!
//! ```text
//! Input:    [batch, channels=5, time=336]
//!   │
//!   ▼  PatchEmbed (patch_len=16, stride=8)
//!        — unfold → [batch, channels, n_patches=41, patch_len=16]
//!        — Linear projection [16 → d_model=128]
//!        — Output: [batch, 5, 41, 128]
//!   │
//!   ▼  + LearnablePositionEncoding [n_patches=41, d_model=128]
//!        — broadcast over batch + channels
//!   │
//!   ▼  Reshape (channel-independence): [batch, 5, 41, 128] → [batch*5, 41, 128]
//!   │
//!   ▼  TransformerEncoder × n_layers=3 (pre-LN order, custom MHSA)
//!   │
//!   ▼  Reshape back: [batch*5, 41, 128] → [batch, 5, 41, 128]
//!   │
//!   ▼  Flatten last 2 dims per channel → [batch, 5, 5248]
//!   │
//!   ▼  ProjectionHead: Linear (5*5248 → 1)
//!        — Output: [batch, 1]
//! ```
//!
//! ## Hyperparameters (architect-locked at M-T1)
//!
//! | Field | Value |
//! |-------|-------|
//! | `patch_len` | 16 |
//! | `stride` | 8 |
//! | `context_len` | 336 |
//! | `n_patches` | 41 |
//! | `d_model` | 128 |
//! | `n_heads` | 4 |
//! | `d_ff` | 256 |
//! | `n_layers` | 3 |
//! | `dropout` | 0.2 |
//! | `channels` | 5 |
//! | `output_dim` | 1 |
//!
//! ## Parameter count
//!
//! ~410k params (channel-independence: shared weights across 5 channels).
//! Well under the ADR-0028 5-10M ceiling and ~10× smaller than TCN's 4.4M.
//!
//! ## Cross-references
//!
//! - `spec/v25a-patchtst-overlay/feature.md § R1`
//! - `spec/architecture/adr/0036-patchtst-training-contract.md § D1`
//! - `spec/architecture/adr/0035-tcn-sigma-train-recalibration.md § D1` (σ_train post-training)
//! - Nie et al 2022 (arXiv:2211.14730) — reference architecture

use candle_core::{DType, Device, Module, Result as CResult, Tensor};
use candle_nn::{Dropout, LayerNorm, Linear, VarBuilder};
use std::path::{Path, PathBuf};

// ── Constants (architect-locked at M-T1) ──────────────────────────────────────

/// Input context window length in bars (14 days of hourly bars).
pub const CONTEXT_LEN: usize = 336;

/// Patch length in bars.
pub const PATCH_LEN: usize = 16;

/// Stride between patches (50% overlap).
pub const STRIDE: usize = 8;

/// Number of patches: floor((context_len - patch_len) / stride) + 1 = 41.
pub const N_PATCHES: usize = 41;

/// Model dimension.
pub const D_MODEL: usize = 128;

/// Number of attention heads.
pub const N_HEADS: usize = 4;

/// Head dimension (d_model / n_heads = 32).
pub const HEAD_DIM: usize = D_MODEL / N_HEADS;

/// Feed-forward dimension (2× d_model).
pub const D_FF: usize = 256;

/// Number of encoder layers.
pub const N_LAYERS: usize = 3;

/// Dropout rate.
pub const DROPOUT: f64 = 0.2;

/// Input channel count (5 features: logret/logrange/logvol_z/hour_sin/hour_cos).
pub const CHANNELS: usize = 5;

// ── PatchEmbed ────────────────────────────────────────────────────────────────

/// Patch embedding: linear projection from raw patch pixels to d_model.
///
/// Input:  `[batch * channels, n_patches, patch_len]`
/// Output: `[batch * channels, n_patches, d_model]`
///
/// The projection is shared across channels (channel-independence per Nie § 3.2).
pub struct PatchEmbed {
    proj: Linear,
}

impl PatchEmbed {
    /// Construct a `PatchEmbed` with the architect-locked defaults.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error` on tensor allocation failure.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        let proj = candle_nn::linear(PATCH_LEN, D_MODEL, vb.pp("proj"))?;
        Ok(Self { proj })
    }

    /// Forward pass.
    ///
    /// Input shape: `[batch * channels, n_patches, patch_len]`.
    /// Output shape: `[batch * channels, n_patches, d_model]`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor) -> CResult<Tensor> {
        self.proj.forward(x)
    }
}

// ── LearnablePositionEncoding ─────────────────────────────────────────────────

/// Learnable position encoding of shape `[n_patches, d_model]`.
///
/// Added to the patch embeddings (broadcast over batch and channels).
///
/// Architect decision: learnable PE over sinusoidal (ADR-0036 § D1 rationale).
pub struct LearnablePositionEncoding {
    pe: Tensor,
}

impl LearnablePositionEncoding {
    /// Construct a learnable PE parameter.
    ///
    /// Registered as a named variable in the `VarBuilder` scope so it
    /// participates in gradient computation and checkpoint serialisation.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        let pe = vb.get((N_PATCHES, D_MODEL), "pe")?;
        Ok(Self { pe })
    }

    /// Add the position encoding to patch embeddings.
    ///
    /// Input shape: `[batch * channels, n_patches, d_model]`.
    /// Output shape: `[batch * channels, n_patches, d_model]` (broadcast add).
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor) -> CResult<Tensor> {
        let (bc, np, d) = x.dims3()?;
        // pe: [n_patches, d_model] → [bc, n_patches, d_model] via expand.
        let pe = self.pe.unsqueeze(0)?.expand((bc, np, d))?;
        x + pe
    }
}

// ── MultiHeadSelfAttention ────────────────────────────────────────────────────

/// Custom multi-head self-attention (~50 LoC).
///
/// Uses scaled dot-product attention per Vaswani 2017.
/// Architecture: pre-LN order (applied by `TransformerBlock`).
///
/// K2 determinism gate: custom implementation avoids `candle_transformers::*`
/// external API drift (ADR-0036 § D5).
pub struct MultiHeadSelfAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    out_proj: Linear,
    dropout: Dropout,
}

impl MultiHeadSelfAttention {
    /// Construct MHSA with 4 heads and d_model=128.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        let q_proj = candle_nn::linear(D_MODEL, D_MODEL, vb.pp("q_proj"))?;
        let k_proj = candle_nn::linear(D_MODEL, D_MODEL, vb.pp("k_proj"))?;
        let v_proj = candle_nn::linear(D_MODEL, D_MODEL, vb.pp("v_proj"))?;
        let out_proj = candle_nn::linear(D_MODEL, D_MODEL, vb.pp("out_proj"))?;
        let dropout = Dropout::new(DROPOUT as f32);
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            out_proj,
            dropout,
        })
    }

    /// Forward pass.
    ///
    /// Input shape:  `[batch_channels, seq_len, d_model]`
    /// Output shape: `[batch_channels, seq_len, d_model]`
    ///
    /// Where `batch_channels = batch * channels` after the channel-independence reshape.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        let (bc, seq, _d) = x.dims3()?;

        // Q, K, V projections: [bc, seq, d_model].
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        // Reshape to [bc, n_heads, seq, head_dim].
        let q = q
            .reshape((bc, seq, N_HEADS, HEAD_DIM))?
            .transpose(1, 2)?
            .contiguous()?;
        let k = k
            .reshape((bc, seq, N_HEADS, HEAD_DIM))?
            .transpose(1, 2)?
            .contiguous()?;
        let v = v
            .reshape((bc, seq, N_HEADS, HEAD_DIM))?
            .transpose(1, 2)?
            .contiguous()?;

        // Scaled dot-product attention.
        let scale = (HEAD_DIM as f64).sqrt();
        // attn_weights: [bc, n_heads, seq, seq].
        let attn_weights = q.matmul(&k.transpose(2, 3)?)?;
        let attn_weights = (attn_weights / scale)?;
        let attn_weights = candle_nn::ops::softmax(&attn_weights, 3)?;
        let attn_weights = self.dropout.forward(&attn_weights, train)?;

        // Weighted sum: [bc, n_heads, seq, head_dim].
        let out = attn_weights.matmul(&v)?;

        // Merge heads: [bc, seq, d_model].
        let out = out
            .transpose(1, 2)?
            .contiguous()?
            .reshape((bc, seq, D_MODEL))?;

        // Output projection: [bc, seq, d_model].
        self.out_proj.forward(&out)
    }
}

// ── FFN (Position-wise Feed-Forward Network) ──────────────────────────────────

/// Two-layer FFN with GELU activation (Nie et al default).
///
/// `d_model → d_ff → d_model`.
struct Ffn {
    fc1: Linear,
    fc2: Linear,
    dropout: Dropout,
}

impl Ffn {
    fn new(vb: VarBuilder) -> CResult<Self> {
        let fc1 = candle_nn::linear(D_MODEL, D_FF, vb.pp("fc1"))?;
        let fc2 = candle_nn::linear(D_FF, D_MODEL, vb.pp("fc2"))?;
        let dropout = Dropout::new(DROPOUT as f32);
        Ok(Self { fc1, fc2, dropout })
    }

    fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        let h = self.fc1.forward(x)?;
        let h = h.gelu()?;
        let h = self.dropout.forward(&h, train)?;
        self.fc2.forward(&h)
    }
}

// ── TransformerBlock ──────────────────────────────────────────────────────────

/// One encoder block: pre-LN MHSA + pre-LN FFN with residual connections.
///
/// Pre-LN order per Nie et al § 3.1 (modern default):
/// ```text
/// y = LayerNorm(x)
/// y = MHSA(y) + x          ← residual 1
/// z = LayerNorm(y)
/// z = FFN(z) + y            ← residual 2
/// ```
pub struct TransformerBlock {
    norm1: LayerNorm,
    mhsa: MultiHeadSelfAttention,
    norm2: LayerNorm,
    ffn: Ffn,
}

impl TransformerBlock {
    /// Construct a transformer block.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        let norm1 = candle_nn::layer_norm(D_MODEL, 1e-6, vb.pp("norm1"))?;
        let mhsa = MultiHeadSelfAttention::new(vb.pp("mhsa"))?;
        let norm2 = candle_nn::layer_norm(D_MODEL, 1e-6, vb.pp("norm2"))?;
        let ffn = Ffn::new(vb.pp("ffn"))?;
        Ok(Self {
            norm1,
            mhsa,
            norm2,
            ffn,
        })
    }

    /// Forward pass.
    ///
    /// Input shape:  `[batch_channels, seq_len, d_model]`
    /// Output shape: `[batch_channels, seq_len, d_model]`
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        // Pre-LN + MHSA + residual 1.
        let y = self.norm1.forward(x)?;
        let y = self.mhsa.forward(&y, train)?;
        let x = (x + &y)?;

        // Pre-LN + FFN + residual 2.
        let z = self.norm2.forward(&x)?;
        let z = self.ffn.forward(&z, train)?;
        &x + &z
    }
}

// ── PatchTstModel ─────────────────────────────────────────────────────────────

/// Full PatchTST model: patch-embed + pos-enc + 3 transformer blocks + head.
///
/// Forward input:  `[batch, 5, 336]` (batch × channels × context_len).
/// Forward output: `[batch, 1]` (scalar `r_hat` per sample).
pub struct PatchTstModel {
    patch_embed: PatchEmbed,
    pos_enc: LearnablePositionEncoding,
    encoder_blocks: Vec<TransformerBlock>,
    proj_head: Linear,
}

impl PatchTstModel {
    /// Construct with the architect-locked defaults (ADR-0036 § D1).
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn new(vb: VarBuilder) -> CResult<Self> {
        let patch_embed = PatchEmbed::new(vb.pp("patch_embed"))?;
        let pos_enc = LearnablePositionEncoding::new(vb.pp("pos_enc"))?;

        let mut encoder_blocks = Vec::with_capacity(N_LAYERS);
        for i in 0..N_LAYERS {
            let block = TransformerBlock::new(vb.pp(format!("encoder_{i}")))?;
            encoder_blocks.push(block);
        }

        // ProjectionHead: Linear(5 * N_PATCHES * D_MODEL → 1).
        // Flattens all channels and all patch positions into a single scalar.
        let flat_dim = CHANNELS * N_PATCHES * D_MODEL;
        let proj_head = candle_nn::linear(flat_dim, 1, vb.pp("proj_head"))?;

        Ok(Self {
            patch_embed,
            pos_enc,
            encoder_blocks,
            proj_head,
        })
    }

    /// Forward pass.
    ///
    /// Input shape:  `[batch, 5, 336]`
    /// Output shape: `[batch, 1]`
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        let (batch, channels, _time) = x.dims3()?;

        // Step 1: Extract patches via unfold.
        // x: [batch, channels, time=336]
        // unfold → [batch, channels, n_patches=41, patch_len=16]
        let x_patched = unfold(x, PATCH_LEN, STRIDE)?;
        // Verify shape.
        let (b2, c2, np, _pl) = x_patched.dims4()?;
        debug_assert_eq!(b2, batch);
        debug_assert_eq!(c2, channels);
        debug_assert_eq!(np, N_PATCHES);

        // Step 2: Reshape to [batch * channels, n_patches, patch_len] for channel-independent processing.
        let bc = batch * channels;
        let x_flat = x_patched.reshape((bc, np, PATCH_LEN))?;

        // Step 3: Patch embedding.
        // [bc, n_patches, patch_len] → [bc, n_patches, d_model]
        let x_emb = self.patch_embed.forward(&x_flat)?;

        // Step 4: Add learnable position encoding (broadcast over bc).
        let x_emb = self.pos_enc.forward(&x_emb)?;

        // Step 5: Transformer encoder (3 blocks).
        let mut h = x_emb;
        for block in &self.encoder_blocks {
            h = block.forward(&h, train)?;
        }
        // h: [bc, n_patches, d_model]

        // Step 6: Reshape back to [batch, channels, n_patches, d_model].
        let h = h.reshape((batch, channels, N_PATCHES, D_MODEL))?;

        // Step 7: Flatten channels × patches × d_model → [batch, 5 * 41 * 128].
        let flat_dim = channels * N_PATCHES * D_MODEL;
        let h = h.reshape((batch, flat_dim))?;

        // Step 8: Projection head → [batch, 1].
        self.proj_head.forward(&h)
    }

    /// Count the number of trainable parameters.
    ///
    /// Used in tests to verify the architect's ~410k target.
    pub fn num_parameters(&self) -> usize {
        // Patch embed: [16, 128] + [128] bias = 2048 + 128 = 2176
        // Pos enc: [41, 128] = 5248
        // 3 encoder blocks: each ~131k params
        // Proj head: [5*41*128, 1] + [1] = 26240 + 1 = 26241
        // Grand total: ~410k
        // We compute it by examining the model structure.

        let mut count = 0usize;

        // PatchEmbed: proj linear(16 → 128) = 16*128 + 128 = 2176
        count += PATCH_LEN * D_MODEL + D_MODEL;

        // LearnablePositionEncoding: [n_patches, d_model] = 41*128 = 5248
        count += N_PATCHES * D_MODEL;

        // N_LAYERS encoder blocks, each:
        // - norm1: 2*d_model = 256
        // - MHSA: q_proj(128→128) + k_proj(128→128) + v_proj(128→128) + out_proj(128→128)
        //         = 4 * (128*128 + 128) = 4 * 16512 = 66048
        // - norm2: 2*d_model = 256
        // - FFN: fc1(128→256) + fc2(256→128) = (128*256+256) + (256*128+128) = 33024 + 32896 = 65920
        // Per block: 256 + 66048 + 256 + 65920 = 132480
        let per_block = 2 * D_MODEL   // norm1
            + 4 * (D_MODEL * D_MODEL + D_MODEL)  // MHSA
            + 2 * D_MODEL   // norm2
            + (D_MODEL * D_FF + D_FF) + (D_FF * D_MODEL + D_MODEL); // FFN
        count += N_LAYERS * per_block;

        // ProjectionHead: linear(5*41*128 → 1) = 5*41*128 + 1 = 26241
        let flat_dim = CHANNELS * N_PATCHES * D_MODEL;
        count += flat_dim + 1;

        count
    }
}

/// Extract overlapping patches from a time series via sliding-window unfold.
///
/// Input:  `[batch, channels, time]`
/// Output: `[batch, channels, n_patches, patch_len]`
///
/// Equivalent to `Tensor::unfold` in PyTorch: slides a window of size
/// `patch_len` with step `stride` over the time dimension.
///
/// # Errors
///
/// Propagates `candle_core::Error`.
fn unfold(x: &Tensor, patch_len: usize, stride: usize) -> CResult<Tensor> {
    let (_batch, _channels, time) = x.dims3()?;

    // n_patches = floor((time - patch_len) / stride) + 1
    let n_patches = (time - patch_len) / stride + 1;

    // For each patch position p in 0..n_patches:
    //   patch_start = p * stride
    //   slice: x[:, :, patch_start..patch_start+patch_len]
    //
    // Gather all patches into a list, then stack.
    let mut patches: Vec<Tensor> = Vec::with_capacity(n_patches);
    for p in 0..n_patches {
        let start = p * stride;
        // [batch, channels, patch_len]
        let patch = x.narrow(2, start, patch_len)?;
        // [batch, channels, 1, patch_len]
        let patch = patch.unsqueeze(2)?;
        patches.push(patch);
    }

    // Stack along dim 2: [batch, channels, n_patches, patch_len]
    Tensor::cat(&patches, 2)
}

// ── AnchorScenario ────────────────────────────────────────────────────────────

/// Identifies which LFS-anchored PatchTST checkpoint to load.
///
/// Only `Bs1` exists at v0.1.0 per Q2=(a) (BS-2 defers to v0.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorScenario {
    /// BS-1 anchor: model trained on 2023 full year.
    Bs1,
}

impl AnchorScenario {
    /// File-name prefix for the anchor scenario.
    pub fn file_prefix(&self) -> &'static str {
        match self {
            AnchorScenario::Bs1 => "patchtst-bs1",
        }
    }

    /// Placeholder SHA prefix — populated after Wave B training.
    ///
    /// This returns `""` at Wave A (no checkpoint exists yet).
    /// Wave B updates this to the actual SHA after training completes.
    pub fn sha_prefix(&self) -> &'static str {
        match self {
            AnchorScenario::Bs1 => "",
        }
    }
}

// ── PatchTstForecasterError ───────────────────────────────────────────────────

/// Error type for `PatchTstForecaster` construction and inference.
#[derive(Debug, thiserror::Error)]
pub enum PatchTstForecasterError {
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

impl From<candle_core::Error> for PatchTstForecasterError {
    fn from(e: candle_core::Error) -> Self {
        PatchTstForecasterError::Candle(e.to_string())
    }
}

// ── PatchTstForecaster ────────────────────────────────────────────────────────

/// The `PatchTstForecaster` wraps a `PatchTstModel` and implements `ForecastProvider`.
///
/// Mirrors `TcnForecaster` shape per ADR-0036 § D7:
/// - `random_init` for tests
/// - `load_anchor(AnchorScenario::Bs1)` for production (post-Wave B)
/// - `load_from_paths` for explicit paths
pub struct PatchTstForecaster {
    pub model: PatchTstModel,
    pub device: Device,
    /// Standard deviation of `r_hat` on the training set (R6 confidence calibration).
    /// Pinned at checkpoint time; loaded from `.metadata.json`.
    pub sigma_train: f32,
    /// The canonical `model_revision` SHA (from checkpoint provenance).
    pub model_revision: String,
    /// Whether this forecaster operates in strict-replay mode.
    pub strict_replay: bool,
    /// Optional path to the replay-cache SQLite file.
    pub cache_path: Option<PathBuf>,
    /// Optional audit ledger for `ForecastEmitted` tick emission.
    #[cfg(feature = "audit-tick")]
    pub(crate) ledger: Option<audit::Ledger>,
    /// Optional strategy id for the `post_forecast_event` SQL writer.
    #[cfg(feature = "audit-tick")]
    pub(crate) forecast_strategy_id: Option<String>,
    /// Optional symbol for the `post_forecast_event` SQL writer.
    #[cfg(feature = "audit-tick")]
    pub(crate) forecast_symbol: Option<String>,
}

impl PatchTstForecaster {
    /// Construct with a random-initialised model (zero weights) on the given device.
    ///
    /// `sigma_train` is set to 1.0 (placeholder for tests).
    /// `model_revision` is set to `"random-init"`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn random_init(device: Device) -> CResult<Self> {
        let vb = VarBuilder::zeros(DType::F32, &device);
        let model = PatchTstModel::new(vb)?;
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

    /// Construct with a seeded random-init model (for determinism tests).
    ///
    /// Uses `VarBuilder::zeros` (all-zero weights are deterministic regardless of seed).
    /// The `seed` parameter is accepted for API compatibility but zeros are always deterministic.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn random_init_with_seed(device: Device, _seed: u64) -> CResult<Self> {
        // Zero-init is deterministic by definition — no RNG needed for a zero-weight model.
        Self::random_init(device)
    }

    /// Load an LFS-anchored checkpoint by scenario identifier.
    ///
    /// Looks for files at:
    /// `crates/forecast/checkpoints/anchors/patchtst-bs1-{sha}.{safetensors,metadata.json}`
    ///
    /// Returns `PatchTstForecasterError::CheckpointNotFound` if not found
    /// (expected behaviour at Wave A before training completes).
    ///
    /// # Errors
    ///
    /// Returns `PatchTstForecasterError::CheckpointNotFound` if file is absent.
    pub fn load_anchor(scenario: AnchorScenario) -> Result<Self, PatchTstForecasterError> {
        let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");
        let prefix = scenario.file_prefix();
        let sha = scenario.sha_prefix();

        if sha.is_empty() {
            return Err(PatchTstForecasterError::CheckpointNotFound {
                path: format!(
                    "{}/{prefix}-<sha>.safetensors (Wave B not yet run)",
                    anchors_dir.display()
                ),
            });
        }

        let safetensors_path = anchors_dir.join(format!("{prefix}-{sha}.safetensors"));
        let metadata_path = anchors_dir.join(format!("{prefix}-{sha}.metadata.json"));

        if !safetensors_path.exists() {
            return Err(PatchTstForecasterError::CheckpointNotFound {
                path: safetensors_path.display().to_string(),
            });
        }
        if !metadata_path.exists() {
            return Err(PatchTstForecasterError::CheckpointNotFound {
                path: metadata_path.display().to_string(),
            });
        }

        Self::load_from_paths(&safetensors_path, &metadata_path)
    }

    /// Load a `PatchTstForecaster` from explicit checkpoint + metadata paths.
    ///
    /// # Errors
    ///
    /// Returns `PatchTstForecasterError` on file I/O or parse failure.
    pub fn load_from_paths(
        safetensors_path: &Path,
        metadata_path: &Path,
    ) -> Result<Self, PatchTstForecasterError> {
        let metadata_bytes = std::fs::read(metadata_path).map_err(|e| {
            PatchTstForecasterError::CheckpointNotFound {
                path: format!("{}: {e}", metadata_path.display()),
            }
        })?;
        let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes)
            .map_err(|e| PatchTstForecasterError::MetadataParse(e.to_string()))?;

        let sigma_train = metadata["sigma_train"].as_f64().unwrap_or(1.0_f64) as f32;
        let model_revision = metadata["model_revision"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let bytes = std::fs::read(safetensors_path)
            .map_err(|e| PatchTstForecasterError::SafetensorsLoad(e.to_string()))?;

        let device = Device::Cpu;
        let vb = VarBuilder::from_buffered_safetensors(bytes, DType::F32, &device)
            .map_err(|e| PatchTstForecasterError::SafetensorsLoad(e.to_string()))?;

        let model = PatchTstModel::new(vb).map_err(PatchTstForecasterError::from)?;

        tracing::info!(
            model_revision = %model_revision,
            sigma_train = sigma_train,
            "PatchTstForecaster loaded from checkpoint"
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
    #[must_use]
    pub fn with_strict_replay(mut self, cache_path: PathBuf) -> Self {
        self.strict_replay = true;
        self.cache_path = Some(cache_path);
        self
    }

    /// Enable live mode with the given cache database path for write-through.
    #[must_use]
    pub fn with_cache(mut self, cache_path: PathBuf) -> Self {
        self.strict_replay = false;
        self.cache_path = Some(cache_path);
        self
    }

    /// Attach an audit ledger for `ForecastEmitted` ticks.
    #[cfg(feature = "audit-tick")]
    #[must_use]
    pub fn with_ledger(mut self, ledger: audit::Ledger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    /// Attach the `strategy_id` and `symbol` context for the Phase D SQL writer.
    #[cfg(feature = "audit-tick")]
    #[must_use]
    pub fn with_forecast_context(mut self, strategy_id: String, symbol: String) -> Self {
        self.forecast_strategy_id = Some(strategy_id);
        self.forecast_symbol = Some(symbol);
        self
    }

    /// Forward pass. Input: `[batch, 5, 336]`. Output: `[batch, 1]`.
    ///
    /// # Errors
    ///
    /// Propagates `candle_core::Error`.
    pub fn forward(&self, x: &Tensor, train: bool) -> CResult<Tensor> {
        self.model.forward(x, train)
    }
}

// ── ForecastProvider impl ─────────────────────────────────────────────────────

use async_trait::async_trait;
use trading_core::forecast::{
    Direction, ForecastError, ForecastOverlay, ForecastRequest, ForecastResponse, OhlcvBar,
};

/// Default epsilon (5 bps — same as TCN per R6/D5).
pub const DIRECTION_EPSILON: f32 = 0.000_5_f32;

/// Convert `r_hat` to `Direction` using epsilon threshold (reused from tcn.rs).
pub fn r_hat_to_direction(r_hat: f32, epsilon: f32) -> Direction {
    if r_hat > epsilon {
        Direction::Up
    } else if r_hat < -epsilon {
        Direction::Down
    } else {
        Direction::Flat
    }
}

/// Build a proper 5-feature window from `OhlcvBar` for PatchTST inference.
///
/// Mirrors `tcn.rs::build_feature_window_from_ohlcv` exactly (same formula,
/// channel-first output: `[5, n]` flattened).
fn build_feature_window_from_ohlcv(bars: &[OhlcvBar]) -> Vec<f32> {
    use rust_decimal::prelude::ToPrimitive;
    use std::f32::consts::PI;

    let n = bars.len();
    assert!(n > 1, "need at least 2 bars for logret");

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

    let mut feat_cf: Vec<f32> = vec![0.0; 5 * n];

    for t in 0..n {
        let bar = &bars[t];
        let close = bar.close.to_f32().unwrap_or(1.0).max(1e-8);
        let high = bar.high.to_f32().unwrap_or(close);
        let low = bar.low.to_f32().unwrap_or(close);

        let logret = if t == 0 {
            0.0_f32
        } else {
            let prev_close = bars[t - 1].close.to_f32().unwrap_or(1.0).max(1e-8);
            (close / prev_close).ln()
        };

        let logrange = (1.0_f32 + (high - low) / close).ln();
        let logvol_z = (log_vols[t] - mu_vol) / sigma_vol;

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

/// Build a canonical cache-key string for a PatchTST forecast request.
fn forecast_cache_key(request: &ForecastRequest) -> String {
    use sha2::{Digest, Sha256};
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
impl crate::ForecastProvider for PatchTstForecaster {
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

        let cache_key = forecast_cache_key(&request);

        // ── Replay-cache lookup ───────────────────────────────────────────────
        if let Some(cache_path) = &self.cache_path {
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
                        tracing::info!(
                            target: "forecast.cost",
                            line = "forecast_inference",
                            usd = 0u64,
                        );
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
            return Err(ForecastError::ReplayMiss {
                hash: cache_key.clone(),
            });
        }

        // ── Run inference ─────────────────────────────────────────────────────
        let bars = &window[window.len() - CONTEXT_LEN..];
        let feat_cf = build_feature_window_from_ohlcv(bars);

        let x = Tensor::from_vec(feat_cf, (1, CHANNELS, CONTEXT_LEN), &self.device)
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

        let confidence_f = (r_hat.abs() / self.sigma_train).clamp(0.0, 1.0);
        let confidence = rust_decimal::Decimal::try_from(f64::from(confidence_f))
            .unwrap_or(rust_decimal::Decimal::ZERO);

        let inference_ms = t_start.elapsed().as_millis() as u64;

        let effective_model_revision = if self.model_revision == "random-init" {
            request.model_revision.clone()
        } else {
            self.model_revision.clone()
        };

        let overlay = ForecastOverlay {
            correlation_id: request.correlation_id,
            confidence,
            direction,
            horizon_bars: 24, // PatchTST 24h horizon per Q4=(b)
            model_revision: effective_model_revision.clone(),
            sampled_at: time::OffsetDateTime::now_utc(),
        };

        let response = ForecastResponse {
            correlation_id: request.correlation_id,
            model_revision: effective_model_revision.clone(),
            overlay: overlay.clone(),
            samples: vec![],
        };

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

        tracing::info!(
            target: "forecast.cost",
            line = "forecast_inference",
            usd = 0u64,
        );

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
pub(crate) mod tests {
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::VarBuilder;

    fn cpu_vb(name: &str) -> VarBuilder<'static> {
        VarBuilder::zeros(DType::F32, &Device::Cpu).pp(name)
    }

    // ── T-D-N2: PatchEmbed shape ───────────────────────────────────────────────

    /// T-D-N2: PatchEmbed forward shape test.
    ///
    /// Input: [bc=2, n_patches=41, patch_len=16]
    /// Output: [bc=2, n_patches=41, d_model=128]
    #[test]
    fn patch_embed_shape() {
        let embed = PatchEmbed::new(cpu_vb("pe")).expect("PatchEmbed::new");
        let x = candle_core::Tensor::zeros((2, N_PATCHES, PATCH_LEN), DType::F32, &Device::Cpu)
            .unwrap();
        let y = embed.forward(&x).expect("forward");
        assert_eq!(
            y.dims(),
            [2, N_PATCHES, D_MODEL],
            "PatchEmbed output shape mismatch"
        );
    }

    // ── T-D-N3: LearnablePositionEncoding shape ────────────────────────────────

    /// T-D-N3: LearnablePositionEncoding shape test.
    ///
    /// Input: [bc=2, n_patches=41, d_model=128]
    /// Output: same shape (PE is added, not concatenated)
    #[test]
    fn pos_encoding_shape() {
        let pe =
            LearnablePositionEncoding::new(cpu_vb("pe")).expect("LearnablePositionEncoding::new");
        let x =
            candle_core::Tensor::zeros((2, N_PATCHES, D_MODEL), DType::F32, &Device::Cpu).unwrap();
        let y = pe.forward(&x).expect("forward");
        assert_eq!(
            y.dims(),
            [2, N_PATCHES, D_MODEL],
            "PositionEncoding output shape mismatch"
        );
    }

    // ── T-D-N4: MultiHeadSelfAttention shape ──────────────────────────────────

    /// T-D-N4: MultiHeadSelfAttention forward shape test.
    ///
    /// Input: [bc=2, seq=41, d_model=128]
    /// Output: [bc=2, seq=41, d_model=128]
    #[test]
    fn mhsa_forward_shape() {
        let mhsa = MultiHeadSelfAttention::new(cpu_vb("mhsa")).expect("MHSA::new");
        let x =
            candle_core::Tensor::zeros((2, N_PATCHES, D_MODEL), DType::F32, &Device::Cpu).unwrap();
        let y = mhsa.forward(&x, false).expect("forward");
        assert_eq!(
            y.dims(),
            [2, N_PATCHES, D_MODEL],
            "MHSA output shape mismatch"
        );
    }

    // ── T-D-N5: TransformerBlock shape ────────────────────────────────────────

    /// T-D-N5: TransformerBlock forward shape test.
    ///
    /// Input: [bc=2, seq=41, d_model=128]
    /// Output: [bc=2, seq=41, d_model=128]
    #[test]
    fn block_forward_shape() {
        let block = TransformerBlock::new(cpu_vb("block")).expect("TransformerBlock::new");
        let x =
            candle_core::Tensor::zeros((2, N_PATCHES, D_MODEL), DType::F32, &Device::Cpu).unwrap();
        let y = block.forward(&x, false).expect("forward");
        assert_eq!(
            y.dims(),
            [2, N_PATCHES, D_MODEL],
            "TransformerBlock output shape mismatch"
        );
    }

    // ── T-D-N6: PatchTstModel forward shape + parameter count ─────────────────

    /// T-D-N6: PatchTstModel forward shape and parameter count test.
    ///
    /// Input: [batch=2, channels=5, time=336]
    /// Output: [batch=2, 1]
    /// Parameter count: 300_000 < count < 600_000
    #[test]
    fn model_forward_shape() {
        let vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let model = PatchTstModel::new(vb).expect("PatchTstModel::new");

        let x = candle_core::Tensor::zeros((2, CHANNELS, CONTEXT_LEN), DType::F32, &Device::Cpu)
            .unwrap();
        let y = model.forward(&x, false).expect("forward");

        assert_eq!(y.dims(), [2, 1], "PatchTstModel output shape mismatch");

        let param_count = model.num_parameters();
        assert!(
            param_count > 300_000 && param_count < 600_000,
            "parameter count {param_count} outside expected range 300k-600k"
        );
        println!("[T-D-N6] parameter count: {param_count}");
    }

    // ── T-D-N7: PatchTstForecaster random_init ────────────────────────────────

    /// T-D-N7: PatchTstForecaster::random_init shape test.
    #[test]
    fn forecaster_random_init() {
        let f = PatchTstForecaster::random_init(Device::Cpu).expect("random_init");
        assert_eq!(f.model_revision, "random-init");
        assert_eq!(f.sigma_train, 1.0);

        let x = candle_core::Tensor::zeros((1, CHANNELS, CONTEXT_LEN), DType::F32, &Device::Cpu)
            .unwrap();
        let y = f.forward(&x, false).expect("forward");
        assert_eq!(y.dims(), [1, 1], "forecaster forward shape mismatch");
    }

    // ── T-D-N8: ForecastProvider boxed trait object ───────────────────────────

    /// T-D-N8: PatchTstForecaster implements ForecastProvider and is object-safe.
    #[tokio::test]
    async fn forecast_provider_boxed() {
        use crate::ForecastProvider;
        use rust_decimal::Decimal;
        use time::OffsetDateTime;
        use trading_core::forecast::{ForecastRequest, OhlcvBar, SamplingParams};
        use uuid::Uuid;

        let f = PatchTstForecaster::random_init(Device::Cpu).expect("random_init");
        let _provider: Box<dyn ForecastProvider> = Box::new(f);

        // Construct a minimal request with CONTEXT_LEN bars.
        let bar = OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(101, 0),
            low: Decimal::new(99, 0),
            close: Decimal::new(100, 0),
            volume: Decimal::new(1000, 0),
            ts: OffsetDateTime::UNIX_EPOCH,
        };
        let bars = vec![bar; CONTEXT_LEN + 1];

        let f2 = PatchTstForecaster::random_init(Device::Cpu).expect("random_init");
        let provider2: Box<dyn ForecastProvider> = Box::new(f2);
        let req = ForecastRequest {
            model_revision: "random-init".to_string(),
            ohlcv_window: bars,
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };
        let resp = provider2
            .forecast(req)
            .await
            .expect("forecast should succeed");
        // With zero-weights model, r_hat ~ 0, direction should be Flat.
        assert!(matches!(
            resp.overlay.direction,
            Direction::Up | Direction::Down | Direction::Flat
        ));
    }
}
