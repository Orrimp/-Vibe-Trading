//! Operator-success-report bin (R1).
//!
//! ```text
//! cargo run -p reports --bin report -- \
//!     --period 7d \
//!     --ledger /path/to/audit.db \
//!     --output spec/reports/success/sample-7d.md \
//!     --seed 0xC0FFEE
//! ```
//!
//! Body filled in T813.  T807 lands a stub orchestrator that exits 0
//! on a happy path so cron / launchd can be wired before the renderers
//! are complete.

#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

use std::path::PathBuf;

use clap::Parser;
use reports::{FrozenMarkSource, ReportError, ReportWindow};

#[derive(Parser, Debug)]
#[command(name = "report", version, about = "Operator success report renderer")]
struct Cli {
    /// Window selector: `7d`, `30d`, `90d`, `weekly`, `monthly`,
    /// `since:<RFC3339>`, or `inception`.
    #[arg(long)]
    period: String,

    /// Path to the audit `SQLite` ledger.
    #[arg(long)]
    ledger: PathBuf,

    /// Output markdown path.  The atomic-write tempfile lands at
    /// `<output>.tmp.<pid>` next to it, so the parent directory must
    /// be writable.
    #[arg(
        long,
        default_value = "spec/operator-success-reports/reports/report.md"
    )]
    output: PathBuf,

    /// Optional fixture seed (`0x` prefix tolerated).  Surfaces in the
    /// front-matter `seed:` field so two reports with the same seed
    /// produce byte-identical bodies.
    #[arg(long)]
    seed: Option<String>,
}

fn parse_seed(s: &str) -> Option<u64> {
    let trimmed = s.trim();
    let stripped = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    u64::from_str_radix(stripped, 16).ok()
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let window = match ReportWindow::parse(&cli.period) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("invalid --period: {e}");
            return std::process::ExitCode::from(2);
        }
    };

    let seed = cli.seed.as_deref().and_then(parse_seed);

    // T807 ships a frozen mark source that returns an empty store —
    // the stub orchestrator does not call into it.  T813 wires the
    // real ParquetMarkSource path.
    let frozen = match FrozenMarkSource::from_csv_str("symbol,close_time,close\n") {
        Ok(m) => m,
        Err(e) => {
            eprintln!("init mark source: {e}");
            return std::process::ExitCode::from(1);
        }
    };

    match reports::generate(window, &cli.ledger, &frozen, &cli.output, seed).await {
        Ok(artifacts) => {
            println!(
                "wrote {} (run_id={})",
                artifacts.markdown_path.display(),
                artifacts.run_id
            );
            std::process::ExitCode::from(0)
        }
        Err(ReportError::Reconciliation { sibling_path }) => {
            eprintln!(
                "RECONCILIATION FAIL — see {} (R11.4)",
                sibling_path.display()
            );
            std::process::ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("report error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}
