//! Agent library crate — exposes all subsystems for use by the agent binary
//! and by the cockpit (via broadcast bus subscriptions).

pub mod activity;
pub mod activity_audit_aggregator;
pub mod bus;
pub mod config;
#[cfg(feature = "in_process_cron")]
pub mod cron;
pub mod kill_switch;
pub mod observability;
pub mod reconciler;
pub mod runtime;
pub mod watcher;

pub use activity::{
    ActivityEvent, ActivityHandle, ActivityId, ActivityKind, ActivityOutcome, ActivityPhase,
    ActivitySender,
};
pub use activity_audit_aggregator::spawn_aggregator;
pub use bus::EventBus;
pub use kill_switch::{
    AgentMode, CommandIncidentSpawner, HaltReason, IncidentSpawnArgs, IncidentSpawner, KillSwitch,
    MockIncidentSpawner,
};
pub use reconciler::ReconcilerTask;
pub use runtime::{RunHandles, build_registry, paper_engine_publisher, run, shutdown_writer};
pub use watcher::run_strategy_watcher;
