//! Phase F — Memory feature module (ui-rethink-phase-f-memory-models-assistant).
//!
//! Houses all Memory-screen-specific logic that is not a widget or a
//! screen view: state shape and the `MemoryHydrate` message handling.
//!
//! Module layout:
//!
//! ```text
//! memory/
//! ├── mod.rs    — this file, re-exports
//! ├── state.rs  — MemoryScreenState + LessonCardCard + MemoryViewMode + MemoryFilter (T-D-N1)
//! └── drawer.rs — Memory entry side-drawer widget (T-D-N12)
//! ```

pub mod drawer;
pub mod state;

pub use state::{LessonCardCard, MemoryFilter, MemoryScreenState, MemoryViewMode};
