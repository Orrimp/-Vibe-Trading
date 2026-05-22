//! Phase F — Assistant-slot per-session state.
//!
//! v0.1.0 (Phase 6 wake, Q4=(a) stub-only) shipped the slot wake structurally
//! with `AssistantMode::Offline` as the only active mode. The v3-llm-
//! forecaster Wave F promotion (T-D-N(F1) + T-D-N(F4)) adds the
//! `ReasoningTrace` mode + the supporting `LlmForecastView` UI mirror.
//!
//! ## Runtime gate (R9.3)
//!
//! [`AssistantMode::ReasoningTrace`] is opt-in: only set when the
//! cockpit boots with `llm_forecaster_v3` registered as an enabled
//! strategy in `agent::config::StrategiesConfig`. The `cockpit_live` bin
//! reads agent config + sets `assistant_state.mode = ReasoningTrace`.
//! Without that wiring, the default ([`AssistantMode::Offline`]) keeps
//! the Phase F placeholder body byte-identical (R9.3 + Q-ASSISTANT-WAKE
//! operator-lock 2026-05-22 → runtime-gated).
//!
//! ## No `strategy` crate dep
//!
//! [`LlmForecastView`] mirrors the relevant render-time fields of
//! `crates/strategy::llm_forecaster::types::LlmForecast`. Mirroring (vs
//! re-exporting) preserves the architecture rule "ui depends only on
//! core + audit + reflection" — the same rule that drives
//! `state::StrategiesConfig` (the UI-local mirror of
//! `agent::config::StrategiesConfig`).
//!
//! The `cockpit_live` boot path translates the strategy crate's
//! `LlmForecast` into [`LlmForecastView`] at the message-bus boundary.

use smol_str::SmolStr;

/// UI-local mirror of the strategy crate's `LlmForecast` for the
/// Phase F Assistant slot reasoning-trace render (R9.2).
///
/// All fields are display-ready `SmolStr` values; the `cockpit_live` bin
/// formats the strategy crate's `Decimal` + `Rating` + `Symbol` types
/// at the message-bus boundary (mirror of `LessonCardCard` pattern at
/// `crates/ui/src/memory/state.rs:43-64`).
///
/// ## Cited-lesson references
///
/// `cited_lessons` is a flat list of card-id strings. The full
/// `LessonCardCard` body for each cited card is looked up in
/// `Cockpit::memory_screen_state.cache` at render time so we don't
/// duplicate the card payload in the assistant slot's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmForecastView {
    /// Symbol display string (e.g. `"BTCUSDT"`).
    pub symbol: SmolStr,
    /// Rating tier display string: one of
    /// `"STRONG_BUY"` / `"BUY"` / `"HOLD"` / `"SELL"` / `"STRONG_SELL"`.
    pub rating: SmolStr,
    /// Confidence formatted for display (e.g. `"0.74"`).
    pub confidence_display: SmolStr,
    /// Reasoning-trace text (50-2000 chars per R1.2 / prompt schema).
    /// Plain text at v0.1.0 (markdown rendering deferred per R1.2 of
    /// the Memory screen — same precedent applies to the Assistant slot).
    pub reasoning_trace: SmolStr,
    /// Card ids of lesson cards cited by the LLM. The full
    /// `LessonCardCard` body is read from
    /// `memory_screen_state.cache` at render time.
    pub cited_lesson_ids: Vec<SmolStr>,
    /// Cumulative LLM spend label for the assistant header cost line
    /// (R9.2). Pre-formatted display string from the cost-event
    /// channel (e.g. `"$0.42 / $100.00 today"`). `None` when the
    /// cost-event channel hasn't fired yet (cold-boot).
    pub cost_line: Option<SmolStr>,
    /// `audit_id` of the `JournalEntry { kind: "llm_forecast" }` that
    /// recorded this forecast. Used by the chevron affordance to
    /// dispatch `Message::OpenTrailFor(audit_id)` (R9.2 trail link).
    /// `None` when the audit-emission path hasn't fired yet
    /// (Wave A/B stub fixtures).
    pub audit_id: Option<SmolStr>,
}

/// Phase F — Assistant mode.
///
/// `Offline` is the v0.1.0 default — slot wakes with the placeholder
/// body when the LLM forecaster is disabled (R9.3 byte-identity guard).
///
/// `ReasoningTrace` is set when the cockpit boots with
/// `llm_forecaster_v3` enabled (v3-llm-forecaster Wave F R9.1). The
/// slot body renders the most-recent forecast trace + cited lessons +
/// cost line.
///
/// `Live` is reserved for v0.2.0 — the full v2 LLM text-stream wire to
/// `crates/llm::AnthropicProvider` (`crates/ui::live` bus channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssistantMode {
    /// v0.1.0 — slot wakes but body renders "Assistant offline"
    /// placeholder (R9.3 default-disabled).
    #[default]
    Offline,
    /// v0.1.0 + v3-llm-forecaster Wave F — body renders the most-
    /// recent forecast trace (R9.2). Set only when the LLM strategy
    /// is enabled in agent config (runtime gate per R9.3 +
    /// Q-ASSISTANT-WAKE T-OD10 operator-lock 2026-05-22).
    ReasoningTrace,
    /// v0.2.0 — full text-stream wire to `crates/llm::AnthropicProvider`.
    Live,
}

/// Phase F — Assistant-slot per-session state.
///
/// Added as `pub assistant_state: AssistantState` on `Cockpit` at
/// `state.rs:~899` (three-touchpoint pattern: struct field + Debug +
/// Default). Sibling of `memory_screen_state` + `models_screen_state`
/// (Phase F).
///
/// ## Wave F additive fields (v3-llm-forecaster T-D-N(F1))
///
/// `last_forecast` + `history` are populated by the
/// `Message::AssistantReasoningTraceUpdate(view)` arm in the cockpit's
/// `update` fn. At v0.1.0 with the strategy *disabled* both fields
/// stay empty and the slot body falls back to the Offline placeholder
/// (R9.3 byte-identity).
#[derive(Debug, Clone, Default)]
pub struct AssistantState {
    /// Whether the right-rail slot is currently open.
    /// `false` on cold-boot per R4.4; default shell right-track stays
    /// at `Length::Fixed(RIGHT_RAIL_WIDTH_PX = 0.0)` (K6 Option A).
    pub is_open: bool,
    /// Active mode. `Offline` at v0.1.0 default; `ReasoningTrace` set
    /// by the `cockpit_live` bin when `llm_forecaster_v3` is enabled
    /// (R9.3 runtime gate).
    pub mode: AssistantMode,
    /// Most-recent LLM forecast view-model. Rendered as the header +
    /// reasoning card + cost line + cited-lessons section when
    /// `mode == ReasoningTrace`. `None` until the first
    /// `AssistantReasoningTraceUpdate` arrives (warming-up state).
    pub last_forecast: Option<LlmForecastView>,
    /// Scrollable history of past forecasts (R9.2 bullet 5). Most-
    /// recent first; capped at [`HISTORY_CAP`] to bound the slot
    /// memory footprint. Wired by the `AssistantReasoningTraceUpdate`
    /// arm: new forecast prepended; ring buffer when at cap.
    pub history: Vec<LlmForecastView>,
}

/// History cap for [`AssistantState::history`]. The R9.2 spec calls for
/// "last N (~20) traces"; we pick 20 to honour the analyst guidance.
pub const HISTORY_CAP: usize = 20;

#[cfg(test)]
mod tests {
    use super::*;

    /// T-D-N(F1) — `AssistantMode::default()` is `Offline` so cold-boot
    /// renders the Phase F placeholder body. R9.3 byte-identity guard.
    #[test]
    fn assistant_mode_default_is_offline() {
        assert_eq!(AssistantMode::default(), AssistantMode::Offline);
    }

    /// T-D-N(F1) — `AssistantState::default()` has `mode == Offline`
    /// and `last_forecast == None`. R9.3 byte-identity guard +
    /// warming-up state.
    #[test]
    fn assistant_state_default_is_offline_and_empty() {
        let s = AssistantState::default();
        assert!(!s.is_open, "default is_open must be false");
        assert_eq!(s.mode, AssistantMode::Offline);
        assert!(
            s.last_forecast.is_none(),
            "default last_forecast must be None"
        );
        assert!(s.history.is_empty(), "default history must be empty");
    }

    /// T-D-N(F1) — `AssistantMode` covers exactly three variants:
    /// `Offline`, `ReasoningTrace`, `Live`. Belt-and-braces against an
    /// accidental variant rename that would silently break the runtime
    /// gate.
    #[test]
    fn assistant_mode_has_three_variants() {
        // Compile-time check via exhaustive match.
        let probe = AssistantMode::Offline;
        let _label: &str = match probe {
            AssistantMode::Offline => "offline",
            AssistantMode::ReasoningTrace => "reasoning_trace",
            AssistantMode::Live => "live",
        };
    }
}
