//! Strategy lifecycle broadcast events and read-side view types.
//!
//! These types are used by:
//! - `agent::EventBus` — broadcast channels for cockpit subscription.
//! - `audit::journal::strategy_event` — persists to `strategy_events` table.
//! - `audit::query::strategy_events_since` / `strategy_history` — read surface.
//! - `ui::widgets::strategies` — cockpit panel.
//!
//! Placing these in `trading_core` avoids dependency cycles:
//! `audit` needs them (as a pure sink); `ui` imports them from `trading_core`
//! (which it already depends on); `agent` publishes them.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::symbol::StrategyId;
use crate::time::Timestamp;

// ── Broadcast event types (Q5 resolution) ─────────────────────────────────────

/// Emitted to `agent::EventBus::strategy_loaded` when a strategy is
/// successfully loaded for the first time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoaded {
    pub id: StrategyId,
    /// SHA-256 of the canonicalized AST — 32 bytes.
    pub hash: [u8; 32],
    /// Repo-relative path to the source TOML.
    pub source_path: SmolStr,
    /// Timestamp from the event source (replay clock in backtest/research;
    /// wall-clock in paper mode).
    pub ts: Timestamp,
}

/// Emitted to `agent::EventBus::strategy_swapped` when an existing strategy
/// is replaced by a new configuration (hot-swap).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySwapped {
    pub id: StrategyId,
    /// SHA-256 of the previous configuration.
    pub old_hash: [u8; 32],
    /// SHA-256 of the replacement configuration.
    pub new_hash: [u8; 32],
    /// Repo-relative path to the source TOML.
    pub source_path: SmolStr,
    /// Timestamp (same clock contract as `StrategyLoaded`).
    pub ts: Timestamp,
}

/// Emitted to `agent::EventBus::strategy_error` when a load or reload attempt
/// is rejected due to parse / typecheck failure.
///
/// The old strategy (if any) continues running unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLoadError {
    /// Repo-relative path to the rejected TOML.
    pub source_path: SmolStr,
    /// `None` when the filename stem itself is not a valid `StrategyId`
    /// (e.g. non-UTF8 filename).
    pub strategy_id: Option<StrategyId>,
    /// Machine-readable error code — values from the error-code table in
    /// `spec/features/v05-composed-strategies.md#design`.
    ///
    /// Examples: `"toml_parse"`, `"unknown_indicator"`, `"arity_mismatch"`.
    pub error_code: SmolStr,
    /// One-line human-readable description for the cockpit error badge.
    pub error_summary: SmolStr,
    /// Timestamp (same clock contract as `StrategyLoaded`).
    pub ts: Timestamp,
}

// ── Read-side view type (used by audit::query) ─────────────────────────────────

/// Discriminator for a `strategy_events` row returned by
/// `audit::query::strategy_history` / `strategy_events_since`.
///
/// v1 adds `RebalanceRejected` (Q6) — written to `strategy_events` table
/// with `kind = "rebalance_rejected"` when the portfolio-exposure validator
/// refuses a rebalance vector. No schema migration needed (TEXT column).
///
/// v1.5a adds two new variants (Q8) — **no SQL migration**; the `kind`
/// column is TEXT so new values are stored directly:
/// - `MeanReversionStop` — emitted when a long position is closed by the
///   `z >= z_stop` hard-stop (R4.1). Distinguishes from the normal
///   `z_exit` reversion close.
/// - `PairShortObservation` — emitted alongside the executed long-leg buy
///   on entry; records "would have shorted `b`" (formulation C residual,
///   R5.3 / Q3). No money moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum StrategyEventKind {
    Load,
    Swap,
    Unload,
    Reject,
    /// v1 Q6 — risk gate rejected the rebalance vector (portfolio exposure breach).
    RebalanceRejected,
    /// v1.5a Q8 — hard-stop triggered: `z >= z_stop` while long.
    MeanReversionStop,
    /// v1.5a Q8 — observation-only: would have shorted `b` leg in formulation C.
    PairShortObservation,
}

impl std::fmt::Display for StrategyEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load => write!(f, "Load"),
            Self::Swap => write!(f, "Swap"),
            Self::Unload => write!(f, "Unload"),
            Self::Reject => write!(f, "Reject"),
            Self::RebalanceRejected => write!(f, "RebalanceRejected"),
            Self::MeanReversionStop => write!(f, "MeanReversionStop"),
            Self::PairShortObservation => write!(f, "PairShortObservation"),
        }
    }
}

/// Read-side view of a row in the `strategy_events` table.
///
/// Returned by `audit::query::strategy_events_since` and
/// `audit::query::strategy_history`. No `sqlx` types leak into this struct
/// (all amounts / ids are plain Rust types from `trading_core`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyEventView {
    /// UUID v4 — row identifier.
    pub id: SmolStr,
    /// RFC-3339 timestamp of the event.
    pub ts: Timestamp,
    /// `Load` | `Swap` | `Unload` | `Reject`.
    pub kind: StrategyEventKind,
    /// Strategy ID — `None` for `Reject` when the filename stem is unparsable.
    pub strategy_id: Option<StrategyId>,
    /// SHA-256 hex (64 chars) of the previous config; present for `Swap` and
    /// `Unload`.
    pub old_hash: Option<SmolStr>,
    /// SHA-256 hex (64 chars) of the new / current config; present for
    /// `Load` and `Swap`.
    pub new_hash: Option<SmolStr>,
    /// Repo-relative source path.
    pub source_path: Option<SmolStr>,
    /// `"system"` in v0.5; future cockpit edit-flow may emit `"user"`.
    pub operator: SmolStr,
    /// Machine-readable error code; present only for `Reject`.
    pub error_code: Option<SmolStr>,
    /// Human-readable error summary; present only for `Reject`.
    pub error_summary: Option<SmolStr>,
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn dummy_ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH)
    }

    #[test]
    fn t501_strategy_loaded_round_trip() {
        let ev = StrategyLoaded {
            id: StrategyId::new("btc_macd_trend"),
            hash: [0u8; 32],
            source_path: SmolStr::new("config/strategies/btc_macd_trend.toml"),
            ts: dummy_ts(),
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: StrategyLoaded = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev.id, back.id);
        assert_eq!(ev.hash, back.hash);
        assert_eq!(ev.source_path, back.source_path);
    }

    #[test]
    fn t501_strategy_swapped_round_trip() {
        let ev = StrategySwapped {
            id: StrategyId::new("btc_macd_trend"),
            old_hash: [0u8; 32],
            new_hash: [1u8; 32],
            source_path: SmolStr::new("config/strategies/btc_macd_trend.toml"),
            ts: dummy_ts(),
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: StrategySwapped = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev.id, back.id);
        assert_ne!(ev.old_hash, ev.new_hash);
        assert_eq!(back.old_hash, [0u8; 32]);
        assert_eq!(back.new_hash, [1u8; 32]);
    }

    #[test]
    fn t501_strategy_load_error_round_trip() {
        let ev = StrategyLoadError {
            source_path: SmolStr::new("config/strategies/bad.toml"),
            strategy_id: None,
            error_code: SmolStr::new("toml_parse"),
            error_summary: SmolStr::new("unexpected token at line 3"),
            ts: dummy_ts(),
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: StrategyLoadError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev.error_code, back.error_code);
        assert!(back.strategy_id.is_none());
    }

    #[test]
    fn t501_strategy_event_view_round_trip() {
        let view = StrategyEventView {
            id: SmolStr::new("550e8400-e29b-41d4-a716-446655440000"),
            ts: dummy_ts(),
            kind: StrategyEventKind::Load,
            strategy_id: Some(StrategyId::new("btc_macd_trend")),
            old_hash: None,
            new_hash: Some(SmolStr::new("a1b2c3")),
            source_path: Some(SmolStr::new("config/strategies/btc_macd_trend.toml")),
            operator: SmolStr::new("system"),
            error_code: None,
            error_summary: None,
        };
        let json = serde_json::to_string(&view).expect("serialize");
        let back: StrategyEventView = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(view.kind, back.kind);
        assert_eq!(view.operator, back.operator);
    }

    #[test]
    fn t501_strategy_event_kind_display() {
        assert_eq!(StrategyEventKind::Load.to_string(), "Load");
        assert_eq!(StrategyEventKind::Swap.to_string(), "Swap");
        assert_eq!(StrategyEventKind::Unload.to_string(), "Unload");
        assert_eq!(StrategyEventKind::Reject.to_string(), "Reject");
    }

    #[test]
    fn t701_strategy_event_kind_v15a_variants() {
        // v1.5a Q8 — two new kind values
        assert_eq!(
            StrategyEventKind::MeanReversionStop.to_string(),
            "MeanReversionStop"
        );
        assert_eq!(
            StrategyEventKind::PairShortObservation.to_string(),
            "PairShortObservation"
        );
    }

    #[test]
    fn t701_strategy_event_kind_v15a_serde_roundtrip() {
        let kind = StrategyEventKind::MeanReversionStop;
        let json = serde_json::to_string(&kind).unwrap();
        let back: StrategyEventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);

        let kind2 = StrategyEventKind::PairShortObservation;
        let json2 = serde_json::to_string(&kind2).unwrap();
        let back2: StrategyEventKind = serde_json::from_str(&json2).unwrap();
        assert_eq!(kind2, back2);
    }
}
