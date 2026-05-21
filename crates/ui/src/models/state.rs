//! Phase F — Models-screen per-session state.
//!
//! Sibling of `crates/ui/src/memory/state.rs` (Phase F) and
//! `crates/ui/src/compare/state.rs` (Phase E). All fields are
//! session-scoped; no on-disk persistence at v0.1.0 (R5.3).

use smol_str::SmolStr;

/// Phase F — model family discriminant (R8.2, § 1.2).
///
/// Discriminated from the checkpoint filename prefix:
/// - `tcn-*` → `Tcn`
/// - `patchtst-*` → `PatchTst` (v0.2.0 — no files on disk at v0.1.0)
/// - `transformer-*` → `Transformer` (v0.2.0)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModelFamily {
    #[default]
    Tcn,
    /// Disabled at v0.1.0 (no files on disk). Toolbar chip renders
    /// with tooltip "Family ships in v2.5a".
    PatchTst,
    /// Disabled at v0.1.0 (no files on disk). Toolbar chip renders
    /// with tooltip "Family ships in v2.5b".
    Transformer,
}

impl ModelFamily {
    /// Human-readable label for the toolbar filter chip.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ModelFamily::Tcn => "TCN",
            ModelFamily::PatchTst => "PatchTST",
            ModelFamily::Transformer => "Transformer",
        }
    }
}

/// Phase F — checkpoint lifecycle status (R2.2, Q7=(c)).
///
/// At v0.1.0 every on-disk checkpoint renders as `Staged` per Q7=(c)
/// with tooltip "Lifecycle classification ships in v0.2.0". `Serving`
/// and `Archived` are included for completeness; the toolbar status
/// filter chip renders all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelStatus {
    /// The checkpoint is currently loaded into a live `ForecastProvider`.
    /// v0.2.0: detected by config parse. v0.1.0: never set (Q7=(c)).
    Serving,
    /// The checkpoint is on disk but not confirmed serving.
    /// v0.1.0 default for every on-disk checkpoint per Q7=(c).
    #[default]
    Staged,
    /// The checkpoint has been explicitly archived (moved to
    /// `crates/forecast/checkpoints/archived/`). v0.2.0 lifecycle.
    Archived,
}

impl ModelStatus {
    /// Human-readable label for the status pill.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            ModelStatus::Serving => "serving",
            ModelStatus::Staged => "staged",
            ModelStatus::Archived => "archived",
        }
    }
}

/// Phase F — UI view-model for one checkpoint row (R8.3, § 1.2).
///
/// Populated by `models::registry_read::discover_checkpoints` from
/// `crates/forecast/checkpoints/anchors/*.metadata.json`. Distinct from
/// `CheckpointMetadata` (the raw serde shape) to avoid leaking it into
/// the UI layer.
#[derive(Debug, Clone)]
pub struct CheckpointMeta {
    /// Full `model_revision` SHA256 hash from metadata.json.
    pub model_revision: SmolStr,
    /// `ModelFamily` derived from the filename prefix.
    pub family: ModelFamily,
    /// ISO-8601 training data span start.
    pub data_span_start: SmolStr,
    /// ISO-8601 training data span end.
    pub data_span_end: SmolStr,
    /// Bar interval (e.g. `"1h"`).
    pub interval: SmolStr,
    /// Number of symbols in the training universe.
    pub symbols_count: usize,
    /// Final validation loss from training.
    pub final_val_loss: f64,
    /// Final training loss from training.
    pub final_train_loss: f64,
    /// Sigma-train calibration constant (0.0 if not present — K2 robustness).
    pub sigma_train: f64,
    /// SHA256 of the safetensors weights file.
    pub weights_sha256: SmolStr,
    /// Size of the safetensors file in bytes.
    pub file_size_bytes: u64,
    /// Q7=(c) — always `Staged` at v0.1.0.
    pub status: ModelStatus,
    /// Filesystem path of the `.metadata.json` file (for diagnostics).
    pub source_path: std::path::PathBuf,
}

/// Phase F — Models-screen per-session state (R4.2).
///
/// Added as `pub models_screen_state: ModelsScreenState` on `Cockpit`
/// at `state.rs:~884` (three-touchpoint pattern: struct field + Debug +
/// Default). Sibling of `memory_screen_state` (Phase F) and
/// `compare_screen_state` (Phase E).
#[derive(Debug, Clone)]
pub struct ModelsScreenState {
    /// Active family filter. Default `[Tcn]` (only family on disk at
    /// v0.1.0). Other chips render disabled per R2.2.
    pub family_filter: Vec<ModelFamily>,
    /// Active status filter. Default `[Staged]` (all on-disk checkpoints
    /// render as `Staged` per Q7=(c)).
    pub status_filter: Vec<ModelStatus>,
    /// Checkpoint list populated by `Message::ModelsHydrate`.
    /// Empty until the first hydrate fires (cold-boot-only per R5.3).
    pub checkpoints: Vec<CheckpointMeta>,
    /// ISO-8601 timestamp of the last successful hydrate. `None` until
    /// first hydrate fires.
    pub last_indexed: Option<SmolStr>,
}

impl Default for ModelsScreenState {
    fn default() -> Self {
        Self {
            family_filter: vec![ModelFamily::Tcn],
            status_filter: vec![ModelStatus::Staged],
            checkpoints: Vec::new(),
            last_indexed: None,
        }
    }
}
