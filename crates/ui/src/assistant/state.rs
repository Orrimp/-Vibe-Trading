//! Phase F — Assistant-slot per-session state (Q4=(a) stub-only).
//!
//! The right-rail slot wakes structurally at Phase F; the v2 LLM
//! text-stream wire defers to v0.2.0 per Q4=(a) + K7 mitigation.

/// Phase F — Assistant mode (Q4=(a) — stub only at v0.1.0).
///
/// `Offline` is the only active mode at v0.1.0. `Live` is the v0.2.0
/// full-wire mode reserved for when `crates/llm::AnthropicProvider`
/// wiring lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssistantMode {
    /// v0.1.0 — slot wakes but body renders "Assistant offline" placeholder.
    #[default]
    Offline,
    /// v0.2.0 — full text-stream wire to `crates/llm::AnthropicProvider`.
    Live,
}

/// Phase F — Assistant-slot per-session state (R3.1 / R4.3).
///
/// Added as `pub assistant_state: AssistantState` on `Cockpit` at
/// `state.rs:~884` (three-touchpoint pattern: struct field + Debug +
/// Default). Sibling of `memory_screen_state` + `models_screen_state`
/// (Phase F).
#[derive(Debug, Clone, Default)]
pub struct AssistantState {
    /// Whether the right-rail slot is currently open.
    /// `false` on cold-boot per R4.4; default shell right-track stays
    /// at `LENGTH::Fixed(RIGHT_RAIL_WIDTH_PX = 0.0)` (K6 Option A).
    pub is_open: bool,
    /// Active mode. `Offline` at v0.1.0 (Q4=(a)).
    pub mode: AssistantMode,
}
