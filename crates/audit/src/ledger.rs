//! `Ledger` — async SQLite-backed double-entry ledger handle.
use tracing::instrument;
use trading_core::LedgerError;

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::tick::{AuditEvent, AuditTick};

/// Internal tick-bus state pre-seeded at session start (Q5).
/// Cheap to clone — sender is `Arc`-backed; context is two `u32`s + a `Uuid`.
#[derive(Clone)]
pub(crate) struct TickBus {
    pub(crate) sender: broadcast::Sender<AuditTick<AuditEvent>>,
    pub(crate) run_id: Uuid,
    pub(crate) agent_pid: u32,
}

/// The double-entry ledger handle.
/// Backed by `SQLite` via `sqlx`.
#[derive(Clone)]
pub struct Ledger {
    pub(crate) pool: sqlx::SqlitePool,
    /// Pre-seeded tick context. `None` means tee dormant (R2.1 / Q3).
    /// Default `Ledger::open(...)` leaves this `None` so the default boot
    /// path is bit-identical to pre-feature behaviour (H2 anchor preservation).
    pub(crate) tick_bus: Option<TickBus>,
}

impl Ledger {
    /// Open (or create) the ledger database at `db_path` and run migrations.
    ///
    /// Default branch — `tick_bus = None`, tee dormant (R2.1).
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Database`] if the file cannot be opened or
    /// migrations fail.
    #[instrument(name = "ledger.open", skip_all, fields(db_path))]
    pub async fn open(db_path: &str) -> Result<Self, LedgerError> {
        let url = if db_path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{db_path}?mode=rwc")
        };

        let pool = sqlx::SqlitePool::connect(&url)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;

        // Run embedded migrations (001..009)
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;

        Ok(Self {
            pool,
            tick_bus: None,
        })
    }

    /// In-memory ledger for tests.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Database`] on connection failure.
    pub async fn in_memory() -> Result<Self, LedgerError> {
        Self::open(":memory:").await
    }

    /// Open (or create) the ledger at `db_path` **and** wire a
    /// `tokio::sync::broadcast` tick bus with the given capacity (R2.2 / Q3 /
    /// Q5). Returns `(Ledger, Sender)` so callers can `.subscribe()` once per
    /// consumer.
    ///
    /// `run_id` defaults to `Uuid::nil()` — call `.with_run_id(uuid)` to stamp
    /// a per-session uuid before handing the ledger to any writer (K4).
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Database`] if the underlying `open` fails.
    pub async fn open_with_tick_bus(
        db_path: &str,
        capacity: usize,
    ) -> Result<(Self, broadcast::Sender<AuditTick<AuditEvent>>), LedgerError> {
        let mut ledger = Self::open(db_path).await?;
        let (sender, _) = broadcast::channel(capacity);
        ledger.tick_bus = Some(TickBus {
            sender: sender.clone(),
            run_id: Uuid::nil(), // operator overrides via .with_run_id()
            agent_pid: std::process::id(), // one syscall, session-lifetime (Q5)
        });
        Ok((ledger, sender))
    }

    /// Return a fresh `Ledger` clone with a new `run_id` stamped on its tick
    /// context. The `SQLite` pool and broadcast sender are shared (cheap `Arc`
    /// clone); only `run_id` is updated. Concurrent backtests get distinct
    /// `run_id`s without contending on the same handle (K4 mitigation).
    #[must_use]
    pub fn with_run_id(&self, run_id: Uuid) -> Self {
        let mut next = self.clone();
        if let Some(bus) = next.tick_bus.as_mut() {
            bus.run_id = run_id;
        }
        next
    }

    /// Test helper — override the pre-seeded `agent_pid` for deterministic
    /// assertions in tick tests.
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_pid(&self, pid: u32) -> Self {
        let mut next = self.clone();
        if let Some(bus) = next.tick_bus.as_mut() {
            bus.agent_pid = pid;
        }
        next
    }

    /// Expose the underlying connection pool (needed for raw SQL in tests /
    /// admin tooling).
    #[must_use]
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}
