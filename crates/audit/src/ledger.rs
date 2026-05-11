//! `Ledger` — async SQLite-backed double-entry ledger handle.
use tracing::instrument;
use trading_core::LedgerError;

/// The double-entry ledger handle.
/// Backed by `SQLite` via `sqlx`.
#[derive(Clone)]
pub struct Ledger {
    pub(crate) pool: sqlx::SqlitePool,
}

impl Ledger {
    /// Open (or create) the ledger database at `db_path` and run migrations.
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

        Ok(Self { pool })
    }

    /// In-memory ledger for tests.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError::Database`] on connection failure.
    pub async fn in_memory() -> Result<Self, LedgerError> {
        Self::open(":memory:").await
    }

    /// Expose the underlying connection pool (needed for raw SQL in tests /
    /// admin tooling).
    #[must_use]
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}
