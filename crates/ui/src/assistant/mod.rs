//! Phase F — Assistant slot module (Lumen Phase 6 wake, Q4=(a) stub-only).
//!
//! Houses the AssistantState type and the view function for the
//! right-rail slot. The v2 LLM text-stream wire defers to v0.2.0.
//!
//! Module layout:
//!
//! ```text
//! assistant/
//! ├── mod.rs   — this file, re-exports
//! ├── state.rs — AssistantState + AssistantMode (T-D-N1)
//! └── view.rs  — assistant slot view fn (T-D-N16)
//! ```

pub mod state;
pub mod view;

pub use state::{AssistantMode, AssistantState};
