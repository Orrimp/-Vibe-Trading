//! Agent library crate — exposes all subsystems for use by the agent binary
//! and by the cockpit (via broadcast bus subscriptions).

pub mod bus;
pub mod config;
pub mod kill_switch;
pub mod observability;
pub mod reconciler;

pub use bus::EventBus;
pub use kill_switch::{AgentMode, HaltReason, KillSwitch};
pub use reconciler::ReconcilerTask;
