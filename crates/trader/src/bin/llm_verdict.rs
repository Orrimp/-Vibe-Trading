//! `llm_verdict` — L0-L4 verdict report bin for the LLM forecaster.
//!
//! Reads the last N `llm_forecast_entries` rows from the audit DB, computes
//! the ADR-0039 § D1.b L-verdict priority tree (L1 → L2 → L3 → L4 → L0),
//! and emits a deterministic markdown report under the output directory.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p trader --bin llm_verdict -- \
//!     --audit-db data/audit.db \
//!     --window-bars 1000 \
//!     --out-dir spec/v3-llm-forecaster/reports/
//! ```
//!
//! ## Read-only contract
//!
//! - NO writes to any DB.
//! - Exactly one filesystem-write: `std::fs::write(out_path, body)`.
//!
//! ## Determinism (ADR-0039 § D2)
//!
//! - No `SystemTime::now()` on any hot path — wall-clock + generated timestamp
//!   go to YAML frontmatter only (excluded from body SHA-256).
//! - All floats serialised with 6-decimal precision per ADR-0039 § D2.
//! - Row order: `ORDER BY ts DESC LIMIT N` then reversed for chronological order.
//! - HashMap iteration replaced by sorted Vec.
//!
//! ## Body-vs-frontmatter discipline (per AGENT.md)
//!
//! | Frontmatter (excluded from hash)   | Body (hashed — deterministic)         |
//! |------------------------------------|---------------------------------------|
//! | `generated:` timestamp             | window stats                          |
//! | `wall_clock_s:`                    | rating histogram                      |
//! | `host:`, `git_commit:`             | verdict section                       |
//! | `audit_db:` path                   | notes section                         |
//!
//! ## Cross-references
//!
//! - ADR-0039 § D1 — L0-L4 verdict algorithm.
//! - ADR-0039 § D2 — Report body shape + float canonicalisation.
//! - `crates/strategy/src/llm_forecaster/verdict.rs` — `classify_l` + types.
//! - `crates/audit/migrations/012_llm_forecast.sql` — DB schema.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use trader::llm_forecaster::verdict::{LlmForecastRow, aggregate_rows, classify_l};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "llm_verdict",
    about = "L0-L4 verdict report bin for the LLM forecaster (ADR-0039 § D1)",
    long_about = "Reads the last --window-bars rows from llm_forecast_entries,\n\
                  computes the L-verdict priority tree (L1 → L2 → L3 → L4 → L0),\n\
                  and emits a deterministic markdown report.\n\n\
                  Read-only contract: no writes to any DB."
)]
struct Args {
    /// Path to the audit SQLite DB (`data/audit.db`).
    #[arg(long, default_value = "data/audit.db")]
    audit_db: PathBuf,

    /// Number of most-recent `llm_forecast_entries` rows to evaluate.
    #[arg(long, default_value = "1000")]
    window_bars: u64,

    /// Output directory for the L-verdict report.
    #[arg(long, default_value = "spec/v3-llm-forecaster/reports/")]
    out_dir: PathBuf,

    /// Architect-locked cost projection from `llm-forecaster-bench` (USD).
    /// Default: $0.10 per bench (spike-confirmed Haiku estimate for 1000 bars).
    #[arg(long, default_value = "0.10")]
    cost_projected_usd: f64,

    /// Per-run USD cost cap (from `LlmForecasterConfig::cost_cap_usd_per_backtest`).
    #[arg(long, default_value = "100.0")]
    cost_cap_usd: f64,

    /// Pearson(confidence, signed-correctness) correlation.
    ///
    /// Computing this requires realised returns which are not in the audit DB.
    /// Pass the correlation explicitly if known from backtesting, otherwise
    /// the default 0.0 triggers L2 as a conservative fallback (no calibration
    /// data available).
    #[arg(long, default_value = "0.0")]
    confidence_outcome_corr: f64,

    /// Label for this evaluation window (appears in the report body).
    #[arg(long, default_value = "audit-db-window")]
    window_label: String,
}

// ── DB read ───────────────────────────────────────────────────────────────────

/// Load the last N rows from `llm_forecast_entries`, ordered chronologically
/// (oldest first). Returns an empty Vec if the table doesn't exist or has no rows.
fn load_rows(db_path: &std::path::Path, limit: u64) -> Result<Vec<LlmForecastRow>> {
    let conn = rusqlite::Connection::open(db_path)
        .with_context(|| format!("open audit DB {}", db_path.display()))?;

    // Check if the table exists first (audit DB may not have migration 012 applied).
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name='llm_forecast_entries'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);

    if !table_exists {
        info!("llm_forecast_entries table does not exist — returning empty row set");
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT rating, confidence, reasoning_trace, trace_sha256, cost_usd \
             FROM llm_forecast_entries \
             ORDER BY ts DESC \
             LIMIT ?",
        )
        .context("prepare SELECT statement")?;

    // Collect rows (DESC order) then reverse to chronological (oldest first).
    let mut rows: Vec<LlmForecastRow> = stmt
        .query_map(rusqlite::params![limit as i64], |row| {
            let rating: String = row.get(0)?;
            let confidence_str: String = row.get(1)?;
            let reasoning_trace: String = row.get(2)?;
            let trace_sha256: String = row.get(3)?;
            let cost_usd_str: String = row.get(4)?;

            let confidence_f64 = confidence_str.parse::<f64>().unwrap_or(0.5);
            let cost_usd_f64 = cost_usd_str.parse::<f64>().unwrap_or(0.0);

            Ok(LlmForecastRow {
                rating,
                confidence_f64,
                reasoning_trace,
                trace_sha256,
                cost_usd_f64,
            })
        })
        .context("query llm_forecast_entries")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("collect rows")?;

    rows.reverse(); // chronological order
    Ok(rows)
}

// ── Report rendering (ADR-0039 § D2) ─────────────────────────────────────────

fn render_report(
    rows: &[LlmForecastRow],
    args: &Args,
    generated: &str,
    wall_clock_s: f64,
) -> String {
    let stats = aggregate_rows(
        rows,
        args.cost_projected_usd,
        args.cost_cap_usd,
        args.confidence_outcome_corr,
        args.window_label.clone(),
    );
    let verdict = classify_l(&stats);

    let host = hostname();
    let git_commit = git_head_sha();

    // ── Frontmatter (advisory, NOT hashed) ────────────────────────────────────
    let frontmatter = format!(
        "---\n\
         slug: v3-llm-forecaster\n\
         scenario: llm-verdict-{window_label}\n\
         generated: {generated}\n\
         wall_clock_s: {wall_clock_s:.1}\n\
         host: {host}\n\
         git_commit: {git_commit}\n\
         audit_db: {audit_db}\n\
         verdict: {verdict_label}\n\
         ---\n",
        window_label = args.window_label,
        audit_db = args.audit_db.display(),
        verdict_label = verdict.label(),
    );

    // ── Body (deterministic, hashed) ──────────────────────────────────────────
    let mut body = String::new();

    body.push_str("# LLM-forecaster L-verdict report (ADR-0039 § D1)\n\n");

    // Window summary section.
    body.push_str("## Window summary\n\n");
    body.push_str("| Field                     | Value                                |\n");
    body.push_str("|---------------------------|--------------------------------------|\n");
    body.push_str(&format!(
        "| window_label              | {}                                   |\n",
        stats.window_label
    ));
    body.push_str(&format!(
        "| window_bars_requested     | {}                                   |\n",
        args.window_bars
    ));
    body.push_str(&format!(
        "| n_calls                   | {}                                   |\n",
        stats.n_calls
    ));
    body.push_str(&format!(
        "| n_unique_traces           | {}                                   |\n",
        stats.n_unique_traces
    ));
    body.push_str(&format!(
        "| n_traces_below_50_chars   | {}                                   |\n",
        stats.n_traces_below_50_chars
    ));
    body.push_str(&format!(
        "| mean_trace_len_chars      | {:.6}                            |\n",
        stats.mean_trace_len_chars
    ));
    body.push_str(&format!(
        "| cost_actual_usd           | {:.6}                            |\n",
        stats.cost_actual_usd
    ));
    body.push_str(&format!(
        "| cost_projected_usd        | {:.6}                            |\n",
        stats.cost_projected_usd
    ));
    body.push_str(&format!(
        "| cost_cap_usd              | {:.6}                            |\n",
        stats.cost_cap_usd
    ));
    body.push_str(&format!(
        "| confidence_outcome_corr   | {:.6}                            |\n",
        stats.confidence_outcome_corr
    ));
    body.push('\n');

    // Rating distribution histogram section.
    body.push_str("## Rating distribution\n\n");
    body.push_str("| Rating       | Count | Fraction |\n");
    body.push_str("|--------------|-------|----------|\n");
    let rating_names = ["STRONG_SELL", "SELL", "HOLD", "BUY", "STRONG_BUY"];
    for (i, name) in rating_names.iter().enumerate() {
        let count = stats.rating_dist[i];
        let frac = count as f64 / stats.n_calls.max(1) as f64;
        body.push_str(&format!("| {:<12} | {:<5} | {:.6} |\n", name, count, frac,));
    }
    body.push('\n');

    // Computed metrics section.
    body.push_str("## Computed metrics (L-verdict inputs)\n\n");
    body.push_str("| Metric                  | Value      | Threshold         |\n");
    body.push_str("|-------------------------|------------|-------------------|\n");
    body.push_str(&format!(
        "| hold_frac               | {:.6}   | >= 0.95 fires L1  |\n",
        stats.hold_frac()
    ));
    body.push_str(&format!(
        "| |confidence_outcome_corr| | {:.6}   | < 0.05 fires L2   |\n",
        stats.confidence_outcome_corr.abs()
    ));
    body.push_str(&format!(
        "| overrun_ratio           | {:.6}   | > 2.0 fires L3    |\n",
        stats.overrun_ratio()
    ));
    body.push_str(&format!(
        "| short_frac              | {:.6}   | > 0.50 fires L4   |\n",
        stats.short_frac()
    ));
    body.push_str(&format!(
        "| duplicate_frac          | {:.6}   | > 0.50 fires L4   |\n",
        stats.duplicate_frac()
    ));
    body.push('\n');

    // Verdict section (ADR-0039 § D2 shape).
    body.push_str("## Verdict\n\n");
    body.push_str("| Field             | Value                                          |\n");
    body.push_str("|-------------------|------------------------------------------------|\n");
    body.push_str(&format!(
        "| Case              | {}                                             |\n",
        verdict.label()
    ));
    body.push_str(&format!("| Trigger evidence  | {} |\n", verdict.evidence()));
    body.push_str(&format!(
        "| Routes to         | {} |\n",
        verdict.routes_to()
    ));
    body.push('\n');

    // Notes section.
    body.push_str("## Notes\n\n");
    body.push_str(
        "- L-verdict algorithm: see \
         [ADR-0039 § D1](../architecture/adr/0039-llm-forecaster-verdict-criteria.md#d1-l-verdict-priority-tree).\n",
    );
    body.push_str("- Read-only against the audit DB; no writes to any table.\n");
    body.push_str(
        "- `confidence_outcome_corr` requires realised returns (not in audit DB). \
         Pass via `--confidence-outcome-corr` if known from a backtest run; \
         default 0.0 triggers L2 as a conservative fallback.\n",
    );
    if stats.n_calls == 0 {
        body.push_str(
            "- **WARNING**: zero rows found in `llm_forecast_entries`. \
             The audit DB may not have migration 012 applied or the LLM forecaster \
             has not been run yet. L-verdict result reflects an empty window.\n",
        );
    }

    // Combine.
    format!("{frontmatter}{body}")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_head_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let odt = time::OffsetDateTime::from_unix_timestamp(now as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        odt.year(),
        odt.month() as u8,
        odt.day(),
        odt.hour(),
        odt.minute(),
        odt.second(),
    )
}

fn today_yyyymmdd() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let odt = time::OffsetDateTime::from_unix_timestamp(now as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!("{:04}{:02}{:02}", odt.year(), odt.month() as u8, odt.day())
}

/// Compute body-only SHA-256 of the report (strips YAML frontmatter).
/// Mirrors `scripts/hash_report.py` strip logic.
fn body_sha256(report: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut in_frontmatter = false;
    let mut past_frontmatter = false;
    let mut body_lines: Vec<&str> = Vec::new();
    for (i, line) in report.lines().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            past_frontmatter = true;
            continue;
        }
        if past_frontmatter {
            body_lines.push(line);
        }
    }
    let body = body_lines.join("\n");
    let digest = Sha256::digest(body.as_bytes());
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in &digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let t0 = Instant::now();

    info!(
        audit_db = %args.audit_db.display(),
        window_bars = args.window_bars,
        "llm_verdict: loading rows from audit DB"
    );

    // ── Load rows ─────────────────────────────────────────────────────────────
    let rows = load_rows(&args.audit_db, args.window_bars)
        .with_context(|| format!("load rows from {}", args.audit_db.display()))?;

    info!(n_rows = rows.len(), "rows loaded");

    // ── Compute verdict ───────────────────────────────────────────────────────
    let stats = aggregate_rows(
        &rows,
        args.cost_projected_usd,
        args.cost_cap_usd,
        args.confidence_outcome_corr,
        args.window_label.clone(),
    );
    let verdict = classify_l(&stats);

    info!(
        verdict = verdict.label(),
        evidence = verdict.evidence(),
        follow_on = verdict.follow_on(),
        "L-verdict computed"
    );

    // ── Render report ─────────────────────────────────────────────────────────
    let wall_clock_s = t0.elapsed().as_secs_f64();
    let generated = now_iso8601();

    let report = render_report(&rows, &args, &generated, wall_clock_s);
    let body_sha = body_sha256(&report);

    // ── Write report ──────────────────────────────────────────────────────────
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create out_dir {}", args.out_dir.display()))?;

    let date_tag = today_yyyymmdd();
    let out_filename = format!("llm-verdict-{}.md", date_tag);
    let out_path = args.out_dir.join(&out_filename);

    std::fs::write(&out_path, &report)
        .with_context(|| format!("write report {}", out_path.display()))?;

    println!("wrote {} (body-SHA256 = {})", out_path.display(), body_sha);

    Ok(())
}
