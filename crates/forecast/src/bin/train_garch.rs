//! `train_garch` — per-symbol GARCH(1,1) MLE fit driver.
//!
//! Reads real Binance OHLCV parquet bars for the BS-1 training span
//! (2023-01-01..2024-01-01), fits a GARCH(1,1) model to each of the 10
//! USDT-quote symbols, and emits a JSON checkpoint under
//! `crates/forecast/checkpoints/anchors/garch-bs1-<sha>.json`.
//!
//! ## JSON checkpoint schema (ADR-0038 § D3)
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "target_kind": "Parkinson",
//!   "target_horizon_bars": 24,
//!   "train_span_start": "2023-01-01T00:00:00Z",
//!   "train_span_end":   "2024-01-01T00:00:00Z",
//!   "data_revision_sha": "<64-hex>",
//!   "params": {
//!     "ADAUSDT":  { "omega": …, "alpha": …, "beta": …, "unconditional_var": …,
//!                   "log_likelihood": …, "n_iters": …, "converged": true },
//!     ...
//!   }
//! }
//! ```
//!
//! ## Aggregate SHA derivation (ADR-0038 § D3)
//!
//! `checkpoint_revision = SHA-256("garch-bs1\n" || "schema_version=1\n" || ...
//!  || canonical_params_block)` where `canonical_params_block` uses `%.6e` floats,
//! alphabetical symbol keys, alphabetical inner-key order.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p forecast --bin train_garch --features candle --release -- --scenario bs1
//! ```
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` on the fit path (wall-clock goes to frontmatter only).
//! - Identical input returns → byte-identical JSON output (R11.4).
//! - No RNG used: GARCH MLE is deterministic given the hyperparameter lock.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use forecast::features::{TimeSpan, load_bars_pub};
use forecast::garch::GarchModel;

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Which training scenario to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioArg {
    /// BS-1: trained on full 2023 (2023-01-01..2024-01-01), Parkinson target, 24h horizon.
    Bs1,
}

impl ScenarioArg {
    fn train_span(self) -> (time::OffsetDateTime, time::OffsetDateTime) {
        match self {
            ScenarioArg::Bs1 => (
                time::macros::datetime!(2023-01-01 00:00:00 UTC),
                time::macros::datetime!(2024-01-01 00:00:00 UTC),
            ),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "garch-bs1",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "train_garch",
    about = "Per-symbol GARCH(1,1) MLE fit driver — emits garch-bs1-<sha>.json checkpoint",
    long_about = "Fits GARCH(1,1) to each of the 10 USDT-quote symbols over the BS-1 training span\n\
                  and emits a deterministic JSON checkpoint.\n\n\
                  Determinism contract: byte-identical JSON on two sequential runs with the same\n\
                  input data (R11.4 / ADR-0038 § D3)."
)]
struct Args {
    /// Which training scenario to run.
    #[arg(long, value_enum, default_value = "bs1")]
    scenario: ScenarioArg,

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Output directory for the checkpoint JSON.
    #[arg(long, default_value = "crates/forecast/checkpoints/anchors/")]
    out_dir: PathBuf,
}

// ── Universe ──────────────────────────────────────────────────────────────────

/// The 10 USDT-quote symbols in alphabetical order (locked, ADR-0038 § D2.a).
const UNIVERSE: &[&str] = &[
    "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
    "SOLUSDT", "XRPUSDT",
];

// ── Per-symbol JSON param types ───────────────────────────────────────────────

/// Per-symbol GARCH params stored in the checkpoint JSON.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SymbolParams {
    omega: f64,
    alpha: f64,
    beta: f64,
    unconditional_var: f64,
    log_likelihood: f64,
    n_iters: usize,
    converged: bool,
}

/// Full checkpoint document.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GarchCheckpoint {
    schema_version: u32,
    target_kind: String,
    target_horizon_bars: u32,
    train_span_start: String,
    train_span_end: String,
    data_revision_sha: String,
    params: BTreeMap<String, SymbolParams>,
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let t0 = Instant::now();
    let (span_start, span_end) = args.scenario.train_span();
    let span = TimeSpan::new(span_start, span_end);
    let scenario_label = args.scenario.label();

    // Read data revision SHA from REVISION.toml.
    let revision_path = args.data_root.join("REVISION.toml");
    let revision_str = std::fs::read_to_string(&revision_path)
        .with_context(|| format!("read {}", revision_path.display()))?;
    let data_revision_sha =
        extract_revision_sha(&revision_str).with_context(|| "parse REVISION.toml")?;

    info!(scenario = scenario_label, data_revision_sha = %data_revision_sha, "train_garch starting");

    let mut params: BTreeMap<String, SymbolParams> = BTreeMap::new();

    for &symbol in UNIVERSE {
        let bars = load_bars_pub(&args.data_root, symbol, &span)
            .with_context(|| format!("load bars for {symbol}"))?;

        // Compute log-returns from close prices.
        let returns: Vec<f64> = bars
            .windows(2)
            .map(|w| {
                let prev = &w[0];
                let curr = &w[1];
                if prev.close > 0.0 && curr.close > 0.0 {
                    (curr.close / prev.close).ln()
                } else {
                    0.0
                }
            })
            .collect();

        let model = GarchModel::fit(&returns)
            .with_context(|| format!("GARCH MLE fit failed for {symbol}"))?;

        info!(
            symbol = symbol,
            omega = model.omega,
            alpha = model.alpha,
            beta = model.beta,
            unconditional_var = model.unconditional_var,
            log_likelihood = model.log_likelihood,
            n_iters = model.n_iters,
            converged = model.converged,
            "garch_fit"
        );

        params.insert(
            symbol.to_string(),
            SymbolParams {
                omega: model.omega,
                alpha: model.alpha,
                beta: model.beta,
                unconditional_var: model.unconditional_var,
                log_likelihood: model.log_likelihood,
                n_iters: model.n_iters,
                converged: model.converged,
            },
        );
    }

    // Build checkpoint document.
    let checkpoint = GarchCheckpoint {
        schema_version: 1,
        target_kind: "Parkinson".to_string(),
        target_horizon_bars: 24,
        train_span_start: "2023-01-01T00:00:00Z".to_string(),
        train_span_end: "2024-01-01T00:00:00Z".to_string(),
        data_revision_sha: data_revision_sha.clone(),
        params,
    };

    // Derive checkpoint_revision SHA-256.
    let checkpoint_revision = derive_checkpoint_revision(&checkpoint);

    // Serialise checkpoint to JSON with %.9e floats for human readability.
    let json_body =
        serde_json::to_string_pretty(&checkpoint).context("serialise checkpoint JSON")?;

    // Write to out_dir/garch-bs1-<sha>.json.
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create {}", args.out_dir.display()))?;

    let out_filename = format!("{scenario_label}-{checkpoint_revision}.json");
    let out_path = args.out_dir.join(&out_filename);
    std::fs::write(&out_path, &json_body)
        .with_context(|| format!("write {}", out_path.display()))?;

    let elapsed = t0.elapsed().as_secs_f64();
    let n_symbols = UNIVERSE.len();

    println!(
        "{scenario_label} fitted {n_symbols} symbols in {elapsed:.1} s; checkpoint_revision = {checkpoint_revision}"
    );

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract `[revision].sha256` from REVISION.toml content.
fn extract_revision_sha(toml_str: &str) -> Result<String> {
    // Simple TOML parse: look for `sha256 = "..."` under `[revision]`.
    for line in toml_str.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("sha256") {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let value = trimmed[eq + 1..].trim().trim_matches('"').to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }
    }
    anyhow::bail!("sha256 field not found in REVISION.toml")
}

/// Derive the aggregate checkpoint_revision SHA-256 per ADR-0038 § D3.
///
/// Input:
/// ```text
/// "garch-bs1\n"
/// "schema_version=1\n"
/// "target_kind=Parkinson\n"
/// "target_horizon_bars=24\n"
/// "train_span=2023-01-01T00:00:00Z..2024-01-01T00:00:00Z\n"
/// "data_revision_sha=<64-hex>\n"
/// canonical_params_block  // %.6e floats, alphabetical symbol keys, alphabetical inner-key order
/// ```
fn derive_checkpoint_revision(checkpoint: &GarchCheckpoint) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    hasher.update(b"garch-bs1\n");
    hasher.update(format!("schema_version={}\n", checkpoint.schema_version).as_bytes());
    hasher.update(format!("target_kind={}\n", checkpoint.target_kind).as_bytes());
    hasher.update(format!("target_horizon_bars={}\n", checkpoint.target_horizon_bars).as_bytes());
    hasher.update(
        format!(
            "train_span={}..{}\n",
            checkpoint.train_span_start, checkpoint.train_span_end
        )
        .as_bytes(),
    );
    hasher.update(format!("data_revision_sha={}\n", checkpoint.data_revision_sha).as_bytes());

    // Canonical params block: alphabetical symbol keys, inner keys alpha-sorted,
    // floats formatted as %.6e.  Inner key order: alpha, beta, converged,
    // log_likelihood, n_iters, omega, unconditional_var.
    for (symbol, p) in &checkpoint.params {
        hasher.update(
            format!(
                "{symbol}: alpha={:.6e} beta={:.6e} converged={} log_likelihood={:.6e} n_iters={} omega={:.6e} unconditional_var={:.6e}\n",
                p.alpha, p.beta, p.converged, p.log_likelihood, p.n_iters, p.omega, p.unconditional_var
            )
            .as_bytes(),
        );
    }

    let digest = hasher.finalize();
    // Lower-case hex.
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in &digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}
