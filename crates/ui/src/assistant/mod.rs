//! Phase F — Assistant slot module (Lumen Phase 6 wake).
//!
//! Houses the [`AssistantState`] type, the [`LlmForecastView`] UI mirror
//! payload, and the view function for the right-rail slot.
//!
//! Wave F (v3-llm-forecaster T-D-N(F1) + T-D-N(F2)) extends the slot
//! from the v0.1.0 Phase F stub-only body to the `ReasoningTrace`
//! composition (R9.2) — runtime-gated per R9.3 so the default-disabled
//! config keeps Phase F byte-identical.
//!
//! Module layout:
//!
//! ```text
//! assistant/
//! ├── mod.rs   — this file, re-exports
//! ├── state.rs — AssistantState + AssistantMode + LlmForecastView
//! └── view.rs  — assistant slot view fn (Offline + ReasoningTrace + Live)
//! ```

pub mod state;
pub mod view;

pub use state::{AssistantMode, AssistantState, HISTORY_CAP, LlmForecastView};
