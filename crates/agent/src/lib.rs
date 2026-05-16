//! Agent library crate — exposes all subsystems for use by the agent binary
//! and by the cockpit (via broadcast bus subscriptions).

pub mod bus;
pub mod config;
#[cfg(feature = "in_process_cron")]
pub mod cron;
pub mod kill_switch;
pub mod observability;
pub mod reconciler;
pub mod runtime;
pub mod watcher;

pub use bus::EventBus;
pub use kill_switch::{
    AgentMode, CommandIncidentSpawner, HaltReason, IncidentSpawnArgs, IncidentSpawner, KillSwitch,
    MockIncidentSpawner,
};
pub use reconciler::ReconcilerTask;
pub use runtime::{build_registry, paper_engine_publisher, run, shutdown_writer, RunHandles};
pub use watcher::run_strategy_watcher;
