//! T810 — Optional in-process cron scheduler for the operator success
//! report (R12.1b / Q7).
//!
//! Compiled only when the `in_process_cron` feature flag is enabled.
//! Default builds skip this entire module.  The operator chooses one of
//! three cron patterns documented under `ops/`:
//!
//!   1. `ops/reports.timer.example` + `ops/reports.service.example`
//!      (systemd, Linux production);
//!   2. `ops/com.trading.reports.plist.example` (launchd, macOS);
//!   3. **this module** — single-binary in-process scheduler, opt-in
//!      via `--features in_process_cron`.
//!
//! Failure to schedule or run the job is warn-logged, never fatal.
//! The scheduler runs `reports::generate(ReportWindow::Weekly, …)` on
//! the configured cron expression (default `0 0 9 * * Mon`).

#![cfg(feature = "in_process_cron")]

use std::path::PathBuf;
use std::sync::Arc;

use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};

/// Configuration for the in-process cron scheduler.
///
/// Default: Mondays 09:00 (`"0 0 9 * * Mon"`), output under
/// `evidence/v1/operator-success-reports/reports/`.  All fields are operator-tunable.
#[derive(Debug, Clone)]
pub struct CronConfig {
    /// Cron expression in tokio-cron-scheduler 6-field form
    /// `"sec min hour day month dow"`.  Default: `"0 0 9 * * Mon"`.
    pub expression: String,
    /// Audit DB path the report renders against.
    pub ledger_db_path: PathBuf,
    /// Parquet root for `ParquetMarkSource`.  Same root the agent uses
    /// for replay feeds.
    pub parquet_root: PathBuf,
    /// Output directory for the rendered markdown.  Filename is computed
    /// from the run timestamp at job-fire time.
    pub output_dir: PathBuf,
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            expression: "0 0 9 * * Mon".to_string(),
            ledger_db_path: PathBuf::from("data/audit/ledger.db"),
            parquet_root: PathBuf::from("data/binance"),
            output_dir: PathBuf::from("evidence/v1/operator-success-reports/reports"),
        }
    }
}

/// Build and start the in-process cron scheduler.
///
/// Returns the running [`JobScheduler`] so the caller can hold it for
/// the lifetime of the agent process.  Failure to construct or start
/// the scheduler is propagated as `Err`; failure inside a fired job is
/// warn-logged, never fatal.
///
/// # Errors
///
/// Returns the underlying `JobSchedulerError` (boxed via `anyhow`) if
/// the scheduler cannot be created, the cron expression is invalid, or
/// the scheduler fails to start.
pub async fn start(cfg: CronConfig) -> anyhow::Result<JobScheduler> {
    let sched = JobScheduler::new().await?;
    let cfg_arc = Arc::new(cfg.clone());

    let job = Job::new_async(cfg.expression.as_str(), move |_uuid, _l| {
        let cfg = Arc::clone(&cfg_arc);
        Box::pin(async move {
            run_weekly(&cfg).await;
        })
    })?;
    sched.add(job).await?;
    sched.start().await?;
    info!(
        cron = %cfg.expression,
        out = %cfg.output_dir.display(),
        "in-process cron scheduler started — Weekly operator success report"
    );
    Ok(sched)
}

/// Single fire of the weekly operator-success-report.  Failures are
/// warn-logged — the scheduler keeps running.
async fn run_weekly(cfg: &CronConfig) {
    use reports::{MarkSource, ParquetMarkSource, ReportWindow};

    // Output filename: `weekly-<RFC3339-ts>.md` so concurrent fires
    // never collide.
    let now_str = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    let out_path = cfg.output_dir.join(format!("weekly-{now_str}.md"));

    if let Err(e) = std::fs::create_dir_all(&cfg.output_dir) {
        warn!(error = %e, dir = %cfg.output_dir.display(), "cron report mkdir failed (non-fatal)");
        return;
    }

    let marks: Box<dyn MarkSource> = Box::new(ParquetMarkSource::new(cfg.parquet_root.clone()));
    match reports::generate(
        ReportWindow::Weekly,
        cfg.ledger_db_path.as_path(),
        marks.as_ref(),
        out_path.as_path(),
        None,
    )
    .await
    {
        Ok(artifacts) => {
            info!(
                run_id = %artifacts.run_id,
                path = %artifacts.markdown_path.display(),
                "in-process cron weekly report rendered"
            );
        }
        Err(e) => {
            warn!(error = %e, "in-process cron weekly report failed (non-fatal)");
        }
    }
}
