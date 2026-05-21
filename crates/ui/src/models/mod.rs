//! Phase F — Models feature module (ui-rethink-phase-f-memory-models-assistant).
//!
//! Houses all Models-screen-specific logic that is not a widget or a
//! screen view: state shape and the cold-boot checkpoint registry reader.
//!
//! Module layout:
//!
//! ```text
//! models/
//! ├── mod.rs          — this file, re-exports
//! ├── state.rs        — ModelsScreenState + CheckpointMeta + ModelFamily + ModelStatus (T-D-N1)
//! └── registry_read.rs — discover_checkpoints + CheckpointMetadata serde structs (T-D-N9)
//! ```

pub mod registry_read;
pub mod state;

pub use state::{CheckpointMeta, ModelFamily, ModelStatus, ModelsScreenState};
