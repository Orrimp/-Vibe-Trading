//! Kill-switch halt reasons (bug-log #92, relocated from `agent` 2026-08-22).
//!
//! Plain data, moved here for the same reason as [`crate::activity`]: `ui`
//! referenced `agent::HaltReason` unconditionally while `agent` is a
//! `live`-only dependency, which broke the documented `--no-default-features`
//! build. The `KillSwitch` itself — the halt-file watcher, the heartbeat
//! monitor, the broadcast of `AgentMode::Halted` — stays in `agent`; only the
//! reason enum moves.
//!
//! ⚠️ Note for anyone extending this: two declared loss stops
//! (`daily_loss_stop_pct`, `max_drawdown_stop_pct`) are configured and
//! documented but **have no variant here and no read site anywhere** — bug-log
//! **#85**. A configured stop with no `HaltReason` cannot trip anything. If you
//! wire them, they need variants here first.

/// Reasons the kill switch can be tripped.
#[derive(Debug, Clone)]
pub enum HaltReason {
    HaltFile,
    HeartbeatTimeout,
    LedgerImbalance,
    ClockSkew,
    ManualOperator,
    Test,
}

impl std::fmt::Display for HaltReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HaltFile => write!(f, "halt_file"),
            Self::HeartbeatTimeout => write!(f, "heartbeat_timeout"),
            Self::LedgerImbalance => write!(f, "ledger_imbalance"),
            Self::ClockSkew => write!(f, "clock_skew"),
            Self::ManualOperator => write!(f, "manual_operator"),
            Self::Test => write!(f, "test"),
        }
    }
}
